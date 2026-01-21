- Feature Name: `build-std-context`
- Start Date: 2025-06-05
- RFC PR: [rust-lang/rfcs#3873](https://github.com/rust-lang/rfcs/pull/3873)
- Rust Issue: N/A

## Summary
[summary]: #summary

While Rust's pre-built standard library has proven itself sufficient for the
majority of use cases, there are a handful of use cases that are not well
supported:

1. Rebuilding the standard library to match the user's profile
2. Rebuilding the standard library with ABI-modifying flags
3. Building the standard library for tier three targets

Proposals to solve these problems come broadly under the umbrella of "build-std"
and date back over 10 years ago, though no complete solution has yet reached
consensus.

**This RFC does not propose any changes directly, only document the background,
history and motivations for build-std. It is part of a series of build-std RFCs
and later RFCs will reference this one.** This RFC is part of the
[build-std project goal].

1. build-std context (this RFC)
    - [Background][background]
    - [History][history]
    - [Motivation][motivation]
2. `build-std="always"` ([rfcs#3874])
    - [Proposal][rfcs#3874-proposal]
    - [Rationale and alternatives][rfcs#3874-rationale-and-alternatives]
    - [Unresolved questions][rfcs#3874-unresolved-questions]
    - [Future possibilities][rfcs#3874-future-possibilities]
    - [Summary of proposed changes][rfcs#3874-summary]
3. Explicit standard library dependencies ([rfcs#3875])
    - [Proposal][rfcs#3875-proposal]
    - [Rationale and alternatives][rfcs#3875-rationale-and-alternatives]
    - [Unresolved questions][rfcs#3875-unresolved-questions]
    - [Future possibilities][rfcs#3875-future-possibilities]
4. `build-std="compatible"` (RFC not opened yet)
5. `build-std="match-profile"` (RFC not opened yet)

This RFC is co-authored by [David Wood][davidtwco] and
[Adam Gemmell][adamgemmell]. To improve the readability of this RFC, it does not
follow the standard RFC template, while still aiming to capture all of the
salient details that the template encourages.

There is also a [literature review appendix][appendix] in a HackMD which
contains a summary of all literature found during the process of writing this
RFC.

### Scope
[scope]: #scope

build-std has a long and storied history of previous discussions and proposals
which cover a large area and many use-cases. Any individual future RFC will not
be able to support many use cases that those waiting for build-std hope that it
will. This is also an explicit and deliberate choice for the build-std project
goal's proposals.

This RFC will focus on summarising these previous discussions and proposals in
order to enable an MVP of build-std to be accepted and stabilised. This will lay
the foundation for future proposals to lift restrictions and enable build-std to
support more use cases, without those proposals having to survey the ten-plus
years of issues, pull requests and discussion that this RFC has.

### Acknowledgements
[acknowledgements]: #acknowledgements

This RFC would not have been possible without the advice, feedback and support
of [Josh Triplett][joshtriplett], [Eric Huss][ehuss],
[Wesley Wiser][wesleywiser] and [Tomas Sedovic][tomassedovic] throughout this
entire effort.

Thanks to [mati865] for advising on some of the specifics related to special
object files, [petrochenkov] for his expertise on rustc's dependency loading and
name resolution; [fee1-dead] for their early and thorough reviews;
[Ed Page][epage] for writing about opaque dependencies and his invaluable Cargo
expertise; [Jacob Bramley][jacobbramley] for his feedback on early drafts; as
well as [Amanieu D'Antras][amanieu], [Tobias Bieniek][turbo87],
[Adam Harvey][lawngnome], [James Munns][jamesmunns],
[Jonathan Pallant][thejpster], [Jieyou Xu][jieyouxu], [Jakub Beránek][kobzol],
[Weihang Lo][weihanglo], and [Mark Rousskov][simulacrum] for providing feedback
from their areas of expertise on later drafts.

### Terminology
[terminology]: #terminology

The following terminology is used throughout the RFC:

- "the standard library" is used to refer to multiple of the crates that
  constitute the standard library such as `core`, `alloc`, `std`, `test`,
  `proc_macro` or their dependencies.
- "std" is used to refer only to the `std` crate, not the entirety of the
  standard library

Throughout the build-std project goal's later RFCs, parentheses with "?" links
([?][rationale-rationale]) will be present which link to the relevant "Rationale
and alternatives" section to justify a decision or provide alternatives to it.

Additionally, "note alerts" will be used in the *Proposal* sections to separate
implementation considerations from the core proposal. Implementation details
should be considered non-normative. These details could change during
implementation and are present solely to demonstrate that the implementation
feasibility has been considered and to provide an example of how implementation
could proceed.

> [!NOTE]
>
> This is an example of a "note alert" that will be used to separate
> implementation detail from the proposal proper.

## Background
[background]: #background

This section aims to introduce any relevant details about the standard library
and compiler that are assumed knowledge by referenced sources and later
RFCs.

See [*Implementation summary*][implementation-summary] for a summary of the
current unstable build-std feature in Cargo.

### Standard library
[background-standard-library]: #standard-library

Since the first stable release of Rust, the standard library has been distributed
as a pre-built artifact via rustup, which has a variety of advantages and/or
rationale:

- It saves Rust users from having to rebuild the standard library whenever they
  start a project or do a clean build
- The standard library has and has had dependencies which require a more
  complicated build environment than typical Rust projects
  - e.g. requiring a working C toolchain to build `compiler_builtins`' `c`
    feature
- To varying degrees at different times in its development, the standard
  library's implementation has been tied to the compiler implementation and has had
  to change in lockstep

Not all targets have a pre-built standard library distributed via rustup, though
it is a minimum requirement for certain platform support tiers. According to
rustc's [platform support docs][platform-support], for tier three targets:

> Tier 3 targets are those which the Rust codebase has support for, but which
> the Rust project does not build or test automatically, so they may or may not
> work. Official builds are not available.

..and tier two targets:

> The Rust project builds official binary releases of the standard library (or,
> in some cases, only the core library) for each tier 2 target, and automated
> builds ensure that each tier 2 target can be used as build target after each
> change.

..and finally, tier one targets:

> The Rust project builds official binary releases for each tier 1 target, and
> automated testing ensures that each tier 1 target builds and passes tests
> after each change.

As an innate property of the target, not all targets can support the `std` crate.
This is independent of its tier, where as stated in the
[Target Tier Policy][target-tier-policy] lower-tier targets may not have a
complete implementation for all APIs in the crates they can support.

All of the standard library crates leverage permanently unstable features
provided by the compiler that will never be stabilised and therefore require
nightly to build.

The configuration for the pre-built standard library build is spread across
bootstrap, the standard library workspace, individual standard library crate
manifests and the target specification. The pre-built standard library is
installed into the sysroot.

At the beginning of compilation, unless the crate has the `#![no_std]`
attribute, the compiler will load the `libstd.rlib` file from the sysroot as a
dependency of the current crate and add an implicit `extern crate std` for it.
This is the mechanism by which every crate has an implicit dependency on the
standard library.

The standard library sources are distributed in the `rust-src` component by
rustup and placed in the sysroot under `lib/rustlib/src/`. The sources consist
of the `library/` workspace plus `src/llvm-project/libunwind`, which was
required in the past to build the `unwind` crate on some targets.

Cargo supports explicitly declaring a dependency on crates with the same names
as standard library crates with a `path` source
(e.g. `core = { path = "../my_core" }`), which rustc will load instead of crates
in the sysroot. Crates with these dependencies are not accepted by crates.io,
but there are crates on GitHub that use this pattern, such as
[embed-rs/stm32f7-discovery][embed-rs-cargo-toml], which are used as `git`
dependencies of other crates on GitHub.

#### Dependencies
[background-dependencies]: #dependencies

Behind the facade, the standard library is split into multiple crates, some of
which are in different repositories and included as submodules or using [JOSH].

As well as local crates, the standard library depends on crates from crates.io.
It needs to be able to point these crates' dependencies on the standard library
at the sources of `core`, `alloc` and `std` in the current [rust-lang/rust]
checkout.

This is achieved through use of the `rustc-dep-of-std` feature. Crates used in
the dependency graph of `std` declare a `rustc-dep-of-std` feature and when
enabled, add new dependencies on `rustc-std-workspace-{core,alloc,std}`.
`rustc-std-workspace-{core,alloc,std}` are empty crates published on crates.io.
As part of the workspace for the standard library,
`rustc-std-workspace-{core,alloc,std}` are patched with a `path` source to the
directory for the corresponding crate.

Historically, there have necessarily been C dependencies of the standard library,
increasing the complexity of the build environment required. While these have
largely been removed over time - for example, `libbacktrace` previously depended
on `backtrace-sys` but now uses `gimli` ([rust#46439]), a pure-rust
implementation. There are still some C dependencies:

- `libunwind` will either link to the LLVM `libunwind` or the system's
  `libunwind`/`libgcc_s`. LLVM's `libunwind` is shipped as part of the
  rustup component for the standard library and will be linked against
  when `-Clink-self-contained` is used
  - This only applies to Linux and Fuchsia targets
- `compiler_builtins` has an optional `c` feature that will use optimised
  routines from `compiler-rt` when enabled. It is enabled for the pre-built
  standard library
- `compiler_builtins` has an optional `mem` feature that provides symbols
  for common memory routines (e.g. `memcpy`)
  - It is enabled automatically on some `no_std` platforms as when `std` is
    built `libc` provides these routines.
  - Users can rely on weak linkage to override these symbols, but in scenarios
    where weak linkage is not supported or where the symbols are to be
    overridden from a shared library, then users must directly turn the feature
    off.
- To use sanitizers, the sanitizer runtimes from LLVM's compiler-rt need to
  be linked against. Building of these is enabled in `bootstrap.toml`
  ([`build.sanitizers`][bootstrap-sanitizers]) and they are
  included in the rustup components shipped by the project.

Dependencies of the standard library may use unstable or internal compiler and
language features only when they are a dependency of the standard library.

#### Features
[background-features]: #features

There are a handful of features defined in the standard library crates'
`Cargo.toml`s. These features are not strictly additive (`llvm-libunwind` and
`system-llvm-libunwind` are mutually exclusive). There is currently no stable
existing mechanism for users to enable or disable these features. The default
set of features is determined by [logic in bootstrap][bootstrap-features-logic]
and [the `rust.std-features` key in `bootstrap.toml`][bootstrap-features-toml].
The enabled features are often different depending on the target.

It is also common for user crates to depend on the standard library (via
`#![no_std]`) conditional on Cargo features being enabled or disabled (e.g. a
`std` feature or if `--test` is used).

#### Target support
[background-target-support]: #target-support

The `std` crate's [`build.rs`][std-build.rs] checks for supported values of the
`CARGO_CFG_TARGET_*` environment variables. These variables are akin to the
conditional compilation [configuration options][conditional-compilation-config-options],
and often correspond to parts of the target triple (for example,
`CARGO_CFG_TARGET_OS` corresponds to the "os" part of a target triple - "linux"
in "aarch64-unknown-linux-gnu"). This filtering is strict enough to distinguish
between built-in targets but loose enough to match similar custom targets. There
is no equivalent mechanism on the `alloc` or `core` crates.

When encountering an unknown or unsupported operating system then the
`restricted_std` cfg is set. `restricted_std` marks the entire standard library
as unstable, requiring `feature(restricted_std)` to be enabled on any crate that
depends on it. The only way for users to enable the `restricted_std` feature on
behalf of dependencies is the uncommon `-Zcrate-attr=features(restricted_std)`
rustc flag and users commonly report that they are not aware how to do this.

Cargo and rustc support custom targets, defined in JSON files according to an
unstable schema defined in the compiler. On nightly, users can dump the
target-spec-json for an existing target using `--print target-spec-json`. This
JSON can be saved in a file, tweaked and used as the argument to `--target`. It
is unintentional but custom target specifications can be used with `--target`
even on stable toolchains ([rust#71009] proposes destabilising this behaviour).
However, as custom targets do not have a pre-built standard library and so must
use `-Zbuild-std`, their use is relegated to nightly toolchains in practice.
Custom targets may have `restricted_std` set depending on their `cfg`
configuration options.

### Prelude
[background-prelude]: #prelude

rustc has the concept of the "extern prelude" which is the set of crates that
can be referred to without an explicit `extern crate` statement. Originally this
was populated by users writing `extern crate $crate` in their code for each
direct dependency. Since the 2018 edition, crates passed via `--extern` are
added to the extern prelude. `core` is always added to the extern prelude. For
crates without the `#![no_std]` attribute, `std` is added to the extern prelude.

`core` or `std`'s prelude module (depending on the presence of `#![no_std]`) is
imported by rustc injecting a `use $crate::prelude::rust_20XX::*` statement.

`extern crate` can still be used and will search for the dependency in locations
where direct dependencies can be found, such as `-L crate=` paths or in the
sysroot. `-L dependency=` paths will not be searched, as these directories only
contain indirect dependencies (i.e. dependencies of direct dependencies).

Although only `std` or `core` are added to the extern prelude automatically,
users can still write `extern crate alloc` or `extern crate test` to load them
from the sysroot.

`--extern` has a `noprelude` modifier which will allow the user to use
`--extern` to specify the location at which a crate can be found without adding
it to the extern prelude. This could allow a path for crates like `alloc` or
`test` to be provided without affecting the observable behaviour of the
language.

### Panic strategies
[background-panic-strategies]: #panic-strategies

Rust has the concept of a *panic handler*, which is a crate that is responsible
for performing a panic. There are various panic handler crates on crates.io,
such as [panic-abort] (which is different from the `panic_abort` panic
runtime!), [panic-halt], [panic-itm], and [panic-semihosting]. Panic handler
crates define a function annotated with `#[panic_handler]`. There can only be
one `#[panic_handler]` in the crate graph.

`core` uses the panic handler to implement panics inserted by code generation
(e.g. arithmetic overflow or out-of-bounds access) and the `core::panic!` macro
immediately delegates to the panic handler crate.

`std` defines a panic handler. `std`'s panic handler function and its
`std::panic!` macro print panic information to stderr and delegate to a
*panic runtime* to decide what to do next, determined by the *panic strategy*.

There are two panic runtime crates in the standard library - `panic_unwind`
(which gracefully unwinds the stack using `libunwind` and performs cleanup) and
`panic_abort` (which terminates the program shortly after being called). Each
target supported by rustc specifies a default panic strategy - either "unwind"
or "abort" - though these are only relevant if `std`'s panic handler is used
(i.e. the target isn't a `no_std` target or being used with a `no_std` crate).

Rust's `-Cpanic` flag allows the user to choose the panic strategy, with the
target's default as a fallback. If `-Cpanic=unwind` is provided then this
doesn't guarantee that the unwind strategy is used, as the target may not
support it.

Both crates are compiled and shipped with the pre-built standard library for
targets which support `std`. Some targets have a pre-built standard library with
only the `core` and `alloc` crates, such as the `x86_64-unknown-none` target.
While `x86_64-unknown-none` defaults to the `abort` panic strategy, as this
target does not support the standard library, this default isn't actually
relevant.

The `std` crate has a `panic_unwind` feature that enables an optional dependency
on the `panic_unwind` crate.

`core` also provides support for the (unstable) `-Cpanic=immediate_abort`
strategy by modifying the `core::panic!` macro to immediately call the abort
intrinsic without calling the panic handler, which can dramatically reduce code
size. `std` also adds an immediate abort to its `panic!` macro.

### Cargo
[background-cargo]: #cargo

Cargo's building of the dependency graph is largely driven by the registry
index, except for crates from `git` or `path` sources.

[Cargo registries][cargo-docs-registry], like crates.io, are centralised sources
for crates. A registry's index is the interface between Cargo and the registry
that Cargo queries to know which versions are available for any given crate,
what its dependencies are, etc.

Cargo can query registries using a Git protocol which caches the registry on
disk, or using a sparse protocol which exposes the index over HTTP and allows
Cargo to avoid having a local copy of the whole index, which has become quite
large for crates.io.

crates.io's registry index is exposed as both a HTTP API and a Git repository -
[rust-lang/crates.io-index] - both are updated automatically by crates.io when
crates are published, yanked, etc. The HTTP API is mostly used.

Each crate in the registry index has a JSON file, following
[a defined schema][cargo-json-schema] which is jointly maintained by the Cargo
and crates.io teams. Crates may refer to those in other registries, but all
non-`path`/`git` crates in the dependency graph must exist in a registry. As the
registry index drives the building of Cargo's dependency graph, all
non-`path`/`git` crates that end up in the dependency graph must be present in a
registry.

When a package is published, Cargo posts a JSON blob to the registry which is
not an index entry but has sufficient information to generate one. crates.io does
not use Cargo's JSON blob, instead re-generating it from the `Cargo.toml` (this
avoids the index and `Cargo.toml` from going out-of-sync due to bugs or
malicious publishes). As a consequence, changes to the index format must be
duplicated in Cargo and crates.io. Behind the scenes, data from the `Cargo.toml`
extracted by crates.io is written to a database, which is where the index entry
and frontend are generated from.

Dependency information of crates in the registry are rendered in the crates.io
frontend.

Registries can have different policies for what crates are accepted. For
example, crates.io does not permit publishing packages named `std` or `core` but
other registries might.

#### Public/private dependencies
[background-pubpriv-dependencies]: #publicprivate-dependencies

[Public and private dependencies][rust#44663] are an unstable feature which
enables declaring which dependencies form part of a library's public interface,
so as to make it easier to avoid breaking semver compatibility.

With the `public-dependency` feature enabled, dependencies are marked as
"private" by default which can be overridden with a `public = true` declaration.

Private dependencies are passed to rustc with a `priv` modifier to the
`--extern` flag. Dependencies without this modifier are treated as public by
rustc for backwards compatibility reasons. rust emits the
`exported-private-dependencies` lint if an item from a private dependency is
re-exported.

### Target modifiers
[background-target-modifiers]: #target-modifiers

[rfcs#3716] introduced the concept of *target modifiers* to rustc. Flags marked
as target modifiers must match across the entire crate graph or the compilation
will fail.

For example, flags are made target modifiers when they change the ABI of
generated code and could result in unsound ABI mismatches if two crates are
linked together with different values of the flag set.

## History
[history]: #history

*The following summary of the prior art is necessarily less detailed than the
source material, which is exhaustively surveyed in
[Appendix: Exhaustive literature review][appendix].*

### [rfcs#1133] (2015)
[rfcs-1133-2015]: #rfcs1133-2015

build-std was first proposed in a [2015 RFC (rfcs#1133)][rfcs#1133] by
[Ericson2314], aiming to improve support for targets that do not have a
pre-built standard library; to enable building the standard library with
different profiles; and to simplify `rustbuild` (now `bootstrap`). It also was
written with the goal of supporting the user in providing a custom
implementation of the standard library and supporting different implementations
of the language that provide their own standard libraries.

This RFC proposed that the standard library be made an explicit dependency in
`Cargo.toml` and be rebuilt automatically when required. An implicit dependency
on the standard library would be added automatically unless an explicit
dependency is written. This RFC was written prior to a stable `#![no_std]`
attribute and so does not address the circumstance where an implicit dependency
would make a `#![no_std]` crate fail to compile on a target that does not
support the standard library.

There were objectives of and possibilities enabled by the RFC that were not
shared with the project teams at the time, such as the standard library being
a regular crate on crates.io and the concept of the sysroot being retired.
Despite this, the RFC appeared to be close to acceptance before being blocked
by Cargo having a mechanism to have unstable features and then closed in favour
of [cargo#4959].

### [xargo] and [cargo#4959] (2016)
[xargo-and-cargo-4959-2016]: #xargo-and-cargo4959-2016

While the discussions around [rfcs#1133] were ongoing, [xargo] was released in
2016. Xargo is a Cargo wrapper that builds a sysroot with a customised standard
library and then uses that with regular Cargo operations (i.e. `xargo build`
performs the same operation as `cargo build` but with a customised standard
library). Configuration for the customised standard library was configured in
the `Xargo.toml`, supporting configuring codegen flags, profile settings, Cargo
features and multi-stage builds. It required nightly to build the standard
library as it did not use `RUSTC_BOOTSTRAP`. Xargo had inherent limitations due
to being a Cargo wrapper, leading to suggestions that its functionality be
integrated into Cargo.

[cargo#4959] is a proposal inspired by [xargo], suggesting that a `[sysroot]`
section be added to `.cargo/config` which would enable similar configuration to
that of `Xargo.toml`. If this configuration is set, Cargo would build and use a
sysroot with a customised standard library according to the configuration
specified and the release profile. This sysroot would be rebuilt whenever
relevant configuration changes (e.g. profiles). [cargo#4959] received varied
feedback: the proposed syntax was not sufficiently user-friendly; it did not
enable the user to customise the standard library implementation; and that
exposing bootstrap stages was brittle and user-unfriendly. [cargo#4959] wasn't
updated after submission so ultimately stalled and remains open.

[rfcs#1133] and [cargo#4959] took very different approaches to build-std, with
[cargo#4959] proposing a simpler approach that exposed the necessary low-level
machinery to users and [rfcs#1133] attempting to take a more first-class and
user-friendly approach that has many tricky design implications.

### [rfcs#2663] (2019)
[rfcs-2663-2019]: #rfcs2663-2019

In 2019, [*rfcs#2663: `std` Aware Cargo*][rfcs#2663] was opened as the most
recent RFC attempting to advance build-std. [rfcs#2663] shared many of the
motivations of [rfcs#1133]: building the standard library for tier three and
custom targets; customising the standard library with different Cargo features;
and applying different codegen flags to the standard library. It did not concern
itself with build-std's potential use in `rustbuild` or with abolishing the
sysroot.

[rfcs#2663] was primarily concerned with what functionality should be available
to the user and what the user experience ought to be. It proposed that `core`,
`alloc` and `std` be automatically built when the target did not have a
pre-built standard library available through rustup. It would be automatically
rebuilt on any target when the profile configuration was modified such that it
no longer matched the pre-built standard library. If using nightly, the user
could enable Cargo features and modify the source of the standard library.
Standard library dependencies were implicit by default, as today, but would be
written explicitly when enabling Cargo features. It also aimed to stabilise the
target-spec-json format and allow "stable" Cargo features to be enabled on
stable toolchains, and as such proposed the concept of stable and unstable Cargo
features be introduced.

There was a lot of feedback on [rfcs#2663] which largely stemmed from it being
very high-level, containing many large unresolved questions and details left for
the implementers to work out. For example, it proposed that there be a concept
of stable and unstable Cargo features but did not elaborate any further, leaving
that as an implementation detail. Nevertheless, the proposal was valuable in
more clearly elucidating a potential user experience that build-std could aim
for, and the feedback provided was incorporated into the [wg-cargo-std-aware]
effort, described below.

### [wg-cargo-std-aware] (2019-)
[wg-cargo-std-aware-2019-]: #wg-cargo-std-aware-2019-

[rfcs#2663] demonstrated that there was demand for a mechanism for being able to
(re-)build the standard library, and the feedback showed that this was a thorny
problem with lots of complexity, so in 2019, the [wg-cargo-std-aware] repository
was created to organise related work and explore the issues involved in
build-std.

[wg-cargo-std-aware] led to the current unstable implementation of `-Zbuild-std`
in Cargo, which is described in detail in the [*Implementation summary*
section][implementation-summary] below.

Issues in the wg-cargo-std-aware repository can be roughly partitioned into seven
categories:

1. **Exploring the motivations and use cases for the standard library**

   There are a handful of motivations catalogued in the [wg-cargo-std-aware]
   repository, corresponding to those raised in the earlier RFCs and proposals:

   - Building with custom profile settings ([wg-cargo-std-aware#2])
   - Building for unsupported targets ([wg-cargo-std-aware#3])
   - Building with different Cargo features ([wg-cargo-std-aware#4])
   - Replacing the source of the standard library ([wg-cargo-std-aware#7])
   - Using build-std in bootstrap/rustbuild ([wg-cargo-std-aware#19])
   - Improving the user experience for `no_std` binary projects
     ([wg-cargo-std-aware#36])

   These are all either fairly self-explanatory, described in the summary of the
   previous RFCs/proposals above, or in the [*Motivation*][motivation] section
   of this RFC.

2. **Support for build-std in Cargo's subcommands**

   Cargo has various subcommands where the desired behaviour when used with
   build-std needs some thought and consideration. A handful of issues were
   created to track this, most receiving little to no discussion:
   [`cargo metadata`][wg-cargo-std-aware#20], [`cargo clean`][wg-cargo-std-aware#21],
   [`cargo pkgid`][wg-cargo-std-aware#24], and [the `-p` flag][wg-cargo-std-aware#26].

   [`cargo fetch`][wg-cargo-std-aware#22] had fairly intuitive interactions with
   build-std - that `cargo fetch` should also fetch any dependencies of the
   standard library - which was implemented in [cargo#10129].

   The [`--build-plan` flag][wg-cargo-std-aware#45] does not support build-std and its
   issue did not receive much discussion, but the future of this flag in its
   entirety seems to be uncertain.

   [`cargo vendor`][wg-cargo-std-aware#23] did receive lots of discussion.
   Vendoring the standard library is desirable (for the same reasons as any
   vendoring), but would lock the user to a specific version of the toolchain
   when using a vendored standard library. However, if the `rust-src` component
   contained already-vendored dependencies, then `cargo vendor` would not need
   to support build-std and users would see the same advantages.

   Vendored standard library dependencies were implemented using a hacky
   approach (necessarily, prior to the standard library having its own
   workspace), but this was later reverted due to bugs. No attempt has been made
   to reimplement vendoring since the standard library has had its own
   workspace.

3. **Dependencies of the standard library**

   There are a handful of dependencies of the standard library that may pose
   challenges for build-std by dint of needing a working C toolchain or
   special-casing.

   [`libbacktrace`][wg-cargo-std-aware#16] previously required a C compiler to
   build `backtrace-sys`, but now uses `gimli` internally.

   [`compiler_builtins`][wg-cargo-std-aware#15] has a `c` feature that uses C
   versions of some intrinsics that are more optimised. This is used by the
   pre-built standard library, and if not used by build-std, could be a point of
   divergence. `compiler-builtins/c` can have a significant impact on code
   quality and build size. It also has a `mem` feature which provides symbols
   (`memcpy`, etc) for platforms without `std` that don't have these same
   symbols provided by `libc`. `compiler_builtins` is also built with a large
   number of compilation units to force each function into a different unit,
   avoiding unintentionally bringing in a symbol that conflicts with one in the
   system's `libgcc`.

   ['unwind'][wg-cargo-std-aware#29] links to the system's version of libunwind.
   Enabling the `llvm-libunwind` feature, `-Clink-self-contained` or
   `-Ctarget-feature=+crt-static` will statically link to the pre-built
   `libunwind` distributed in the standard library component for the target, if
   present.

   [Sanitizers][wg-cargo-std-aware#17], when enabled, require a sanitizer
   runtime to be present. These are currently built by bootstrap and part of
   LLVM.

4. **Design considerations**

   There are many design considerations discussed in the [wg-cargo-std-aware]
   repository:

   [wg-cargo-std-aware#5] explored how/if dependencies on the standard library
   should be declared. The issue claims that users should have to opt-in to
   build-std, support alternative standard library implementations, and that
   Cargo needs to be able to pass `--extern` to rustc for all dependencies.

   It is an open question how to handle multiple dependencies each declaring a
   dependency on the standard library. A preference towards unifying standard
   library dependencies was expressed (these would have no concept of a version,
   so just union all features).

   There was no consensus on how to find a balance between explicitly depending
   on the standard library versus implicitly, or on whether the pre-built-ness
   of a dependency should be surfaced to the user.

   [wg-cargo-std-aware#6] argues that target-spec-json would be de-facto stable
   if it can be used by build-std on stable. While `--target=custom.json` can be
   used on stable today, it effectively requires build-std and so a nightly
   toolchain. As build-std enables custom targets to be used on stable, this
   would effectively be a greater commitment to the current stability of custom
   targets than currently exists and would warrant an explicit decision.

   [wg-cargo-std-aware#8] highlighted that a more-portable standard library
   would be beneficial for build-std (i.e. a `std` that could build on any
   target), but that making the standard library more portable isn't necessarily
   in-scope for build-std.

   [wg-cargo-std-aware#11] investigated how build-std could get the standard
   library sources. rustup can download `rust-src`, but there was a preference
   expressed that rustup not be required. Cargo could have reasonable default
   probing locations that could be used by distros and would include where
   rustup puts `rust-src`.

   [wg-cargo-std-aware#12] concluded that the `Cargo.lock` of the standard
   library would need to be respected so that the project can guarantee that the
   standard library works with the project's current testing.

   [wg-cargo-std-aware#13] explored how to determine the default set of cfg
   values for the standard library. This is currently computed by bootstrap.
   This could be duplicated in Cargo in the short-term, made visible to
   build-std through some configuration, or require the user to explicitly
   declare them.

   [wg-cargo-std-aware#14] looks into additional rustc flags and environment
   variables passed by bootstrap to the compiler. A comparison of the
   compilation flags from bootstrap and build-std was
   [posted in a comment][wg-cargo-std-aware#14-review]. No solutions were
   suggested, other than that it may need a similar mechanism as
   [wg-cargo-std-aware#13].

   [wg-cargo-std-aware#29] tries to determine how to support different panic
   strategies. Should Cargo use the profile to decide what to use? How does it
   know which panic strategy crate to use? It is argued that Cargo ought to work
   transparently - if the user sets the panic strategy differently then a
   rebuild is triggered.

   [wg-cargo-std-aware#30] identifies that some targets have special handling in
   bootstrap which will need to be duplicated in build-std. Targets could be
   allowlisted or denylisted to avoid having to address this initially.

   [wg-cargo-std-aware#38] argues that a forced lock of the standard library
   is desirable, to which there was no disagreement. This was more relevant
   when build-std did not use the on-disk `Cargo.lock`.

   [wg-cargo-std-aware#39] explores the interaction between build-std and
   public/private dependencies ([rfcs#3516]). Should the standard library always
   be public? There were no solutions presented, only that if defined in
   `Cargo.toml`, the standard library will likely inherit the default from that.

   [wg-cargo-std-aware#43] investigates the options for the UX of build-std.
   `-Zbuild-std` flag is not a good experience as it needs added to every
   invocation and has few extension points. Using build-std should be an
   unstable feature at first. It was argued that build-std should be transparent
   and happen automatically when Cargo determines it is necessary. There are
   concerns that this could trigger too often and that it should only happen
   automatically for ABI-modifying flags.

   [wg-cargo-std-aware#46] observes that some targets link against special
   object flags (e.g. `crt1.o` on musl) and that build-std will need to handle
   these without hardcoding target-specific logic. There were no conclusions,
   but `-Clink-self-contained` might be able to help.

   [wg-cargo-std-aware#47] discusses how to handle targets that typically ship
   with a different linker (e.g. `rust-lld` or `gcc`). `rust-lld` is now shipped
   by default reducing the potential impact of this, though it is discovered via
   the sysroot, and so will need to be found via another mechanism if disabled.

   [wg-cargo-std-aware#50] argues that the impact on build probes ought to be
   considered and was later closed as t-cargo do not want to support build
   probes.

   [wg-cargo-std-aware#51] plans for removal of `rustc-dep-of-std`, identifying
   that if explicit dependencies on the standard library are adopted, that the
   need for this feature could be made redundant.

   [wg-cargo-std-aware#68] notices that `profiler_builtins` needs to be compiled
   after `core` (i.e. `core` can't be compiled with profiling). The error
   message has been improved for this but there was otherwise no commentary.
   This has changed since the issue was filed, as `profiler_builtins` is now a
   `#![no_core]` crate.

   [wg-cargo-std-aware#85] considers that there has to be a deliberate testing
   strategy in place between the [rust-lang/rust] and [rust-lang/cargo]
   repositories to ensure there is no breakage. `rust-toolstate` could be used
   but is not very good. Alternatively, Cargo could become a [JOSH] subtree of
   [rust-lang/rust].

   [wg-cargo-std-aware#86] proposes that the initial set of targets supported by
   build-std be limited at first to further reduce scope and limit exposure to
   the trickier issues.

   [wg-cargo-std-aware#88] reports that `cargo doc -Zbuild-std` doesn't generate
   links to the standard library. Cargo doesn't think the standard library comes
   from crates.io, and bootstrap isn't involved to pass
   `-Zcrate-attr="doc(html_root_url=..)"` like in the pre-built standard
   library.

   [wg-cargo-std-aware#90] asks how `restricted_std` should apply to custom
   targets. `restricted_std` is triggered based on the `target_os` value, which
   means it will apply for some custom targets but not others. build-std needs
   to determine what guarantees are desirable/expected. Current implementation
   wants slightly-modified-from-default target specs to be accepted and
   completely new target specs to hit `restricted_std`.

   [wg-cargo-std-aware#92] suggests that some targets could be made "unstable"
   and as such only support build-std on nightly. This forces users of those
   targets to use nightly where they will receive more frequent fixes for their
   target. It would also permit more experimentation with build-std while
   enabling stabilisation for mainstream targets.

5. **Implementation considerations**
   These won't be discussed in this summary, see [the implementation summary][implementation-summary]
   or [the relevant section of the literature review for more detail][appendix-impl]

6. **Bugs in the compiler or standard library**
   These aren't especially relevant to this summary, see [the relevant section
   of the literature review for more detail][appendix-bugs]

7. **Cargo feature requests narrowly applied to build-std**
   These aren't especially relevant to this summary, see [the relevant section
   of the literature review for more detail][appendix-cargo-feats]

Since around 2020, activity in the [wg-cargo-std-aware] repository largely
trailed off and there have not been any significant developments related to
build-std since.

#### Implementation summary
[implementation-summary]: #implementation-summary

*An exhaustive review of implementation-related issues, pull requests and
discussions can be found in
[the relevant section of the literature review][appendix-impl].*

There has been an unstable and experimental implementation of build-std in Cargo
since August 2019 ([wg-cargo-std-aware#10]/[cargo#7216]).

[cargo#7216] added the [`-Zbuild-std`][build-std] flag to Cargo. `-Zbuild-std`
re-builds the standard library crates which rustc then uses instead of the
pre-built standard library from the sysroot.

Originally, `-Zbuild-std` always build `std` by default. Since the addition of
the `std` field to target metadata in [rust#122305], Cargo only builds `std` by
default if `metadata.std` is true.

`test` is also built if `std` is being built and tests are being run with the
default harness.

Optionally, users can provide the list of crates to be built, though this was
intended as an escape hatch to work around bugs - the arguments to the flag are
unstable since the names of crates comprising the standard library are not
stable.

Cargo has a hardcoded list of what dependencies need to be added for a given
user-requested crate (i.e. `std` implies building `core`, `alloc`,
`compiler_builtins`, etc.). It is common for users to manually specify the
`panic_abort` crate.

Originally, `-Zbuild-std` required that `--target` be provided
([wg-cargo-std-aware#25]) to force Cargo to use different sysroots for the host
and target , but this restriction was later resolved ([cargo#14317]).

A second flag, [`-Zbuild-std-features`][build-std-features], was added in
[cargo#8490] and allows overriding the default Cargo features of the standard
library. Like the arguments to `-Zbuild-std`, the values accepted by this flag
are inherently unstable as the library team has not committed to any of the
standard library's Cargo features being stable. Features are enabled on the
`sysroot` crate and propagate down through the crate graph of the standard
library (e.g. `compiler-builtins-mem` is a feature in `sysroot`, `std`, `alloc`,
and `core` until `compiler_builtins`).

build-std gets the source of the standard library from the `rust-src` rustup
component. This does not happen automatically and the user must ensure the
component has been downloaded themselves. Only the standard library crates from
the [rust-lang/rust] repository are included in the `rust-src` dependency (i.e.
none of the crates.io dependencies).

When `-Zbuild-std` has been passed, Cargo creates a second workspace for the
standard library based on the `Cargo.{toml,lock}` from the `rust-src` component.
Originally this was an in-memory workspace, prior to the standard library having
a separate workspace from the compiler which could be used independently
([rust#128534]/[cargo#14358]). This workspace is then resolved separately and
the resolve is combined with the user's resolve to produce a dependency graph of
things to build with the user's crates depending on the standard library's
crates. Some additional work is done to deduplicate crates across the graph and
then this crate graph is used to drive work (usually rustc invocations) as
usual. This approach allows for build-time parallelism and sharing of crates
between the two separate resolves but does involve `build-std`-specific logic in
and around unit generation and is very unlike the rest of Cargo
([wg-cargo-std-aware#64]).

Resolving the standard library separately from the user's crate helps guarantee
that the exact dependency versions of the pre-built standard library are used,
which is a key constraint ([wg-cargo-std-aware#12]). Locking the standard
library could also help ([wg-cargo-std-aware#38]). A consequence of this is that
each of the Cargo subcommands (e.g. `cargo metadata`) need to have special
support for build-std implemented, but this might be desirable.

The standard library crates are considered non-local packages and so are not
compiled with incremental compilation or dep-info fingerprint tracking and any
warnings will be silenced.

build-std provides newly-built standard library dependencies to rustc using
`--extern noprelude:$crate`. `noprelude` was added in [rust#67074] to support
build-std and ensure that loading from the sysroot and using `--extern` were
equivalent ([wg-cargo-std-aware#40]). Prior to the addition of `noprelude`,
build-std briefly created new sysroots and used those instead of `--extern`
([cargo#7421]). rustc can still try to load a crate from the sysroot if the user
uses it which is currently a common source of confusing "duplicate lang item"
errors (as the user ends up with build-std `core` and sysroot `core`
conflicting).

Host dependencies like build scripts and `proc_macro` crates use the
existing pre-built standard library from the sysroot, so Cargo does not
pass `--extern` to those.

Modifications to the standard library are not supported. While build-std
has no mechanism to detect or prevent modifications to the `rust-src` content,
rebuilds aren't triggered automatically on modifications. The user cannot
override dependencies in the standard library workspace with `[patch]` sections
of their `Cargo.toml`.

To simplify build-std in Cargo, build-std wants to be able to always build
`std`, which is accomplished through use of the
[`unsupported` module in `std`'s platform abstraction layer][std-unsupported],
and `restricted_std`. `std` checks for unsupported targets in its
[`build.rs`][std-build.rs] and applies the `restricted_std` cfg which marks the
standard library as unstable for unsupported targets.

Users can enable the `restricted_std` feature in their crates. This mechanism
has been noted as confusing ([wg-cargo-std-aware#87]) and has the issue that the
user cannot opt into the feature on behalf of dependencies
([wg-cargo-std-aware#69]).

The initial implementation does not include support for build-std in many of
Cargo's subcommands including `metadata`, `clean`, `vendor`, `pkgid` and the
`-p` options for various commands. Support for `cargo fetch` was implemented in
[cargo#10129].

### `no_std` Usability
[no_std-usability]: #no_std-usability

There are also issues related to the usability of `no_std` crates:

- Discoverability of `no_std` crates is difficult with a mix of categories
  (`no-std`) and keywords (`nostd`/`no_std`) that are not used consistently by
  `no_std` crates ([crates.io#7306]).

- `no_std` crates can accidentally and easily depend on crates that use `std`
  which can result in build failures in some targets ([cargo#8798]).

### Related work
[related-work]: #related-work

There are a variety of ongoing efforts, ideas, RFCs or draft notes describing
features that are related or would be beneficial for build-std:

- **[Opaque dependencies]**, [epage], May 2025
  - Introduces the concept of an opaque dependency that has its own
    `Cargo.lock`, `RUSTFLAGS` and `profile`
  - Opaque dependencies could enable a variety of build-time performance
      improvements:
    - Caching - differences in dependency versions can cause unique instances of
      every dependent crate
    - Pre-built binaries - can leverage a pre-built artifact for a given opaque
      dependency
      - e.g. the standard library's distributed `rlib`s
    - MIR-only/cross-crate lazy compilation - Small dependencies could be built
      lazily and larger dependencies built once
    - Optimising dependencies - dependencies could always be optimised when they
      are unlikely to be needed during debugging

## Motivation
[motivation]: #motivation

> [!IMPORTANT]
>
> This section lists all of the motivations that have been associated with
> build-std in its various iterations, but not all of these use cases will be
> addressed by this project goal.
>
> The motivations that will not be addressed are nevertheless mentioned here so
> that reviewers have a more complete context for what has and hasn't been
> desired of build-std over time.

While the pre-built standard library has been sufficient for the majority of
Rust users, there are a variety of use-cases which require the ability to
rebuild the standard library.

1. **Building the standard library without relying on unstable escape hatches**

    - While tangential to the core of build-std as a feature, projects like Rust
      for Linux want to be able to build crates from the standard library using
      a stable toolchain without relying on escape hatches like
      `RUSTC_BOOTSTRAP` that the Rust project does not encourage use of

        - It is relatively straightforward to support this, hence its inclusion

        - Cargo's implementation of build-std should be able to re-use whichever
          mechanism is designed to address this

2. **Building standard library crates that are not shipped for a target**

    - Targets which have limited `std` support may wish to use the subsets of
      the standard library which could work but are not shipped by the project
      (e.g. `std` on `x86_64-unknown-none`)

3. **Using the standard library with tier three targets**

    - There is no stable mechanism for using the standard library on a tier
      three target that does not ship a pre-built std

    - While it is common for these targets to not support the `std` crate, they
      should be able to use `core`

    - These users are forced to use nightly and the unstable `-Zbuild-std`
      feature or third-party tools like [cargo-xbuild] (formerly [xargo])

4. **Unblock stabilisation of ABI-modifying compiler flags**

    - Any compiler flags which change the ABI cannot currently be stabilised as
      they would immediately mismatch with the pre-built standard library

        - Without an ability to rebuild the standard library using these flags, it
          is impossible to use them effectively and safely if stabilised

    - ABI-modifying flags are designated as target modifiers
      ([rfcs#3716]/[rust#136966]) and require that the same value for the flag
      is passed to all compilation units

        - Flags which need to be set across the entire crate graph to uphold some
          property (i.e. enhanced security) are also target modifiers

        - For example: sanitizers, control flow integrity, `-Zfixed-x18`, etc

5. **Re-building the standard library with different codegen flags or profile**
   ([wg-cargo-std-aware#2])

    - Embedded users need to optimise aggressively for size, due to the limited
      space available on their target platforms, which can be achieved in Cargo
      by setting `opt-level = s/z` and `panic = "abort"` in their profile.
      However, these settings will not apply to the pre-built standard library

    - Similarly, when deploying to known environments, use of `target-cpu` or
      `target-feature` can improve the performance of code generation or allow
      the use of newer hardware features than the target's baseline provides. As
      above, these configurations will not apply to the pre-built standard
      library

    - While the pre-built standard library is built to support debugging without
      compromising size and performance by setting `debuginfo=1`, this isn't
      ideal, and building the standard library with the dev profile would
      provide a better experience when debugging

The following use cases are not currently planned as part of this project goal,
but could be supported with follow-up RFCs (and any RFCs proposed as part of
this goal will attempt to ensure they remain viable as future possibilities):

1. **Using the standard library with custom targets**

    - There is no stable mechanism for using the standard library for a custom
      target (using target-spec-json)

    - Like tier three targets, these targets often only support `core` and are
      forced to use nightly today

2. **Enabling Cargo features for the standard library** ([wg-cargo-std-aware#4])

    - There are opportunities to expose Cargo features from the standard library
      that would be useful for certain subsets of the Rust users.

        - For example, embedded users may want to enable `optimize_for_size` or
          disable `backtrace` to reduce binary size

3. **Progress towards using miri on a stable toolchain**

    - One of the limitations of miri is that it requires building the standard
      library with specific compiler flags that would not be appropriate for the
      pre-built standard library, this is part of miri's dependency on nightly
      to build its own sysroot using [rustc-build-sysroot]

Some use cases are unlikely to be supported by the project unless a new and
compelling use-case is presented, and so this project goal may make decisions
which make these motivations harder to solve in future:

1. **Modifying the source code of the standard library** ([wg-cargo-std-aware#7])

    - Some platforms require a heavily modified standard library that would not
      be suitable for upstreaming, such as [Apache's SGX SDK][sgx] which
      replaces some standard library and ecosystem crates with forks or custom
      crates for a custom `x86_64-unknown-linux-sgx` target

    - Similarly, some tier three targets may wish to patch standard library
      dependencies to add or improve support for the target

    - If a stable mechanism were provided to make such changes to the standard
      library, then this would constrain future standard library development.
      These changes are better attempted by maintaining a fork of the standard
      library.

2. **Retire the concept of the sysroot**

    - Earlier proposals for build-std were motivated in-part by the desire to see
      the concept of the sysroot retired.

      - This is challenging while maintaining backwards-compatibility,
        especially for users who do not use Cargo and assume rustc can find the
        standard library in the sysroot. Removing the sysroot has no advantages
        to the end-user of Rust in itself.

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

These rationales and alternatives apply to the build-std proposal as a whole:

### Why have rationale sections?
[rationale-rationale]: #why-have-rationale-sections

A separate rationale section makes for easier reading by letting the proposal
sections of the RFCs flow better without interruptions or tangents.

↩ [*Terminology*][terminology]

### Why not do nothing?
[rationale-why-not-do-nothing]: #why-not-do-nothing

Support for rebuilding the standard library is a long-standing feature request
from subsets of the Rust community and blocks the work of some project teams
(e.g. sanitisers and branch protection in the compiler team, amongst others).
Inaction forces these users to remain on nightly and depend on the unstable
`-Zbuild-std` flag indefinitely. RFCs and discussion dating back to the first
stable release of the language demonstrate the longevity of build-std as a
need.

### Shouldn't build-std be part of rustup?
[rationale-in-rustup]: #shouldnt-build-std-be-part-of-rustup

build-std is effectively creating a new sysroot with a customised standard
library. rustup as Rust's toolchain manager has existing machinery to create and
maintain sysroots, and if it could invoke Cargo to build the standard library
then it could create a new toolchain from a build from a `rust-src` component.
rustup would be invoking tools from the next layer of abstraction (Cargo) in the
same way that Cargo invokes tools from the layer of abstraction after it
(rustc).

A brief prototype of this idea was created and a
[short design document was drafted][why-not-rustup] before concluding that it
would not be possible. With Cargo's artifact dependencies it may be desirable
to build with a different standard library and if rustup was creating different
toolchains per-customised standard library then Cargo would need to have
knowledge of these to switch between them, which isn't possible (and something
of a layering violation). It is also unclear how Cargo would find and use the
uncustomized host sysroot for build scripts and procedural macros. In addition
rustup's knowledge of sysroots and toolchains is limited to the archives it
unpacks - it becoming a part of the build system is not trivial, especially
considering it uses a different versioning system to Cargo, Rust and the
standard library.

[build-std project goal]: https://rust-lang.github.io/rust-project-goals/2025h2/build-std.html

[rfcs#3874]: https://github.com/rust-lang/rfcs/pull/3874
[rfcs#3874-proposal]: https://github.com/davidtwco/rfcs/blob/build-std-part-two-always/text/3874-build-std-always.md#proposal
[rfcs#3874-rationale-and-alternatives]: https://github.com/davidtwco/rfcs/blob/build-std-part-two-always/text/3874-build-std-always.md#rationale-and-alternatives
[rfcs#3874-unresolved-questions]: https://github.com/davidtwco/rfcs/blob/build-std-part-two-always/text/3874-build-std-always.md#unresolved-questions
[rfcs#3874-future-possibilities]: https://github.com/davidtwco/rfcs/blob/build-std-part-two-always/text/3874-build-std-always.md#future-possibilities
[rfcs#3874-summary]: https://github.com/davidtwco/rfcs/blob/build-std-part-two-always/text/3874-build-std-always.md#summary-of-proposed-changes
[rfcs#3875]: https://github.com/rust-lang/rfcs/pull/3875
[rfcs#3875-proposal]: https://github.com/davidtwco/rfcs/blob/build-std-part-three-explicit-dependencies/text/3875-build-std-explicit-dependencies.md#proposal
[rfcs#3875-rationale-and-alternatives]: https://github.com/davidtwco/rfcs/blob/build-std-part-three-explicit-dependencies/text/3875-build-std-explicit-dependencies.md#rationale-and-alternatives
[rfcs#3875-unresolved-questions]: https://github.com/davidtwco/rfcs/blob/build-std-part-three-explicit-dependencies/text/3875-build-std-explicit-dependencies.md#unresolved-questions
[rfcs#3875-future-possibilities]: https://github.com/davidtwco/rfcs/blob/build-std-part-three-explicit-dependencies/text/3875-build-std-explicit-dependencies.md#future-possibilities

[davidtwco]: https://github.com/davidtwco
[adamgemmell]: https://github.com/adamgemmell
[amanieu]: https://github.com/Amanieu
[ehuss]: https://github.com/ehuss
[epage]: https://github.com/epage
[fee1-dead]: https://github.com/fee1-dead
[jacobbramley]: https://github.com/jacobbramley
[jamesmunns]: https://github.com/JamesMunns
[jieyouxu]: https://github.com/jieyouxu
[joshtriplett]: https://github.com/joshtriplett
[kobzol]: https://github.com/kobzol
[lawngnome]: https://github.com/LawnGnome
[mati865]: https://github.com/mati865
[petrochenkov]: https://github.com/petrochenkov
[simulacrum]: https://github.com/simulacrum
[thejpster]: https://github.com/thejpster
[tomassedovic]: https://github.com/tomassedovic
[turbo87]: https://github.com/Turbo87
[weihanglo]: https://github.com/weihanglo
[wesleywiser]: https://github.com/wesleywiser
[Ericson2314]: https://github.com/Ericson2314

[appendix]: https://hackmd.io/@davidtwco/BJG0jgZkbl
[appendix-impl]: https://hackmd.io/@davidtwco/BJG0jgZkbl#implementation
[appendix-bugs]: https://hackmd.io/@davidtwco/BJG0jgZkbl#bugs-in-the-compiler-or-standard-library
[appendix-cargo-feats]: https://hackmd.io/@davidtwco/BJG0jgZkbl#cargo-feature-requests-narrowly-applied-to-build-std

[why-not-rustup]: https://hackmd.io/@davidtwco/rkYRlKv_1x
[Opaque dependencies]: https://hackmd.io/@epage/ByGfPtRell

[JOSH]: https://josh-project.github.io/josh/intro.html
[panic-abort]: https://crates.io/crates/panic-abort
[panic-halt]: https://crates.io/crates/panic-halt
[panic-itm]: https://crates.io/crates/panic-itm
[panic-semihosting]: https://crates.io/crates/panic-semihosting
[rust-lang/cargo]: https://github.com/rust-lang/cargo
[rust-lang/crates.io-index]: https://github.com/rust-lang/crates.io-index
[rust-lang/rust]: https://github.com/rust-lang/rust
[sgx]: https://github.com/apache/incubator-teaclave-sgx-sdk
[wg-cargo-std-aware]: https://github.com/rust-lang/wg-cargo-std-aware
[cargo-xbuild]: https://github.com/rust-osdev/cargo-xbuild
[xargo]: https://github.com/japaric/xargo
[rustc-build-sysroot]: https://github.com/ralfjung/rustc-build-sysroot

[build-std]: https://doc.rust-lang.org/cargo/reference/unstable.html#build-std
[build-std-features]: https://doc.rust-lang.org/cargo/reference/unstable.html#build-std-features
[bootstrap-features-logic]: https://github.com/rust-lang/rust/blob/00b526212bbdd68872d6f964fcc9a14a66c36fd8/src/bootstrap/src/lib.rs#L732
[bootstrap-features-toml]: https://github.com/rust-lang/rust/blob/00b526212bbdd68872d6f964fcc9a14a66c36fd8/bootstrap.example.toml#L816
[bootstrap-sanitizers]: https://github.com/rust-lang/rust/blob/d13a431a6cc69cd65efe7c3eb7808251d6fd7a46/bootstrap.example.toml#L388
[cargo-docs-registry]: https://doc.rust-lang.org/nightly/nightly-rustc/cargo/sources/registry/index.html
[cargo-json-schema]: https://doc.rust-lang.org/cargo/reference/registry-index.html#json-schema
[conditional-compilation-config-options]: https://doc.rust-lang.org/reference/conditional-compilation.html#set-configuration-options
[embed-rs-cargo-toml]: https://github.com/embed-rs/stm32f7-discovery/blob/e2bf713263791c028c2a897f2eb1830d7f09eceb/Cargo.toml#L21
[platform-support]: https://doc.rust-lang.org/nightly/rustc/platform-support.html
[std-build.rs]: https://github.com/rust-lang/rust/blob/f315e6145802e091ff9fceab6db627a4b4ec2b86/library/std/build.rs#L17
[std-unsupported]: https://github.com/rust-lang/rust/blob/f768dc01da9a681716724418ccf64ce55bd396c5/library/std/src/sys/pal/mod.rs#L68-L69
[target-tier-policy]: https://doc.rust-lang.org/nightly/rustc/target-tier-policy.html

[cargo#10129]: https://github.com/rust-lang/cargo/pull/10129
[cargo#14317]: https://github.com/rust-lang/cargo/pull/14317
[cargo#14358]: https://github.com/rust-lang/cargo/pull/14358
[cargo#4959]: https://github.com/rust-lang/cargo/issues/4959
[cargo#7216]: https://github.com/rust-lang/cargo/pull/7216
[cargo#7421]: https://github.com/rust-lang/cargo/pull/7421
[cargo#8490]: https://github.com/rust-lang/cargo/pull/8490
[cargo#8798]: https://github.com/rust-lang/cargo/issues/8798
[crates.io#7306]: https://github.com/rust-lang/crates.io/pull/7306
[rfcs#1133]: https://github.com/rust-lang/rfcs/pull/1133
[rfcs#2663]: https://github.com/rust-lang/rfcs/pull/2663
[rfcs#3516]: https://rust-lang.github.io/rfcs/3516-public-private-dependencies.html
[rfcs#3716]: https://rust-lang.github.io/rfcs/3716-target-modifiers.html
[rust#44663]: https://github.com/rust-lang/rust/issues/44663
[rust#46439]: https://github.com/rust-lang/rust/pull/46439
[rust#71009]: https://github.com/rust-lang/rust/issues/71009
[rust#67074]: https://github.com/rust-lang/rust/issues/67074
[rust#122305]: https://github.com/rust-lang/rust/pull/122305
[rust#128534]: https://github.com/rust-lang/rust/pull/128534
[rust#136966]: https://github.com/rust-lang/rust/issues/136966

[wg-cargo-std-aware#2]: https://github.com/rust-lang/wg-cargo-std-aware/issues/2
[wg-cargo-std-aware#3]: https://github.com/rust-lang/wg-cargo-std-aware/issues/3
[wg-cargo-std-aware#4]: https://github.com/rust-lang/wg-cargo-std-aware/issues/4
[wg-cargo-std-aware#5]: https://github.com/rust-lang/wg-cargo-std-aware/issues/5
[wg-cargo-std-aware#6]: https://github.com/rust-lang/wg-cargo-std-aware/issues/6
[wg-cargo-std-aware#7]: https://github.com/rust-lang/wg-cargo-std-aware/issues/7
[wg-cargo-std-aware#8]: https://github.com/rust-lang/wg-cargo-std-aware/issues/8
[wg-cargo-std-aware#10]: https://github.com/rust-lang/wg-cargo-std-aware/issues/10
[wg-cargo-std-aware#11]: https://github.com/rust-lang/wg-cargo-std-aware/issues/11
[wg-cargo-std-aware#12]: https://github.com/rust-lang/wg-cargo-std-aware/issues/12
[wg-cargo-std-aware#13]: https://github.com/rust-lang/wg-cargo-std-aware/issues/13
[wg-cargo-std-aware#14-review]: https://github.com/rust-lang/wg-cargo-std-aware/issues/14#issuecomment-2315878717
[wg-cargo-std-aware#14]: https://github.com/rust-lang/wg-cargo-std-aware/issues/14
[wg-cargo-std-aware#15]: https://github.com/rust-lang/wg-cargo-std-aware/issues/15
[wg-cargo-std-aware#16]: https://github.com/rust-lang/wg-cargo-std-aware/issues/16
[wg-cargo-std-aware#17]: https://github.com/rust-lang/wg-cargo-std-aware/issues/17
[wg-cargo-std-aware#19]: https://github.com/rust-lang/wg-cargo-std-aware/issues/19
[wg-cargo-std-aware#20]: https://github.com/rust-lang/wg-cargo-std-aware/issues/20
[wg-cargo-std-aware#21]: https://github.com/rust-lang/wg-cargo-std-aware/issues/21
[wg-cargo-std-aware#22]: https://github.com/rust-lang/wg-cargo-std-aware/issues/22
[wg-cargo-std-aware#23]: https://github.com/rust-lang/wg-cargo-std-aware/issues/23
[wg-cargo-std-aware#24]: https://github.com/rust-lang/wg-cargo-std-aware/issues/24
[wg-cargo-std-aware#25]: https://github.com/rust-lang/wg-cargo-std-aware/issues/25
[wg-cargo-std-aware#26]: https://github.com/rust-lang/wg-cargo-std-aware/issues/26
[wg-cargo-std-aware#29]: https://github.com/rust-lang/wg-cargo-std-aware/issues/29
[wg-cargo-std-aware#30]: https://github.com/rust-lang/wg-cargo-std-aware/issues/30
[wg-cargo-std-aware#36]: https://github.com/rust-lang/wg-cargo-std-aware/issues/36
[wg-cargo-std-aware#38]: https://github.com/rust-lang/wg-cargo-std-aware/issues/38
[wg-cargo-std-aware#39]: https://github.com/rust-lang/wg-cargo-std-aware/issues/39
[wg-cargo-std-aware#40]: https://github.com/rust-lang/wg-cargo-std-aware/issues/40
[wg-cargo-std-aware#43]: https://github.com/rust-lang/wg-cargo-std-aware/issues/43
[wg-cargo-std-aware#45]: https://github.com/rust-lang/wg-cargo-std-aware/issues/45
[wg-cargo-std-aware#46]: https://github.com/rust-lang/wg-cargo-std-aware/issues/46
[wg-cargo-std-aware#47]: https://github.com/rust-lang/wg-cargo-std-aware/issues/47
[wg-cargo-std-aware#50]: https://github.com/rust-lang/wg-cargo-std-aware/issues/50
[wg-cargo-std-aware#51]: https://github.com/rust-lang/wg-cargo-std-aware/issues/51
[wg-cargo-std-aware#64]: https://github.com/rust-lang/wg-cargo-std-aware/issues/64
[wg-cargo-std-aware#68]: https://github.com/rust-lang/wg-cargo-std-aware/issues/68
[wg-cargo-std-aware#69]: https://github.com/rust-lang/wg-cargo-std-aware/issues/69
[wg-cargo-std-aware#85]: https://github.com/rust-lang/wg-cargo-std-aware/issues/85
[wg-cargo-std-aware#86]: https://github.com/rust-lang/wg-cargo-std-aware/issues/86
[wg-cargo-std-aware#87]: https://github.com/rust-lang/wg-cargo-std-aware/issues/87
[wg-cargo-std-aware#88]: https://github.com/rust-lang/wg-cargo-std-aware/issues/88
[wg-cargo-std-aware#90]: https://github.com/rust-lang/wg-cargo-std-aware/issues/90
[wg-cargo-std-aware#92]: https://github.com/rust-lang/wg-cargo-std-aware/issues/92
