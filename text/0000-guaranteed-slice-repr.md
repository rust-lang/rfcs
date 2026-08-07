- Feature Name: guaranteed_slice_repr
- Start Date: 2025-02-18
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC guarantees the in-memory representation of slice and str references.
Specifically, `&[T]` and `&mut [T]` are guaranteed to have the same layout as:

```rust
#[repr(C)]
struct Slice<T> {
    data: *const T,
    len: usize,
}
```

The layout of `&str` is the same as that of `&[u8]`, and the layout of
`&mut str` is the same as that of `&mut [u8]`.

# Motivation
[motivation]: #motivation

This RFC allows non-Rust (e.g. C or C++) code to read from or write to existing
slices and to declare slice fields or locals.

For example, guaranteeing the representation of slice references allows
non-Rust code to read from the `data` or `len` fields of `string` in the type
below without intermediate FFI calls into Rust:

```rust
#[repr(C)]
struct HasString {
    string: &'static str,
}
```

Note: prior to this RFC, the type above is not even properly `repr(C)` since the
size and alignment of slices were not guaranteed. However, the Rust compiler
accepts the `repr(C)` declaration above without warning.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Slice references are represented with a pointer and length pair. Their in-memory
layout is the same as a `#[repr(C)]` struct like the following:

```rust
#[repr(C)]
struct Slice<T> {
    data: *const T,
    len: usize,
}
```

The precise ABI of slice references is not guaranteed, so `&[T]` may not be
passed by-value or returned by-value from an `extern "C" fn`.

