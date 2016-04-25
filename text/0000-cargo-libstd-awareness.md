- Feature Name: cargo_stdlib_awareness
- Start Date: 2015-05-26
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)


# Summary

Currently, Cargo doesn't know what parts of the standard library packages depend on. This makes
Cargo difficult to use when both the standard library (or a portion of it) and some consumer of it
need to be built together. Examples of this need are freestanding development, where the exact
compilation target is usually custom-tailed to the specific project at hand, and working on rustc or
the standard library itself.

This new interfaces are simple, as is the the proposed implementation. However, the proposed
implementation also comes with some limitations, and past proposals of the RFC have been deemed
under-specified. I therefore suggest that this RFC be accepted an on experimental basis as an
unstable feature.


# Motivation

First, some background. `rustc` can be given crates and their location with `--extern
<name>=<path>`. When finding a library, `rustc` first sees if its location has been specified with
`--extern`, then looks in any directories specified with `-L`, and finally looks in the "sysroot"
[specifically `<sysroot>/lib/<target>`]. Cargo passes in all dependencies it builds with `--extern`.
However Cargo does not know about the standard library, so builds of it are taken from the sysroot.

## Short-Term Needs

Freestanding projects need to cross-compile the standard library, as it is infeasible for anyone to
provide binaries for all possible target triples. "Vendoring" in `rustc` and cross-compiling with
make is both clunky, and unsuitable for libraries, as then one ends up with a separate copy of the
standard library for each library used. Since freestanding rust needs unstable features (and will
continue to need them for the foreseeable future), the most elegant thing to do is just get Cargo to
cross-compile the standard library. This is suitable for libraries and binaries alike, and most
easily ensures the library and it's consumers will be built to the exact same target
specification. For examples of how this is rigged up, see my simple
https://github.com/RustOS-Fork-Holding-Ground/rust/tree/rustos/cargo, and the more flexible
https://github.com/hackndev/rust-libcore.

The problem with this is that each and every library that is used must be forked and made to
explicitly depend on the standard library crates. This both impedes writing libraries for hosted and
freestanding use, but also for different freestanding projects, as there is currently no official
Cargoization of the standard library. We want to be able to somehow "inject" our Cargoized standard
library as an extra dependency to a library written without this injection in mind.

Already, we have a means for overriding dependencies with Cargo, `.cargo/config` files. Suppose that
one could specify an override for core, alloc, std, etc, and then all packages would be built with
that override as dependency instead of the binary that came with `rustc`. This precisely solves the
problem of using libraries in both contexts.

There is one slight complication: how do we know which libraries would be linked by default?
Importantly, the crates behind the facade are not yet stabilized. Cargo versions are not currently
tied to compiler versions, so Cargo cannot be hard-coded to know the exact set of crate names. One
solution is to just wait and prioritize stabilizing at least the names of those crates. But that
would harm freestanding development in other ways. To better support the menagerie of platforms, it
would be wise to break out the implementation of std further so we can opt into e.g. networking but
not persistent storage, or vice-versa. However, since that use-case was brought up last year, the
number of crates behind the facade actually decreased---presumably to ease rapid development as 1.0
approached. Clearly then, to keep the door open for finer-grained crates behind the facade, we
should not do anything that would motivate stabilizing the names of those crates. Instead, we can
have Cargo look in the sysroot before calling `rustc`, and only accept overrides from
`.cargo/config` that either match a dependency, or a crate in the sysroot.

## Long-Term Needs

