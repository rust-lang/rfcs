- Feature Name: better_static_kind
- Start Date: 2016-02-05
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Adds a new `kind=better_static` (name entirely open to bikeshedding) that is used to link static libraries by passing them to the linker, unlike `kind=dynamic` which is intended for dynamic libraries and `kind=static` which has rustc bundle it instead of passing it to the linker.

# Motivation
[motivation]: #motivation

There is currently no way to properly link a static library by passing it to the linker instead of bundling it with rustc. `kind=dynamic` doesn't work because it thinks the library is a dynamic library resulting in issues such as passing the library on to later linker invocations beyond the first immediate linker invocation and also on Windows, once dllimport is actually supported correctly, `kind=dynamic` would cause dllimport to be emitted for all the symbols which is incorrect for static libraries. `kind=static` doesn't work because instead of passing the library to the linker, rustc bundles it, which results in rustc not looking in the standard library paths for the library.

These issues have led to [confusion](https://github.com/rust-lang/rust/issues/27438) and [frustration](https://internals.rust-lang.org/t/meaning-of-link-kinds/2686) for me and other people find the behavior [weird](https://github.com/rust-lang/rust/issues/31419) as well.

# Detailed design
[design]: #detailed-design

`kind=better_static` can be applied the same way as any of the other `kind`s, whether via flags passed to cargo via build scripts or `#[link]` attributes.

The behavior is that when a library is given such a `kind`, `rustc` will __not__ look for that library itself (unlike `kind=static`). Instead it will trust that it exists and pass it to the first immediate linker invocation (but not to later downstream linker invocations unlike `kind=dynamic`).

## Example

* `a.rlib` is a Rust library that depends on native library `foo.lib`.
* `b.dll` is a dynamic Rust library that depends on `a.rlib`.
* `c.exe` is a Rust executable that depends on `b.dll`.

* If I specify `kind=static` `foo.lib` is bundled into `a.rlib` by `rustc` and __not__ passed to the linker invocations for `b.dll` and `c.exe`.
* If I specify `kind=dynamic` `foo.lib` is passed to the linker invocations for `b.dll` and `c.exe`.
* If I specify `kind=better_static` `foo.lib` is passed to the linker invocation for `b.dll` but __not__ `c.exe`.

## dllimport and dllexport

When a native library is linked into a Rust dynamic library (such as `b.dll` in the example above) via `kind=better_static`, the Rust dynamic library must `dllexport` any symbols from the native library which are reachable (aka any public function as well as anything which can be referenced from a reachable inline/monomorphizable function).

When referencing the symbols within the same binary (such as `a.rlib` or `b.dll` referencing `foo.lib` symbols), `dllimport` will __not__ be applied to those symbols. Once it goes past a dynamic library boundary (so if `c.exe` is trying to reference `foo.lib` symbols) then `dllimport` will be applied.

# Drawbacks
[drawbacks]: #drawbacks

*It adds another `kind` that has to be supported and tested.

# Alternatives
[alternatives]: #alternatives

*Don't do this and make me very sad.

# Unresolved questions
[unresolved]: #unresolved-questions

*The name of the `kind`. Please bikeshed vigorously.
