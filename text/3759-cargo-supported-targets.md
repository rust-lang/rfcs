- Feature Name: `supported-targets`
- Start Date: 2025-01-08
- Pre-RFC: [Rust
  internals](https://internals.rust-lang.org/t/pre-rfc-allow-packages-to-specify-a-set-of-supported-targets/21979)
- RFC PR: [rust-lang/rfcs#3759](https://github.com/rust-lang/rfcs/pull/3759)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

The word _target_ is extensively used in this document. The
[glossary](https://doc.rust-lang.org/cargo/appendix/glossary.html#target) defines its many meanings.
Here, _target_ refers to the "Target Architecture" for which a package is built. Otherwise, the
terms "cargo-target" and "target-tuple" are used in accordance with their definitions in the
glossary.

# Summary

The addition of `supported-targets` to `Cargo.toml`. This field is a `cfg` string that restricts the
set of targets which a package supports. Packages can only be built for targets that satisfy their
`supported-targets`.

```toml
[package]
name = "hello_cargo"
supported-targets = 'cfg(any(target_os = "linux", target_os = "macos"))'
```

# Motivation

_For more background, see [rust-lang/cargo#6179](https://github.com/rust-lang/cargo/issues/6179)_

Some packages rely on features or behavior that is not available on every platform `rustc` can target. Currently, there is no way to formally
specify platform requirements of a package.

When working on a project with packages that only build on certain platforms, users cannot run Cargo commands across the entire workspace (e.g. `cargo test --workspace`) but must individually select packages that only work on the specific platform (e.g. `cargo test --workspace --exclude firmware`).  This extends to CI with people wanting to write matrix jobs but have to hand maintain the list of packages for each platform in the matrix.

## Long-term Motivations

This RFC unblocks further work to improve platform-specific packages.  While these problems are important, solving them has been left to  [future
possibilities](#future-possibilities) to deliver an MVP we can then build on.

### Include fewer packages with `cargo vendor`

`Cargo.lock`, and by extension, `cargo vendor`, must assume that a package may be built on any platform that has or will exist.  This means that if a transitive dependency pulls in Windows-specific dependencies, `cargo vendor` will include them when run on a Linux-only application.  Being able to tell `cargo vendor` what platforms to care about can reduce the space used in a repo and reduce churn.

### Dependency Management 

Likewise, today users either need to audit dependencies irrelevant for the platforms they target or filter these out somehow.  By providing first-class support for specifying what platform features a package requires, audit tools can consolidate on that for narrowing down the list of what dependencies to audit.

### More specific error messages

The error message when a library has platform-specific features, like requiring atomics, is about parts of `std` missing which could be for one of several reasons. Some of these problems won't be found until you've built or tested your project on one of these platforms. Like with [#2495](https://rust-lang.github.io/rfcs/2495-min-rust-version.html), if library authors could provide this information to Cargo, developers can get an improved error message under any circumstance.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The `supported-targets` field can be added to `Cargo.toml` under the `[package]` table.

This field is a string containing a `cfg` specification (as for the `[target.'cfg(**)']` table). The
supported `cfg` syntax is the same as the one for [platform-specific
dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#platform-specific-dependencies)
(i.e., `cfg(test)`, `cfg(debug_assertions)`, and `cfg(proc_macro)` are not supported). If a selected
target satisfies the `supported-targets`, then the package can be built for that target.

__For example:__
```toml
[package]
name = "hello_cargo"
version = "0.1.0"
edition = "2021"
supported-targets = 'cfg(any(target_os = "linux", target_os = "macos"))'
```
Here, only targets with the `linux` OS or the `macos` OS, are allowed to build the package. User
experience is enhanced by raising an error that fails compilation when the supported targets of a
package are not satisfied by the selected target.

This feature should be used when a package clearly does not support all targets. For example:
`io-uring` requires `cfg(target_os = "linux")`, `gloo` requires `cfg(target_family = "wasm")`, and
`riscv` requires `cfg(any(target_arch = "riscv32", target_arch = "riscv64"))`.

This feature increases cargo's knowledge of a package. For example, when working in a workspace
where some packages are for a platform with `target_os = "none"`, and some others are tools that
require a desktop OS, using `supported-targets` makes `cargo <command>` ignore packages which have
`supported-targets` that are not satisfied by the selected target.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The `supported-targets` field is an optional key that tells cargo which targets the package can be
built for. However, it does not affect which host can build the package i.e., any host can still build
the package, but only for certain targets.
```toml
[package]
# ...
supported-targets = 'cfg(any(target_os = "linux", target_os = "macos"))'
```
The value of this field must respect the [`cfg` syntax](https://doc.rust-lang.org/reference/conditional-compilation.html),
and does __not__ accept `cfg(test)`, `cfg(debug_assertions)`, nor `cfg(proc_macro)` as configuration options.
A malformed `supported-targets` field will raise an error.

If the `supported-targets` field is not present, then the package is assumed to support all targets. That is,
the default value is `'cfg(all())'` (understood as `cfg(true)`).

When a `cargo` build command (e.g. `check`, `build`, `run`, `clippy`) is run, it checks that the
selected target satisfies the `supported-targets` of the package being built. If it does not, the
package is skipped or an error is raised, depending on how [`cargo` was invoked](ignoring-builds).
However, `supported-targets` is _only_ checked for commands that take a `--target` option and does
not affect other commands (e.g., `cargo fmt`).

As this field is limited to local development, `cargo package` / `cargo publish` will strip it from `Cargo.toml`.
Including the field in the `.crate` file is left as a [future possibility](#future-possibilities) for now.

This field is subject to [workspace inheritance](https://doc.rust-lang.org/cargo/reference/workspaces.html#the-package-table).

## Ignoring builds for unsupported targets
[ignoring-builds]: #igonring-builds-for-unsupported-targets

If cargo is invoked in a workspace or virtual workspace without specifying a package as
build-target, then `cargo` skips any package that does not support the selected target. If a package
is specified using `--package` or if `cargo` is invoked on a single package, and the selected target
does not satisfy the `supported-targets` of the package, then an error is raised. The intent is to mimic
the behavior of `required-features` with package filtering based on targets. Hence, `required-targets`
is proposed as an [alternative name](#naming).

# Drawbacks
[drawbacks]: #drawbacks

- This is the first step towards a target aware `cargo`, which may increase `cargo`'s complexity,
  and bring more feature requests along these lines.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Format

The `cfg` string format was chosen because of its simplicity and expressiveness.
Other formats can be considered:

Using a list of `cfg` strings, and also accepting explicit target-tuples:
```toml
supported-targets = [
    'cfg(target_family = "unix")',
    'cfg(target_family = "wasm")',
    "x86_64-pc-windows-gnu",
]
```
This can be unintuitive to understand however, as the list implies a union of all its elements,
which is not immediately obvious.

Using the `[target]` table, for example:
```toml
[target.'cfg(target_os = "linux")']
supported = true
```
If the list of supported targets is long (should it ever be?), then the `Cargo.toml` file becomes
very verbose as well.

A `[suppported]` table, with `arch = ["<arch>", ...]`, `os = ["<os>", ...]`, `target = ["<target>",
...]`, etc. This is more verbose, complex to implement, learn, and remember. It is also not obvious
how `not` and `all` could be represented in this format. For example:
```toml
[supported]
os = ["linux", "macos"]
arch = ["x86_64"]
```

## Naming
[naming]: #Naming

Some other names for this field can be considered:

- `required-targets`. Pro: it matches with the naming of `required-features`. Con:
  `required-features` is a list of features that must _all_ be enabled (conjunction), whereas
  `supported-targets` is a list of targets where _any_ is allowed (disjunction).
- `targets`. As in "this package _targets_ ...". Pro: Concise. Con: Ambiguous, and could be confused
  with the `target` table.

## Package scope vs. cargo-target scope

The `supported-targets` field is placed at the package level, and not at the cargo-target level
(i.e., under, `[lib]`, `[[bin]]`, etc.)

It is possible to allow cargo-targets to further restrict the `supported-targets` of the package,
but this is left as a [future possibility](#future-possibilities).

See also: [using a package vs. using a workspace](package-vs-workspace).

[package-vs-workspace]:
#https://blog.rust-lang.org/inside-rust/2024/02/13/this-development-cycle-in-cargo-1-77.html#when-to-use-packages-or-workspaces

## Field format 

Using the `cfg` syntax complicates the implementation (and thus maintenance), and may require a substantial
amount of calls to `rustc` to check target-`cfg` compatibility. Some alternatives are discussed here
along with their drawbacks.

### Target-tuples

The initial proposal allowed both the `cfg` syntax and whitelisting specific target-tuples, to follow the behavior of [platform-specific
dependency tables](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#platform-specific-dependencies).
This was removed as it was deemed better to accept targets based on their _attributes_ rather than on their
_name_. Indeed, `rustc` supported target-tuples have changed names, and have been added or removed in the past.
Target-tuple names also do not encapsulate the semantics of the target.

### Using wildcards

Instead of using `cfg` specifications, one could use wildcards (e.g., `x86_64-*-linux-*`). This is
much simpler to implement, target-tuples are syntactically checked for a match instead of solving
set relations for `cfg`. However, this is not as expressive as `cfg`, and does not correctly
represent the semantics of target-tuples. For example, supporting `target_family = "unix"` would
require an annoyingly long list of wildcard patterns. Things like `target_pointer_width = "32"` are
even harder to represent, and things like `target_feature = "avx"` are basically not representable.
Also, this is new syntax not currently used by cargo.

### Allowing only target-tuples

This is an even stricter version of the above. Set relations between `supported-targets` lists are
exact, and the resolver can determine if a platform-specific dependency can be pruned from the
dependency tree more easily, hence why the original proposal chose this format. Being even simpler
to implement, this alternative may not be expressive enough for the common use case. Packages rarely
support specific target-tuples, rather they support/require specific target attributes. What would
likely happen is that packages would copy and paste the target-tuple list matching their
requirements from somewhere or someone else. Every time a new target with the same attribute is
added, the whole ecosystem would have to be updated.

# Prior art
[prior-art]: #prior-art

Users can already select which packages they want to select in a workspace with the flags
`--package` and `--exclude`. Cargo features can also be used to restrict which cargo-target
is built using the `required-features` field. However, `required-features` does not allow filtering
packages in a workspace, nor does it allow filtering out the library of a package.

The `per-package-target` nightly feature defines the `force-target` field, which is supposed to
force the package to build for a specific target-tuple. This does not interact well when used in
dependencies, as one would expect a dependency to be built for the same target as the package.
`supported-targets` supersedes `force-target` because instead of enforcing a single target, it
enforces a set of targets.

Published crates have mainly used their documentation to specify which targets they support, or they
would leave it up to the user to infer it. Some crates also made use of compile time errors to
ensure that `cfg` requirements are met, for example:
```rust
#[cfg(not(any(…)))]
compile_error!("unsupported target cfg");
```
[`getrandom`](https://github.com/rust-random/getrandom/blob/9fb4a9a2481018e4ab58d597ecd167a609033149/src/backends.rs#L156-L160)
is an example of a crate utilizing this method.

In other system level languages, vendoring dependencies is a common practice, and the user would be
responsible for ensuring that the dependencies are compatible with the target.

Some higher-level languages and build tools have the ability to specify which platforms are compatible.
- Python package has [classifiers](https://pypi.org/classifiers/) as package metadata that includes supported platforms
- Python wheels (pre-built packages) have [platform compatibility tags](https://packaging.python.org/en/latest/specifications/platform-compatibility-tags/#platform-compatibility-tags).
    The reference explains how these are [used](https://packaging.python.org/en/latest/specifications/platform-compatibility-tags/#use)
    by installers to determine which build of a package to install.
- `npm` allows specifying which [`os`](https://docs.npmjs.com/cli/v11/configuring-npm/package-json#os) and
    [`cpu`](https://docs.npmjs.com/cli/v11/configuring-npm/package-json#cpu) a package supports. These generate an
    error when installing a package that does not support the platform used.
- Swift has [`package.platforms`](https://developer.apple.com/documentation/packagedescription/package/platforms), which
    allows specifying which platforms and versions a package support (mostly for apple products e.g., `macOS`, `iOS`, `watchOS`, `tvOS`).
- [Buck](https://buck2.build/docs/rule_authors/configurations/#target-platform-compatibility)
    and [Bazel](https://bazel.build/reference/be/common-definitions#common.target_compatible_with)
    both provide `target_compatible_with`.
Some accept a string or list of strings representing the platforms, while `Buck` & `Bazel` seem to accept a
form comparable to `cfg` in Rust.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Should we strip the `cfg` prefix from the field e.g., `supported-targets = 'target_os = "linux"'`?

# Future possibilities
[future-possibilities]: #future-possibilities

## Ensuring proper use of dependencies

Complicated errors caused by packages and dependencies that are incompatible with the selected
target can be avoided by using the information in the `supported-targets` field. For example, a
warning or an error could be raised if a package uses a dependency that does not accept the package's
`supported-targets`:
```toml
[package]
name = "bar"
supported-targets = 'cfg(target_os = "windows")'
```
```toml
[package]
name = "foo"
supported-targets = 'cfg(target_os = "linux")'

[dependencies]
bar = "0.1.0"
```
Here, a compilation error helps by showing which dependency is incompatible with the package's
`supported-targets`, rather than a cryptic error message about missing parts of `std`, or runtime
errors.

Cargo's documentation should give clear guidance for when to use this field, and should not suggest
using it by default. In particular, we should steer users to use this when they have good reason to
believe the crate will not compile or work as expected (e.g. because it uses target-specific APIs),
and not use it merely for "I haven't personally tested this on other targets".

Even then, it will happen that crates unnecessarily limit their dependents and users because of 
overly restrictive `supported-targets`.
Some options for handling this include
- Doing nothing, encouraging people to upstream patches
- Encourage `[patch]`ing the dependency
  - Requires managing a fork
  - Every dependent of the package with a questionable `supported-targets` must do this
- Encourage unidiff `[patch]`es
  - Design has unresolved questions ([cargo#4648](https://github.com/rust-lang/cargo/issues/4648))
  - Every dependent of the package with a questionable `supported-targets` must do this
- A bespoke manifest override
  - One-off feature that needs design work
- A CLI override like `--ignore-rust-version`
  - This precludes `Cargo.lock` trimming as the lockfile is meant to capture dependencies for every potential state a package may be run in
  - This affects the entire dependency tree and not just the package with questionable `supported-targets`
  - Every dependent of the package with a questionable `supported-targets` must do this
- A lint like proposed for `package.rust-version`
  - Blocked on [cargo#12235](https://github.com/rust-lang/cargo/issues/12235)
  - See also CLI override
- Allow a registry database to override `supported-targets`
  - Blocked on a lot of design work ([related discussion](https://blog.rust-lang.org/inside-rust/2024/03/26/this-development-cycle-in-cargo-1.78.html#why-is-this-yanked))

### Compatibility of `[dependencies]`

One could restrict the set of `supported-targets` of a package to be a subset of the
`supported-targets` of its `[dependencies]`. If the crate itself had no `supported-targets`
specified, then all dependencies would need to support all targets.

If a dependency does not respect this requirement (if it is not compatible), an error would be
raised and the build would fail.

Enforcing this means a package cannot support targets that are not supported by its dependencies,
which is a good thing assuming the dependencies have correctly specified their `supported-targets`.

### Compatibility of `[dev-dependencies]`

`[dev-dependencies]` should be checked using the same method as regular `[dependencies]`. That is,
the package's `supported-targets` needs to be a subset of every `[dev-dependencies]`'s
`supported-targets`. The rationale is that an example, test, or benchmark has access to the
package's library and binaries, and so it must respect the `supported-targets` of the package.

### Compatibility of `[build-dependencies]`
[build-dependencies-compatability]: #compatibility-of-build-dependencies

What makes `[build-dependencies]` unique is that they are built for the host computer, and not the
selected target. As such, they are not restrained by the `supported-targets` of the package. Hence,
all dependencies are allowed in the `[build-dependencies]` table. However, a build error could be
raised if one of the build dependencies does not support the _host-tuple_ at build time.

A problem can arise if a crate's build script depends on a package that does not support `target_os
= "windows"` for example. It would be possible to only allow dependencies supporting all targets in
`[build-dependencies]`.


### Platform-specific dependencies

Platform-specific dependencies are dependencies under the `[target.**]` table. This includes normal
dependencies, build-dependencies, and dev-dependencies. Rules could be defined to ensure that
platform-specific dependencies are declared correctly.

When platform-specific dependencies are declared, the conditions under which they are declared
should be a subset of each dependency's `supported-targets`. For example, a dependency declared
under `[target.'cfg(target_os = "linux")'.dependencies]` should at least support the `linux` OS.

For regular dependencies and dev-dependencies, it would suffice for a platform-specific dependency
to support the _intersection_ of the package's supported-targets, and the target conditions it is
declared under. For example:
```toml
[package]
# ...
supported-targets = 'cfg(target_os = "linux")'

[target.'cfg(target_pointer_width = "64")'.dependencies]
foo = "0.1.0"
```
Here, it would suffice for `foo` to support `cfg(all(target_os = "linux", target_pointer_width =
"64"))`.

This would ensure that a package properly uses dependencies that are not available on all targets.
Assuming that the crate `io-uring` has `supported-targets = 'cfg(target_os = "linux")'`, a crate
could depend on it using:
```toml
[package]
# ...

[target.'cfg(target_os = "linux")'.dependencies]
io-uring = "0.1.0"
```
This would not be required if the package itself had `supported-targets = 'cfg(target_os =
"linux")'`, or an even stricter set.

### Artifact dependencies

If an artifact dependency has a `target` field, then the dependency would not be checked against the
package's `supported-targets`. However, the selected `target` for the dependency would need to be
compatible with the dependency's `supported-targets`, or else an error is raised. If the artifact
dependency does not have a `target` field, then it would be checked against the package's
`supported-targets`, like any other dependency.


## Eliminating unused dependencies from `Cargo.lock`

A package's dependencies may themselves have `[target.'cfg(..)'.dependencies]` tables, which may
never be used because of the `supported-targets` restrictions of the package. These can safely be
eliminated from the dependency tree of the package.

Consider the following example:
```toml
[package]
name = "foo"
# ...
supported-targets = 'cfg(target_os = "linux")'

[dependencies]
bar = "0.1.0"
```
```toml
[package]
name = "bar"

[target.'cfg(target_os = "macos")'.dependencies]
baz = "0.1.0"
```
Currently, `baz` is included in the dependency tree of `foo`, even though `foo` is never built for
`macos`. `baz` could be pruned from the dependency tree of `foo`, since `target_os = "macos"` is
mutually exclusive with `target_os = "linux"`.

This only applies to `[dependencies]` and `[dev-dependencies]`, as `[build-dependencies]` are
[not restrained by `supported-targets`](build-dependencies-compatability), so they are not pruned.

Formally, dependencies (and transitive dependencies) under `[target.**.dependencies]` tables are
eliminated from the dependency tree of a package if the `supported-targets` of the package is
mutually exclusive with the target preconditions of the dependency.

### Comparing `supported-targets`

To prune the dependency tree, and to ensure proper use of dependencies, it becomes necessary to
compare `supported-targets`. When comparing two sets of `supported-targets`, it is necessary to
know if one is a _subset_ of the other, or if both are _mutually exclusive_. To proceed, both
are flattened to the same representation, and they are then compared. This process is done
internally, and does not affect the `Cargo.toml` file.

#### Flattening `not`, `any`, and `all` in `cfg` specifications

Since `cfg` specifications can contain `not`, `any`, and `all` operators, these must be handled.
This is done by flattening the `cfg` specification to a specific form. This form is equivalent to
[disjunctive normal form](https://en.wikipedia.org/wiki/Disjunctive_normal_form).

The `not` operator is "passed through" `any` and `all` operators using [De Morgan's
laws](https://en.wikipedia.org/wiki/De_Morgan%27s_laws), until it reaches a single `cfg`
specification. For example, `cfg(not(all(target_os = "linux", target_arch = "x86_64")))` is
equivalent to `cfg(any(not(target_os = "linux"), not(target_arch = "x86_64")))`.

The `cfg` definition is transformed into `any` of `all` (top level union).

Top level `all` operators are kept as is, as long as they do not contain nested `any`s or `all`s. If
there is an `any` inside an `all`, the statement is split into multiple `all` statements. For
example,
```toml
supported-targets = 'cfg(all(target_os = "linux", any(target_arch = "x86_64", target_arch = "arm"))'
```
is transformed into
```toml
supported-targets = 'cfg(any(all(target_os = "linux", target_arch = "x86_64"), all(target_os = "linux", target_arch = "arm")))'
```
If an `all` contains an `all`, the inner `all` is flattened into the outer `all`.

The result of these transformations on a `cfg` specification is a union of `cfg` specifications that
either contains a single specification, or an `all` operator with no nested operators.

#### The subset relation

To determine if the `supported-targets` set "A" is a subset of another such set "B", the standard
mathematical definition of subset is used. That is, "A" is a subset of "B" if and only if each
element of "A" is contained in "B".

So each element of the union forming "A" is compared against each element of the union forming "B".
A `cfg(all(A, B, ...))` is a subset of a `cfg(all(C, D ...))`, if the list `C, D, ...` is a subset
of the list `A, B, ...`.

_Note_: `cfg(A) == cfg(all(A))`.

#### Mutual exclusivity

For the `supported-targets` set "A" to be mutually exclusive with another such set "B", each element
of "A" must be mutually exclusive with _all_ elements of "B" (The inverse is also true).

So each element of "A" is compared against each element of "B". A `cfg(all(A, B, ...))` is mutually
exclusive with a `cfg(all(C, D, ...))` if any element of the list `A, B, ...` is mutually exclusive
with any element of the list `C, D, ...`.

_Note_: `cfg(A) == cfg(all(A))`.

Two `cfg` singletons are mutually exclusive under the following rules:
- `cfg(A)` is mutually exclusive with `cfg(not(A))`.
- `cfg(<option> = "A")` is mutually exclusive with `cfg(<option> = "B")` if `A` and `B` are
  different, and `<option>` has mutually exclusive elements.

Some `cfg` options have mutually exclusive elements, while some do not. What is meant here is, for
example, `target_arch = "x86_64"` and `target_arch = "arm"` are mutually exclusive (a target-tuple
cannot have both), while `target_feature = "avx"` and `target_feature = "rdrand"` are not.

`cfg` options that have mutually exclusive elements:
- `target_arch`
- `target_os`
- `target_env`
- `target_abi`
- `target_endian`
- `target_pointer_width`
- `target_vendor`

Those that do not:
- `target_feature`
- `target_has_atomic`
- `target_family`

#### More `cfg` relations

Even more relations could be defined. Consider the following scenario:
```toml
[package]
name = "bar"
supported-targets = 'cfg(target_family = "unix")'
# ...
```
```toml
[package]
name = "foo"
supported-targets = 'cfg(target_os = "macos")'

[dependencies]
bar = "0.1.0"
```
This could compile if `target_os = "macos"` was a subset of `target_family = "unix"`.

Specifically, two extra relations can be defined:
- `cfg(target_os = "windows")` ⊆ `cfg(target_family = "windows")`.
- `cfg(target_os = <unix-os>)` ⊆ `cfg(target_family = "unix")`, where `<unix-os>` is any of
  `["freebsd", "linux", "netbsd", "redox", "illumos", "fuchsia", "emscripten", "android", "ios",
  "macos", "solaris"]`. This list needs to be updated if a new `unix` OS is supported by `rustc`'s
  official target list. This would make the first example compile.

_Note:_ The contrapositive of these relations is also true.

Also, `target_family` is currently defined as not having mutually exclusive elements. This is
because `target_family = "wasm"` is not mutually exclusive with other target families. But,
`target_family = "unix"` could be defined as mutually exclusive with `target_family = "windows"` to
increase usability. By extension, `target_family = "windows"` would now be mutually exclusive with
`target_os = "linux"`, for example.

_Note:_ More relations could be defined, for example `target_feature = "neon"` ⊆ `target_arch =
"arm"`. With this however, things start to get complicated.

## Lint against unused target-specific tables

If a package has:
```toml
[package]
name = "example"
# ...
supported-targets = 'cfg(target_os = "linux")'

[target.'cfg(target_os = "windows")'.dependencies]
# ...
```

A lint could be added to highlight the fact that the `[target]` table is unused.

Exception should be made for `target.'cfg(any())'`/`target.'cfg(false)` tables, as they are often
used to lock the version of transitive dependencies, and should not be linted against.

## `supported-targets` at the cargo-target level

The `supported-targets` field could also be added at the cargo-target level to have more
fine-grained control over which targets a cargo-target supports. This would function similarly to
the `edition` field, which is available at both the package and the cargo-target level. The
`supported-targets` of a cargo-target would most likely need to be a subset of the package's
`supported-targets`.

This could also allow for a cargo-target to be swapped out based on the selected target. For example,
one could specify which binary should be used as `main` based on the selected target
[#9208](https://github.com/rust-lang/cargo/issues/9208).
.

This could also help WebAssembly targets, as `wasm` executables need to be built as libraries to then
be executed by a JavaScript environment [#12260](https://github.com/rust-lang/cargo/issues/12260).

## Interaction with crate features

Currently, crate `[features]` and `supported-targets` do not interact. It is possible however that a
crate feature interacts with the set of `supported-targets`, either by restraining or expanding it.
It could be possible to allow crate features to modify the `supported-targets` of a package.

## Misc

- Have `cargo add` check the `supported-targets` before adding a dependency.
- Show which targets are supported on `docs.rs`.
- Have search filters on `crates.io` for crates with support for specific targets.
