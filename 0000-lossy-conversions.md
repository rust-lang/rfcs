- Feature Name: `lossy_conversions`
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary

[summary]: #summary

Add traits for lossy numeric conversions as an alternative to the `as` operator, and deprecate `as` for lossy numeric casts in a future edition, so

```rust
let n = f64::PI as usize;
```

becomes

```rust
let n: usize = f64::PI.lossy_into();
```

# Motivation

[motivation]: #motivation

The `as` operator is a footgun when used to convert between number types. For example, converting an `i64` to an `i32` may silently truncate digits. Other conversions may wrap around, saturate, or lose numerical precision.

The problem is that this is not obvious when reading the code; the `as` operator looks innocuous, so the risk of getting a wrong result is easily overlooked. This goes against Rust's design philosophy of highlighting potential problems with explicit syntax.

# Guide-level explanation

[guide-level-explanation]: #guide-level-explanation

Conversions between number types can sometimes be lossy. For example, when converting a `f64` to `f32`, the number gets less precise, and very large numbers become _Infinity_. Rust offers the following traits for converting between numbers:

- `From`/`Into`: These cover _lossless_ conversions, where the output is the exact same number as the input. For example, converting a `u8` to a `u32` is lossless, because every value of `u8` can be represented by a `u32`. You may also use the `as` operator for lossness numeric conversions.

- `TryFrom`/`TryInto`: These cover _fallible_ conversions, which return an error when the input can't be represented by the output type.

- `TruncatingFrom`/`TruncatingInto`: These traits are used for lossy integer conversions. When using these traits, leading bits that don't fit into the output type are cut off; the remaining bits are reinterpreted as the output type. This can change the value completely, even turn a negative number into a positive number or vice versa. This conversion is very fast, but should be used with care.

- `SaturatingFrom`/`SaturatingInto`: Like the `Truncating*` traits, these traits are used for lossy integer conversions. They check if the input fits into the output type, and if not, the closest possible value is used instead. For example, converting `258` to a `u8` with this strategy results in `255`, which is the highest `u8`.

- `LossyFrom`/`LossyInto`: These traits cover conversions involving floats (`f32` and `f64`). The converted value may be both rounded and saturated. When converting from a float to an integer, `NaN` is converted to `0`.

Although the `as` operator can also be used for _truncating_ and _lossy_ numeric conversions, this is discouraged and will be deprecated in the future. The `cast_lossy` lint warns against this, and will become an error in a future edition.

## Examples

```rust
42_u8 as i16 == 42  // for lossless conversions, `as` is ok

i16::from(42_u8) == 42

i8::try_from( 42_u8) == Ok(42)
i8::try_from(-24_u8).is_err()

i8::truncating_from( 42_u8)  == 42
u8::truncating_from(-24_i8)  == 232  // u8::MAX + 1 - 24
i8::truncating_from(232_u8)  == -24  // u8::MAX + 1 - 232
u8::truncating_from(280_i16) == 24   // 280 % u8::MAX = 24

u8::saturating_from( 42_i8)  == 42
u8::saturating_from(-14_i8)  == 0    // u8::MIN
i8::saturating_from(169_u8)  == 127  // i8::MAX
u8::saturating_from(280_i16) == 255  // u8::MAX

f32::lossy_from(42_i32)         == 42.0
f32::lossy_from(1073741827_i32) == 1073741800.0 // rounded
i32::lossy_from(f32::PI)        == 3            // rounded
i32::lossy_from(f32::INFINITY)  == i32::MAX     // saturated
```

## How does this impact writing code?

This way of doing conversions is more verbose than in other languages. However, it is also very flexible, since you can choose _how_ a value should be converted. And since the behavior is explicit, you can't choose a truncating conversion instead of a lossless one by accident. Method names such as `truncating_from` alert the reader to the possibility of a bug.

The `as` operator can be used instead of `truncating_from` or `lossy_from`. However, this is discouraged, and will become a warning and then an error in the future. `as` does not guard against logic bugs, and may even encourage sloppy code. That's why it should no longer be used for conversions between numbers.

# Reference-level explanation

[reference-level-explanation]: #reference-level-explanation

The `as` operator has more than one purpose. Besides numeric casts, it is also used for

