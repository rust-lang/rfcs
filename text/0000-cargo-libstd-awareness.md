- Feature Name: cargo_stdlib_awareness
- Start Date: 2015-05-26
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)


# Summary

Currently, Cargo doesn't know what parts of the standard library packages depend on.
By giving it this knowledge, we can make cross compilation and exotic platform development easier, simplify rustbuild, and allow anyone to easily specify semver requirements on the standard library for their packages.
This will allow building parts of the standard library from source, but in order to not disrupt existing workflows, the binaries that come with rustc will still be used by default.


# Motivation

First, some background.
Rustc needs to load dependency crates in order to work.
Multiple flags exist to instruct rustc on how to find these crates.
`--extern <name>=<path>` tells rustc to find the crate with the given name at the given path.
This has the highest priority, overriding locations specified or inferred via other means.
`-L <kind>=<dir>` has the second highest priority, telling rustc to look for matching crates in the given directory.
The `<kind>=` part is optionally, but one variant that is crucial for Cargo's purposes is `-L dependeny=<dir>`.
The `dependeny=` part tells rustc to only look in the directory when resolving transitives deps ("deps of deps"), as opposed to `extern crate`s in the current crate.
As a last resort, rustc will look within the sysroot (specifically `<sysroot>/lib/rustlib/<target-triple>/lib`). The sysroot is a hard-coded location relative to rustc, but can also be overridden with `--sysroot=<path>`.

Cargo passes immediate dependencies to rustc with `--extern <name>=<path-to-rlib>`, and transitive dependencies with `-L dependency=<cargo-artifacts-dir>`.
However Cargo does not know about the standard library, so builds of it are taken from the sysroot, and any crate can `extern crate` any rlib that happens to be there.

