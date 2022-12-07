- Feature Name: `cross_type_number_comparison`
- Start Date: 2022-12-07
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Implement `PartialEq` and `PartialOrd` for all combinations of `u8`, `u16`, `u32`, `u64`, `u128`, `usize`, `i8`, `i16`, `i32`, `i64`, `i128`, `isize`, `f32`, `f64`; as well as `Eq` and `Ord` for the integer combinations.

# Motivation
[motivation]: #motivation

Comparing numbers of different types, whether done deliberately or accidentally, currently results in a compilation error. This means, users either have to cascadingly change numbers to be of the same type; or, hand-roll cross-type comparisons.

This is an annoyance and even a point of confusion for newer programmers. Additionally, hand-rolled comparisons are prone to subtle mistakes regarding unsigned/signed integers or float rounding. Also, achieving optimal performance is non-trivial and even more prone to mistakes.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Makes it possible to compare numbers of different types to each other, by adding `PartialOrd`/`Ord` and `PartialEq`/`Eq` trait implementations for different number types:
- `3 < 3.5` is `true`
- `200_u8 > -1_i8` is `true`
- `10000000000000001_i64 == 10000000000000000_f64` is `false`
- ...

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

There will be macros to generate the trait implementations for float<->int comparison, int<->int comparison, and float<->float comparison. The actual implementation can be adapted from @orlp's [num-ord crate](https://github.com/orlp/num-ord).

Comparing any integer to a float NaN returns `false` (`None` in `partial_cmp()`)

In a comparison, floats are compared by the exact mathematical value they represent according to IEEE-754. For example:
- `19999999_f32 == 20000000` because `19999999` is rounded to `20000000` in 32 bit floats
- `20000000_f32 != 20000001` because both literals represent their value exactly, and the values are different
- `19999999_f64 != 20000000` because f64 has enough precision to exactly represent `19999999`

# Drawbacks
[drawbacks]: #drawbacks

- Cross-type comparisons are potentially expensive
    - Unsigned<->signed integer comparisons are barely slower than native same-type comparisons: 5.1ns vs 5.0ns on my machine [playground link](https://play.rust-lang.org/?version=nightly&mode=debug&edition=2021&gist=5a8704dfd953be0d0e750b2d916ccc7f)
    - However, float<->int comparisons remain to be benchmarked and optimized
- Cross-type comparisons may hint at a design problem in the code
    - Quite subjective, and better solved with a lint

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Restructuring the code to use the same number type everywhere

This is not always possible or desirable.

## Using a third-party library like [num-ord](https://github.com/orlp/num-ord)

Requires wrapping number types in the third-party newtype which is ergonomic and not possible in some situations.

## Continue to hand-roll cross-type comparisons when they come up

As explained above, this is easy to get wrong. Also naive implementations have suboptimal performance.

# Prior art
[prior-art]: #prior-art

@orlp's [num-ord crate](https://github.com/orlp/num-ord).

@lifthrasiir's [num-cmp crate](https://github.com/lifthrasiir/num-cmp) (abandoned).

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Is the performance difference between float<->int and float<->float comparisons acceptable?

# Future possibilities
[future-possibilities]: #future-possibilities

- A lint against cross-type comparisons, probably in `clippy::restriction`.
