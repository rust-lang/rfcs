- Feature Name: `required-targets`
- Start Date: 2026-10-02
- Pre-RFC:
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

The word _target_ is extensively used in this document. The
[glossary](https://doc.rust-lang.org/cargo/appendix/glossary.html#target) defines its many meanings.
Here, _target_ refers to the "Target Architecture" for which a package is built. Otherwise, the
terms "cargo-target" and "target-tuple" are used in accordance with their definitions in the
glossary.

# Summary

The `required-targets` field in the `[package]` table of `Cargo.toml` lets workspace members
declare target requirements using
[`cfg` syntax](https://doc.rust-lang.org/reference/conditional-compilation.html). During local
development, commands such as `cargo check --workspace` skip workspace members whose requirements
do not match the selected target, much like `required-features` skips cargo-targets when their
required features are not enabled.

```toml
[package]
name = "hello_cargo"
required-targets = 'cfg(any(target_os = "linux", target_os = "macos"))'
```

For example, the following command skips this workspace member when checking for Windows:

```console
$ cargo check --workspace --target x86_64-pc-windows-msvc
```

# Motivation

_For more background, see [rust-lang/cargo#6179](https://github.com/rust-lang/cargo/issues/6179)_

Some packages rely on features or behavior that is not available on every platform `rustc` can target. Currently, there is no way to formally
specify platform requirements of a package.

When working on a project with packages that only build on certain platforms, users cannot run Cargo commands across the entire workspace (e.g. `cargo test --workspace`) but must individually select packages that only work on the specific platform (e.g. `cargo test --workspace --exclude firmware`).  This extends to CI with people wanting to write matrix jobs but have to hand maintain the list of packages for each platform in the matrix.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The `required-targets` field can be added to `Cargo.toml` under the `[package]` table to declare
target requirements for local development. Cargo uses these requirements to skip workspace members
that do not support the selected target.

This field is a string containing a `cfg` specification (as for the `[target.'cfg(**)']` table). The
supported `cfg` syntax is the same as the one for [platform-specific
dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#platform-specific-dependencies)
(i.e., `cfg(feature = "...")`, `cfg(test)`, `cfg(debug_assertions)`, and `cfg(proc_macro)` are not supported).

__For example:__
```toml
[package]
name = "hello_cargo"
version = "0.1.0"
edition = "2021"
required-targets = 'cfg(any(target_os = "linux", target_os = "macos"))'
```

This workspace member declares that it requires Linux or macOS. When checking the workspace for
Windows, Cargo skips `hello_cargo`:

```console
$ cargo check --workspace --target x86_64-pc-windows-msvc
```

Explicitly selecting `hello_cargo` for the same target instead produces an error:

```console
$ cargo check --package hello_cargo --target x86_64-pc-windows-msvc
```

An error is also raised when building a standalone package outside an explicitly declared workspace
for a target that does not satisfy its requirements. This distinction lets workspace commands skip
incompatible members while reporting an error when the user specifically asks to build one.

Documentation can be built for a matching target with `cargo doc --target <target>`. For docs.rs
builds, targets can be configured through [package metadata](https://docs.rs/about/metadata).

These requirements apply only during local development. `cargo package` and `cargo publish` strip
the field from the packaged `Cargo.toml`, so it does not restrict users of the published package.
Checking compatibility between a package's requirements and those of its dependencies is left as
a [future possibility](#ensuring-proper-use-of-dependencies).

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The `required-targets` field is an optional key that tells Cargo which targets the package can be
built for. The field does not impose requirements on the build host.
```toml
[package]
# ...
required-targets = 'cfg(any(target_os = "linux", target_os = "macos"))'
```
The value of this field must respect the [`cfg` syntax](https://doc.rust-lang.org/reference/conditional-compilation.html),
and does __not__ accept `cfg(feature = "...")`, `cfg(test)`, `cfg(debug_assertions)`, or
`cfg(proc_macro)` as configuration options.
A malformed `required-targets` field will raise an error.

For package selection, omitting `required-targets` has the same effect as specifying `'cfg(all())'`:
the package is eligible for every target.

When Cargo selects packages for compilation, checking, or documentation (e.g. `build`, `check`,
`run`, `clippy`, `doc`), it checks that the selected target satisfies the `required-targets` of
each directly selected package. Commands that do not perform these operations, such as `clean`,
`fetch`, `tree`, and `fmt`, do not check `required-targets`, even if they accept `--target`.

Cargo determines the selected target using its existing selection rules, including
[per-package target settings](https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#per-package-target).
Cargo checks the requirement separately for each selected target, before adjusting individual
cargo-targets to build for the host. A directly selected proc-macro package is therefore checked
against the package's selected target, even though its proc-macro library is compiled for the host.

As this field is limited to local development, `cargo package` / `cargo publish` strip it from the
normalized `Cargo.toml`. Retaining the field in that manifest is left as a
[future possibility](#future-possibilities).

This field supports [workspace inheritance](https://doc.rust-lang.org/cargo/reference/workspaces.html#the-package-table).
For example, a workspace can declare shared requirements:

```toml
# Workspace Cargo.toml
[workspace.package]
required-targets = 'cfg(any(target_os = "linux", target_os = "macos"))'
```

A member opts in to those requirements:

```toml
# hello_cargo/Cargo.toml
[package]
required-targets.workspace = true
```

## Ignoring builds for unsupported targets
[ignoring-builds]: #ignoring-builds-for-unsupported-targets

After normal package selection, Cargo skips incompatible members of an explicitly declared workspace
when no package is specified with `--package`. This also applies when Cargo is invoked from a
member's directory. If a package is specified with `--package`, or Cargo is invoked on a standalone
package outside an explicitly declared workspace, a target mismatch raises an error. The intent is
to mimic the behavior of `required-features` with package filtering based on targets, as reflected in the
[field name](#naming).

Cargo removes packages skipped for every selected target from the selected packages used to resolve
dependency features for compilation. Features required through retained dependencies still apply.
With [`resolver.feature-unification = "workspace"`](https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#resolverfeature-unification),
all workspace members continue contributing dependency features. This does not change lockfile
resolution or feature unification between packages retained for different targets.

# Drawbacks
[drawbacks]: #drawbacks

- Adding target requirements to package selection increases Cargo's complexity.
- Authors must maintain accurate target requirements. An overly restrictive condition can exclude
  a package from workspace checks on a target it actually supports.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Conditional workspace membership

Alternatively, instead of adding a `required-targets` package field, Cargo could make
`workspace.members` and `workspace.exclude` conditional on target `cfg` expressions.

For example, an extension could look like:

```toml
[workspace.'cfg(any(target_os = "linux", target_os = "macos"))']
members = ["hello_cargo"]
```

This would make `hello_cargo` a workspace member only when the selected target is Linux or macOS.

However, this approach would have several drawbacks:

- A package's workspace membership would change depending on the selected target.
- Target conditions for individual packages would be stored in the workspace manifest instead of
  their own manifests.
- Excluding a package from a workspace does not necessarily mean it cannot build for that target,
  so conditional membership does not directly provide the target requirements needed for
  [future dependency compatibility checks](#ensuring-proper-use-of-dependencies).

With `required-targets`, packages remain workspace members while only their selection for a build
changes. Each package declares its own target requirements, which could also support those future
checks.

## Format

The `cfg` string format was chosen because of its simplicity and expressiveness.
Other formats can be considered:

Using a list of `cfg` strings, and also accepting explicit target-tuples:
```toml
required-targets = [
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

A `[supported]` table, with `arch = ["<arch>", ...]`, `os = ["<os>", ...]`, `target = ["<target>",
...]`, etc. This is more verbose, complex to implement, learn, and remember. It is also not obvious
how `not` and `all` could be represented in this format. For example:
```toml
[supported]
os = ["linux", "macos"]
arch = ["x86_64"]
```

## Naming
[naming]: #naming

The name `required-targets` follows `required-features`: both express requirements that must be
satisfied for a package or cargo-target to be included in a build.

Unlike the list in `required-features`, `required-targets` contains a single `cfg` expression.
Conjunctions and disjunctions are explicit through `all(...)` and `any(...)`.

Some other names for this field can be considered:

- `targets`. As in "this package _targets_ ...". Pro: Concise. Con: Ambiguous, and could be confused
  with the `target` table.

## Package scope vs. cargo-target scope

The `required-targets` field is placed at the package level, and not at the cargo-target level
(i.e., under `[lib]`, `[[bin]]`, etc.)

It is possible to allow cargo-targets to further restrict the `required-targets` of the package,
but this is left as a [future possibility](#future-possibilities).

See also: [using a package vs. using a workspace][package-vs-workspace].

[package-vs-workspace]: https://blog.rust-lang.org/inside-rust/2024/02/13/this-development-cycle-in-cargo-1-77/#when-to-use-packages-or-workspaces

## Field format 

Cargo already evaluates `cfg` expressions for platform-specific dependencies using cached target
information obtained from `rustc`. `required-targets` uses the same kind of matching for the selected
target. Comparing sets of allowed targets is only needed for the
[future dependency compatibility checks](#ensuring-proper-use-of-dependencies).
Some alternative formats are discussed here along with their drawbacks.

### Target-tuples

The initial proposal allowed both the `cfg` syntax and whitelisting specific target-tuples, to follow the behavior of [platform-specific
dependency tables](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#platform-specific-dependencies).
This was removed as it was deemed better to accept targets based on their _attributes_ rather than on their
_name_. Indeed, `rustc` supported target-tuples have changed names, and have been added or removed in the past.
Target-tuple names also do not encapsulate the semantics of the target.

### Using wildcards

Instead of using `cfg` specifications, one could use wildcards (e.g., `x86_64-*-linux-*`) to match
target-tuple names directly. However, this is not as expressive as `cfg`, and does not correctly
represent the semantics of target-tuples. For example, supporting `target_family = "unix"` would
require an annoyingly long list of wildcard patterns. Things like `target_pointer_width = "32"` are
even harder to represent, and things like `target_feature = "avx"` are basically not representable.
Also, this is new syntax not currently used by Cargo.

### Allowing only target-tuples

This is an even stricter version of the above. Explicit target-tuple lists would simplify set
comparisons for future dependency compatibility checks. However, this alternative may not be
expressive enough for the common use case. Packages rarely support specific target-tuples, rather
they support/require specific target attributes. What would
likely happen is that packages would copy and paste the target-tuple list matching their
requirements from somewhere or someone else. Every time a new target with the same attribute is
added, the whole ecosystem would have to be updated.

# Prior art
[prior-art]: #prior-art

Users can already select which packages they want to select in a workspace with the flags
`--package` and `--exclude`. Cargo features can also be used to restrict which cargo-target
is built using the `required-features` field. However, `required-features` does not allow filtering
packages in a workspace, nor does it allow filtering out the library of a package.

The `per-package-target` nightly feature defines the `forced-target` field, which forces a package
to build for a specific target-tuple. `required-targets` instead determines whether a package is
included for the selected target. It does not select a different target, so it does not replace
`forced-target` for workflows that build packages for different targets in one command.

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

# Future possibilities
[future-possibilities]: #future-possibilities

## Additional target conditions

Additional conditions could express standard-library support, whether the selected target matches
the host, or target support tiers.

## Workspace selection of build tools

A complementary feature could let workspace members that provide procedural macros or build-script
helpers opt out of bulk workspace selection by commands such as `cargo check --workspace`, while
still being built as dependencies or explicitly checked and tested.

For example, see [Alacritty's duplicate proc-macro builds](https://github.com/rust-lang/cargo/issues/13321)
or [Stellar's workspace exclusion workaround](https://github.com/rust-lang/cargo/issues/10827).

An always-false `required-targets` condition also skips direct workspace builds, but prevents
explicitly checking or testing the package.

## Target-specific dependency resolution

`Cargo.lock`, and by extension, `cargo vendor`, must assume that a package may be built on any platform that has or will exist.  This means that if a transitive dependency pulls in Windows-specific dependencies, `cargo vendor` will include them when run on a Linux-only application.  Being able to tell `cargo vendor` what platforms to care about can reduce the space used in a repo and reduce churn.

Likewise, today users either need to audit dependencies irrelevant for the platforms they target or
filter these out somehow. Target-specific dependency resolution could let audit tools focus on
dependencies for the configured targets.

Target-specific dependency resolution and vendoring can proceed independently of this RFC.

A separate `resolver.targets` setting in `.cargo/config.toml` could specify concrete target-tuples
for dependency resolution and vendoring. This would describe an application's deployment targets
rather than the package's target requirements.

### Eliminating unused dependencies from `Cargo.lock`

A package's dependencies may themselves have `[target.'cfg(..)'.dependencies]` tables, which may
never be used for the targets specified in `resolver.targets`. Omitting these dependencies from
`Cargo.lock` could reduce the packages included by `cargo vendor`.

Consider an application with the following `.cargo/config.toml`:

```toml
[resolver]
targets = ["x86_64-unknown-linux-gnu"]
```

Its dependency manifests are:

```toml
[package]
name = "foo"
# ...

[dependencies]
bar = "0.1.0"
```
```toml
[package]
name = "bar"

[target.'cfg(target_os = "macos")'.dependencies]
baz = "0.1.0"
```
Currently, `baz` is included in the dependency tree of `foo`. With resolution restricted to the
configured Linux target, the macOS-only dependency on `baz` would not be needed. If no other
selected dependency path needs `baz`, it could be omitted from `Cargo.lock` and vendoring.

Dependencies used by build scripts and procedural macros must still be considered for the host,
which can differ from the configured deployment targets. Restricting deployment targets must not
remove dependencies needed to build on that host.

Open questions for this separate design include:

- [Dependency-path pruning](https://github.com/rust-lang/rfcs/pull/3759#discussion_r1973807418).
- [Lockfile stability across Cargo versions](https://github.com/rust-lang/rfcs/pull/3759#discussion_r1973817534).
- [Whether to record resolution targets and how to publish target-restricted lockfiles](https://github.com/rust-lang/rfcs/pull/3759#discussion_r1973868712).

## Ensuring proper use of dependencies

Missing target capabilities, such as particular atomic operations, can produce errors about
unavailable APIs. Some of these problems won't be found until you've built or tested your project
on an affected target. Like with [#2495](https://rust-lang.github.io/rfcs/2495-min-rust-version.html),
declared requirements could let Cargo identify an incompatible dependency before compiling it.

For example, a warning or an error could be raised if a package uses a dependency that does not
accept the package's `required-targets`:
```toml
[package]
name = "bar"
required-targets = 'cfg(target_os = "windows")'
```
```toml
[package]
name = "foo"
required-targets = 'cfg(target_os = "linux")'

[dependencies]
bar = "0.1.0"
```
Here, a compilation error helps by showing which dependency is incompatible with the package's
`required-targets`, rather than a cryptic error message about unavailable APIs, or runtime
errors.

Cargo's documentation should give clear guidance for when to use this field, and should not suggest
using it by default. In particular, we should steer users to use this when they have good reason to
believe the crate will not compile or work as expected (e.g. because it uses target-specific APIs),
and not use it merely for "I haven't personally tested this on other targets".

Even then, it will happen that crates unnecessarily limit their dependents and users because of 
overly restrictive `required-targets`.
Some options for handling this include
- Doing nothing, encouraging people to upstream patches
- Encourage `[patch]`ing the dependency
  - Requires managing a fork
  - Every dependent of the package with a questionable `required-targets` must do this
- Encourage unidiff `[patch]`es
  - Design has unresolved questions ([cargo#4648](https://github.com/rust-lang/cargo/issues/4648))
  - Every dependent of the package with a questionable `required-targets` must do this
- A bespoke manifest override
  - One-off feature that needs design work
- A CLI override like `--ignore-rust-version`
  - Ignoring package requirements need not affect lockfile pruning based on a separate `resolver.targets` setting
  - This affects the entire dependency tree and not just the package with questionable `required-targets`
  - Every dependent of the package with a questionable `required-targets` must do this
- A lint like proposed for `package.rust-version`
  - See also CLI override
- Allow a registry database to override `required-targets`
  - Blocked on a lot of design work ([related discussion](https://blog.rust-lang.org/inside-rust/2024/03/26/this-development-cycle-in-cargo-1.78.html#why-is-this-yanked))

### Compatibility of `[dependencies]`

For dependencies built for the same target as the package, one could restrict the package's
`required-targets` to be a subset of each dependency's `required-targets`. If the crate itself had
no `required-targets` specified, then those dependencies would need to support all targets.

Alternatively, omitting `required-targets` could opt out of dependency compatibility checks.
Explicitly specifying `required-targets = 'cfg(all())'` would request those checks for all targets.
This distinction would not affect package selection.

If a dependency does not respect this requirement (if it is not compatible), an error would be
raised and the build would fail.

Enforcing this means a package cannot support targets that are not supported by dependencies built
for those targets, assuming the dependencies have correctly specified their `required-targets`.

Procedural macros and their dependencies are built for the host, so dependency compatibility checks
would need to consider the host target, as with build dependencies. This requires distinguishing
host build requirements from the selected-target eligibility defined by this RFC. A proc-macro's
`required-targets` condition cannot simply be reinterpreted as a host requirement.

### Compatibility of `[dev-dependencies]`

`[dev-dependencies]` should be checked using the same method as regular `[dependencies]`, including
the separate consideration of host compatibility for procedural macros. For dependencies built for
the package's target, the package's `required-targets` needs to be a subset of each dependency's
`required-targets`.
The rationale is that an example, test, or benchmark has access to the
package's library and binaries, and so it must respect the `required-targets` of the package.

### Compatibility of `[build-dependencies]`
[build-dependencies-compatibility]: #compatibility-of-build-dependencies

Build dependencies are built for the host computer, and not the selected target. As such,
they are not restrained by the `required-targets` of the package. Hence,
all dependencies are allowed in the `[build-dependencies]` table. However, a build error could be
raised if one of the build dependencies does not support the _host-tuple_ at build time.

A problem can arise if a crate's build script depends on a package that does not support `target_os
= "windows"` for example. It would be possible to only allow dependencies supporting all targets in
`[build-dependencies]`.


### Platform-specific dependencies

Platform-specific dependencies are dependencies under the `[target.**]` table. This includes normal
dependencies, build-dependencies, and dev-dependencies. Rules could be defined to ensure that
platform-specific dependencies are declared correctly.

For platform-specific dependencies built for the package's target, each dependency's
`required-targets` would need to include the _intersection_ of the package's `required-targets` and
the target condition under which the dependency is declared. For example:
```toml
[package]
# ...
required-targets = 'cfg(target_os = "linux")'

[target.'cfg(target_pointer_width = "64")'.dependencies]
foo = "0.1.0"
```
Here, it would suffice for `foo` to support `cfg(all(target_os = "linux", target_pointer_width =
"64"))`.

This would ensure that a package properly uses dependencies that are not available on all targets.
Assuming that the crate `io-uring` has `required-targets = 'cfg(target_os = "linux")'`, a crate
could depend on it using:
```toml
[package]
# ...

[target.'cfg(target_os = "linux")'.dependencies]
io-uring = "0.1.0"
```
This would not be required if the package itself had `required-targets = 'cfg(target_os =
"linux")'`, or an even stricter set.

### Artifact dependencies

Each artifact would be checked against the dependency's `required-targets` using the target it is
built for. An explicit `target` field selects that target. Without it, build-dependency artifacts
use the host target, while other artifacts use the declaring package's target.
`target = "target"` makes a build-dependency artifact use the declaring package's target.

With `lib = true`, the dependency also provides an ordinary Rust library. That library would be
checked separately using the rules for its dependency kind. An artifact's `target` override does
not change the target used for this additional library build.

### Comparing `required-targets`

`required-targets` uses Rust's existing `cfg` syntax to check the selected target. The
[future dependency compatibility checks](#ensuring-proper-use-of-dependencies) instead compare
the sets of targets that two expressions allow, to determine whether one is a _subset_ of the
other or whether they are _mutually exclusive_.

The rules below give sufficient conditions for proving these relations, but do not cover every
equivalent expression. Failure to prove a relation with these rules does not mean it is false.
A complete comparison algorithm and handling of inconclusive results remain to be defined.

#### Flattening `not`, `any`, and `all` in `cfg` specifications

To compare these sets, each expression is rewritten in
[disjunctive normal form](https://en.wikipedia.org/wiki/Disjunctive_normal_form). In the worst case,
the number of terms grows exponentially with the size of the original expression.

The `not` operator is "passed through" `any` and `all` operators using [De Morgan's
laws](https://en.wikipedia.org/wiki/De_Morgan%27s_laws), until it reaches a single `cfg`
specification. For example, `cfg(not(all(target_os = "linux", target_arch = "x86_64")))` is
equivalent to `cfg(any(not(target_os = "linux"), not(target_arch = "x86_64")))`.

The `cfg` definition is transformed into `any` of `all` (top level union).

Top level `all` operators are kept as is, as long as they do not contain nested `any`s or `all`s. If
there is an `any` inside an `all`, the statement is split into multiple `all` statements. For
example,
```toml
required-targets = 'cfg(all(target_os = "linux", any(target_arch = "x86_64", target_arch = "arm")))'
```
is transformed into
```toml
required-targets = 'cfg(any(all(target_os = "linux", target_arch = "x86_64"), all(target_os = "linux", target_arch = "arm")))'
```
If an `all` contains an `all`, the inner `all` is flattened into the outer `all`.

The result is a union of individual conditions or `all` groups with no nested `any` or `all`
operators. Individual conditions may still be negated with `not`.

#### The subset relation

To determine if the `required-targets` set "A" is a subset of another such set "B", the standard
mathematical definition of subset is used. That is, "A" is a subset of "B" if and only if each
element of "A" is contained in "B".

One sufficient test is to show that each term of the union forming "A" is contained in at least
one term of the union forming "B". This does not cover cases where several terms of "B" together
cover a term of "A".
A `cfg(all(A, B, ...))` is a subset of a `cfg(all(C, D ...))`, if the list `C, D, ...` is a subset
of the list `A, B, ...`. For negated conditions, `cfg(A)` is a subset of `cfg(not(B))` if
`cfg(A)` and `cfg(B)` are mutually exclusive.
For example, `cfg(target_os = "linux")` is a subset of `cfg(not(target_os = "windows"))`, because
a target cannot have both operating system values.

_Note_: `cfg(A) == cfg(all(A))`.

#### Mutual exclusivity

For the `required-targets` set "A" to be mutually exclusive with another such set "B", each element
of "A" must be mutually exclusive with _all_ elements of "B" (The inverse is also true).

So each element of "A" is compared against each element of "B". A `cfg(all(A, B, ...))` is mutually
exclusive with a `cfg(all(C, D, ...))` if any element of the list `A, B, ...` is mutually exclusive
with any element of the list `C, D, ...`.

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
required-targets = 'cfg(target_family = "unix")'
# ...
```
```toml
[package]
name = "foo"
required-targets = 'cfg(target_os = "macos")'

[dependencies]
bar = "0.1.0"
```
This could compile if `target_os = "macos"` was a subset of `target_family = "unix"`.

The following relations are valid only for a set of targets whose definitions are known to satisfy
them, such as verified built-in targets. They must not be assumed for arbitrary
[custom targets](https://doc.rust-lang.org/rustc/targets/custom.html), which can use
`target_os = "linux"` without the Unix family or belong to both the Unix and Windows families.

Specifically, two extra relations can be defined:
- `cfg(target_os = "windows")` ⊆ `cfg(target_family = "windows")`.
- `cfg(target_os = <unix-os>)` ⊆ `cfg(target_family = "unix")`. Examples of `<unix-os>` include
  `["freebsd", "linux", "netbsd", "redox", "illumos", "fuchsia", "emscripten", "android", "ios",
  "macos", "solaris"]`. These relations need to be kept in sync with the target definitions used
  for the comparison. This would make the first example compile.

_Note:_ The contrapositive of these relations is also true.

Also, `target_family` is currently defined as not having mutually exclusive elements. This is
because `target_family = "wasm"` is not mutually exclusive with other target families. But,
`target_family = "unix"` could be defined as mutually exclusive with `target_family = "windows"` to
increase usability. By extension, `target_family = "windows"` would now be mutually exclusive with
`target_os = "linux"`, for example.

## Lint against unused target-specific tables

If a package has:
```toml
[package]
name = "example"
# ...
required-targets = 'cfg(target_os = "linux")'

[target.'cfg(target_os = "windows")'.dependencies]
# ...
```

A lint could be added to highlight the fact that the `[target]` table is unused.

An exception should be made for `target.'cfg(any())'`/`target.'cfg(false)'` tables, as they are often
used to lock the version of transitive dependencies, and should not be linted against.

## `required-targets` at the cargo-target level

The `required-targets` field could also be added at the cargo-target level to have more
fine-grained control over which targets a cargo-target supports. The
`required-targets` of a cargo-target would most likely need to be a subset of the package's
`required-targets`.

This could also allow for a cargo-target to be swapped out based on the selected target. For example,
one could specify which binary should be used as `main` based on the selected target
[#9208](https://github.com/rust-lang/cargo/issues/9208).

This could also let a package select its `cdylib` library target for `wasm-pack` builds and its
binary target for desktop builds, as requested in [#12260](https://github.com/rust-lang/cargo/issues/12260).

## Interaction with crate features

Currently, crate features do not change a package's `required-targets`. Crate features could be
allowed to modify these requirements to restrict or expand the set of permitted targets.

## Misc

- Have `cargo add` check the `required-targets` before adding a dependency.
- Show which targets are supported on `docs.rs`.
- Have search filters on `crates.io` for crates with support for specific targets.