- enum to discriminant casts
- casts involving raw pointers, addresses, and function items
- [type coercions](https://doc.rust-lang.org/stable/reference/type-coercions.html) (e.g. from `&mut T` to `&T`, or from `[T; N]` to `[T]`)

These are _not_ affected by this RFC; this proposal only concerns itself with casts between numbers.

To be able to deprecate `as` for lossy numeric casts, any numeric conversion must be achievable by other means. The most promising solution for this is to use traits with the same design as `From`/`Into`.

To make potential errors explicit, we can distinguish between these numeric errors:

1. **Truncation**: Digits from the beginning of the number are cut off
2. **Wrapping**: The bits of a signed integer are reinterpreted as an unsigned integer, or vice versa
3. **Saturation**: If the number is too high or too low, the closest possible number is selected
4. **Precision loss**: The number is rounded, resulting in fewer significant digits

Truncation and Wrapping often occur together; for example, an `i32 → u16` conversion can both truncate and wrap around. To keep the complexity to a minimum, we treat wrapping as a special case of truncation, so we arrive at the following 6 new traits:

- `Truncating{From,Into}` — truncating conversions between integers
- `Saturating{From,Into}` — saturating conversions between integers
- `Lossy{From,Into}` — lossy conversions that involve floats

Note that the word "lossy" means any conversion that doesn't preserve the input value (including truncation and saturation), but the `Lossy*` traits have a narrower scope.

```rust
pub trait TruncatingFrom<T> {
    fn truncating_from(value: T) -> Self;
}

pub trait SaturatingFrom<T> {
    fn saturating_from(value: T) -> Self;
}

pub trait LossyFrom<T> {
    fn lossy_from(value: T) -> Self;
}
```

`TruncatingFrom` and `LossyFrom` can be implemented in the standard library using `as` by silencing the lint. For example:

```rust
#![allow(cast_lossy)]

impl TruncatingFrom<i16> for i8 {
    fn truncating_from(value: i16) -> i8 {
        value as i8
    }
}

impl LossyFrom<f64> for f32 {
    fn lossy_from(value: f64) -> f32 {
        value as f32
    }
}
```

`SaturatingFrom` must be implemented manually, but is straightforward:

```rust
#![allow(cast_lossy)]

impl SaturatingFrom<i16> for i8 {
    fn saturating_from(value: u8) -> i8 {
        if value < i8::MIN as i16 {
            i8::MIN
        } else if value > i8::MAX as i16 {
            i8::MAX
        } else {
            value as i8
        }
    }
}
```

The `*Into` traits are implemented with blanket implementations:

```rust
impl<T, U> TruncatingInto<U> for T
where
    U: TruncatingFrom<T>,
{
    fn truncating_into(self) -> U {
        U::truncating_from(self)
    }
}

impl<T, U> SaturatingInto<U> for T
where
    U: SaturatingFrom<T>,
{
    fn saturating_into(self) -> U {
        U::saturating_from(self)
    }
}

impl<T, U> LossyInto<U> for T
where
    U: LossyFrom<T>,
{
    fn lossy_into(self) -> U {
        U::lossy_from(self)
    }
}
```

The traits will be added to the standard library prelude in a future edition.

This list of conversions should be implemented:

- `Truncating*` and `Saturating*`:
  - all **signed** to **unsigned** integers
  - all **signed** to **smaller signed** integers (e.g. `i16 → i8`)
  - all **unsigned** to **smaller or equal-sized** integers (e.g. `u32 → u16` or `u32 → i32`)
  - specifically, for `isize` and `usize` (we assume they have 16 to 128 bits):
    - `u32`, `u64`, `u128`, `i8`, `i16`, `i32`, `i64`, `i128` into `usize`
    - `u16`, `u32`, `u64`, `u128`, `i32`, `i64`, `i128` into `isize`
    - `isize` into `usize`, `u8`, `u16`, `u32`, `u64`, `u128`, `i8`, `i16`, `i32`, `i64`
    - `usize` into `isize`, `u8`, `u16`, `u32`, `u64`, `i8`, `i16`, `i32`, `i64`, `i128`
- `Lossy*`:
  - `f64` into `f32`
  - `f64`/`f32` into any integer
  - any integer with more than 32 bits (including `isize`/`usize`) into `f64`
  - any integer with more than 16 bits (including `isize`/`usize`) into `f32`

## The lint

A `cast_lossy` lint is added to rustc that lints against using the `as` operator for lossy conversions.

This lint is allow-by-default, and can be enabled with `#[warn(cast_lossy)]`. The lint is later enabled as a warning, either after a certain time has passed, or at an edition boundary. Eventually, it will become an error at an edition boundary.

# Drawbacks

[drawbacks]: #drawbacks

1. The API surface of this change is pretty big: It has 3 new traits and over 200 impls. These make the documentation for primitive types less clear.

2. The traits make the language more complex, as there is one more thing to learn.

3. When the traits are added to the default prelude, more things are implicitly in scope.

4. This may change the overall character of the language. However, I believe it would make the language feel more consistent, since Rust already leans towards explicitness in most other situations.

5. This may negatively impact compile times _(to be verified)_.

# Rationale and alternatives

[rationale-and-alternatives]: #rationale-and-alternatives

1. The `Saturating*` traits aren't needed to deprecate `as` for lossy numeric conversions, so we could add only the `Truncating*` and `Lossy*` traits.

   However, the standard library already contains saturating math operations, so adding saturating conversions makes sense.

2. We could require to import the traits explicitly, instead of putting them in the standard library prelude.

   However, I believe that not having the traits in scope by default would make the feature much less ergonomic. `TryFrom`/`TryInto` were added to the standard library prelude for the same reason.

   **NOTE**: [Future possibilites: Inherent methods][inherent-methods] describes a solution that doesn't require changing the prelude.

3. Instead of deprecating `as` only for lossy numeric casts, it could be deprecated for all numeric casts, so `From`/`Into` is required in these situations.

   This feels like overkill. If people really want to forbid `as` for lossless conversions, they can use clippy's `cast_lossless` lint.

4. Instead of adding traits, the conversions could be added as inherent methods.

   However, then the output type must be part of the name, so there would be `i32::saturating_into_i16()`, `i32::saturating_into_i8()`, and so on. I prefer the comparatively shorter `i32::saturating_into()`.

5. The `Lossy*` traits could have a more descriptive name, since the term "lossy" seems to include truncation and saturation. The only name I could find that kind of describes the behavior of `LossyFrom` is `Approximate`

6. The traits could be implemented in an external crate, but then the traits couldn't be added to the standard library prelude. Furthermore, to deprecate `as` for numeric conversions, the APIs to replace it should be available in the standard library, so they can be recommended in compiler warnings/errors.

7. Of course we could do nothing about this. Rust's increasing popularity means that this change would impact millions of developers, so we should be sure that the benefits justify the churn. This feature isn't _required_; Rust has worked well until now without it, and Rustaceans have learned to be extra careful when using `as` for numeric conversions.

   However, I am convinced that removing this papercut will make Rust safer and prevent more bugs.

# Prior art

[prior-art]: #prior-art

This proposal was previously discussed in [this internals thread](https://internals.rust-lang.org/t/lets-deprecate-as-for-lossy-numeric-casts/16283).

For the proposed lint, there exists prior art in clippy:

- `cast_possible_truncation`
- `cast_possible_wrap`
- `cast_precision_loss`
- `cast_sign_loss`

These lints show that lossy numeric casts can pose enough of a problem to forbid them, even though there is currently no alternative. Another data point is , which received

# Unresolved questions

[unresolved-questions]: #unresolved-questions

- Are there better trait and method names?
- Does this impact compile times?
- Should the traits remain perma-unstable, so they can be used, but not implemented outside of the standard library?

# Future possibilities

[future-possibilities]: #future-possibilities

## Inherent methods

[inherent-methods]: #inherent-methods

Inherent methods similar to [`str::parse`](https://doc.rust-lang.org/std/primitive.str.html#method.parse) can be added to make usage more ergonomic, e.g.

```rust
impl i32 {
    pub fn truncate<T: TruncatingFrom<i32>>(self) -> T {
        T::truncating_from(self)
    }

    pub fn saturate<T: SaturatingFrom<i32>>(self) -> T {
        T::saturating_from(self)
    }

    pub fn approx<T: LossyFrom<i32>>(self) -> T {
        T::lossy_from(self)
    }
}
```

Usage:

```rust
value.truncate::<u8>()
// instead of
u8::truncating_from(value)
```

Benefits are:

- it is shorter
- unlike `value.truncating_into()` it allows specifying the output type
- unlike `T::truncating_from(value)`, it is chainable
- it doesn't require an import, so the proposed traits don't need to be added to the standard library prelude

## NonZero types

Conversions could also be implemented for `NonZero{U,I}{8,16,32,64,128}`.

## Pattern types

If [pattern types](https://github.com/rust-lang/rust/pull/107606) (e.g. `u32 is 1..`) are added, the compiler can often verify when an `as` cast is lossless:

```rust
let x: u32 is 0..=1000 = 42;
let y = x as i32; // no warning; the cast is lossless
```
