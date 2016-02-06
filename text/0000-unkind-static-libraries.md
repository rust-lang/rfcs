- Feature Name: better_static_kind
- Start Date: 2016-02-05
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Adds a new `kind=better_static` (name entirely open to bikeshedding) that is used to link static libraries by passing them to the linker, unlike `kind=dynamic` which is intended for dynamic libraries and `kind=static` which has rustc bundle it instead of passing it to the linker.

# Motivation
[motivation]: #motivation

Rust currently does not expose the options necessary to have a static library properly linked by the linker. `kind=dynamic` doesn't work because it informs Rust that the library is a dynamic library resulting in issues such as passing the library on to later linker invocations beyond the first immediate linker invocation resulting in symbol duplication, and on Windows it would cause dllimport to be emitted for all the symbols which is incorrect for static libraries. `kind=static` doesn't work because it causes rustc to bundle the library into the `*.rlib` instead of passing it along to the linker, which results in rustc not looking in the standard library paths for the library.

By adding `kind=better_static` Rust will be able to support passing a static library directly to the linker, thus allowing the library to be found in standard linker search paths, but at the same time still treating it as a static library, thus ensuring `dllimport` and `dllexport` are applied correctly.

## Related issues and discussions

* https://github.com/rust-lang/rust/issues/27438
* https://internals.rust-lang.org/t/meaning-of-link-kinds/2686
* https://github.com/rust-lang/rfcs/pull/1296
* https://github.com/rust-lang/rust/issues/31419

# Detailed design
[design]: #detailed-design

`kind=better_static` can be applied the same way as any of the other `kind`s, whether via flags passed to cargo via build scripts or `#[link]` attributes.

The behavior is that when a library is given such a `kind`, `rustc` will __not__ look for that library itself (unlike `kind=static`). Instead it will trust that it exists and pass it to the first immediate linker invocation (but not to later downstream linker invocations unlike `kind=dynamic`).

## dllimport and dllexport

When a native library is linked into a Rust dynamic library via `kind=better_static`, the Rust dynamic library must `dllexport` any symbols from the native library which are reachable (aka any public function as well as anything which can be referenced from a reachable inline/monomorphizable function).

When referencing the symbols within the same binary, `dllimport` will __not__ be applied to those symbols. Once it goes past a dynamic library boundary then `dllimport` will be applied.

## Example

* `a.rlib` is a Rust library that depends on native library `foo.lib`.
* `b.dll` is a dynamic Rust library that depends on `a.rlib`.
* `c.exe` is a Rust executable that depends on `b.dll`.

* If I specify `kind=static` `foo.lib` is bundled into `a.rlib` by `rustc` and __not__ passed to the linker invocations for `b.dll` and `c.exe`. `a.rlib` and `b.dll` do __not__ use `dllimport` for symbols they reference from `foo.lib`. `b.dll` will `dllexport` any symbols from `foo.lib` that are reachable and `c.exe` will `dllimport` any symbols from `foo.lib` that it uses.
* If I specify `kind=dynamic` `foo.lib` is passed to the linker invocations for `b.dll` and `c.exe`. `a.rlib` `b.dll` and `c.exe` will all use `dllimport` when referencing symbols from `foo.lib` and `b.dll` will __not__ `dllexport` any of the symbols from `foo.lib`.
* If I specify `kind=better_static` `foo.lib` is passed to the linker invocation for `b.dll` but __not__ `c.exe`. `a.rlib` and `b.dll` do __not__ use `dllimport` for symbols they reference from `foo.lib`. `b.dll` will `dllexport` any symbols from `foo.lib` that are reachable and `c.exe` will `dllimport` any symbols from `foo.lib` that it uses.

## Details of linking on Windows

On Windows, in the MSVC world, the only kind of library you ever link to is a `foo.lib` library. This may either be a static library or an import library for a DLL (theoretically the library could contain both static symbols and dynamic imports but that is typically rare). MinGW can also sometimes link to a DLL directly.

When linking to symbols from an import library, `dllimport` needs to be applied to the symbols, otherwise the generated code is less than ideal and it may even fail to link. When linking to symbols from a static library, `dllimport` should not be applied to the symbols, otherwise the generated code is less than ideal and it may even fail to link. Here is a table of various combinations of `dllimport` and their results on linking with MSVC.

Library type | Static | Function | Result
------------ | ------ | -------- | ------
Dynamic | No | No | Success
Dynamic | Plain | No | Error
Dynamic | Dllimport | No | Success
Dynamic | No | Plain | Success
Dynamic | Plain | Plain | Error
Dynamic | Dllimport | Plain | Success
Dynamic | No | Dllimport | Success
Dynamic | Plain | Dllimport | Error
Dynamic | Dllimport | Dllimport | Success
Static | No | No | Success
Static | Plain | No | Success
Static | Dllimport | No | Error
Static | No | Plain | Success
Static | Plain | Plain | Success
Static | Dllimport | Plain | Warning
Static | No | Dllimport | Error
Static | Plain | Dllimport | Warning
Static | Dllimport | Dllimport | Error

# Drawbacks
[drawbacks]: #drawbacks

* It adds another `kind` that has to be supported and tested.

# Alternatives
[alternatives]: #alternatives

* Don't do this and make me very sad.
* Change the behavior of `kind=static`. Would have poor backwards compatibility though.

# Unresolved questions
[unresolved]: #unresolved-questions

* The name of the `kind`. Please bikeshed vigorously.
