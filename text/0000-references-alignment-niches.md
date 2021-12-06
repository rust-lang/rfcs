- Feature Name: `references_alignment_niches`
- Start Date: 2021-12-06
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add new "alignment niches" to references (`&'a T`, `&'a mut T`), `Vec<T>`, and all smart pointer types, allowing better packing for `enum`s containing these types.

# Motivation
[motivation]: #motivation

Consider the following `enum`:

```rs
enum E<'a> {
    A(&'a u16),
    B,
    C,
}
```

Currently (on 64 bit targets), `mem::size_of::<E>() == 16`:
  - 8 bytes for the `&'a u16`;
  - 1 byte for the discriminant;
  - 7 bytes of padding.

However, this is suboptimal; because `&'a u16` must be well-aligned (in particular, 2-aligned), there are unused bit patterns that can be exploited to pack the `enum` into a single `usize`:
- `E::A(&'a u16)` stores the reference directly;
- `E::B` is stored as the bit pattern `0b0`;
- `E::C` is stored as the bit pattern `0b1`.

This RFC aims to introduces new niches to references types and smart pointer types, so that such `enum` optimizations become possible.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

"Pointer-like" containers (`&T`, `&mut T`, etc) gain new niches, corresponding to the scalar values strictly below `align(T)` and strictly above `align(T).wrapping_neg()`. For 1-aligned types, this is equivalent to the existing null niche.

The full list of affected types is: `&T, &mut T, Box<T>, Arc<T>, Rc<T>, {sync, rc}::Weak<T>, Vec<T>`.


This has the consequence that `enum`s of the form:
```rs
enum E {
  A(&T or &mut T or ...),
  B,
  C,
  // ...
}
```
where the total number of variants is less than or equal to `2*align(T)` may be optimized by the compiler to fit in the space of a single reference.

## Stability guarantees

Unlike null pointer optimization for `Option<&T>`-like types, this optimization isn't guaranteed by the Rust language. 

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## `&T` and `&mut T`

References must always be valid, non-null and well-aligned. This means that a reference's scalar value must lie in `align(T)..=align(T).wrapping_neg()`; values outside this range can be used as niches by the compiler.

## `Unique<T>`

To enable this optimization on `Box<T>` and `Vec<T>`, the validity invariant of the `std`-internal type `std::ptr::Unique<T>` is modified.

Introduce two special values `align` and `align_high` for the attributes `#[rustc_layout_scalar_valid_range_start/end]` (bikesheddable syntax). These values are only applicable to types with a single generic parameter `T`.
- `align` stands for `mem::align_of::<T>()`;
- `align_high` stands for `mem::align_of::<T>().wrapping_neg()`.

Then, tighten the invariant on `std::ptr::Unique<T>` to require that the wrapped pointer is correctly aligned, and add the following attributes, enabling the alignment niches:
```rs
#[rustc_layout_scalar_valid_range_start(align)]
#[rustc_layout_scalar_valid_range_end(align_high)]
```

## `Aligned<T>`

To handle other smart pointers (`Arc<T>`, `Rc<T>`, `Weak<T>`), a new `std`-internal type `std::ptr::Aligned<T>` is introduced.

This type behaves mostly like `NonNull<T>`, but also requires that the wrapped pointer is well-aligned, enabling the use of the attributes presented in the previous section.

`std` implementors can then replace their use of `NonNull<T>` with this type to enable alignment niches.

**Note:** `std::{rc, sync}::Weak<T>` uses a non-aligned pointer as a sentinel, this would need to be changed to use `min(2, align(T)).wrapping_neg()` instead.


# Drawbacks
[drawbacks]: #drawbacks

None known.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

An alternative would be to fully exploit the unused bit patterns in well-aligned references, and say that *all* unaligned bit patterns can be used as a niche: e.g. this would mean that `&'a u16` has a niche for `0`, and for every odd integer value.

While this massively increases the number of niches, and thus the maximum number of enum variants that can be optimized, this would require adding support for non-contiguous scalar value ranges to the compiler, a difficult task.

In comparison, this RFC builds upon existing support for contiguous scalar value ranges in the compiler, and requires only minimal changes to the compiler logic.

# Prior art
[prior-art]: #prior-art

None known.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Should the `Aligned<T>` type be part of `stdlib`'s public API? This would allow library authors (like `hashbrown`) to also exploit these niches.
- "size niches" could also be introduced: e.g. `&[u8; 100]` has no alignment niches, but has size niches above `100.wrapping_neg()` (thanks scottmcm for the idea!)
  - This somewhat complicates the validity invariant of `Aligned<T>`, is it acceptable?
  - This forces `Weak<T>` to drop back to using a `NonNull<T>`, because the sentinel value is no longer a valid `Aligned<T>`.

# Future possibilities
[future-possibilities]: #future-possibilities

Combined with smarter `enum` packing optimizations, this RFC (in the "all unaligned values are niches" form) could allow for automatic tagged-pointer-like `enum` layouts in some restricted cases, e.g.
```rs
enum E<'a> {
  A(&'a u32),
  B(u16),
  C(u16),
  D(u16),
}
```
Could have the following layout on 32-bit targets (and a similar one on 64-bit targets):
```
     [             &'a u32               ]
E::A: ******** ******** ******** ******00
     [       u16       ]               ^^
E::B: ******** ******** < pad. > 00000001
     [       u16       ]               ^^
E::C: ******** ******** < pad. > 00000010
     [       u16       ]               ^^
E::D: ******** ******** < pad. > 00000011
                                       ^^
                                 discriminant
```


