- Feature Name: support_external_linkers
- Start Date: 2026-08-04
- RFC PR: [rust-lang/rfcs#3993](https://github.com/rust-lang/rfcs/pull/3993)
- Rust Issue: [rust-lang/rust#73632](https://github.com/rust-lang/rust/issues/73632)

## Summary
[summary]: #summary

Large projects written in multiple languages need to be able to link together
multiple Rust crates that form complex dependency trees, each compiled with
separate invocations of the Rust compiler. The current staticlib format exports
symbols from dependencies of the crate being compiled, which can cause
multiple-definition errors at link time. This RFC specifies an opt-in stable rlib
format that external build systems can use to produce a library including only
symbols from the crate being compiled, and no others, avoiding possible link errors.

## Motivation
[motivation]: #motivation

Frequently, Rust code is just one part of a large binary or dynamic library, perhaps
built with a language-neutral build system other than Cargo, such as
[Bazel](https://bazel.build/) and [Buck2](https://buck2.build/). In these projects,
there may be arbitrary combinations of Rust and C++ code such that the same crate
arises as a dependency at multiple points in the graph. The amount of investment
in the toolchain and workflow for these projects frequently predates the introduction
of Rust by years. Thus it is desirable to preserve a standard linking setup, in which
the build system directly invokes the system linker (e.g. ld), in order to build a
binary containing Rust code alongside code written in other languages.

Right now, the only [documented supported way][linkage-compiler-docs] to achieve
this is by compiling the crate with the `--crate-type=staticlib` switch (or
`crate-type = ["staticlib"]` in Cargo.toml). This works well for small projects.
However, it has the fundamental problem that dependencies of the Rust crate being
compiled are included in the resulting native library. This causes problems with
*diamond dependencies*. Suppose that we have the following dependency hierarchy
specified in the native build system:

```
        D (C++ Executable)
       / \
      B   C  (Rust Libraries, each built as staticlib)
       \ /
        A (Rust Library)
```

Rust crates B and C, both compiled with staticlib, depend on the Rust crate A, while
the C++ target D depends on B and C. Because of the semantics of staticlib, the
contents of A will be duplicated into B and C. This can cause D to fail to link,
because the linker can see definitions from A twice and exit with a "multiple
definition" error. (Note that multiple definition errors are not *guaranteed* in the
above scenario, because linkers are "lazy" and will only bring in symbols as requested.
The success of the link is determined by the particular symbols in use in these four
targets, as well as the number and makeup of each package's codegen units.)

The simplest way to solve this problem is to provide a supported way for the Rust
compiler to produce artifacts that export *only* the symbols from the crate being
compiled. That way, the build system, which has complete knowledge of the dependency
graph, can produce a final link line that guarantees each crate is included only
once in the resulting binary. In fact, Rust has a mechanism that is nearly perfect
for handling this already (and which Cargo uses to solve this exact problem): the
rlib format, which does not include symbols from dependent crates. However, the
contents of rlibs are unstable, so external build systems can't technically use them
without depending on implementation details of the compiler.

This RFC proposes an opt-in mechanism that external build systems can use to produce
rlib files with a stable format. It's intentionally minimal and avoids stabilizing any
more than is absolutely necessary for external build systems to work properly.
Additionally, this RFC proposes a simple mechanism for packaging foundational Rust
libraries such as the standard library into a format that the system linker can link
against, allowing the creation of usable Rust binaries without rustc driving the linker.


## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

### Compiler switch

A new compilation switch, `-C rlib-version`, is added to the compiler to control the
contents of .rlib archives. It takes one of two values, with more possible in future
versions of Rust:

* `-C rlib-version=unstable` - The default value, this option indicates that the
  contents of .rlib archives are unspecified. External tools should not rely on
  .rlib files conforming to any particular format.
* `-C rlib-version=v0` - This value indicates that .rlib files conform to the
  *version 0 format* defined here.

### Version 0 rlibs
A *version 0 rlib* is an archive file in the *native format* of the target, with
the usual extension (.lib or .a) replaced by .rlib. The native format is the usual
file format for statically-linked libraries on the target, which for all targets is
some variation of the [common ar archive format][wikipedia-ar].

Inside the rlib file, any number of object files may be present that provide code
and data for symbols defined by the Rust crate being compiled. Other files may also
be present, such as `.rmeta` files. This RFC makes no guarantees whatsoever about what
these files may or may not contain: in particular, this RFC doesn't stabilize any
kind of metadata format. External tools such as linkers should ignore any non-object
files, as their contents are unstable.

There must be a file inside the archive whose name begins with the string `_rlib_v00`.
The contents are typically empty. The name of this file allows tools to determine the
version of the rlib.

The object files inside a version 0 rlib must collectively contain *global definitions*
for all the *non-generic* functions and statics defined by the crate being compiled.
Global definitions must *not* be provided for any upstream dependencies of the crate,
to avoid symbol collisions when linking. It's OK for functions for upstream dependencies
to be present, but such symbols must be marked local to the archive. Symbol names should
be appropriately mangled; in the case of v0 symbol mangling, they should follow [Rust RFC 2603][rust-mangling-scheme].

rlib files often contain undefined symbols with definitions in other objects, whether
those objects be rlib files (i.e. crate dependencies) or other libraries such as native
static or dynamic libraries. This RFC intentionally doesn't provide a way for an external
tool to locate those dependencies. That's assumed to be the job of the build system.

These requirements are designed to allow non-rustc linkers to link executables created by
the Rust compiler, driven by a variety of build systems, in a way that doesn't result in
symbol conflicts when diamond dependencies are involved.

### Standard library bundles
In order to successfully produce a binary containing both Rust code and native code, a
way to link to the Rust standard library is needed. This RFC specifies a simple mechanism
for doing so: simply compile an empty crate (an empty lib.rs file is fine) as a staticlib
with a flag `-C emit-std-bundle=yes`. Any desired crate-level metadata and/or compiler
flags can be supplied in the process of compiling this *standard library bundle*, for
example `#![no_std]` to omit the standard library, or `-C target-feature` to enable
specific CPU features. The resulting artifact will be a linkable version of the standard
library.

An example standard library bundle workflow is as follows:

```console
$ echo '' > stdrust.rs
$ rustc --crate-type=staticlib -C emit-std-bundle=yes stdrust.rs
$ ls -lh libstdrust.a
-rw-r--r--  1 username  staff    16M  4 Aug 10:17 libstdrust.a
```

The resulting libstdrust.a may be installed into the library search path, at which
point `-lstdrust` may be added to the link line in order to link Rust executables.

### Supported Platforms
This technique is supported on `x86_64-unknown-linux-gnu`. This may function on non-windows
Tier 1 platforms. Other platforms are not supported.

### Complete Workflow
A complete worked example using this technique to compile and link a C++ application
which uses some Rust code looks like this.

```rust
// rustprintnumber/src/lib.rs
#[unsafe(no_mangle)]
pub extern "C" fn print_number(num: u64) -> u64 {
    println!("Number from Rust: {}", num);
    num
}
```

```c++
// main.cpp
#include <cstdint>
extern "C" {
    uint64_t print_number(uint64_t num);
}

int main() {
    return print_number(42) == 42 ? 0 : 1;
}
```
To keep it somewhat portable this example builds the C++ code using `c++`.

This example also performs the link using this same command, mainly to avoid specifying
C++ standard library dependencies. If these are known, `ld` or some other linker
could be invoked directly.

The rust crate needs to be built as `--crate-type=rlib` with `-C rlib-version=v0`.

The standard library bundle needs to be created by building an empty file as
`--crate-type=staticlib` with `-C emit-std-bundle=yes`.

These need to be passed to the linker in the correct order.

Below is a full pasteable script which performs all of the above:

```bash
cargo new --lib rustprintnumber

cat > rustprintnumber/src/lib.rs <<'EOF'
#[unsafe(no_mangle)]
pub extern "C" fn print_number(num: u64) -> u64 {
    println!("Number from Rust: {}", num);
    num
}
EOF

cat > main.cpp <<'EOF'
#include <cstdint>
extern "C" { uint64_t print_number(uint64_t num); }

int main() { return print_number(42) == 42 ? 0 : 1; }
EOF

echo '' | rustc --crate-type=staticlib -C emit-std-bundle=yes -o libstdrust.a -
rustc --crate-type=rlib -C rlib-version=v0 --crate-name rustprintnumber rustprintnumber/src/lib.rs -o librustprintnumber.rlib
c++ -c main.cpp -o main.o
c++ -o app main.o librustprintnumber.rlib -L. -lstdrust
./app
echo $?
```

RFC Note: This works today by removing the `-C emit-std-bundle=yes` and `-C rlib-version=v0` flags.

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD",
"SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be
interpreted as described in RFC 2119. They are not capitalized, for clarity.

### rlib versioning

When the compiler is instructed to produce rlib output, the contents of the resulting
artifact depend on the *rlib version* in use. The rlib version is specified by a
compiler switch with the syntax `-C rlib-version=VERSION`, with VERSION replaced by
one of the following:

* unstable - When the rlib version is unstable, the contents of the rlib file are
completely unspecified by this RFC. In particular, the resulting rlib files may,
or may not, actually be in v0 format. External tools should not assume that rlibs
with version unstable conform to any specific format.

*Note (non-normative):* Most likely, unstable will result in a version 0 rlib being
produced initially. The primary reason why unstable is left unspecified is so as not
to preclude the possibility of MIR-only rlibs in the future.

* v0 - When the rlib version is v0, the contents of the rlib must match the definition
supplied in the following section.

Other valid values of VERSION may exist. Their semantics are unspecified by this RFC.

### Version 0 rlib contents

A version 0 rlib must be an archive file in the native format of the target. All supported
targets use some variant of the [common ar archive format][wikipedia-ar].
The precise on-disk format of the archive file is unspecified by this RFC, but it must
contain linkable object files as well as a symbol table.

For targets that do not use a variant of the common ar archive format, as well as
WebAssembly, this RFC does not define the format of a version 0 rlib. Such platforms
may or may not support version 0 rlibs at all.

In this section, we make reference to concepts of the [BFD library](https://en.wikipedia.org/wiki/Binary_File_Descriptor_library).
This provides a convenient way to abstract over the concepts that correspond to one
another in different object formats.

*Note (non-normative):* BSD, System V, and Windows use incompatible mechanisms for
specifying symbol tables inside the ar format, so we must use an abstraction.

The *target crate* is the crate that the current invocation of the compiler is
compiling and producing an rlib artifact for.

A *global symbol* is a symbol with the [BFD BSF_GLOBAL flag](https://ftp.gnu.org/old-gnu/Manuals/bfd-2.9.1/html_mono/bfd.html#SEC57)
set. An rlib library defines whatever global symbols are required to link, subject
to the three conditions below.

A *local symbol* is a symbol with the [BFD BSF_LOCAL flag](https://ftp.gnu.org/old-gnu/Manuals/bfd-2.9.1/html_mono/bfd.html#SEC57)
set. An rlib library may contain any number of local symbols. Their names and contents
are unspecified by this RFC.

This RFC does not define the contents of the set of global symbols exported by an
rlib archive. Instead, it requires that some number of global symbols shall be
exported such that the following three conditions are fulfilled:

1. If some Rust crate B depends on Rust crate A, and both A and B are in .rlib format,
  the .rlib files corresponding to A and B shall be successfully linkable using the
  system linker, notwithstanding other requirements specified in this section.

2. Any set of crates in .rlib format compiled by the same Rust compiler (including
  compiler version) must be linkable together as long as the following conditions
  are fulfilled:
    1. For each crate, all dependencies of that crate must be in the set.
    2. The set contains each .rlib file no more than once.

There are two exceptions to this rule:

(i) Multiple crates that define the same *language item* may not be linkable together.

(ii) Multiple crates that define identically-named items marked with `#[no_mangle]`
     may not be linkable together.

3. For each item with a `#[no_mangle]` annotation, a global symbol must be present
  in the archive with a name matching that of an identically-named C symbol definition
  on the target.

*Note (non-normative):* Some binary formats mark C symbols in some way (e.g. Mach-O
  represents them with a leading _).

*Note (non-normative):* "Linkable using the system linker" implies that there are
  neither undefined nor multiply-defined symbols.

*Note (non-normative):* [Rust RFC 2603][rust-mangling-scheme] specifies a mangling
  scheme for symbols.

*Note (non-normative):* Symbols relating to global allocation and panic handling
  must not be defined in the .rlib unless the crate itself defines those symbols.

The object files containing the symbols that the target crate defines shall be present
inside the rlib archive. Any other files necessary for the Rust compiler to link to
the target crate and use it as a dependency must also be present. The object files
must be linkable in a format that the target supports.

*Note (non-normative):* Examples of linkable object files on various platforms include
but are not limited to ELF, Mach-O, PE/COFF, LLVM bitcode for full LTO, LLVM bitcode
for ThinLTO, and LLVM bitcode wrapped in a native object file.

*Note (non-normative):* The most important non-object file is the .rmeta file, which
contains data necessary for downstream Rust invocations to use the crate as a dependency,
such as the types and contents of inlined functions. The system linker ignores this information.

Additionally, a file with a name beginning with the string `_rlib_v00` (`00` to allow for `10`)
must be present inside the rlib archive. The contents of this file are unspecified. It
allows build systems and other tools to determine that the rlib is in version 0 format.

Any other files may also be present inside the rlib archive. Whether these files
exist, and what they contain, is unspecified by this RFC.

All rlib files that are to be linked together must be built in a *compatible manner*.
The precise definition of *compatible manner* is unspecified by this RFC.

*Note (non-normative):* The reason for not defining the term *compatible manner* precisely
is so that new linkage restrictions may be added in the future. For example, one could
imagine a later version of Rust introducing multiple ABIs such as those suggested in the
[interoperable_abi proposal](https://github.com/rust-lang/rust/pull/105586). In this
case, the ABI of the rlibs and the ABI of the standard library bundle would all need to
match for the link to succeed. This RFC is intended to preserve maximum flexibility for
such changes in the future.

### Standard library bundles

In order to successfully perform the final link of an executable containing Rust code, the
core library must be present on the link line. Frequently, the std library must be present
as well. This RFC specifies a mechanism to produce a *standard library bundle* containing
core or std, appropriately configured to match the given target.

A crate successfully compiled as staticlib that contains no Rust symbols and no
dependencies other than core or std with the `-C emit-std-bundle=yes` flag is known
as a *standard library bundle*. The native system linker must be able to successfully
link an executable containing Rust code if:

* All such crates containing Rust code are supplied precisely once to the system linker.
* All transitive rlib dependencies of all such crates are supplied precisely once to the
  system linker.
* Exactly one of the core or std standard library bundles is supplied to the system
  linker. This standard library bundle must have been built in a *compatible manner*
  with all rlibs to be linked.

*Note (non-normative):* At the time of writing, the `-C emit-std-bundle=yes` flag can
  simply be a no-op, as the Rust compiler can successfully create such staticlibs
  already by compiling an empty crate. The purpose of the flag is to ensure that this
  behavior is preserved in the future in an opt-in fashion.

The exact symbols that are exposed in a standard library bundle is unspecified by this
RFC. In general, they are expected to change with every Rust release and may change
depending on the manner in which the standard library channel was compiled.

*Note (non-normative):* The standard library bundle approach allows this RFC to
avoid specifying details like the behavior of allocator shims, raw-dylib, bundled
static libraries, `#[global_allocator]`, the allocation error handler,
`-C panic=abort` and `-C panic=unwind`, and so forth.

### Supported Platforms
This RFC is written in a non-platform specific way, and this workflow could be
extended to work on most of the platforms Rust builds and runs on. This RFC is
only targeting `x86_64-unknown-linux-gnu` for the purpose of reducing the
support surface area. Likely supporting all Tier 1 platforms wouldn't be too
tricky. See Unresolved Questions at the bottom.

Note: during earlier discussions some concerns around AIX were raised.
`powerpc64-ibm-aix` is a Tier 3 platform and was not considered during RFC writing.

### Bundled Static Libraries
rustc now supports the concept of [bundled static libraries](https://github.com/rust-lang/rust/pull/100101),
which are native libraries placed *inside* a .rlib file. The v0 rlib format doesn't
support such libraries; generally, native build systems would prefer to keep
libraries separate, for better interoperability with native code. This can be revisited
with future rlib versions if need be.


### Interaction with Externally Implementable Items (EII)
The largest change since the original pre-RFC is the work done towards [Externally
Implementable Items (EII)][eii-issue].

Some symbols that a Rust link line needs, notably the global allocator and the
panic handler, are emitted and their consistency (exactly one definition across
the program) is enforced by `rustc` at link time. When `rustc` does not
drive the final link, nothing enforces that global consistency, and nothing
guarantees the symbols are present exactly once.

Currently these symbols are defined in the standard library bundle.

The intended long-term resolution is for these items to be modeled as EIIs and
for the global allocator and panic handlers to be provided through that mechanism,
so that their definitions live in crates (and therefore optionally in `rlib`s,
not only the standard library bundle) with well-defined single-definition semantics.
That work is not yet complete.

This RFC does not intend to block on that work (or block that work) by leaving it the
future responsibility of the build system to bring compatible EIIs to the link-step as
required by the application being built.

## Drawbacks
[drawbacks]: #drawbacks

### Maintenance
This scheme doesn't preclude Rust changing the rlib format (for example, introducing
MIR-only rlibs), but if Rust does so, under this RFC the compiler will need to retain
support for the version 0 rlib support described here behind a compiler switch.
This may add some amount of maintenance burden.

### Externally Implementable Items
This is a new supported build workflow that interacts with the EII work. Because the
standard library bundle is the current home for allocator/panic symbols, any changes
to this as EII is finalized must consider this workflow too. Practically, since this
workflow is already used in production (incl. by stakeholders involved in that work),
a sensible compromise between rustc and build system responsibility feels reasonable
and achievable here.

### Users potentially exposed to Linker Errors
This workflow links cleanly only under the stated constraints. A build system that
violates them (a missing transitive `rlib`, a crate linked twice, an incompatible
bundle, etc) gets ordinary linker errors with little Rust-augmented diagnosis. Rust
users in general are not exposed to linker errors. Since the intended usage of this
feature is within third party, often complex, build systems, the responsibility for
preventing these errors, or at least keeping them debuggable lies with the owners of
that build system.


## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

### Why stabilize a versioned subset of the current rlib format rather than something new
The `rlib` is already produced by `rustc`, is already the artifact with exactly
the required properties, and Cargo already relies on it to solve this problem
internally. Stabilizing a versioned subset is intended as the least-invasive change
that unblocks external build systems while preserving room to evolve the internal
format.

### Alternative: Leave this behavior unsupported
Bazel/Buck2 already rely on this unsupported behavior. Any change to the internal
format silently breaks these external systems, although they have accepted that
risk. Providing a supported workflow here should ease incremental Rust adoption
in large C++ code bases.

### Alternative: Require external build systems invoke `rustc` for the final link
This would permit `rlib` to continue to be unspecified. But this requires every
C/C++ binary that transitively contains any Rust to switch its linker driver, which
is a large ask for an existing monorepo and blocks incremental adoption. This
requirement also does not compose well with other languages since only one can own
the final link.

### Alternative: A pre-link `rustc` invocation that bundles all Rust deps
A special `rustc` invocation that bundles all Rust deps into one system library
just before the final link. If Rust and non-Rust are freely intermixed in the
build graph, the only place this pre-link can run is immediately before the final
link, which makes it sort of a linker wrapper with similar issues as the
previous alternative. Adding a per-executable pre-link step on the critical path
of every non-Rust binary that contains Rust could significantly increase build times,
since every such executable caches its own bundled copy of its transitive Rust
dependencies.

### Alternative: A new crate type `staticlib-nobundle`
A new crate type, staticlib-nobundle or similar, which works like staticlib but
without marking symbols from dependent crates global.

This would mean that the same crate cannot be officially used from both Rust and
C++, despite being essentially identical. In projects that have both Rust and C++
upstream crates that depend on a single Rust downstream crate, this would result
in duplicate symbol errors as both the rlib and the staticlib-nobundle would be
linked into the final binary.

### Alternative: `--emit=obj` instead of using staticlibs or rlibs
With this approach, there is no obvious place for the Rust compiler to emit metadata
(.rmeta files). Without metadata, the crate would no longer be linkable from Rust,
only from C or C++, meaning that a library meant to be used from both C/C++ and
Rust would need to be built twice. Additionally, this forces the number of codegen
units to 1, causing compilation performance problems. Finally, this would have the
same problem as staticlib-nobundle in that if both Rust and C++ link to the same crate,
duplicate symbol errors would result, as Rust would be linking to an rlib and C++ would
be linking to an object file with the same symbols.

### Alternative: Use `--emit=obj`, and add support for multiple codegen units when using `--emit=obj`
rustc could generate the object files separately and then use `ld -r` to link them
together. The `-r` (relocatable) switch to ld allows multiple .o files to be
combined into another .o file that can then be further linked into a binary.
Unfortunately, this was tried early on in Rust's development and it was discovered
that `ld -r` is often poorly supported by OS toolchains on account of how seldom
the feature is used. Furthermore, this inherits the same problems mentioned before
regarding metadata and duplicate symbols.

### Alternative: Use an flag that doesn't carry a version number, like `-C rlib-format=platform`
This would be essentially the same as this RFC, but would not leave room for
different versions in the future. For example, platforms might introduce new
library formats in the future, or we might want to add some extra information
to the .rlib format consumable by outside tools. In these cases, the ability
to release a v1 version and beyond would be useful.

### Alternative: Stabilize the `rlib` contents with no version flag
This would prevent Rust from adopting MIR-only rlibs in the future, which are a
commonly-discussed feature. The goal of this RFC isn't to hinder experimentation
with alternative rlib formats.


## Prior art
[prior-art]: #prior-art

### Swift
Swift documents how to build Swift code and link using an external linker, for
the purpose of integrating Swift code into external build systems:
https://github.com/swiftlang/swift/blob/main/docs/Driver.md

The similarities to what is proposed in this RFC are:
1. Swift builds modules into linkable files, and then can be linked using the build
   system's chosen linker.
2. The tone of the Swift document is that this technique should only be needed by
   those writing build systems.
3. The guarantees and purpose of `.swiftmodule` look similar to the `.rmeta` bundle.
   I.e. unstable contents, compiler-version specific contents which are required to
   use the package in a swift specific build system.

The differences are:
1. Swift also standardizes a `.swiftinterface` file which allows libraries to be
   distributed and used by other compiler versions. This RFC is not proposing
   this for Rust.
2. Swift embeds link dependency information in the object files which appears to be
   mac specific (with a helper tool for other platforms), and also documents that
   external build systems can produce this information themselves. This RFC instead
   requires that the build system understand the build graph, and follow a specific
   procedure to create the standard library bundle which must be passed to the
   final link.
3. The unstable `.swiftmodule` files required by the swift compiler for consuming a
   module are separate from the object files passed to the linker. The `.rlib` files
   embed this information within the archive.
4. Swift can build modules into an object file, or an ar archive, or a `bundle`.
   This RFC is only aiming to produce `ar` archives (with the file extension `.rlib`)


## Unresolved questions
[unresolved-questions]: #unresolved-questions

### Resolve before merge:

- **Should we introduce `-C emit-std-bundle=yes` at all** when building a
  staticlib from an empty .rs file works today. We could skip adding a new
  flag, though being explicit about the purpose of building that staticlib
  feels useful from a future extension perspective.
- **How much of "compatible manner" needs documenting?**
  How much beyond "same compiler version" should we document being required to
  match (We could state relevant `-C` flags, target features, panic strategy).
  Options not currently explored in this RFC include enforcing aspects of this
  through version/guard symbol dependencies in rlibs, to be fulfilled by the
  static library bundle.
- **Flag naming and shape.** `-C rlib-version=v0` vs. `-C rlib-format=platform`
  vs. something else.
- **Should an extra constraint be added to cover EII?**
  If rustc isn't invoked before the final link, there are fewer opportunities
  to validate all EII symbols are provided. It seems fair that complex build
  systems which do not link using rustc lose these benefits. Is there an extra
  link constraint we can add which makes it clear these build systems need to
  consider how to resolve EIIs in future?
- **Supported Platforms.** All non-Windows Tier 1 platforms are likely straight
  forward to support. But should the initial support just be
  `x86_64-unknown-linux-gnu` anyway? Is there interest for supporting Mac or Windows?
  Dropping other platforms is primarily for forward momentum & author
  knowledge/requirements.

### Resolve during implementation / before stabilization:

- **The CI-tested configuration.** Which compiler flags, panic strategies,
  and platforms are in-scope for covering under tests.
- **Testing scope.** What should the tests check for. I.e. link test
  of a diamond graph, symbol-table checks on emitted `rlib`s?, etc.

### Out of scope:
- Support for non-tier 1 platforms, including WebAssembly. These are primarily
  excluded due to lack of demand, and potential practical or specification complexity.


## Future possibilities
[future-possibilities]: #future-possibilities

### Machine Readable --print native-static-libs
The libstdrust.a standard library bundle still requires a few other native libraries
to link correctly when invoking `ld` directly. These are nearly all also required by
most C++ code, but currently the best way to get this is to pass `--print native-static-libs`
while building the standard library bundle, and parse this output:

```
note: link against the following native artifacts when linking against this static library. The order and any duplication can be significant on some platforms.

note: native-static-libs: -lSystem -lc -lm
```

### Support more targets
More target platforms could be supported for this workflow, if needed.

### MIR-only `rlib`s
Leaving the default `unstable` `rlib` version unspecified gives freedom for future
`rlib` changes.


## Acknowledgements
This RFC is largely directly based on Patrick Walton (@pcwalton)'s pre-RFC [Stabilize a version of the `rlib` format][pre-rfc] and the discussion in
[rust-lang/rust#73632][issue-73632], which was originally filed by Adrian Taylor.

Thanks to Patrick Walton for the original writing, and everyone who participated in the
Discourse thread, GitHub issue, and input and support in conversations online & offline
in the last couple of months including but not limited to: Taylor Foxhall, Daniel Ababei,
Kojo Adams, Chris Bauer, Taylor Cramer, and Tyler Mandry.

pcwalton also left credit to Jeremy Fitzhardinge, Matt Hammerly, Dana Jansens, Augie Fackler, Marcel Hlopko,
and bjorn3 for feedback on the pre-RFC.

[pre-rfc]: https://internals.rust-lang.org/t/pre-rfc-stabilize-a-version-of-the-rlib-format/17558
[issue-73632]: https://github.com/rust-lang/rust/issues/73632
[rust-mangling-scheme]: https://rust-lang.github.io/rfcs/2603-rust-symbol-name-mangling-v0.html
[eii-issue]: https://github.com/rust-lang/rust/issues/125418
[linkage-compiler-docs]: https://doc.rust-lang.org/reference/linkage.html#r-link.foreign-code.foreign-linkers
[wikipedia-ar]: https://en.wikipedia.org/wiki/Ar_\(Unix\)
