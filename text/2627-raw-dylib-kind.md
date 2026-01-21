- Feature Name: `raw_dylib_kind`
- Start Date: 2019-01-22
- RFC PR: [rust-lang/rfcs#2627](https://github.com/rust-lang/rfcs/pull/2627)
- Rust Issue: [rust-lang/rust#58713](https://github.com/rust-lang/rust/issues/58713)

## Summary
[summary]: #summary

Extend the `#[link]` attribute by adding a new kind `kind="raw-dylib"` for use on Windows which emits idata sections for the items in the attached `extern` block, so they may be linked against without linking against an import library. Also add a `#[link_ordinal]` attribute for specifying symbols that are actually ordinals.

## Motivation
[motivation]: #motivation

[dll]: https://en.wikipedia.org/wiki/Dynamic-link_library

Traditionally, to link against a [dll], the program must actually link against an import library. For example to depend on some symbols from `kernel32.dll` the program links to `kernel32.lib`. However, this requires that the correct import libraries be available to link against, and for third party libraries that are only distributed as a dll creating an import library can be quite difficult, especially given that `lib.exe` is incapable of creating an import library that links to `stdcall` symbols.

A real advantage of this feature, however, is the fact that symbols will be *guaranteed* to come from the specified dll. Currently, linking is a very finnicky process where if multiple libraries provide the same symbol the linker will choose one of them to provide the symbol and the user has little control over it. With `kind="raw-dylib"` the user is ensured that the symbol will come from the specified dll.

Sometimes, a crate may know exactly which dll it wants to link against, but which import library it ends up linking against is unknown. In particular the `d3dcompiler.lib` provided by the Windows SDK can link to several different versions of the d3dcompiler dll depending on which version of the Windows SDK the user has installed. `kind="raw-dylib"` would allow `winapi` to link to a specific version of that dll and ensure the symbols are correct for that version.

This would also allow `winapi` to not have to bundle import libraries for the `pc-windows-gnu` targets, saving on bandwidth and disk space for users.

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When trying to link to a Windows dll, the `dylib` kind may sometimes be unsuitable, and `kind="raw-dylib"` can be used instead. A central requirement of `kind="raw-dylib"` is that the dll has a stable ABI. Here are some examples of valid reasons to use `kind="raw-dylib"`:

* You've had it up to here with trying to create an import library for a dll that has `stdcall` functions.
* You're in linking hell with multiple import libraries providing the same symbol but from different dlls.
* You know exactly which dll you need a symbol from, but you don't know which version of the dll the import library is going to give you.
* You maintain `winapi`.

Here is an example of usage:

```rust
#[cfg(windows)]
#[link(name = "kernel32.dll", kind = "raw-dylib")]
#[allow(non_snake_case)]
extern "system" {
    fn GetStdHandle(nStdHandle: u32) -> *mut u8;
}
```

Some symbols are only exported by ordinal from the dll in which case `#[link_ordinal(..)]` may be used:

```rust
#[cfg(windows)]
#[link(name = "ws2_32.dll", kind = "raw-dylib")]
#[allow(non_snake_case)]
extern "system" {
    #[link_ordinal(116)]
    fn WSACleanup() -> i32;
}
```

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Add a new attribute `#[link_ordinal]` taking a single unsuffixed integer value, such as `#[link_ordinal(116)]`. It can only be specified on symbols in an extern block using `kind="raw-dylib"`.

Add a new possible value `raw-dylib` to the `kind` property of the `link` attribute. When this kind is specified, the `name` must explicitly include the extension. In addition, for all items in the associated extern block, Rust will *keep* the symbol mangled, instead of having an unmangled symbol. Rust will emit an idata section that maps from the *mangled* symbol to a symbol in the specified dll. The symbol in the dll that the idata section maps to depends on which attributes are specified on the item in question:

* If `#[link_ordinal]` is specified the idata section will map from the mangled symbol to the ordinal specified in the dll.
* If `#[link_name]` is specified the idata section will map from the mangled symbol to the name specified in the dll, without any calling convention decorations added. If calling convention decorations are desired they must be specified explicitly in the value of the `#[link_name]` attribute.
* If both `#[link_ordinal]` and `#[link_name]` are specified, an error will be emitted.
* If neither `#[link_ordinal]` nor `#[link_name]` are specified, the idata section will map from the mangled symbol to its unmangled equivalent in the dll. The unmangled symbol will *not* have calling convention decorations.
* If `#[no_mangle]` is specified an error will be emitted.

[idata section]: https://docs.microsoft.com/en-us/windows/desktop/debug/pe-format#the-idata-section
[import libraries]: https://docs.microsoft.com/en-us/windows/desktop/debug/pe-format#import-library-format

The [idata section] that is produced is equivalent to the idata sections found in [import libraries], and should result in identical code generation by the linker.

## Drawbacks
[drawbacks]: #drawbacks

Additional complexity in the language through a new `kind` and a new attribute for specifying ordinals.

## Rationale and alternatives
[alternatives]: #alternatives

The RFC as proposed would allow for full control over linking to symbols from dlls with syntax as close as possible to existing extern blocks.

No alternatives are currently known other than the status quo.

## Prior art
[prior-art]: #prior-art

Many non-native languages have the ability to import symbols from dlls, but this uses runtime loading by the language runtime and is not the same as what is being proposed here.

Delphi is a native language that has the ability to import symbols from dlls without import libraries.

## Unresolved questions
[unresolved]: #unresolved-questions

Whether there are any unresolved questions is an unresolved question.

## Future possibilities
[future-possibilities]: #future-possibilities

* With the features described in this RFC, we would be one step closer towards a fully standalone pure Rust target for Windows that does not rely on any external libraries (aside from the obvious and unavoidable runtime dependence on system libraries), allowing for easy installation and easy cross compilation.
    * If that were to happen, we'd no longer need to pretend the pc-windows-gnu toolchain is standalone, and we'd be able to stop bundling MinGW bits entirely in favor of the user's own MinGW installation, thereby resolving a bunch of issues such as [rust-lang/rust#53454](https://github.com/rust-lang/rust/issues/53454).
    * Also with that pure Rust target users would stop complaining about having to install several gigabytes of VC++ just to link their Rust binaries.
* A future extension of this feature would be the ability to optionally lazily load such external functions, since Rust would naturally have all the information required to do so. This would allow users to use functions that may not exist, and be able to write fallback code for older versions.
* Another future extension would be to extend this feature to support shared libraries on other platform, as they could also benefit from the ability to be more precise about linking. For example, on Linux and other platforms using ELF shared libraries, the compiler would emit an ELF `NEEDED` entry for the specified shared library name, and an undefined symbol for each function declared. (On ELF platforms, using the `link_ordinal` attribute would produce an error.) On such platforms, the `link_name` attribute may also specify a symbol name that includes a symbol version, including the `@@`.
    * Windows, however, should be the priority and figuring out details of support for other platforms should **not** block implementation and stabilization of this feature on Windows.
