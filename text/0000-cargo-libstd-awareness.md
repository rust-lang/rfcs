- Feature Name: cargo_stdlib_awareness
- Start Date: 2015-05-26
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)


# Summary

Currently, Cargo doesn't know what parts of the standard library packages depend on.
By giving it this knowledge, we can make cross compilation and exotic platform development easier, simplify rustbuild, and allow anyone to easily specify bounds on the standard library for their packages.
This will allow building parts of the standard library from source, but in order to not disrupt existing workflows, the binaries that come with rustc will still be used by default.


# Motivation

First, some background.
`rustc` can be given crates and their location with `--extern <name>=<path>`.
When finding a library, `rustc` first sees if its location has been specified with `--extern`, then looks in any directories specified with `-L`, and finally looks in the "sysroot" [specifically `<sysroot>/lib/rustlib/<target>/lib`].
Cargo passes in all dependencies it builds with `--extern`.
However Cargo does not know about the standard library, so builds of it are taken from the sysroot.

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
Keeping multiple in sync is nuisance, and this gets us one step closer to a single `cargo build` building rustc.

The use-cases so far mainly benefit niche corners of the Rust community, but the last should be useful for just about everyone.
Now that multiple versions of Rust have been released, it can be useful to specify the minimum version.
If and when Rust 2, a version with breaking changes, comes out, this will be all the more important.
We don't yet have a plan yet to track the language itself, but by tracking standard library dependencies we make it trivial to specify version requirements like any other package.


# Detailed design

The subsections with "interface" in their title form the normative part of this RFC.
The rest of this section just illustrate hows they would likely be implemented.

## `Cargo.toml` Interface

First and foremost, one will now be able to depend on standard library crates by version, e.g. `std = "1.10"`.
This will work just as if `std` was on crates.io---features and other modifiers are supported.

For backwards compatibility, Cargo must inject such standard library dependencies for existing packages.
These injected standard library dependencies are called "implicit dependencies" because the user does not specify them explicitly.
Exactly which dependencies will be injected is unresolved, but a requirement at least as strong as `std = "^1.0"` as a primary, build, and development dependency is assured.
We also have to account for `core` somehow, as it is now stable so packages using it implicitly too cannot be broken.
Other dependencies of `std` besides core we don't need to worry about, because they are only transitive dependencies through `std`, not direct dependencies.
`test`, the built-in testing framework's runtime, will also be an implicit development dependency.

Now, not all crates depend on `std`, or whatever the implicit dependencies are decided to be, so there must be a way to opt out.
For this, we introduce a new `implicit-dependencies` key.
It is defined by default as:
```toml
implicit-dependencies = ["primary", "build", "dev"]
```
This indicates each of `dependencies`, `build-dependencies`, and `dev-dependencies` maps (respectively) is augmented with implicit elements.
A manual definition may be that or almost any subset, in which case only the included dependency maps are augmented.

Finally, if an (explicit) dependency conflicts with one of the implicit defaults, that category of implicit dependency will be skipped.
For example, if a crate explicit depends on `std` as a build-dependency, neither `std` nor any other implicit build dependency will be injected.

## Cargo command-line interface

A flag will be added
```
--resolve-compiler-specific-deps=<true|false>
```
If this flag is false, Cargo will only use crates.io (or whatever registries Cargo.toml specifies to use by default).
If this flag is true, Cargo, in the case the package is not found in one of the user specified registries, fallback first on a compiler-provided package registry, and if that fails, allow the use of sysroot binaries to satisfy deps.
It is assumed that the version of all sysroot rlibs is the same as the version of Rust which the compiler implements.
For backwards compatibility, the default is `--resolve-compiler-specific=true`--i.e. this fallbacks will be used.

## Compiler command-line interface

As will be explained, with these changes Cargo will always use `--extern` to pass dependencies, so rustc's sysroot lookup can and should be bypassed.
This will prevent users using the sysroot from depending on more crates than their cargo file declares (implicitly or explicitly)..
A `--no-resolve-sysroot` flag will be added for this purpose.
Compilers that don't have sysroot binaries should except this flag and ignore it.
Note that even if this flag is passed, rustc should still pass the sysroot to the linker for the sake of `compiler-rt`.
A `--no-link-sysroot` flag will be added to prevent that.

