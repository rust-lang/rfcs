- Feature Name: `c_void_reunification`
- Start Date: 2018-08-02
- RFC PR: [rust-lang/rfcs#2521](https://github.com/rust-lang/rfcs/pull/2521)
- Rust Issue: [rust-lang/rust#53856](https://github.com/rust-lang/rust/issues/53856)

# Summary
[summary]: #summary

Unify `std::os::raw::c_void` and `libc::c_void` by making them both re-exports
of a definition in libcore.


# Motivation
[motivation]: #motivation

`std::os::raw::c_void` and `libc::c_void` are different types:

```rust
extern crate libc;

fn allocate_something() -> *mut std::os::raw::c_void {
    unimplemented!()
}

fn foo() {
    let something = allocate_something();
    // ...
    libc::free(something)
}
```
```rust
error[E0308]: mismatched types
  --> a.rs:10:16
   |
10 |     libc::free(something)
   |                ^^^^^^^^^ expected enum `libc::c_void`, found enum `std::os::raw::c_void`
   |
   = note: expected type `*mut libc::c_void`
              found type `*mut std::os::raw::c_void`

error: aborting due to previous error
```

There is no good reason for this, the program above should compile.

Note that having separate definitions is not as much of a problem for other `c_*` types
since they are `type` aliases. `c_int` *is* `i32` for example,
and separate aliases with identical definitions are compatible with each other in the type system.
`c_void` however is currently defined as an `enum` (of size 1 byte, with semi-private variants),
and two `enum` types with identical definitions are still different types.

This has been extensively discussed already:

* [Issue #31536: std `c_void` and libc `c_void` are different types](https://github.com/rust-lang/rust/issues/31536)
* [Internals #3268: Solve `std::os::raw::c_void`](https://internals.rust-lang.org/t/solve-std-os-raw-c-void/3268)
* [Issue #36193: Move std::os::raw to libcore?](https://github.com/rust-lang/rust/issues/36193)
* [RFC #1783: Create a separate libc_types crate for basic C types](https://github.com/rust-lang/rfcs/pull/1783)
* [Issue #47027: Types in std::os::raw should be same as libc crate](https://github.com/rust-lang/rust/issues/47027)
* [Internals #8086: Duplicate std::os::raw in core?](https://internals.rust-lang.org/t/duplicate-std-raw-in-core/8086)
* [PR #52839: Move std::os::raw into core](https://github.com/rust-lang/rust/pull/52839)


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

With this RFC implemented in both the standard library and in the `libc` crate,
`std::os::raw::c_void` and `libc::c_void` are now two ways to name the same type.

If two independent libraries both provide FFI bindings to C functions that involve `void*` pointers,
one might use `std` while the other uses `libc` to access the `c_void` type in order to expose
`*mut c_void` in their respective public APIs.
A pointer returned from one library can now be passed to the other library without an `as` pointer cast.

`#![no_std]` crates can now also access that same type at `core::ffi::c_void`.


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

In the standard library:

* Create a new `core::ffi` module.
* Move the `enum` definition of `c_void` there.
* In `c_void`’s former location (`std::os::raw`), replace it with a `pub use` reexport.
* For consistency between `core` and `std`, also add a similar `pub use` reexport at `std::ffi::c_void`.
  (Note that the `std::ffi` module already exists.)

Once the above lands in Nightly, in the `libc` crate:

* Add a build script that detects the existence of `core::ffi::c_void`
  (for example by executing `$RUSTC` with a temporary file like
  `#![crate_type = "lib"] #![no_std] pub use core::ffi::c_void;`)
  and conditionally set a compilation flag for the library.
* In the library, based on the presence of that flag,
  make `c_void` be either `pub use core::ffi::c_void;` or its current `enum` definition,
  to keep compatibility with older Rust versions.


# Drawbacks
[drawbacks]: #drawbacks

This proposal is a breaking change for users who implement a trait of theirs like this:

```rust
trait VoidPointerExt {…}
impl VoidPointerExt for *mut std::os::raw::c_void {…}
impl VoidPointerExt for *mut libc::c_void {…}
```

With the two `c_void` types being unified, the two `impl`s would overlap and fail to compile.

Hopefully such breakage is rare enough that we can manage it.
Rarity could be evaluated with Crater by either:

* Adding support to Crater if it doesn’t have it already
  for adding a `[patch.crates-io]` section to each root `Cargo.toml` being tested,
  in order to test with a patched `libc` crate in addition to a patched Rust.

* Or speculatively landing the changes in `libc` and publishing them in crates.io
  before landing them in Rust


# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

`libc` cannot reexport `std::os::raw::c_void`
because this would regress compatibility with `#![no_std]`.

[RFC #1783](https://github.com/rust-lang/rfcs/pull/1783) proposed adding
to the standard library distribution a new crate specifically for the C-compatible types.
Both `std` and `libc` would depend on this crate.

This was apparently in response to reluctance about having operating-system-dependant definitions
(such as for `c_long`) in libcore.
This concern does not apply to `c_void`, whose definition is the same regardless of the target.
However there was also reluctance to having an entire crate for so little functionality.

That RFC was closed / postponed with this explanation:

> The current consensus is to offer a canonical way of producing
> an "unknown, opaque type" (a better c_void), possible along the lines of
> [#1861](https://github.com/rust-lang/rfcs/pull/1861)

RFC 1861 for `extern` types is now being implemented, but those types are `!Sized`.
Changing `c_void` from `Sized` to `!Sized` would be a significant breaking change:
for example, `ptr::null::<c_void>()` and `<*mut c_void>::offset(n)` would not be usable anymore.

We could deprecated `c_void` and replace it with a new differently-named extern type,
but forcing the ecosystem through that transition seems too costly for this theoretical nicety.
Plus, this would still be a nominal type.
If this new type is to be present if both `libc` and `std`,
 it would still have to be in `core` as well.


# Unresolved questions
[unresolved-questions]: #unresolved-questions

What is the appropriate location for `c_void` in libcore?

This RFC proposes `core::ffi` rather than `core::os::raw`
on the basis that C-compatible types are misplaced in `std::os::raw`.
`std::os` is documented as “OS-specific functionality”,
but everything currently available under `std::os::raw` is about interoperabily with C
rather than operating system functionality.
(Although the exact definition of `c_char`, `c_long`, and `c_ulong` does vary
based on the target operating system.)
FFI stands for Foreign Function Interface and is about calling or being called from functions
in other languages such as C.
So the `ffi` module seems more appropriate than `os` for C types, and it already exists in `std`.

Following this logic to this conclusion,
perhaps the rest of `std::os::raw` should also move to `std::ffi` as well,
and the former module be deprecated eventually.
This is left for a future RFC.

This RFC does not propose any change such as moving to libcore for the C types other than `c_void`.

Although some in previous discussions have expressed desire for using C-compatible types
without linking to the C runtime library (which the `libc` crate does) or depending on `std`.
This use case is also left for a future proposal or RFC.
