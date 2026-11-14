- Feature Name: `num_wrapping_from`
- Start Date: 2024-09-13
- RFC PR: [rust-lang/rfcs#3703](https://github.com/rust-lang/rfcs/pull/3703)
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

Having this in the standard library is implicitly a request to add implementations of these to lots of ecosystem crates.


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

## What about just inherent methods on integers?

We could go through and add 12 `.as_u8()`/`.as_isize()`/etc inherent methods to integers.
Subjectively that's kinda messy, though, and doesn't help with the generics cases,
nor the external types like `num::BigUint`.  When there's that many different cases,
it'd be nicer to do it via the type system instead of copying type names into method names.

This RFC was opened as an RFC to try to address the broader problem in a way that the ecosystem can use.
It would be a failure of the RFC, from the perspective of its author, to end up with just a few inherent methods.
If that's the way forward, it means closing this RFC as not-planned and having a separate ACP instead.

There's probably space for *some* inherent methods on integers, however,
in order to address particularly-common cases where not needing to write out a type would help.
See, for example, [ACP#453] proposing inherent methods for converting `iNN`↔`uNN`.
Those are a different question from this RFC, however.

[ACP#453]: https://github.com/rust-lang/libs-team/issues/453

## What about using `From<Wrapping<T>>` instead of a new trait?

Coherence makes this one less appetizing.

With a new trait, one can do this outside `core`
```rust
impl WrappingFrom<&BigInteger> for u32 {
    fn wrapping_from(x: &BigInteger) -> u32 { … }
}
```

whereas with `num::Wrapping` you get an error like
```text
error[E0117]: only traits defined in the current crate can be implemented for primitive types
 --> src/lib.rs:4:1
  |
4 | impl From<Wrapping<&BigInteger>> for u32 {
  | ^^^^^---------------------------^^^^^---
  | |    |                               |
  | |    |                               `u32` is not defined in the current crate
  | |    `Wrapping` is not defined in the current crate
  | impl doesn't use only types from inside the current crate
  |
  = note: define and implement a trait or new type instead
```

Maybe it'd be possible to mark `Wrapping` as `#[fundamental]` and thus allow crates to add impls like that,
but that's a much bigger hammer than just adding a new trait for it.

Also, having a separate trait that's just about numerics means that the error message when something isn't implemented
can talk just about the implementations for that trait, rather than potentially giving you the giant list of every
`From` that probably includes a bunch of irrelevant ones.  (That said, smart diagnostics could mitigate this too.)

## Can we implement this for `NonZero<_>`?

Trivially there cannot be a blanket impl
```rust
impl<T: WrappingFrom<U>, U> WrappingFrom<NonZero<U>> for NonZero<T> { … }
```
because `NonZero<u8>::wrapping_from(NonZero(0x100_u16))` would give `0`, which is disallowed.

Thus what, if anything, to do here is a philosophical question about the intent of `NonZero` and `WrappingFrom`.

This RFC doesn't propose any implementations for `NonZero` as part of the initial change.  People can use
```rust
NonZero::new(u8::wrapping_from(x.get()))
```
or similar if they want the version that gives an `Option`, which seems fine for now.

There's at least two possible way we could go here:

1. Convert to `Option`

We could plausibly have something like
```rust
impl<T: WrappingFrom<U>, U> WrappingFrom<NonZero<U>> for Option<NonZero<T>> { … }
```
which calls `NonZero::new` internally.

That seems like it'd be annoying to use, though, since the caller would have
to write out the `Option::<NonZero<u8>>::wrapping_from(blah)`.

2. Use the non-zero lattice instead

It turns out that
- `NonZero<u8>` has 2⁸ - 1 = 255 × 1 values
- `NonZero<u16>` has 2¹⁶ - 1 = 255 × 257 values
- `NonZero<u32>` has 2³² - 1 = 255 × 16843009 values
- `NonZero<u64>` has 2⁶⁴ - 1 = 255 × 72340172838076673 values
- `NonZero<u128>` has 2¹²⁸ - 1 = 255 × 1334440654591915542993625911497130241 values

(This works for anything that's a multiple of octets, as `0xFF…FF = 0xFF * 0x01…01`.)

So there is, in fact, a coherent wrapping arithmetic on `NonZero` itself.

It would thus have
```rust
assert_eq!(<NonZero<u8>>::wrapping_from(NonZero(  1_u16)), NonZero(  1_u8));
assert_eq!(<NonZero<u8>>::wrapping_from(NonZero(  2_u16)), NonZero(  2_u8));
assert_eq!(<NonZero<u8>>::wrapping_from(NonZero(254_u16)), NonZero(254_u8));
assert_eq!(<NonZero<u8>>::wrapping_from(NonZero(255_u16)), NonZero(255_u8));
assert_eq!(<NonZero<u8>>::wrapping_from(NonZero(256_u16)), NonZero(  1_u8));
assert_eq!(<NonZero<u8>>::wrapping_from(NonZero(257_u16)), NonZero(  2_u8));
assert_eq!(<NonZero<u8>>::wrapping_from(NonZero(509_u16)), NonZero(254_u8));
assert_eq!(<NonZero<u8>>::wrapping_from(NonZero(510_u16)), NonZero(255_u8));
assert_eq!(<NonZero<u8>>::wrapping_from(NonZero(511_u16)), NonZero(  1_u8));
assert_eq!(<NonZero<u8>>::wrapping_from(NonZero(512_u16)), NonZero(  2_u8));
```
and analogously for other types.

This would make sense if we decided to add a `wrapping_add` method to `NonZero`
such that `NonZero(255_u8).wrapping_add(1)` → `NonZero(1_u8)`.

But it's not clear that we *want* to do that.  It might be that we want people
to think about `NonZero` as behaving like the underlying type, just with a value
restriction, rather than a different cycle of values.

As such, this RFC doesn't propose this directly, just like how we don't have
`NonZero<u32>::wrapping_add` even as unstable.  So far we only have methods on
`NonZero` which don't need to resolve that question: `saturating_add` and
`checked_add` for *unsigned* types only, which by only being able to
strictly-increase a value work the same as on the underlying type.

An implementation of `wrapping_from` would need to deal with that issue, so this
RFC leaves it as something to consider in the future.


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
