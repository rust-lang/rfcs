- Feature Name: `references_alignment_niches`
- Start Date: 2021-12-06
- RFC PR: [rust-lang/rfcs#3204](https://github.com/rust-lang/rfcs/pull/3204)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add new "alignment niches" and "size niches" to references (`&'a T`, `&'a mut T`), all smart pointer types, and most collections types, allowing better packing for `enum`s containing these types.  

Add a new `core::ptr::WellFormed<T>` pointer type so that data-structures from third-party libraries can benefit from these niches.

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

"Pointer-like" containers (`&T`, `&mut T`, etc) gain new niches, corresponding to following scalar ranges:

-  `..align_of::<T>()`; this includes the already-existing null niche.
-  `size_of::<T>().wrapping_neg()..`.

Special cases apply to dynamically-sized and zero-sized types, see the reference-level section for more details.

The full list of affected types is: `&T, &mut T, Box<T>, Arc<T>, Rc<T>, {sync, rc}::Weak<T>, Vec<T>, VecDeque<T>, BTreeMap<K, V>, HashMap<K, V>`.

Note that unlike null pointer optimization for `Option<&T>`-like types, this optimization is best-effort only, and isn't guaranteed by the Rust language.

## `WellFormed<T>`

To allow `std` and third-party crates to exploit these new niches, a new pointer-like type is added: `core::ptr::WellFormed<T>`. It is the preferred type to represent references to objects with unmanaged lifetimes.

A `WellFormed<T>` has the same layout as a reference (including all niches), but doesn't carry any lifetime information; it provides the following (unstable) API:

``` rust
// Covariant in T, like NonNull<T>
pub struct WellFormed<T>(...);

impl<T: ?Sized> WellFormed<T> {
    // SAFETY: ptr must point to some (possibly deallocated)
    // region of memory that is valid for an instance of T.
    pub const unsafe fn new_unchecked(ptr: *mut T) -> Self { ... }
    pub const fn as_ptr(self) -> *mut T { ... }
    // SAFETY: self must point to an initialized T, valid for the chosen lifetime.
    pub unsafe fn as_ref<'a>(&self) -> &'a T { ... }
    // SAFETY: self must point to an initialized T, valid for the chosen lifetime.
    pub unsafe fn as_mut<'a>(&self) -> &'a mut T { ... }
    // Equivalent to std::mem::size_of_val_raw, but *always safe*.
    pub fn size_of_val(self) -> usize { ... }
    // Equivalent to std::mem::align_of_val_raw, but *always safe*.
    pub fn align_of_val(self) -> usize { ... }
}

// Not Send or Sync
impl<T: ?Sized> !Send for WellFormed<T> {}
impl<T: ?Sized> !Sync for WellFormed<T> {}

// Safe conversion from references
impl<'a, T: ?Sized> From<&'a T> for WellFormed<T> { ... }
impl<'a, T: ?Sized> From<&'a mut T> for WellFormed<T> { ... }

// Safe conversion to NonNull
impl<T: ?Sized> From<WellFormed<T>> for NonNull<T> { ... }

// Elided - implementations of standard traits:
// Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash,
// UnwindSafe, CoerceUnsized, DispatchFromDyn, Pointer
```

# Reference-level explanation

[reference-level-explanation]: #reference-level-explanation

## New niches for references

References (to a type `T`) must always be valid, non-null and well-aligned. This means that a reference's scalar value is restricted, and has the following properties:

- for any type `T`, `&T` is always non-null;

- for any `T`, `&T` and `&[T]` are always well aligned (as given by `align_of_val::<T>`);

- for any `T`, offsetting a `&T` by `size_of_val::<T>` bytes doesn't wrap around the address space.

This implies that the scalar values of references always lie in the following ranges, and that values outside these ranges can be used as niches by the compiler:

|         Type         | Lower bound (inclusive) | Upper bound (inclusive) |
| :------------------: | :---------------------: | :---------------------: |
|   sized types `T`    |    `align_of::<T>()`    |  `-size_of::<T>() - 1`  |
| zero-sized types `T` |    `align_of::<T>()`    |   `-align_of::<T>()`    |
|     slices `[T]`     |    `align_of::<T>()`    |   `-align_of::<T>()`    |
|      other DSTs      |           `1`           |          None           |

For "compound" unsized types with a sized prefix, the valid range is the intersection of the ranges of the sized part and the unsized prefix.

Additionally, fat pointer metadata is always treated as having no niches, even if invalid values exist that could be exploited (e.g. for `&dyn Trait`, the metadata must be a valid reference to some trait vtable).

These niches are applied to `&T` and `&mut T`, and also to `WellFormed<T>` and `Unique<T>` via a new perma-unstable attribute `#[rustc_layout_reference]`.

## Invariants of `WellFormed<T>`

A `WellFormed<T>`, having the same niches as a plain reference `&T`, should have a safety invariant compatible with these niches.

### Safety invariant

To shield the programmer from the exact bit-level niches of `WellFormed<T>` (which may evolve in the future, or become platform-specific), we require that every `WellFormed<T>` point to some (maybe deallocated) memory region. More precisely, users calling `WellFormed::<T>::new` must ensure the following:

- The pointer is non-null.
- For fat pointers, the metadata is valid (as described in `mem::{size, align}_of_val_raw`).
- The pointer points to a region of memory that has suitable size and alignment (it must fit the layout returned by `Layout::from_value_raw`). There is no constraint placed upon the "liveness" of this memory; it may be deallocated while the `WellFormed<T>` exists, or already deallocated when the `WellFormed` is created.

### Thin DSTs

The safety invariant outlined in the previous section implicitly assumes that calling `mem::{size, align}_of_val_raw` isn't UB (this is witnessed by the present of the corresponding safe methods on `WellFormed`). This is true of all dynamically-sized types in current Rust, however it is incompatible with hypothetical thin DSTs which would have to store layout information in the pointed-to value.

There are three ways to resolve this:

- Say that  `mem::size_of_val_raw() == 0` and `mem::align_of_val_raw() == 1` for thin DSTs (like extern types currently behave).
- Make `WellFormed::{size, align}_of_val` unsafe, and treat thin DSTs as having `size: 0, align: 1` for the purposes of the `WellFormed` invariant.
- Never add thin DSTs to the language.

## Layout algorithm changes

Directly making the layout of references types dependent on the layout of the referenced type is impossible, as this would cause cycles when layouting recursive types.

**Example:**  

```rust
enum List {
    Cons(i32, Box<List>),
    Nil,
}
```

Computing the layout of `List` causes the following cycle:

- `layout_of(List)`
- requires `layout_of(Box<List>)`
- requires `size_of(List)` and `align_of(List)`, for determining niches
- requires `layout_of(List)`, completing the cycle.

To break this cyclic dependency, we introduce a new `min_layout_of` query that computes an underestimate of the size and alignment of a type, without having to recurse on reference types. Reference types can then use this new query to determine available alignment niches.

### The `min_layout_of` algorithm 

At a high-level, this is the "full" layout algorithm, with the following simplifications:

- We don't care about layout details (field order, ABI, etc), only about the total size and alignment.
- We treat any non-empty niche, regardless of its size, as being able to fit an arbitrary number of discriminants.

```rust
struct MinLayout {
    min_size: usize,	// Size lower-bound
    min_align: usize,	// Alignment lower-bound
    has_niches: bool,	// Does the layout contains exploitable niches?
}

fn min_layout_of(ty: &Ty) -> MinLayout {
    // This doesn't actually correspond to the structures used in rustc,
    // but let's keep this simple.
    match ty {
        Primitive(_) => /*
        	Returns the layout for each primitive; note that
        	bool and char have `has_niches: true`.
        */,
        Reference(_) | RawPointer(_) => /*
        	Return the layout for a reference:
        	- One or two usizes, depending on pointee Sized-ness
        	- Always has at least one niche (null, at minimum)
        */,
        Struct(_) | Tuple(_) => /*
        	- Concatenate (summing sizes, max'ing alignments)
        	    the MinLayout of each field
        		- If repr(C), add padding between fields.
        	- If repr(packed) or repr(align), set the correct alignment
        	- Pad the final size to the next multiple of the alignment
        */,
        Enum(_) => /*
        	- Merge (max'ing sizes and alignments) the MinLayout of each variant
        	- If repr(i/uN) or if no variant has niches, the enum has an
        	  explicit discriminant field; concatenate its MinLayout
        	- Set has_niches to true
        	
        	The underestimation happens here: if the available non-empty niches
        	end up not big enough to fit the discriminant, the enum will be
        	bigger than predicted.
        */,
        Union(_) => /*
			- Merge (max'ing sizes and alignments) the MinLayout of each variant
        	- Set has_niches to false
        */
    }
}
```


# Drawbacks
[drawbacks]: #drawbacks

This introduces a slighy increase in layout calculation due to the `min_layout_of` step; additionally, rustc must guarantee that `min_layout_of` always returns a `MinLayout` compatible with the full layout.

This also introduces a new pointer type, `WellFormed<T>`, with somewhat extensive API duplication between it and other pointer types.

Depending on the chosen invariant for `WellFormed<T>`, this also precludes the addition of thin DSTs with "correct" `{size, align}_of_val`; however, such thin DSTs already have other issues regardless of this RFC (for example, their interaction with `UnsafeCell`).

# Alternatives
[alternatives]: #alternatives

## Non-contiguous niches

An alternative would be to fully exploit the unused bit patterns in well-aligned references, and say that *all* unaligned bit patterns can be used as a niche: e.g. this would mean that `&'a u16` has a niche for `0`, `-2`, and every odd integer value.

While this massively increases the number of niches, and thus the maximum number of enum variants that can be optimized, this would require adding support for non-contiguous scalar value ranges to the compiler, a difficult task.

In comparison, this RFC builds upon existing support for contiguous scalar value ranges in the compiler, and requires less changes to the compiler logic.

## `Aligned<T>`

As an alternative to `WellFormed<T>`,  an `Aligned<T>` type (representing a non-null aligned pointer) could be used instead, with the following safety invariant:

- Must always be non-null;
- For fat pointers, metadata must always be valid;
- Must be aligned to the alignment given by `std::mem::align_of_val_raw`.

This makes for a simpler invariant (no need to talk about deallocated memory regions) and slightly simpler layout computation (we only need to compute `min_align`, not the full `MinLayout`), but comes with several disadvantages:

- Loss of "size niches" (e.g. `-2` for `u16`) and future "restricted ranges niches".
- This fixes the exact niche layout of `WellFormed<T>`, preventing the addition of more niches in the future.

# Prior art
[prior-art]: #prior-art

None known.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Bikeshedding the name of `WellFormed<T>`; possible alternatives are `ValidPtr<T>`, `UnsafeRef<T>`, `&'unsafe T` (see RFC [#3199]([3199](https://github.com/rust-lang/rfcs/pull/3199)))
- What is the best validity invariant for `WellFormed<T>`?
- Should `WellFormed::{size, align}_of_val` be unsafe, and how to handle hypothetical thin DSTs?

# Future possibilities
[future-possibilities]: #future-possibilities

More niches could potentially be added to references:

- Add a niche for every unaligned address. As noted in the *Alternatives* section, this requires support for non-contiguous scalar ranges in the compiler.
- Add niches corresponding to restricted address ranges (which would be, by necessity, platform-specific). For example, on 64-bit linux, the upper-half of the address space is unavailable to userspace and could be exploited.
  - Note that because any constant address is valid for zero-sized values, references to zero-sized values can't have these niches.

Combined with smarter `enum` packing optimizations, unaligned niches could allow for automatic tagged-pointer-like `enum` layouts in some restricted cases, e.g.

```rust
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


