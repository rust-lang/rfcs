- Feature Name: `build-std-always`
- Start Date: 2025-06-05
- RFC PR: [rust-lang/rfcs#3874](https://github.com/rust-lang/rfcs/pull/3874)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary
[summary]: #summary

Add a new Cargo configuration option, `build-std = "always|never"`, which will
unconditionally rebuild standard library dependencies. The set of standard
library dependencies can optionally be customised with a new `build-std.crates`
option. It also describes how Cargo (or external tools) should build the
standard library crates on stable (i.e., which flags to pass and features to
enable).

This proposal limits the ways the built standard library can be customised (such
as by settings in the profile) and intends that the build standard library
matches the prebuilt one (if available) as closely as possible.

**This RFC is is part of the [build-std project goal] and a series of build-std
RFCs:**

1. build-std context ([rfcs#3873])
    - [Background][rfcs#3873-background]
    - [History][rfcs#3873-history]
    - [Motivation][rfcs#3873-motivation]
2. `build-std="always"` (this RFC)
    - [Proposal][proposal]
    - [Rationale and alternatives][rationale-and-alternatives]
    - [Unresolved questions][unresolved-questions]
    - [Future possibilities][future-possibilities]
    - [Summary of proposed changes][summary-of-changes]
3. Explicit standard library dependencies ([rfcs#3875])
    - [Proposal][rfcs#3875-proposal]
    - [Rationale and alternatives][rfcs#3875-rationale-and-alternatives]
    - [Unresolved questions][rfcs#3875-unresolved-questions]
    - [Future possibilities][rfcs#3875-future-possibilities]
4. `build-std="compatible"` (RFC not opened yet)
5. `build-std="match-profile"` (RFC not opened yet)

[build-std project goal]: https://rust-lang.github.io/rust-project-goals/2025h2/build-std.html

[rfcs#3873]: https://github.com/rust-lang/rfcs/pull/3873
[rfcs#3873-proposal]: https://github.com/davidtwco/rfcs/blob/build-std-part-one-context/text/3873-build-std-context.md#proposal
[rfcs#3873-background]: https://github.com/davidtwco/rfcs/blob/build-std-part-one-context/text/3873-build-std-context.md#background
[rfcs#3873-history]: https://github.com/davidtwco/rfcs/blob/build-std-part-one-context/text/3873-build-std-context.md#history
[rfcs#3873-motivation]: https://github.com/davidtwco/rfcs/blob/build-std-part-one-context/text/3873-build-std-context.md#history

[rfcs#3875]: https://github.com/rust-lang/rfcs/pull/3875
[rfcs#3875-proposal]: https://github.com/davidtwco/rfcs/blob/build-std-part-three-explicit-dependencies/text/3875-build-std-explicit-dependencies.md#proposal
[rfcs#3875-rationale-and-alternatives]: https://github.com/davidtwco/rfcs/blob/build-std-part-three-explicit-dependencies/text/3875-build-std-explicit-dependencies.md#rationale-and-alternatives
[rfcs#3875-unresolved-questions]: https://github.com/davidtwco/rfcs/blob/build-std-part-three-explicit-dependencies/text/3875-build-std-explicit-dependencies.md#unresolved-questions
[rfcs#3875-future-possibilities]: https://github.com/davidtwco/rfcs/blob/build-std-part-three-explicit-dependencies/text/3875-build-std-explicit-dependencies.md#future-possibilities

## Motivation
[motivation]: #motivation

This RFC builds on a large collection of prior art collated in the
[`build-std-context`][rfcs#3873-proposal] RFC, and is aimed at supporting the
following [motivations][rfcs#3873-motivation] it identifies:

- Building the standard library without relying on unstable escape hatches
- Building standard library crates that are not shipped for a target
- Using the standard library with tier three targets

While the enabling and disabling of some standard library features is mentioned
in this RFC (when required to support existing stable features of Cargo), the
enabling and disabling of arbitrary standard library features is handled by
[RFC #3875][rfcs#3875-features].

[rfcs#3875-features]: https://github.com/davidtwco/rfcs/blob/build-std-part-three-explicit-dependencies/text/3875-build-std-explicit-dependencies.md#features

## Proposal
[proposal]: #proposal

This proposal section is quite broad, and a
[summary of changes][summary-of-changes] is available for a very brief list of
proposed changes.

Cargo configuration will contain a new key `build-std` under the `[build]`
section ([?][rationale-build-std-in-config]), permitting one of two values -
"never" ([?][rationale-build-std-never]) or "always", defaulting to "never":

```toml
[build]
build-std = "never" # or `always`
```

`build-std` can also be specified in the `[target.<triple>]` and
`[target.<cfg>]` sections ([?][rationale-build-std-target-section]):

```toml
[target.aarch64-unknown-illumos]
build-std = "never" # or `always`
```

The `build-std` configuration locations have the following precedence
([?][rationale-build-std-precedence]):

1. `[target.<triple>]`
2. `[target.<cfg>]`
3. `[build]`

As the Cargo configuration is local to the current user (typically in
`.config/cargo.toml` in the project root and/or Cargo home directory), the value
of `build-std` is not influenced by the dependencies of the current crate.

When `build-std` is set to "always", then the standard library will be
unconditionally recompiled ([?][rationale-unconditional]) in the release profile
defined in its workspace as part of every clean build
([?][rationale-release-profile]). This is primarily useful for users of tier
three targets. As with other dependencies, the standard library's build will
respect the `RUSTFLAGS` environment variable.

> [!NOTE]
>
> Configuration of the pre-built standard library is split across bootstrap and
> the Cargo packages for the standard library. As much of this configuration as
> possible should be moved to the Cargo profile for these packages so that the
> artifacts produced by build-std match the pre-built standard library as much
> as is feasible.

`build-std` is a short-hand for an object which sets the `when` key:

```toml
[build]
build-std = { when = "always" }
```

Alongside `build-std`, a `build-std.crates` key will be introduced
([?][rationale-build-std-crate]), which can be used to specify which crates from
the standard library should be built. Only "core", "alloc" and "std" are valid
values for `build-std.crates`.

```toml
[build]
build-std = { when = "always", crates = "std" }
```

A value of "std" means that every crate in the graph has a direct dependency on
`std`, `alloc` and `core`. Similarly, "alloc" means `alloc` and `core`, and
"core" means just `core`.

If `std` is to be built and Cargo is building a test or benchmark using the
default test harness then Cargo will also build the `test` crate.

If [*Standard library dependencies*][rfcs#3875] are implemented then `builtin`
dependencies will be used if `build-std.crates` is not explicitly set.
Otherwise, `build-std.crates` will default to the crate intended to be supported
by the target (see later
[*Standard library crate stability*][standard-library-crate-stability] section).

> [!NOTE]
>
> Inspired by the concept of [opaque dependencies][Opaque dependencies], the
> standard library is resolved differently to other dependencies:
>
> - The lockfile included in the standard library source will be used when
>   resolving the standard library's dependencies ([?][rationale-lockfile]).
>
> - The dependencies of the standard library crates are entirely opaque to the
>   user. Different semver-compatible versions of these dependencies can
>   exist in the user's resolve. The user cannot control compilation any of
>   the dependencies of the `core`, `alloc` or `std` standard library crates
>   individually (via profile overrides, for example).
>
> - The release profile defined by the standard library will be used.
>
>     - This profile will be updated to match the current compilation options
>       used by the pre-built standard library as much as possible (e.g. using
>       `-Cembed-bitcode=yes` to support LTO).
>
> - Standard library crates and their dependencies from `build-std.crates`
>   cannot be patched/replaced by the user in the Cargo manifest or config
>   (e.g. using source replacement, `[replace]` or `[patch]`)
>
> - Lints in standard library crates will be built using `--cap-lints allow`
>   matching other upstream dependencies.
>
> Cargo will resolves the dependencies of opaque dependencies, such as the
> standard library, separately in their own workspaces. The root of such a
> resolve will be the crates specified in `build-std.crates` or, if
> [*Standard library dependencies*][rfcs#3875] is implemented, the unified set of
> packages that any crate in the dependency has a direct dependency on. A
> dependency on the relevant roots are added to all crates in the main resolve.
>
> Regardless of which standard library crates are being built, Cargo will build
> the `sysroot` crate of the standard library workspace. `alloc` and `std` will
> be optional dependencies of the `sysroot` crate which will be enabled when the
> user has requested them. Panic runtimes are dependencies of `std` and will be
> enabled depending on the features that Cargo passes to `std` (see
> [*Panic strategies*][panic-strategies]).
>
> rustc loads panic runtimes in a different way to most dependencies, and
> without looking in the sysroot they will fail to load correctly unless passed
> in with `--extern`. rustc will need to be patched to be able to load panic
> runtimes from `-L dependency=` paths in line with other transitive
> dependencies.
>
> The standard library will always be a non-incremental build
> ([?][rationale-incremental]), Cargo's dep-info fingerprint tracking will not
> track the standard library crate sources, Cargo's `.d` dep-info file will not
> include standard library crate sources, and only a `rlib` produced (no
> `dylib`) ([?][rationale-no-dylib]). It will be built in the Cargo `target`
> directory of the crate or workspace like any other dependency.
>
> Standard library crates are provided to the compiler using the `--extern` flag
> with the `noprelude` modifier ([?][rationale-noprelude-with-extern]).

The host pre-built standard library will always be used for procedural macros
and build scripts ([?][rationale-host-deps-cross],
[?][rationale-host-deps-host]). Multi-target projects (resulting from the
`target` field in artifact dependencies or the use of `per-pkg-target` fields)
may result in the standard library being built multiple times - once for each
target in the project.

*See the following sections for rationale/alternatives:*

- [*Why put `build-std` in the Cargo config?*][rationale-build-std-in-config]
- [*Why accept `never` as a value for `build-std`?*][rationale-build-std-never]
- [*Why add `build-std` to the `[target.<triple>]` and `[target.<cfg>]` sections?*][rationale-build-std-target-section]
- [*Why does `[target]` take precedence over `[build]` for `build-std`?*][rationale-build-std-precedence]
- [*Why does "always" rebuild unconditionally?*][rationale-unconditional]
- [*Why does "always" rebuild in release profile?*][rationale-release-profile]
- [*Why add `build-std.crates`?*][rationale-build-std-crate]
- [*Why use the lockfile of the `rust-src` component?*][rationale-lockfile]
- [*Why not build the standard library in incremental?*][rationale-incremental]
- [*Why not produce a `dylib` for the standard library?*][rationale-no-dylib]
- [*Why use `noprelude` with `--extern`?*][rationale-noprelude-with-extern]
- [*Why use the pre-built standard library for procedural macros and build scripts in host mode?*][rationale-host-deps-host]
- [*Why use the pre-built standard library for procedural macros and build scripts in cross-compile mode?*][rationale-host-deps-cross]

*See the following sections for relevant unresolved questions:*

- [*What should the `build-std` configuration in `.cargo/config` be named?*][unresolved-config-name]
- [*What should the "always" and "never" values of `build-std` be named?*][unresolved-config-values]
- [*What should `build-std.crates` be named?*][unresolved-build-std-crate-name]
- [*Should the standard library inherit RUSTFLAGS?*][unresolved-inherit-rustflags]

*See the following sections for future possibilities:*

- [*Allow reusing sysroot artifacts if available*][future-reuse-sysroot]

[Opaque dependencies]: https://hackmd.io/@epage/ByGfPtRell

### Standard library crate stability
[standard-library-crate-stability]: #standard-library-crate-stability

An optional `standard_library_support` field
([?][rationale-why-standard-library-support]) is added to the target
specification ([?][rationale-target-spec-purpose]), replacing the existing
`metadata.std` field. `standard_library_support` has two fields:

- `supported`, which can be set to either "core", "core and alloc", or
  "core, alloc, and std"
- `default`, which can be set to either "core", "core and alloc", or
  "core, alloc, and std"
  - `default` cannot be set to a value which is "less than" that of `supported`
    (i.e. "core and alloc" when `supported` was only set to "core")

The `supported` field determines which standard library crates Cargo will permit
to be built for this target on a stable toolchain. On a nightly toolchain, Cargo
will build whichever standard library crates are requested by the user.

The `default` field determines which crate will be built by Cargo if
`build-std.when = "always"` and `build-std.crates` is not set. Users can specify
`build-std.crates` to build more crates than included in the `default`, as long
as those crates are included in `supported`.

The correct value for `standard_library_support` is independent of the tier of
the target and depends on the set of crates that are intended to work for a
given target, according to its maintainers.

If `standard_library_support` is unset for a target, then Cargo will not permit
any standard library crates to be built for the target on a stable toolchain. It
will be required to use a nightly toolchain to use build-std with that target.

Cargo's `build-std.crates` field will default to the value of the
`standard_library_support.default` field (`std` for "core, alloc, and std",
`alloc` for "core and alloc", and `core` for "core"). This does not prevent
users from building more crates than the default, it is only intended to be a
sensible default for the target that is probably what the user expects.

The `target-standard-library-support` option will be supported by rustc's
`--print` flag and will be used by Cargo to query this value for a given target:

```shell-session
$ rustc --print target-standard-library-support --target armv7a-none-eabi
default: core
supported: core, alloc
$ rustc --print target-standard-library-support --target aarch64-unknown-linux-gnu
default: std
supported: core, alloc, std
```

*See the following sections for rationale/alternatives:*

- [*Why introduce `standard_library_support`?*][rationale-why-standard-library-support]
- [*Should target specifications own knowledge of which standard library crates are supported?*][rationale-target-spec-purpose]

### Interactions with `#![no_std]`
[interactions-with-no_std]: #interactions-with-no_std

Behaviour of crates using `#![no_std]` will not change whether or not `std` is
rebuilt and passed via `--extern` to rustc, and `#![no_std]` will still be
required in order for `rustc` to not attempt to load `std` and add it to the
extern prelude. [*Standard library dependencies*][rfcs#3875] describes a future
possibility for how the `no_std` mechanism could be replaced.

*See the following sections for future possibilities:*

- [*Replace `#![no_std]` as the source-of-truth for whether a crate depends on `std`*][future-replace-no_std] (RFC 3875)

[future-replace-no_std]: https://github.com/davidtwco/rfcs/blob/build-std-part-three-explicit-dependencies/text/3875-build-std-explicit-dependencies.md#replace-no_std-as-the-source-of-truth-for-whether-a-crate-depends-on-std

### `restricted_std`
[restricted_std]: #restricted_std

The existing `restricted_std` mechanism will be removed from `std`'s
[`build.rs`][std-build.rs].

*See the following sections for rationale/alternatives:*

- [*Why remove `restricted_std`?*][rationale-remove-restricted-std]

[std-build.rs]: https://github.com/rust-lang/rust/blob/f315e6145802e091ff9fceab6db627a4b4ec2b86/library/std/build.rs#L17

### Custom targets
[custom-targets]: #custom-targets

Cargo will detect when the standard library is to be built for a custom target
and will emit an error ([?][rationale-disallow-custom-targets]).

> [!NOTE]
>
> Cargo could detect use of a custom target either by comparing it with the list
> of built-in targets that rustc reports knowing about (via `--print target-list`)
> or by checking if a file exists at the path matching the provided target name.
>
> This does not require any changes to rustc. If it is invoked to build the
> standard library then it will continue to do so, as is possible today, it is
> only the build-std functionality in Cargo that will not support custom targets
> initially.

Furthermore, custom targets will be destabilised in rustc (as in [rust#71009]).
This will not be a significant breaking change as custom targets cannot
effectively be used currently without nightly (needing build-std to have
`core`).

Custom targets can still be used with build-std on nightly toolchains provided
that `-Zunstable-options` is provided to Cargo.

*See the following sections for rationale/alternatives:*

- [*Why disallow custom targets?*][rationale-disallow-custom-targets]

*See the following sections for future possibilities:*

- [*Allow custom targets with build-std*][future-custom-targets]

### Preventing implicit sysroot dependencies
[preventing-implicit-sysroot-dependencies]: #preventing-implicit-sysroot-dependencies

Cargo will pass a new flag to rustc which will prevent rustc from loading
top-level dependencies from the sysroot ([?][rationale-root-sysroot-deps]).

> [!NOTE]
>
> rustc could add a `--no-implicit-sysroot-deps` flag with this behaviour. For
> example, writing `extern crate foo` in a crate will not load `foo.rlib` from
> the sysroot if it is present, but if an `--extern noprelude:bar.rlib` is
> provided which depends on a crate `foo`, rustc will look in
> `-L dependency=...` paths and the sysroot for it.

*See the following sections for rationale/alternatives:*

- [*Why prevent rustc from loading root dependencies from the sysroot?*][rationale-root-sysroot-deps]

### Vendored `rust-src`
[vendored-rust-src]: #vendored-rust-src

When it is necessary to build the standard library, Cargo will look for sources
in a fixed location in the sysroot ([?][rationale-custom-src-path]):
`lib/rustlib/src`. rustup's `rust-src` component downloads standard library
sources to this location and will be made a default component. If the sources
are not found, Cargo will emit an error and recommend the user download
`rust-src` if using rustup.

`rust-src` will contain the sources for the standard library crates as well as
its vendored dependencies ([?][rationale-vendoring]). As a consequence sources
of standard library dependencies will not need be fetched from crates.io.

> [!NOTE]
>
> Cargo will not perform any checks to ensure that the sources in `rust-src`
> have been modified ([?][rationale-src-modifications]). It will be documented
> that modifying these sources is not supported.

*See the following sections for rationale/alternatives:*

- [*Why not allow the source path for the standard library be customised?*][rationale-custom-src-path]
- [*Why vendor standard library dependencies?*][rationale-vendoring]
- [*Why not check if `rust-src` has been modified?*][rationale-src-modifications]

*See the following sections for relevant unresolved questions:*

- [*Should `rust-src` be a default component?*][unresolved-rust-src]

### Panic strategies
[panic-strategies]: #panic-strategies

Panic strategies are unlike other profile settings insofar as they influence
which crates are built and which flags are passed to the standard library build.
For example, if `panic = "unwind"` were set in the Cargo profile then the
`panic_unwind` feature would need to be provided to `std` and `-Cpanic=unwind`
passed to suggest that the compiler use that panic runtime.

If Cargo is not building `std`, then neither of the panic runtimes will be
built. In this circumstance rustc will continue to throw an error when a
unwinding panic strategy is chosen.

If Cargo would build `std` for a project then Cargo's behaviour depends on
whether or not `panic` is set in the profile:

- If `panic` is not set in the profile then unwinding may still be the default
  for the target and Cargo will need to enable the `panic_unwind` feature to the
  `sysroot` crate to build `panic_unwind` just in case it is used

- If `panic` is set to "unwind" then the `panic_unwind` feature of `sysroot`
  will be enabled and `-Cpanic=unwind` will be passed

- If `panic` is set to "abort" then `-Cpanic=abort` will be passed

  - `panic_abort` is a non-optional dependency of `std` so it will always be
    built

- If `panic` is set to "immediate-abort" then `-Cpanic=immediate-abort` will be
  passed

  - Neither `panic_abort` or `panic_unwind` need to be built, but as
    `panic_abort` is non-optional, it will be

  - `-Cpanic=immediate-abort` is unstable

Tests, benchmarks, build scripts and proc macros continue to ignore the "panic"
setting and `panic = "unwind"` is always used - which means the standard library
needs to be recompiled again if the user is using "abort". Once
`panic-abort-tests` is stabilised, the standard library can be built with the
profile's panic strategy even for tests and benchmarks.

In line with Cargo's stance on not parsing the `RUSTFLAGS` environment variable,
it will not be checked for compilation flags that would require additional
crates to be built for compilation to succeed.

> [!NOTE]
>
> The `unwind` crate will continue to link to the system's `libunwind` which
> will need to match the target modifiers used by the standard library to avoid
> incompatibilities. Likewise, if `llvm-libunwind`, `-Clink-self-contained=yes`
> or `-Ctarget-feature=+crt-static` are used and the distributed `libunwind` is
> used then it will also need to match the target modifiers of the standard
> library to avoid incompatibilities.

*See the following sections for future possibilities:*

- [*Avoid building `panic_unwind` unnecessarily*][future-panic_unwind]

### Building the standard library on a stable toolchain
[building-the-standard-library-on-a-stable-toolchain]: #building-the-standard-library-on-a-stable-toolchain

rustc will automatically assume `RUSTC_BOOTSTRAP` when the source path of the
crate being compiled is within the same sysroot as the rustc binary being
invoked ([?][rationale-implied-bootstrap]). Cargo will not need to use
`RUSTC_BOOTSTRAP` when compiling the standard library with a stable toolchain.
The standard library's dependencies will not be permitted to use build probes to
detect whether a nightly version is being used.

*See the following sections for rationale/alternatives:*

- [*Why allow building from the sysroot with implied `RUSTC_BOOTSTRAP`?*][rationale-implied-bootstrap]

### Self-contained objects
[self-contained-objects]: #self-contained-objects

A handful of targets require linking against special object files, such as
`windows-gnu`, `linux-musl` and `wasi` targets. For example, `linux-musl`
targets require `crt1.o`, `crti.o`, `crtn.o`, etc.

Since [rust#76158]/[compiler-team#343], the compiler has a stable
`-Clink-self-contained` flag which will look for special object files in
expected locations, typically populated by the `rust-std` components. Its
behaviour can be forced by `-Clink-self-contained=true`, but is force-enabled
for some targets and inferred for others.

Rust will ship `rust-self-contained` components for any targets which
need it. These components will contain the special object files normally
included in `rust-std`, and will be distributed for all tiers of targets. While
generally these objects are specific to the architecture and C runtime (CRT)
(and so `rust-self-contained-$arch-$crt` could be sufficient and result in fewer
overall components), it's technically possible that Rust could support two
targets with the same architecture and same CRT but different versions of the
CRT, so having target-specific components is most future-proof. These would
replace the `self-contained` directory in existing `rust-std` components.

Similarly, for any architectures which require it, LLVM's `libunwind` will be
built and shipped in the `rust-self-contained` component.

As long as these components have been downloaded, as well as any other support
components, such as `rust-mingw`, rustc's `-Clink-self-contained` will be able
to link against the object files and build-std should never fail on account of
missing special object files. rustc will attempt to detect when
`rust-self-contained` components are missing and provide helpful diagnostics in
this case.

`-Clink-self-contained` also controls whether rustc uses the linker shipped with
Rust. build-std's use of `-Clink-self-contained` will endeavour to ensure that
the whatever the default linker for the current target is (self-contained or
otherwise) will be used.

*See the following sections for future possibilities:*

- [*Enable local recompilation of special object files/sanitizer runtimes*][future-recompile-special]

[rust#76158]: https://github.com/rust-lang/rust/pull/76158
[compiler-team#343]: https://github.com/rust-lang/compiler-team/issues/343

### `compiler-builtins`
[compiler-builtins]: #compiler-builtins

`compiler-builtins` is always built with `-Ccodegen-units=10000` to force each
intrinsic into its own object file to avoid symbol clashes with libgcc. This is
currently enforced with a profile override in the standard library's workspace
and is unchanged.

See [*Allow local builds of `compiler-rt` intrinsics*][future-compiler-builtins-c]
for discussion of the `compiler-builtins-c` feature.

[future-compiler-builtins-c]: https://github.com/davidtwco/rfcs/blob/build-std-part-three-explicit-dependencies/text/3875-build-std-explicit-dependencies.md#allow-local-builds-of-compiler-rt-intrinsics

#### `compiler-builtins/mem`
[compiler-builtins-mem]: #compiler-builtinsmem

It is not possible to use weak linkage to make the symbols provided by
`compiler_builtins/mem` trivially overridable in every case
([?][rationale-no-weak-linkage]).

The `mem` feature of `compiler_builtins` will be inverted to a new feature named
`external-mem` ([?][rationale-no-mem]). This will not be a default feature, so
`compiler_builtins` will provide mem symbols unless the `external-mem` is
provided.

`std`, which provides memory symbols via `libc`, will depend on the
`external-mem` feature. Most `no_std` users will use the `compiler_builtins`
implementation of these symbols and will work by default when they do not depend
on `std`.

Those users providing their own mem symbols can override on weak linkage of the
`compiler_builtins` symbols, or use a nightly toolchain to enable the
`external-mem` feature of an explicit dependency on the standard library (per
[*Standard library dependencies*][rfcs#3875]).

*See the following sections for rationale/alternatives:*

- [*Why not rely on weak linkage for `compiler-builtins/mem` symbols?*][rationale-no-weak-linkage]
- [*Why invert the `mem` feature?*][rationale-no-mem]

### `profiler-builtins`
[profiler-builtins]: #profiler-builtins

`profiler-builtins` will not be built by build-std, thus preventing
profile-guided optimisation with a locally-built standard library.
`profiler-builtins` has native dependencies which may fail compilation of the
standard library if missing were `profiler-builtins` to be built by default as
part of the standard library build.

*See the following sections for future possibilities:*

- [*Allow building `profiler-builtins`*][future-profiler-builtins]

### Caching
[caching]: #caching

Standard library artifacts built by build-std will be reused equivalently to
today's crates/dependencies that are built within a shared target directory. By
default, this limits sharing to a single workspace ([?][rationale-caching]).

*See the following sections for rationale/alternatives:*

- [*Why not globally cache builds of the standard library?*][rationale-caching]

### Generated documentation
[generated-documentation]: #generated-documentation

When running `cargo doc` for a project to generate documentation and rebuilding
the standard library, the generated documentation for the user's crates will
link to the locally generated documentation for the `core`, `alloc` and `std`
crates, rather than the upstream hosted generation as is typical for non-locally
built standard libraries.

*See the following sections for rationale/alternatives:*

- [*Why not link to hosted standard library documentation in generated docs?*][rationale-generated-docs]

### Cargo subcommands
[cargo-subcommands]: #cargo-subcommands

Any Cargo command which accepts a package spec with `-p` will not recognise
`core`, `alloc`, `std` or none of their dependencies (unless
[*Standard library dependencies*][rfcs#3875] is implemented). Many of Cargo's
subcommands will need modification to support build-std:

[`cargo clean`][cargo-clean] will additionally delete any builds of the standard
library performed by build-std.

[`cargo fetch`][cargo-fetch] will not fetch the standard library dependencies as
they are already vendored in the `rust-src` component.

[`cargo miri`][cargo-miri] is not built into Cargo, it is shipped by miri, but
is mentioned in Cargo's documentation. `cargo miri` is unchanged by this RFC,
but build-std is one step towards `cargo miri` requiring less special support.

> [!NOTE]
>
> `cargo miri` could be re-implemented using build-std to enable a `miri`
> profile and always rebuild. The `miri` profile would be configured in the
> standard library's workspace, setting the flags/options necessary for `miri`.

[`cargo report`][cargo-report] will not include reports from the standard
library crates or their dependencies.

[`cargo update`][cargo-update] will not update the dependencies of `std`,
`alloc` and `core`, as these are vendored as part of the distribution of
`rust-src` and resolved separately from the user's dependencies. Neither will
`std`, `alloc` or `core` be updated, as these are unversioned and always match
the current toolchain version.

[`cargo vendor`][cargo-vendor] will not vendor the standard library crates or
their dependencies. These are pre-vendored as part of the `rust-src` component
([?][rationale-vendoring]).

The following commands will now build the standard library if required as part
of the compilation of the project, just like any other dependency:

- [`cargo bench`][cargo-bench]
- [`cargo build`][cargo-build]
- [`cargo check`][cargo-check]
- [`cargo clippy`][cargo-clippy]
- [`cargo doc`][cargo-doc]
- [`cargo fix`][cargo-fix]
- [`cargo run`][cargo-run]
- [`cargo rustc`][cargo-rustc]
- [`cargo rustdoc`][cargo-rustdoc]
- [`cargo test`][cargo-test]

This part of the RFC has no implications for the following Cargo subcommands:

- [`cargo add`][cargo-add]
- [`cargo remove`][cargo-remove]
- [`cargo fmt`][cargo-fmt]
- [`cargo generate-lockfile`][cargo-generate-lockfile]
- [`cargo help`][cargo-help]
- [`cargo info`][cargo-info]
- [`cargo init`][cargo-init]
- [`cargo install`][cargo-install]
- [`cargo locate-project`][cargo-locate-project]
- [`cargo login`][cargo-login]
- [`cargo logout`][cargo-logout]
- [`cargo metadata`][cargo-metadata]
- [`cargo new`][cargo-new]
- [`cargo owner`][cargo-owner]
- [`cargo package`][cargo-package]
- [`cargo pkgid`][cargo-pkgid]
- [`cargo publish`][cargo-publish]
- [`cargo search`][cargo-search]
- [`cargo tree`][cargo-tree]
- [`cargo uninstall`][cargo-uninstall]
- [`cargo version`][cargo-version]
- [`cargo yank`][cargo-yank]

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

### Stability guarantees
[stability-guarantees]: #stability-guarantees

build-std enables a much greater array of configurations of the standard library
to exist and be produced by stable toolchains than the single configuration that
is distributed today.

It is not feasible for the Rust project to test every combination of profile
configuration, Cargo feature, target and standard library crate. As such, the
stability of build-std as a mechanism must be separated from the stability
guarantees which apply to configurations of the standard library it enables.

For example, while a stable build-std mechanism may permit the standard library
to be built for a tier three target, the Rust project continues to make no
commitments or guarantees that the standard library for that target will
function correctly or build at all. Even on a tier one target, the Rust project
cannot test every possible variation of the standard library that build-std
enables.

The tier of a target no longer determines the possibility of using the standard
library, but rather the level of support provided for the standard library on
the target.

Cargo and Rust project documentation will clearly document the configurations
which are tested upstream and are guaranteed to work. Any other configurations
are supported on a strictly best-effort basis. The Rust project may later choose
to provide more guarantees for some well-tested configurations (e.g. enabling
sanitisers). This documentation need not go into detail about the exact
compilation flags used in a configuration - for example, "the release profile
with the address sanitizer is tested to work" would be sufficient.

There are also no guarantees about the exact configuration of the standard
library. Over time, the standard library built by build-std could be changed to
be closer to that of the pre-built standard library.

Additionally, there are no guarantees that the build environment required for
the standard library will not change over time (e.g. new minimum versions of
system packages or C toolchains, etc).

Building the standard library crates in the sysroot without requiring
`RUSTC_BOOTSTRAP` is intended for enabling the standard library to be built with
a stable toolchain and stable compiler flags, despite that the standard library
uses unstable features in its source code, not as a general mechanism for
bypassing Rust's stability mechanisms.

## Drawbacks
[drawbacks]: #drawbacks

There are some drawbacks to build-std:

- build-std overlaps with the initial designs and ideas for opaque dependencies
  in Cargo, thereby introducing a risk of constraining or conflicting with the
  eventual complete design for opaque dependencies

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This section aims to justify all of the decisions made in the proposed design
from [*Proposal*][proposal] and discuss why alternatives were not chosen.

### Why put `build-std` in the Cargo config?
[rationale-build-std-in-config]: #why-put-build-std-in-the-cargo-config

There are various alternatives to putting `build-std` in the Cargo configuration:

1. Cargo could continue to use an explicit command-line flag to enable
   build-std, such as the current `-Zbuild-std` (stabilised as `--build-std`).

   This approach is proven to work, as per the current unstable implementation,
   but has a poor user experience, requiring an extra argument to every
   invocation of Cargo with almost every subcommand of Cargo.

   However, this approach does not lend itself to use with other future and
   current Cargo features. Additional flags would be required to enable Cargo
   features (like today's `-Zbuild-std-features`) and would still necessarily be
   less fine-grained than being able to enable features on individual standard
   library crates. Similarly for public/private dependencies or customising the
   profile for the standard library crates.

2. build-std could be enabled or disabled in the `Cargo.toml`. However, under
   which conditions the standard library is rebuilt is better determined by the
   user of Cargo, rather than the package being built.

   A user may want to never rebuild the standard library so as to avoid
   invalidating the guarantees of their qualified toolchain, or may want to
   rebuild unconditionally to further optimise the standard library for their
   known deployment platform, or may only want to rebuild as necessary to ensure
   the build will succeed. All of these rationale can apply to the same crate in
   different circumstances, so it doesn't make sense for a crate to decide this
   once in its `Cargo.toml`.

   It would be a waste of resources if a dependency declared that it must always
   rebuild the standard library when the pre-built crate would be sufficient and
   this could not be overridden. It is also unclear how to aggregate different
   configurations of the `build-std` key from different crates in the dependency
   graph into a single value.

While using `build-std` key in the Cargo configuration shares some of the
downsides of using an explicit flag - not having a natural extension point for
other Cargo options exposed to dependencies -
[*Standard library dependencies*][rfcs#3875] addresses these concerns.

↩ [*Proposal*][proposal]

### Why accept `never` as a value for `build-std`?
[rationale-build-std-never]: #why-accept-never-as-a-value-for-build-std

The user can specify `never` (the default value) if they prefer which will never
rebuild the standard library. rustc will still return an error when the user's
target-modifiers do not match the pre-built standard library.

The `never` value is useful particularly for qualified toolchains where
rebuilding the standard library may invalidate the testing that the qualified
toolchain has undergone.

↩ [*Proposal*][proposal]

### Why add `build-std` to the `[target.<triple>]` and `[target.<cfg>]` sections?
[rationale-build-std-target-section]: #why-add-build-std-to-the-targettriple-and-targetcfg-sections

Supporting `build-std` as a key of both `[build]` and `[target]` sections allows
the greatest flexibility for the user. The overhead of rebuilding the standard
library may not be desirable in general but would be required when building on
targets which do not ship a pre-built standard library.

↩ [*Proposal*][proposal]

### Why does `[target]` take precedence over `[build]` for `build-std`?
[rationale-build-std-precedence]: #why-does-target-take-precedence-over-build-for-build-std

`[target]` configuration is necessarily more narrowly scoped so it makes sense
for it to override a global default in `[build]`.

↩ [*Proposal*][proposal]

### Why have a manual "always" option instead of a "when-needed" mode?
[rationale-unconditional]: #why-have-a-manual-always-option-instead-of-a-when-needed-mode

Always using a locally-built standard library avoids the complexity associated
with an automatic build-std mechanism while still being useful for users of tier
three targets. By leaving an automatic mechanism for a later RFC, fewer of the
technical challenges of build-std need to be addressed all at once.

Having an opt-in mechanism initially, such as `build-std = "always"`, allows for
early issues with build-std to be ironed out without potentially affecting more
users like an automatic mechanism. Later proposals will extend the `build-std`
option with an automatic mechanism.

↩ [*Proposal*][proposal]

### Why does "always" rebuild in release profile?
[rationale-release-profile]: #why-does-always-rebuild-in-release-profile

The release profile most closely matches the existing pre-built standard
library, which has proven itself suitable for a majority of use cases.

By minimising the differences between a newly-built std and a pre-built std,
there is less chance of the user experiencing bugs or unexpected behaviour from
the well-tested and supported pre-built std. Later proposals will extend the
`build-std` option with customised standard library builds that use the user's
profile.

↩ [*Proposal*][proposal]

### Why add `build-std.crates`?
[rationale-build-std-crate]: #why-add-build-stdcrates

Not all standard library crates will build on all targets. In a `no_std` project
for a tier three target, `build-std.crates` gives the user the ability to limit
which crates are built to those they know they need and will build successfully.

*See [Standard library dependencies*][rfcs#3875] for an alternative to
`build-std.crates`.*

↩ [*Proposal*][proposal]

### Why use the lockfile of the `rust-src` component?
[rationale-lockfile]: #why-use-the-lockfile-of-the-rust-src-component

Using different dependency versions for the standard library would invalidate
the upstream testing of the standard library. In particular, some crates use
unstable APIs when included as a dependency of the standard library meaning that
there is a high risk of build breakage if any package version is changed.

Using the lockfile included in the `rust-src` component guarantees that the same
dependency versions are used as in the pre-built standard library. As the
standard library does not re-export types from its dependencies, this will not
affect interoperability with the same dependencies of different versions used by
the user's crate.

Using the lockfile does prevent Cargo from resolving the standard library
dependencies to newer patch versions that may contain security fixes. However,
this is already impossible with the pre-built standard library.

See
[*Why vendor the standard library's dependencies?*][rationale-vendoring]

↩ [*Proposal*][proposal]

### Why not build the standard library in incremental?
[rationale-incremental]: #why-not-build-the-standard-library-in-incremental

The standard library sources are not intended to be modified locally, similarly
to those Cargo fetches from `registry` or `git` sources. Incremental compilation
would only add a compilation time overhead for any package sources which do not
change.

↩ [*Proposal*][proposal]

### Why not produce a `dylib` for the standard library?
[rationale-no-dylib]: #why-not-produce-a-dylib-for-the-standard-library

The standard library supports being built as both a `rlib` and a `dylib` and
both are shipped as part of the `rust-std` component. As the `dylib` does not
contain a metadata hash, it can be rebuilt unnecessarily when toolchain versions
change (e.g. switching between stable and nightly and back). The `dylib` is only
linked against when `-Cprefer-dynamic` is used. build-std will initially be
conservative and not include the `dylib` and `-Cprefer-dynamic` would fail
compilation.

*See the following sections for future possibilities:*

- [*Build both `dylib` and `rlib` variants of the standard library*][future-crate-type]

↩ [*Proposal*][proposal]

### Why use the pre-built standard library for procedural macros and build scripts in cross-compile mode?
[rationale-host-deps-cross]: #why-use-the-pre-built-standard-library-for-procedural-macros-and-build-scripts-in-cross-compile-mode

Procedural macros always run on the host and need to be built with a
configuration that are compatible with the host toolchain's Cargo and rustc,
limiting the potential customisations of the standard library that would be
valid. There is little advantage to using a custom standard library with
procedural macros, as they are not part of the final output artifact and
anywhere they can run already have a toolchain with host tools and a pre-built
standard library.

Build scripts similarly always run on the host and thus would require building
the standard library again for the host. There is little advantage to doing this
as build scripts are not part of the final output artifact. Build scripts do not
respect `RUSTFLAGS` which could result in target modifier mismatches if
rebuilding the standard library does respect `RUSTFLAGS`.

↩ [*Proposal*][proposal]

### Why use the pre-built standard library for procedural macros and build scripts in host mode?
[rationale-host-deps-host]: #why-use-the-pre-built-standard-library-for-procedural-macros-and-build-scripts-in-host-mode

Unlike when in cross-compile mode, if Cargo is in host mode (i.e. `--target` is
not provided), the standard library built by build-std could hypothetically be
used for procedural macros and build scripts without additional recompilations
of the standard library.

However, as with [cross-compile mode][rationale-host-deps-cross], there is
little advantage to using a customised standard library for procedural macros or
build scripts, and both would require limitations on the customisations possible
with build-std in order to guarantee compatibility with the compiler or build
script, respectively.

↩ [*Proposal*][proposal]

### Should target specifications own knowledge of which standard library crates are supported?
[rationale-target-spec-purpose]: #should-target-specifications-own-knowledge-of-which-standard-library-crates-are-supported

It is much simpler to record this information in a target's specification than
build this information into Cargo or to try and match on the target's cfg values
in the standard library's `build.rs` and set a cfg that Cargo could read.

Target specifications have typically been considered part of the compiler and
there has been hesitation to have target specs be the source of truth for
information like standard library support, as this is the domain of the library
team and ought to be owned by the standard library (such as in the standard
library's `build.rs`). However, with appropriate processes and sync points,
there is no reason why the target specification could not be primarily
maintained by the compiler team but in close coordination with library and other
relevant teams.

↩ [*Standard library crate stability*][standard-library-crate-stability]

### Why introduce `standard_library_support`?
[rationale-why-standard-library-support]: #why-introduce-standard_library_support

Attempting to compile the standard library crates may fail for some targets
depending on which standard library crates that target intends to support. When
enabled, build-std should default to only building those crates that are
expected to succeed, and should prevent the user from attempting to build those
crates that are expected to fail. This will provide a much improved user
experience than attempting to build standard library crates and encountering
complex and unexpected compilation failures.

For example, `no_std` targets often do not support `std` and so should inform
the error with a helpful error message that `std` cannot be built for the target
rather than attempt to build it and fail with confusing and unexpected errors.
Similarly, many `no_std` targets do support `alloc` if a global allocator is
provided, but if build-std built `alloc` by default for these targets then it
would often be unnecessary and could often fail.

It is not sufficient to determine which crates should be supported for a target
based on its the tier. For example, targets like `aarch64-apple-tvos` are tier
three while intending to fully support the standard library. It would be
needlessly limiting to prevent build-std from building `std` for this target.
However, build-std does provide a stable mechanism to build `std` for this
target that did not previously exist, so there must be clarity about what
guarantees and level of support is provided by the Rust project:

1. Whether a standard library crate is part of the stable interface of
   the standard library as a whole is determined by the library team and the set
   of crates that comprise this interface is the same for all targets

2. Whether any given standard library crate can be built with build-std is
   determined on a per-target basis depending on whether it is intended that the
   target be able to support that crate

3. Whether the Rust project provide guarantees or support for the standard
   library on a target is determined by the tier of the target

4. Whether the pre-built standard library is distributed for a target is
   determined by the tier of the target and which crates it intends to support

5. Which crate is built by default by build-std is determined on a per-target
   basis

For example, consider the following targets:

- `armv7a-none-eabihf`

  1. As with any other target, the `std`, `alloc` and `core` crates are stable
     interfaces to the standard library

  2. It intends to support the `core` and `alloc` crates, which build-std will
     permit to be built. `std` cannot be built by build-std for this target (on
     stable)

  3. It is a tier three target, so no support or guarantees are provided for the
     standard library crates

  4. It is a tier three target, so no standard library crates are distributed

  5. `alloc` would not build without a global allocator crate being provided by
     the user and may not be required by all users, so only `core` will be built
     by default

- `aarch64-apple-tvos`

  1. As with any other target, the `std`, `alloc` and `core` crates are stable
     interfaces to the standard library

  2. It intends to support `core`, `alloc` and `std` crates, which build-std
     will permit to be built

  3. It is a tier three target, so no support or guarantees are provided for the
     standard library crates

  4. It is a tier three target, so no standard library crates are distributed

  5. All of `core`, `alloc` and `std` will be built by default

- `armv7a-none-eabi`

  1. As with any other target, the `std`, `alloc` and `core` crates are stable
     interfaces to the standard library

  2. It intends to support the `core` and `alloc` crates, which build-std will
     permit to be built. `std` cannot be built by build-std for this target (on
     stable)

  3. It is a tier two target, so the project guarantees that the `core` and
     `alloc` crates will build

  4. It is a tier two target, so there are distributed artifacts for the `core`
     and `alloc` crates

  5. `alloc` would not build without a global allocator crate being provided by
     the user and may not be required by all users, so only `core` will be built
     by default

- `aarch64-unknown-linux-gnu`

  1. As with any other target, the `std`, `alloc` and `core` crates are stable
     interfaces to the standard library

  2. It intends to support the `core`, `alloc` and `std` crates, which build-std
     will permit to be built

  3. It is a tier one target, so the project guarantees that the `core`, `alloc`
     and `std` will build and that they have been tested

  4. It is a tier one target, so there are distributed artifacts for the `core`,
     `alloc` and `std` crates

  5. All of `core`, `alloc` and `std` will be built by default

↩ [*Standard library crate stability*][standard-library-crate-stability]

### Why remove `restricted_std`?
[rationale-remove-restricted-std]: #why-remove-restricted_std

`restricted_std` was originally added as part of a mechanism to enable the
standard library to build on all targets (just with stubbed out functionality),
however stability is not an ideal match for this use case. rustc will still try
to compile unstable code, so this doesn't help ensure the standard library builds
on all targets.

Furthermore, when `restricted_std` applies, users must add
`#![feature(restricted_std)]` to opt-in to using the standard library anyway
(conditionally, only for affected targets), and have no mechanism for opting-in
on behalf of their dependencies (including first-party crates like `libtest`).

It is still valuable for the standard library to be able to compile on as many
targets as possible using the `unsupported` module in its platform abstraction
layer, but this mechanism does not use `restricted_std`.

↩ [*`restricted_std`*][restricted_std]

### Why disallow custom targets?
[rationale-disallow-custom-targets]: #why-disallow-custom-targets

While custom targets can be used on stable today, in practice, they are only
used on nightly as `-Zbuild-std` would need to be used to build at least `core`.
As such, if build-std were to be stabilised, custom targets would become much
more usable on stable toolchains. This is undesirable as there are many open
questions surrounding the [unstable target-spec-json][rust#71009] for custom
targets and how they ought to be supported.

In order to avoid users relying on the unstable format with a stable toolchain,
using custom targets with build-std on a stable toolchain is disallowed by Cargo
until another RFC can consider all the implications of this thoroughly.

Similarly, custom targets are destabilised in rustc, as the changes in
[*Building the standard library on a stable toolchain*][building-the-standard-library-on-a-stable-toolchain]
could allow the unstable format to be relied upon even with Cargo's prohibition
of custom targets.

↩ [*Custom targets*][custom-targets]

[rust#71009]: https://github.com/rust-lang/rust/pull/71009

### Why prevent rustc from loading root dependencies from the sysroot?
[rationale-root-sysroot-deps]: #why-prevent-rustc-from-loading-root-dependencies-from-the-sysroot

Loading root dependencies from the sysroot could be a source of bugs.

For example, if a crate has an explicit dependency on `core` which is newly
built, then there will be no `alloc` or `std` builds present. A user could still
write `extern crate alloc` and accidentally load `alloc` from the sysroot
(compiled with the default profile settings) and consequently `core` from the
sysroot, conflicting with the newly build `core`. `extern crate alloc` should
only be able to load the `alloc` crate if the crate depends on it in its
`Cargo.toml`. A similar circumstance can occur with dependencies like
`panic_unwind` that the compiler tries to load itself.

Dependencies of packages can still be loaded from the sysroot, even with
`--no-implicit-sysroot-deps`, to support the circumstance where Cargo uses a
pre-built standard library crate (e.g.
`$sysroot/lib/rustlib/$target/lib/std.rlib`) and needs to load the dependencies
of that crate which are also in the sysroot.

`--no-implicit-sysroot-deps` is a flag rather than default behaviour to preserve
rustc's usability when invoked outside of Cargo. For example, by compiler
developers when working on rustc.

`--sysroot=''` is an existing mechanism for disabling the sysroot - this is not
used as it remains desirable to load dependencies from the sysroot as a
fallback. In addition, rustc uses the sysroot path to find `rust-lld` and
similar tools and would not be able to do so if the sysroot were disabled by
providing an empty path.

↩ [*Preventing implicit sysroot dependencies*][preventing-implicit-sysroot-dependencies]

### Why use `noprelude` with `--extern`?
[rationale-noprelude-with-extern]: #why-use-noprelude-with---extern

Using `noprelude` allows `build-std` to closer match rustc's behaviour when it
loads crates from the sysroot. Without `noprelude`, rustc adds crates provided
with `--extern` flags to the extern prelude. As a consequence, if a newly-built
`alloc` were passed using `--extern alloc=alloc.rlib` then `extern crate alloc`
would not be required to use the locally-built `alloc`, but it would be to use
the pre-built `alloc` when `--extern alloc=alloc.rlib` is not provided. This
difference in how a crate is made available to rustc should not be observable to
the user as they have not opted into the migration.

Passing crates without `noprelude` with the existing prelude behaviour has also
been a source of [bugs][wg-cargo-std-aware#40] in previous `-Zbuild-std`
implementations.

↩ [*Preventing implicit sysroot dependencies*][preventing-implicit-sysroot-dependencies]

[wg-cargo-std-aware#40]: https://github.com/rust-lang/wg-cargo-std-aware/issues/40

### Why not allow the source path for the standard library be customised?
[rationale-custom-src-path]: #why-not-allow-the-source-path-for-the-standard-library-be-customised

It is not a goal of this proposal to enable or improve the usability of custom
or modified standard libraries.

↩ [*Vendored `rust-src`*][vendored-rust-src]

### Why vendor the standard library's dependencies?
[rationale-vendoring]: #why-vendor-the-standard-librarys-dependencies

Vendoring the standard library is possible since it currently has its own
workspace, allowing the dependencies of just the standard library crates (and
not the compiler or associated tools in `rust-lang/rust`) to be easily packaged.
Doing so has multiple advantages..

- Avoid needing to support standard library dependencies in `cargo vendor`
- Avoid needing to support standard library dependencies in `cargo fetch`
- Re-building the standard library does not require an internet connection
- Standard library dependency versions are fixed to those in the `Cargo.lock`
  anyway, so initial builds with `build-std` start quicker with these
  dependencies already available
- Allow build-std to continue functioning if a `crates.io` dependency is
  "yanked"
  - This leaves the consequences of a toolchain version using yanked
    dependencies the same as without this RFC

..and few disadvantages:

- A larger `rust-src` component takes up more disk space and takes longer to
  download
  - If using build-std, these dependencies would have to be downloaded at build
    time, so this is only an issue if build-std is not used and `rust-src` is
    downloaded.
  - `rustc-src` is currently 3.5 MiB archived and 44 MiB extracted, and if
    dependencies of the standard library were vendored, then it would be 9.1 MiB
    archived and 131 MiB extracted.
- Vendored dependencies can't be updated with the latest security fixes
  - This is no different than the pre-built standard library

How this affects `crates.io`/`rustup` bandwidth usage or user time spent
downloading these crates is unclear and depends on user patterns. If not
vendored, Cargo will "lazily" download them the first time `build-std` is used
but this may happen multiple times if they are cleaned from its cache without
upgrading the toolchain version.

See
[*Why use the lockfile of the `rust-src` component?*][rationale-lockfile]

↩ [*Vendored `rust-src`*][vendored-rust-src]

### Why not check if `rust-src` has been modified?
[rationale-src-modifications]: #why-not-check-if-rust-src-has-been-modified

This is in line with other immutable dependency sources (like registry or git).
It is also likely that any protections implemented to check that the sources in
`rust-src` have not been modified could be trivially bypassed.

Any crate that depends on `rust-src` having been modified would not be usable
when published to crates.io as the required modifications will obviously not be
included.

↩ [*Vendored `rust-src`*][vendored-rust-src]

### Why allow building from the sysroot with implied `RUSTC_BOOTSTRAP`?
[rationale-implied-bootstrap]: #why-allow-building-from-the-sysroot-with-implied-rustc_bootstrap

Cargo needs to be able to build the standard library crates, which inherently
require unstable features. It could set `RUSTC_BOOTSTRAP` internally to do this
with a stable toolchain, but this is a bypass mechanism that the project do not
want to encourage use of, and as this is a shared requirement with other build
systems that wish to build an unmodified standard library and want to work on
stable toolchains, it is worth establishing a narrow general mechanism.

For example, Rust's project goal to enable Rust for Linux to build using only a
stable toolchain would require that it be possible to build `core` without
nightly.

It is not sufficient for rustc to special-case the `core`, `alloc` and `std`
crate names as, when being built as part of the standard library, dependencies
of the standard library also use unstable features and it is not practical to
special-case all of these crates.

↩ [*Building the standard library on a stable toolchain*][building-the-standard-library-on-a-stable-toolchain]

### Why invert the `mem` feature?
[rationale-no-mem]: #why-invert-the-mem-feature

While "negative" features are typically discouraged due to how features unify
(e.g. `std` features are preferred to `no_std`): the `mem` feature's current
behaviour is the opposite of what is optimal.

Ideally, a crate should be able to provide alternate memory symbols and disable
`compiler_builtins`' symbols for the entire crate graph by enabling a feature
(e.g. `std`/`libc` could do this) - this is what an `external-mem` feature
enables.

↩ [*`compiler-builtins-mem`*][compiler-builtins-mem]

### Why not use weak linkage for `compiler-builtins/mem` symbols?
[rationale-no-weak-linkage]: #why-not-use-weak-linkage-for-compiler-builtinsmem-symbols

Since [compiler-builtins#411], the relevant symbols in `compiler_builtins`
already have weak linkage. However, it is nevertheless not possible to simply
remove the `mem` feature and have the symbols always be present:

- Some targets, such as those based on MinGW, do not have sufficient support for
  weak definitions (at least with the default linker).
- Weak linkage has precedence over shared libraries and the symbols of a
  dynamically-linked `libc` should be preferred over `compiler_builtins`'s
  symbols.

↩ [*`compiler-builtins-mem`*][compiler-builtins-mem]

[compiler-builtins#411]: https://github.com/rust-lang/compiler-builtins/pull/411

### Why not globally cache builds of the standard library?
[rationale-caching]: #why-not-globally-cache-builds-of-the-standard-library

The standard library is no different than regular dependencies in being able to
benefit from global caching of dependency builds. It is out-of-scope of this
proposal to propose a special-cased mechanism for this that applies only to the
standard library. [cargo#5931] tracks the feature request of intermediate
artifact caching in Cargo.

↩ [*Caching*][caching]

[cargo#5931]: https://github.com/rust-lang/cargo/issues/5931

### Why not link to hosted standard library documentation in generated docs?
[rationale-generated-docs]: #why-not-link-to-hosted-standard-library-documentation-in-generated-docs

Cargo would need to pass `-Zcrate-attr="doc(html_root_url=..)"` to the standard
library crates when building them but doesn't have the required information to
know what url to provide. Cargo would require knowledge of the current toolchain
channel to build the correct url and doesn't know this.

↩ [*Generated documentation*][generated-documentation]

## Unresolved questions
[unresolved-questions]: #unresolved-questions

The following are aspects of the proposal which warrant further discussion or
small details are likely to be bikeshed prior to this part of the RFC's
acceptance or stabilisation and aren't pertinent to the overall design:

### What should the `build-std.when` configuration in `.cargo/config` be named?
[unresolved-config-name]: #what-should-the-build-stdwhen-configuration-in-cargoconfig-be-named

What should this configuration option be named? `build-std`?
`rebuild-standard-library`?

↩ [*Proposal*][proposal]

### What should the "always" and "never" values of `build-std` be named?
[unresolved-config-values]: #what-should-the-always-and-never-values-of-build-std-be-named

What is the most intuitive name for the values of the `build-std` setting?
`always`? `manual`? `unconditional`?

`always` combined with the configuration option being named `build-std` -
`build-std = "always"` - is imperfect as it reads as if the standard library
will be re-built every time, when it actually just avoids use of the pre-built
standard library and caches the newly-built standard library.

↩ [*Proposal*][proposal]

### What should `build-std.crates` be named?
[unresolved-build-std-crate-name]: #what-should-build-stdcrates-be-named

What should this configuration option be named?

↩ [*Proposal*][proposal]

### Should the standard library inherit RUSTFLAGS?
[unresolved-inherit-rustflags]: #should-the-standard-library-inherit-rustflags

Existing designs for *[Opaque dependencies]* intended that `RUSTFLAGS` would not
apply to the opaque dependency. However, if a target modifier were set using
`RUSTFLAGS` and build-std ignored the variable, then rustc would fail to build
the user's project due to incompatible target modifiers. This would necessitate
that every stable target modifier be exposed via Cargo to be usable in practice.

↩ [*Proposal*][proposal]

### Should `rust-src` be a default component?
[unresolved-rust-src]: #should-rust-src-be-a-default-component

Ensuring `rust-src` is a default component reduces friction for users, and CI,
who have to otherwise need to install the component manually the first time they
use `build-std`.

On the other hand this increases their storage and bandwidth costs, plus
bandwidth costs for the project. The impact on usability is limited for the user
to once per toolchain as the component persists through updates.

↩ [*Vendored rust-src*][vendored-rust-src]

## Prior art
[prior-art]: #prior-art

See the [*Background*][rfcs#3873-background] and [*History*][rfcs#3873-history]
of the build-std context RFC.

## Future possibilities
[future-possibilities]: #future-possibilities

There are many possible follow-ups to this part of the RFC:

### Allow reusing sysroot artifacts if available
[future-reuse-sysroot]: #allow-reusing-sysroot-artifacts-if-available

This part of the RFC proposes rebuilding all required crates unconditionally as
this fits Cargo's existing compilation model better. However, just building a
crate equivalent to one already in the sysroot is inefficient. Cargo could learn
when to reuse artifacts in the sysroot when equivalent to ones it intends to
build, but this is complex enough to warrant its own proposal if desired.

↩ [*Proposal*][proposal]

### Allow custom targets with build-std
[future-custom-targets]: #allow-custom-targets-with-build-std

This would require a decision from the relevant teams on the exact stability
guarantees of the target-spec-json format and whether any large changes to
the format are desirable prior to broader use.

↩ [*Custom targets*][custom-targets]

### Avoid building `panic_unwind` unnecessarily
[future-panic_unwind]: #avoid-building-panic_unwind-unnecessarily

This would require adding a `--print default-unwind-strategy` flag to rustc and
using that to avoid building `panic_unwind` if the default is abort for any
given target and `panic` is not set in the profile.

↩ [*Panic strategies*][panic-strategies]

### Enable local recompilation of special object files/sanitizer runtimes
[future-recompile-special]: #enable-local-recompilation-of-special-object-filessanitizer-runtimes

These files are shipped pre-compiled for relevant targets and are not compiled
locally. If a user wishes to customise the compilation of these files like the
standard library, then there is no mechanism to do so.

↩ [*Self-contained objects*][self-contained-objects]

### Allow building `profiler-builtins`
[future-profiler-builtins]: #allow-building-profiler-builtins

It may be possible to ship a rustup component with pre-compiled native
dependencies of `profiler-builtins` so that build-std can reliably compile the
`profiler-builtins` crate regardless of the environment. Alternatively,
stability guarantees could be adjusted to set expectations that some parts of
the standard library may not build without external system dependencies.

If `profiler-builtins` can be reliably built, then it should be unconditionally
included in part of the standard library build.

↩ [*profiler-builtins*][profiler-builtins]

### Build both `dylib` and `rlib` variants of the standard library
[future-crate-type]: #build-both-dylib-and-rlib-variants-of-the-standard-library

build-std could build both the `dylib` and `rlib` of the standard library.

↩ [*Why not produce a `dylib` for the standard library?*][rationale-no-dylib]

## Summary of proposed changes
[summary-of-changes]: #summary-of-proposed-changes

These are each of the changes which would need to be implemented in the Rust
toolchain grouped by the project team whose purview the change would fall under:

- Bootstrap/infra/release
  - [Vendoring standard library sources into `rust-src`][vendored-rust-src]
  - [`rust-src` is a default component][vendored-rust-src]
  - [`rust-self-contained` components][self-contained-objects]
  - [Testing build-std in rust-lang/rust CI][summary-constraints]
- Cargo
  - [`build-std = "always"`][proposal]
    - [Extending Cargo subcommmands][cargo-subcommands]
  - [Prohibiting custom targets][custom-targets]
- Compiler
  - [Loading `panic_unwind` from `-L dependency=`][proposal]
  - [`--no-implicit-sysroot-deps`][preventing-implicit-sysroot-dependencies]
  - [Destabilise custom targets][custom-targets]
  - [Assuming `RUSTC_BOOTSTRAP` for sysroot builds][building-the-standard-library-on-a-stable-toolchain]
  - [Detect missing `rust-self-contained` components and provide diagnostics][self-contained-objects]
  - [Forcing many codegen-units for `compiler-builtins`][compiler-builtins]
- Project-wide
  - [Documenting build-std stability guarantees][stability-guarantees]
- Standard library
  - [Removing `restricted_std`][restricted_std]
  - [Moving configuration into the standard library's profile][proposal]

### New constraints on the standard library, compiler and bootstrap
[summary-constraints]: #new-constraints-on-the-standard-library-compiler-and-bootstrap

A stable mechanism for building the standard library imposes some constraints on
the rest of the toolchain that would need to be upheld:

- No further required customisation of the pre-built standard library through
  any means other than the profile in `Cargo.toml`
- Avoid mandatory C dependencies on the standard library
  - At the very least, new dependencies on the standard library will impact
    whether the standard library can be successfully built by users with varying
    environments and this impact will need to be considered going forward
  - New C dependencies will need to be careful not to cause symbol conflicts
    with user crates that pull in the same dependency (e.g. using
    [`links =...`][links])
    - If this did come up, it might be possible to work around it with
      postprocessing that renames C symbols used by the standard library but
      that would be better avoided
- The standard library continues to exist in its own workspace, with its own
  lockfile
- The name of the `test` crate becomes stable (but not its interface)
- The `panic-unwind` and `compiler-builtins-mem` `sysroot` features become
  stable so Cargo can refer to them
  - This should not necessitate a "stable/unstable features" mechanism, rather a
    guarantee from the library team that they're happy for these to stay
- Dependencies of the standard library cannot use build probes to detect whether
  nightly features can be used
  - With
    [*Assuming `RUSTC_BOOTSTRAP` for sysroot builds*][building-the-standard-library-on-a-stable-toolchain],
    these build probes would always assume the crate is being built on nightly

> [!NOTE]
>
> Cargo will likely be made a [JOSH] subtree of the [rust-lang/rust] so that all
> relevant parts of the toolchain can be updated in tandem when this is
> necessary.

[JOSH]: https://josh-project.github.io/josh/intro.html
[rust-lang/rust]: https://github.com/rust-lang/rust
[links]: https://doc.rust-lang.org/nightly/cargo/reference/manifest.html#the-links-field