The [Rust in 2016](http://blog.rust-lang.org/2015/08/14/Next-year.html) blog post announces an
infrastructure goal of automatic cross-compilation. For the former, the plan is that Cargo will
download libstd builds for the destination platform. But note that this only works well for
platforms where libstd is buildable. For platforms where only some crates behind the facade are
buildable, the best we could do is download all the crates behind the facade that build on a
platform, and hope that is sufficient for anything the build at hand needs. For Cargo download a
bunch of binaries, and only thereafter err saying more are needed is annoying, and contrary to it's
normal behavior of determining whether all (transitive) dependencies can be built before attempting
the build itself.

Also, https://github.com/rust-lang/rust/pull/27003#issuecomment-121677828 states that we "probably
want to eventually move in the direction of a Cargo-based build system in the future". If/when do we
do, it would be nice to ensure that, when building `rustc` or the standard library, we don't
actually link anything in the sysroot by mistake. Granted, this sanity check is not essential for a
Cargo-based build system.

In the even longer term, we can imagine these two goals converging. Instead of merely offering a
downloaded pre-compiled standard library, we allow library maintainers to host their own binaries,
and downstream consumers to opt into the binary caches they trust. Mozilla, with the cross-compiled
libstd builds, would merely be another user of that system. Likewise, my short-term hack of using
Cargo overrides is no longer necessary. For platforms where binaries for the standard library is not
available, Cargo would just cross-compile it using the official Cargo-based build system.
Conveniently, Packages designed with the short-term hack in mind would not need to be republished.
Simply removing the suggestion of using Cargo overrides from their READMEs would be sufficient.

## Possible solutions

A first attempt at a solution would be to allow packages to whitelist what crates they can link from
the sysroot. The standard library would have an empty whitelist, while libraries for embedded
systems would opt into just what they actually need. This solves the problem, but is somewhat
inelegant in that it exposes the existence of the sysroot in the `Cargo.toml` schema, and thus
commits us to having the sysroot indefinitely. I don't see any advantage to making that
commitment. Consider that the cross-compiled standard library is distributed using a service
available to all library developers, the proposal I make above. We certainly don't want to allow
arbitrary Cargo libraries to be installed in a way that code that doesn't declare a dependency, and
special-casing the std lib to be installed in the sysroot while other packages aren't negates the
elegance of Mozilla dog-food-ing such a service as just another user. Of course that is all
hypothetical, but I wanted provide a concrete example of how exposing the notion of the sysroot
today could bite us down the road.

The salient attribute of the crates in the sysroot is not that they are in the sysroot, but that
they are part of the standard library. For a second attempt, Instead of allowing packages to
whitelist which sysroot crates they can link to, we could allow them to whitelist which standard
library crates they can link to. For now, this would be implemented exactly the same way, but allows
us to step away from having a sysroot, or stop using it with Cargo.

Finally, note that this is orthogonal to Rust's `no_std`, especially since it's recent modification
in #1184. It may seem silly to explicitly `extern crate`-ing std, but implicitly depending on it in
the Cargo file, or implicitly `extern-crate` std, but explicitly whitelist depending on it. However
both of these are naturally well-defined from the definition of `no_std` and the contents of this
RFC.

For the record, I first raised this issue [here](https://github.com/rust-lang/cargo/issues/1096).


# Detailed design


## Interface

### Explicit Dependencies

First, we need to allow packages to opt out of implicitly depending on all standard library
crates. A new optional Boolean field is added called called `implicit-deps` to the `Cargo.toml`
schema. When true, the package will implicitly depend on all crates that are part of the standard
library.

Second, we need a way for packages that have opted-out of these implicit dependencies to explicitly
depend on a subset of standard library crates. For this we add a new "virtual version". Packages will
able to declare:
```toml
[dependencies]
std = "stdlib"
```
or
```toml
[dependencies.std]
version = "stdlib"
```
which will declare a dependency on a crate called `std` which must be part of the standard library.

When no explicit dependencies are specified, `implicit-deps` defaults to `true`. (This is necessary
for compatibility with existing packages.) When an explicit stdlib dependency is specified this
defaults to `false`. It possible to have no explicit stdlib dependencies specified, and set
`implicit-deps` to be `false` --- this would be used by `core` for example. However the reverse is
prohibited: it is not allowed to specify explicit stdlib dependencies yet also use implicit stdlib
dependencies.

### `[replace]` and `.cargo/config` Overrides.

In keeping with "stdlib" being a "virtual version", one can do
```
[replace]
"some-crate:std-lib" = ...
```
to replace a std-lib crate with one of ones choosing. This is useful when developing the standard
library and a consumer of it, like `rustc`, together.

`.cargo/config` overrides are likewise extended as one would expect. If the Cargo package at the given
path matches a stdlib crate, the override is used instead.


## Implementation

On to the gritty details! With this RFC, there are two ways to depend on std library
crates. Likewise, for the immediate future at least, we to support either building the needed parts
of the standard library from source (the main purpose of this rfc!) or using pre-built sysroot
binaries. This yields 4 combinations.

As a prerequisite, `rustc` will need one new flag `--sysroot-whitelist=[crates]`, a whitelist of
crates it is allowed to look for in the sysroot. Importantly this only restricts immediate `extern
crate`s in the current compilation unit; no restrictions are placed on opening transitive
dependencies.

### Explicit dependencies and freshly-built stdlib

Explicit stdlib dependencies, if not overridden, are simply desugared to git dependencies on
`http://github.com/rust-lang/rust` with the revision given by `rustc`. (On start-up, Cargo already
queries `rustc` for its verbose version, which includes the git revision.) Additionally for any
crate with `implicit-deps = true`, `--sysroot-whitelist=` (i.e. the empty whitelist) is passed to
`rustc` as sysroot binaries should never be used.

### Implicit dependencies and freshly-built stdlib

Exactly what parts of the standard library do we need to build? One can assume implicit deps on
`std` and `core`, as those are the only stable crates today. Keep in mind however that since Cargo
doesn't currently know what crates constitute the standard library for a given `rustc`, that there
is no simple way to correctly implement `.cargo/config` and `[replace]` overrides. Because this is
an experimental feature, I propose just punting and erring should overrides be given.

### Explicit dependencies and pre-built stdlib from sysroot

Build as today, but pass via `--sysroot-whitelist` the list of explicit stdlib dependencies whenever
`implicit-deps` is false. That way, we can be sure regardless of the built strategy that no
undeclared stdlib dependencies "leak in".

For overrides, one could filter the writelist of any crates that were overridden. On the other hand
this doesn't take into account other std-lib crates that depended on the overridden crates---they
two should be freshly built and not pulled from the sysroot so as to depend on the overridden
crates. It may be better to punt again then.

### Implicit dependencies and pre-built stdlib from sysroot

This is today, and should work as today. Again, punt on the newly-proposed types of overrides.


# Drawbacks

Notably, all of these relate to the suggested implementation, and not the interface that is the core
of this RFC. I remain optimistic that over time these implementation issues can be resolved without
changing the proposed interface. But my optimism is no more that---speculation.

 - The override limitations already specified within the implementation section.

 - Due to the way git dependencies work, if we naively desugar explicit stdlib dependencies to them
   as proposed, packages will be able to depend on *any* crate in the `rust` repo, including the
   various `rustc_*` crates there is no intention of keeping around.

 - Cargo doesn't currently have any unstable features, so giving them that requires work and
   planning.


# Alternatives

 - It is possible we can just use wildcard dependencies for standard library crates, and don't need
   to add a notion of a standard library dependency. Crucially, while there are many different
   versions of the standard library, there is only one that works with any given version of `rustc`.

   Specifically, we can have it so whenever Cargo comes across a wildcard crates.io dependency it
   can't resolve, assume it is a std lib dependency and either resolve it as such (desugar to git
   dependencies) or leave it for `rustc` to find in the sysroot according to the build type.

   I made this to be an alternative because it might be a bit too magical. Also, Semver won't be
   able to understand the release trains---pre-release/beta builds are allowed in its grammar but
   have different semantics.

 - Simply have `implicit-deps = false` make Cargo pass `--use-sysroot=false` to `rustc`, and don't
   have any way to explicitly depend on things in the sysroot.

   - This doesn't by-itself make a way for package to depend on only some of the crates behind the
     facade. That, in turn, means Cargo is little better at cross compiling those than before.

   - While unstable compiler users can just package the standard library and depend on it as a
     normal crate, it would be weird to have freestanding projects coalesce around some bootleg
     core on crates.io.

 - Make it so that packages with implicit dependencies only depend on std. This would be more
   elegant, but breaks packages (including ones using stable Rust) that just depend on crates.io.

 - Make it so all dependencies, even libstd, must be explicit. C.f. Cabal and base. Simple and
   elegant, but breaks all existing packages.

 - Don't do this, and be stuck with problems detailed in the motivation section.


# Unresolved questions

 - There are multiple lists of dependencies for different things (e.g. tests, build-time). How
   should `implicit-deps = false` affect them?

 - Just as this makes the standard library a real dependency, we can make `rustc` a real
   dev-dependency. The standard library can thus be built with Cargo by depending on the associated
   unstable compiler. Cargo would need to be taught an "x can build for y" relation for
   stable/unstable compiler compatibility however, rather than simply assuming all distinct
   compilers are mutually incompatible. This almost certainly is better addressed in a later RFC.
