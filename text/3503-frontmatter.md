- Feature Name: `frontmatter`
- Start Date: 2023-09-26
- RFC PR: [rust-lang/rfcs#3503](https://github.com/rust-lang/rfcs/pull/3503)
- Rust Issue: [rust-lang/cargo#12207](https://github.com/rust-lang/cargo/issues/12207)


# Summary
[summary]: #summary

Add a frontmatter syntax to Rust as a way for [cargo to have manifests embedded in source code](https://github.com/rust-lang/rfcs/pull/3502):
````rust
#!/usr/bin/env cargo
```cargo
[dependencies]
clap = { version = "4.2", features = ["derive"] }
```

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

["cargo script"](https://github.com/rust-lang/rfcs/pull/3502) is in need of a syntax for embedding manifests in source.
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

We are carrying this concept over to Rust with a twist: using fence code blocks which
will be familiar to Rust developers when documenting their code:
````rust
#!/usr/bin/env cargo
```cargo
[dependencies]
clap = { version = "4.2", features = ["derive"] }
```

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

As we work to better understand how tool authors will want to use frontmatter, we are restricting it to just the `cargo` infostring.
This means users will only be exposed to this within the concept of ["cargo script"](https://github.com/rust-lang/rfcs/pull/3502).

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

When parsing Rust code, after stripping the shebang (`#!`), rustc will strip a fenced code block:
- Must be immediately at the top (after shebang stripping), meaning no blank lines
- Opens with 3+ backticks and "cargo" followed by a newline
  - As we aren't supporting an arbitrarily nested file format (though may show up in one), we likely don't need the flexibility
  - We are prioritizing on "one right way to do it" to make it easier to learn to write and to read a variety of files.
- All content is ignored until the same number of backticks is found at the start of a line.
  It is an error to have anything besides spaces and tabs between the backticks and the newline.
  - Unlike commonmark, it is an error to not close the fenced code block seeing to detect problems earlier in the process seeing as the primary content is what comes after the fenced code block

As cargo will be the first step in the process to parse this,
the responsibility for high quality error messages will largely fall on cargo.

# Drawbacks
[drawbacks]: #drawbacks

- A new concept for Rust syntax, adding to overall cognitive load
- Ecosystem tooling updates to deal with new syntax
- When sharing in markdown documents (e.g. GitHub issues), requires people escape markdown code fences with an extra backtick which they are likely not used to doing (or aware even exists)

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Within this solution,
we considered starting with only allowing this in the root `mod` (e.g. `main.rs`)
but decided to allow it in any file mostly for ease of implementation.
Like with Python, this allows any file in a package (with the correct deps and `mod`s) to be executed, allowing easier interacting experiences in verifying behavior.

As for the hard-coded infostring used by cargo, that is a decision for [RFC 3502](https://github.com/rust-lang/rfcs/pull/3502).

## Syntax

When choosing a syntax for embedding manifests in source files, our care-abouts are
- How obvious it is for new users when they see it
- How easy it is for newer users to remember it and type it out
- How machine editable it is for `cargo add` and friends
- Needs to be valid Rust code for quality of error messages, etc
- Leave the door open in case we want to reuse the syntax for embedded lockfiles
- Leave the door open for single-file `lib`s

### Fenced Code Block Frontmatter

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

We are intentionally supporting only a subset of commonmark code fences.
Markdown, like HTML, is meant to always be valid which is different than Rust syntax.
Differences include:
- backticks but not tilde's
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
- When sharing in markdown documents (e.g. GitHub issues), requires people escape markdown code fences with an extra backtick which they are likely not used to doing (or aware even exists)
- Familiar syntax in an unfamiliar use may make users feel unsettled, unsure how to proceed (what works and what doesn't).
- If viewed from the lens of a comment, it isn't a variant of comment syntax like doc-comments

### Alternative 1: Doc-comment

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
- When discussing with a Rust crash course teacher, it was felt their students would have a hard time learning to write these manifests from scratch
  - Having the explain the overloading of concepts to new users
  - Unpredictable location (both the doc comment and the cargo code block within it)
  - Visual clutter (where clutter is overwhelming already in Rust)
- Might be a bit complicated to do edits (translating between location within
  `toml_edit` spans to the location within `syn` spans especially with different comment styles)
- Either we expose `syn`s lesser parse errors or we skip errors, deferring to rustc's, but then have the wrong behavior on commands that don't invoke rustc, like `cargo metadata`
- Requires pulling in a full markdown parser to extract the manifest
  - Incorrectly formatted markdown would lead to a missing manifest and confusing error messages at best or silent incorrect behavior at worse

### Alternative 2: Macro

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
- When discussing with a Rust crash course teacher, it was felt their students would have a hard time learning to write these manifests from scratch
  - Unpredictable location (both the doc comment and the cargo code block within it)
- The `cargo` macro would need to come from somewhere (`std`?) which means it is taking on `cargo`-specific knowledge
  - An unexplored direction we could go with this is a `meta!` macro (e.g. we'd need to have a format marker in it)
- A lot of tools/IDEs have problems in dealing with macros
- Free-form rust code makes it harder for cargo to make edits to the manifest
- Either we expose `syn`s lesser parse errors or we skip errors, deferring to rustc's, but then have the wrong behavior on commands that don't invoke rustc, like `cargo metadata`

### Alternative 3: Attribute

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
- When discussing with a Rust crash course teacher, it was felt their students would have a hard time learning to write these manifests from scratch
  - Unpredictable location (both the doc comment and the cargo code block within it)
- From talking to a teacher, users are more forgiving of not understanding the details for structure data in an unstructured format (doc comments / comments) but something that looks meaningful, they will want to understand it all requiring dealing with all of the concepts
 - The attribute approach requires explaining multiple "advanced" topics: One teacher doesn't get to teaching any attributes until the second level in his crash course series and two teachers have found it difficult to teach people raw strings
- Attributes look "scary" (and they are in some respects for the hidden stuff they do)
- Either we expose `syn`s lesser parse errors or we skip errors, deferring to rustc's, but then have the wrong behavior on commands that don't invoke rustc, like `cargo metadata`


### Alternative 4: Presentation Streams

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
- Difficult to parse without assistance from something like `syn` as we'd need to distinguish what the start of a stream is vs content of a string literal
- Being a new file format (a "text tar" format), there would be a lot of details to work out, including
  - How to delineate and label documents
  - How to allow escaping to avoid conflicts with content in a documents
  - Potentially an API for accessing the document from within Rust
- Unfamiliar, new syntax, unclear how it will work out for newer users

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
- Unfamiliar syntax
- When discussing with a Rust crash course teacher, it was felt their students would have a hard time learning to write these manifests from scratch
  - Having the explain the overloading of concepts to new users
  - Unpredictable location (both the doc comment and the cargo code block within it)
  - Visual clutter (where clutter is overwhelming already in Rust)
- New style of structured comment for the ecosystem to support with potential
  compatibility issues, likely requiring a new edition
- Assuming it can't be parsed with `syn` and either we need to write a
  sufficiently compatible comment parser or pull in a much larger rust parser
  to extract and update comments.
  - Like with doc comments, this should map to an attribute and then we'd just start the MVP with that attribute

### Alternative 6: Static-site generator frontmatter

```rust
#!/usr/bin/env cargo
---
[package]
edition = "2018"
---

fn main() {
}
```
This is a subset/specialization of YAML presentation streams that mirrors people's experience with static site generators:
- The first line post-shebang-stripping is 3+ dashes, then capture all content until a matching pair of dashes on a dedicated line.  This would be captured into a `#![frontmatter = ""]`.  `frontmatter` attribute is reserved for crate roots.  The 3+ with matching pair is a "just in case" a TOML multi-line string has that syntax in it)
- Future evolution: Allow a markdown-like infostring on the frontmatter opening dashes to declare the format with `cargo` being the default
- Future evolution: Allow `frontmatter` attribute on any module

Benefits
- Visually/syntactically lightweight
- Has parallels to ideas outside of Rust, building on external knowledge that might exist
- Easy for cargo to parse and modify
- Can easily be leveraged by buck2, meson, etc in the future
- Users can edit/copy/paste the manifest without dealing with leading characters

Downsides
- Too general that people might abuse it
- We've extended the frontmatter syntax, undoing some of the "familiarity" benefit
- People are used to YAML going in frontmatter (though some systems allow other syntaxes)
- Doesn't feel very rust-like

### Alternative 7: Extended Shebang

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

Benefits
- Visually connected to the shebang
- Has parallels to ideas outside of Rust, building on external knowledge that might exist
- Easy for cargo to parse and modify
- Can easily be leveraged by buck2, meson, etc in the future
- Maybe we can get others on board with this syntax

Downsides
- More syntactically heavy than the frontmatter solution
  - Visually
  - More work to type it out / copy-paste
  - More to get wrong
- Requires users to deal with leading characters when editing/copying/pasting the manifest

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

- Treat `cargo` as the default infostring
  - Since this is a one-way door, we want to make sure we are confident that that is the right default / policy
- Support more infostring languages
  - We need to better understand use cases for how this should be extended
  - Hard coding it for now is much like how rustc has hard coded support for `#[rustdoc]` or `clippy`/`rustdoc` lint categories
- Support infostring attributes
  - We need to better understand use cases for how this should be extended
- Loosen the code-fence syntax, like allowing newlines
- Add support for a `#[frontmatter(info = "", content = "")]` attribute that this syntax maps to.
  - Since nothing will read this, whether we do it now or in the future will have no affect

## Multiple frontmatters

At least for cargo's use cases, the only other file that we would consider supporting is `Cargo.lock`
and we have other avenues we want to explore as future possibilities before we
even consider the idea of multiple frontmatters.

So **if** we decide we need to embed additional metadata, we have a couple of options for extending frontmatter support.

Distinct blocks, maybe with newlines

````md
```Cargo.toml
```

```Cargo.lock
```
````

Continuous blocks
````md
```Cargo.toml
```Cargo.lock
```
````

Distinct blocks is more like the source inspiration, markdown,
though has more noise, places to get things wrong, and syntax questions
(newlines).
