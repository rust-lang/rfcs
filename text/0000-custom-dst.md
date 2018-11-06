- Feature Name: `custom_dst`
- Start Date: 2018-11-06
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Allow Rust code to define dynamically sized types with custom thick (and thin)
pointers, and define slice functions in terms of these, instead of transmute.
Also, convert the `CStr` type to use this functionality,
and make it a thin pointer; this will allow use with FFI.

# Motivation
[motivation]: #motivation

As of right now, the lack of custom DSTs in Rust means that we can't communicate
with C in important ways - we lack the ability to define a `CStr` in the
language such that `&CStr` is compatible with `char const *`,
and we lack the ability to communicate nicely with C code that uses Flexible
Array Members. This RFC attempts to fix this, as well as introduce more
correctness to existing practices.

Apart from FFI, it also has usecases for indexing and slicing 2-d arrays.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

There's a new language trait in the standard library, under `std::ops`:

```rust
unsafe trait DynamicallySized {
    type Metadata: 'static + Copy;

    fn size_of_val(&self) -> usize;
    fn align_of_val(&self) -> usize;
}
```

with an automatic implementation for all `Sized` types:

```rust
unsafe impl<T> DynamicallySized for T {
    type Metadata = ();

    fn size_of_val(&self) -> usize { size_of::<T>() }
    fn align_of_val(&self) -> usize { align_of::<T>() }
}
```

If you have a type which you would like to be unsized,
you can implement this trait for your type!

```rust
#[repr(C)]
struct CStr([c_char; 0]);

unsafe impl DynamicallySized for CStr {
    type Metadata = ();

    fn size_of_val(&self) -> usize { strlen(&self.0 as *const c_char) }
    fn align_of_val(&self) -> usize { 1 }
}
```

and automatically, your type will not implement `Sized`.

The existing `DynamicallySized` types will continue to work;
if one writes a `DynamicallySized` type `T`,
and then wraps `T` into a struct, they'll get the obvious semantics.

```rust
struct Foo {
    x: usize,
    y: CStr,
}

// size_of_val(&foo) returns size_of_val(&foo.y)
// same with align_of_val
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

In addition to the explanation given above,
we will also introduce four functions into the standard library,
in `std::raw`, which allow you to create and destructure these
pointers to `DynamicallySized` types:

```rust
mod std::raw {
    pub fn from_raw_parts<T: DynamicallySized>(
        ptr: *const (),
        meta: <T as DynamicallySized>::Metadata,
    ) -> *const T;

    pub fn from_raw_parts_mut<T: DynamicallySized>(
        ptr: *mut (),
        meta: <T as DynamicallySized>::Metadata,
    ) -> *mut T;

    pub fn metadata<T: DynamicallySized>(
        ptr: *const T,
    ) -> <T as DynamicallySized>::Metadata;

    pub fn ptr<T: DynamicallySized>(ptr: *const T) -> *const ();

    pub fn ptr_mut<T: DynamicallySized>(ptr: *mut T) -> *mut ();
}
```

Notes:
  - names of the above functions should be bikeshed
  - `extern type`s do not implement `DynamicallySized`, although in theory one
    could choose to do this (that usecase is not supported by this RFC).
  - `T: DynamicallySized` bounds imply a `T: ?Sized` bound.

We will also change `CStr` to have the implementation from above.

On an ABI level, we promise that pointers to any type with

```rust
size_of::<Metadata>() == 0
&& align_of::<Metadata>() <= align_of::<*const ()>()
```

are ABI compatible with a C pointer - this is important,
since we want to be able to write:

```rust
extern "C" {
    fn printf(fmt: &CStr, ...) -> c_int;
}
```

Unfortunately, we won't be able to change existing declarations in `libc`
without a new major version.


# Drawbacks
[drawbacks]: #drawback

- More complication in the language.
- Lack of a `Sized` type dual to these unsized types --
  the lack of a `[u8; N]` to these types' `[u8]` is unfortunate.
- Inability to define a custom DST safely

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This has been a necessary change for quite a few years.
The only real alternatives are those which are simply different ways of writing
this feature. We need custom DSTs.

# Prior art
[prior-art]: #prior-art

- [FAMs in C](https://en.wikipedia.org/wiki/Flexible_array_member)
- Existing Rust which could use this feature:
  - [CStr](https://doc.rust-lang.org/stable/std/ffi/struct.CStr.html)
  - [Pascal String](https://github.com/ubsan/epsilon/blob/master/src/string.rs#L11)
  - [Bit Vector](https://github.com/skiwi2/bit-vector/blob/master/src/bit_slice.rs)

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- How should these thick pointers be passed,
  if they are larger than two pointers?
- Are `std::raw::ptr` and `std::raw::ptr_mut` necessary?
  You can get this behavior with `as`.

# Future possibilities
[future-possibilities]: #future-possibilities

- By overloading `DerefAssign`, we could add a `BitReference` type
