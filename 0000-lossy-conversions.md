- Feature Name: `lossy_conversions`
- Start Date: 2023-04-14
- RFC PR: [rust-lang/rfcs#3415](https://github.com/rust-lang/rfcs/pull/3415)
- Rust Issue: TBD <!-- [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000) -->

# Summary

[summary]: #summary

Add traits for lossy numeric conversions as an alternative to the `as` operator, and deprecate `as` for lossy numeric casts in a future edition, so

```rust
let n = f64::PI as usize;
```

becomes

```rust
let n: usize = f64::PI.approx();
// or
let n = f64::PI.approx::<usize>();
```

# Motivation

[motivation]: #motivation

The `as` operator is a footgun when used to convert between number types. For example, converting an `i64` to an `i32` may silently truncate digits. Other conversions may wrap around, saturate, or lose numerical precision.

The problem is that this is not obvious when reading the code; the `as` operator looks innocuous, so the risk of getting a wrong result is easily overlooked. This goes against Rust's design philosophy of highlighting potential problems with explicit syntax. This is similar to the `unsafe` keyword, which makes unsafe Rust more verbose, but also highlights code that could cause Undefined Behaviour. `as` can not introduce UB, but it can be a logic error. Rust also tries to prevent logic errors, e.g. by requiring that a `match` covers all possible values, and errors aren't silently ignored.

# Guide-level explanation

[guide-level-explanation]: #guide-level-explanation

Conversions between number types can sometimes be lossy. For example, when converting a `f64` to `f32`, the number gets less precise, and very large numbers become _Infinity_. Rust offers the following methods for converting between numbers:

- `into()`: This covers _lossless_ conversions, where the output is the exact same number as the input. For example, converting a `u8` to a `u32` is lossless, because every value of `u8` can be represented by a `u32`. You may also use the `as` operator for lossness numeric conversions.

- `try_into()`: This covers _fallible_ conversions, which return an error when the input can't be represented by the output type.

- `truncate()`: Used for **lossy integer conversions**. This truncates leading bits that don't fit into the output type; the remaining bits are reinterpreted as the output type. This can change the value completely, even turn a negative number into a positive number or vice versa. This conversion is very fast, but should be used with care.

  In mathematical terms, this returns the only result for <code>_value_ (mod 2<sup>_n_</sup>)</code> that lies in the output type's range, where _n_ is the output type's number of bits.

- `saturate()`: Like `truncate()`, this is used for **lossy integer conversions**. It checks if the input fits into the output type, and if not, the closest possible value is used instead. For example, converting `258` to a `u8` with this method results in `255`, which is the highest `u8`.

- `approx()`: This must be used when the input or output type is a **float** (`f32` and `f64`). The value may be both rounded and saturated. When converting from a float to an integer, `NaN` is turned into `0`.

Although the `as` operator can also be used instead of `truncate()` or `approx()`, this is discouraged and will be deprecated in the future. The `cast_lossy` lint warns against this, and will become an error in a future edition.

## Conversion traits

`into()` and `try_into()` are trait methods from the `Into`/`TryInto` traits, and exist on many types besides numbers. On the other hand, `truncate()`, `saturate()`, and `approx()` are inherent methods that only exist on numeric types in the standard library. Their traits are unstable for now, so they ca not be implemented for custom types.

## Examples

```rust
42_u8 as i16 == 42  // for lossless conversions, `as` is ok

i16::from(42_u8) == 42

i8::try_from( 42_u8) == Ok(42)
i8::try_from(-24_u8).is_err()

 42_u8.truncate::<i8>()  == 42
-24_i8.truncate::<u8>()  == 232  // 2⁸ - 24
232_u8.truncate::<i8>()  == -24  // 232 - 2⁸
536_i16.truncate::<u8>() == 24   // 536 mod 2⁸

 42_i8.saturate::<u8>()  == 42
-14_i8.saturate::<u8>()  == 0    // u8::MIN
169_u8.saturate::<i8>()  == 127  // i8::MAX
280_i16.saturate::<u8>() == 255  // u8::MAX

        42_i32.approx::<f32>() == 42.0
1073741827_i32.approx::<f32>() == 1073741800.0 // rounded
       f32::PI.approx::<i32>() == 3            // rounded
 f32::INFINITY.approx::<i32>() == i32::MAX     // saturated
```

## How does this impact writing code?

This way of doing conversions is more verbose than in other languages. However, it is also very flexible, since you can choose _how_ a value should be converted. And since the behavior is explicit, you can't choose a truncating conversion instead of a lossless one by accident. Method names such as `truncate` alert the reader to the possibility of a bug.

The `as` operator can be used instead of `truncate` or `approx`. However, this is discouraged, and will become a warning and then an error in the future. `as` does not guard against logic bugs, and may even encourage sloppy code. That's why it should no longer be used for conversions between numbers.

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

Truncation and Wrapping often occur together; for example, an `i32 → u16` conversion can both truncate and wrap around. To keep the complexity to a minimum, we treat wrapping as a special case of truncation, so we arrive at the following 3 new traits:

- `TruncatingFrom<T>` — truncating conversions between integers
- `SaturatingFrom<T>` — saturating conversions between integers
- `ApproxFrom<T>` — lossy conversions that involve floats

```rust
pub trait TruncatingFrom<T> {
    fn truncating_from(value: T) -> Self;
}

pub trait SaturatingFrom<T> {
    fn saturating_from(value: T) -> Self;
}

pub trait ApproxFrom<T> {
    fn approx_from(value: T) -> Self;
}
```

`TruncatingFrom` and `ApproxFrom` can be implemented in the standard library using `as` by silencing the lint. For example:

```rust
#![allow(cast_lossy)]

impl TruncatingFrom<i16> for i8 {
    fn truncating_from(value: i16) -> i8 {
        value as i8
    }
}

impl ApproxFrom<f64> for f32 {
    fn approx_from(value: f64) -> f32 {
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

These traits are **unstable** for now. Before stabilizing them, we should consider adding `*Into` traits as well, but that discussion is left for the future.

## Inherent methods

[inherent-methods]: #inherent-methods

Inherent methods similar to [`str::parse`](https://doc.rust-lang.org/std/primitive.str.html#method.parse) are added to make usage more ergonomic, e.g.

```rust
impl i32 {
    pub fn truncate<T: TruncatingFrom<i32>>(self) -> T {
        T::truncating_from(self)
    }

    pub fn saturate<T: SaturatingFrom<i32>>(self) -> T {
        T::saturating_from(self)
    }

    pub fn approx<T: ApproxFrom<i32>>(self) -> T {
        T::approx_from(self)
    }
}
```

This has several benefits. Unlike `value.truncating_into()` it allows specifying the output type, and unlike `T::truncating_from(value)`, it is chainable. Furthermore, inherent methods are always in scope and don't require importing a trait.

## List of conversions

This list of conversions should be implemented:

- `TruncatingFrom` and `SaturatingFrom`:
  - all **signed** to **unsigned** integers
  - all **signed** to **smaller signed** integers (e.g. `i16 → i8`)
  - all **unsigned** to **smaller or equal-sized** integers (e.g. `u32 → u16` or `u32 → i32`)
  - specifically, for `isize` and `usize` (we assume they have 16 to 128 bits):
    - `u32`, `u64`, `u128`, `i8`, `i16`, `i32`, `i64`, `i128` into `usize`
    - `u16`, `u32`, `u64`, `u128`, `i32`, `i64`, `i128` into `isize`
    - `isize` into `usize`, `u8`, `u16`, `u32`, `u64`, `u128`, `i8`, `i16`, `i32`, `i64`
    - `usize` into `isize`, `u8`, `u16`, `u32`, `u64`, `i8`, `i16`, `i32`, `i64`, `i128`
- `ApproxFrom`:
  - `f64` into `f32`
  - `f64`/`f32` into any integer
  - any integer with more than 32 bits (including `isize`/`usize`) into `f64`
  - any integer with more than 16 bits (including `isize`/`usize`) into `f32`

## The lint

A `cast_lossy` lint is added to rustc that lints against using the `as` operator for lossy conversions.

This lint is allow-by-default, and can be enabled with `#[warn(cast_lossy)]`. The lint is later enabled as a warning, either after a certain time has passed, or at an edition boundary. Eventually, it will become an error at an edition boundary.

# Drawbacks

[drawbacks]: #drawbacks

1. This makes code more verbose.

   I do not think that this is a deal-breaker. Rustaceans have come to accept that you need `.unwrap()` to access an optional value, and `Box::new()` to allocate heap memory: things that many popular languages do automatically. But since Rust had a more concise way of converting integers, and may now abandon it, people might be unhappy because they will have to change their coding habits. Furthermore, the new way isn't _obviously_ better than the old one in every way. Probably only those who have had to deal with integer truncation bugs will fully appreciate this change.

2. The API surface of this change is rather big: Most numeric types get 3 new methods.

3. This may change the overall character of the language.

   However, I believe it would make the language feel more consistent, since Rust already leans towards explicitness in most other situations.

4. This may negatively impact compile times _(to be verified)_.

# Rationale and alternatives

[rationale-and-alternatives]: #rationale-and-alternatives

First, I'd like to compare this with integer arithmetic. An expression like `x + 200_u8` can overflow, which is implicit, similarly to integer casts. However, there are some crucial differences:

- Arithmetic is more common than conversions, so there is a bigger need to be concise
- For arithmetic, there already exist checked, truncating and wrapping methods. For example, you can write `200_u8.saturating_add(x)` if you want; an equivalent for conversions does not exist.
- By default, overflow on arithmetic operations wraps around in release builds and panics in debug builds. This means that bugs due to integer overflow can be caught with tests. Integer conversions on the other hand are _always_ unchecked. This means that bugs due to truncation are easier to miss.

Therefore, I believe that making lossy conversions more explicit would go a long way toward avoiding bugs, making you more confident that your code is correct, and saving you time debugging and writing tests. But is the proposal outlined above the best possible solution? Let's look at a few alternatives:

1. We could add the conversion methods and the `cast_lossy` lint, but never turn it into an error. It would remain a warning, which people are free to ignore or disable. This makes sense if avoiding the `as` operator is seen more as a stylistic preference than a correctness issue.

2. `saturate()` isn't needed to deprecate `as` for lossy numeric conversions, so we could add only `truncate()` and `approx()`.

   However, the standard library already contains saturating math operations, so adding saturating conversions makes sense.

3. Instead of deprecating `as` only for lossy numeric casts, it could be deprecated for all numeric casts, so `From`/`Into` is required in these situations.

   This feels like overkill. If people really want to forbid `as` for lossless conversions, they can use clippy's `cast_lossless` lint.

4. The `approx()` method could have a more descriptive name. Likewise, `truncate()` isn't ideal since it sometimes wraps around. I am open to better suggestions (though bear in mind that having multiple methods, like `truncate()`, `wrap()` and `truncate_and_wrap()` will make the feature more complicated and harder to learn).

5. `truncate` and `saturate` could be abbreviated as `trunc` and `sat`. This would make it more concise, which is appealing to people who convert between numbers a lot.

6. This could be implemented in an external crate as extension traits, but then the traits must be imported everywhere they are used. Furthermore, to deprecate `as` for numeric conversions, the APIs to replace it should be available in the standard library, so they can be recommended in compiler warnings/errors.

7. Another option is to deprecate `as` only for lossy integer-to-integer casts. From what I understand, conversions involving floats are more common, and the implied rounding behaviour is usually desired. Having to spell `.approx()` instead of ` as _` is not a huge deal, but the ecosystem migration may be considered too much of a hassle.

8. Of course we could do nothing about this. Rust's increasing popularity means that this change would impact millions of developers, so we should be sure that the benefits justify the churn. This feature isn't _required_; Rust has worked well until now without it, and Rustaceans have learned to be extra careful when using `as` for numeric conversions.

   However, I am convinced that removing (or at least reducing) this papercut will make Rust safer and prevent more bugs. This is similar in spirit to the `unsafe` keyword, which makes Rust more verbose, but also more explicit about potential problems.

# Prior art

[prior-art]: #prior-art

This proposal was previously discussed in [this internals thread](https://internals.rust-lang.org/t/lets-deprecate-as-for-lossy-numeric-casts/16283).

I'm not aware of a language with explicit integer or float casting methods that distinguish between different numerical errors.

For the proposed lint, there exists prior art in clippy:

- `cast_possible_truncation`
- `cast_possible_wrap`
- `cast_precision_loss`
- `cast_sign_loss`

These lints show that lossy numeric casts can pose enough of a problem to forbid them, even though there is currently no alternative in the cases where truncation/saturation/rounding is desired.

API-wise, the most similar features are the [`FromIterator`](https://doc.rust-lang.org/std/iter/trait.FromIterator.html)/[`IntoIterator`](https://doc.rust-lang.org/std/iter/trait.IntoIterator.html) traits used by [`collect()`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.collect), and the [`FromStr`](https://doc.rust-lang.org/std/str/trait.FromStr.html) trait used by [`parse()`](https://doc.rust-lang.org/std/primitive.str.html#method.parse).

# Unresolved questions

[unresolved-questions]: #unresolved-questions

- Are there better method names?

- Does this impact compile times?

- Should the traits be private? Or if not, should they remain perma-unstable, so they can not implemented outside the standard library?

# Future possibilities

[future-possibilities]: #future-possibilities

## NonZero types

Conversions could also be implemented for `NonZero{U,I}{8,16,32,64,128}`.

## Pattern types

If [pattern types](https://github.com/rust-lang/rust/pull/107606) (e.g. `u32 is 1..`) are added, the compiler can often verify when an `as` cast is lossless:

```rust
let x: u32 is 0..=1000 = 42;
let y = x as i32; // no warning; the cast is lossless
```
