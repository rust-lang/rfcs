- Feature Name: `build-std-explicit-dependencies`
- Start Date: 2025-06-05
- RFC PR: [rust-lang/rfcs#3875](https://github.com/rust-lang/rfcs/pull/3875)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary
[summary]: #summary

Allow users to add explicit dependencies on standard library crates in the
`Cargo.toml`. This enables Cargo to determine which standard library crates are
required by the crate graph without `build-std.crates` being set and for
different crates to require different standard library crates.

**This RFC is is part of the [build-std project goal] and a series of build-std
RFCs:**

1. build-std context ([rfcs#3873])
    - [Background][rfcs#3873-background]
    - [History][rfcs#3873-history]
    - [Motivation][rfcs#3873-motivation]
2. `build-std="always"` ([rfcs#3874])
    - [Proposal][rfcs#3874-proposal]
    - [Rationale and alternatives][rfcs#3874-rationale-and-alternatives]
    - [Unresolved questions][rfcs#3874-unresolved-questions]
    - [Future possibilities][rfcs#3874-future-possibilities]
3. Explicit standard library dependencies (this RFC)
    - [Proposal][proposal]
    - [Rationale and alternatives][rationale-and-alternatives]
    - [Unresolved questions][unresolved-questions]
    - [Future possibilities][future-possibilities]
4. `build-std="compatible"` (RFC not opened yet)
5. `build-std="match-profile"` (RFC not opened yet)

[build-std project goal]: https://rust-lang.github.io/rust-project-goals/2025h2/build-std.html

[rfcs#3873]: https://github.com/rust-lang/rfcs/pull/3873
[rfcs#3873-proposal]: https://github.com/davidtwco/rfcs/blob/build-std-part-one-context/text/3873-build-std-context.md#proposal
[rfcs#3873-background]: https://github.com/davidtwco/rfcs/blob/build-std-part-one-context/text/3873-build-std-context.md#background
[rfcs#3873-history]: https://github.com/davidtwco/rfcs/blob/build-std-part-one-context/text/3873-build-std-context.md#history
[rfcs#3873-motivation]: https://github.com/davidtwco/rfcs/blob/build-std-part-one-context/text/3873-build-std-context.md#motivation
[rfcs#3873-dependencies]: https://github.com/davidtwco/rfcs/blob/build-std-part-one-context/text/3873-build-std-context.md#dependencies

[rfcs#3874]: https://github.com/rust-lang/rfcs/pull/3874
[rfcs#3874-proposal]: https://github.com/davidtwco/rfcs/blob/build-std-part-two-always/text/3874-build-std-always.md#proposal
[rfcs#3874-rationale-and-alternatives]: https://github.com/davidtwco/rfcs/blob/build-std-part-two-always/text/3874-build-std-always.md#rationale-and-alternatives
[rfcs#3874-unresolved-questions]: https://github.com/davidtwco/rfcs/blob/build-std-part-two-always/text/3874-build-std-always.md#unresolved-questions
[rfcs#3874-future-possibilities]: https://github.com/davidtwco/rfcs/blob/build-std-part-two-always/text/3874-build-std-always.md#future-possibilities

## Motivation
[motivation]: #motivation

This RFC builds on a large collection of prior art collated in the
[`build-std-context`][rfcs#3873-proposal] RFC. It does not directly address the
main [rfcs#3873-motivation] it identifies but supports later proposals.

The main motivation for this proposal is to support future extensions to
build-std which allow public/private standard library dependencies or enabling
features of the standard library. Allowing the standard library to behave
similarly to other dependencies also reduces user friction and can improve build
times.

## Proposal
[proposal]: #proposal

Users can now optionally declare explicit dependencies on the standard library
in their `Cargo.toml` files ([?][rationale-why-explicit-deps]):

```toml
[package]
name = "hello_world"
version = "0.1.0"
edition = "2024"

[dependencies]
std = { builtin = true }
```

`builtin` is a new source of dependency, like registry dependencies (with the
`version` key and optionally the `registry` key), `path` dependencies or `git`
dependencies. `builtin` can only be set to `true` and cannot be combined with
any other dependency source for a given dependency
([?][rationale-builtin-other-sources]).

`builtin` can only be used with crates named `core`, `alloc` or `std`
([?][rationale-no-builtin-other-crates]) on stable. This set could be expanded
with new crates in future.

Use with any other crate name is gated on a perma-unstable `cargo-feature`
([?][rationale-unstable-builtin-crates]). If a builtin dependency on a unstable
crate name exists but is not used due to cfgs, then Cargo will still require the
Cargo feature.

> [!NOTE]
>
> Explicit dependencies are passed to rustc without the `noprelude` modifier
> ([?][rationale-explicit-noprelude]). When adding an explicit dependency, users
> may need to adjust their code (removing extraneous `extern crate` statements
> or root-relative paths, like `::std`).

Crates without an explicit dependency on the standard library now have a
implicit dependency ([?][rationale-no-migration]) on that target's default set
of standard library crates (see
[build-std-always][rfcs#3874-standard-library-crate-stability]). Any explicit
`builtin` dependency present in any dependency table will disable the implicit
dependencies.

> [!NOTE]
>
> Implicit dependencies are passed to rustc with the `noprelude` modifier to
> ensure backwards compatibility as in
> [`build-std=always`][rfcs#3874-noprelude].

When a `std` dependency is present an additional implicit dependency on the
`test` crate is added for crates that are being tested with the default test
harness. The `test` crate's name, but not its interface, will be stabilised so
Cargo can refer to it.

crates.io will accept crates published which have `builtin` dependencies.

Standard library dependencies can be marked as `optional` and be enabled
conditionally by a feature in the crate:

```toml
[package]
name = "hello_world"
version = "0.1.0"
edition = "2024"

[dependencies]
std = { builtin = true, optional = true }
core = { builtin = true }

[features]
default = ["std"]
std = ["dep:std"]
```

If there is an optional dependency on the standard library then Cargo will
validate that there is at least one non-optional dependency on the standard
library (e.g. an optional `std` and non-optional `core` or `alloc`, or an
optional `alloc` and non-optional `core`). `core` cannot be optional. For
example, the following example will error as it could result in a build without
`core` (if the `std` feature were disabled):

```toml
[package]
name = "hello_world"
version = "0.1.0"
edition = "2024"

[dependencies]
std = { builtin = true, optional = true }
# error: must have a non-optional dependency on core

[features]
default = ["std"]
std = ["dep:std"]
```

However, in this example, a build for the `x86-64-pc-windows-gnu` target would
have an explicit dependency on `alloc` (and indirectly on `core`), while a build
for any other target would have implicit dependencies on `std`, `alloc` and
`core`:

```toml
[package]
name = "hello_world"
version = "0.1.0"
edition = "2024"

[dependencies]
# implicit deps on `core`, `alloc` and `std` unless target='x86_64-pc-windows-gnu'

[target.x86_64-pc-windows-gnu.dependencies]
alloc.builtin = true
```

Dependencies with `builtin = true` cannot be renamed with the `package` key
([?][rationale-package-key]). It is not possible to perform source replacement
on the `builtin` source using the `[source]` Cargo config table
([?][rationale-source-replacement]), and nor is it possible to override
`builtin` dependencies with the `[replace]` sections or `paths` overrides
([?][rationale-overriding-builtins]), though [patching][patches] is permitted.

Dependencies with `builtin = true` can be specified as platform-specific
dependencies:

```toml
[target.'cfg(unix)'.dependencies]
std = { builtin = true}
```

Implicit and explicit standard library dependencies are added to `Cargo.lock`
files ([?][rationale-cargo-lock]).

> [!NOTE]
>
> A new version of the `Cargo.lock` file will be introduced to add support for
> packages with a `builtin` source:
>
> ```toml
> [[package]]
> name = "std"
> version = "0.0.0"
> source = "builtin"
> ```
>
> The package version of `std`, `alloc` and `core` will be fixed at `0.0.0`. The
> optional lockfile fields `dependencies` and `checksum` will not be present for
> `builtin` dependencies.

*See the following sections for rationale/alternatives:*

- [*Why explicitly declare dependencies on the standard library in `Cargo.toml`?*][rationale-why-explicit-deps]
- [*Why disallow builtin dependencies to be combined with other sources?*][rationale-builtin-other-sources]
- [*Why disallow builtin dependencies on other crates?*][rationale-no-builtin-other-crates]
- [*Why unstably allow all names for `builtin` crates?*][rationale-unstable-builtin-crates]
- [*Why not use `noprelude` for explicit `builtin` dependencies?*][rationale-explicit-noprelude]
- [*Why not require builtin dependencies instead of supporting implicit ones?*][rationale-no-migration]
- [*Why disallow renaming standard library dependencies?*][rationale-package-key]
- [*Why disallow source replacement on `builtin` packages?*][rationale-source-replacement]
- [*Why add standard library dependencies to Cargo.lock?*][rationale-cargo-lock]

*See the following sections for relevant unresolved questions:*

- [*What syntax is used to identify dependencies on the standard library in `Cargo.toml`?*][unresolved-dep-syntax]
- [*What is the format for builtin dependencies in `Cargo.lock`?*][unresolved-lockfile]

*See the following sections for future possibilities:*

- [*Replace `#![no_std]` as the source-of-truth for whether a crate depends on `std`*][future-replace-no_std]
- [*Allow unstable crate names to be referenced behind cfgs without requiring nightly*][future-cfg-unstable-crate-name]
- [*Allow `builtin` source replacement*][future-source-replacement]
- [*Remove `rustc_dep_of_std`*][future-rustc_dep_of_std]

[rfcs#3874-standard-library-crate-stability]: https://github.com/davidtwco/rfcs/blob/build-std-part-two-always/text/3874-build-std-always.md#standard-library-crate-stability
[rfcs#3874-noprelude]: https://github.com/davidtwco/rfcs/blob/build-std-part-two-always/text/3874-build-std-always.md#why-use-noprelude-with---extern

### Non-`builtin` standard library dependencies
[non-builtin-standard-library-dependencies]: #non-builtin-standard-library-dependencies

Cargo already supports `path` and `git` dependencies for crates named `core`,
`alloc` and `std` which continue to be supported and work:

```toml
[package]
name = "hello_world"
version = "0.1.0"
edition = "2024"

[dependencies]
std = { path = "../my_std" } # already supported by Cargo
```

A `core`/`alloc`/`std` dependency with a `path`/`git` source can be combined
with `builtin` dependencies:

```toml
[package]
name = "hello_world"
version = "0.1.0"
edition = "2024"

[dependencies]
std = { path = "../my_std" }
core = { builtin = true }
```

Crates with these dependency sources will remain unable to be published to
crates.io.

### Patches
[patches]: #patches

Under a perma-unstable feature it is permitted to patch standard library
dependencies with `path` and `git` sources (or any other source)
([?][rationale-patching]):

```toml
[package]
name = "hello_world"
version = "0.1.0"
edition = "2024"

[dependencies]
std = { builtin = true }

[patch.builtin] # permitted on nightly
std = { .. }

[patch.builtin] # permitted on nightly
std = { path = "../libstd" }
```

As with dependencies, crates with `path`/`git` patches for `core`, `alloc` or
`std` are not accepted by crates.io.

*See the following sections for rationale/alternatives:*

- [*Why unstably permit patching of standard library dependencies?*][rationale-patching]

*See the following sections for relevant unresolved questions:*

- [*What syntax is used to patch dependencies on the standard library in `Cargo.toml`?*][unresolved-patch-syntax]

### Features
[features]: #features

On a stable toolchain, it is not permitted to enable or disable features of
explicit standard library dependencies ([?][rationale-features]), as in the
below example:

```toml
[package]
name = "hello_world"
version = "0.1.0"
edition = "2024"

[dependencies]
std = { builtin = true, features = [ "foo" ] } # not permitted
# ..or..
std = { builtin = true, default-features = false } # not permitted
```

*See the following sections for rationale/alternatives:*

- [*Why limit enabling standard library features to an unstable feature?*][rationale-features]

*See the following sections for future possibilities:*

- [*Allow enabling/disabling features with build-std*][future-features]

### Public and private dependencies
[public-and-private-dependencies]: #public-and-private-dependencies

Implicit and explicit dependencies on the standard library default to being
public dependencies ([?][rationale-default-public]).

```toml
[package]
name = "hello_world"
version = "0.1.0"
edition = "2024"

[dependencies]
```

..is equivalent to the following explicit dependency on `std`:

```toml
[package]
name = "hello_world"
version = "0.1.0"
edition = "2024"

[dependencies]
std = { builtin = true, public = true }
```

*See the following sections for relevant unresolved questions:*

- [*Should standard library dependencies default to public?*][unresolved-std-default-public]

*See the following sections for rationale/alternatives:*

- [*Why default to public for standard library dependencies?*][rationale-default-public]

### `dev-dependencies` and `build-dependencies`
[dev-dependencies-and-build-dependencies]: #dev-dependencies-and-build-dependencies

Explicit dependencies on the standard library can be specified in
`dev-dependencies` in the same way as regular `dependencies`. Any explicit
`builtin` dependency present in `dev-dependencies` table will disable the
implicit dependencies. It is possible for `dev-dependencies` to have additional
`builtin` dependencies that the `dependencies` section does not have (e.g.
requiring `std` when the regular dependencies only require `core`).

Build scripts and proc macros continue to use the pre-built standard library as
in [`build-std=always`][rfcs#3874-proposal], and so explicit dependencies on the
standard library are not supported in `build-dependencies`.

*See the following sections for relevant unresolved questions:*

- [*Should we support `build-dependencies`?*][unresolved-build-deps]

### Registries
[registries]: #registries

Standard library dependencies will be present in the registry index
([?][rationale-cargo-index]). A `builtin_deps` key is added to the
[index's JSON schema][cargo-json-schema] ([?][rationale-cargo-builtindeps]).
`builtin_deps` is similar to the existing `deps` key and contains a list of JSON
objects, each representing a dependency that is "builtin" to the Rust toolchain
and cannot otherwise be found in the registry. The
["publish" endpoint][cargo-registry-web-publish] of the Registry Web API will
similarly be updated to support `builtin_deps`.

> [!NOTE]
>
> It is expected that the keys of these objects will be:
>
> - `name`
>   - String containing name of the `builtin` package. Can shadow the names of
>     other packages in the registry (except those packages in the `deps` key
>     of the current package) ([?][rationale-cargo-index-shadowing])
>
> - `features`:
>   - An array of strings containing enabled features in order to support
>     changing the standard library features on nightly. Optional, empty by
>     default.
>
> - `optional`, `default_features`, `target`, `kind`:
>   - These keys have the same definition as in the `deps` key
>
> The keys `req`, `registry` and `package` from `deps` are not required per the
> limitations on builtin dependencies.
>
> The `builtin_deps` key is optional and if not present its default value will
> be the implicit builtin dependencies:
>
> ```json
> "builtin_deps" : [
>     {
>         "name": "std",
>         "features": [],
>         "optional": false,
>         "default_features": true,
>         "target": null,
>         "kind": "normal",
>     },
>     {
>         "name": "alloc",
>         ... # as above
>     },
>     {
>         "name": "core",
>         ... # as above
>     }
> ]
> ```
>
> When producing a registry index entry for a package Cargo will not serialise
> any `builtin` dependencies it inferred. This allows the set of inferred
> packages to change in the future if needed and prevents publishing a package
> with a new Cargo from raising your MSRV. Similarly, the published `Cargo.toml`
> will not explicitly declare any inferred dependencies.

*See the following sections for rationale/alternatives:*

- [*Why add standard library crates to Cargo's index?*][rationale-cargo-index]
- [*Why add a new key to Cargo's registry index JSON schema?*][rationale-cargo-builtindeps]
- [*Why can `builtin_deps` shadow other packages in the registry?*][rationale-cargo-index-shadowing]

[cargo-registry-web-publish]: https://doc.rust-lang.org/cargo/reference/registry-web-api.html#publish

### Cargo subcommands
[cargo-subcommands]: #cargo-subcommands

Any Cargo command which accepts a package spec with `-p` will now additionally
recognise `core`, `alloc` and `std` and none of their dependencies. Many of
Cargo's subcommands will need modification to support build-std:

[`cargo add`][cargo-add]'s heuristics will include adding `std`, `alloc` or
`core` as builtin dependencies if these crate names are provided. `cargo add`
will additionally have a `--builtin` flag to allow for adding crates with a
`builtin` source explicitly:

```toml
[package]
name = "hello_world"
version = "0.1.0"
edition = "2024"

[dependencies]
std = { builtin = true } # <-- this would be added
```

If attempting to add a crate name outside of `core`, `alloc` or `std` this will
fail unless the required `cargo-feature` is added to allow other `builtin` crate
names as described in [the rationale][rationale-unstable-builtin-crates].

If attempting to add a `builtin` crate with features then this will fail unless
the required `cargo-feature` is enabled as described in [*Features*][features].

Once public and private dependencies are stabilised ([rust#44663]), `cargo add`
will add `public = true` by default for the standard library dependencies added:

```toml
[package]
name = "hello_world"
version = "0.1.0"
edition = "2024"

[dependencies]
std = { builtin = true, public = true } # <-- this would be added
```

[`cargo info`][cargo-info] will learn how to print information for the built-in
`std`, `alloc` and `core` dependencies:

```shell-session
$ cargo info std
std
rust standard library
license: Apache 2.0 + MIT
rust-version: 1.86.0
documentation: https://doc.rust-lang.org/1.86.0/std/index.html
```

```shell-session
$ cargo info alloc
alloc
rust standard library
license: Apache 2.0 + MIT
rust-version: 1.86.0
documentation: https://doc.rust-lang.org/1.86.0/alloc/index.html
```

```shell-session
$ cargo info core
core
rust standard library
license: Apache 2.0 + MIT
rust-version: 1.86.0
documentation: https://doc.rust-lang.org/1.86.0/core/index.html
```

[`cargo metadata`][cargo-metadata] will emit `std`, `alloc` and `core`
dependencies to the metadata emitted by `cargo metadata` (when those crates are
explicit dependencies). `source` would be set to `builtin` and the remaining
fields would be set like any other dependency. See also unresolved question
[*Should `cargo metadata` include the standard library's dependencies?*][unresolved-cargo-metadata].

> [!NOTE]
>
> `cargo metadata` output could look as follows:
>
> ```json
> {
>   "packages": [
>     {
>       /* ... */
>       "dependencies": [
>         {
>           "name": "std",
>           "source": "builtin",
>           "req": "*",
>           "kind": null,
>           "rename": null,
>           "optional": false,
>           "uses_default_features": true,
>           "features": ["compiler-builtins-mem"],
>           "target": null,
>           "public": true
>         }
>       ],
>       /* ... */
>     }
>   ]
> }
> ```

[`cargo pkgid`][cargo-pkgid] when passed `-p core` would print
`builtin://.#core` as the source, likewise with `alloc` and `std`. This format
complies with [Cargo's spec for Package IDs][cargo-pkgid-spec].

[`cargo remove`][cargo-remove] will remove `core`, `alloc` or `std` explicitly
from the manifest if invoked with those crate names (using the same heuristics
as those described above for `cargo add`):

```toml
[package]
name = "hello_world"
version = "0.1.0"
edition = "2024"

[dependencies]
std = { builtin = true } # <-- this would be removed
```

[`cargo tree`][cargo-tree] will show `std`, `alloc` and `core` at appropriate
places in the tree of dependencies. As opaque dependencies, none of the other
dependencies of `std`, `alloc` or `core` will be shown. Neither `std`, `alloc`
or `core` will have a version number.

> [!NOTE]
>
> `cargo tree` output could look as follows:
>
> ```shell-session
> $ cargo tree
> myproject v0.1.0 (/myproject)
> ├── rand v0.7.3
> │   ├── getrandom v0.1.14
> │   │   ├── cfg-if v0.1.10
> │   │   │   └── core v0.0.0
> │   │   ├── libc v0.2.68
> │   │   │   └── core v0.0.0
> │   │   └── core v0.0.0
> │   ├── libc v0.2.68 (*)
> │   │   └── core v0.0.0
> │   ├── rand_chacha v0.2.2
> │   │   ├── ppv-lite86 v0.2.6
> │   │   │   └── core v0.0.0
> │   │   ├── rand_core v0.5.1
> │   │   │   ├── getrandom v0.1.14 (*)
> │   │   │   └── core v0.0.0
> │   │   └── std v0.0.0
> │   │       └── alloc v0.0.0
> │   │           └── core v0.0.0
> │   ├── rand_core v0.5.1 (*)
> │   └── std v0.0.0 (*)
> └── std v0.0.0 (*)
> ```

This part of the RFC has no implications for the following Cargo subcommands:

- [`cargo bench`][cargo-bench]
- [`cargo build`][cargo-build]
- [`cargo check`][cargo-check]
- [`cargo clean`][cargo-clean]
- [`cargo clippy`][cargo-clippy]
- [`cargo doc`][cargo-doc]
- [`cargo fetch`][cargo-fetch]
- [`cargo fix`][cargo-fix]
- [`cargo fmt`][cargo-fmt]
- [`cargo generate-lockfile`][cargo-generate-lockfile]
- [`cargo help`][cargo-help]
- [`cargo init`][cargo-init]
- [`cargo install`][cargo-install]
- [`cargo locate-project`][cargo-locate-project]
- [`cargo login`][cargo-login]
- [`cargo logout`][cargo-logout]
- [`cargo miri`][cargo-miri]
- [`cargo new`][cargo-new]
- [`cargo owner`][cargo-owner]
- [`cargo package`][cargo-package]
- [`cargo publish`][cargo-publish]
- [`cargo report`][cargo-report]
- [`cargo run`][cargo-run]
- [`cargo rustc`][cargo-rustc]
- [`cargo rustdoc`][cargo-rustdoc]
- [`cargo search`][cargo-search]
- [`cargo test`][cargo-test]
- [`cargo uninstall`][cargo-uninstall]
- [`cargo update`][cargo-update]
- [`cargo vendor`][cargo-vendor]
- [`cargo version`][cargo-version]
- [`cargo yank`][cargo-yank]

[rust#44663]: https://github.com/rust-lang/rust/issues/44663
[cargo-pkgid-spec]: https://doc.rust-lang.org/cargo/reference/pkgid-spec.html

[cargo-add]: https://doc.rust-lang.org/cargo/commands/cargo-add.html
[cargo-bench]: https://doc.rust-lang.org/cargo/commands/cargo-bench.html
[cargo-build]: https://doc.rust-lang.org/cargo/commands/cargo-build.html
[cargo-check]: https://doc.rust-lang.org/cargo/commands/cargo-check.html
[cargo-clean]: https://doc.rust-lang.org/cargo/commands/cargo-clean.html
[cargo-clippy]: https://doc.rust-lang.org/cargo/commands/cargo-clippy.html
[cargo-doc]: https://doc.rust-lang.org/cargo/commands/cargo-doc.html
[cargo-fetch]: https://doc.rust-lang.org/cargo/commands/cargo-fetch.html
[cargo-fix]: https://doc.rust-lang.org/cargo/commands/cargo-fix.html
[cargo-fmt]: https://doc.rust-lang.org/cargo/commands/cargo-fmt.html
[cargo-generate-lockfile]: https://doc.rust-lang.org/cargo/commands/cargo-generate-lockfile.html
[cargo-help]: https://doc.rust-lang.org/cargo/commands/cargo-help.html
[cargo-info]: https://doc.rust-lang.org/cargo/commands/cargo-info.html
[cargo-init]: https://doc.rust-lang.org/cargo/commands/cargo-init.html
[cargo-install]: https://doc.rust-lang.org/cargo/commands/cargo-install.html
[cargo-locate-project]: https://doc.rust-lang.org/cargo/commands/cargo-locate-project.html
[cargo-login]: https://doc.rust-lang.org/cargo/commands/cargo-login.html
[cargo-logout]: https://doc.rust-lang.org/cargo/commands/cargo-login.html
[cargo-metadata]: https://doc.rust-lang.org/cargo/commands/cargo-metadata.html
[cargo-miri]: https://doc.rust-lang.org/cargo/commands/cargo-miri.html
[cargo-new]: https://doc.rust-lang.org/cargo/commands/cargo-new.html
[cargo-owner]: https://doc.rust-lang.org/cargo/commands/cargo-owner.html
[cargo-package]: https://doc.rust-lang.org/cargo/commands/cargo-package.html
[cargo-pkgid]: https://doc.rust-lang.org/cargo/commands/cargo-pkgid.html
[cargo-publish]: https://doc.rust-lang.org/cargo/commands/cargo-publish.html
[cargo-remove]: https://doc.rust-lang.org/cargo/commands/cargo-remove.html
[cargo-report]: https://doc.rust-lang.org/cargo/commands/cargo-report.html
[cargo-run]: https://doc.rust-lang.org/cargo/commands/cargo-run.html
[cargo-rustc]: https://doc.rust-lang.org/cargo/commands/cargo-rustc.html
[cargo-rustdoc]: https://doc.rust-lang.org/cargo/commands/cargo-rustdoc.html
[cargo-search]: https://doc.rust-lang.org/cargo/commands/cargo-search.html
[cargo-test]: https://doc.rust-lang.org/cargo/commands/cargo-test.html
[cargo-tree]: https://doc.rust-lang.org/cargo/commands/cargo-tree.html
[cargo-uninstall]: https://doc.rust-lang.org/cargo/commands/cargo-uninstall.html
[cargo-update]: https://doc.rust-lang.org/cargo/commands/cargo-update.html
[cargo-vendor]: https://doc.rust-lang.org/cargo/commands/cargo-vendor.html
[cargo-version]: https://doc.rust-lang.org/cargo/commands/cargo-version.html
[cargo-yank]: https://doc.rust-lang.org/cargo/commands/cargo-yank.html

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This section aims to justify all of the decisions made in the proposed design
from [*Proposal*][proposal] and discuss why alternatives were not chosen.

### Why explicitly declare dependencies on the standard library in `Cargo.toml`?
[rationale-why-explicit-deps]: #why-explicitly-declare-dependencies-on-the-standard-library-in-cargotoml

If there are no explicit dependencies on standard library crates, Cargo would
need to be able to determine which standard library crates to build when this is
required:

- Cargo could unconditionally build `std`, `alloc` and `core`. Not only would
  this be unnecessary and wasteful for `no_std` crates in the embedded
  ecosystem, but sometimes a target may not support building `std` at all and
  this would cause the build to fail.

- rustc could support a `--print` value that would print whether the crate
  declares itself as `#![no_std]` crate, and based on this, Cargo could build
  `std` or only `core`. This would require asking rustc to parse crates'
  sources while resolving dependencies, slowing build times. Alternatively,
  Cargo can already read Rust source to detect frontmatter (for `cargo script`)
  so it could additionally look for `#![no_std]` itself. Regardless of how it
  determines a crate is no-std, Cargo would also need to know whether to build
  `alloc` too, which checking for `#![no_std]` does not help with. Cargo could
  go further and ask rustc whether a crate (or its dependencies) used `alloc`,
  but this seems needlessly complicated.

- Cargo could allow the user to specify which crates are required to be built,
  such as with the existing options to the `-Zbuild-std=` flag.
  [`build-std=always`][rfcs#3874-proposal] proposes a `build-std.crates` flag to
  enable explicit dependencies to be a separate part of this RFC.

Furthermore, supporting explicit dependencies on standard library crates enables
use of other Cargo features that apply to dependencies in a natural and
intuitive way. If there were not explicit standard library dependencies and
enabling features on the `std` crate was desirable, then a mechanism other than
the standard syntax for this would be necessary, such as a flag (e.g.
`-Zbuild-std-features`) or option in Cargo's configuration. This also applies to
optional dependencies, public/private features, etc.

Users already use Cargo features to toggle `#![no_std]` in crates which support
building without the standard library. When dependencies on the standard library
are exposed in `Cargo.toml` then they can be made optional and enabled by the
existing Cargo features that crates already have.

↩ [*Proposal*][proposal]

### Why disallow builtin dependencies to be combined with other sources?
[rationale-builtin-other-sources]: #why-disallow-builtin-dependencies-to-be-combined-with-other-sources

If using `path`/`git` sources with `builtin` dependencies worked in the same way
as using `path`/`git` sources with `version` sources, then: crates with
`path`/`git` standard library dependencies could be pushed to crates.io.

This is not desirable as it is unclear that supporting `path`/`git` sources
which shadow standard library crates was a deliberate choice and so enabling
that pattern to be used more widely when not necessary is needlessly permissive.

In addition, when combined with a `git`/`path` source, the `version` constraint
also applies to package from the `git`/`path` source. If `version` were used
alongside `builtin`, then this behaviour would be a poor fit as..

- ..the `std`, `alloc` and `core` crates all currently have a version of `0.0.0`

- ..choosing different version requirements for different `builtin` crates is
  confusing when a single version of these crates is provided by the toolchain

Hypothetically, choosing a different version for `builtin` crates could be a way
of supporting per-target/per-profile MSRVs, but this has limited utility.

↩ [*Proposal*][proposal]

### Why disallow builtin dependencies on other crates?
[rationale-no-builtin-other-crates]: #why-disallow-builtin-dependencies-on-other-crates

`builtin` dependencies could be accepted on two other crates - dependencies of
the standard library, like `compiler_builtins`, or other crates in the sysroot
added manually by users, however:

- The standard library's dependencies are not part of the stable interface of
  the standard library and it is not desirable that users can observe their
  existence or depend on them directly

- Other crates in the sysroot added by users are not something that can
  reasonably be supported by build-std and these crates should become regular
  dependencies

↩ [*Proposal*][proposal]

### Why unstably allow all names for `builtin` crates?
[rationale-unstable-builtin-crates]: #why-unstably-allow-all-names-for-builtin-crates

For any crate shipped with the standard library in the sysroot, the user can
already write an `extern crate` declaration to use it. Most are marked unstable
either explicitly or implicitly with the use of `-Zforce-unstable-if-unmarked`
so this does not allow items from these crates to be used on stable.

For example, some users write benchmarks using `libtest` and have written
`extern crate test` without the `#[cfg(test)]` attribute to load the crate.
There may be other niche uses of unstable sysroot crates that this enables to
continue on nightly toolchains.

An allowlist of `builtin` crate names isn't used here to avoid Cargo needing to
hardcode the names of many crates in the sysroot which are inherently unstable.

↩ [*Proposal*][proposal]

### Why not use `noprelude` for explicit `builtin` dependencies?
[rationale-explicit-noprelude]: #why-not-use-noprelude-for-explicit-builtin-dependencies

Explicit builtin dependencies without the `noprelude` modifier behave more
consistently with other dependencies specified in the Cargo manifest.

This is a trade-off, trading consistency of user experience with special-casing
in Cargo. Cargo would have to handle implicit vs explicit dependencies
differently. An explicit dependency on the standard library will behave
similarly to other dependencies in their manifest, but the behaviour will be
subtly different than with implicit builtin dependencies (where `extern crate`
is required).

↩ [*Proposal*][proposal]

### Why not require builtin dependencies instead of supporting implicit ones?
[rationale-no-migration]: #why-not-require-builtin-dependencies-instead-of-supporting-implicit-ones

Requiring explicit `builtin` dependencies over an edition would increase the
boilerplate required for users of Cargo and make the minimal `Cargo.toml` file
larger.

Supporting implicit dependencies allows the majority of the Rust ecosystem from
having to make any changes - `no_std` crates (or crates with a `std` feature)
will still benefit from adding explicit dependencies as allow them to be easily
used with `no_std` targets but users can still work around any legacy crates in
the graph with [`build-std.crates`][rfcs#3874-proposal].

↩ [*Proposal*][proposal]

### Why disallow renaming standard library dependencies?
[rationale-package-key]: #why-disallow-renaming-standard-library-dependencies

Cargo allows [renaming dependencies][cargo-docs-renaming] with the `package`
key, which allows user code to refer to dependencies by names which do not
match their `package` name in their respective `Cargo.toml` files.

However, rustc expects the standard library crates to be present with their
existing names - for example, `core` is always added to the
[extern prelude][rust-extern-prelude].

Alternatively, a mechanism could be added to rustc so that it could be informed
of the user's names for `builtin` crates.

↩ [*Proposal*][proposal]

[cargo-docs-renaming]: https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#renaming-dependencies-in-cargotoml
[rust-extern-prelude]: https://doc.rust-lang.org/reference/names/preludes.html#extern-prelude

### Why disallow source replacement on `builtin` packages?
[rationale-source-replacement]: #why-disallow-source-replacement-on-builtin-packages

Modifying the source code of the standard library in the `rust-src` component is
not supported. Source replacement of the `builtin` source could be a way to
support this in future but this is out-of-scope for this proposal.

See [*Allow `builtin` source replacement*][future-source-replacement].

↩ [*Proposal*][proposal]

### Why not permit overriding dependencies with `replace` or `paths`?
[rationale-overriding-builtins]: #why-not-permit-overriding-dependencies-with-replace-or-paths

Similarly to [source replacement][rationale-source-replacement], easing
modification of the standard library sources is out-of-scope for this proposal.

↩ [*Proposal*][proposal]

### Why add standard library dependencies to `Cargo.lock`?
[rationale-cargo-lock]: #why-add-standard-library-dependencies-to-cargolock

`Cargo.lock` is a direct serialisation of a resolve and that must be a two-way
non-lossy process in order to make the `Cargo.lock` useful without doing further
resolution to fill in missing `builtin` packages.

↩ [*Proposal*][proposal]

### Why unstably permit patching of the standard library dependencies?
[rationale-patching]: #why-unstably-permit-patching-of-the-standard-library-dependencies

Being able to patch `builtin = true` dependencies and replace their source with
a `path` dependency is required to be able to replace `rustc_dep_of_std`. As
crates which use these sources cannot be published to crates.io, this would not
enable a usable general-purpose mechanism for crates to modify the standard
library sources. This capability is restricted to nightly toolchains as that is
all that is required for it to be used in replacing `rustc_dep_of_std`.

↩ [*Patches*][patches]

### Why limit enabling standard library features to an unstable feature?
[rationale-features]: #why-limit-enabling-standard-library-features-to-an-unstable-feature

If it were possible to enable features of the standard library crates on stable
then all of the standard library's current features would immediately be held to
the same stability guarantees as the rest of the standard library, which is not
desirable. See
[*Allow enabling/disabling features with build-std*][future-features]

↩ [*Features*][features]

### Why default to public for standard library dependencies?
[rationale-default-public]: #why-default-to-public-for-standard-library-dependencies

There are crates building on stable which re-export from the standard library.
If implicit standard library dependencies were not public then these crates would
start to trigger the `exported_private_dependencies` lint when upgrading to a
version of Cargo with a implicit standard library dependency.

↩ [*Public and private dependencies*][public-and-private-dependencies]

### Why add standard library crates to Cargo's index?
[rationale-cargo-index]: #why-add-standard-library-crates-to-cargos-index

When Cargo builds the dependency graph, it is driven by the index (not
`Cargo.toml`), so builtin dependencies need to be included in the index.

↩ [*Registries*][registries]

### Why add a new key to Cargo's registry index JSON schema?
[rationale-cargo-builtindeps]: #why-add-a-new-key-to-cargos-registry-index-json-schema

Cargo's [registry index schema][cargo-json-schema] is versioned and making a
behaviour-of-Cargo-modifying change to the existing `deps` keys would be a
breaking change. Each package is published under one particular version of the
schema, meaning that older versions of Cargo cannot use newer versions of
packages which are defined using a schema it does not have knowledge of.

Cargo ignores packages published under an unsupported schema version, so older
versions of Cargo cannot use newer versions of packages relying on these
features (though this would be true because of an incompatible Cargo manifest
anyway). New schema versions are disruptive to users on older toolchains, as the
resolver will act as if a package does not exist. Recent Cargo versions have
improved error reporting for this circumstance.

Some new fields, including `rust-version`, were added to all versions of the
schema. Cargo ignores fields it does not have knowledge of, so older versions of
Cargo will simply not use `rust-version` and its presence does not change their
behaviour.

Existing versions of Cargo already function correctly without knowledge of
crate's standard library dependencies. A new top-level key will be ignored by
older versions of Cargo, while newer versions will understand it. This is a
different approach to that taken when artifact dependencies were added to the
schema, as those do not have a suitable representation in older versions of
Cargo.

The obvious alternative to a `builtin_deps` key is to modify `deps` entries with
a new `builtin: bool` field and to increment the version of the schema. However,
these entries would not be understood by older versions of Cargo which would
look in the registry to find these packages and fail to do so.

That approach could be made to work if dummy packages for `core`/`alloc`/`std`
were added to registries. Older versions of Cargo would pass these to rustc
via `--extern` and shadow the real standard library dependencies in the sysroot,
so these packages would need to contain `extern crate std; pub use std::*;` (and
similar for `alloc`/`core`) to try and load the pre-built libraries from the
sysroot (this is the same approach as packages like [embed-rs][embed-rs-source]
take today, using `path` dependencies for the standard library to shadow it).

↩ [*Registries*][registries]

[cargo-json-schema]: https://doc.rust-lang.org/cargo/reference/registry-index.html#json-schema
[embed-rs-source]: https://github.com/embed-rs/stm32f7-discovery/blob/e2bf713263791c028c2a897f2eb1830d7f09eceb/core/src/lib.rs#L7

### Why can `builtin_deps` shadow other packages in the registry?
[rationale-cargo-index-shadowing]: #why-can-builtin_deps-shadow-other-packages-in-the-registry

While `crates.io` forbids certain crate names including `std`, `alloc` and
`core`, third party registries may allow them without a warning. The schema
needs a way to refer to packages with the same name either in the registry or
builtin, which `builtin_deps` allows.

`builtin_deps` names are not allowed to shadow names of packages in `deps` as
these would conflict when passed to rustc via `--extern`.

↩ [*Registries*][registries]

## Unresolved questions
[unresolved-questions]: #unresolved-questions

The following small details are likely to be bikeshed prior to this part of the
RFC's acceptance or stabilisation and aren't pertinent to the overall design:

### What syntax is used to identify dependencies on the standard library in `Cargo.toml`?
[unresolved-dep-syntax]: #what-syntax-is-used-to-identify-dependencies-on-the-standard-library-in-cargotoml

What syntax should be used for the explicit standard library dependencies?
`builtin = true`? `sysroot = true` (not ideal, as "sysroot" isn't a concept that
we typically introduce to end-users)?

↩ [*Proposal*][proposal]

### What is the format for builtin dependencies in `Cargo.lock`?
[unresolved-lockfile]: #what-is-the-format-for-builtin-dependencies-in-cargolock

How should `builtin` deps be represented in lockfiles? Is `builtin = true`
appropriate? Could the `source` field be reused with the string "builtin" or
should it stay only as a URL+scheme?

↩ [*Proposal*][proposal]

### What syntax is used to patch dependencies on the standard library in `Cargo.toml`?
[unresolved-patch-syntax]: #what-syntax-is-used-to-patch-dependencies-on-the-standard-library-in-cargotoml

`[patch.builtin]` is the natural syntax given `builtin` is a new source, but may
be needlessly different to existing packages.

↩ [*Patches*][patches]

### Should standard library dependencies default to public?
[unresolved-std-default-public]: #should-standard-library-dependencies-default-to-public

Standard library dependencies defaulting to public is a trade-off between
special-casing in Cargo and requiring that any user with a dependency on the
standard library who re-exports from the standard library manually declare their
dependency as public.

It is also inconsistent with
[*Why not use `noprelude` for explicit `builtin` dependencies?*][rationale-explicit-noprelude]
which aims to make builtin dependencies consistent with other dependencies in
the manifest.

↩ [*Public and private dependencies*][public-and-private-dependencies]

### Should we support `build-dependencies`?
[unresolved-build-deps]: #should-we-support-build-dependencies

Allowing `builtin` dependencies to be used in `dependencies` and
`dev-dependencies` but not in `build-dependencies` is an inconsistency.

However, supporting `builtin` dependencies in `build-dependencies` would permit
no-std build scripts. It is unclear whether supporting no-std build scripts
would be desirable.

↩ [*`dev-dependencies` and `build-dependencies`*][dev-dependencies-and-build-dependencies]

### Should `cargo metadata` include the standard library's dependencies?
[unresolved-cargo-metadata]: #should-cargo-metadata-include-the-standard-librarys-dependencies

`cargo metadata` is used by tools like rust-analyzer to determine the entire
crate graph and would benefit from knowledge of the standard library's
dependencies, but this leaks internal details of the standard library and is
counter to the intent behind opaque dependencies.

↩ [*Cargo subcommands*][cargo-subcommands]

## Prior art
[prior-art]: #prior-art

See the [*Background*][rfcs#3873-background] and [*History*][rfcs#3873-history]
of the build-std context RFC.

## Future possibilities
[future-possibilities]: #future-possibilities

This RFC unblocks fixing [rust-lang/cargo#8798], enabling no-std crates from
being prevented from having std dependencies.

There are also many possible follow-ups to this part of the RFC:

[rust-lang/cargo#8798]: https://github.com/rust-lang/cargo/issues/8798

### Replace `#![no_std]` as the source-of-truth for whether a crate depends on `std`
[future-replace-no_std]: #replace-no_std-as-the-source-of-truth-for-whether-a-crate-depends-on-std

Crates can currently use the crate attribute `#![no_std]` to indicate a lack of
dependency on `std`. Introducing `build-std.crates` from [RFC #3874][rfcs#3874]
or explicit dependencies would add a second way for the user to indicate a lack
of dependency on the standard library. It could therefore be desirable to
deprecate `#![no_std]` so that there remains only a single way to express a
dependency on the standard library.

`#![no_std]` serves two purposes - it stops the compiler from adding `std` to
the extern prelude and it prevents the user from depending on anything from
`std` accidentally. rustc's default behaviour of loading `std` when not
explicitly provided the crate via an `--extern` flag should be preserved for
backwards-compatibility with existing direct invocations of rustc.

Initially, if a crate has the `#![no_std]` attribute and has implicit
dependencies on the standard library in its `Cargo.toml`, a lint could be
emitted to suggest that their Cargo dependencies are adjusted.

Eventually, `#![no_std]` could instead become a compiler flag which would
indicate to the compiler that `std` should not be loaded by default and that
`core`'s prelude should be used instead. Cargo would use this flag when driving
rustc, providing explicit paths to the newly-built or pre-built standard library
crates, just as with any other dependency.

In addition, uses of the `#![no_std]` attribute could be migrated to denying a
lint which would prevent use of items from `std`.

↩ [*Proposal*][proposal]

### Allow unstable crate names to be referenced behind cfgs without requiring nightly
[future-cfg-unstable-crate-name]: #allow-unstable-crate-names-to-be-referenced-behind-cfgs-without-requiring-nightly

It is possible to allow builtin dependencies on unstable crate names to exist
behind cfgs and for the crate to be compiled on a stable toolchain as long as
the cfgs are not active. This is a trade-off - it adds a large constraint on
when Cargo can validate the set of crate names, but would enable users to avoid
using nightly or doing MSRV bumps.

↩ [*Proposal*][proposal]

### Allow `builtin` source replacement
[future-source-replacement]: #allow-builtin-source-replacement

This involves allowing the user to blanket-override the standard library sources
with a `[source.builtin]` section of the Cargo configuration.

As [rationale-source-replacement] details it is unclear if users need to do this
or if it's even something the Rust project wishes to support.

↩ [*Proposal*][proposal]

### Remove `rustc_dep_of_std`
[future-rustc_dep_of_std]: #remove-rustc_dep_of_std

With first-class explicit dependencies on the standard library,
`rustc_dep_of_std` is rendered unnecessary and explicit dependencies on the
standard library can always be present in the `Cargo.toml` of the standard
library's dependencies.

The `core`, `alloc` and `std` dependencies can be patched in the standard
library's workspace to point to the local copy of the crates. This avoids
`crates.io` dependencies needing to add support for `rustc_dep_of_std` before
the standard library can depend on them.

↩ [*Proposal*][proposal]

### Allow enabling/disabling features with build-std
[future-features]: #allow-enablingdisabling-features-with-build-std

This would require the library team be comfortable with the features declared on
the standard library being part of the stable interface of the standard library.

The behaviour of disabling default features has been highlighted as a potential
cause of breaking changes.

Alternatively, this could be enabled alongside another proposal which would
allow the standard library to define some features as stable and others as
unstable.

As there are some features that Cargo will set itself when appropriate (e.g. to
enable or disable [panic runtimes][rfcs#3874-panic-strategies] or
[`compiler-builtins/mem`][rfcs#3874-compiler-builtins-mem]), Cargo may need to always
prevent some otherwise stable features from being toggled as it controls those.

↩ [*Features*][features]

[rfcs#3874-panic-strategies]: https://github.com/davidtwco/rfcs/blob/build-std-part-two-always/text/3874-build-std-always.md#panic-strategies
[rfcs#3874-compiler-builtins-mem]: https://github.com/davidtwco/rfcs/blob/build-std-part-two-always/text/3874-build-std-always.md#compiler-builtinsmem

### Allow local builds of `compiler-rt` intrinsics
[future-compiler-builtins-c]: #allow-local-builds-of-compiler-rt-intrinsics

The [`c` feature][rfcs#3873-dependencies] of `compiler_builtins` (which is also
exposed by `core`, `alloc` and `std` through `compiler-builtins-c`) causes its
`build.rs` file to build and link in more optimised C versions of intrinsics.

It will not be enabled by default because it is possible that the target
platform does not have a suitable C compiler available. The user being able to
enable this manually will be enabled through work on features (see
[*Allow enabling/disabling features with build-std*][future-features]). Once the
user can enable `compiler-builtins/c`, they will need to manually configure
`CFLAGS` to ensure that the C components will link with Rust code.
