- Feature Name: cargo_libstd_awareness
- Start Date: 2015-05-26
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Currently, all packages implicitly depend on libstd. This makes Cargo unsuitable for packages that
need a custom-built libstd, or otherwise depend on crates with the same names as libstd and the
crates behind the facade. The proposed fixes also open the door to a future where libstd can be
Cargoized.

# Motivation

Bare-metal work cannot use a standard build of libstd. But since any crate built with Cargo can link
with a system-installed libstd if the target matches, using Cargo for such projects can be irksome
or impossible.

Cargoizing libstd also generally simplifies the infrastructure, and makes cross compiling much
slicker, but that is a separate discussion.

Finally, I first raised this issue here: https://github.com/rust-lang/Cargo/issues/1096 Also, there
are some (heavily bit-rotted) projects at https://github.com/RustOS-Fork-Holding-Ground that depend
on each other in the way this RFC would make much more feasible.

# Detailed design

The current situation seems to be more of an accident of `rustc`'s pre-Cargo history than an
explicit design decision. Cargo passes the location and name of all depended on crates to `rustc`.
This is good because it means that that no undeclared dependencies on other Cargo packages can leak
through. However, it also passes in `--sysroot /path/to/some/libdir`, the directory being were
libstd is. This means packages are free to use libstd, the crates behind the facade, or none of the
above, with Cargo being none the wiser.

The only new interface proposed is a boolean field to the package meta telling Cargo that the
package does not depend on libstd by default. This need not imply Rust's `no_std`, as one might want
to `use` their own build of libstd by default. To disambiguate, this field is called
`implicit-deps`; please, go ahead and bikeshead the name. `implicit-deps` is true by default to
maintain compatibility with existing packages.

The meaning of this flag is defined in 3 phases, where each phase extends the last. The idea being
is that while earlier phases are easier to implement, later phases yield a more elegant system.

## Phase 1

Add a `--use-sysroot=<true|false>` flag to `rustc`, where true is the default. Make Cargo pass
`--use-sysroot=false` to `rustc` is the case that `implicit-deps` is false.

This hotfix is enough to allow us bare-metal devs to use Cargo for our own projects, but doesn't
suffice for creating an ecosystem of packages that depend on crates behind the facade but not libstd
itself. This is because the choices are all or nothing: Either one implicitly depends on libstd or
the crates behind the facade, or they don't depend on them at all.

## Phase 2

Since, passing in a directory of crates is inherently more fragile than passing in a crate itself,
make Cargo use `--use-sysroot=false` in all cases.

Cargo would special case package names corresponding to the crates behind the facade, such that if
the package don't exist, it would simply pass the corresponding system crate to `rustc`. I assume
the names are blacklisted on crates.io already, so by default the packages won't exist. But users
can use config files to extend the namespace so their own modded libstds can be used instead. Even
if they don't want to change libstd but just cross-compile it, this is frankly the easiest way as
Cargo will seemliest cross compile both their project and it's transitive dependencies.

In this way we can put packages on crates.io that depend on the crates behind the facade. Some
packages that already exist, like liblog and libbitflags, should be given features that optionally
allow them to avoid libstd and just depend directly on the crates behind the facade they really
need.

## Phase 3

If/when the standard library is built with Cargo and put on crates.io, all the specially-cased
package names can be treated normally,

The standard library is downloaded and built from crates.io. Or equivalently, Cargo comes with a
cache of that build, as Cargo should be able cache builds between projects at this point. Just as in
phase 2, `implicit-deps = false` just prevents libstd from implicitly being appended to the list of
dependencies.

Again, to make this as least controversial as possible, this RFC does not propose outright that the
standard library should be Cargoized. This 3rd phases just describes how this feature would work
were that to happen.

# Drawbacks

I really don't know of any. Development for hosted environments would hardly be very affected.

# Alternatives

Make it so all dependencies, even libstd, must be explicit. C.f. Cabal and base.

# Unresolved questions

There are multiple lists of dependencies for different things (e.g. tests), Should libstd be append
to all of them in phases 2 and 3?
