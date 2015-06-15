- Feature Name: cargo_libstd_awareness
- Start Date: 2015-05-26
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Currently, Cargo doesn't know whether packages depend on libstd. This makes Cargo unsuitable for
packages that need a cross-compiled or custom libstd, or otherwise depend on crates with the same
names as libstd and the crates behind the facade. The proposed fixes also open the door to a future
where libstd can be Cargoized.


# Motivation

First some background. The current situation seems to be more of an accident of `rustc`'s pre-Cargo
history than an explicit design decision. Cargo passes the location and name of all depended-on
crates to `rustc`. This method is good for a number of reasons stemming from its fine granularity,
such as:

 - No undeclared dependencies can be used

 - Conversely, `rustc` can warn against *unused* declared dependencies

 - Crate/symbol names are frobbed so that packages with the overlapping names don't conflict


However rather than passing in libstd and its deps, Cargo lets the compiler look for them as need in
the compiler's sysroot [specifically `<sysroot>/lib/<target>`]. This is quite coarse in comparison,
and we loose all the advantages of the previous method:

 - Packages may link or not link against libs in that directory as they please, with Cargo being
   none the wiser. For the foreseeable future, libstd should be the only crate in that directory
   which stable Rust code link, but unstable Rust code can also freely link std's deps, or anything
   that ends up there by mistake.

 - Cargo-built crates with the same name as those in there will collide, as the sysroot libs don't
   have their names frobbed.

 - Cross compiling may fail at build-time (as opposed to the much shorter
   "gather-dependencies-time") because of missing packages


Cargo doesn't look inside the sysroot to see what is or isn't there, but it would hardly help if it
did, because it doesn't know what any package needs. Assuming all packages need libstd, for example,
means Cargo just flat-out won't build freestanding packages that just use libcore on a platform that
doesn't support libstd.

For an anecdote: in https://github.com/RustOS-Fork-Holding-Ground I tried to rig up Cargo to cross
compile libstd for me. Since I needed to use an unstable compiler anyways, it was possible in
principle to build absolutely everything I needed with the same `rustc` version. Because of some
trouble with Cargo and target JSONs, I didn't use a custom target specification, and just used
`x86_64-unknown-linux-gnu`, meaning that depending on platform I was compiling on, I may or may have
been cross-compiling. In the case where I wasn't, I couldn't complete the build because `rustc`
complained about the libstd I was building overlapping with the libstd in the sysroot.

For these reasons, most freestanding projects I know of avoid Cargo altogether, and just include
submodule rust and run make in that. Cargo can still be used if one manages to get the requisite
libraries in the sysroot. But this is a tedious operation that individual projects shouldn't need to
reimplement, and one that has serious security implications if the normal libstd is modified.

The fundamental plan proposed in this RFC is to make sure that anything Cargo builds never blindly
links against libraries in the sysroot. This is achieved by making Cargo aware of all dependencies,
including those libstd or its backing crates. That way, these problems are avoided.

For the record, I first raised this issue [here](https://github.com/rust-lang/Cargo/issues/1096).


# Detailed design

The only new interface proposed is a boolean field in `Cargo.toml` specifying that the package does
not depend on libstd by default. Note that this is technically orthogonal to Rust's `no_std`, as one
might want to `use` their own build of libstd by default, or implicitly depend on it but not
glob-import the prelude. To disambiguate, this field is called `implicit-deps`; please, go ahead and
bikeshead the name. `implicit-deps` is true by default to maintain compatibility with existing
packages. When true, "std" will be implicitly appended to the list of dependencies.

When Cargo sees a package name it cannot resolve, it will query `rustc` for the default sysroot, and
look inside to see if it can find a matching rlib. [It is necessary to query `rustc` because the
`rustc` directory layout is not stabilized and `rustc` and Cargo are versioned independently. The
same version issues make giving a Cargo a whitelist of potential standard library crate-names
risky.] If a matching rlib is successful found, Cargo will copy it (or simlink it) into the
project's build directly as if it built the rlib. Each rlib in the sysroot must be paired with some
sort of manifest listing its dependencies, so Cargo can copy those too.

`rustc` will have a new `--use-sysroot=<true|false>` flag. When Cargo builds a package, it will
always pass `--use-sysroot=false` to `rustc`, as any rlibs it needs will have been copied to the
build directory. Cargo can and will then pass those rlibs directly just as it does with normal Cargo
deps.

If Cargo cannot find the libraries it needs in the sysroot, or a library's dependency manifest is
missing, it will complain that the standard libraries needed for the current job are missing and
give up.

## Future Compatibility

In the future, rather than giving up if libraries are missing Cargo could attempt to download them
from some build cache. In the farther future, the stdlib libraries may be Cargoized, and Cargo able
to query pre-built binaries for any arbitrary package. In that scenario, we can remove all code
relating to falling back on the sysroot to look for rlibs.

In the meantime, developers living dangerously with an unstable compiler can package the standard
library themselves, and use their Cargo config file to get Cargo to cross compiler libstd for them.


# Drawbacks

Cargo does more work than is strictly necessary for rlibs installed in sysroot; some more metadata
must be maintained by `rustc` or its installation.

 - But in a future where Cargo can build stdlib like any other, all this cruft goes away.


# Alternatives

 - Simply have `implicit-deps = false` make Cargo pass `--use-sysroot=false` to `rustc`.

   - This doesn't by-itself make a way for package to depend on only some of the crates behind the
     facade. That, in turn, means Cargo is little better at cross compiling those than before.

   - While unstable compiler users can just package the standard library and depend on it as a
     normal crate, it would be weird to have freestanding projects coalesce around some bootleg
     libcore on crates.io.

 - Make it so all dependencies, even libstd, must be explicit. C.f. Cabal and base. Slightly
   simpler, but breaks nearly all existing packages.

 - Don't track stdlib depencies. Then, in the future when Cargo tries to obtain libs for cross
   compiling, stick them in the sysroot instead. Cargo either assumes package needs all of stdlib,
   or examines target to see what crates behind the facade are buildable and just goes for those.

    - Cargo does extra work if you need less of the stdlib

    - No nice migration into a world where Cargo can build stdlib without hacks.


# Unresolved questions

 - There are multiple lists of dependencies for different things (e.g. tests), Should libstd be
   append to all of them in phases 2 and 3?

 - Should rlibs in the sysroot respect Cargo name-frobbing conventions? If they don't, should Cargo
   frob the name when it copies it (e.g. with `ld -i`)?

 - Just as make libstd a real dependency, we can make `rustc` a real dev dependency. The standard
   library can thus be built with Cargo by depending on the associated unstable compiler. There are
   some challenges to be overcome, including:

    - Teaching Cargo and its frobber an "x can build for y" relation for stable/unstable compiler
      compatibility, rather than simply assuming all distinct compilers are mutually incompatible.

    - Coalescing a "virtual package" out of many different packages with disjoint dependencies. This
      is needed because different `rustc` version has a different library implementation that
      present the same interface.

   This almost certainly is better addressed in a later RFC.
