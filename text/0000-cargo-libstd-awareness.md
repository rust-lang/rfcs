- Feature Name: cargo_stdlib_awareness
- Start Date: 2015-05-26
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Currently, Cargo doesn't know what parts of the standard library packages depend on. This makes
Cargo unsuitable for packages that are typically cross compiled and only use some of the crates
behind the facade--in other words, libraries intended for freestanding use. If/when the standard
library is Cargoized in the future, the proposed fixes will also allow projects to better take
advantage of that.

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

First, we need to allow packages to opt out of implicitly depending on all standard library
crates. A new optional Boolean field is added called called `implicit-deps` to the `Cargo.toml`
schema.`implicit-deps` is true by default to maintain compatibility with existing packages. When
true, the package will implicitly depend on all crates that are part of the standard library.

Second, we need a way for packages that have opted-out of these implicit dependencies to explicitly
depend on a subset of standard library crates. For this we add a new type of dependency, in addition
to the git, local path, and crates.io dependencies we support today. Packages will able to declare:
```toml
[dependencies.std]
stdlib = true
```
which will declare a dependency on a crate called `std` which must be part of the standard library.

It is an error to explicitly depend on a standard library crate when the entire standard library is
already depended-upon implicitly, as the explicit dependency is redundant. This can be relaxed in
the future if a need arises.

## Implementation

Here is how we can implement this interface until future infrastructure changes.

`rustc` will gain the ability to not use a sysroot, in which case only the information specified in
`--extern` and `-L` flags will be used to find dependencies. For backwards compatibility, a sysroot
will be used by default. To control this a `--use-sysroot=<true|false>` flag is added, with the
obvious meaning. It is an error to specify `--use-sysroot=false` and a custom sysroot with
`--sysroot=<some path>`.

The way in which Cargo builds packages will be modified as follows. When the standard library is
depended-on implicitly, everything happens as it does today. When the standard library isn't
depended the following two extra steps happen when Cargo is gathering all dependency metadata to see
if a build is possible.

 1. Cargo queries `rustc` for the default sysroot. The means in which it does this is not specified
 nor stabilized.

 2. Cargo computes gathers all explicit standard library dependencies of the current package and its
 dependencies, and collects their paths within the sysroot. If it cannot find a build in the sysroot
 for any gathered dependency, Cargo errs.

Finally, when Cargo invokes `rustc` to actually build something, it does so with
`--use-sysroot=false`, and passes in any explicitly-depended-on standard library crates (immediate,
not transitive dependencies) with `--extern`.

Remember that Cargo current aims to work with multiple versions of `rustc`, and also that the crates
behind the facade are not stabilized. This is why Cargo needs to both query `rustc` for the sysroot
location, and use the contents of the sysroot rather than some hard-coded list to decide whether a
standard library dependency with a given (crate) name is valid.


# Drawbacks

Adds a notion of standard library dependencies that may be superfluous---see first alternative.


# Alternatives

 - It is possible we can just use wildcard dependencies for standard library crates, and don't need
   to add a notion of a standard library dependency. Crucially, while there are many different
   versions of the standard library, there is only one that works with any given version of `rustc`.

   Specifically, we can have it so whenever Cargo comes across a wildcard crates.io dependency it
   can't resolve, fallback on looking in the sysroot. In the future, the standard library could
   actually be put in crates.io, released every time a compiler is released.

   I moved this to be an alternative because it might be a bit too magical. Also, I don't know how
   much Cargo is aware of different Rust / `rustc` versions at the moment, and this RFC depends on
   that awareness in some form. Finally, Semver won't be able to understand the release
   trains---pre-release/beta builds are allowed in its grammar but have different semantics.

 - Simply have `implicit-deps = false` make Cargo pass `--use-sysroot=false` to `rustc`, and don't
   have any way to explicitly depend on things in the sysroot.

   - This doesn't by-itself make a way for package to depend on only some of the crates behind the
     facade. That, in turn, means Cargo is little better at cross compiling those than before.

   - While unstable compiler users can just package the standard library and depend on it as a
     normal crate, it would be weird to have freestanding projects coalesce around some bootleg
     libcore on crates.io.

 - Make it so all dependencies, even libstd, must be explicit. C.f. Cabal and base. Simpler to
   implement, but breaks nearly all existing packages.

 - Don't do this, and be stuck with problems detailed in the motivation section.


# Unresolved questions

 - There are multiple lists of dependencies for different things (e.g. tests, build-time). How
   should `implicit-deps = false` affect them?

 - Just as make libstd a real dependency, we can make `rustc` a real dev dependency. The standard
   library can thus be built with Cargo by depending on the associated unstable compiler. Cargo
   would need to be taught an "x can build for y" relation for stable/unstable compiler
   compatibility however, rather than simply assuming all distinct compilers are mutually
   incompatible. This almost certainly is better addressed in a later RFC.
