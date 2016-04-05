- Feature Name: `source_map`, `include_dir`
- Start Date: 2016-02-29
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

This RFC proposes two changes to the Rust compiler and Cargo in order
to better support code generators:

* Add source mapping support to the compiler that allows the compiler to
  bidirectionally associate tokens in an output rust file with one or more
  input template files.  This then will be used to report error messages in the
  original file.
* Add support to `rustc` for multiple source directories, and update Cargo
  to automatically add it's `$OUT_DIR` directory to this directory.

# Motivation
[motivation]: #motivation

[Syntex](https://github.com/serde-rs/syntex) is a convenient tool that enables
libraries like [Serde](https://github.com/serde-rs/serde) to support Rust
Nightly-style syntax extensions in Stable Rust.  Syntex is a code generator,
where it expands syntax extensions from a template Rust file into a stable Rust
file.  This then can be compiled by the Stable Rust compiler.

Unfortunately there are some major challenges to using Syntex which prevents
libraries like Serde getting wide usage.  There are three major problems with
Syntex.  First, wiring Syntex into a project results in an inconvenient amount
of boilerplate code.  It requires the following `build.rs`, that is copy-pasted
into every Serde project, which registers the Serde plugin with Syntex, and
informs Syntex which files it should be expanding:

```rust
extern crate syntex;
extern crate serde_codegen;

use std::env;
use std::path::Path;

fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();

    let src = Path::new("src/queen.rs.in");
    let dst = Path::new(&out_dir).join("queen.rs");

    let mut registry = syntex::Registry::new();

    serde_codegen::register(&mut registry);
    registry.expand("", &src, &dst).unwrap();
}
```

It also requires an unfortunate amount of macros to link in the generated
file, with a command like:

```rust
include!(concat!(env!("OUT_DIR"), "/queen.rs"));
```

Second, after a project has been Syntex-ified, it is actually inconvenient to
use in daily development because the generated files produce terrible error
messages.  This happens because error locations are reported inside the
generated file, not from within the template file.  Debugging an error then
requires opening up the generated file, finding the error, and then manually
searching the template file to find the error.

For example, a type error in `queen.rs.in` might produce this error message
that is in a file:

```
target/debug/build/test-ba65ec36dc6f8bb0/out/queen.rs:25:18: 2:23 error: mismatched types:
 expected `u64`,
    found `&'static str`
(expected u64,
    found &-ptr) [E0308]
target/debug/build/test-ba65ec36dc6f8bb0/out/queen.rs:25     let x: u64 = "foo";
                                                                          ^~~~~
```

Third, because of this difficulty with error locations, most users of Serde do
their development in Nightly Rust with the Serde plugin that is compatible with
Nightly Rust syntax extensions and gives good error locality.  Not only does
this cause more of our ecosystem to use Nightly Rust and it's unstable
features, it also requires even more inconvenient boilerplate code to make a
project compatible with Syntex and Nightly Rust plugins.  The `build.rs` from
before needs to be modified to:

```rust
#[cfg(feature = "with-syntex")]
mod with_syntex {
    extern crate syntex;
    extern crate serde_codegen;

    use std::env;
    use std::path::Path;

    pub fn main() {
        let out_dir = env::var_os("OUT_DIR").unwrap();

        let src = Path::new("src/queen.rs.in");
        let dst = Path::new(&out_dir).join("queen.rs");

        let mut registry = syntex::Registry::new();

        serde_codegen::register(&mut registry);
        registry.expand("", &src, &dst).unwrap();
    }
}

#[cfg(not(feature = "with-syntex"))]
mod with_syntex {
    pub fn main() {}
}

pub fn main() {
    with_syntex::main();
}
```

and the entry point into the library needs to be modified to:

```rust
#![cfg_attr(not(feature = "with-syntex"), feature(custom_attribute, custom_derive, plugin))]
#![cfg_attr(not(feature = "with-syntex"), plugin(serde_macros))]

extern crate serde;

#[cfg(feature = "with-syntex")]
include!(concat!(env!("OUT_DIR"), "/lib.rs"));

#[cfg(not(feature = "with-syntex"))]
include!("lib.rs.in");
```

Beyond Syntex, there are a number of other tools that work by way of code
generation:

* [ANTLR](http://www.antlr.org/)
* [Lex](http://dinosaur.compilertools.net/lex/index.html)
* [Protocol Buffers](https://developers.google.com/protocol-buffers/)
* [Thrift](https://thrift.apache.org/)
* [Yacc](http://dinosaur.compilertools.net/yacc/index.html)

It is unlikely these projects would be rewritten in Rust, and so would also be
subject to the same "reporting errors in the generated file" that Syntex has.

# Detailed design
[design]: #detailed-design

This RFC proposes two changes that will help improve Rust's code generation
story.

## Source Mapping
[source mapping]: #source-mapping

Because of the challenges debugging generated code, this RFC proposes that Rust
be extended to produce and consume a file that contains a mapping from the
input generated file to the output Rust file.  Lets consider using the rustc
pretty printer to convert one Rust source into another.  For example, consider
a simple crate that's made up of two files.  `queen.rs`:

```rust
pub mod love;

pub struct Person { ... }
```

and it's submodule, `love.rs`:

```rust
use super::Person;

pub fn find(people: &[Person]) -> Option<&Person> {
    people.find(|person| person.lovable())
}
```

The pretty printer produces a single output file that merges the two files
together, and would look something like this:

```
pub mod love {
    use super::Person;

    pub fn find(people: &[Person]) -> Option<&Person> {
        people.find(|person| person.lovable())
    }
}

pub struct Person { ... }
```

By itself, this process loses the information that the module `love`
came from the file `love.rs`.  To avoid that, the pretty printer will
instead generate a file, `queen.rs.map`, that conceptually contains the
following mapping:

| dst line | dst col | source file | src line | src col | token           |
| -------- | ------- | ----------- | -------- | ------- | --------------  |
| 0        | 0       | "queen.rs"  | 0        | 0       | pub             |
| 0        | 4       | "queen.rs"  | 0        | 4       | mod             |
| 0        | 8       | "queen.rs"  | 0        | 8       | love\_canidates |
| 0        | 24      | "queen.rs"  | 0        | 24      | ;               |
| 2        | 0       | "love.rs"   | 0        | 0       | use             |

This mapping will then be used by the Rust compiler during parsing to map
tokens to their original location.

Rather than Rust developing their own custom mapping file, this RFC proposes
that Rust adopt the
[JavaScript Source Map](https://source-map.github.io/)
[v3 specification](https://docs.google.com/document/d/1U1RGAehQwRypUTovF1KRlpiOFze0b-_2gc6fAH0KY0k/edit)
This would be done in order to simplify the implementation
since there are already a number of X-to-JavaScript Source Map generators.

## Source Search Paths
[paths]: #paths

In order to cut down on the boilerplate necessary including generated source into
a crate, the Rust Compiler should be extended to support the concept of source
search paths, similar to GCC's `-I some-path` option, as in
`rustc -I src -I $OUT_DIR/src`.  When Rust needs to look for some file, it will
check first in the current directory, then it will iterate through each search
path until the file is found.

Cargo would then be updated to add the `$OUT_DIR` first in the search path
order, which would allow generated files to be referenced with `mod queen;`
instead of `include!(...)`.

The exceptions to this are the `#[path="..."]`, `include!(...)` and
related macros, which in order to remain backwards compatible, must be relative
to the Rust entry point.  This means that if there is a directory like:

```
src/lib.rs
src/submodule/foo.rs
```

The file `src/lib.rs` could contain:

```rust
include!("submodule/queen.rs");

#[path = "submodule/queen.rs")
mod another_queen;
```

# Drawbacks
[drawbacks]: #drawbacks

* Nick Fitzgerald (@fitzgen), the coauthor of the Source Map specification,
  has written two articles
  ([1](http://fitzgeraldnick.com/weblog/55/),
  [2](http://fitzgeraldnick.com/weblog/62/))
  about the limitations of Source Maps at encoding things like scoping
  information.  He attempts to address this in this
  [RFC](https://github.com/source-map/source-map-rfc/pull/4), which adds
  DWARF-like records to Source Maps.  For Syntex, this doesn't matter since
  it's source language is also Rust, but if a language like
  [dyon](https://github.com/PistonDevelopers/dyon) grow the ability to generate
  Rust code, they might want to encode their scoping information in the Source
  Map.
* WebAssembly is probably going to adopt a different approach to source mapping
  ([1](https://github.com/WebAssembly/design/issues/602),
  [2](https://github.com/WebAssembly/spec/issues/258),
  [3](https://github.com/WebAssembly/design/blob/master/Tooling.md)),
  but it hasn't been spec-ed out yet.  How can we avoid being locked into a
  format with a potentially short lifespan?
 * One option is for the compiler to just ignore old Source Map files.  Since
   this is mainly used for debug info, this would just fail gracefully back to
   error locations in the generated file.

# Alternatives
[alternatives]: #alternatives

Instead of source maps we could embed the span in the source itself through a
macro that's interpreted by the parser.  Some options for this include:

* `#line "foo.rs" 1 2` in the style of CPP.
* `set_line!("foo.rs", 1, 1);`
* `set_source_span!("foo.rs", 1, 1);`
* `set_location!("foo.rs", 1, 1);`

In order to make this work with Syntex, the pretty printer would have to grow
the ability to physically output tokens in the same line and column so that
each token wouldn't need to be annotated with the correct positioning.

Another option would be to just adopt the [DWARF](...) debuginfo format,
but it would be much more complicated to produce.

# Unresolved questions
[unresolved]: #unresolved-questions

* If the macro-form for source mapping is used, it will see little use outside
  of generated code, and doesn't necessarily need to be added to the default
  namespace. Could it instead be placed somewhere to be used with
  `#[macro_use]`?
* Is it actually backwards incompatible to have `#[path="..."]` find paths in
  the search paths?
* Do we need to track column information?
