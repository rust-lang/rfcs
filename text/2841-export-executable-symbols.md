- Feature Name: export-executable-symbols
- Start Date: 2019-12-28
- RFC PR: [rust-lang/rfcs#2841](https://github.com/rust-lang/rfcs/pull/2841)
- Rust Issue: [rust-lang/rust#84161](https://github.com/rust-lang/rust/issues/84161)

# Summary
[summary]: #summary

Add the ability to export symbols from executables, not just dylibs, via a new
compiler flag: `-C export-executable-symbols`.

# Motivation
[motivation]: #motivation

Java and C# can't statically link against C/Rust code.  Both require dylib
symbols for their common native interop solution.  Which is fine if you let
their executables call your dylib, but is a problem if you want your Rust
executable to load a JVM instance, and let it call back into your executable.
You might want to do this to allow you to:
* Load multiple language runtimes into the same process (Rust + C# + Java + Lua anyone?  Only one of them can be the entry executable...)
* Display user-friendly error messages if language runtimes are missing (maybe even a download link!)
* [#[test] Java/Rust interop via cargo test.](https://github.com/MaulingMonkey/jerk/blob/04250c9d1b6ccc292eb27663f70919345c31007f/example-hello-world-jar/src/Global.rs)

For this last case, I
[manually export](https://github.com/MaulingMonkey/jerk/blob/04250c9d1b6ccc292eb27663f70919345c31007f/example-hello-world-jar/exports.def)
executable symbols via
[LINK](https://github.com/MaulingMonkey/jerk/blob/04250c9d1b6ccc292eb27663f70919345c31007f/example-hello-world-jar/build.rs#L4).
This is ugly, brittle, and rustc
[already knows](https://github.com/rust-lang/rust/blob/a916ac22b9f7f1f0f7aba0a41a789b3ecd765018/src/librustc_codegen_ssa/back/linker.rs#L706-L717)
how to do this automatically, across more platforms, and better.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

https://doc.rust-lang.org/rustc/codegen-options/index.html could gain:

```md
## export-executable-symbols

This flag causes `rustc` to export symbols from executables, as if they were dynamic libraries.

You might use this to allow the JVM or MSCLR to call back into your executable's
Rust code from Java/C# when embedding their runtimes into your Rust executable.
```

`rustc -C help` could gain:

```
    -C    export-executable-symbols -- export symbols from executables, as if they were dynamic libraries.
```

My Java interop [Quick Start](https://github.com/MaulingMonkey/jerk/blob/master/Readme.md#quick-start)
would start recommending a `.cargo/config` with:
```toml
[build]
rustflags = ["-C", "export-executable-symbols"]
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

On a technical level, this just involves preventing an early bailout when
calling `fn export_symbols` on executables with MSVC or GNU linker backends.
Other linker backends (EmLinker, WasmLd, PtxLinker) do not have this early
bailout in the first place, and remain unaffected.

# Drawbacks
[drawbacks]: #drawbacks

* Options bloat
* The burden of supporting a niche use-case in hideously platform specific code

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This is *very* simple to implement, leverages existing code to enable it to do exactly what it was meant to do, and has few drawbacks.

Alternatives:

- Unconditionally export symbols from executables instead of introducing a new compiler flag.
- Introduce a crate-level attribute instead of a compiler flag (`#![export_all_symbols]`? `#![export_symbols]`?)
- Write *yet another* cargo subcommand to install/remember for interop testing instead of using cargo test.
- Write interop tests exclusively as integration tests, in an entirely separate crate, that can load the testee as a dylib.
- Continue abusing LINK, writing a tool to auto-generate .defs via build scripts - possibly by reading metadata from other tools.
- Use nightly link-args instead of LINK, but still write a .def generator.
- Remember to always cargo build a dylib copy of a crate manually before cargo test ing, and load that instead.
  (That would also add a whole second copy of all functions and static vars in the same unit test process!)

# Prior art
[prior-art]: #prior-art

C and C++ compilers can already do this via `__declspec(dllexport)` annotations.
Most people don't really notice it, for good or for ill.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Is this a good name for it?
- Should it be more general and export when limit_rdylib_exports or crate_type == ProcMacro?

# Future possibilities
[future-possibilities]: #future-possibilities

We could introduce a new source annotation, `#[export]`.  For backwards
compatibility with current behavior, `#[no_mangle]` symbols could be exported
by default - and possibly disabled with `#[export(false)]`.  This would
reduce the need to hide this change to compiler/linker behavior behind a
compiler flag or crate annotation.

Maybe other options to control what symbols get exported?  Although I'd fear
turning rustc into yet another linker script implementation, so maybe not.

My own building atop this in the wider language ecosystem would be for improved
Java/Rust interop/testing, with the eventual goal of improved Android API
support for Rust.  Many APIs are only exposed via Java, and I'd like said APIs
to be usable in a safe and sound fashion.
