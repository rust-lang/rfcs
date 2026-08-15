- Feature Name: recommended-bin-packages
- Start Date: 2023-01-04
- RFC PR: [rust-lang/rfcs#3383](https://github.com/rust-lang/rfcs/pull/3383)

# Summary
[summary]: #summary

Add an optional `recommended-bin-packages` field to the `[package]`
section of `Cargo.toml`, to enable package authors to point out related
binary packages in the error message Cargo users get when attempting to
`cargo install` a package without binaries.

# Motivation
[motivation]: #motivation

Command-line tools written in Rust are often published in packages named
different than the command, since that name is already occupied by a
related library package, for instance:

* the `diesel` command is provided by the `diesel_cli` binary package,
  that depends on the `diesel` library package

* the `wasm-bindgen` command is provided by the `wasm-bindgen-cli`
  binary package, which is different from the `wasm-bindgen` library package

While such a setup has several benefits, it currently leads to a
user experience problem with Cargo: To obtain a command, users will be
tempted to run `cargo install <command>`, which will however inevitably fail:

```
$ cargo install diesel
error: there is nothing to install in `diesel v2.0.3`, because it has no binaries
`cargo install` is only for installing programs, and can't be used with libraries.
To use a library crate, add it as a dependency in a Cargo project instead.
```

The idea of this RFC is that the `Cargo.toml` of such
a library-only package could specify for instance:

```toml
[package]
name = "diesel"
# ...
recommended-bin-packages = ["diesel-cli"]
```

which could be picked up by Cargo in order to additionally include
a note such as the following in the above error message:

> The developers of `diesel` suggest you may want to install `diesel-cli` instead.

resulting in a more seamless user experience.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The following is written as if it was part of the [manifest page] of the Cargo Book.

## The `recommended-bin-packages` field

The `recommended-bin-packages` field is an array of names of related
binary packages that have been published in the same registry.

```toml
[package]
name = "foobar"
# ...
recommended-bin-packages = ["foobar-cli"]
```

Specifying this field for a library-only package, enables Cargo to print
a more user-friendly error message for `cargo install`, for example:

```
$ cargo install foobar
error: there is nothing to install in `foobar v1.2.3`, because it has no binaries
`cargo install` is only for installing programs, and can't be used with libraries.
To use a library crate, add it as a dependency in a Cargo project instead.

The developers of `foobar` suggest you may want to install `foobar-cli` instead.
```

(Notice the last line in the above output, which is enabled by the
`recommended-bin-packages` field.)

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The `cargo install` command has already parsed the `Cargo.toml` manifest
file when it prints this error message, so it would simply have to
additionally check for this new field when printing the error message.

`cargo publish` should assert that the referenced
packages already exist and have not been yanked.

# Drawbacks
[drawbacks]: #drawbacks

* It introduces yet another manifest `field`.

* The packages referenced by this field could become abandoned, out-of-date or yanked
  (although the referencing and referenced package will presumably often be published
  by the same person/group, in which case this can be simply avoided).

* Like any other manifest field, updating the field requires you to publish
  a new version of the package (a patch version bump suffices).

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The problem addressed by this RFC can be sidestepped by publishing the
library along with the binary in a single package. This does however come
with two disadvantages:

* Cargo currently doesn't support artifact-specific dependencies
  (although that may change, see [RFC 2887] & [RFC 3374]).

* You would have to bump the library version each time you want to
  publish a new version of the binary. If you want independently
  incrementable versions for your library and your binary, you have to
  publish them in separate packages.

The problem could also be sidestepped by publishing the command in a
package with the same name as the command and using a different name for
the library package. (While `cargo add` currently does not fail for
binary-only packages that could very well be addressed without an RFC.)
A disparity between package name and library name again comes with its
own disadvantages, e.g. Rust users often expect libraries published
on crates.io to be documented at docs.rs/{lib_name}.

So while these two alternatives have their own disadvantages, they are
also generally not an option for projects that have already been published
using the described naming convention, since converting a library-only
package to a binary-only package is bound to result in confusion.

The proposed field provides a simple solution to a rather annoying
problem and could be easily applied to existing projects using this
common naming convention to solve the described user experience problem.

# Prior art
[prior-art]: #prior-art

Most other package managers do not have such a problem since their
install command does not mandate that the package contains binaries.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

* Is `recommended-bin-packages` a good name for the field?

# Future possibilities
[future-possibilities]: #future-possibilities

* crates.io and/or lib.rs could additionally link packages referenced via
  this new field on their library web pages

* Clippy could gain a lint to check that the referenced packages actually
  exist, have not been yanked and actually contain binary targets.


[manifest page]: https://doc.rust-lang.org/cargo/reference/manifest.html
[RFC 2887]: https://github.com/rust-lang/rfcs/pull/2887
[RFC 3374]: https://github.com/rust-lang/rfcs/pull/3374
