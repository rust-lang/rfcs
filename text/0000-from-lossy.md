- Feature Name: from-lossy
- Start Date: 2018-06-22
- RFC PR: https://github.com/rust-lang/rfcs/pull/2484
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add `FromLossy`, `TryFromLossy` and `WrappingFrom` traits.

Discuss the bigger picture of conversions and the `as` keyword.

Specify that `From` implementations must not only be *safe*, but also *exact*
(homomorphisms).

# Motivation
[motivation]: #motivation

Currently many numeric conversions can only be done via the `as` keyword or third-party libraries. To [quote @scottmcm](https://internals.rust-lang.org/t/lossy-conversion-trait-as/7672/4?u=dhardy):

> I’m strongly against [a trait] that’s just “whatever as does”, since I find it an unfortunate mess of conversions right now – some coercions, some extensions, some truncations, …

This has several problems:

- `as` can perform several different types of conversion (as noted) and is therefore error-prone
- `as` conversions must be infallible, which allows no correct result for some
  operations, for example when converting very large floats to integers (see
  [this issue](https://github.com/rust-lang/rust/issues/10184))
- since `as` is not a trait, it cannot be used as a bound in generic code (excepting via `num_traits::cast::AsPrimitive`)
- these conversions are mostly primitive instructions and are very widely used, so requiring users to use another crate like `num` is unexpected

Several types of conversion are possible:

- safe, exact conversions (e.g. `u32` → `u64`) are handled by the `From` trait ([RFC 529](https://github.com/rust-lang/rfcs/pull/529))
- fallible conversions (e.g. `u32` → `i8`) are handled by the `TryFrom` trait ([RFC 1542](https://github.com/rust-lang/rfcs/pull/1542))
- lossy conversions (e.g. `i64` → `f32`)
- lossy fallible conversions (e.g. `f64` → `u64`)
- truncations on unsigned integers (e.g. `u64` → `u32`, dropping unused high bits)
- sign-ignoring coercions/transmutations (e.g. `i8` → `u8`, `i64` → `i32`);
  these can yield totally different values due to interpretation of the sign
  bit (e.g. `3i32 << 14` is 49152=2^14+2^15; converting to `i16` yields -16384=2^14-2^15)
- conversions between types with platform-dependent size (i.e. `usize` and `isize`)

This RFC is principally concerned with lossy conversions, but considers other
conversions for a broader picture.

It is *my opinion* that we should cover all *common* conversions performed by
the `as` keyword redundant with `std` library functionality, and *consider*
deprecating `as` eventually. Whether this entails traits covering all the above
conversions or methods on primitive types for some conversions, or even leaves
no alternative to `transmute` in some cases is beyond the scope of this RFC.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

We use informal definitions here using the word *lossy*, implying that some
precision may be lost. *All conversions should preserve value at least
approximately* (excepting `WrappingFrom` which has a different interpretation).

Type conversions can be handled by the following traits:

- `From` for infallible, exact conversions (e.g. widening, `u8` → `u16`)
- `TryFrom` for fallible, exact conversions (e.g. narrowing, `u16` → `u8`, and signed, `i16` → `u16`)
- `FromLossy` for infallible, lossy conversions (mostly concerning floating-point types, e.g. `u32` → `f32`)
- `TryFromLossy` for fallible, inexact conversions (e.g. `f32` → `u32`)
- `WrappingFrom` for truncations (e.g. `u16` → `u8`) and sign coercions (e.g. `u16 → i16`)

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This concerns traits in the `std::convert` module: the existing `From` and
`TryFrom`, as well as some new traits.

## (Approximate) semantic value preserving conversions

### `From` trait

Tweak the documentation to clarify that `From` implementations should not lose
precision (i.e. should be injective):

```rust
/// Simple and safe type conversions to `Self`. It is the reciprocal of `Into`.
/// 
/// Implementations should not lose precision. That is, given `from` mapping
/// from type `S` to type `T` (`from: S ↦ T`), it should be possible to define
/// an inverse mapping `g: T ↦ S` such that for all `x ∈ S`, `g(from(x))` is
/// equivalent to `x`. This implies that `from` must be injective. Note that
/// function `g` may be extended to a `TryFrom<T> for S` implementation,
/// though this is not required.
/// 
/// Where `S: Eq`, we use `Eq` as our equivalence relation; otherwise, where
/// `S: PartialEq`, we form an equivalence relation consistent with `PartialEq`
/// over the subset of elements where `PartialEq` is reflexive (i.e. for `x`
/// where `x.eq(x)`), and consider all other elements to be in an "other"
/// subset `O` such that for `x, y ∈ O, x = y` and `x ∈ O, y ∉ O, x ≠ y`.
..
pub trait From<S> {
    fn from(S) -> Self;
}
```

Nightly rust currently has several implementations of `From` on SIMD types
which should be removed (e.g. `f32x4` → `i8x4` (fallible) and `u64x4` → `f32x4`
(lossy i.e. not injective)).

### `TryFrom` trait

This trait currently lacks documentation. It should have similar requirements
to the `From` trait:

```rust
/// Potentially fallible type conversions to `Self`. It is the reciprocal of
/// `TryInto`.
/// 
/// Currently `From` is a special case of `TryFrom`. Where conversions never
/// fail, `From` should be used instead.
/// 
/// Successful conversions should not lose precision. That is, given `try_from`
/// mapping from type `S` to type `T` (`try_from: S ↦ T`), it should be possible
/// to define an inverse mapping `g: T ↦ S` such that for all `x ∈ S`, if
/// `try_from(x)` does not fail, then `g(try_from(x))` is equivalent to `x`.
/// This implies that `try_from` must be injective on the subset of the source
/// type `S` where conversions succeed. Note that function `g` may be extended
/// to a `TryFrom<T> for S` implementation, though this is not required.
/// 
/// Where `S: Eq`, we use `Eq` as our equivalence relation; otherwise, where
/// `S: PartialEq`, we form an equivalence relation consistent with `PartialEq`
/// over the subset of elements where `PartialEq` is reflexive (i.e. for `x`
/// where `x.eq(x)`), and consider all other elements to be in an "other"
/// subset `O` such that for `x, y ∈ O, x = y` and `x ∈ O, y ∉ O, x ≠ y`.
..
pub trait TryFrom<S> {
    type Error;
    fn try_from(value: S) -> Result<Self, Self::Error>;
}
```

It would be nice to be able to make `TryFrom` an extension of `From`.
Unfortunately this would be a potentially breaking change since more
concrete implementations of both traits may exist for some types; we therefore
propose to add a default implementation once
[specialisation](https://github.com/rust-lang/rfcs/pull/1210) is ready:
```rust
impl<S, T> TryFrom<S> for T where From<S>: T {
    default type Error = !;
    default fn try_from(value: S) -> Result<Self, Self::Error> {
        Ok(T::from(value))
    }
}
```

### `FromLossy` trait

Add `std::convert::FromLossy`:

```rust
/// A trait for conversions which may lose precision.
/// 
/// Like `From`, implementations should not fail.
/// 
/// Unlike `From`, implementations may lose precision, however, all results
/// should be approximations to the input value or are an appropriate special
/// value. For example, it is reasonable for an implementation converting `f64`
/// to `f32` to lose precision and to return positive or negative infinity for
/// out-of-range results, as well as to drop payloads from NaN values.
/// 
/// We do not specify the rounding mode used by implementations, but all results
/// which are not special values should be numerically *close to* the input
/// value. If `x` is the input value and `u` is the precision of the result type
/// at `x`, then it should normally hold that `|x - from_lossy(x)| < u`. The
/// precision `u` may be constant (as in integer types) or variable (as in
/// floating point types) but should be signficantly smaller than the magnitude
/// of x (`u << |x|`) and should be consistent with typical values for the
/// result type.
/// 
/// If the mapping has no suitable approximate value for some inputs and no
/// special value which may be used instead, then conversion should be
/// implemented using the `TryFromLossy` trait instead.
pub trait FromLossy<S> {
    fn from_lossy(x: S) -> Self;
}
```

Add implementations to `f64` from all of:

- `u64, i64, u128, i128, usize, isize`

and to `f32` from all of:

- `f64, u32, i32, u64, i64, u128, i128, usize, isize`

These conversions should round to the nearest representable value, with ties to
even (as is commonly used for floating-point operations).

Note that for the platform-dependent types `usize` and `isize`, some conversions
will always be exact on some platforms (i.e. to `f64` on 32-bit platforms, and
to both float types on 16-bit platforms). We consider this acceptable despite
overlap with a potential `From` implementation in such cases.

(Note: other integer → float conversions are already handled by `From` since
they are loss-less. There is a question below about trait overlap.)

### `TryFromLossy` trait

Add `std::convert::TryFromLossy`:

```rust
/// A trait for conversions which may fail and may lose precision.
/// 
/// Like `FromLossy`, implementations may lose precision. Like `TryFrom`,
/// conversions may fail on some input values.
/// 
/// Implementations should fail when the result type has no reasonable
/// approximation of the input type and no appropriate special value (such as a
/// representation of overflown or non-numeric values); otherwise, the
/// conversion should succeed with an approximation of the input value.
/// 
/// We do not specify the rounding mode used by implementations, but all results
/// where `try_from_lossy` does not fail and where the result is not a special
/// value should be numerically *close to* the input value. That is, if `x` is
/// the input value and `u` is the precision of the result type at `x` and
/// `Ok(r) = try_from_lossy(x)`, then it should normally hold that
/// `|x - try_from_lossy(x).unwrap()| < u`. The precision `u` may be constant
/// (as in integer types) or variable (as in floating point types) but should be
/// signficantly smaller than the magnitude of x (`u << |x|`) and should be
/// consistent with typical values for the result type.
pub trait TryFromLossy<S> {
    type Error;
    fn try_from_lossy(x: S) -> Result<Self, Self::Error>;
}
```

As with `TryFrom`, we propose to add a default implementation once
specialisation is available:
```rust
impl<S, T> TryFromLossy<S> for T where FromLossy<S>: T {
    default type Error = !;
    default fn try_from_lossy(value: S) -> Result<Self, Self::Error> {
        Ok(T::from_lossy(value))
    }
}
```

Additionally, define implementations from all of:

- `f32, f64`

to all of:

- `u8, u16, u32, u64, u128`
- `i8, i16, i32, i64, i128`

(Note: `f32` → `u128` is infallible (but still lossy) *if* the input is not
negative, infinite or an NaN. So even though the output type has large enough
range, this conversion trait is still applicable.)

The implementations should fail on NaN, Inf, and values whose integer
approximation is not representable. The integer approximation should be the
value rounded towards zero. E.g.:

- 1.6f32 → u32: 1
- -1f32 → u32: error
- -0.2f32 → u32: 0
- 100_000f32 → u16: error

These implementations should use the following error type, defined in
`std::num` (as noted by @SimonSapin, this allows modification later):

```rust
struct TryFromFloatError{
    _dummy: ()
};
```


## Other conversions

### `WrappingFrom` trait

There are several types of integer conversion which do not preserve the
semantic value:

-   sign transmutation (i.e. reinterpretation as another integer type of the
    same size but with different signed-ness), e.g. `128u8 → -128i8`
-   unsigned truncation (simply dropping excess high bits), e.g. `260u16 → 4u8`
-   signed truncation (dropping excess high bits and sign-transmuting the
    remainder), e.g. `-260i16 → -4i8`

Note that there is less incentive against usage of `as` in these cases since the
conversions do not preserve "value", although alternatives still have some use
(e.g. to clarify that a conversion is a truncation).

We can add `std::convert::WrappingFrom`:

```rust
/// A trait for wrapping type conversions with behaviour equivalent to the `as`
/// operator on integer types.
/// 
/// Implementations convert values between fixed size integer types by
/// truncating, extending and re-interpreting the underlying bits:
/// 
/// -   if the target type is smaller than the source type, excess high bits are
///     dropped
/// -   if the target type is larger than the source type then, for unsigned
///     source types, the value is zero-extended; for signed source types, the
///     value is sign-extended
/// -   once the value has the same size as the target type, it is reinterpreted
/// 
/// Equivalently, the conversion results can be defined numerically (where `n`
/// is the initial value and `M` is the number of distinct values in the target
/// type):
/// 
/// -   for unsigned target, `n % M`
/// -   for signed target, `if n % M < M/2 { n % M } else { n % M - M }`
pub trait WrappingFrom {
    fn wrapping_from(x: T) -> Self;
}
```

Add implementations `impl WrappingFrom<U> for V` for all pairs of integer types
`U, V` (including where `U = V`, and including the platform-dependent types
`usize` and `isize`).

This has overlap with the `From` and `TryFrom` traits. To quote @scottmcm:

> I believe it should have all pairs for the integer types, because a wrapping
> conversion from u8 to u16 has perfectly-well defined semantics. (Though I'd
> also like a clippy lint suggesting `from` instead where possible.) And I think
> it could be useful for machine-generated code as mentioned in
> [#2438 (comment)](https://github.com/rust-lang/rfcs/pull/2438#issuecomment-403255258).

## Core lib

`core` is the subset of `std` which is applicable to all supported platforms.
Since it already includes all types mentioned here as well as `From` and
`TryFrom`, the traits and implementations proposed by this RFC should in fact
be placed in the `core` lib with aliases in `std`.

# Related problems

These problems are discussed in the search for a complete solution; however it
is not currently proposed to solve them within this RFC.

## Platform-dependent types

### isize / usize

The `isize` and `usize` types have undefined size, though there appears to be
an assumption that they are at least 16 bits (existing `From` implementations)
and that they could be larger than 64 bits.
[Discussion thread on internals](https://internals.rust-lang.org/t/numeric-into-should-not-require-everyone-to-support-16-bit-and-128-bit-usize/3609/7).
[Discussion about `usize` → `u64` on users forum](https://users.rust-lang.org/t/cant-convert-usize-to-u64/6243).

Checked integer conversions using `TryFrom` [are being reintroduced](https://github.com/rust-lang/rust/issues/49415).

It remains to be decided whether `From` should support conversions such as
`From<usize> for u32` on some platforms only, or not at all. For this reason,
the platform-dependent types are currently also excluded from the `FromLossy`
trait.

# Drawbacks
[drawbacks]: #drawbacks

This RFC proposes at least two new conversion traits and *still* doesn't solve
all conversion problems (notably, platform-depedent types).

It also makes numeric conversions complex, and *deliberately does so*. This does
of course make the language more difficult to learn. Is there a simpler option?
The current undefined behaviour when converting from floating-point types
highlights the problems with a very generic solution.

# Rationale and alternatives
[alternatives]: #alternatives

As mentioned, the `as` keyword has multiple problems, is not Rustic, and we
should aim to provide users with *safer* alternatives.

This RFC mostly approaches the problem via conversion traits, though as
highlighted with the suggested `transmute_sign` method, traits are not the only
approach. Since multiple types of conversion have multiple possible target types
for each initial type, traits are however appropriate.

There is also the possibility to leave this type of conversion trait to external
libraries. Given how widely useful some of these conversions are (especially
those to/from floating-point types), it would be desireable to have a `std`-lib
solution.

As a simplification, all integer transmutation conversions discussed above could
be handled by a single trait, as a "safe" alternative to `mem::transmute`.

# Prior art
[prior-art]: #prior-art

Rust-specific:

- Conversion traits: [RFC 529](https://github.com/rust-lang/rfcs/pull/529)
- `TryFrom` trait: [RFC 1542](https://github.com/rust-lang/rfcs/pull/1542)
- internals thread: https://internals.rust-lang.org/t/lossy-conversion-trait-as/7672/4
- internals thread: https://internals.rust-lang.org/t/numeric-into-should-not-require-everyone-to-support-16-bit-and-128-bit-usize/3609/7
- users thread: https://users.rust-lang.org/t/cant-convert-usize-to-u64/6243
- [`num_traits::cast::cast`](https://docs.rs/num-traits/0.2.4/num_traits/cast/fn.cast.html)

C++ tries to add some degree of explicitness with [`static_cast`](https://en.cppreference.com/w/cpp/language/static_cast) etc., but with much less granularity than discussed here.

# Unresolved questions
[unresolved]: #unresolved-questions

**Should we add `FromLossy` implementations from `usize`/`isize` types?**

I suspect it is better to leave this decision until another RFC solves
conversions on those types.

**Should the infallible traits be special cases of the fallible traits?**

@newpavlov points out that `From<T>` is equivalent to `TryFrom<T, Error=!>`
and similarly for `FromLossy` and `TryFromLossy`. Should the former traits
therefore just be aliases of the latter?

Since `From` is already stable we cannot tweak it in backward incompatible ways,
so the first question should be whether this change can be made without
breakage.

Probably it will be better to revisit this later (but there are implications on
the next question).

**Should we allow overlapping conversion implementations?**

E.g. `FromLossy` could have a blanket implementation for all `From`
implementations, and similarly for `TryFromLossy` and `TryFrom`.
Similarly, `TryFrom` could be implemented for all `From` implementations,
and the same for `TryFromLossy` for `FromLossy` implementations — except this
would have rules to implement `TryFromLossy` for `From` implementations through
both `FromLossy` and `TryFrom`, which is a type error (possibly solvable with
specialization).

This would be useful in generic code, but isn't strictly necessary.
Unfortunately without these overlapping implementations generic code wanting to
support types over multiple conversion traits requires a local re-implementation;
worse, this is a technique used today, thus adding overlap today will cause
breakage (although again specialization may solve this).

**Should the new traits be added to the prelude?**

If the ultimate aim is to make the `as` keyword redundant, then probably the answer is yes, but I will refer you to [this post by @alexcrichton](https://github.com/rust-lang/rfcs/pull/1542#issuecomment-211592332):

> On question I'd have is whether we'd want to add these traits to the prelude? One of the reasons Into and From are so easy to use is because of their prominent placement and lack of a need to import them. I'd be of the opinion that these likely want to go into the prelude at some point, but we may not have all of the resolve changes in place to do that without unduly breaking crates. To me though one of the major motivations for this RFC is integer conversions, which I believe aren't gonna work out unless they're in the prelude, so I'd be fine adding it sooner rather than later if we can.

**Should we eventually remove the `as` keyword?**

I suspect it is too late to make such a change for Rust 2018, and do not see a
need to rush this (at the very least, conversions involving `usize` and `isize`
also need solving before removal of `as`).

Removing this keyword would no doubt be contraversial and would further distance
Rust from *convenient* languages like Python, so this is not a decision to make
lightly.

Alternatively we could simply rely on Clippy to lint against "unnecessary" uses of `as`.
