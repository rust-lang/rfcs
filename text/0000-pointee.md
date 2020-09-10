- Feature Name: `pointee`
- Start Date: 2020-09-09
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add two language traits `Pointee` and `DynSized` to allow for more fine-grained type sizing.

# Motivation
[motivation]: #motivation

As of right now, the lack of anything more fine-grained than `Sized` means Rust can deal with only two cases: a `Sized` type whose pointers and references are always thin, or a `!Sized` type whose pointers and references are always wide.

There is thus no way to encode extern types as in [RFC #1861], nor any custom dynamically sized type in the far future with [RFC #2594].

This RFC focuses on the necessary steps to be able to specify and implement extern types and custom dynamically sized types in the future.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Two new language traits are added to the standard library under `core::marker`:

```rust
/// Types whose values can be the target of a pointer.
///
/// As all types can be used in a pointer or reference type, this trait is implemented
/// automatically by the compiler for all types.
trait Pointee {
    /// The pointer metadata associated with values of this type.
    ///
    /// It is the unit type `()` for `Sized` and `extern` types, `usize` for slice types,
    /// and an unspecified opaque type for `dyn` types.
    ///
    /// Pointer types `*const T` and `*mut T` implement `Copy`, `Eq`, `Ord` and `Unpin`,
    /// so the pointer metadata must implement too. Furthermore, reference types `&T` and
    /// `&mut T` may implement `Send` and `Sync`, so the pointer metadata should also
    /// implement them.
    ///
    /// The list of traits should be considered non-exhaustive, as more auto traits may be
    /// added there in the future. For that reason, it is not allowed to use the metadata
    /// in a generic context as there is no way to preemptively satisfy those extensions
    /// to the trait bound.
    ///
    /// ```rust,ignore
    /// The following function does not compile successfully:
    /// fn generic_metadata<T, U>()
    /// where
    ///     T: Pointee<Meta = U> + ?Sized,
    ///     U: 'static + Copy + Eq + Ord + Send + Sync + Unpin,
    /// {}
    /// ```
    type Meta: 'static + Copy + Eq + Ord + Send + Sync + Unpin + …;
}

/// Types with a dynamic size.
///
/// All type parameters have an implicit bound of `DynSized`. The special syntax
/// `?DynSized` can be used to remove this bound and the implicit `Sized` bound
/// if it's not appropriate.
///
/// ```
/// struct A<T>(T);
/// struct B<T: ?Sized>(T);
/// struct C<T: ?DynSized>(T);
///
/// extern { type ExternType; }
///
/// struct AWithSlice(A<[i32]>); // error: Sized is not implemented for [i32]
/// struct BWithSlice(B<[i32]>); // OK
/// struct CWithSlice(C<[i32]>); // OK
///
/// struct AWithExtern(A<ExternType>); // error: Sized is not implemented for ExternType
/// struct BWithExtern(B<ExternType>); // error: DynSized is not implemented for ExternType
/// struct CWithExtern(C<ExternType>); // OK
/// ```
trait DynSized: Pointee {}
```

The existing `Sized` marker trait is also redefined to be included as the top of the trait hierarchy:

```rust
trait Sized: DynSized<Meta = ()> {}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

By language traits, we mean that `Pointee`, `DynSized` are all language items. Neither of them can be implemented by hand and only `Sized` types, `extern` types and slice types have a known `Pointee::Meta` type that can be used in trait bounds.

The name `DynSized` is chosen because `Sized: DynSized<Meta = ()>` reads as "a sized type is a dynamically sized type whose metadata doesn't contain any extra information", meaning that the type is *statically* sized.

A third language auto trait `Meta` is added to reject code trying to use generics with pointee metadata as explained in `DynSized`'s own documentation in the guide-level explanation, it is an implementation detail and not destined to become stable:

```rust
auto trait Meta {}

trait Pointee {
    // The actual trait bound includes `Meta`.
    type Meta: 'static + Copy + Eq + Ord + Send + Sync + Unpin + Meta;
}
```

## Casts

Casts from `*const T` to `*const U` are now valid for `T, U: Pointee<Meta = ()>` instead of `T, U: Sized`.

## Interaction with `extern` types

This small RFC allows users to target both `Sized` types and `extern` types generically to handle all thin references and pointers:

```rust
fn thin_pointer_to_usize<T>(ptr: *const T) -> usize
where
    T: ?DynSized + Pointee<Meta = ()>,
{
    ptr as usize
}

thin_pointer_to_usize(0xbeef as *const ()); // 0xbeef
thin_pointer_to_usize(0xcafe as *const ExternType); // 0xcafe
```

# Drawbacks
[drawbacks]: #drawback

This RFC on its own brings virtually nothing to end users.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

There is a need for a better API to handle wide pointers and references as explained in [RFC #2580], a need for `extern` types as explained in [RFC #1861], and a distant need for dynamically sized types in the future as explained in [RFC #2594]. Those three RFCs share a common core whose responsibility is to encode in the type system what exactly is a thin pointer and a wide pointer, what kind of metadata and is found in both, and whether or not they have a size that can be computed at run-time. Many of the concerns in those three RFCs are located at the intersection of those very RFCs. This one attempts to set in stone those parts so the actual features can progress.

# Prior art
[prior-art]: #prior-art

* [RFC #1861 — `extern` types][RFC #1861]
* [RFC #2580 — pointer metadata][RFC #2580]
* [RFC #2594 — custom dynamically sized types][RFC #2594]

# Unresolved questions
[unresolved-questions]: #unresolved-questions

* Name bikeshedding:
  * `DynSized`: `DynamicallySized`? `Contiguous` like in [RFC #2594]?
  * `Pointee::Meta`, `Meta`: `Metadata`?
* Should thin pointers use `()` or a custom zero-sized type for their metadata?
* Should `Pointee` and `DynSized` be added to the prelude?

# Future possibilities
[future-possibilities]: #future-possibilities

* [RFC #2580] can be slimmed down to a concrete definition of `Pointee::Meta` for trait objects and the addition of standard library functions to deconstruct pointers and references.
* [RFC #2594] can be expressed in terms of implementing `DynSized` and `Pointee` by hand.


[RFC #1861]: https://rust-lang.github.io/rfcs/1861-extern-types.html
[RFC #2580]: https://github.com/rust-lang/rfcs/pull/2580
[RFC #2594]: https://github.com/rust-lang/rfcs/pull/2594
