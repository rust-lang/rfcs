- Feature Name: `required-targets`
- Start Date: 2026-10-02
- Pre-RFC:
- RFC PR: [rust-lang/rfcs#4013](https://github.com/rust-lang/rfcs/pull/4013)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary

The `required-targets` field in the `[package]` table of `Cargo.toml` lets workspace members
declare target requirements using [`cfg` syntax](https://doc.rust-lang.org/reference/conditional-compilation.html).
Commands such as `cargo check --workspace` skip workspace members whose requirements do not match the selected target, much like
[`required-features`](https://doc.rust-lang.org/cargo/reference/cargo-targets.html#the-required-features-field)
skips cargo-targets when their required features are not enabled.

```toml
[package]
name = "hello_cargo"
required-targets = 'cfg(any(target_os = "linux", target_os = "macos"))'
```

# Motivation

_For more background, see [rust-lang/cargo#6179](https://github.com/rust-lang/cargo/issues/6179)_

Some packages rely on features or behavior that is not available on every platform `rustc` can target. Currently, there is no way to formally
specify platform requirements of a package.

When working on a project with packages that only build on certain platforms, users cannot run Cargo commands across the entire workspace (e.g. `cargo test --workspace`) but must individually select packages that only work on the specific platform (e.g. `cargo test --workspace --exclude firmware`).  This extends to CI with people wanting to write matrix jobs but have to hand maintain the list of packages for each platform in the matrix.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The word _target_ is extensively used in this document. The
[glossary](https://doc.rust-lang.org/cargo/appendix/glossary.html#target) defines its many meanings.
Here, _target_ refers to the "Target Architecture" for which a package is built. Otherwise, the
terms "cargo-target" and "target-tuple" are used in accordance with their definitions in the
glossary.

The `required-targets` field can be added to `Cargo.toml` under the `[package]` table to declare
target requirements. Cargo uses these requirements to skip workspace members
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

This workspace member declares that it requires Linux or macOS. Consider a workspace containing
this package and a portable tool with no `required-targets` field:

```text
workspace
├── hello_cargo     requires Linux or macOS
└── portable_tool   no target requirements
```

When checking the workspace for Windows, Cargo skips `hello_cargo` and checks `portable_tool`.
For example:

```console
$ cargo check --workspace --target x86_64-pc-windows-msvc
    Skipping hello_cargo (`required-targets` not satisfied by x86_64-pc-windows-msvc)
```

Explicitly selecting `hello_cargo` for the same target instead produces an error. For example:

```console
$ cargo check --package hello_cargo --target x86_64-pc-windows-msvc
error: package `hello_cargo` does not support target `x86_64-pc-windows-msvc`
```

An error is also raised when building a standalone package outside an explicitly declared workspace
for a target that does not satisfy its requirements. This distinction lets workspace commands skip
incompatible members while reporting an error when the user specifically asks to build one.

Documentation can be built for a matching target with `cargo doc --target <target>`. For docs.rs
builds, targets can be configured through [package metadata](https://docs.rs/about/metadata).

`cargo install` checks `required-targets` when installing a published package.
Cargo also checks each dependency's requirements against the target that dependency is built for,
and reports an error on a mismatch. Comparing the full sets of targets allowed by a package and
its dependencies is left as a [future possibility](#dependency-compatibility-checks).

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Manifest field

The `required-targets` field is an optional key that controls whether Cargo directly selects a
package for a given target and whether it can build that package as a dependency.
```toml
[package]
# ...
required-targets = 'cfg(any(target_os = "linux", target_os = "macos"))'
```
The value of this field must respect the [`cfg` syntax](https://doc.rust-lang.org/reference/conditional-compilation.html),
and does __not__ accept `cfg(feature = "...")`, `cfg(test)`, `cfg(debug_assertions)`, or
`cfg(proc_macro)` as configuration options. `RUSTFLAGS` does not affect the evaluation of `required-targets`.
A malformed `required-targets` field will raise an error.

Package selection and dependency checks use the same compiler-provided configuration for a given
target, with the exclusions above and without additions from build scripts or profile settings.

Omitting `required-targets` has the same effect as specifying `'cfg(all())'` for these checks:
the package is eligible for every target.

## Package selection

Commands that support target selection through `--target`, except `cargo package` and `cargo publish`,
check that the selected target satisfies the `required-targets` of each directly selected package.
Examples include `cargo build`, `cargo check`, `cargo doc`, `cargo fetch`, and `cargo tree`.
`cargo package` and `cargo publish` check the requirement only for [verification](#packaging).
Commands without target selection, such as `cargo fmt`, do not check `required-targets`.

Cargo determines the selected target using its existing selection rules, including
[per-package target settings](https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#per-package-target).
Cargo checks the requirement for the package as a whole, separately for each selected target,
before adjusting individual cargo-targets to build for the host. A directly selected proc-macro
package is therefore checked against the package's selected target, even though its proc-macro
library is compiled for the host.

After normal package selection, Cargo handles a target mismatch as follows:

| Package selection | Result for an incompatible package |
| --- | --- |
| Selection within an explicitly declared workspace without `--package`, including from a member's directory | Skip |
| Explicit selection with `--package` | Error |
| Standalone package outside an explicitly declared workspace | Error |

Cargo removes packages skipped for every selected target from the selected packages used to resolve
dependency features for compilation. Features required through retained dependencies still apply.
With [`resolver.feature-unification = "workspace"`](https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#resolverfeature-unification),
all workspace members continue contributing dependency features. This does not change lockfile
resolution or feature unification between packages retained for different targets.

It's worth noting that how skipped packages contribute dependency features may change as
`feature-unification` moves towards [stabilization](https://github.com/rust-lang/cargo/issues/14774).

## Dependencies

Before building a dependency, Cargo checks its `required-targets` against its compilation target
and reports an error on a mismatch. This applies to every direct or transitive dependency needed
for the build, regardless of its source. A skipped workspace member is still checked if needed
as a dependency. Inactive dependencies are not checked, and these checks do not change dependency
version selection or prune `Cargo.lock`.

These checks use the same target configuration and `cfg` exclusions as package selection,
as described under [Manifest field](#manifest-field), including ignoring `RUSTFLAGS`.

Dependencies compiled for the host, including procedural macros and build dependencies, are checked
against the host target. A dependency built for multiple targets must satisfy its requirement for
each build.

Compiling a package's own build script for the host does not require the package's `required-targets`
to match the host. Its build dependencies are checked against their own requirements.

Artifact dependencies are checked against each artifact's compilation target. With `lib = true`,
the additional library is checked separately against its own compilation target.

### Proc-macro packages

Consider a package `foo` with no `required-targets` that depends on a proc-macro package
`foo-gen` declaring `required-targets = 'cfg(target_os = "macos")'`. Assume `foo-gen`'s implementation
uses only macOS APIs, but `foo` and the code generated by `foo-gen` compile for both Linux and macOS.
Without per-package target overrides, the outcomes of
`cargo check --package <package> --target <target>` are:

| Selected package | Host | `--target` | Check of `foo-gen`'s `required-targets` | Outcome |
| --- | --- | --- | --- | --- |
| `foo` | Linux | Linux | Error (host target) | Requirement error (`foo-gen` needs a macOS host) |
| `foo` | Linux | macOS | Error (host target) | Requirement error (`foo-gen` needs a macOS host) |
| `foo` | macOS | Linux | Pass (host target) | Success (`foo-gen` compiles for macOS, `foo` for Linux) |
| `foo` | macOS | macOS | Pass (host target) | Success (both compile for macOS) |
| `foo-gen` | Linux | Linux | Error | Requirement error (before compilation) |
| `foo-gen` | Linux | macOS | Pass | Compilation error (`foo-gen` needs a macOS host) |
| `foo-gen` | macOS | Linux | Error | Requirement error (before compilation, even with `--lib`) |
| `foo-gen` | macOS | macOS | Pass | Success (`foo-gen` compiles for macOS) |

The proc-macro library compiles for the host. When used as a dependency, its requirement is checked
against the host target. Direct package selection still checks the selected target, which can
reject an otherwise valid host library build.

## Packaging

`cargo package` and `cargo publish` do not filter package selection using `required-targets`.
Before their verification build, they check each package's requirement from the original manifest
against each selected target and report an error on a mismatch. `--no-verify` skips this check
along with the verification build, allowing authors to package and publish for other targets.

# Drawbacks
[drawbacks]: #drawbacks

- Adding target requirements to package selection increases Cargo's complexity.
- Authors must maintain accurate target requirements. An overly restrictive condition can exclude
  a package from workspace checks or reject a dependency build on a target it actually supports.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Do nothing

We could introduce no new features and continue selecting workspace packages with `--package`
and `--exclude`. Users would still need to maintain the appropriate package selection for each
target in their commands and CI configuration.

Published crates have mainly used their documentation to specify which targets they support, or they
would leave it up to the user to infer it. Some crates also made use of compile time errors to
ensure that `cfg` requirements are met, for example:
```rust
#[cfg(not(any(…)))]
compile_error!("unsupported target cfg");
```
[`getrandom`](https://github.com/rust-random/getrandom/blob/9fb4a9a2481018e4ab58d597ecd167a609033149/src/backends.rs#L156-L160)
is an example of a crate utilizing this method.

These approaches do not automatically skip incompatible workspace packages.

## Stripping `required-targets` on publication

Cargo could strip `required-targets` from published manifests to prevent incorrect or overly
restrictive declarations from rejecting otherwise valid downstream builds. This would allow the
field to be tried in local development before enforcing it for users of published packages.

We have chosen to retain the field so Cargo can check the requirements of published dependencies
and packages installed with `cargo install`, reporting target mismatches before compilation.
Incorrect declarations should be fixed upstream, and authors may deliberately restrict targets
to those they can maintain even when the code could compile elsewhere.

## Overriding `required-targets`

Cargo could provide a flag to ignore `required-targets`, or make mismatches a deny-by-default lint
that users can disable. Either would allow otherwise valid builds when a declaration is too restrictive.

We have chosen not to introduce an override initially. Incorrect declarations should be fixed
upstream, and we'll see how it goes initially before deciding whether an override is needed.
Override mechanisms remain a future possibility.

## Using `forced-target`

The `per-package-target` nightly feature defines the `forced-target` field, which forces a package
to build for a specific target-tuple. `required-targets` instead determines whether a package is
included for the selected target. It does not select a different target, so it does not replace
the target-selection behavior of `forced-target` on its own. Alternatives for these workflows are
discussed under [removing `forced-target`](#removing-forced-target).

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
  [dependency checks](#dependencies).

With `required-targets`, packages remain workspace members. Each package declares its own target
requirements, which Cargo uses for package selection and dependency checks.

## Supported configuration options

`required-targets` describes target requirements for a whole package. `cfg(test)`,
`cfg(debug_assertions)`, and `cfg(proc_macro)` instead depend on the profile or the individual
crate being compiled, so their values can differ between builds of the same package.

Allowing `cfg(feature = "...")` would couple package selection to feature resolution, which package
selection itself affects. Feature-dependent requirements are left as a
[future possibility](#interaction-with-crate-features).

We exclude `RUSTFLAGS` to avoid adding further confusion over which `cfg` values apply, as seen
with differences between `[target.*.dependencies]` and Rust source code.

## Field syntax

The `cfg` string format was chosen because of its simplicity and expressiveness.

Cargo already evaluates `cfg` expressions for platform-specific dependencies using cached target
information obtained from `rustc`. `required-targets` uses the same kind of matching for the relevant
target. Comparing sets of allowed targets is only needed for the
[future dependency compatibility checks](#dependency-compatibility-checks).

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
If the list of supported targets is long, then the `Cargo.toml` file becomes
very verbose as well.

A `[supported]` table, with `arch = ["<arch>", ...]`, `os = ["<os>", ...]`, `target = ["<target>",
...]`, etc. This is more verbose, complex to implement, learn, and remember. It is also not obvious
how `not` and `all` could be represented in this format. For example:
```toml
[supported]
os = ["linux", "macos"]
arch = ["x86_64"]
```

### Target-tuples

The initial proposal allowed both the `cfg` syntax and whitelisting specific target-tuples, to follow the behavior of [platform-specific
dependency tables](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#platform-specific-dependencies).
This was removed as it was deemed better to accept targets based on their _attributes_ rather than on their
_name_. Indeed, `rustc` supported target-tuples have changed names, and have been added or removed in the past.
Target-tuple names also do not encapsulate the semantics of the target. Support for target tuples remains a [future possibility](#target-tuples-1).

### Allowing only target-tuples

This alternative accepts target-tuples without `cfg` expressions. Explicit target-tuple lists would simplify set
comparisons for future dependency compatibility checks. However, this alternative may not be expressive enough for the common use case.
Packages rarely support specific target-tuples, rather they support/require specific target attributes.
What would likely happen is that packages would copy and paste the target-tuple list matching their requirements from somewhere or someone else.
Every time a new target with the same attribute is added, the whole ecosystem would have to be updated.

### Using wildcards

Instead of using `cfg` specifications, one could use wildcards (e.g., `x86_64-*-linux-*`) to match
target-tuple names directly. However, this is not as expressive as `cfg`, and does not correctly
represent the semantics of target-tuples. For example, supporting `target_family = "unix"` would
require an annoyingly long list of wildcard patterns. Things like `target_pointer_width = "32"` are
even harder to represent, and things like `target_feature = "avx"` are basically not representable.
Also, this is new syntax not currently used by Cargo.

## Package scope vs. cargo-target scope

The `required-targets` field is placed at the package level, and not at the cargo-target level
(i.e., under `[lib]`, `[[bin]]`, etc.)

It is possible to allow cargo-targets to further restrict the `required-targets` of the package,
but this is left as a [future possibility](#future-possibilities).

See also: [using a package vs. using a workspace][package-vs-workspace].

[package-vs-workspace]: https://blog.rust-lang.org/inside-rust/2024/02/13/this-development-cycle-in-cargo-1-77/#when-to-use-packages-or-workspaces

# Prior art
[prior-art]: #prior-art

The `required-features` field can restrict which targets are built based on enabled features.
However, it does not allow filtering packages in a workspace, nor does it allow filtering out
the library of a package.

Other languages and build tools express platform compatibility at different stages:

| Feature | What it does | Comparison |
| --- | --- | --- |
| Python [classifiers](https://packaging.python.org/en/latest/guides/writing-pyproject-toml/#classifiers) | Describe supported platforms for searching and browsing, without enforcing installation restrictions. | `required-targets` affects which packages Cargo selects. |
| Python wheel [compatibility tags](https://packaging.python.org/en/latest/specifications/platform-compatibility-tags/#use) | Select compatible prebuilt distributions for installation, accounting for Python, ABI, and platform requirements. | This RFC checks target requirements for packages built from source rather than selecting compatible prebuilt artifacts. |
| npm [`os`](https://docs.npmjs.com/cli/v11/configuring-npm/package-json#os) and [`cpu`](https://docs.npmjs.com/cli/v11/configuring-npm/package-json#cpu) | Restrict installation to allowed platforms. Incompatible required dependencies cause errors, while [optional dependencies](https://docs.npmjs.com/cli/v11/configuring-npm/package-json#optionaldependencies) can be omitted. | `required-targets` also checks installation and dependencies. An enabled optional dependency that fails its requirement causes an error rather than being omitted. |
| Swift [supported platforms](https://docs.swift.org/package-manager/PackageDescription/PackageDescription.html#supportedplatform) | Set minimum deployment versions and check that dependencies do not require higher versions. | This RFC checks dependencies against their compilation targets, but does not introduce deployment-version requirements or compare packages' full sets of supported targets. |
| [Buck2](https://buck2.build/docs/concepts/configurations/#using-configuration-compatibility) and [Bazel](https://bazel.build/concepts/platforms#skipping-incompatible-targets) `target_compatible_with` | Skip incompatible targets selected through patterns and, by default, error on explicit selection. | Similar skip vs. error behavior. This RFC errors on incompatible dependencies rather than using them to skip selected packages, and applies requirements at the package level. |

Reusing Cargo's existing `cfg` syntax allows combinations such as permitting an architecture on
one operating system but not another, beyond npm's separate lists. It avoids introducing a
separate constraint system, but doesn't provide Buck2 and Bazel's general build-configuration
constraints.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

## Field name

The field's name will be resolved through the implementation and stabilization process.

Throughout this RFC, we use `required-targets` for now, following the naming of `required-features`.

Unlike the list in `required-features`, `required-targets` contains a single `cfg` expression.
Conjunctions and disjunctions are explicit through `all(...)` and `any(...)`.

Names under consideration:

| Name | Pros | Cons |
| --- | --- | --- |
| `required-targets` | Follows `required-features` for skipping items whose requirements are not met. | "Requires targets" can sound like all targets are required, rather than the selected target needing to satisfy a condition. |
| `supported-targets` | Naturally describes the targets a package supports. | "Supported" can suggest only targets actively tested by the author, encouraging overly restrictive declarations. |
| `allowed-targets` | Makes clear that targets outside the condition are rejected. | Loses the naming connection to `required-features` and its skip behavior. |
| `targets` | Concise, as in "this package targets ...". | Ambiguous and could be confused with the `target` table. |
| `known-broken-targets` | Encourages declaring known incompatibilities rather than restricting builds to tested targets. | Reverses the condition's meaning and requires authors to describe incompatible targets, which may be harder than stating a requirement. |

# Related work

## Target-specific dependency resolution

A separate RFC could introduce a `resolver.targets` setting to restrict dependency resolution to
an application's deployment targets, reducing the packages included in `Cargo.lock` and by
`cargo vendor`.

This is separate because an application's chosen deployment targets can be narrower than the
targets its packages support. A dedicated setting could name those targets directly without
requiring packages to restrict their `required-targets` just to reduce vendored dependencies.
Applications already choose target tuples for deployment, so using those directly avoids much of
the complexity of comparing `cfg` expressions. Pruning also raises separate questions about
keeping lockfile resolution stable and handling published lockfiles, which a separate RFC can address.

# Future possibilities
[future-possibilities]: #future-possibilities

## Workspace inheritance

`required-targets` could support [workspace inheritance](https://doc.rust-lang.org/cargo/reference/workspaces.html#the-package-table).
For example, a workspace could declare shared requirements:

```toml
# Workspace Cargo.toml
[workspace.package]
required-targets = 'cfg(any(target_os = "linux", target_os = "macos"))'
```

A member would opt in to those requirements:

```toml
# hello_cargo/Cargo.toml
[package]
required-targets.workspace = true
```

Sharing target requirements between some members may not justify workspace inheritance, since
newly created packages would inherit those requirements by default.

## Target tuples

`required-targets` could also accept target tuples alongside `cfg` expressions. One way to express
this would be an array containing either form. It's worth noting that the existing `required-features`
field uses AND for its array entries, whereas this array would use OR. The syntax needs further consideration.

## Removing `forced-target`

`forced-target` must select a specific target to build for, whereas a `cfg` expression can match
several targets without choosing one. `required-targets` can use these expressions to describe a
package's compatibility requirements, making it a more general way to express those requirements.
The unstable `forced-target` field could be removed if its target-selection use cases have
acceptable alternatives. Workflows to preserve include:

- [Building workspace packages for different targets](https://github.com/rust-lang/cargo/issues/7004)
  in one command.
- Building a component for a different target and consuming its compiled output, such as a
  WebAssembly component used by a native application.
- [Running a portable library's tests on the host](https://github.com/rust-lang/cargo/issues/17383#issuecomment-5377029221)
  while the workspace defaults to an embedded target.

Users of `forced-target` can migrate to this by changing `Cargo.toml`
```diff
-cargo-features = ["per-package-target"]
-
[package]
-forced-target = "wasm32-unknown-unknown"
+required-targets = 'cfg(target_arch = "wasm32")'
```

And adding to your `.cargo/config.toml`:
```diff
[build]
+target = ["host-tuple", "wasm32-unknown-unknown"]
```
With [target tuple support](#target-tuples-1), each package could use `required-targets` to match
its intended target and skip the other, allowing one workspace command to build packages for
different targets. However, this does not provide separate dependency feature resolution for each
target.

Other replacements or workarounds include explicit target selection,
separate Cargo invocations, and
[artifact dependencies](https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#artifact-dependencies).
Some workflows may still require separate commands or scripts, losing some of the convenience of
automatically selecting a fixed target for each package.

## Additional target conditions

Additional conditions could express standard-library support, whether the selected target matches
the host, or target support tiers.

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

## Workspace selection of build tools

When cross-compiling with `cargo check --workspace --target <target>`, a proc-macro workspace
member can be built as a host dependency and also built separately because it was directly selected.
A complementary feature could let workspace members that provide procedural macros or build-script
helpers opt out of that direct selection while still being built as dependencies and allowing
their tests to be run.

For example, see [Alacritty's duplicate proc-macro builds](https://github.com/rust-lang/cargo/issues/13321)
or [Stellar's workspace exclusion workaround](https://github.com/rust-lang/cargo/issues/10827).

An always-false `required-targets` condition also skips direct workspace builds, but prevents
explicitly checking or testing the package and rejects its use as a dependency.

## Dependency compatibility checks

This RFC checks each dependency's requirement against its compilation target. A further extension
could compare the full sets of targets allowed by packages and their dependencies, finding
inconsistencies without requiring a build for each target.

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
Here, comparing the declarations would reveal that the packages' requirements are mutually
exclusive, independently of the target selected for a particular build.

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
This distinction would not affect the checks against compilation targets defined by this RFC.

If a dependency does not respect this requirement (if it is not compatible), an error would be
raised and the build would fail.

Enforcing this means a package cannot support targets that are not supported by dependencies built
for those targets, assuming the dependencies have correctly specified their `required-targets`.

Procedural macros and their dependencies are built for the host. Comparing declarations would
therefore need to distinguish allowed host targets from the targets of the consuming package,
rather than requiring one set to be a subset of the other.

### Compatibility of `[dev-dependencies]`

`[dev-dependencies]` should be checked using the same method as regular `[dependencies]`, including
the separate consideration of host compatibility for procedural macros. For dependencies built for
the package's target, the package's `required-targets` needs to be a subset of each dependency's
`required-targets`.
The rationale is that an example, test, or benchmark has access to the
package's library and binaries, and so it must respect the `required-targets` of the package.

### Compatibility of `[build-dependencies]`
[build-dependencies-compatibility]: #compatibility-of-build-dependencies

Build dependencies are checked against the host target by this RFC. Comparing declarations would
need to account for the hosts on which a package is expected to build, separately from its
`required-targets` for the selected target.

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

### Comparing target requirements

`required-targets` uses Rust's existing `cfg` syntax to check a concrete target. The
[future dependency compatibility checks](#dependency-compatibility-checks) instead compare
the sets of targets that two expressions allow, to determine whether one is a _subset_ of the
other or whether they are _mutually exclusive_.

For example, using operating-system requirements:

| Package allows | Dependency allows | Relationship |
| --- | --- | --- |
| Linux | Linux or macOS | Package's targets are a subset |
| Linux | Windows | Mutually exclusive |
| Linux or macOS | Linux | Dependency does not cover every package target |

The rules below give sufficient conditions for proving these relations, but do not cover every
equivalent expression. Failure to prove a relation with these rules does not mean it is false.
A complete comparison algorithm and handling of inconclusive results remain to be defined.

<details>
<summary>Comparison algorithm details</summary>

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

</details>

## Checking for unexpected configuration options

Cargo could check `required-targets` for unexpected configuration names and values, similar to
the [`unexpected_cfgs` lint](https://doc.rust-lang.org/rustc/lints/listing/warn-by-default.html#unexpected-cfgs).
This would help catch typos such as `target_os = "windwos"` and confusion over which configuration
options the field supports.

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

## Tooling integration

- Expose `required-targets` through `cargo metadata` when needed by tools, for example to help
  generate CI target matrices without reparsing manifests.
- Have `cargo add` check the `required-targets` before adding a dependency.
- Show which targets are supported on `docs.rs`.
- Have search filters on `crates.io` for crates with support for specific targets.
