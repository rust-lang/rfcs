- Feature Name: dll_kind
- Start Date: 2018-06-27
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Extend the `#[link]` attribute by adding a new kind `kind="dll"` for use on Windows which emits idata sections for the items in the attached `extern` block, so they may be linked against without linking against an import library. Also add a `#[ordinal]` attribute for specifying symbols that are actually ordinals.

# Motivation
[motivation]: #motivation

Traditionally in order to link against a dll the program must actually link against an import library. For example to depend on some symbols from kernel32.dll the program links to kernel32.lib. However this requires that the correct import libraries be available to link against, and for third party libraries that are only distributed as a dll creating an import library can be quite difficult, especially given lib.exe is incapable of creating an import library that links to stdcall symbols.

A real advantage of this feature, however, is the fact that symbols will be *guaranteed* to come from the specified dll. Currently linking is a very finnicky process where if multiple libraries provide the same symbol the linker will choose one of them to provide the symbol and the user has very little control over it. With `kind="dll"` the user is ensured that the symbol will come from the specified dll.

Sometimes a crate may know exactly which dll it wants to link against, but which import library it ends up linking against is unknown. In particular the d3dcompiler.lib provided by the Windows SDK can link to several different versions of the d3dcompiler dll depending on which version of the Windows SDK the user has installed. `kind="dll"` would allow `winapi` to link to a specific version of that dll and ensure the symbols are correct for that version.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When trying to link to a Windows dll, the `dylib` kind may sometimes be unsuitable, and `kind="dll"` can be used instead:

```rust
#[cfg(windows)]
#[link(name = "kernel32.dll", kind = "dll")]
#[allow(non_snake_case)]
extern "system" {
    fn GetStdHandle(nStdHandle: u32) -> *mut u8;
}
```

Some symbols are only exported by ordinal from the dll in which case `#[ordinal]` may be used:

```rust
#[cfg(windows)]
#[link(name = "ws2_32.dll", kind = "dll")]
#[allow(non_snake_case)]
extern "system" {
    #[ordinal(116)]
    fn WSACleanup() -> i32;
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Add a new value `dll` to the `kind` property of the `link` attribute. When this kind is specified, Rust will *keep* the symbol mangled, instead of having an unmangled symbol. Rust will emit an idata section that maps from the *mangled* symbol to the unmangled symbol in the specified dll. The unmangled symbol will *not* have calling convention decorations. The `name` specified in the `#[link]` attribute when `kind = "dll"` is used *must* include the extension.

`#[ordinal]` is an attribute taking a single numerical value, such as `#[ordinal(116)]`. It can only be specified on symbols in an extern block using `kind = "dll"`. When specified, the idata section will map from the mangled symbol to the ordinal in the specified dll.

The attribute `#[link_name]` may be used on symbols in extern blocks using `kind="dll"`. When specified, the idata section will map from the mangled symbol to the symbol name specified in `#[link_name]` in the specified dll, without any calling convention decorations added. If calling convention decorations are desired they must be specified explicitly in the `#[link_name]` attribute.

# Drawbacks
[drawbacks]: #drawbacks

Additional complexity in the language through a new `kind` and a new attribute for specifying ordinals.

# Rationale and alternatives
[alternatives]: #alternatives

The RFC as proposed would allow for full control over linking to symbols from dlls with syntax as close as possible to existing extern blocks.

No alternatives are currently known other than the status quo.

# Prior art
[prior-art]: #prior-art

No native languages are known of that allow link time linking to symbols from dlls withot import libraries. Please note that this is distinct from runtime loading of dlls.

# Unresolved questions
[unresolved]: #unresolved-questions

* Bikeshedding on attribute names.
* Should this feature be extended to other platforms?

# Future possibilities
[future-possibilities]: #future-possibilities

With the features described in this RFC, we would be one step closer towards a fully standalone pure Rust target for Windows that does not rely on any external libraries (aside from the obvious and unavoidable runtime dependence on system libraries), allowing for easy installation and incredibly easy cross compilation.
