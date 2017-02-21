- Feature Name: array-trait
- Start Date: 2017-02-21
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

Extend and stabilize the `FixedSizeArray` trait,
as a stop-gap solution for integer parameters in generics.


# Motivation
[motivation]: #motivation

One of Rust’s basic families of types is fixed-size arrays:
`[T; N]` where `T` is any (statically-sized) type, and `N` is an (`usize`) integer.
Arrays with the same `T` item type but of different sizes are themselves different types.

A [long-standing](https://github.com/rust-lang/rfcs/pull/884)
feature [request](https://github.com/rust-lang/rfcs/issues/1038)
it the ability to write code that is generic over the size of an array.
The solution typically proposed for this it “type-level integers”,
where generic items can have integers as a new *kind* of type-level parameters,
in addition to lifetime parameters and type parameters.

However, every RFC on the subject so far has been postponed:
[1](https://github.com/rust-lang/rfcs/pull/884),
[2](https://github.com/rust-lang/rfcs/pull/1062),
[3](https://github.com/rust-lang/rfcs/pull/1520),
[4](https://github.com/rust-lang/rfcs/issues/1557),
[5](https://github.com/rust-lang/rfcs/pull/1657).

This RFC propose an addition to the language that,
although less theoretically pleasing,
is much smaller and therefore hopefully more likely to be accepted.
Additionally, it does not prevent a more general solution from also happening in the future.

Arrays are the only case where integers occur in types, in current Rust.
Therefore a solution specifically for arrays
hopefully covers many  of the use cases for type-level integers.


# Detailed design
[design]: #detailed-design

The `core::array` module currently contains a `FixedSizeArray` trait.
Both are unstable.
This RFC proposes three changes:

* Extend the `FixedSizeArray`. (This is optional, see alternatives.)
* Reexport the module as `std::array`.
* Stabilize both the module an the trait.

The current definition of the trait is:

```rust
pub unsafe trait FixedSizeArray<T> {
    /// Converts the array to immutable slice
    fn as_slice(&self) -> &[T];
    /// Converts the array to mutable slice
    fn as_mut_slice(&mut self) -> &mut [T];
}

unsafe impl<T, A: Unsize<[T]>> FixedSizeArray<T> for A {
    #[inline]
    fn as_slice(&self) -> &[T] {
        self
    }
    #[inline]
    fn as_mut_slice(&mut self) -> &mut [T] {
        self
    }
}
```

A single `impl` applies to array of all sizes, thanks to the `Unsize` trait.
That trait is also unstable.
It exists to support for [DST coercions](https://github.com/rust-lang/rust/issues/27732),
itself a somewhat complex feature that might take some time to stabilize.

This trait is already useful as-is,
but it would be nice to extend it to include the array’s length and item type
as an associated constant
and an associated type (which replaces the type parameter):

```rust
pub unsafe trait FixedSizeArray {
    // Added:
    type Item;
    const LENGTH: usize;

    // Unchanged:
    fn as_slice(&self) -> &[Self::Item];
    fn as_mut_slice(&mut self) -> &mut [Self::Item];
}
```

However these can not be provided by in `impl` based on `Unsize<[T]>` like the current one.
Instead, this RFC proposes that all array types implement the trait through “compiler magic”.
There would be no corresponding `impl` block in libcore or other library crates,
the existence of these implementations would be part of the definition of the Rust language.
There is precedent for such “magic” implementations in the language of other traits:

* `Unsize`
* `Sized`
* `Fn`, `FnMut`, `FnMove` (for closures types, which are very magical themselves)
* `Copy`, `Drop` (not implemented implicitly, but `impl`s have “magical” requirements)


# How We Teach This
[how-we-teach-this]: #how-we-teach-this

The `array` module and `FixedSizeArray` trait already have rustdoc documentation
that seems appropriate.

The trait could be mentioned in the *Primitive Types* or *Generics* chapter of the book.


# Drawbacks
[drawbacks]: #drawbacks

This an ad-hoc work-around for something that could be solved with a more general mechanism
in the future.


# Alternatives
[alternatives]: #alternatives

* Stabilize the `FixedSizeArray` trait as-is
  (without an associated type or associated constant, with a type parameter).

* Figure out type-level integers, and deprecate / remove the `FixedSizeArray` trait.


# Unresolved questions
[unresolved]: #unresolved-questions

* To extend or not to extend. (See [Alternatives](#alternatives).)

* Should the compiler prevent other implementations of the trait?
  There is precedent for that with the `Unsize` trait and error number E0328.

* Rename `FixedSizeArray` to `Array`?
  All arrays in Rust are fixed-size.
  The things similar to arrays that are not fixed-size are called slices or vectors.
