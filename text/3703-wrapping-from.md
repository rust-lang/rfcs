- Feature Name: `num_wrapping_from`
- Start Date: 2024-09-13
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add a `core::num::WrappingFrom<T>` trait for converting between integer types which might truncate.


# Motivation
[motivation]: #motivation

Today you can convert between arbitrary primitive integer types with `as`.

But there's a few ways in which it'd be nice to have something else:

1. You can't use it in generic code.
2. You can't use it with non-primitive types like `BigInteger`.
3. `as` can do enough other things -- like the [evil] `i32::max as u32` -- that you might want to
   more clearly communicate the intent by using a more specific API.

[evil]: https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=bddbeb692d342b8ebfefa1d25be9275d

This PR thus proposes a new `WrappingFrom` trait for these scenarios:

1. You can `where T: WrappingFrom<U>` in your generics.
2. The crate defining `BigInteger` can `impl WrappingFrom<BigInteger> for u32 { … }` and such.
3. You can write `u32::wrapping_from(x)` for a conversion if you prefer, and `u32::from(i32::max)` just won't compile.


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

We've seen the `From` trait already, which is for *lossless* and *value-preserving* conversions.
With integers, though, you might sometimes want *truncating* conversions that you can't do via `From`.
For that, you can use the `num::WrappingFrom` trait instead.

`WrappingFrom` is specific to *numbers*.  You can't use it for things like `String: From<&str>` which we've used a bunch.

It's is implemented for the numeric widening conversions that also exist in `From`.
So `i128: From<i32>` and `i128: WrappingFrom<i32>` both exist.

But while `i32: From<i128>` intentionally doesn't exist -- as it'd be lossy -- there *is*
an `i32: WrappingFrom<i128>` implementation which truncates to just give the bottom 32 bits.


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

```rust
// in core::num (and std::num)

/// Performs potentially-lossy conversions in a quantized numeric lattice.
///
/// For an `impl WrappingFrom<T> for U`, if a conceptual value *x* is representable
/// exactly in both `T` and `U`, then `<U as WrappingFrom<T>>::wrapping_from(x)` must be value-preserving.
///
/// If `impl WrappingFrom<T> for U` exists, then `impl WrappingFrom<U> for T` should also exist.
///
/// For an integer `a: Big` and a smaller integer type `Small`, truncating with `wrapping_from`
/// commutes with `wrapping_add`, so `Small::wrapping_from(a).wrapping_add(b)` and
/// `Small::wrapping_from(a.wrapping_add(b))` will give the same result.
pub trait WrappingFrom<T> {
    fn wrapping_from(value: T) -> Self;
}

impl i8/i16/i32/i64/i128/isize/u8/u16/u32/u64/u128/usize for i8/i16/i32/i64/i128/isize/u8/u16/u32/u64/u128/usize {
    fn wrapping_from(value: …) -> … {
        value as _
    }
}
```


# Drawbacks
[drawbacks]: #drawbacks

This could be left to an ecosystem crate instead of the standard library.

Having this in the standard library is implicitly a request to add implemenetations of these to lots of ecosystem crates.


# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## No reflexive impl

There could be a blanket `impl<T> WrappingFrom<T> for T { … }` implementation.

That's left out mostly because that adds it for non-numeric things as well.

But also there are various `From` cases where the reflexive impl causes problems,
so leaving it off here might be nicer.  For example, if we ever wanted to add an
implementation for `Option`, that'll be easier without a reflexive impl.

## No super-trait

We could require something like `trait WrappingFrom<T> : TryFrom<T> { … }`.

Without having any provided methods, though, there seems to be minimal value to such a restriction.
Yes, you probably *should* implement `TryFrom` for these cases too, but we don't need to *require* it.

Generic methods can add the extra bound if needed.

## No *technical* symmetry requirement

If there'a `A: WrappingFrom<B>`, then there *should* be an `B : WrappingFrom<A>` as well.

That can be difficult with coherence, though, so it's left as a *should* in the documentation,
rather than having some technical enforcement in the trait system.

Later helper methods might choose to bound on `where A: WrappingFrom<B>, B: WrappingFrom<A>`,
if that's helpful to them, but the basic primitive won't.


# Prior art
[prior-art]: #prior-art

This is obviously inspired by the `From` and `TryFrom` traits.  (The latter being why there's no `num::CheckedFrom` needed.)

Much discussion related to the general idea happened in [RFC PR 2484](https://github.com/rust-lang/rfcs/pull/2484).

The `num_traits` crate has a [`FromPrimitive`](https://docs.rs/num-traits/latest/num_traits/cast/trait.FromPrimitive.html) trait.

C++ has a `static_cast` primitive which, like Rust's `as`, can do a great many things.  Boost added a [`numeric_cast`]
function to be more restrictive, though it's more like `T::try_from(x).unwrap()` than like the `wrapping_from` proposed here.

[`numeric_cast`]: https://live.boost.org/libs/numeric/conversion/doc/html/boost_numericconversion/improved_numeric_cast__.html


# Unresolved questions
[unresolved-questions]: #unresolved-questions

(None yet known.)


# Future possibilities
[future-possibilities]: #future-possibilities

Once the trait exists, there's many possible different ways to expose it.

There could, for example, be some sort of `.truncate()` method similar to `.into()`,
perhaps tweaked for better turbofishing.

There could be convenience methods that check the conversion in debug builds,
while using the wrapping conversion in release.

There could be more implementations, like one on `Option`.

But the goal for this RFC is to have the core building block, then we can figure
out exactly how to provide convenience methods over it later.