The validity requirements for the in-memory representation of slice references
are the same as [those documented on `std::slice::from_raw_parts`](https://doc.rust-lang.org/std/slice/fn.from_raw_parts.html) for shared slice references, and
[those documented on `std::slice::from_raw_parts_mut`](https://doc.rust-lang.org/std/slice/fn.from_raw_parts_mut.html)
for mutable slice references.

Namely:

* `data` must be non-null, valid for reads (for shared references) or writes
  (for mutable references) for `len * mem::size_of::<T>()` many bytes,
  and it must be properly aligned. This means in particular:

    * The entire memory range of this slice must be contained within a single allocated object!
      Slices can never span across multiple allocated objects.
    * `data` must be non-null and aligned even for zero-length slices or slices of ZSTs. One
      reason for this is that enum layout optimizations may rely on references
      (including slices of any length) being aligned and non-null to distinguish
      them from other data. You can obtain a pointer that is usable as `data`
      for zero-length slices using [`NonNull::dangling()`].

* `data` must point to `len` consecutive properly initialized values of type `T`.

* The total size `len * mem::size_of::<T>()` of the slice must be no larger than `isize::MAX`,
  and adding that size to `data` must not "wrap around" the address space.
  See the safety documentation of [`pointer::offset`].

## `str`

The layout of `&str` is the same as that of `&[u8]`, and the layout of
`&mut str` is the same as that of `&mut [u8]`. More generally, `str` behaves like
`#[repr(transparent)] struct str([u8]);`. Safe Rust functions may assume that
`str` holds valid UTF8, but [it is not immediate undefined-behavior to store
non-UTF8 data in `str`](https://doc.rust-lang.org/std/primitive.str.html#invariant).

## Pointers

Raw pointers to slices such as `*const [T]` or `*mut str` use the same layout
as slice references, but do not necessarily point to anything.

# Drawbacks
[drawbacks]: #drawbacks

## Zero-sized types

One could imagine representing `&[T]` as only `len` for zero-sized `T`.
This proposal would preclude that choice in favor of a standard representation
for slices regardless of the underlying type.

Alternatively, we could choose to guarantee that the data pointer is present if
and only if `size_of::<T> != 0`. This has the possibility of breaking exising
code which smuggles pointers through the `data` value in `from_raw_parts` /
`into_raw_parts`.

## Uninhabited types

Similarly, we could be *extra* tricky and make `&[!]` or other `&[Uninhabited]`
types into a ZST since the slice can only ever be length zero.

If we want to maintain the pointer field, we could also make `&[!]` *just* a
pointer since we know the length can only be zero.

Either option may offer modest performance benefits for highly generic code
which happens to create empty slices of uninhabited types, but this is unlikely
to be worth the cost of maintaining a special case.

## Compatibility with C++ `std::span`

The largest drawback of this layout and set of validity requirements is that it
may preclude `&[T]` from being representationally equivalent to C++'s
`std::span<T, std::dynamic_extent>`.

* `std::span` does not currently guarantee its layout. In practice, pointer + length
  is the common representation. This is even observable using `is_layout_compatible`
  [on MSVC](https://godbolt.org/z/Y8ardrshY), though not
  [on GCC](https://godbolt.org/z/s4v4xehnG) nor
  [on Clang](https://godbolt.org/z/qsd1K5oGq). Future changes to guarantee a
  different layout in the C++ standard (unlikely due to MSVC ABI stabilitiy
  requirements) could preclude matching the layout with `&[T]`.

* Unlike Rust, `std::span` allows the `data` pointer to be `nullptr`. One
  possibile workaround for this would be to guarantee that `Option<&[T]>` uses
  `data: std::ptr::null(), len: 0` to represent the `None` case, making
  `std::span<T>` equivalent to `Option<&[T]>` for non-zero-sized types.

  Note that this is not currently the case. The compiler currenty represents
  `None::<&[u8]>` as `data: std::ptr::null(), len: uninit` (though this is
  not guaranteed).

* Rust uses a dangling pointer in the representation of zero-length slices.
  It's unclear whether C++ guarantees that a dangling pointer will remain
  unchanged when passed through `std::span`. However, it does support
  dangling pointers during regular construction via the use of
  [`std::to_address`](https://en.cppreference.com/w/cpp/container/span/span)
  in the iterator constructors.

Note that C++ also does not support zero-sized types, so there is no naive way
to represent types like `std::span<SomeZeroSizedRustType>`.

## Flexibility

Additionally, guaranteeing layout of Rust-native types limits the compiler's and
standard library's ability to change and take advantage of new optimization
opportunities.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

* We could avoid committing to a particular representation for slices.

* We could try to guarantee layout compatibility with a particular target's
  `std::span` representation, though without standardization this may be
  impossible. Multiple different C++ stdlib implementations may be used on
  the same platform and could potentially have different span representations.
  In practice, current span representations also use ptr+len pairs.

* We could avoid storing a data pointer for zero-sized types. This would result
  in a more compact representation but would mean that the representation of
  `&[T]` is dependent on the type of `T`. Additionally, this would break
  existing code which depends on storing data in the pointer of ZST slices.

  This would break popular crates such as [bitvec](https://docs.rs/crate/bitvec/1.0.1/source/doc/ptr/BitSpan.md)
  (55 million downloads) and would result in strange behavior such as
  `std::ptr::slice_from_raw_parts(ptr, len).as_ptr()` returning a different
  pointer from the one that was passed in.

  Types like `*const ()` / `&()` are widely used to pass around pointers today.
  We cannot make them zero-sized, and it would be surprising to make a
  different choice for `&[()]`.
  

# Prior art
[prior-art]: #prior-art

The layout in this RFC is already documented in
[the Unsafe Code Guildelines Reference.](https://rust-lang.github.io/unsafe-code-guidelines/layout/pointers.html)

# Future possibilities
[future-possibilities]: #future-possibilities

* Consider defining a separate Rust type which is repr-equivalent to the platform's
  native `std::span<T, std::dynamic_extent>` to allow for easier
  interoperability with C++ APIs. Unfortunately, the C++ standard does not
  guarantee the layout of `std::span` (though the representation may be known
  and fixed on a particular implementation, e.g. libc++/libstdc++/MSVC).
  Zero-sized types would also not be supported with a naive implementation of
  such a type.
