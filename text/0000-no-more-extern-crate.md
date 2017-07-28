- Feature Name: infer-extern-crate
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

This RFC reduces redundant boilerplate when including external crates.
`extern crate` declarations will be inferred from the arguments passed to `rustc`.
With this change, projects using Cargo
(or other build systems using the same mechanism)
will no longer have to specify `extern crate`:
dependencies added to `Cargo.toml` will be automatically imported.
Projects which require more flexibility can still use manual `extern crate`
and will be unaffected by this RFC.

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
Allowing the omission of the redundant `extern crate` syntax contributes to the
roadmap goals of
[improving Rust's ergonomics](https://github.com/rust-lang/rust-roadmap/issues/17)
and
[providing easy access to high-quality crates.](https://github.com/rust-lang/rust-roadmap/issues/9)

# Guide-Level Explanation
[guide]: #guide

When you add a dependency to your `Cargo.toml`, it is immediately usable within
the source of your crate:

```
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

External crates can be passed to the rust compiler using the
`--extern CRATE_NAME=PATH` flag.
For example, `cargo build`-ing a crate `my_crate` with a dependency on `rand`
results in a call to rustc that looks something like
`rustc --crate-name mycrate src/main.rs --extern rand=/path/to/librand.rlib ...`.

When an external crate is specified this way,
the crate will automatically brought into scope as if an
`extern crate name_of_crate;`
declaration had been added to the current crate root.
This behavior won't occur when including a library using the `-l`
or `-L` flags.

We will continue to support the current `extern crate` syntax,
both for backwards compatibility and to enable users who want to use manual
`extern crate` in order to have more fine grained control-- say, if they wanted
to import an external crate only inside an inner module.
No automatic import will occur if an `extern crate` declaration for the same
external dependency appears anywhere within the crate.
For example, if `rand = "0.3"` is listed as a dependency in Cargo.toml
and `extern crate rand;` appears somewhere in the crate being compiled,
then no implicit `extern crate rand;` will be added.
If Cargo.toml were to also list another dependency, `log = "0.3"`, and no
`extern crate log;` appears in the crate being compiled,
then an `extern crate log;` would be implicitly added.

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
However, as specified in
[RFC 1561](https://github.com/rust-lang/rfcs/blob/master/text/1561-macro-naming.md#importing-macros),
macros 2.0 will no longer require `#[macro_use]`, replacing it with
normal `use` declarations, for which no `extern crate` is required.

One final remaining use case of `extern crate` syntax is for aliasing, i.e.
`extern crate foo as bar;`. There is no way to infer aliasing information from
Cargo.toml, so aliased crates will need to be specied using `extern crate`
syntax.

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
