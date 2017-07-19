- Feature Name: infer-extern-crate
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Infer `extern crate` declarations from the arguments passed to `rustc`.
With this change, projects using Cargo will no longer have to specify
`extern crate`: adding dependencies to `Cargo.toml` will result in the
module being automatically imported.

# Motivation
[motivation]: #motivation

One of the principles of Rust is that using external crates should be as
easy and natural as using the standard library.
This allows the standard library to be kept small, and allows mature, standard
solutions to be developed by the community.

Currently, however, external crates must be specified twice: once in a build
system such as Cargo and again in the source code using `extern crate`.
When external dependencies are conditional (`cfg`) upon feature flags or the
target platform, the conditional logic must appear in both `Cargo.toml` and
the `extern crate` declarations.

This duplication causes unnecessary effort and results in one more opportunity
for mistakes when working with conditionally-enabled dependencies.

# Guide-Level Explanation
[guide]: #guide

When you add a dependency to your `Cargo.toml`, it is immediately usable within
the source of your crate:

```toml
# Cargo.toml:
name = "my_crate"
version = "0.1.0"
authors = ["Me" <me@mail.com>]

[dependencies]
rand = "0.3"
```

```rust
// src/main.rs:

fn main() {
    println!"A random character: {}", rand::random::<char>());
}
```

# Reference-Level Explanation
[reference]: #reference

External crates are passed to the rust compiler using the `-L` or `-l` flags.
When an external crate is specified this way, an `extern crate name_of_crate;`
declaration will be added to the current crate root.

However, for backwards-compatibility with legacy `extern crate` syntax, no
automatic import will occur if an `extern crate` declaration for the same
external dependency appears anywhere within the crate.

Additionally, items such as modules, types, or functions that conflict with
the names of implicitly imported crates will cause the implicit `extern crate`
declaration to be removed.
Note that this is different from the current behavior of the
implicitly-imported `std` module.
Currently, creating a root-level item named `std` results in a name conflict
error. For consistency with other crates, this error will be removed.
Creating a root-level item named `std` will prevent `std` from being included,
and will trigger a warning.

It will still be necessary to use the `extern crate` syntax when using
`#[macro_use]` to import macros from a crate. This is necessary in order to
prevent macros from being automatically brought into scope and changing the
behavior of existing code.

# Alternatives
[alternatives]: #alternatives

- Don't do this.
- Specify external dependencies using only `extern crate`, rather than only
`Cargo.toml`, by using `extern crate foo = "0.2";` or similar. This would
require either `Cargo` or `rustc` to first scan the source before determining
the build dependencies of the existing code, a system which requires fairly
tight coupling between a build system and `rustc`, and which would almost
certainly interact poorly with third-party build systems.

# Unresolved questions
[unresolved]: #unresolved-questions

- What interactions does this have with future procedural macros?
- Should we lint/warn on local items shadowing implicitly imported crates?
It seems like a useful warning, but it's also a potential
backwards-compatibility hazard for crates which previously depended on a
crate, didn't import it with `extern crate`, and had a root-level item with
an overlapping name (although this seems like an extreme edge case).
- How can we prevent having to use `extern crate` whenever we need to import
a macro? In a future "macros 2.0" world it may be possible to import macros
using some other syntax, which could resolve this issue.
