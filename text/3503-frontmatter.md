- Feature Name: `frontmatter`
- Start Date: 2023-09-26
- RFC PR: [rust-lang/rfcs#3503](https://github.com/rust-lang/rfcs/pull/3503)
- Rust Issue: [rust-lang/cargo#12207](https://github.com/rust-lang/cargo/issues/12207)


# Summary
[summary]: #summary

Add a frontmatter syntax to Rust as a way for [cargo to have manifests embedded in source code][RFC 3502]:
````rust
#!/usr/bin/env cargo
---
[dependencies]
clap = { version = "4.2", features = ["derive"] }
---

use clap::Parser;

#[derive(Parser, Debug)]
#[clap(version)]
struct Args {
    #[clap(short, long, help = "Path to config")]
    config: Option<std::path::PathBuf>,
}

fn main() {
    let args = Args::parse();
    println!("{:?}", args);
}
````

# Motivation
[motivation]: #motivation

["cargo script"][RFC 3502] is in need of a syntax for embedding manifests in source.
See that RFC for its motivations.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Static site generators use a frontmatter syntax to embed metadata in markdown files:
```md
---
name: My Blog Post
---

## Stuff happened

Hello world!
```

We are carrying this concept over to Rust while merging some lessons from commonmark's fenced code blocks:
````rust
#!/usr/bin/env cargo
---
[dependencies]
clap = { version = "4.2", features = ["derive"] }
---

use clap::Parser;

#[derive(Parser, Debug)]
#[clap(version)]
struct Args {
    #[clap(short, long, help = "Path to config")]
    config: Option<std::path::PathBuf>,
}

fn main() {
    let args = Args::parse();
    println!("{:?}", args);
}
````

Like with [commonmark code fences](https://spec.commonmark.org/0.30/#info-string),
an info-string is allowed after the opening `---` for use by the command interpreting the block to identify the contents of the block.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

When parsing Rust source, after stripping the shebang (`#!`), rustc will strip the frontmatter:
- May include 0+ blank lines (whitespace + newline)
- Opens with 3+ dashes followed by 0+ whitespace, an optional term (one or more characters excluding whitespace and commas), 0+ whitespace, and a newline
  - The variable number of dashes is an escaping mechanism in case `---` shows up in the content
- All content is ignored by `rustc` until the same number of dashes is found at the start of a line.
  The line must terminate by 0+ whitespace and then a newline.
- Unlike commonmark, it is an error to not close the frontmatter seeing to detect problems earlier in the process seeing as the primary content is what comes after the frontmatter

This applies anywhere shebang stripping is performed.
For example, if [`include!`](https://doc.rust-lang.org/std/macro.include.html) strips shebangs, then it will also frontmatter.

As cargo will be the first step in the process to parse this,
the responsibility for high quality error messages will largely fall on cargo.

# Drawbacks
[drawbacks]: #drawbacks

- A new concept for Rust syntax, adding to overall cognitive load
- Ecosystem tooling updates to deal with new syntax

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Within this solution,
we considered starting with only allowing this in the root `mod` (e.g. `main.rs`)
but decided to allow it in any file mostly for ease of implementation.
Like with Python, this allows any file in a package (with the correct deps and `mod`s) to be executed, allowing easier interacting experiences in verifying behavior.

## Required vs Optional Shebang

We could require the shebang to be present for all cargo-scripts.
This would most negatively impact Windows users as the shebang is a no-op.
We still care about Windows because cargo-scripts can still be used for exploration and prototyping,
even if they can't directly be used as drop-in utilities.

The main reason to require a shebang is to positively identify the associated "interpreter".
However, statically analyzing a shebang is [complicated](https://stackoverflow.com/questions/38059830/how-does-perl-avoid-shebang-loops)
and we are wanting to avoid it in the core workflow.
This isn't to say that tools like rust-analyzer might choose to require it to help their workflow.

## Blank lines

Originally, the proposal viewed the block as being "part of" the shebang and didn't allow them to be separated by blank lines.
However, the shebang is optional and users are likely to assume they can use blanklines
(see https://www.youtube.com/watch?v=S8MLYZv_54w).

This could cause ordering confusion (doc comments vs attributes vs frontmatter)

## Infostring

The main question on infostrings is whether they are tool-defined or rustc-defined.
At one time, we proposed requiring the infostring and requiring it be `cargo` as a way to defer this decision.

As the design requirements are catered to processing by external tools, as opposed to rustc,
we are instead reserving this syntax for external tools by making the infostrings tool-defined.
The Rust toolchain (rustc, clippy, rustdoc, etc) already have access to attributes for user-provided content.
If they need a more ergonomic way of specifying content, we should solve that more generally for attributes.

With that decision made, the infostring can be optional.
Can it also be deferred out?
Possibly, but we are leaving them in for unpredictable exception cases and in case users want to make the syntax explicit for their editor (especially if its not `cargo` which more trivial editor implementations will likely assume).
We may at least defer stabilization of infostrings.

The infostring syntax was selected to allow file names (e.g. `Cargo.lock`).
Additional attributes are left to a future possibility.

## Syntax

[RFC 3502] lays out some design principles, including
- Single-file packages should have a first-class experience
  - Provides a higher quality of experience (doesn't feel like a hack or tacked on)
  - Transferable knowledge, whether experience, stackoverflow answers, etc
  - Easier unassisted migration between single-file and multi-file packages
  - The more the workflows deviate, the higher the maintenance and support costs for the cargo team

This led us to wanting to re-use the existing manifest format inside of Rust code.
The question is what that syntax for embedding should be.

When choosing the syntax, our care-abouts are
- How obvious it is for new users when they see it
- How easy it is for newer users to remember it and type it out
- How machine editable it is for `cargo add` and friends
- Needs to be valid Rust code for quality of error messages, etc
- Simple enough syntax for tools to parse without a full Rust parser
  - Leave Rust syntax errors to rustc, rather than masking them with lower quality ones
  - Ideally we allows random IDE tools (e.g. [`crates.nvim`](https://github.com/Saecki/crates.nvim) to still have easy access to the manifest
- Leave the door open in case we want to reuse the syntax for embedded lockfiles
- Leave the door open for single-file `lib`s

### Why add to Rust syntax, rather than letting Cargo handle it

The most naive way for cargo to handle this is for cargo to strip the manifest, write the Rust file to a temporary file, and compile that.
This is what has traditionally been done with various iterations of "cargo script".

This provides a second-class experience which runs counter to one of the design goals
- Error messages, `cargo metadata`, etc point to the stripped source with an "odd" path, rather than the real source
- Every tool that plans to support it would need to be updated to do stripping (`cargo fmt`, `cargo clippy`, etc)

A key part in there is "plan to support".
We'd need to build up buy-in for tools to be supporting a Cargo-only syntax.
This becomes more difficult when the tool in question tries to be Cargo-agnostic.
By having Cargo-agnostic external tool syntax in Rust, this mostly becomes transparent.

We could build a special relationship with rustc to support this.
For example, rustdoc passes code to rustc on stdin and sets `UNSTABLE_RUSTDOC_TEST_PATH` and `UNSTABLE_RUSTDOC_TEST_LINE` to control how errors are rendered.
We could then also special case the messages inside of cargo.
This both adds a major support burden to keep this house of lies standing but still falls short when it comes to tooling support.
Now every tool that wants to support the Cargo-only syntax has to build their own house of lies.

### Frontmatter

This proposed syntax builds off of the precedence of Rust having syntax specialized for an external tool
(doc-comments for rustdoc).
However, a new syntax is used instead of extending the comment syntax:
- Simplified for being parsed by arbitrary tools (cargo, vim plugins, etc) without understanding the full Rust grammar
- Side steps compatibility issues with both
  user expectations with the looseness of comment syntax (which supporting would make it harder to parse)
  and any existing comments that may look like a new structured comment syntax

The difference between this syntax and comments is comments are generally
geared towards people, even if a subset (doc-comments) are also able to be
somewhat processed by a program, while this is geared mostly towards machine
processing.

This proposal mirrors the location of YAML frontmatter (absolutely first).
As we learn more of its uses and problems people run into in practice,
we can evaluate if we want to loosen any of the rules.

Differences with YAML frontmatter include:
- Variable number of dashes (for escaping)
- Optional frontmatter

Besides characters, differences with commonmark code fences include:
- no indenting of the fenced code block
- open/close must be a matching pair, rather than the close having "the same or more"

Benefits:
- Visually/syntactically lightweight
- Users can edit/copy/paste the manifest without dealing with leading characters
- Has parallels to ideas outside of Rust, building on external knowledge that might exist
- Easy for cargo and any third-party tool to parse and modify
  - As cargo will be parsing before rustc,
    cargo being able to work off of a reduced syntax is paramount for ensuring
    cargo doesn't mask the high quality rustc errors with lower-quality errors of
    its own
- In the future, this can be leveraged by other build systems or tools

Downsides:
- Familiar syntax in an unfamiliar use may make users feel unsettled, unsure how to proceed (what works and what doesn't).
- If viewed from the lens of a comment, it isn't a variant of comment syntax like doc-comments

### Alternative 1: Vary the opening/closing character

Instead of dashes, we could do another character, like
- backticks, like in commonmark code fences
  - `~`, using a lesser known markdown code fence character
- `+` like [zola and hugo's TOML frontmatter](https://www.getzola.org/documentation/getting-started/overview/#markdown-content)
- `=`
- Open with `>>>` and close with `<<<`, like with HEREDOC (or invert it)

In practice (with infostrings):
````rust
#!/usr/bin/env cargo
```cargo
[package]
edition = "2018"
```

fn main() {
}
````
```rust
#!/usr/bin/env cargo
~~~cargo
[package]
edition = "2018"
~~~

fn main() {
}
```
```rust
#!/usr/bin/env cargo
+++cargo
[package]
edition = "2018"
+++

fn main() {
}
```
```rust
#!/usr/bin/env cargo
===cargo
[package]
edition = "2018"
===

fn main() {
}
```
```rust
#!/usr/bin/env cargo
>>>cargo
[package]
edition = "2018"
<<<

fn main() {
}
```
```rust
#!/usr/bin/env cargo
<<<cargo
[package]
edition = "2018"
>>>

fn main() {
}
```

Downsides
- With `>>>` it isn't quite like HEREDOC to have less overhead
- `>>>`, `<<<`, `|||`, `===` at the beginning of lines start to look like merge conflicts which might confuse external tools
- Backticks have a problem with users knowing how to and remembering to escape these blocks when sharing them in markdown.  Knowing the syntax (only because I've implemented a parser for it), I'm at about 50/50 on whether I properly escape.

Note:
- `"` was not considered because that can feel too familiar and users might carry over their expectations for how strings work

### Alternative 2: Extended Shebang

````rust
#!/usr/bin/env cargo
# ```cargo
# [dependencies]
# foo = "1.2.3"
# ```

fn main() {}
````
This is a variation on other options that ties itself closer to the shebang syntax.
The hope would be that we could get buy-in from other languages.
- The first line post-shebang-stripping is a hash plus 3+ backticks, then capture all content until a matching pair of backticks on a dedicated line.  This would be captured into a `#![frontmatter(info = "cargo", content = "..."]`.  `frontmatter` attribute is reserved for crate roots.  The 3+ with matching pair is a "just in case" a TOML multi-line string has that syntax in it).  Each content line must be indented to at least the same level as the first backtick.
  - Backticks are needed to know to avoid parsing `#[dependencies]` as an attribute
  - This also allows an infostring so this isn't just a cargo feature
- Future evolution: Allow `cargo` being the default `info` string
- Future evolution: Allow any `info` string with cargo checking for `content.starts_with(["cargo", "cargo,"])`
- Future evolution: Allow `frontmatter` attribute on any module

Syntactically, this avoids confusion with attributes by being stripped before lexing.
We could make this less ambiguous by using a double hash.
````rust
#!/usr/bin/env cargo
## ```cargo
## [dependencies]
## foo = "1.2.3"
## ```

fn main() {}
````

Benefits
- Visually connected to the shebang
- Has parallels to ideas outside of Rust, building on external knowledge that might exist
- Easy for cargo to parse and modify
- Can easily be leveraged by buck2, meson, etc in the future
- Maybe we can get others on board with this syntax

Downsides
- `#` prefix plus a TOML `[heading]` looks too much like a Rust `#[attribute]`.
- More syntactically heavy than the frontmatter solution
  - Visually
  - More work to type it out or copy-paste between cargo scripts and regular manifests
  - More to get wrong

If we dropped future possibilities for additional content, we could remove the opening/closing syntax,
greatly reducing the minimum syntax needed in some cases.
````rust
#!/usr/bin/env cargo
## package.edition = "2018"

fn main() {}
````

### Alternative 3: Doc-comment

```rust
#!/usr/bin/env cargo

//! ```cargo
//! [package]
//! edition = "2018"
//! ```

fn main() {
}
```

Benefits
- Parsers are available to make this work (e.g. `syn`, `pulldown-cmark`)
- Familiar syntax both to read and write.
  - When discussing with a Rust author, it was pointed out many times people preface code with a comment specifying the dependencies ([example](https://github.com/dtolnay/prettyplease#example)), this is the same idea but reusable by cargo
  - When discussing on forums, people expressed how they had never seen the syntax but instantly were able to understand it
- Depending on doc-comment style used, users may be able to edit/copy/paste the manifest without dealing with leading characters

Downsides:
- **Blocker** Either we expose `syn`s lesser parse errors or we skip errors, deferring to rustc's, but then have the wrong behavior on commands that don't invoke rustc, like `cargo metadata`
  - If we extend additional restrictions to make it more tool friendly, then we break from user expectations for how this syntax works
- When discussing with a Rust crash course teacher, it was felt their students would have a hard time learning to write these manifests from scratch
  - Having the explain the overloading of concepts to new users
  - Unpredictable location (both the doc comment and the cargo code block within it)
  - Visual clutter (where clutter is overwhelming already in Rust)
- Might be a bit complicated to do edits (translating between location within
  `toml_edit` spans to the location within `syn` spans especially with different comment styles)
- Requires pulling in a full markdown parser to extract the manifest
  - Incorrectly formatted markdown would lead to a missing manifest and confusing error messages at best or silent incorrect behavior at worse

### Alternative 4: Attribute

```rust
#!/usr/bin/env cargo

#![cargo(manifest = r#"
[package]
edition = "2018"
"#)]

fn main() {
}
```
- `cargo` could register this attribute or `rustc` could get a generic `metadata` attribute
- As an alternative, `manifest` could a less stringly-typed format but that
  makes it harder for cargo to parse and edit, makes it harder for users to
  migrate between single and multi-file packages, and makes it harder to transfer
  knowledge and experience

Benefits
- Parsers are available to make this work (e.g. `syn`)
- Users can edit/copy/paste the manifest without dealing with leading characters

Downsides
- **Blocker** Either we expose `syn`s lesser parse errors or we skip errors, deferring to rustc's, but then have the wrong behavior on commands that don't invoke rustc, like `cargo metadata`
  - If we extend additional restrictions to make it more tool friendly, then we break from user expectations for how this syntax works
- When discussing with a Rust crash course teacher, it was felt their students would have a hard time learning to write these manifests from scratch
  - Unpredictable location (both the doc comment and the cargo code block within it)
- From talking to a teacher, users are more forgiving of not understanding the details for structure data in an unstructured format (doc comments / comments) but something that looks meaningful, they will want to understand it all requiring dealing with all of the concepts
 - The attribute approach requires explaining multiple "advanced" topics: One teacher doesn't get to teaching any attributes until the second level in his crash course series and two teachers have found it difficult to teach people raw strings
- Attributes look "scary" (and they are in some respects for the hidden stuff they do)

### Alternative 5: Regular Comment

Simple header:
```rust
#!/usr/bin/env cargo
/* Cargo.toml:
[package]
edition = "2018"
*/

fn main() {
}
```

HEREDOC:
```rust
#!/usr/bin/env cargo
/* Cargo.toml >>>
[package]
edition = "2018"
<<<
*/

fn main() {
}
```
The manifest can be a regular comment with a header.  If we were to support
multiple types of content (manifest, lockfile), we could either use multiple
comments or HEREDOC.
This does not prescribe the exact syntax used or supported comments

Benefits
- Natural to support `Cargo.lock` as well
- Without existing baggage, can use file extensions, making a firmer
  association in users minds for what is in these (for those used to `Cargo.toml`)
- Depending on the exact syntax decided on, users may be able to edit/copy/paste the manifest without dealing with leading characters

Downsides
- **Blocker** Assuming it can't be parsed with `syn` and either we need to write a
  sufficiently compatible comment parser or pull in a much larger rust parser
  to extract and update comments.
  - If we extend additional restrictions to make it more tool friendly, then we break from user expectations for how this syntax works
  - Like with doc comments, this should map to an attribute and then we'd just start the MVP with that attribute
- Unfamiliar syntax
- When discussing with a Rust crash course teacher, it was felt their students would have a hard time learning to write these manifests from scratch
  - Having the explain the overloading of concepts to new users
  - Unpredictable location (both the doc comment and the cargo code block within it)
  - Visual clutter (where clutter is overwhelming already in Rust)
- New style of structured comment for the ecosystem to support with potential
  compatibility issues, likely requiring a new edition

### Alternative 6: Macro

```rust
#!/usr/bin/env cargo

cargo! {
[package]
edition = "2018"
}

fn main() {
}
```
Benefits
- Parsers are available to make this work (e.g. `syn`)
- Users can edit/copy/paste the manifest without dealing with leading characters

Downsides
- **Blocker** Either we expose `syn`s lesser parse errors or we skip errors, deferring to rustc's, but then have the wrong behavior on commands that don't invoke rustc, like `cargo metadata`
  - If we extend additional restrictions to make it more tool friendly, then we break from user expectations for how this syntax works
- When discussing with a Rust crash course teacher, it was felt their students would have a hard time learning to write these manifests from scratch
  - Unpredictable location (both the doc comment and the cargo code block within it)
- The `cargo` macro would need to come from somewhere (`std`?) which means it is taking on `cargo`-specific knowledge
  - An unexplored direction we could go with this is a `meta!` macro (e.g. we'd need to have a format marker in it)
- A lot of tools/IDEs have problems in dealing with macros
- Free-form rust code makes it harder for cargo to make edits to the manifest

Bazel has an [import proc-macro](https://github.com/bazelbuild/rules_rust/pull/1142) but its more for simplifying the writing of `extern crate`.

### Alternative 7: Presentation Streams

```rust
#!/usr/bin/env cargo

fn main() {
}

---Cargo.toml
[package]
edition = "2018"
```
YAML allows several documents to be concatenated together variant
[presentation streams](https://yaml.org/spec/1.2.2/#323-presentation-stream)
which might seem familiar as this is frequently used in static-site generators
for adding frontmatter to pages.
What if we extended Rust's syntax to allow something similar?

Benefits
- Flexible for other content
- Users can edit/copy/paste the manifest without dealing with leading characters

Downsides
- **Blocker** Difficult to parse without assistance from something like `syn` as we'd need to distinguish what the start of a stream is vs content of a string literal
- Being a new file format (a "text tar" format), there would be a lot of details to work out, including
  - How to delineate and label documents
  - How to allow escaping to avoid conflicts with content in a documents
  - Potentially an API for accessing the document from within Rust
- Unfamiliar, new syntax, unclear how it will work out for newer users

# Prior art
[prior-art]: #prior-art

See also [Single-file scripts that download their dependencies](https://dbohdan.com/scripts-with-dependencies)
which enumerates the syntax used by different tools.

## `cargo-script` family

There are several forks of [cargo script](https://github.com/DanielKeep/cargo-script).

doc-comments
```rust
#!/usr/bin/env run-cargo-script
//! This is a regular crate doc comment, but it also contains a partial
//! Cargo manifest.  Note the use of a *fenced* code block, and the
//! `cargo` "language".
//!
//! ```cargo
//! [dependencies]
//! time = "0.1.25"
//! ```
extern crate time;
fn main() {
    println!("{}", time::now().rfc822z());
}
```
short-hand
```rust
// cargo-deps: time="0.1.25"
// You can also leave off the version number, in which case, it's assumed
// to be "*".  Also, the `cargo-deps` comment *must* be a single-line
// comment, and it *must* be the first thing in the file, after the
// hashbang.
extern crate time;
fn main() {
    println!("{}", time::now().rfc822z());
}
```

## RustExplorer

[Rust Explorer](https://users.rust-lang.org/t/rust-playground-with-the-top-10k-crates/75746)
uses a comment syntax for specifying dependencies

Example:
```rust
/*
[dependencies]
actix-web = "*"
ureq = "*"
tokio = { version = "*", features = ["full"] }
*/

use actix_web::App;
use actix_web::get;
use actix_web::HttpResponse;
use actix_web::HttpServer;
use actix_web::post;
use actix_web::Responder;
use actix_web::web;
use tokio::spawn;
use tokio::sync::oneshot;
use tokio::task::spawn_blocking;
```

## PL/Rust

Example:
```sql
CREATE OR REPLACE FUNCTION randint() RETURNS bigint LANGUAGE plrust AS $$
[dependencies]
rand = "0.8"

[code]
use rand::Rng; 
Ok(Some(rand::thread_rng().gen())) 
$$;
```

See [External Dependencies](https://github.com/tcdi/plrust/blob/main/doc/src/dependencies.md)

## YAML frontmatter

As a specialization of [YAML presentation streams](https://yaml.org/spec/1.2.2/#323-presentation-stream),
static site generators use frontmatter to embed YAML at the top of files.
Other systems have extended this for non-YAML use, like
[zola using `+++` for TOML](https://www.getzola.org/documentation/content/page/#front-matter).

## Proposed Python syntax

Currently the draft [PEP 723](https://peps.python.org/pep-0723/) proposes allowing begin/end markers inside of regular comments:

```python
# /// pyproject
# [run]
# requires-python = ">=3.11"
# dependencies = [
#   "requests<3",
#   "rich",
# ]
# ///

import requests
from rich.pretty import pprint

resp = requests.get("https://peps.python.org/api/peps.json")
data = resp.json()
pprint([(k, v["title"]) for k, v in data.items()][:10])
```

# Unresolved questions
[unresolved-questions]: #unresolved-questions

# Future possibilities
[future-possibilities]: #future-possibilities

- Support infostring attributes
  - We need to better understand use cases for how this should be extended, particularly what the syntax should be (see infostring language)
  - Some tools use comma separated attributes, some use more elaborate syntax wrapped in `{}`
  - A safe starting point could be to say that a space or comma separates the identifier from the attributes and everything after it is defined as part of the "language"
- Add support for a `#[frontmatter(info = "", content = "")]` attribute that this syntax maps to.
  - Since nothing will read this, whether we do it now or in the future will have no affect

## Multiple frontmatters

At least for cargo's use cases, the only other file that we would consider supporting is `Cargo.lock`
and we have other avenues we want to explore as future possibilities before we
even consider the idea of multiple frontmatters.

So **if** we decide we need to embed additional metadata, we have a couple of options for extending frontmatter support.

Distinct blocks, maybe with newlines

````md
---Cargo.toml
---

---Cargo.lock
---
````

Continuous blocks
````md
---Cargo.toml
---Cargo.lock
---
````

Distinct blocks is more like the source inspiration, markdown,
though has more noise, places to get things wrong, and syntax questions
(newlines).

[RFC 3502]: https://github.com/rust-lang/rfcs/pull/3502