Additionally, compilers besides rustc may have version numbers distinct from the version of Rust they implement.
For this purpose, the verbose version output (`$COMPILER -vV` should contain an additional line:
```
language-version: $version
```
For now, `$version` should be a version, not version requirement, and the patch number must be zero as patch numbers don't make sense for interfaces.
[This may be relaxed in the future for compilers which implement multiple versions.]

## Compiler source packaging

While the standard library *interface* defined with each rustc version, the implementation, by virtue of using unstable features, is compiler-specific.
This makes the standard library unfit for crates.io.
(Additionally, the issue of dealing with nightly also makes crates.io hard to use, but that is a less clear-cut obstacle.)

Cargo will soon gain the ability to create and chain custom registries, as described in
https://github.com/rust-lang/cargo/pull/2361 .
Compiler's should package the source of their implementation of the standard library as a registry, which can be distributed with the compiler.
In practice, rustc will optionally contain the source in its sysroot.
Rustup may be able to put it there if the default download does not contain it already.

## Cargo implementation

The one thing this RFC requires of the upcoming registry implementation is the ability to chain registries providing defaults and fallbacks when a registry is not manually specified.
By default, crates.io is used, but somebody may wish to prohibit that, or have Cargo first check a repository of local forks before falling back on crates.io.
This overlaps with `[[replace]]` a bit awkwardly, but is generally useful when the exact set of overrides is subject to change out of band with the current project:
the project's `Cargo.toml` would specify something like `default-registry = [ "local-forks", "crates-io" ]`, along with the definition of "local-forks".

This mechanism is used here used so standard library crates not on crates.io (or whatever the users' `default-registry` fallback chains happens to be) are instead provided by the compiler.
We need to avoid forcing users' packages to enumerate the contents of the compiler registry because exactly what packages the compiler provides changes from one version of Rust to the next.
For example, if a portable `collections` is written for example, Cargo should use that rather than the compiler's own package.
This allows transparently making the standard library less-compiler specific.

Whenever Cargo encounters a `Cargo.toml`, the first thing it always does is inject any applicable implicit deps.
The idea is that by doing this so early on, most of Cargo can stay the same in only knowing or caring about explicit deps, simplifying both this RFC and its implementation.
This process is completely defined by the rules described in the first subsection, so there is really nothing to elaborate upon here. the remainder subsection will focus on the meaning of the `--resolve-compiler-specific=` flag.

If that flag is false, then everything proceeds as normally.
Dependencies specified by version only are always resolved from crates.io.

If that flag is true, Cargo behaves as if two registries were appended (with lowest priority), first the compiler source registry and then the compiler binary mock registry.
The first, with second-lowest priority, the compiler source registry, is located at `${$CARGO_RUSTC --print sysroot}/src`.
The second, with absolute lowest priority, the compiler binary mock registry, is generated by examining the contents of the sysroot.
Any binary in there will be added to the mock registry, with a version
taken either from the `language-version` described above, or the compiler version if that key is not present (as it would be with existing rustc releases).
Cargo likewise will have to be conservative with other metadata, e.g. both aborting if any feature is requested of a dep that is resolved to this mock registry, and also aborting if `default-features = false` is specified in such a dep.
Packages in the mock registry are not built, and when they serve as (transitive) dependencies, Cargo passes them in with `--extern` and their sysroot location.
This is different from other deps, whose binaries are placed in Cargo's output directory, and sysroot deps today, where `--extern` isn't used as all.

Since `--extern` is used even in the sysroot binary case, rustc can and will pass `--no-resolve-sysroot` to rustc in all cases.

## Rustbuild improvements

When building rust, binaries or source associated with the previous stage are never used, so rustbuild will always pass `--resolve-compiler-specific=false`.

In order to allow building packages that aren't specifically tailored for building rust itself (e.g. they might come from crates.io), Cargo needs to be taught to resolve standard library packages with the current workspace. Either `[[replace]]` can be used for this, or perhaps the members of the workspace would themselves act as a registry.

All binaries for a specific phase can be built with a single `cargo build` (barring special requirements for individual libraries).
Rather than have a multitude of build artifact directories per stage, only one is needed.
After the last compiler is build, an additional mini-stage of building just the standard library could be performed, but distributions wishing to build all deps from source in a standardized fashion (e.g. NixOS) would probably forgo this.


# Drawbacks

 - The mock registry for sysroot binaries is a disgusting hack.

 - Only some crates in the rust repo (at least `core`, `alloc` and `collections`) can properly be built just based upon their `Cargo.toml`.
   However it's precisely only these "foundational" crates that will be of interest to freestanding developers.
   Hosted developers can likely get pre-built binaries for the platform they need with `rustup`, just as they do today.

 - No means of compiling `compiler-rt` is proposed.
   But just as freestanding developers need to provide `rlibc` or similar to successfully link, I think that for the time-being they deal with this themselves.
   This is no step backwards.

 - Compilers could provided crates in their sysroot that don't match the Rust specification, and Cargo would be none the wiser.
   [Technically, this problem already exists with falling back on the sysroot binaries, but users will probably expect better when they can specify standard library dependencies explicitly.]
   Since the *interface* of the stdlib is specified, it would be neat if we could but a big crate type/interface on crates.io, which compiler implementations would need to match.


# Alternatives

 - Instead of copying binaries from the sysroot, we could just leave rustc to find them.
   But then a simple `--no-resolve-sysroot` would not work, and the logic for passing `--extern` would need be more complicated.

 - Previous versions of this RFC were a simpler but more brittle.
   Please refer to the git history to see them.

 - Should `--no-resolve-sysroot` influence linking after all, and `core` have a dep on some `compiler-rt-sys` crate?
   This dependency could be a default feature.

# Unresolved questions

 - Users of the stable compiler should be able to build the stdlib from source, since it is trusted, but cannot because it uses unstable features.
   Some notion of a trusted package/registry or way to route the secret bootstrap key would be required to fix this.

 - It is unclear how `core` should be an implicit dependency.
   `core = "^1.0"` might work but is a little weird as that core 1.0 is not stabilized.
   This relies on users to not `extern crate core;` if they don't use it, to get around the stability warning on older versions of rust, which is rather sketchy.

 - It is unclear what should go in the lockfile when building with sysroot binaries.

 - Whether to add the `--no-resolve-sysroot` flag to rustc, as described above.

 - Should `cargo new` specify `std`, or any other stdlib crates explicitly by default?
   I'd hope so!

 - Should one be able to opt-out of implicit build and development dependencies?
   I'd like to create a new crate containing testing annotations as compiler plugins, but this entails creating a new sort of test-only plugin dependency (combination of development and build).
   Additionally, Currently, it makes sense to always make `std` available for `build.rs` since it must exist for the compiler.
   But if platform-specific parts of the `std` are exposed only with features or "scenarios" (a newly-proposed mechanism specifically for handling environment differences), then we loose an opportunity to be able to express mandatory cross-compiling.
   Finally, in the far future it may be possible to build rustc on platforms where all of `std` isn't available, invalidating the reasoning that `std` is never unavailable as a build dependency.

 - It is somewhat unclear how Cargo should deal with architecture-specific configuration that is not captured in the target spec nor Cargo feature flags (like CPU features).
   [RFC #1645](https://github.com/rust-lang/rfcs/pull/1645) proposes just adding some such configuration to the target triple, whereas https://internals.rust-lang.org/t/pre-rfc-a-vision-for-platform-architecture-configuration-specific-apis/3502/26 proposes a new "scenarios" interfaces.
   When building from source, this question is orthogonal to this RFC because it just reuses Cargo's existing methods of keeping binaries for different configurations separate.
   When building with sysroot binaries, however, this does matter because cargo needs to deduce or assume exactly what configuration beyond the target triple applies.
