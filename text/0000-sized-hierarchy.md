- Feature Name: `sized_hierarchy`
- Start Date: 2024-09-30
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

All of Rust's types are either *sized*, which implement the `Sized` trait and
have a statically computable size during compilation, or *unsized*, which do not
implement the `Sized` trait and are assumed to have a size which can be computed
at runtime. However, this dichotomy misses two categories of type - types whose
size is unknown during compilation but statically known at runtime, and types
whose size can never be known. Supporting the former is a prerequisite to stable
scalable vector types and supporting the latter is a prerequisite to unblocking
extern types.

*The ideas in this RFC are heavily inspired by and borrow from blog posts and
previous RFCs which have proposed similar changes to the language, see
[Prior Art](#prior-art) for a full list with appropriate attributions.*

# Background
[background]: #background

Rust has the `Sized` marker trait which indicates that a type's size is
statically known at compilation time. `Sized` is an trait which is automatically
implemented by the compiler on any type that has a statically known size. All
type parameters have a default bound of `Sized` and `?Sized` syntax can be used
to remove this bound.

Due to `size_of_val::<T>`'s `T: ?Sized` bound, it is expected that the size of a
`?Sized` type can be computable at runtime, and therefore a `T` with `T: ?Sized`
cannot be a type with no size.

# Motivation
[motivation]: #motivation

Introducing a hierarchy of `Sized` traits resolves blockers for other RFCs which
have had significant interest.

## Runtime-sized types and scalable vector types
[runtime-sized-types-and-scalable-vector-types]: #runtime-sized-types-and-scalable-vector-types

Rust already supports [SIMD][rfc_simd] (*Single Instruction Multiple Data*),
which allows operating on multiple values in a single instruction. Processors
have SIMD registers of a known, fixed length and a variety of intrinsics
which operate on these registers. For example, x86-64 introduced 128-bit SIMD
registers with SSE, 256-bit SIMD registers with AVX, and 512-bit SIMD registers
with AVX-512, and Arm introduced 128-bit SIMD registers with Neon.

As an alternative to releasing SIMD extensions with greater bit widths, Arm and
RISC-V have vector extensions (SVE/Scalable Vector Extension and the "V" Vector
Extension respectively) where the bit width of vector registers depends on the
CPU implementation, and the instructions which operate these registers are bit
width-agnostic.

As a consequence, these types are not `Sized` in the Rust sense, as the size of
a scalable vector cannot be known during compilation, but is statically known at
runtime. However, these are value types which should implement `Copy` and can be
returned from functions, can be variables on the stack, etc. These types should
implement `Copy` but given that `Copy` is a supertrait of `Sized`, they cannot
be `Copy` without being `Sized`, and aren't `Sized`.

Introducing a hierarchy of `Sized` traits will enable `Copy` to be a supertrait
of the trait for types whose size is known statically at runtime, and therefore
enable these types to be `Copy` and function correctly without special cases in
the type system. See [rfcs#3268: Scalable Vectors][rfc_scalable_vectors].

## Unsized types and extern types
[unsized-types-and-extern-types]: #unsized-types-and-extern-types

[Extern types][rfc_extern_types] has long been blocked on these types being
neither `Sized` nor `?Sized` ([relevant issue][issue_extern_types_align_size]).

RFC #1861 defined that `std::mem::size_of_val` and `std::mem::align_of_val`
should not be defined for extern types but not how this should be achieved, and
suggested an initial implementation could panic when called with extern types,
but this is always wrong, and not desirable behavior in keeping with Rust's
values. `size_of_val` returns 0 and `align_of_val` returns 1 for extern types in
the current implementation. Ideally `size_of_val` and `align_of_val` would error
if called with an extern type, but this cannot be expressed in the bounds of
`size_of_val` and `align_of_val` and this remains a blocker for extern types.

Furthermore, unsized types cannot be members of structs as their alignment is
unknown and this is necessary to calculate field offsets. `extern type`s also
cannot be used in `Box` as `Box` requires size and alignment for both allocation
and deallocation.

Introducing a hierarchy of `Sized` traits will enable extern types to implement
a trait for unsized types and therefore not meet the bounds of `size_of_val`
and `align_of_val`.

# Terminology
[terminology]: #terminology

In the Rust community, "unsized" and "dynamically sized" are often used
interchangeably to describe any type that does not implement `Sized`. This is
unsurprising as any type which does not implement `Sized` is necessarily
"unsized" and the only types this description captures are those which are
dynamically sized.

In this RFC, a distinction is made between "unsized" and "dynamically sized"
types. Unsized types is used to refer only to those which have no known
size/alignment, such as those described by [the extern types
RFC][rfc_extern_types]. Dynamically-sized types describes those types whose size
cannot be known statically at compilation time and must be computed at runtime.

Within this RFC, no terminology is introduced to describe all types which do not
implement `Sized` in the same sense as "unsized" is colloquially used.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation
Most types in Rust have a size known at compilation time, such as `u32` or
`String`. However, some types in Rust do not have known sizes.

For example, slices have an unknown length while compiling and are
known as *dynamically-sized types*, their size must be computed at runtime.
There are also types with a statically known size but only at runtime, and
*unsized types* with no size whatsoever.

Rust uses marker traits to indicate whether a type is sized, when this is known
and if the size needs to be computed. There are four traits related to the size of
a type in Rust: `Sized` , `RuntimeSized`, `DynSized` and `Unsized`. 

Everything which implements `Sized` also implements `RuntimeSized`, and
likewise with `RuntimeSized` with `DynSized`, and `DynSized` with `Unsized`.

```
┌─────────────────────────────────────────────────┐
│ ┌─────────────────────────────────────┐         │
│ │ ┌────────────────────────┐          │         │
│ │ │ ┌───────┐              │          │         │
│ │ │ │ Sized │ RuntimeSized │ DynSized │ Unsized │
│ │ │ └───────┘              │          │         │
│ │ └────────────────────────┘          │         │
│ └─────────────────────────────────────┘         │
└─────────────────────────────────────────────────┘
```

`Unsized` is implemented by any type which may or may not be sized, which is
to say, every type - from a `u32` which is obviously sized (32 bits) to an
`extern type` (from [rfcs#1861][rfc_extern_types]) which has no known size.

`DynSized` is a subset of `Unsized`, and excludes those types whose sizes
cannot be computed at runtime.

Similarly, `RuntimeSized` is a subset of `DynSized`, and excludes those types
whose sizes are not statically known at runtime. Scalable vectors (from
[rfc#3268][rfc_scalable_vectors]) have an unknown size at compilation time but
statically known size at runtime.

And finally, `Sized` is a subset of `RuntimeSized`, and excludes those types
whose sizes are not statically known at compilation time.

All type parameters have an implicit bound of `Sized` which will be automatically
removed if a `RuntimeSized`, `DynSized` or `Unsized` bound is present instead.
Prior to the introduction of `RuntimeSized`, `DynSized` and `Unsized`, `Sized`'s
implicit bound could be removed using the `?Sized` syntax, which is now
equivalent to a `DynSized` bound and will be deprecated in the next edition.

Various parts of Rust depend on knowledge of the size of a type to work, for
example:

- `std::mem::size_of_val` computes the runtime size of a value, and thus
  cannot accept `extern type`s, and this should be prevented by the type
  system.
- Rust allows dynamically-sized types to be used as struct fields, but the
  alignment of the type must be known, which is not the case for
  `extern type`s.
- Allocation and deallocation of an object with `Box` requires knowledge of
  its size and alignment, which `extern type`s do not have.
- For a value type to be allocated on the stack, it needs to have statically
  known size, which dynamically-sized and unsized types do not have (but 
  sized and "runtime sized" types do).

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Introduce three new marker traits, `RuntimeSized`, `DynSized` and `Unsized`,
creating a "hierarchy" of `Sized` traits:

- `Unsized`
    - Types that may not have a knowable size at all.
    - `Unsized` will be implemented for:
        - `DynSized` types
            - if a type's size is computable at runtime then it may or
              may not have a size (it does)
        - `extern type`s from [rfcs#1861][rfc_extern_types] 
        - compound types where every element is `Unsized`
    - In practice, every type will implement `Unsized`.
- `DynSized`
    - Types whose size is computable at runtime.
    - `DynSized` is a subtrait of `Unsized`.
    - `DynSized` will be implemented for:
        - `RuntimeSized` types
            - if a type's size is statically known at runtime time then it is
              trivially computable at runtime
        - slices `[T]`
        - string slice `str`
        - trait objects `dyn Trait`
        - compound types where every element is `DynSized`
- `RuntimeSized`
    - Types whose size is statically known at runtime.
    - `RuntimeSized` is a subtrait of `DynSized`.
    - `RuntimeSized` will be implemented for:
        - `Sized` types
            - if a type's size is statically known at compilation time then it
              is also statically known at runtime
        - scalable vectors from [rfcs#3268][rfc_scalable_vectors]
        - compound types where every element is `RuntimeSized`
- `Sized`
    - Types whose size is statically known at compilation time.
    - `Sized` is a subtrait of `RuntimeSized`.
    - `Sized` will be implemented for:
        - primitives `iN`, `uN`, `fN`, `char`, `bool`
        - pointers `*const T`, `*mut T`
        - function pointers `fn(T, U) -> V`
        - arrays `[T; n]`
        - never type `!`
        - unit tuple `()`
        - closures and generators
        - compound types where every field is `Sized`
        - anything else which currently implements `Sized`

Like `Sized`, implementations of these three traits are automatically generated
by the compiler and cannot be implemented manually.

In the compiler, `?Sized` would be made syntactic sugar for a `DynSized`
bound (and eventually removed in [an upcoming edition][#edition-changes]).
A `DynSized` bound is equivalent to a `?Sized` bound: all values in
Rust today whose types do not implement `Sized` are valid arguments to
`std::mem::size_of_val` and as such have a size which can be computed at
runtime, and therefore will implement `DynSized`. As there are currently no
`extern type`s or other types which would not implement `DynSized`, every type
in Rust today which would satisfy a `?Sized` bound would satisfy a `DynSized`
bound.

A default implicit bound of `Sized` is added by the compiler to every type
parameter `T` that does not have an explicit `Sized`, `?Sized`, `RuntimeSized`,
`DynSized` or `Unsized` bound. This is somewhat unintuitive as typically adding
a trait bound does not remove the implementation of another trait bound from
being implemented, however it's debatable whether this is more or less confusing
than existing `?Sized` bounds.

An implicit `DynSized` bound is added to the `Self` type of traits. Like
implicit `Sized` bounds, this is omitted if an explicit `Sized`, `RuntimeSized`
bound is present.

Types implementing only `Unsized` cannot be used in structs (unless it is
`#[repr(transparent)]`) as the alignment of these types would need to be known
in order to calculate field offsets and this would not be possible. Types
implementing only `DynSized` and `Unsized` could continue to be used in structs,
but only as the last field.

As `RuntimeSized`, `DynSized` and `Unsized` are not default bounds, there is no
equivalent to `?Sized` for these traits.

There is a potential performance impact within the trait system to adding
supertraits to `Sized`, as implementation of these supertraits will need to be
proven whenever a `Sized` obligation is being proven (and this happens very
frequently, being a default bound). It may be necessary to implement an
optimisation whereby `Sized`'s supertraits are assumed to be implemented and
checking them is skipped - this should be sound as all of these traits are
implemented by the compiler and therefore this property can be guaranteed by
the compiler implementation.

## Edition changes
[edition-changes]: #edition-changes

In the next edition, writing `?Sized` bounds would no longer be accepted and the
compiler would suggest to users writing `DynSized` bounds instead. Existing `?
Sized` bounds can be trivially rewritten to `DynSized` bounds by `rustup`.

## Auto traits and backwards compatibility
[auto-traits-and-backwards-compatibility]: #auto-traits-and-backwards-compatibility

A hierarchy of `Sized` traits sidesteps [the backwards compatibility hazards
which typically scupper attempts to add new traits implemented on every
type][changing_rules_of_rust].

Adding a new auto trait to the bounds of an existing function would typically
be a breaking change, despite all types implementing the new auto trait, in
two cases:

1. Callers with generic parameters would not have the new bound. For example,
   adding a new auto trait (which is not a default bound) as a bound to `std_fn`
   would cause `user_fn` to stop compiling as `user_fn`'s `T` would need the bound
   added too:

    ```rust
fn user_fn<T>(value: T) { std_fn(value) }
fn std_fn<T: NewAutoTrait>(value: T) { /* .. */ }
//~^ ERROR the trait bound `T: NewAutoTrait` is not satisfied
    ```
   
   Unlike with an arbitrary new auto trait, the proposed traits are all
   subtraits of `Sized` and every generic parameter either has a default bound
   of `Sized` or has a `?Sized` bound, which enables this risk of backwards
   compatibility to be avoided.

   Relaxing the `Sized` bound of an existing function's generic parameters to
   `RuntimeSized`, `DynSized` or `Unsized` would not break any callers, as those
   callers' generic parameters must already have a `T: Sized` bound and therefore
   would already satisfy the new relaxed bound. Callers may now have a stricter
   bound than is necessary, but they likewise can relax their bounds without that
   being a breaking change.

   If an existing function had a generic parameter with a `?Sized` bound and
   this bound were changed to `DynSized` or relaxed to `Unsized`, then callers'
   generic parameters would either have a `T: Sized` or `T: ?Sized` bound:

   - If callers' generic parameters have a `T: Sized` bound then there would be
     no breaking change as `T: Sized` implies the changed or relaxed bound.
   - If callers' generic parameter have a `T: ?Sized` bound then this is
     equivalent to a `T: DynSized` bound, as described earlier. Therefore there would
     be no change as `T: DynSized` implies the changed or relaxed bound.

     ```rust
     fn user_fn<T>(value: T) { std_fn(value) } // T: Sized, so T: RuntimeSized
     fn std_fn<T: RuntimeSized>(value: T) { /* .. */ }
     ```

     If an existing function's generic parameter had a `?Sized` bound and this
     bound were changed to `RuntimeSized` then this *would* be a breaking change, but
     it is not expected that this change would be applied to any existing functions
     in the standard library.
   
     The proposed traits in this RFC are only a non-breaking change because the
     new auto traits are being added as subtraits of `Sized`, adding supertraits of
     `Sized` would be a breaking change.

2. Trait objects passed by callers would not imply the new trait. For example,
   adding a new auto trait as a bound to `std_fn` would cause `user_fn` to stop
   compiling as its trait object would not automatically implement the new auto
   trait:

   ```rust
   fn user_fn(value: Box<dyn ExistingTrait>) { std_fn(value) }
   fn std_fn<T: NewAutoTrait>(value: &T) { /* ... */}
   //~^ ERROR the trait bound `dyn ExistingTrait: NewAutoTrait` is not satisfied in `Box<dyn ExistingTrait>`
   ```

   Like the previous case, due to the proposed traits being subtraits of
   `Sized`, and every trait object implementing `Sized`, adding a `RuntimeSized`,
   `DynSized`, or `Unsized` bound to any existing generic parameter would be
   already satisfied.

Additionally, it is not expected that this RFC's additions would result in much
churn within the ecosystem. All bounds in the standard library should be re-evaluated
during the implementation of this RFC, but bounds in third-party crates need not be:

Up-to-`RuntimeSized`-implementing types will primarily be used for localised
performance optimisation, and `Unsized`-only-implementing types will primarily be
used for localised FFI, neither is expected to be so pervasive throughout Rust
software to the extent that all existing `Sized` or `?Sized` bounds would need to
be immediately reconsidered in light of their addition. If a user of a
up-to-`RuntimeSized`-implementing type or a `Unsized`-only-implementing type did
encounter a bound that needed to be relaxed, this could be changed in a patch to
the relevant crate without breaking backwards compatibility as-and-when such bounds
are discovered.

## Changes to the standard library
[changes-to-the-standard-library]: #changes-to-the-standard-library

With these new traits and having established changes to existing bounds which
can be made while preserving backwards compatibility, the following changes
could be made to the standard library:

- [`std::mem::size_of_val`][api_size_of_val] and
  [`std::mem::align_of_val`][api_align_of_val]
    - `T: ?Sized` becomes `T: DynSized`
    - As described previously, `?Sized` is equivalent to `DynSized` due to the
      existence of `size_of_val`, therefore this change does not break any existing
      callers.
    - `DynSized` would not be implemented by `extern type`s from [rfcs#1861]
      [rfc_extern_types], which would prevent these functions from being invoked on
      types which have no known size or alignment.
    - `size_of_val` and `align_of_val` are currently `const` unstably and these
      types would no longer be able to be made `const` as this would require `T:
      Sized`, not `T: DynSized`.
- [`std::clone::Clone`][api_clone]
    - `Clone: Sized` becomes `Clone: RuntimeSized`
    - `RuntimeSized` is implemented by more types than `Sized` and by all types
      which implement `Sized`, therefore any current implementor of `Clone` will not
      break.
    - `RuntimeSized` will be implemented by scalable vectors from
      [rfcs#3268][rfc_scalable_vectors] whereas `Sized` would not have been, and
      this is correct as `RuntimeSized` types can still be cloned. As `Copy: Clone`,
      this allows scalable vectors to be `Copy`, which is necessary and currently a
      blocker for that feature.
- [`std::boxed::Box`][api_box]
    - `T: ?Sized` becomes `T: DynSized`
    - As before, this is not a breaking change and prevents types only
      implementing `Unsized` from being used with `Box`, as these types do not have
      the necessary size and alignment for allocation/deallocation.

As part of the implementation of this RFC, each `Sized`/`?Sized` bound in the standard
library would need to be reviewed and updated as appropriate.

# Drawbacks
[drawbacks]: #drawbacks

This is a fairly large change to the language given how fundamental the `Sized`
trait is, so it could be deemed too confusing to introduce more complexity.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

There are various points of difference to the [prior art](#prior-art) related to
`Sized`, which spans almost the entirety of the design space:

- In contrast to [rfcs#709][rfc_truly_unsized_types], marker types aren't used
  to disable `Sized` because we are able to introduce traits to do so without
  backwards compatibility hazards and that feels more appropriate for Rust.
- In contrast to [rfcs#1524][rfc_custom_dst], the `Sized` trait isn't extended
  as this wouldn't be sufficient to capture the range in sizedness that this RFC
  aims to capture (i.e. for scalable vectors with `RuntimeSized` or `extern type`s
  with `Unsized`), even if it theoretically could enable Custom DSTs.
- In contrast to [rfcs#1993][rfc_opaque_data_structs], [rust#44469][pr_dynsized],
  [rust#46108][pr_dynsized_rebase],  [rfcs#2984][rfc_pointee_and_dynsized] and
  [eRFC: Minimal Custom DSTs via Extern Type (DynSized)][erfc_minimal_custom_dsts_via_extern_type],
  none of the traits proposed in this RFC are default bounds and therefore do not
  need to support being relaxed bounds (i.e. no `?RuntimeSized`), which avoids
  additional language complexity and backwards compatibility hazards related to
  relaxed bounds and associated types.
- In contrast to [rfcs#1524][rfc_custom_dst], [rfc#1993][rfc_opaque_data_structs],
  [Pre-eRFC: Let's fix DSTs][pre_erfc_fix_dsts], [Pre-RFC: Custom DSTs][prerfc_custom_dst]
  and [eRFC: Minimal Custom DSTs via Extern Type (DynSized)][erfc_minimal_custom_dsts_via_extern_type],
  `DynSized` does not have `size_of_val`/`align_of_val` methods to support custom DSTs,
  but like [rfcs#2310][rfc_dynsized_without_dynsized], this RFC prefers keeping
  these functions out of `DynSized` and having all of the traits be solely marker
  traits. Custom DSTs are still compatible with this proposal using a `Contiguous`
  trait as in [rfcs#2594][rfc_custom_dst_electric_boogaloo].

## Bikeshedding
All of the trait names proposed in the RFC can be bikeshed and changed, they'll
ultimately need to be decided but aren't the important part of the RFC.

## Why have `Unsized`?
It may seem that the `Unsized` trait is unnecessary as this is equivalent to the
absense of any bounds whatsoever, but having an `Unsized` trait is necessary to
enable the meaning of `?Sized` to be re-defined to be equivalent to `DynSized`
and avoid complicated behaviour change over an edition.

Without `Unsized`, if a user wanted to remove all sizedness bounds from a generic
parameter then they would have two options:

1. Introduce new relaxed bounds (i.e. `?DynSized`), which has been found
   unacceptable in previous RFCs ([rfcs#2255][issue_more_implicit_bounds]
   summarizes these discussions)
2. Maintain `?Sized`'s existing meaning of removing the implicit `Sized` bound
    - This could be maintained with or without having the existence of bounds
      for `RuntimeSized` and `DynSized` remove the implicit `Sized` bound.
      
The latter is the only viable option, but this would complicate changing
`size_of_val`'s existing `?Sized` bound.

`?Sized` can be redefined to be equivalent to `DynSized` in all editions
and the syntax can be removed in a future edition only because adding an
`Unsized` bound is equivalent to removing the `Sized` bound and imposing
no constraints on the sizedness of a parameter.

Without `Unsized`, a complicated migration would be necessary to change
all current uses of `?Sized` to `DynSized` (as these are equivalent) and
then future uses of `?Sized` would now accept more types than `?Sized`
previously did (they would now accept the `extern type`s).

In [rfcs#3396][rfc_extern_types_v2], `MetaSized` was introduced and used a
similar mechanism over an edition to redefine `?Sized`. 

## Alternatives to this RFC
There are not many alternatives to this RFC to unblock `extern type`s and
scalable vectors:

- Without this RFC, scalable vectors from [rfcs#3268][rfc_scalable_vectors]
  would remain blocked unless special-cased by the compiler to bypass the type
  system.
- Extern types from [rfcs#1861][rfc_extern_types] would remain blocked if no
  action was taken, unless:
    - The language team decided that having `size_of_val` and `align_of_val`
      panic was acceptable.
    - The language team decided that having `size_of_val` and `align_of_val`
      return `0` and `1` respectively was acceptable.
    - The language team decided that `extern type`s could not be instantiated
      into generics and that this was acceptable.
    - The language team decided that having `size_of_val` and `align_of_val`
      produce post-monomorphisation errors for `extern type`s was acceptable.
- It would not be possible for this change to be implemented as a library or
  macro instead of in the language itself.

# Prior art
[prior-art]: #prior-art

There have been many previous proposals and discussions attempting to resolve
the `size_of_val` and `align_of_val` for `extern type`s through modifications to
the `Sized` trait, summarised below:

- [rfcs#709: truly unsized types][rfc_truly_unsized_types], [mzabaluev][author_mzabaluev], Jan 2015
    - Earliest attempt to opt-out of `Sized`.
    - Proposes dividing types which do not implement `Sized` into DSTs and types
      of indeterminate size.
        - Adding a field with a `std::marker::NotSized` type will make a type
          opt-out of `Sized`, preventing the type from being used in all the places where
          it needs to be `Sized`.
        - Dynamically sized types will "intrinsically" implement `DynamicSize`,
          references to these types will use fat pointers.
    - Ultimately postponed for post-1.0.
- [rfcs#813: truly unsized types (issue)][issue_truly_unsized_types], [pnkfelix][author_pnkfelix], Feb 2015
    - Tracking issue for postponed [rfcs#709][rfc_truly_unsized_types].
    - Links to an newer version of [rfcs#709][rfc_truly_unsized_types], still
      authored by [mzabaluev][author_mzabaluev].
    - Proposes being able to opt-out of `Sized` with a negative impl (a `CStr`
      type containing only a `c_char` is the example given of a DST which would
      opt-out of `Sized`).
        - Also proposes removing `Sized` bound on various `AsPtr`/`AsMutPtr`/
          `FromPtr`/`FromMutPtr` traits as they existed at the time, so that a user might
          be able to implement these to preserve the ability to use a thin pointer for
          their unsized type when that is possible.
    - Ultimately closed after [rfcs#1861][rfc_extern_types] was merged and
      intended that [rfcs#2255][issue_more_implicit_bounds] be used to discuss the
      complexities of that proposal.
- [rfcs#1524: Custom Dynamically Sized Types][rfc_custom_dst], [strega-nil][author_strega_nil], Mar 2016
    - Successor of [rfcs#709][rfc_truly_unsized_types]/[rfcs#813][issue_truly_unsized_types].
    - Proposes an `unsafe trait !Sized` (which isn't just a negative impl), with
      an associated type `Meta` and `size_of_val` method.
        - Under this proposal, users would create a "borrowed" version of their
          type (e.g. what `[T]` is to `Vec<T>`) which has a zero-sized last field, which
          is described in the RFC as "the jumping off point for indexing your block of
          memory".
        - These types would implement `!Sized`, providing a type for `Meta`
          containing any extra information necessary to compute the size of the DST (e.g.
          a number of strides) and an implementation of `size_of_val` for the type.
        - There would be intrinsics to help make create instances of
          these dynamically sized types, namely `make_fat_ptr`, `fat_ptr_meta` and
          `size_of_prelude`.
- [rfcs#1861: extern types][rfc_extern_types], [canndrew][author_canndrew], Jan 2017
    - Merged in Jul 2017.
    - This RFC mentions the issue with `size_of_val` and `align_of_val` but
      suggests that these functions panic in an initial implementation and that
      "before this is stabilised, there should be some trait bound or similar on them
      that prevents there use statically". Inventing an exact mechanism was intended
      to be completed by [rfcs#1524][rfc_custom_dst] or its like.
- [rfcs#1993: Opaque Data structs for FFI][rfc_opaque_data_structs], [mystor][author_mystor], May 2017
    - This RFC was an alternative to the original `extern type`s RFC
      ([rfcs#1861][rfc_extern_types]) and introduced the idea of a `DynSized` auto
      trait.
    - Proposes a `DynSized` trait which was a built-in, unsafe, auto trait,
      a supertrait of `Sized`, and a default bound which could be relaxed with
      `? DynSized`.
        - It would automatically implemented for everything that didn't have an
          `Opaque` type in it (this RFC's equivalent of an `extern type`).
        - `size_of_val` and `align_of_val` would have their bounds changed to
          `DynSized`.
        - Trait objects would have a `DynSized` bound by default and the
          `DynSized` trait would have `size_of_val` and `align_of_val` member functions.
    - Ultimately closed as [rfcs#1861][rfc_extern_types] was entering final
      comment period.
- [rust#43467: Tracking issue for RFC 1861][issue_tracking_extern_types], [aturon][author_aturon], Jul 2017
    - Tracking thread created for the implementation of [rfc#1861][rfc_extern_types].
    - In 2018, the language team had consensus against having `size_of_val`
      return a sentinel value and adding any trait machinery, like `DynSized`, didn't
      seem worth it, preferring to panic or abort.
        - This was considering `DynSized` with a relaxed bound.
        - Anticipating some form of custom DSTs, there was the possibility
          that `size_of_val` could run user code and panic anyway, so making it panic
          for `extern type`s wasn't as big an issue. `size_of_val` running in unsafe code
          could be a footgun and that caused mild concern.
        - See [this comment](https://github.com/rust-lang/rust/issues/43467#issuecomment-377521693)
          and [this comment](https://github.com/rust-lang/rust/issues/43467#issuecomment-377665733).
    - Conversation became more sporadic following 2018 and most
      recent discussion was spurred by the
      [Sized, DynSized and Unsized][blog_dynsized_unsized] blog post.
        - See [this comment](https://github.com/rust-lang/rust/issues/43467#issuecomment-2073513472)
          onwards.
        - It's unclear how different language team opinion is since the 2018
          commentary, but posts like above suggest some change.
- [rust#44469: Add a `DynSized` trait][pr_dynsized], [plietar][author_plietar], Sep 2017
    - This pull request intended to implement the `DynSized` trait from
      [rfcs#1993][rfc_opaque_data_structs].
    - `DynSized` as implemented is similar to that from
      [rfcs#1993][rfc_opaque_data_structs] except it is implemented for every
      type with a known size and alignment at runtime, rather than requiring an
      `Opaque` type.
    - In addition to preventing `extern type`s being used in `size_of_val` and
      `align_of_val`, this PR is motivated by wanting to have a mechanism by which
      `!DynSized` types can be prevented from being valid in struct tails due to needing
       to know the alignment of the tail in order to calculate its offset, and this is
    - `DynSized` had to be made a implicit supertrait of all traits in this
      implementation - it is presumed this is necessary to avoid unsized types
      implementing traits.
    - This actually went through FCP and would have been merged if not
      eventually closed for inactivity.
- [rust#46108: Add DynSized trait (rebase of #44469)][pr_dynsized_rebase], [mikeyhew][author_mikeyhew], Nov 2017
    - This pull request is a resurrection of [rust#44469][pr_dynsized].
    - Concerns were raised about the complexity of adding another `?Trait` to
      the language, and suggested that having `size_of_val` panic was sufficient (the
      current implementation does not panic and returns zero instead, which is also
      deemed undesirable).
        - It was argued that `?Trait`s are powerful and should be made more
          ergonomic rather than avoided.
    - [kennytm][author_kennytm] left a useful comment summarising [which
      standard library bounds would benefit from relaxation to a `DynSized` bound]
      (https://github.com/rust-lang/rust/pull/46108#issuecomment-353672604).
    - Ultimately this was closed [after a language team meeting](https://github.com/rust-lang/rust/pull/46108#issuecomment-360903211)
      deciding that `?DynSized` was ultimately too complex and couldn't be
      justified by support for a relatively niche feature like `extern type`.
- [rfcs#2255: More implicit bounds (?Sized, ?DynSized, ?Move)][issue_more_implicit_bounds], [kennytm][author_kennytm], Dec 2017
    - Issue created following [rust#46108][pr_dynsized_rebase] to discuss the
      complexities surrounding adding new traits which would benefit from relaxed
      bounds (`?Trait` syntax).
    - There have been various attempts to introduce new auto traits with
      implicit bounds, such as `DynSized`, `Move`, `Leak`, etc. Often rejected due to
      the ergonomic cost of relaxed bounds.
        - `?Trait` being a negative feature confuses users.
        - Downstream crates need to re-evaluate every API to determine if adding
          `?Trait` makes sense, for each `?Trait` added.
        - `?Trait` isn't actually backwards compatible like everyone thought due
          to interactions with associated types.
    - This thread was largely motivated by the `Move` trait and that was
      replaced by the `Pin` type, but there was an emerging consensus that `DynSized`
      may be more feasible due to its relationship with `Sized`.
- [Pre-eRFC: Let's fix DSTs][pre_erfc_fix_dsts], [mikeyhew][author_mikeyhew], Jan 2018
    - This eRFC was written as a successor to [rfcs#1524][rfc_custom_dst].
    - It proposes `DynSized` trait and a bunch of others. `DynSized` is a
      supertrait of `Sized` (indirectly) and contains a `size_of_val` method. This
      proposal is the first to remove `Sized` bounds if another sized trait (e.g.
      `DynSized`) has an explicit bound.
        - This enables deprecation of `?Sized` like this RFC proposes.
    - A `Thin` type to allow thin pointers to DSTs is also proposed in
      this pre-eRFC - it is a different `Thin` from the currently unstable
      `core::ptr::Thin` and it's out-of-scope for this RFC to include a similar type
      and since accepted [rfcs#2580][rfc_pointer_metadata_vtable] overlaps.
    - This pre-eRFC may be the origin of the idea for a family of `Sized`
      traits, later cited in [Sized, DynSized, and Unsized][blog_dynsized_unsized].
    - [rfcs#2510][rfc_pointer_metadata_vtable] was later submitted which was a
      subset of this proposal (but none of the `DynSized` parts).
    - This eRFC ultimately fizzled out and didn't seem to result in a proper RFC
      being submitted.
- [rfcs#2310: DynSized without ?DynSized][rfc_dynsized_without_dynsized], [kennytm][author_kennytm], Jan 2018
    - This RFC proposed an alternative version of `DynSized` from
      [rfcs#1993][rfc_opaque_data_structs]/[rust#44469][pr_dynsized] but without
      being an implicit bound and being able to be a relaxed bound (i.e. no
      `?DynSized`).
        - Adding new implicit bounds which can be relaxed has backwards
          compatibility hazards, see [rfcs#2255][issue_more_implicit_bounds].
    - The proposed `DynSized` trait in [rfcs#2310][rfc_dynsized_without_dynsized]
      is really quite similar to the trait proposed by this RFC except:
        - It includes an `#[assume_dyn_sized]` attribute to be added to
          `T: ?Sized` bounds instead of replacing them with `T: DynSized`, which
          would warn instead of error when a non-`DynSized` implementing type is
          substituted into `T`.
            - This is to avoid a backwards compatibility break for uses of
              `size_of_val` and `align_of_val` with `extern type`s, but it is
              unclear why this is necessary given that `extern type`s are
              unstable.
        - It does not include `RuntimeSized` or `Unsized`.
        - Adding an explicit bound for `DynSized` does not remove the implicit
          bound for `Sized`.
- [rust#49708: `extern type` cannot support `size_of_val` and `align_of_val`][issue_extern_types_align_size], [joshtriplett][author_joshtriplett], Apr 2018
    - Primary issue for the `size_of_val`/`align_of_val` `extern type`s
      blocker, following no resolution from either of [rfcs#1524][rfc_custom_dst] and
      [rust#44469][pr_dynsized] or their successors.
    - This issue largely just re-hashes the arguments made in other threads
      summarised here.
- [Pre-RFC: Custom DSTs][prerfc_custom_dst], [ubsan][author_ubsan], Nov 2018
    - This eRFC was written as a successor to [rfcs#1524][rfc_custom_dst].
    - Proposes addition of a `DynamicallySized` trait with a `Metadata`
      associated type and `size_of_val` and `align_of_val` member functions.
        - It has an automatic implementation for all `Sized` types, where
          `Metadata = ()` and `size_of_val` and `align_of_val` just call `size_of` and
          `align_of`.
        - It can be manually implemented for DSTs and if it is, the type will
          not implement `Sized`.
    - Due to `DynamicallySized` not being a supertrait of `Sized`, this proposal
      had no way of modifying the bounds of `size_of_val` and `align_of_val` without
      it being a breaking change (and so did not propose doing so).
    - This eRFC ultimately fizzled out and didn't seem to result in a proper RFC
      being submitted.
- [rfcs#2594: Custom DSTs][rfc_custom_dst_electric_boogaloo], [strega-nil][author_strega_nil], Nov 2018 
    - This eRFC was written as a successor to [rfcs#1524][rfc_custom_dst].
        - This is more clearly an direct evolution of [rfcs#1524][rfc_custom_dst]
          than other successors were, unsurprisingly given the same author.
    - Proposes a `Pointee` trait with `Metadata` associated type and a
      `Contiguous` supertrait of `Pointee` with `size_of_val` and `align_of_val`
      members.
        - `Sized` is a subtrait of `Pointee<Metadata = ()>`  (as sized types
          have thin pointers). `Sized` also implements `Contiguous` calling `size_of` and
          `align_of` for each of the member functions.
        - Dynamically sized types can implement `Pointee` manually and provide
          a `Metadata` associated type, and then `Contiguous` to implement `size_of_val`
          and `align_of_val`.
        - Intrinsics are added for constructing a pointer to a dynamically
          sized type from its metadata and value, and for accessing the metadata of a
          dynamically sized type.
        - `extern type`s do not implement `Contiguous` but do implement `Pointee`.
        - `Contiguous` is a default bound and so has a relaxed form `?Contiguous`.
    - There's plenty of overlap here with [rfcs#2580][rfc_pointer_metadata_vtable]
      and its `Pointee` trait - the accepted [rfcs#2580][rfc_pointer_metadata_vtable]
      does not make `Sized` a subtrait of `Pointee` or have a `Contiguous` trait but
      the `Pointee` trait is more or less compatible.
    - Discussed in a [November 4th 2020 design meeting](https://www.youtube.com/watch?v=wYmJK62SSOM&list=PL85XCvVPmGQg-gYy7R6a_Y91oQLdsbSpa&index=63)
      ([pre-meeting notes](https://hackmd.io/1Fq9TcAQRWa4_weWTe9adA) and
      [post-meeting notes](https://github.com/rust-lang/lang-team/blob/master/design-meeting-minutes/2020-11-04-RFC-2580-and-custom-dst.md)).
        - Meeting was mostly around [rfcs#2580][rfc_pointer_metadata_vtable] but
          mentioned the state of Custom DSTs.
    - Mentioned briefly in a [language team triage meeting](https://www.youtube.com/watch?v=NzURKQouuEU&t=3292s)
      in March 2021 and postponed until [rfcs#2510][rfc_pointer_metadata_vtable]
      was implemented.
- [Design Meeting][design_meeting], Language Team, Jan 2020
    - Custom DSTs and `DynSized` are mentioned but there aren't any implications
      for this RFC.
- [rfcs#2984: introduce `Pointee` and `DynSized`][rfc_pointee_dynsized], [nox][author_nox], Sep 2020
    - This RFC aims to land some traits in isolation so as to enable progress on
      other RFCs.
    - Proposes a `Pointee` trait with associated type `Meta` (very similar to
      accepted [rfcs#2580][rfc_pointer_metadata_vtable]) and a `DynSized` trait which
      is a supertrait of it. `Sized` is made a supertrait of `DynSized<Meta = ()>`.
      Neither new trait can be implemented by hand.
        - It's implied that `DynSized` is implemented for all dynamically sized
          types, but it isn't clear.
    - Despite being relatively brief, this RFC has lots of comments.
        - The author argues that `?DynSized` is okay and disagrees with
           previous concerns about complexity and that all existing bounds would need to be
           reconsidered in light of `?DynSized`.
            - In response, it is repeatedly argued that there is a mild
              preference for making `size_of_val` and `align_of_val` panic instead of adding
              `?Trait` bounds and that having the ability to do `Pointee<Meta = ()>` type
              bounds is sufficient.
- [Exotically sized types (`DynSized` and `extern type`)][design_notes_dynsized_constraints], Language Team, Jun 2022
    - Despite being published in Jun 2022, these are reportedly notes from a
      previous Jan 2020 meeting, but not the one above.
    - Explores constraints `Arc`/`Rc` and `Mutex` imply on `DynSized` bounds.
    - `MetaSized` is first mentioned in these meeting notes, as when the size/
      alignment can be known from pointer metadata.
- [eRFC: Minimal Custom DSTs via Extern Type (DynSized)][erfc_minimal_custom_dsts_via_extern_type], [CAD97][author_cad97], May 2022
    - This RFC proposes a forever-unstable default-bound unsafe trait `DynSized`
      with `size_of_val_raw` and `align_of_val_raw`, implemented for everything other
      than `extern type`s. Users can implement `DynSized` for their own types. This
      proposal doesn't say whether `DynSized` is a default bound but does mention a
      relaxed form of the trait `?DynSized`.
- [rfcs#3319: Aligned][rfc_aligned], [Jules-Bertholet][author_jules_bertholet], Sep 2022
    - This RFC aims to separate the alignment of a type from the size of the
      type with an `Aligned` trait.
        - Automatically implemented for all types with an alignment (includes
          all `Sized` types).
        - `Aligned` is a supertrait of `Sized`.
- [rfcs#3396: Extern types v2][rfc_extern_types_v2], [Skepfyr][author_skepfyr], Feb 2023
    - Proposes a `MetaSized` trait for types whose size and alignment can
      be determined solely from pointer metadata without having to dereference the
      pointer or inspect the pointer's address.
        - Under this proposal, `[T]` is `MetaSized` as the pointer metadata
          knows the size, rather than `DynSized`.
        - `MetaSized` is automatically implemented for all types except extern
          types.
        - `MetaSized` types can be the last field of a struct as the offset can
          be determined from the pointer metadata alone.
        - `Sized` is not a supertrait or subtrait of `MetaSized`.
            - This may make the proposal subject to the backwards
              incompatibilities described in
              [Changes to the Standard Library](#changes-to-the-standard-library).
        - `size_of_val`'s bound would be changed to `T: ?Sized + MetaSized`.
    - Attempts to sidestep backwards compatibility issues with introducing a
      default bound via changing what `?Sized` means across an edition boundary.
        - This [may be backwards incompatible](https://github.com/rust-lang/rfcs/pull/3396#issuecomment-1728509626).
    - Discussed in [a language team design meeting](https://hackmd.io/TSXpOX4iS3qqDdVD00z7tw?view).
- [rfcs#3536: Trait for `!Sized` thin pointers][rfc_not_sized_thin_pointers], [jmillikin][author_jmillikin], Nov 2023
    - Introduces unsafe trait `DynSized` with a `size_of_val` method.
        - It can be implemented on `!Sized` types. 
            - It is an error to implement it on `Sized` types.
        - References to types that implement `DynSized` do not need to store the
          size in pointer metadata. Types implementing `DynSized` without other pointer
          metadata are thin pointers.
    - This proposal has no solution for `extern type` limitations, its sole aim
      is to enable more pointers to be thin pointers.
- [Sized, DynSized, and Unsized][blog_dynsized_unsized], [Niko Matsakis][author_nikomatsakis], Apr 2024
    - This proposes a hierarchy of `Sized`, `DynSized` and `Unsized` traits
      like in this RFC and proposes deprecating `T: ?Sized` in place of `T: Unsized`
      and sometimes `T: DynSized`. Adding a bound for any of `DynSized` or `Unsized`
      removes the default `Sized` bound.
    - As described below it is the closest inspiration for this RFC.

There are some even older RFCs that have tangential relevance that are listed
below but not summarized:

- [rfcs#5: virtual structs][rfc_virtual_structs], [nrc][author_nrc], Mar 2014
- [rfcs#9: RFC for "fat objects" for DSTs][rfc_fat_objects], [MicahChalmer][author_micahchalmer], Mar 2014
- [pre-RFC: unsized types][rfc_unsized_types], [japaric][author_japaric], Mar 2016

There haven't been any particular proposals which have included an equivalent
of the `RuntimeSized` trait, as the scalable vector types proposal in [RFC 3268]
[rfc_scalable_vectors] is relatively newer and less well known:

- [rfcs#3268: Add scalable representation to allow support for scalable vectors][rfc_scalable_vectors], [JamieCunliffe][author_jamiecunliffe], May 2022
    - Proposes temporarily special-casing scalable vector types to be able to
      implement `Copy` without implementing `Sized` and allows function return values
      to be `Copy` or `Sized` (not just `Sized`).
        - Neither of these changes would be necessary with this RFC, scalable
          vectors would just implement `RuntimeSized` and function return values would
          just need to implement `RuntimeSized`, not `Sized`.

To summarise the above exhaustive listing of prior art:

- No previous works have proposed a `RuntimeSized` trait or a `Unsized` trait
  (with the exception of [Sized, DynSized, and Unsized][blog_dynsized_unsized] for
  `Unsized`), only `DynSized`.
- One proposal proposed adding a marker type that as a field would result in the
  containing type no longer implementing `Sized`.
- Often proposals focused at Custom DSTs preferred to combine the
  escape-the-sized-hierarchy part with the Custom DST machinery.
    - e.g. `DynSized` trait with `Metadata` associated types and `size_of_val`
      and `align_of_val` methods, or a `!Sized` pseudo-trait that you could implement.
    - Given the acceptance of [rfcs#2580][rfc_pointer_metadata_vtable], Rust
      doesn't seem to be trending in this direction, as the `Metadata` part of this is
      now part of a separate `Pointee` trait.
- Most early `DynSized` trait proposals (independent or as part of Custom DSTs)
  would make `DynSized` a default bound mirroring `Sized`, and consequently had a
  relaxed form `?DynSized`.
    - Later proposals were more aware of the language team's resistance towards
      adding new relaxed bounds and tried to avoid this.
- Backwards compatibility concerns were the overriding reason for the rejection
  of previous `DynSized` proposals.
    - These can be sidestepped by avoiding having a relaxed form and by relying
      on being a supertrait of `Sized`.

The [Rationale and Alternatives](#rationale-and-alternatives) section provides
rationale for some of the decisions made in this RFC and references the prior
art above when those proposals made different decisions.

No previous proposal captures the specific part of the design space that this
proposal attempts to, but these proposals are the closest matches for parts of
this proposal:

- [Pre-eRFC: Let's fix DSTs][pre_erfc_fix_dsts] was the only other proposal
  removing `Sized` bounds when a bound for another sized trait (only `DynSized`
  in that pre-eRFC's case) was present, which makes reasoning simpler by avoiding
  relaxed bounds.
    - However, this proposal had `size_of_val` methods its `DynSized` and
      proposed a bunch of other things necessary for Custom DSTs.
- [rfcs#2310: DynSized without ?DynSized][rfc_dynsized_without_dynsized] was
  proposed at a similar time and was similarly focused only on making `Sized` more
  flexible, but had a bunch of machinery for avoiding backwards incompatibility
  that this RFC believes is unnecessary. Like this proposal, it avoided making
  `DynSized` a default bound and avoided having a relaxed form of it.
    - However, this proposal didn't suggest removing default `Sized` bounds in
      the presence of other size trait bounds.
- [Sized, DynSized, and Unsized][blog_dynsized_unsized] is very similar and a
  major inspiration for this proposal. It has everything this proposal has except
  for `RuntimeSized` and all the additional context an RFC needs.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- [rust#21974][issue_regions_too_simplistic] has been linked in previous
  `DynSized` proposals as a reason why `DynSized` (as then proposed) must be
  an implicit supertrait of all traits, but it isn't obvious why this issue is
  relevant.
    - It seems like `DynSized` being a supertrait of all traits is necessary so
      that `extern type`s do not implement any traits anyway.
- `extern type` `OpaqueListContents` is used as the last field of a struct in
  rustc, which currently works because it has a presumed alignment of one. This
  would be prohibited by the RFC as written, but relaxing this is listed as a
  future possibility.
    - Adding limitations to the use of `Unsized` types in structs could be left
      as a follow-up as it is more related to `extern type` than extending `Sized`
      anyway. Implementation of this RFC would not create any `Unsized` types.
- How would this interact with proposals to split alignment and sizedness into
  separate traits?
- Some prior art had different rules for automatically implementing `DynSized`
  on structs/enums vs on unions - are those necessary for these traits?

# Future possibilities
[future-possibilities]: #future-possibilities

- Additional size traits could be added as supertraits of `Sized` if there are
  other delineations in sized-ness that make sense to be drawn.
    - e.g. `MetaSized` from [rfcs#3396][rfc_extern_types_v2], something like
      `Sized: MetaSized`, `MetaSized: RuntimeSized`, `RuntimeSized: DynSized` and
      `DynSized: Unsized`.
- The requirement that users cannot implement any of these traits could be
  relaxed in future to support custom DSTs or any other proposed feature which
  required it.
- Relax prohibition of `Unsized` fields in some limited contexts, for example if
  it is the final field and the offset of it is never computed.
- In addition to the default bound changes described above, the default
  `T: Sized` bound could be omitted if any other bound `T: Trait` had an explicit
  supertrait of `RuntimeSized`, `DynSized` or `Unsized`. This would allow
  boilerplate `?Sized` bounds to be removed.
    - Credit to [Sized, DynSized, and Unsized][blog_dynsized_unsized] for this idea.
- This proposal is compatible with adding Custom DSTs in future.
    - Leveraging the already accepted [rfcs#2580][rfc_pointer_metadata_vtable]
      and taking an approach similar to [rfcs#2594][rfc_custom_dst_electric_boogaloo]
      with its `Contiguous` trait to provide somewhere for `size_of_val` and
      `align_of_val` to be implemented for the Custom DST.
        - Custom DSTs would need to implement `DynSized` and not implement
          `Sized` to be compatible with the traits introduced in this proposal, and maybe
          you'd want to add `Pointee` as a supertrait of `DynSized` with some non-`()`
          metadata type, or something along those lines.

[api_align_of_val]: https://doc.rust-lang.org/std/mem/fn.align_of_val.html
[api_box]: https://doc.rust-lang.org/std/boxed/struct.Box.html
[api_clone]: https://doc.rust-lang.org/std/clone/trait.Clone.html
[api_size_of_val]: https://doc.rust-lang.org/std/mem/fn.size_of_val.html
[author_aturon]: https://github.com/aturon
[author_cad97]: https://github.com/CAD97
[author_canndrew]: https://github.com/canndrew
[author_jamiecunliffe]: https://github.com/JamieCunliffe
[author_japaric]: https://github.com/japaric
[author_jmillikin]: https://github.com/jmillikin
[author_joshtriplett]: https://github.com/joshtriplett
[author_jules_bertholet]: https://github.com/Jules-Bertholet
[author_kennytm]: https://github.com/kennytm
[author_micahchalmer]: https://github.com/MicahChalmer
[author_mikeyhew]: https://github.com/mikeyhew
[author_mystor]: https://github.com/mystor
[author_mzabaluev]: https://github.com/mzabaluev 
[author_nikomatsakis]: https://github.com/nikomatsakis
[author_nox]: https://github.com/nox
[author_nrc]: https://github.com/nrc
[author_plietar]: https://github.com/plietar
[author_pnkfelix]: https://github.com/pnkfelix
[author_skepfyr]: https://github.com/Skepfyr
[author_strega_nil]: https://github.com/strega-nil
[author_ubsan]: https://github.com/ubsan
[blog_dynsized_unsized]: https://smallcultfollowing.com/babysteps/blog/2024/04/23/dynsized-unsized/
[changing_rules_of_rust]: https://without.boats/blog/changing-the-rules-of-rust/
[design_meeting]: https://hackmd.io/7r3_is6uTz-163fsOV8Vfg
[design_notes_dynsized_constraints]: https://github.com/rust-lang/lang-team/blob/master/src/design_notes/dynsized_constraints.md
[erfc_minimal_custom_dsts_via_extern_type]: https://internals.rust-lang.org/t/erfc-minimal-custom-dsts-via-extern-type-dynsized/16591?u=cad97
[issue_extern_types_align_size]: https://github.com/rust-lang/rust/issues/49708
[issue_more_implicit_bounds]: https://github.com/rust-lang/rfcs/issues/2255
[issue_regions_too_simplistic]: https://github.com/rust-lang/rust/issues/21974#issuecomment-331886186
[issue_tracking_extern_types]: https://github.com/rust-lang/rust/issues/43467
[issue_truly_unsized_types]: https://github.com/rust-lang/rfcs/issues/813
[pr_dynsized]: https://github.com/rust-lang/rust/pull/44469
[pr_dynsized_rebase]: https://github.com/rust-lang/rust/pull/46108
[pre_erfc_fix_dsts]: https://internals.rust-lang.org/t/pre-erfc-lets-fix-dsts/6663
[prerfc_custom_dst]: https://internals.rust-lang.org/t/pre-rfc-custom-dsts/8777
[rfc_aligned]: https://github.com/rust-lang/rfcs/pull/3319
[rfc_custom_dst_electric_boogaloo]: https://github.com/rust-lang/rfcs/pull/2594
[rfc_custom_dst]: https://github.com/rust-lang/rfcs/pull/1524
[rfc_dynsized_without_dynsized]: https://github.com/rust-lang/rfcs/pull/2310
[rfc_extern_types]: https://rust-lang.github.io/rfcs/1861-extern-types.html
[rfc_extern_types_v2]: https://github.com/rust-lang/rfcs/pull/3396
[rfc_fat_objects]: https://github.com/rust-lang/rfcs/pull/9
[rfc_not_sized_thin_pointers]: https://github.com/rust-lang/rfcs/pull/3536
[rfc_opaque_data_structs]: https://github.com/rust-lang/rfcs/pull/1993
[rfc_pointee_dynsized]: https://github.com/rust-lang/rfcs/pull/2984
[rfc_pointer_metadata_vtable]: https://github.com/rust-lang/rfcs/pull/2580
[rfc_scalable_vectors]: https://github.com/rust-lang/rfcs/pull/3268
[rfc_simd]: https://rust-lang.github.io/rfcs/1199-simd-infrastructure.html
[rfc_truly_unsized_types]: https://github.com/rust-lang/rfcs/pull/709
[rfc_unsized_types]: https://github.com/japaric/rfcs/blob/unsized2/text/0000-unsized-types.md
[rfc_virtual_structs]: https://github.com/rust-lang/rfcs/pull/5
