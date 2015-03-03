- Feature Name: extract_math_from_float
- Start Date: 2015-03-03
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Extract mathematical functions from `Float` trait to `Math` trait.

# Motivation

Currently `Float` trait is kind of God-trait that manage everything that floats
can do. [This also cause some problems for newcomers about usage of basic mathematical
functions like `sin`][sin-problem].

# Detailed design

Move most of `Float` methods to additional trait that will be defined as follows:

```rust
trait Math + Copy + NumCast {
    fn ln(self) -> Self;
    fn log(self, base: Self) -> Self;
    fn log2(self) -> Self;
    fn log10(self) -> Self;

    fn hypot(self, other: Self) -> Self;

    fn sin(self) -> Self;
    fn cos(self) -> Self;
    fn tan(self) -> Self;
    fn sin_cos(self) -> (Self, Self);

    fn asin(self) -> Self;
    fn acos(self) -> Self;
    fn atan(self) -> Self;
    fn atan2(self, other: Self) -> Self;

    fn exp_m1(self) -> Self;
    fn ln_1p(self) -> Self;

    fn sinh(self) -> Self;
    fn cosh(self) -> Self;
    fn tanh(self) -> Self;

    fn asinh(self) -> Self;
    fn acosh(self) -> Self;
    fn atanh(self) -> Self;
}
```

Which would be implemented for `f64` and `f32` as is. This will be more idiomatic
to use `std::num::Math` when we need trigonometric function and will allow calls
like `Math::sin(3.14)` which will be more familiar to newcomers from other languages
than `Float::sin(3.14)`.

Also this will allow more generic code, i.e. we do not need to create additional
trait to add this functions, i.e. to `Complex` numbers or interval arithmetic.

It kind of reverse [RFC #0369][rfc-0369], but in my opinion it has been too radical
which can be seen as [`num` crate][num] bring most of these traits back. This makes
`num` crate kind of _must have_ for any code related to math. Also other (`IteratorExt`,
`SliceExt`, `SliceConcatExt`) examples show us that this kind of separation
of concerns is desirable.

## Examples of reintroducing RFC #0369 traits

- [`num`][num]
- [`image`](https://github.com/PistonDevelopers/image/blob/master/src/traits.rs)
- [`onezero.rs`](https://github.com/japaric/onezero.rs)
- [`rust-geom`](https://github.com/servo/rust-geom/blob/master/src/num.rs)
- [`scirust`](https://github.com/indigits/scirust/tree/master/src/number)
- [`cgmath`](https://github.com/bjz/cgmath-rs)

# Drawbacks

It seems that it add another layer of complexity to codebase, but in long term
I think that this would improve newcomers experience.

# Alternatives

- Leave this as is.
- Move all math from `libstd` to `num` crate (IMHO too radical).

# Unresolved questions

- List of methods in `Math` trait. Should it be less/more?
- Leave with one trait or split it into even more?

[sin-problem]: http://stackoverflow.com/questions/28010779/where-is-the-sine-function
[rfc-0369]: https://github.com/rust-lang/rfcs/blob/master/text/0369-num-reform.md
[num]: https://github.com/rust-lang/num
