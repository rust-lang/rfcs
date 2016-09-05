- Feature Name: more_link_kinds
- Start Date: 2016-02-05
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Adds a new `kind=better_static` (name entirely open to bikeshedding) that is used to link static libraries by passing them to the linker, unlike `kind=dylib` which is intended for dynamic libraries and `kind=static` which has rustc bundle it instead of passing it to the linker. Also adds a new `kind=object` (or `kind=obj` if you want) that is used to link object files.

# Motivation
[motivation]: #motivation

Rust currently does not expose the options necessary to have a static library properly linked by the linker. `kind=dylib` doesn't work because it informs Rust that the library is a dynamic library resulting in issues such as passing the library on to later linker invocations beyond the first immediate linker invocation resulting in symbol duplication, and on Windows it would cause dllimport to be emitted for all the symbols which is incorrect for static libraries. `kind=static` doesn't work because it causes rustc to bundle the library into the `*.rlib` instead of passing it along to the linker, which results in rustc looking for the library at compile time instead of leaving the job to the linker, which can result in the library not being found.

By adding `kind=better_static` Rust will be able to support passing a static library directly to the linker, thus allowing the library to be found in standard linker search paths, but at the same time still treating it as a static library, thus ensuring `dllimport` and `dllexport` are applied correctly.

Rust is also currently incapable of linking object files directly, so adding `kind=object` which expose the ability to do that as well. It would behave similarly to `kind=better_static` where it is passed along to the first linker invocation so it can take care of it.

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

When external code is linked into a Rust dynamic library `crate_type=dylib` via `kind=better_static` or `kind=object`, the Rust dynamic library must `dllexport` any symbols from the external code which are reachable (aka any public extern symbols as well as anything which can be referenced even transitively from a reachable inlinable/monomorphizable function).

When referencing the symbols within the same binary, `dllimport` will __not__ be applied to those symbols. Once it goes past a dynamic library boundary then `dllimport` will be applied.

## Example

* `foo.lib` is an external library. Can also be an external object file as `foo.obj`.
* `a.rlib` is a Rust rlib that depends on the native library `foo.lib`.
* `b.dll` is a Rust dylib that depends on `a.rlib`.
* `c.exe` is a Rust executable that depends on `b.dll`.

* If I specify `kind=static` `foo.lib` is bundled into `a.rlib` by `rustc` and __not__ passed to the linker invocations for `b.dll` and `c.exe`. `a.rlib` and `b.dll` do __not__ use `dllimport` for symbols they reference from `foo.lib`. `b.dll` will `dllexport` any symbols from `foo.lib` that are reachable and `c.exe` will `dllimport` any symbols from `foo.lib` that it uses.
* If I specify `kind=dylib` `foo.lib` is passed to the linker invocations for `b.dll` and `c.exe`. `a.rlib` `b.dll` and `c.exe` will all use `dllimport` when referencing symbols from `foo.lib` and `b.dll` will __not__ `dllexport` any of the symbols from `foo.lib`.
* If I specify `kind=better_static` `foo.lib` is passed to the linker invocation for `b.dll` but __not__ `c.exe`. `a.rlib` and `b.dll` do __not__ use `dllimport` for symbols they reference from `foo.lib`. `b.dll` will `dllexport` any symbols from `foo.lib` that are reachable and `c.exe` will `dllimport` any symbols from `foo.lib` that it uses.

# Drawbacks
[drawbacks]: #drawbacks

* It adds two more `kind`s that have to be supported and tested.

# Alternatives
[alternatives]: #alternatives

* Don't do this and make me very sad.
* Change the behavior of `kind=static`. Remove the bundling aspect and simply make it provide the knowledge to rustc that the symbols are static instead of dynamic. Since Cargo ensures the non-Rust static library will hang around until link time anyway, this would not really break anything for most people. Only a few people would be broken by this and it would be fairly easy to fix. Has the advantage of not adding another `kind`.

# Unresolved questions
[unresolved]: #unresolved-questions

* The name of the `kind`s. Please bikeshed vigorously.
