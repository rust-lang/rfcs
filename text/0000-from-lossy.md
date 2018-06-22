- Feature Name: from-lossy
- Start Date: 2018-06-22
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add `FromLossy`, `TryFromLossy` traits.

Discuss the bigger picture of conversions and the `as` keyword.

Specify that `From` implementations must not only be *safe*, but also *exact*.

# Motivation
[motivation]: #motivation

Currently many numeric conversions can only be done via the `as` keyword or third-party libraries. To [quote @scottmcm](https://internals.rust-lang.org/t/lossy-conversion-trait-as/7672/4?u=dhardy):

> I’m strongly against [a trait] that’s just “whatever as does”, since I find it an unfortunate mess of conversions right now – some coercions, some extensions, some truncations, …

This has several problems:

- `as` can perform several different types of conversion (as noted) and is therefore error-prone
- `as` can have [undefined behaviour](https://github.com/rust-lang/rust/issues/10184)
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

Type conversions can be handled by the following traits:

- `From` for infallible, exact conversions (e.g. widening, `u8` → `u16`)
- `TryFrom` for fallible, exact conversions (e.g. narrowing, `u16` → `u8`, and signed, `i16` → `u16`)
- `FromLossy` for infallible, lossy conversions (mostly concerning floating-point types, e.g. `u32` → `f32`)
- `TryFromLossy` for fallible, inexact conversions (e.g. `f32` → `u32`)
- `TruncateFrom` for truncations (e.g. `u16` → `u8` which drops high bits)

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

### `From` trait

Tweak the documentation to clarify this.

Remove several implementations on SIMD types (currently including many fallible
conversions like `f32x4` → `i8x4`; nightly only).

### `FromLossy` trait

Add `std::convert::FromLossy`:

```rust
/// A trait for conversions which are potentially inexact.
/// 
/// It is required that conversions do not fail, and required that results are
/// either approximately equivalent or are an appropriate special value.
/// For example, it is reasonable for an implementation converting `f64` to
/// `f32` to lose precision and to return positive or negative infinity for
/// out-of-range results, as well as to drop payloads from NaN values.
pub trait FromLossy {
    fn from_lossy(x: T) -> Self;
}
```

Add implementations to `f64` from all of:

- `u64, i64, u128, i128`

and to `f32` from all of:

- `f64, u32, i32, u64, i64, u128, i128`

(Note: other integer → float conversions are
already handled by `From`. There is a question below about trait overlap.)

Where conversions are not exact, they should be equivalent to a conversion
which first forces some number of low bits of the integer representation to
zero, and then does an exact conversion to floating-point.

### `TryFromLossy` trait

Add `std::convert::TryFromLossy`:

```rust
/// A trait for conversions which may fail and may be inexact.
/// 
/// Implementations should fail when the output type has no suitable
/// corresponding or general-purpose value, and return an approximately
/// equivalent value when possible.
pub trait TryFromLossy {
    type Error;
    fn try_from_lossy(x: T) -> Result<Self, Self::Error>;
}
```

Add implementations from all of:

- `f32, f64`

to all of:

- `u8, u16, u32, u64, u128`
- `i8, i16, i32, i64, i128`

(Note: `f32` → `u128` is infallible (but still lossy) *if* the input is not
negative, infinite or an NaN. So even though the output type has large enough
range, this conversion trait is still applicable.)

The implementations should fail on NaN, Inf, negative values (for unsigned
integer result types), and values whose integer approximation is not
representable. The integer approximation should be the value rounded towards
zero. E.g.:

- 1.6f32 → u32: 2
- -0.2f32 → u32: error
- -0.6f32 → i32: 0
- 100_000f32 → u16: error

(Alternatively we could allow floats in the range (-1, 0] to convert to 0, also
for unsigned integers.)

# Related problems

These problems are discussed in the search for a complete solution; however it
is not currently proposed to solve them within this RFC.

## Integer transmutations

There are several types of transmutation, discussed here separately, although
they are all transmutations.

Note that there is less insentive against usage of `as` in these cases since the
conversions do not preserve "value", although alternatives still have some use
(e.g. to clarify that a conversion is a truncation).

### Sign transmutations

The conversions done by `as` between signed and unsigned types of the same size
are simply transmutations (reinterpretations of the underlying bits), e.g.
`0x80u8 as i8 == -128`.

As these are not simple mathematical operations we could simply not provide any
alternative to `as`, and suggest usage of `mem::transmute` if necessary.

Alternatively, we could add `transmute_sign` methods to all primitive integer
types, e.g.:
```rust
impl i32 {
    ...
    fn transmute_sign(self) -> u32 { ... }
}
```

### Unsigned truncation

We could add `std::convert::TruncateFrom`:

```rust
/// A trait for conversions which are truncate values by dropping unused high
/// bits.
/// 
/// Note that this is distinct from other types of conversion since high bits
/// are explicitly ignored and results are thus not numerically equivalent to
/// input values.
pub trait TruncateFrom {
    fn truncate_from(x: T) -> Self;
}
```

Add implementations for each unsigned integer type to each smaller unsigned
integer type. (See below regarding signed types.)

Note that we *could* suggest users drop unwanted high bits (via masks or
bit-shifting) *then* use `TryFrom`, but this is a very unergonomic approach to
what is a simple and commonly used operation.

### Signed truncation

Bitwise operations on signed integers can have "unintuitive" results. For example,
```rust
fn main() {
    let x = 3i32 << 14;
    println!("{}", x);
    println!("{}", x as i16);
}
```
prints:
```
49152
-16384
```
since the 16th bit is later interpreted as a -2<sup>15</sup> in the Two's
Complement representation, the numeric value on conversion to `i16` is quite
different despite all the dropped bits being 0.

Essentially, operations like `i32` → `i16` are *shorten-and-transmute*.

Since these operations are not intuitive and not so widely useful, it may not
be necessary to implement traits over them.

Instead, we could suggest users implement signed truncations like this:
`x.transmute_sign().truncate_into::<u16>().transmute_sign()`.

## Platform-dependent types

### isize / usize

The `isize` and `usize` types have undefined size, though there appears to be
an assumption that they are at least 16 bits (existing `From` implementations)
and that they could be larger than 64 bits.
[Discussion thread on internals](https://internals.rust-lang.org/t/numeric-into-should-not-require-everyone-to-support-16-bit-and-128-bit-usize/3609/7).
[Discussion about `usize` → `u64` on users forum](https://users.rust-lang.org/t/cant-convert-usize-to-u64/6243).

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