For cross-compiling, one can often download standard library binaries with rustup.
This is convenient, but one cannot expect pre-built binaries for all platforms.
In particular, embedded systems often have detailed configurations to convey as much information as possible about the hardware to the compiler.
Furthermore, not all of the stdlib is available on every platform---there is an RFC in the works to pre-build a smaller set of crates for [Cortex-M microcontrollers](https://github.com/rust-lang/rfcs/pull/1645).
It would be nice to know if the available subset is adequate before attempting a build.
We can only do that if all packages have explicit standard library deps to cross-reference with platform requirements of each standard library crate.

Now rustup could be augmented to build stdlib binaries in addition to downloading them, but we have extra configuration options for the standard library such as [panic strategies](https://github.com/rust-lang/rfcs/blob/master/text/1513-less-unwinding.md) along with plans to add more such as [`core` without floats](https://github.com/rust-lang/rfcs/issues/1364).
One could also cobble together a way to to tell rustup such configuration options, but we'd like to make sure that all dependencies agree with the plan before trying to execute it.
For panicking, packages can already set a `profile.dev.panic` option to require a specific strategy, and for float support we should add a (default opt-in) feature to core.
Then, if packages can explicitly depend on core they can also specify whether the float feature is needed.
Cargo would be able to infer both of these options by inspecting all crates in the dependency graph.

Rustbuild must currently perform multiple `cargo builds`, the first to build the standard library and the rest to build things which depend on the standard library.
If rustc, rustfmt, etc, and their deps (some of which come from crates.io, and thus aren't specially tailored for building rust) declare deps on std, rustbuild wouldn't need multiple lockfiles.
Keeping multiple lockfiles in sync is a nuisance, and this gets us one step closer to a single `cargo build` building rustc.

The use-cases so far mainly benefit niche corners of the Rust community, but the last should be useful for just about everyone.
Now that multiple versions of Rust have been released, it can be useful to specify the minimum version.
If and when Rust 2, a version with breaking changes, comes out, this will be all the more important.
We don't yet have a plan yet to track which version of Rust is used in the current crate (in order to opt in to the use of a hitherto unstable feature).
However, because the versions of standard library crate are required to be the same as the version of the language supported by the compiler, specifying the semver requirements of a crate in the sysroot effectively specifies the semver requirements of the language itself.


# Detailed design

## Standard library dependencies

First and foremost, one will now be able to explicitly depend on standard library crates, e.g. with `std = { version = "1.10", stdlib = true }`.
From the users's perspective, `stdlib = true` simply indicates that the depended-on crate is from the standard library.
The version for stdlib crates comes from the version of Rust their interfaces are defined in.
A version requirement must be specified.
The full breadth of options available with our existing dependencies, e.g. features and overrides, will be supported.

For the initial roll out of the feature, only normal dependencies, not build or dev dependencies, will be allowed to include explicit stdlib dependencies.

## Implicit dependencies

For backwards compatibility, Cargo must inject such standard library dependencies for existing packages.
These injected standard library dependencies are called "implicit dependencies" because the user does not specify them explicitly.
We have an obligation to not break packages depending only on stable interfaces, so the implicit dependencies will include both `std` and `core`:
```toml
[dependencies]
core = { version = "^1.0", stdlib = true }
std  = { version = "^1.0", stdlib = true }

[dev-dependencies]
core = { version = "^1.0", stdlib = true }
std  = { version = "^1.0", stdlib = true }
test = { version = "^1.0", stdlib = true }

[build-dependencies]
core = { version = "^1.0", stdlib = true }
std  = { version = "^1.0", stdlib = true }
```
The version requirement for `core` of `^1.0` may seem odd because core was not stable in Rust 1.0, but anything else would either break newer packages using core, or prevent older packages from working on versions of Rust predating core's stabilization.
Remember that rustc only complains if an unstable crate is actually imported, so the Cargo dependency on its own is harmless.

`test` is a similar scenario.
While importing it explicitly remains unstable, it's currently injected and thus needs to be built.
Other dependencies of `std` besides core we don't need to worry about, because they are only transitive dependencies through `std`, not direct dependencies.

Now, not all crates depend on these crates, so there must be a way to opt out.
The primary way is just to create a conflict.
If an (explicit) dependency has the same name as one of the implicit defaults, implicit dependencies of the same sort will be skipped.
For example, if a crate explicit depends on `std` as a regular dependency, neither `std` nor any other implicit regular dependency will be injected.
Since currently regular dependencies can be included stdlib dependencies, only regular dependencies can be opted out of.
This means we are free to change the implicit dev and build dependencies without breaking anything.

Opting out via conflict as described above is adequate for almost all cases.
The one exception is `core` itself, which must of course not depend on `core` or `std` implicitly or explicitly---or anything else for that matter.
For it, a key, `implicit-dependencies = <true|false>`, will be introduced.
Because it doesn't generalize if we make the implicit build or dev stdlib deps optional, this key will be permanently unstable.

## Compiler language version

Compilers besides rustc may have version numbers distinct from the version of Rust they implement.
For this purpose, the verbose version output (`$CARGO_RUSTC -vV`) should contain an additional line:
```
language-version: <version>
```
For now, `<version>` should be a version, not a version requirement, and the patch number must be zero as patch numbers don't make sense for interfaces.
[This may be relaxed in the future for compilers which implement multiple versions.]

## Compiler Source

While the standard library *interface* is defined with each rustc version, the implementation of many crates, by virtue of using unstable features, is compiler-specific.
This makes the standard library unfit for crates.io.
(Additionally, the issue of dealing with nightly also makes crates.io hard to use, but that is a less clear-cut obstacle.)

To get around this, Cargo will give compilers the option of distributing the source of their implementation of the standard library in a location Cargo knows of.
Conveniently, Cargo has a "source" abstraction for providers of packages.
Examples of this are file-system paths, git repositories, and the upcoming registries.
To implement this, Cargo will gain knowledge of of a new source, the "compiler source".
The compiler source, if it is present, will be located in the sysroot in `<sysroot>/lib/rustlib/src`.
The exact format this takes will be determined during implementation and added back to this RFC before stabilization, but that of a "local registry" is likely, now that
https://github.com/rust-lang/cargo/pull/2857 has landed.
Compilers should include the source of each crate of their implementation of the standard library in side.

It is presumed that, Rustup may be able to put it there if the default download does not contain it already.

## Cargo Pipeline

Whenever Cargo encounters a `Cargo.toml`, the first thing it always does is inject any applicable implicit deps.
The idea is that by doing this so early on, most of Cargo can stay the same in only knowing or caring about explicit deps, simplifying both this RFC and its implementation.
This process is completely defined by the rules described in the first subsection, so there is really nothing to elaborate upon here.

Just as `git = ...`, and `path = ...` are parsed into a "source id", so `stdlib = true` will into a new "stdlib source id" too.
But instead of mapping to a specific source, this source id will map either to the "compiler source" as described above, or the "sysroot binary mock source", as described below.

The "sysroot binary mock source" is generated by examining the contents of the sysroot.
Just as today, binaries are located in `<sysroot>/lib/rustlib/<target-triple>/lib`.
Any binary in there will be added to the mock source, with a version
taken either from the `language-version` key described above, or the compiler version if that key is not present (as it would be with existing rustc releases).

If the compiler source exists, that is used to resolve `stdlib = true` deps, and the sysroot binary mock source need not even be built.
If the compile source is absent, then the binary mock source is used.
Note that this prioritization doesn't depend on the outgoing dependencies trying to be resolved.
Once the source backing stdlib deps is picked, it is the only one used even if the other source also exists and contains the missing package---sticking arbitrarily named rlibs in the sysroot will not effect Cargo when everything is being built from source.

When the build plan just involves the compiler source and/or existing types of sources, it can be executed just like today.
The awkward scenario is when packages from the sysroot binary mock source need to be used in the build plan.
Because Cargo doesn't know much about the sysroot binaries, it must be very conservative when deciding whether or not they can be used.
For example, Cargo may assume they are built with only default features enabled but it can't know what those are.
If features are explicitly requested, or the default features are disabled (by all dependent packages) then the binaries are ineligible for the build plan under construction.
Cargo likewise will have to be conservative inferring any other package metadata it may use.

Packages in the binary mock source are not built by Cargo, since they are prebuilt, and when they serve as immediate dependencies, Cargo passes them in with `--extern` and their sysroot location.
This is different from other deps, whose binaries are placed in Cargo's output directory, and sysroot deps today, where `--extern` isn't used as all.
Also whenever they are in any way part of the build plan, Cargo also must pass `-L dependency=<sysroot>/lib/rustlib/<target-triple>/lib` so rustc can find transitive deps here. This is needed both because the binary mock source crates may in fact be transitive deps of the crates built from source, and also because they *themselves* may also have arbitrary binary mock source deps.

Because of this use of `--extern` and `-L` with the binary mock source, rustc when invoked with Cargo should never need fallback looking for binaries in the sysroot.
To prevent it from doing so with broken packages, Cargo will also pass rustc `--sysroot=` (i.e. the empty path) to prevent it from doing so.
[Once [Rust PR #35021](https://github.com/rust-lang/rust/pull/35021/files) lands in some form, `compiler-rt` will be a Cargoized dependency so the sysroot won't be needed for linking either.]

## Rustbuild improvements

As advertised in the motivation section, with this RFC, rustbuild can use a single workspace to build the standard library and all executables.

One complication with the RFC is that that no sysroot binaries or source associated with the bootstrap compiler (or previous stage) are ever used; one needs to bypass the compiler source and sysroot binary mock source.
To accomplish this, rustbuild's workspace will need to use `[replace]` to redirect all stdlib deps to use the workspace's packages.

All binaries for a specific phase can be built with a single `cargo build` (barring special requirements for individual libraries).
Rather than have a multitude of build artifact directories per stage, only one is needed.
After the last compiler is build, an additional mini-stage of building just the standard library could be performed, but distributions wishing to build all deps from source in a standardized fashion (e.g. probably NixOS) would forgo this.

## Forward Compatibility

The custom registries PR https://github.com/rust-lang/cargo/pull/2857 starts with just mirroring existing registries.
As followup work, it expected that packages (probably just the workspace root, definitely not non-packages like cargo config) will be able to specify the "default" source, i.e. the one used when none is specified (today this is always crates.io).
Similarly, one could specify a "stdlib" source, to be used for `stdlib = true` deps instead of the compiler source or sysroot binary mock source.
This would simplify rustbuild as it could use that once instead of `[replace]` for each package.
This doesn't require any planning from this RFC.

More importantly, it would be nice to move stdlib crates that don't use unstable features to crates.io.
`collections` and `test` almost don't use any unstable and are thus good candidates for this.
With something like what is described in the first paragraph, it could be possible for individual packages to instruct Cargo to first check crates.io, and then the compiler source, for stdlib crates.
But this shifts the burden to individual packages, and means we'd still need to vendor source of any crate moved to crates.io in the compiler source for packages that didn't make the switch.

More interesting would be to change Cargo's *default* behavior to check both the compiler-specific sources (compiler source and sysroot binary mock source) and crates.io.
This would allow standard crates to seamlessly migrate to crates.io without extra work per package.
This could be either be done where crates.io overrides the compiler-specific sources, or the compiler specific sources override crates.io.
We don't want to commit to either variant in this RFC, however, so we instead want to keep all 3 options open (no fallback, crates.io over compiler-specific, compiler-specific over crates.io).
To achieve this, we want to keep the sysroot/compiler source and crates.io disjoint: no package should be contained in both sources.
That way unioning them together with either priority (the fallback scheme effectively crates a union source) has the same effect.

The easiest way to achieve this is to make sure that standard library crates use names reserved on crates.io.
We don't want to bake crates.io policy into Cargo however, so instead of absolutely prohibiting stdlib deps with non-reserved names, crates.io will just lint packages being uploaded.
Also, care will be taken so that any stdlib crate that is stabilized must use a reserved name or already be published on crates.io.
That still doesn't protect unpublished packages using unstable stdlib crates without reserved names from breakage, but due to their use of unstable interfaces we have no obligation to keep them working.
Also, once we have an option to explicitly provide the source for stdlib deps, they can force the behavior they want.
This seems good enough.


# Drawbacks

 - The sysroot binary mock source is a complicated special case whose implementation will probably span many parts of Cargo.
   In the near future, it is unlikely to be generalized into something more elegant.

 - Even with this RFC and a nightly compiler, a single `cargo build` is incapable of building the entire standard library due to external dependencies.
   But I believe we will eventually reach that goal, and furthermore this RFC will help us reach it.

 - Compilers could provide crates in their sysroot that don't match the Rust specification, and Cargo would be none the wiser.
   (Technically, this problem already exists with falling back on the sysroot binaries, but users will probably expect better when they can specify standard library dependencies explicitly.)
   Since the *interface* of the stdlib is specified, it would be neat if we could put a big crate type/interface on crates.io, which compiler implementations would need to match.
   [That is, the interface of the stable crates.
   Unstable crates behind the std facade are a compiler-specific implementation detail, and thus it would be counter-productive, even to likewise constrain their interfaces.]

 - The name "compiler source" is unfortunate because it sounds like the source of the compiler itself.
   But perhaps the best solution is to rename Cargo's "source" abstraction (and the traits that go with it).


# Alternatives

 - Previous versions of this RFC were simpler but more brittle.
   Please refer to the git history to see them.

 - Should the "stdlib" virtual source instead be called "sysroot" (e.g. `core =  { sysroot = true, .. }`)?
   This emphasizes how those dependencies are resolved as opposed to what they are for.

 - If a way to specify the language version like #1709 or #1707 is added, the version of stdlib dependencies could be pulled from that.

 - This has been deemed sufficiently complicated to warrant the introduction of unstable features to Cargo.


# Unresolved questions

 - Users of the stable compiler should be able to build the stdlib from source, since it is trusted, but cannot because it uses unstable features.
   Some notion of a trusted package/registry or way to route the secret bootstrap key would be required to fix this.

 - It is unclear what should go in the lockfile when building with sysroot binaries.

 - Should `cargo new` specify `std`, or any other stdlib crates explicitly by default?
   I'd hope so!

 - Should one be able to opt-out of implicit build and development dependencies?
   Currently, it makes sense to always make `std` available for `build.rs` since it must exist for the compiler.
   But if platform-specific parts of the `std` are exposed only with features or "scenarios" (a newly-proposed mechanism specifically for handling environment differences), then we lose an opportunity to be able to express mandatory cross-compiling.
   Finally, in the far future it may be possible to build rustc on platforms where all of `std` isn't available, invalidating the reasoning that `std` is never unavailable as a build dependency.

 - It is somewhat unclear how Cargo should deal with architecture-specific configuration that is not captured in the target spec nor Cargo feature flags (like CPU features).
   [RFC #1645](https://github.com/rust-lang/rfcs/pull/1645) proposes just adding some such configuration to the target triple, whereas https://internals.rust-lang.org/t/pre-rfc-a-vision-for-platform-architecture-configuration-specific-apis/3502/26 proposes a new "scenarios" interfaces.
   When building from source, this question is orthogonal to this RFC because it just reuses Cargo's existing methods of keeping binaries for different configurations separate.
   When building with sysroot binaries, however, this does matter because cargo needs to deduce or assume exactly what configuration beyond the target triple applies.
