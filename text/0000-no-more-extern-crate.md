- Feature Name: immediately-usable-extern-crates
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

This RFC reduces redundant boilerplate when including external crates.
With this change, projects using Cargo
(or other build systems using the same mechanism)
will no longer have to specify `extern crate`:
dependencies added to `Cargo.toml` will be automatically `use`able.
We continue to support `extern crate` for backwards compatibility
with the option of phasing it out in future Rust epochs.

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
the source of your crate. For example, imagine that you needed to print a random
character. You'd start by adding the `rand` crate to your `Cargo.toml`:

```
# Cargo.toml:
name = "my_crate"
version = "0.1.0"
authors = ["Me" <me@mail.com>]

[dependencies]
rand = "0.3"
```

And then you can immediately `use` the crate:

```rust
// src/main.rs:
use rand;

fn main() {
    let c: char = rand::random();
    println!("A random character: {}", c);
}
```

Alternatively, we can `use` just the specific function we need:

```rust
use rand::random;

fn main() {
    let c: char = random();
    println!("A random character: {}", c);
}
```

# Reference-Level Explanation
[reference]: #reference

External crates can be passed to the rust compiler using the
`--extern CRATE_NAME=PATH` flag.
For example, `cargo build`-ing a crate `my_crate` with a dependency on `rand`
results in a call to rustc that looks something like
`rustc --crate-name mycrate src/main.rs --extern rand=/path/to/librand.rlib ...`.

When an external crate is specified this way, it will be automatically
available to any module in the current crate through `use` statements or
absolute paths (e.g. `::rand::random()`). It will _not_ be automatically
imported at root level as happens with current `extern crate`.
None of this behavior will occur when including a library using the `-l`
or `-L` flags.

We will continue to support the current `extern crate` syntax for backwards
compatibility. `extern crate foo;` will behave just like it does currently.
Writing `extern crate foo;` will not affect the availability of `foo` in
`use` and absolute paths as specified by this RFC.

Additionally, items such as modules, types, or functions that conflict with
the names of implicitly imported crates will result in a warning and will
require the external crate to be brought in manually using `extern crate`.
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

One remaining use case of `extern crate` syntax is for aliasing, i.e.
`extern crate foo as bar;`. In order to support aliasing, a new "alias" key
will be added to the `Cargo.toml` format.
Users who want to use the `rand` crate but call it `random` instead can now
write `rand = { version = "0.3", alias = "random" }`.

When compiling, an external crate is only included if it is used
(through either `extern crate`, `use`, or absolute paths).
This prevents unnecessary inclusion of crates when compiling crates with
both `lib` and `bin` targets, or which bring in a large number of possible
dependencies (such as
[the current Rust Playground](https://users.rust-lang.org/t/the-official-rust-playground-now-has-the-top-100-crates-available/11817)).
It also prevents `no_std` crates from accidentally including `std`-using
crates.

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
- `extern crate foo` has linking side effects even if `foo` isn't visibly
used from Rust source. After this change, `use foo;` would have similar
effects. This seems potentially undesirable-- what's the right way of handling
crates which are brough in only for their side effects?
