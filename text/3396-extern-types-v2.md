- Feature Name: `extern_types_v2`
- Start Date: 2023-02-19
- RFC PR: [rust-lang/rfcs#3396](https://github.com/rust-lang/rfcs/pull/3396)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)


# Summary
[summary]: #summary

Define types external to Rust and introduce syntax to declare them.
This additionally introduces the `MetaSized` trait to allow these types to be interacted with in a generic context.
This supersedes [RFC 1861].


# Motivation
[motivation]: #motivation

The motivation from [RFC 1861] for why we want extern types at all still applies.

The summary of that is that when writing FFI bindings, the foreign code may give you a pointer to a type but not provide the layout for that type.
The current approach for representing these types in Rust, suggested by the 'nomicon, is with a ZST like the following.
```rust
#[repr(C)]
pub struct Opaque {
    _data: [u32; 0],
    _marker: PhantomData<(*mut u8, PhantomPinned)>
}
```
The type is `repr(C)` so that it can be used in FFI.
The `_data` field provides the alignment of the type, and the `_marker` field causes the correct set of automatically implemented traits to be implemented, in this case `!Send`, `!Sync`, `!Unpin`, but `Freeze` (`UnsafeCell` can be used to remove the last one of those).
This works, however the compiler believes this type is statically sized and aligned which may not be true.
This means that the type can be easily misused by placing it on the stack, allocating it on the heap, calling `ptr::read`, calling `mem::size_of`, etc.
This proposal introduces extern types, a way to accurately represent these opaque types in the Rust type system such that the compiler prevents you from misusing it.

The motivation for replacing [RFC 1861] is around computing the offset of fields in aggregate types.
Extern types do not have an alignment known to the Rust compiler and therefore cannot be included as a field in an aggregate type as it is impossible to correctly calculate their offset, however that RFC did not address this.
This implies that extern types cannot be included in any (non-`repr(transparent)`) structs and so must be prevented.

Extern types should be able to be used in generic contexts so that they receive blanket trait implementations and can be used inside generic wrapper types.
Prior to this, all generic types could be included as fields of structs and it was possible to obtain the size and alignment of any type.
Extern types do not have a size or alignment so this RFC allows users to specify generic bounds sufficiently to statically prevent the above issues.


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When writing bindings to foreign code, any types that are opaque to the Rust type system should be declared as follows:
```rust
extern {
    type Foo;
}
```
This is called an extern type and is `!Sized`, `!MetaSized` (explained below), `!Send`, `!Sync`, and is FFI-safe.
Unlike other dynamically-sized types (DSTs), currently only slices and trait objects, pointers to it (`&Foo`, `&mut Foo`, `*const Foo`, `*mut Foo`, etc) are thin, that is they are 1 `usize` wide not 2.

These types cannot be included in structs as Rust is not able to compute the offset of the field because it does not know the alignment.
```rust
struct Header {
    data: u8,
    tail: Foo, // Error due to unknown offset
}
```

However, these types may be placed in `repr(transparent)` structs:
```rust
#[repr(transparent)]
struct Wrapper<T> {
    inner: Foo,
    _marker: PhantomData<T>,
}
```

The lack of the `Sized` and `MetaSized` traits on these structs prevent you from calling `ptr::read`, `mem::size_of_val`, etc, which are not meaningful for opaque types.

In the 2021 edition and earlier, these types cannot be used in generic contexts as `T: Sized` and `T: ?Sized` both imply that `T` has a computable size and alignment.

In the 2024 edition and later, `T: ?Sized` no longer implies any knowledge of the size and alignment so opaque types can be used in generic contexts.
If you require your generic type to have a computable size and alignment you can use the bound `T: ?Sized + MetaSized`, which will enable you to store the type in a struct.

The automated tooling for migrating from the 2021 edition to the 2024 edition will replace `?Sized` bounds with `?Sized + MetaSized` bounds.


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Some nomenclature to assist the rest of this explanation:

Types have a size and alignment that is known:
* statically - the size/alignment is known by the Rust compiler at compile time. This is the current `Sized` trait.
  Most types in Rust are statically sized and aligned, like `u32`, `String`, `&[u8]`.
* from metadata - the size/alignment can be derived purely from pointer metadata without having to inspect or dereference the pointer.  
  All remaining types fit in this category and are DSTs.
  `[u8]` has a statically known alignment but the size can only be determined from the pointer metadata, `dyn Debug`'s size and alignment are both obtained from the vtable in the pointer metadata.
* dynamically - the size/alignment can only be determined at run time.
  There are no types currently expressible in the language with dynamically known size or alignment.  
  The most discussed potential type in this category is `CStr`, which has a statically known alignment but it's size can only be determined by iterating over it's contents to find the position of the null byte.
  Note that these types are odd, for example determining the size of a `Mutex<CStr>` requires taking a lock on the mutex.
* unknown - the size/alignment is not able to be determined at compile time or run time.  
  This is the category that opaque types fall in (and no other existing types occupy), without any additional domain specific knowledge.
  Therefore extern types will occupy this category to allow the most flexibility.

The rest of this document will refer to types as "statically sized", "metadata aligned", etc.

"dynamically aligned" (or "unknown aligned") types cannot be placed as the last field of a struct as their offset cannot be determined, without already having a pointer to the field.

In the Rust 2021 edition and earlier `T: Sized` implies that `T` is "statically sized" and "statically aligned" and `T: ?Sized` implies that `T` is "metadata sized" and "metadata aligned".

In the Rust 2024 edition and later `T: Sized` means the same but `T: ?Sized` implies that `T` is "unknown sized" and "unknown aligned".
A `MetaSized` bound can be introduced to regain the previous meaning.

## `MetaSized` trait
[metasized-trait]: #metasized-trait

This introduces a new trait `core::marker::MetaSized` that represents a type that is "metadata sized" and "metadata aligned":
```rust
#[lang = "meta_sized"]
trait MetaSized {}
```
This trait is automatically implemented for all types except extern types.

All types that implement `MetaSized` can be placed as the last field in a struct because the compiler can emit code to determine the offset of the field purely from the pointer metadata.
At the time of writing all `MetaSized` types are either statically aligned, or are trait objects and have their alignment in their vtable.
This RFC does not propose changing codegen for computing the offset as it does not introduce any new `MetaSized` types.

All locations in the standard library that use `?Sized` bounds need to be reviewed when migrating to the 2024 edition and replaced with either `?Sized` or `?Sized + MetaSized` as appropriate. Some examples:
```rust
pub fn size_of_val<T: ?Sized + MetaSized>(val: &T) -> usize
```
This requires `MetaSized` because the size must be known at run time.

```rust
pub struct Box<T: ?Sized + MetaSized, A: Allocator = Global>(Unique<T>, A);
```
This requires `MetaSized` because the `Box` must be able to determine the Layout of the type at run time to allocate and free the backing memory.

```rust
impl<A: ?Sized, B: ?Sized> const PartialEq<&B> for &A
where
    A: ~const PartialEq<B>,
```
This does not require `MetaSized` because the references to `A` and `B` always remain behind a pointer and the size and alignment is not required to call a function on the type.

## Extern types
[extern-types]: #extern-types

Extern types are defined as above, they are thin DSTs, that is their metadata is `()`. They cannot ever exist except behind a pointer, and so attempts to dereference them fail at compile time similar to trait objects.

`repr` attributes are not permitted on extern types as none of the existing representations are applicable.

Extern types do not automatically implement any traits (except `Pointee`), but users can manually implement, for example, `Send`.
This does mean that all extern types will be `!Freeze` as it is private, however this is a compiler internal that is only used for optimisations, so this only causes some potential missed optimisations.

An extern type can be included in a `repr(transparent)` struct as it is always at offset 0.
The transparent struct is then also a thin DST and inherits traits as normal.


# Drawbacks
[drawbacks]: #drawbacks

This introduces language complexity for a feature that most users will not use directly.

(Copied from [RFC 1861])
The syntax has the potential to be confused with introducing a type alias, rather than a new nominal type. The use of extern here is also a bit of a misnomer as the name of the type does not refer to anything external to Rust.


# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Do nothing

Doing nothing is not a compelling option here, [RFC 1861] is merged but is not implementable.
At minimum we should mark that RFC as deprecated/unimplemented and remove extern types from the compiler and docs.
However, the fact that RFC was merged is a very strong indication that something is needed here, and the current workaround with sized types is not good enough.

## No generics

This design aims to be minimal while still allowing extern types to feel like a fully supported part of the language, namely retaining the ability for them to be used in generics.
A simpler alternative would be to not allow extern types in generics, this would mean not adding `MetaSized` and retaining the existing meaning of `?Sized`.
This would severely restrict the utility of extern types and would prevent them from implementing many useful traits that have blanket implementations in the standard library.

## More traits

We could introduce a large portion of the traits implied by the "dynamically/metadata/statically sized/aligned" nomenclature above.
This would allow the most flexibility about exactly what a generic needs but existing usage suggests that this is not necessary and would be a significant amount of language complexity.

## Post-monomorphisation errors

We could allow extern types to be used in generics without the `MetaSized` machinery and simply generate a post-monomorphisation error when the compiler is not able to lay out a type or a function attempts to call `size_of_val` or `align_of_val`.
This would be a significant departure from the compilers approach to generics, but is not entirely unprecedented as these sorts of errors can be generated by const evaluation.

## Opt-out trait bound - `?MetaSized`

This change could be made in an entirely backwards compatible way by leaving the existing meaning of `?Sized` alone and allowing relaxing the implied `MetaSized` bound with `T: ?Sized + ?MetaSized` (or possibly just `T: ?MetaSized`).
This is not suggested by this document because the lang team has historically said that they do not wish to add any more opt-out bounds.

## Trait methods

`MetaSized` could include `size_of_val` and `align_of_val` rather than relying on the free functions:
``` rust
trait MetaSized: Pointee {
    fn size_of_val(metadata: <Self as Pointee>::Metadata) -> usize;
    fn align_of_val(metadata: <Self as Pointee>::Metadata) -> Alignment;
}
```
This is the more obvious way of writing the trait, however it would prevent splitting `MetaSized` and `MetaAligned` in the future and adding these functions later would be hard but probably not impossible.


# Prior art
[prior-art]: #prior-art

There is a lot of prior art in rust itself from previous attempts at custom DSTs, the `DynSized` trait, and some other things related to "exotically sized types".
- [Lang team design notes on exotically sized types](https://github.com/rust-lang/lang-team/blob/master/src/design_notes/dynsized_constraints.md).  
  This document contains notes from the lang team about what `?Sized + DynSized` needs to imply.
  This document outlines `DynSized`, `MetaSized`, and `Sized` which inspired the "metadata sized" and friends in this RFC.
  I believe this RFC satisfies the constraints outlined, mostly by dropping `DynSized` as an available bound, which limits expressiveness in favour of simplicity.
- [Custom DSTs](https://github.com/rust-lang/rfcs/pull/2594).  
  This postponed RFC attempts to introduce a generic framework for DSTs with arbitrary metadata.
  This RFC aims to be compatible with future attempts at custom DSTs, as it is in essence a very restricted form.
- [DynSized without ?DynSized](https://github.com/rust-lang/rfcs/pull/2310).  
  This contains a lot of very useful analysis of what exactly commonly used crates want when they state `?Sized`.
  However, this RFC aims to be simpler than the lint-based solution presented there.
  Additionally, this deals with not being able to place extern types as fields in structs, rather than solely on `size_of_val` and `align_of_val`.
- [More implicit bounds](https://github.com/rust-lang/rfcs/issues/2255).  
  This discusses whether we should be adding more implicit bounds into the language, and there associated relaxations (like `?MetaSized`).
  This RFC aims to sidestep this issue by utilising the edition system to change the definition of `?Sized`.


# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Should "metadata sized" imply "metadata aligned" or should we be adding the `MetaAligned` trait rather than `MetaSized`?
- Should `MetaSized` be a supertrait of `Sized`? All `Sized` things are `MetaSized` but `Sized` doesn't semantically require `MetaSized`.
- Should users be able to slap a `#[repr(align(n))]` attribute onto opaque types to give them an alignment?  
  This would allow us to represent `CStr` properly but would necessitate splitting `MetaSized` and `MetaAligned` as it is only "dynamically sized" but "statically aligned".
  (We may be able to get away with the [Aligned trait](https://github.com/rust-lang/rfcs/pull/3319))
- Should the `extern type` syntax exist, or should there just be a `repr(unsized)`?  
  This would allow headers with opaque tails (which are very common in C code) but is a more significant departure from the original RFC, and looks more like custom DSTs.


# Future possibilities
[future-possibilities]: #future-possibilities

The most obvious future possibility is custom DSTs.
This would provide a mechanism for allowing users to implement types with entirely custom metadata and therefore custom implementations of `size_of_val` and `align_of_val`.
This would likely mean that users could implement types that acted like extern types without using the `extern type` syntax.
This should not be an issue as `extern type` communicates the intent of these types well, and guarantees FFI-safety.


[RFC 1861]: https://rust-lang.github.io/rfcs/1861-extern-types.html
