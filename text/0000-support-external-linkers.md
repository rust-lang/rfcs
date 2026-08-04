- Feature Name: (fill me in with a unique ident, `my_awesome_feature`)
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary
[summary]: #summary

Large projects written in multiple languages need to be able to link together multiple Rust crates that form complex dependency trees, each compiled with separate invocations of the Rust compiler. The current staticlib format exports symbols from dependencies of the crate being compiled, which can cause multiple-definition errors at link time. This RFC specifies an opt-in stable rlib format that external build systems can use to produce a library including only symbols from the crate being compiled, and no others, avoiding possible link errors.

## Motivation
[motivation]: #motivation

Frequently, Rust code is just one part of a large binary or dynamic library, perhaps built with a language-neutral build system other than Cargo, such as [Bazel](https://bazel.build/) and [Buck](https://buck.build/). In these projects, there may be arbitrary combinations of Rust and C++ code such that the same crate arises as a dependency at multiple points in the graph. The amount of investment in the toolchain and workflow for these projects frequently predates the introduction of Rust by years. Thus it is desirable to preserve a standard linking setup, in which the build system directly invokes the system linker (e.g. ld), in order to build a binary containing Rust code alongside code written in other languages.

Right now, the documented way to achieve this is by compiling the crate with the [\--crate-type=staticlib switch](https://doc.rust-lang.org/reference/linkage.html) (or crate-type \= \["staticlib"\] in Cargo.toml). This works well for small projects. However, it has the fundamental problem that dependencies of the Rust crate being compiled are included in the resulting native library. This causes problems with *diamond dependencies*. Suppose that we have the following dependency hierarchy specified in the native build system:

![A -\> {B, C} -\> D][image1]

Rust crates B and C, both compiled with staticlib, depend on the Rust crate A, while the C++ target D depends on B and C. Because of the semantics of staticlib, the contents of A will be duplicated into B and C. This can cause D to fail to link, because the linker can see definitions from A twice and exit with a "multiple definition" error. (Note that multiple definition errors are not *guaranteed* in the above scenario, because linkers are "lazy" and will only bring in symbols as requested. The success of the link is determined by the particular symbols in use in these four targets, as well as the number and makeup of each package's codegen units.)

The simplest way to solve this problem is to provide a supported way for the Rust compiler to produce artifacts that export *only* the symbols from the crate being compiled. That way, the build system, which has complete knowledge of the dependency graph, can produce a final link line that guarantees each crate is included only once in the resulting binary. In fact, Rust has a mechanism that is nearly perfect for handling this already (and which Cargo uses to solve this exact problem): the rlib format, which does not include symbols from dependent crates. However, the contents of rlibs are unstable, so external build systems can't technically use them without depending on implementation details of the compiler.

This RFC proposes an opt-in mechanism that external build systems can use to produce rlib files with a stable format. It's intentionally minimal and avoids stabilizing any more than is absolutely necessary for external build systems to work properly. Additionally, this RFC proposes a simple mechanism for packaging foundational Rust libraries such as the standard library into a format that the system linker can link against, allowing the creation of usable Rust binaries without rustc driving the linker.


## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

### Compiler switch

A new compilation switch, \-C rlib-version, is added to the compiler to control the contents of .rlib archives. It takes one of two values, with more possible in future versions of Rust:

* \-C rlib-version=unstable — The default value, this option indicates that the contents of .rlib archives are unspecified. External tools should not rely on .rlib files conforming to any particular format.
* \-C rlib-version=v0 — This value indicates that .rlib files conform to the *version 0 format* defined here.

### Version 0 rlibs

A *version 0 rlib* is an archive file in the *native format* of the target, with the usual extension (.lib or .a) replaced by .rlib. The native format is the usual file format for statically-linked libraries on the target, which for all targets is some variation of the [common ar archive format](https://en.wikipedia.org/wiki/Ar_\(Unix\)). (The format of WebAssembly rlibs is unspecified in this RFC.)

Inside the rlib file, any number of object files may be present that provide code and data for symbols defined by the Rust crate being compiled. Other files may also be present, such as .rmeta files. This RFC makes no guarantees whatsoever about what these files may or may not contain: in particular, this RFC doesn't stabilize any kind of metadata format. External tools such as linkers should ignore any non-object files, as their contents are unstable.

There must be a file inside the archive whose name begins with the string \_rlib\_v00. The contents are typically empty. The name of this file allows tools to determine the version of the rlib.

The object files inside a version 0 rlib must collectively contain *global definitions* for all the *non-generic* functions and statics defined by the crate being compiled. Global definitions must *not* be provided for any upstream dependencies of the crate, to avoid symbol collisions when linking. It's OK for functions for upstream dependencies to be present, but such symbols must be marked local to the archive. Symbol names should be appropriately mangled; in the case of v0 symbol mangling, they should follow [Rust RFC 2603](https://rust-lang.github.io/rfcs/2603-rust-symbol-name-mangling-v0.html).

rlib files often contain undefined symbols with definitions in other objects, whether those objects be rlib files (i.e. crate dependencies) or other libraries such as native static or dynamic libraries. This RFC intentionally doesn't provide a way for an external tool to locate those dependencies. That's assumed to be the job of the build system.

These requirements are designed to allow non-rustc linkers to link executables created by the Rust compiler, driven by a variety of build systems, in a way that doesn't result in symbol conflicts when diamond dependencies are involved.

### Standard library bundles

In order to successfully produce an binary containing both Rust code and native code, a way to link to the Rust standard library is needed. This RFC specifies a simple mechanism for doing so: simply compile an empty crate (an empty lib.rs file is fine) as a staticlib with a flag \-C emit-std-bundle=yes. Any desired crate-level metadata and/or compiler flags can be supplied in the process of compiling this *standard library bundle*, for example \#\!\[no\_std\] to omit the standard library, or \-C target-feature to enable specific CPU features. The resulting artifact will be a linkable version of the standard library.

An example workflow is as follows:

$ cargo new \--lib stdrust
$ cd stdrust
$ echo "\[lib\]" \>\>Cargo.toml
$ echo "crate-type \= \['staticlib'\]" \>\>Cargo.toml
$ RUSTFLAGS="-C emit-std-bundle=yes" cargo build \--release
$ ls \-l target/release/libstdrust.a
\-rw-------  1 pcwalton  staff  17031504 Jan 19 19:37 target/release/libstdrust.a

The resulting libstdrust.a may be installed into the library search path, at which point \-lstdrust may be added to the link line in order to link Rust executables.


## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in RFC 2119\. They are not capitalized, for clarity.

## **rlib versioning**

When the compiler is instructed to produce rlib output, the contents of the resulting artifact depend on the *rlib version* in use. The rlib version is specified by a compiler switch with the syntax \-C rlib-version=VERSION, with VERSION replaced by one of the following:

* unstable — When the rlib version is unstable, the contents of the rlib file are completely unspecified by this RFC. In particular, the resulting rlib files may, or may not, actually be in v0 format. External tools should not assume that rlibs with version unstable conform to any specific format.

*Note (non-normative):* Most likely, unstable will result in a version 0 rlib being produced initially. The primary reason why unstable is left unspecified is so as not to preclude the possibility of MIR-only rlibs in the future.

* v0 — When the rlib version is v0, the contents of the rlib must match the definition supplied in the following section.

Other valid values of VERSION may exist. Their semantics are unspecified by this RFC.

## **Version 0 rlib contents**

A version 0 rlib must be an archive file in the native format of the target. All supported targets use some variant of the [common ar archive format](https://en.wikipedia.org/wiki/Ar_\(Unix\)). In particular, all supported targets begin their archive format with the string \!\<arch\> followed by a newline character: i.e. the bytes 0x21 0x3C 0x61 0x72 0x63 0x68 0x3E 0x0A. The precise on-disk format of the archive file is unspecified by this RFC, but it must contain linkable object files as well as a symbol table.

For targets that do not use a variant of the common ar archive format, as well as WebAssembly, this RFC does not define the format of a version 0 rlib. Such platforms may or may not support version 0 rlibs at all.

In this section, we make reference to concepts of the [BFD library](https://en.wikipedia.org/wiki/Binary_File_Descriptor_library). This provides a convenient way to abstract over the concepts that correspond to one another in different object formats.

*Note (non-normative):* BSD, System V, and Windows use incompatible mechanisms for specifying symbol tables inside the ar format, so we must use an abstraction.

The *target crate* is the crate that the current invocation of the compiler is compiling and producing a rlib artifact for.

A *global symbol* is a symbol with the [BFD BSF\_GLOBAL flag](https://ftp.gnu.org/old-gnu/Manuals/bfd-2.9.1/html_mono/bfd.html#SEC57) set. An rlib library defines whatever global symbols are required to link, subject to the three conditions below.

A *local symbol* is a symbol with the [BFD BSF\_LOCAL flag](https://ftp.gnu.org/old-gnu/Manuals/bfd-2.9.1/html_mono/bfd.html#SEC57) set. A rlib library may contain any number of local symbols. Their names and contents are unspecified by this RFC.

This RFC does not define the contents of the set of global symbols exported by an rlib archive. Instead, it requires that some number of global symbols shall be exported such that the following three conditions are fulfilled:

1. If some Rust crate B depends on Rust crate A, and both A and B are in .rlib format, the .rlib files corresponding to A and B shall be successfully linkable using the system linker, notwithstanding the requirements specified in the "additional linking requirements" section.
2. Any set of crates in .rlib format compiled by the same Rust compiler (including compiler version) must be linkable together as long as the following conditions are fulfilled:

a. For each crate, all dependencies of that crate must be in the set.

b. All conditions specified in the "additional linking requirements" section are met.

c. The set contains each .rlib file no more than once.

There are two exceptions to this rule:

(i) Multiple crates that define the same *language item* may not be linkable together.

(ii) Multiple crates that define identically-named items marked with \#\[no\_mangle\] may not be linkable together.

3. For each item with a \#\[no\_mangle\] annotation, a global symbol must be present in the archive with a name matching that of an identically-named C symbol definition on the target.

*Note (non-normative):* Some binary formats mark C symbols in some way (e.g. Mach-O represents them with a leading \_).

*Note (non-normative):* "Linkable using the system linker" implies that there are neither undefined nor multiply-defined symbols.

*Note (non-normative):* [Rust RFC 2603](https://rust-lang.github.io/rfcs/2603-rust-symbol-name-mangling-v0.html) specifies a mangling scheme for symbols.

*Note (non-normative):* Symbols relating to global allocation and panic handling must not be defined in the .rlib unless the crate itself defines those symbols.

The object files containing the symbols that the target crate defines shall be present inside the rlib archive. Any other files necessary for the Rust compiler to link to the target crate and use it as a dependency must also be present. The object files must be linkable in a format that the target supports.

*Note (non-normative):* Examples of linkable object files on various platforms include but are not limited to ELF, Mach-O, PE/COFF, LLVM bitcode for full LTO, LLVM bitcode for ThinLTO, and LLVM bitcode wrapped in a native object file.

*Note (non-normative):* The most important non-object file is the .rmeta file, which contains data necessary for downstream Rust invocations to use the crate as a dependency, such as the types and contents of inlined functions. The system linker ignores this information.

Additionally, a file with a name beginning with the string \_rlib\_v00 must be present inside the rlib archive. The contents of this file are unspecified. It allows build systems and other tools to determine that the rlib is in version 0 format.

Any other files may also be present inside the rlib archive. Whether these files exist, and what they contain, is unspecified by this RFC.

All rlib files that are to be linked together must be built in a *compatible manner*. The precise definition of *compatible manner* is unspecified by this RFC.

*Note (non-normative):* The reason for not defining the term *compatible manner* precisely is so that new linkage restrictions may be added in the future. For example, one could imagine a later version of Rust introducing multiple ABIs such as those suggested in the [interoperable\_abi proposal](https://github.com/rust-lang/rust/pull/105586). In this case, the ABI of the rlibs and the ABI of the standard library bundle would all need to match for the link to succeed. This RFC is intended to preserve maximum flexibility for such changes in the future.

## **Standard library bundles**

In order to successfully perform the final link of an executable containing Rust code, the core library must be present on the link line. Frequently, the std library must be present as well. This RFC specifies a mechanism to produce a *standard library bundle* containing core or std, appropriately configured to match the given target.

A crate successfully compiled as staticlib that contains no Rust symbols and no dependencies other than core or std with the \-C emit-std-bundle=yes flag is known as a *standard library bundle*. The native system linker must be able to successfully link an executable containing Rust code if:

* All such crates containing Rust code are supplied precisely once to the system linker.
* All transitive rlib dependencies of all such crates are supplied precisely once to the system linker.
* Exactly one of the core or std standard library bundles is supplied to the system linker. This standard library bundle must have been built in a *compatible manner* with all rlibs to be linked.

*Note (non-normative):* At the time of writing, the \-C emit-std-bundle=yes flag can simply be a no-op, as the Rust compiler can successfully create such staticlibs already by compiling an empty crate. The purpose of the flag is to ensure that this behavior is preserved in the future in an opt-in fashion.

The exact symbols that are exposed in a standard library bundle is unspecified by this RFC. In general, they are expected to change with every Rust release and may change depending on the manner in which the standard library channel was compiled.

*Note (non-normative):* The standard library bundle approach allows this RFC to avoid specifying details like the behavior of allocator shims, raw-dylib, bundled static libraries, \#\[global\_allocator\], the allocation error handler, \-C panic=abort and \-C panic=unwind, and so forth.

## Drawbacks
[drawbacks]: #drawbacks

This scheme doesn't preclude Rust changing the rlib format (for example, introducing MIR-only rlibs), but if Rust does so, under this RFC the compiler will need to retain support for the version 0 rlib support described here behind a compiler switch. This may add some amount of maintenance burden.


### Addressing potential issues

* rustc now supports the concept of [bundled static libraries](https://github.com/rust-lang/rust/pull/100101), which are native libraries placed *inside* a .rlib file. The v0 rlib format doesn't support such libraries; generally, native build systems would prefer to keep libraries separate, for better interoperability with native code. This can be revisited with future rlib versions if need be.
* [An issue regarding static initializers](https://github.com/rust-lang/rust/issues/73632#issuecomment-1043036479) was raised during the discussion: they don't reliably work unless \--whole-archive is provided when linking the rlib. However, \--whole-archive is not available on AIX. AIX is currently not a supported platform for Rust, however; if and when it becomes one, the RFC for support for that platform can specify what to do here. Additionally, this is an issue that would be present regardless of whether the rlib format is specified.

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

* *Keep the format of .rlib files unstable officially, but have external build systems depend on their format anyway.* This wouldn't immediately have any ill effects, as external build systems like Buck and Bazel could depend on the contents of .rlib files and things would probably continue to work for some time. It would also have the advantage of avoiding the complexity of extra compiler switches and would allow the compiler to make a clean switch to MIR-only rlibs someday. However, this would cause breakage if Rust ever decides to change the format of rlibs.
* *Have external build systems invoke rustc instead of ld to perform the final link.* This would allow Rust to make a clean break with the past if it switches to MIR-only rlibs. It would also potentially obviate the need for the build system to be aware of the dependency graph, including standard library crates. However, it would force large C++ projects to switch linkers whenever they link in any Rust at all, which would significantly reduce the willingness of many C++ projects to incrementally adopt Rust by burdening the build system with extra logic. It would also be incompatible with any other language wanting to "take over" linking in this way; only one language can be in charge of the last linking stage, and the advantage of system ld is that it's language-neutral.
* *Have external build systems invoke rustc to bundle all Rust dependencies together into one library, which is then linked into the final binary.* This is similar to the previous alternative, except it adds an extra step. It would have the advantage of allowing rustc to automatically add extra libraries that need to be added to the final link line, such as allocator shims and native bundled libraries, without having to duplicate that logic into the external build system. However, this has potential performance issues due to needing to process Rust code twice, once with the rustc\-invoked linker and once with the native linker. Additionally, this would complicate the common task of introducing Rust components to two unrelated portions of a large binary by requiring the build system to track every binary to determine whether Rust is involved and adding an extra global linking step if so.
* *Add a new crate type, staticlib-nobundle or similar, which works like staticlib but without marking symbols from dependent crates global.* This would mean that the same crate cannot be officially used from both Rust and C++, despite being essentially identical. In projects that have both Rust and C++ upstream crates that depend on a single Rust downstream crate, this would result in duplicate symbol errors as both the rlib and the staticlib-nobundle would be linked into the final binary.
* *Use \--emit=obj instead of using staticlibs or rlibs.* With this approach, there is no obvious place for the Rust compiler to emit metadata (.rmeta files). Without metadata, the crate would no longer be linkable from Rust, only from C or C++, meaning that a library meant to be used from both C/C++ and Rust would need to be built twice. Additionally, this forces the number of codegen units to 1, causing compilation performance problems. Finally, this would have the same problem as staticlib-nobundle in that if both Rust and C++ link to the same crate, duplicate symbol errors would result, as Rust would be linking to an rlib and C++ would be linking to an object file with the same symbols.
* *Use \--emit=obj, and add support for multiple codegen units when using \--emit=obj to rustc by having the compiler generate the object files separately and then use ld \-r to link them together.* The \-r (relocatable) switch to ld allows multiple .o files to be combined into another .o file that can then be further linked into a binary. Unfortunately, this was tried early on in Rust's development and it was discovered that ld \-r is often poorly supported by OS toolchains on account of how seldom the feature is used. Furthermore, this inherits the same problems mentioned before regarding metadata and duplicate symbols.
* *Use an flag that doesn't carry a version number, like \-C rlib-format=platform.* This would be essentially the same as this RFC, but would not leave room for different versions in the future. For example, platforms might introduce new library formats in the future, or we might want to add some extra information to the .rlib format consumable by outside tools. In these cases, the ability to release a v1 version and beyond would be useful.
* *Instruct the linker to discard duplicate Rust symbols instead of emitting errors, and have external build systems use \-C crate-type=staticlib.* The COMDAT feature in the ELF format (exposed as linkonce\_odr in LLVM) can be used for this. This is what C++ does to avoid duplicate symbol errors when different object files include expansions of the same template. This solution gets the job done in practice, but it means that static libraries duplicate their dependencies, which results in extra needless I/O during the compilation (quadratic blow-up in the worst case). Moreover, it's inelegant.
* *Stabilize the contents of .rlib files in perpetuity.* This would prevent Rust from adopting MIR-only rlibs in the future, which are a commonly-discussed feature. The goal of this RFC isn't to hinder experimentation with alternative rlib formats.


## Prior art
[prior-art]: #prior-art

Discuss prior art, both the good and the bad, in relation to this proposal.
A few examples of what this can include are:

- For language, library, cargo, tools, and compiler proposals: Does this feature exist in other programming languages and what experience have their community had?
- For community proposals: Is this done by some other community and what were their experiences with it?
- For other teams: What lessons can we learn from what other communities have done here?
- Papers: Are there any published papers or great posts that discuss this? If you have some relevant papers to refer to, this can serve as a more detailed theoretical background.

This section is intended to encourage you as an author to think about the lessons from other languages, provide readers of your RFC with a fuller picture.
If there is no prior art, that is fine - your ideas are interesting to us whether they are brand new or if it is an adaptation from other languages.

Note that while precedent set by other languages is some motivation, it does not on its own motivate an RFC.
Please also take into consideration that rust sometimes intentionally diverges from common language features.

## Unresolved questions
[unresolved-questions]: #unresolved-questions

- What parts of the design do you expect to resolve through the RFC process before this gets merged?
- What parts of the design do you expect to resolve through the implementation of this feature before stabilization?
- What related issues do you consider out of scope for this RFC that could be addressed in the future independently of the solution that comes out of this RFC?

## Future possibilities
[future-possibilities]: #future-possibilities

Think about what the natural extension and evolution of your proposal would
be and how it would affect the language and project as a whole in a holistic
way. Try to use this section as a tool to more fully consider all possible
interactions with the project and language in your proposal.
Also consider how this all fits into the roadmap for the project
and of the relevant sub-team.

This is also a good place to "dump ideas", if they are out of scope for the
RFC you are writing but otherwise related.

If you have tried and cannot think of any future possibilities,
you may simply state that you cannot think of anything.

Note that having something written down in the future-possibilities section
is not a reason to accept the current or a future RFC; such notes should be
in the section on motivation or rationale in this or subsequent RFCs.
The section merely provides additional information.

# **References**

See [GitHub issue \#73632](https://github.com/rust-lang/rust/issues/73632) and [the pre-RFC Discourse thread](https://internals.rust-lang.org/t/pre-rfc-stabilize-a-version-of-the-rlib-format/17558) for the discussion that led up to this RFC.

# **Acknowledgements**

Thanks to Jeremy Fitzhardinge, Matt Hammerly, Dana Jansens, Augie Fackler, Marcel Hlopko, and bjorn3 for feedback on this RFC, and everyone who took part in GitHub issue \#73632 and the Discourse thread.
