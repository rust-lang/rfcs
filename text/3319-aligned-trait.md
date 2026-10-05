- Feature Name: `aligned`
- Start Date: 2022-09-24
- RFC PR: [rust-lang/rfcs#3319](https://github.com/rust-lang/rfcs/pull/3319)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary

Add an `Aligned` marker trait to `core::marker`, and the prelude, as a
supertrait of the `Sized` trait. `Aligned` is implemented for all types with an
alignment determined at compile time. This includes all `Sized` types, as well
as slices and records containing them. Relax `core::mem::align_of<T>()`'s trait
bound from `T: Sized` to `T: ?Sized + Aligned`.

# Motivation

Some data structures and containers can store unsized types only if their
alignment can be known at compile-time. Additionally, compile-time known
alignment can enable more efficient algorithms and additional APIs. A built-in
`Aligned` trait allows Rust code to implement APIs that that are fully usable
with all aligned types.

In addition, this RFC allows implementing certain object-safe traits for slices,
in a more complete fashion than was possible before.

## Case study: rustc `Aligned` trait

In rustc, a [manually-implemented version](https://github.com/rust-lang/rust/blob/a76ec181fba25f9fe64999ec2ae84bdc393560f2/compiler/rustc_data_structures/src/aligned.rs#L22-L25)
of the `Aligned` trait is used to support pointer tagging. A pointer to a type
with known alignment has log2(alignment) low bits available for a tag. Because
rustc uses pointer tagging with unsized types, [including custom DSTs](https://github.com/rust-lang/rust/blob/a76ec181fba25f9fe64999ec2ae84bdc393560f2/compiler/rustc_middle/src/ty/list.rs#L222-L232),
it needs to use a custom trait, along with potentially error-prone `unsafe`
impls.

## Case study: `unsized-vec`

The [`unsized-vec` crate](https://docs.rs/unsized-vec/0.0.2-alpha.7/unsized_vec/)
provides an analogue of the standard library `Vec<T>` that permits `T: ?Sized`.
The layout of the `UnsizedVec<T>` differs depending on whether `T`'s size or
alignment can be known at compile-time:

### `T: Sized`

If `T: Sized`, `UnsizedVec<T>` is a thin wrapper around `alloc::vec::Vec<T>`.

### `T: ?Sized`, alignment known at compile-time

In this case, `UnsizedVec<T>` contains two allocations. One allocation contains
the values of the vector, laid out end-to-end, aligned to `T`'s alignment. The
other allocation is an `alloc::vec::Vec`, of
`(ptr_metadata, offset_of_end_of_element_from_start_of_allocation)` pairs (one
for each element of the `UnsizedVec`). For example, an `UnsizedVec<[u8]>` might
look like this:

```text
Layout of `unsize_vec![[1, 2], [3, 4, 5], [], [6]]`
---------------------------------------------------

Main allocation (1 block = 1 byte)
⮦Address 0xDEADBEEF
┌──────┬──────┬──────┬──────┬──────╥──────┐
│ 0x01   0x02 │ 0x03   0x04   0x05 ║ 0x06 │
└──────┴──────┴──────┴──────┴──────╨──────┘

Metadata allocation (1 block = 1 word, "meta" = pointer metadata, "ofst" = offset)
┌──────┬──────┬──────┬──────┬──────┬──────┬──────┬──────┐
│meta 2 ofst 2│meta 3 ofst 5│meta 0 ofst 5│meta 1 ofst 6│
└──────┴──────┴──────┴──────┴──────┴──────┴──────┴──────┘
```

To retrieve a reference to the element at index `i` of the `UnsizedVec`, we
first index into the pairs `Vec`: once at index `i` to get the pointer metadata,
and once at index `i - 1` to get the offset of the start of element (if `i = 0`,
this offset is just `0`). We then offset the pointer to the main allocation by
the offset we just retrieved, and reconstruct the fat pointer to the
`UnsizedVec` element from that result + the pointer metadata. For example, to
retrieve element 1 (0-indexed) of the vec above, we would:

  1. Retrieve the 2nd metadata entry from the metadata allocation (value is 3);
  2. Retrieve the 2-1=1st offset entry from the metadata allocation (value is 2);
  3. Offset the address of the main allocation (`0xDEADBEEF`) by the result of
     step 2 to get the pointer to the element (address `0xDEADBEF1`);
  4. Combine that with the metadata from step 1 to construct the full fat
     pointer `(pointer = 0xDEADBEF1, metadata = 3)`.

### `T: ?Sized`, aligment known only at runtime

When `T`'s alignment is not known at compile-time, `UnsizedVec<T>`'s elements
can't just be placed end-to-end, as that would lead to them being improperly
aligned. To account for the varing alignments, we keep track of the largest
required alignment out of all the elements in the `UnsizedVec`, and ensure every
that every element is padded to that maximum. For example, consider this
`UnsizedVec<dyn Debug>`:

```text
Layout of `unsize_vec![3_u32, "hello", 42_u128]`
---------------------------------------------------

Maximum alignment: 16 bytes

Main allocation (1 block = 4 bytes)
⮦Address 0xDEADBEEF
┌──────┬──────┬──────┬──────┬──────┬──────┬──────┬──────┬──────┬──────┬──────┬──────┐
│ 3u32 │      padding       │ 64-bit ptr  │   padding   │          42_u128          │
└──────┴──────┴──────┴──────┴──────┴──────┴──────┴──────┴──────┴──────┴──────┴──────┘

Metadata allocation (1 block = 1 word, "vp" = vtable pointer, ofst = "offset")
┌──────┬──────┬──────┬──────┬──────┬──────┐
│vp u32 ofst16│vp&str ofst32│vpu128 ofst48│
└──────┴──────┴──────┴──────┴──────┴──────┘
```

Compared to the unsized+aligned case, we must add introduce an extra field to
our `UnsizedVec` in order to track alignment, and also need additional code to
manage the padding. We would prefer to pay that cost only when truly necessary;
only an `Aligned` trait provided by the compiler can make that possible.

# Guide-level explanation

`Aligned` is a marker trait defined in `core::marker`, and re-exported in the
prelude. It's automatically implemented for all types with an alignment
determined at compile time. This includes all `Sized` types (`Aligned` is a
supertrait of `Sized`), as well as slices and records containing them. Trait
objects are not `Aligned`.

You can't implement `Aligned` yourself.

To get the alignment of a type that implements `Aligned`, call
`core::mem::align_of<T>()`.

Implied `Sized` bounds also imply `Aligned`, because `Aligned` is a supertrait
of `Sized`. To bound a type parameter by `Aligned` only, write
`?Sized + Aligned`.

# Reference-level explanation

`Aligned` is not object-safe. Trait methods bounded by `Self: Aligned` can't be
called from a vtable, but don't affect the object safety of the trait as a
whole, just like `Self: Sized` currently. Relaxing `Self: Sized` bounds to
`Self: Aligned` allows implementing those methods for more `Self` types, while
preserving the trait's object safety.

`core::mem::offset_of!` supports any `Aligned` field.

# Drawbacks

- Slightly complicates situation around implied `Sized` bounds.
- May make certain object safety diagnostics more confusing, as they will now
  refer to the new, lesser-known `Aligned` trait instead of `Sized`.

# Rationale and alternatives

- `core::mem::align_of<T>()` for slices could be implemented with a library.
  However, a library would be unable to support records that contain a slice as
  the last field. Also, relaxing the trait dyn safety requirements can only be
  done with a language feature.
- `?Aligned` could be accepted as new syntax, equivalent to `?Sized`. However, I
  don't think it's worth it to have two ways to spell the exact same concept in
  the same edition.
- There may be a use-case for types that are `Sized` but not `Aligned`. However,
  I don't know of such, and allowing it would likely cause
  backward-compatibility issues.

# Prior art

In libraries:

- [`rustc`](https://github.com/rust-lang/rust/blob/f9a6b71580cd53dd4491d9bb6400f7ee841d9c22/compiler/rustc_data_structures/src/aligned.rs#L22)
- [`unsized-vec`](https://github.com/Jules-Bertholet/unsized-vec/blob/278befae4c08db42ff77461e9d7ce30eccf0c5bc/src/marker.rs#L16)

# Unresolved questions

- Should `Aligned` be `#[fundamental]`? (`Sized` is.)

# Future possibilities

- Relaxing `NonNull::<T>::dangling()`'s trait bound from `T: Sized` to
  `T: ?Sized + Aligned + Pointee<Metadata: ~const Default>` may be desirable
  once the necessary library and language features are stabilized.
- `extern type`s may want to be able to implement `Aligned`.
- In a future edition, `?Sized` could be replaced with `?Aligned`, with `?Sized`
  then meaning "opt out of `Sized` bound only, not `Aligned`."
- Certain `Self: Sized` bounds in the standard library could be relaxed to
  `Self: Aligned`. However, this might cause backward-compatibility issues.
  - [IRLO topic](https://internals.rust-lang.org/t/removing-self-sized-and-backward-compatibility/17456)
    on how the issues could be addressed.
- There has been discussion about adding other traits into the `Sized`
  hierarchy, like `DynSized`. If both `Aligned` and these other traits are
  integrated into Rust, their relative positions in the trait hierarchy will
  need to be determined.
