- Feature Name: restore_div_rem
- Start Date: 2015-02-14
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Add the "div_rem" function and method back into `std::num` using a new trait.

# Motivation

It is common to want to find `x / y` and `x % y` at the same time, often in
low-level code, frequently called code where fast execution is important. For
instance:

 - Base conversion
 - Converting between two different representations of an offset into data, e.g.
   converting between a view of data as a series of chunks versus a single flat
   array, or reinterpreting an array of bytes as an array of bits.
 - Algorithms that leverage number theory, e.g. to hash/compress/encode/decode
   data, or to produce pseudorandom numbers.

The naïve solution is to use this:

```rust
let (div, rem) = (x / y, x % y);
```

However, one would like some assurance that only one division will be done,
which could be provided by `div_rem`:

```rust
let (div, rem) = div_rem(x, y);
```

On architectures where there is a single instruction to perform divmod, one
would expect that that instruction will be used (for signed integers, the sign
bit may need to be adjusted). Where there is no hardware implementation, one
might expect something like the following to be done for integers:

```rust
let div = x / y;
let rem = x - div*y;
```

# Detailed design

The following trait should be added to `std::num`, and implemented for all
integer and floating-point types.

```rust
pub trait DivRem<RHS = Self>: Div<RHS> + Rem<RHS> {
    fn div_rem(self, rhs: RHS)
               -> (<Self as Div<RHS>>::Output, <Self as Rem<RHS>>::Output);
}
```

All impls of this trait should return the tuple that would result from
performing `(self / rhs, self % rhs)`. The initial implementation for primitive
types is therefore trivial, though some optimization should be done if the
compiler does not already optimize the integer versions so that they perform
only a single division.

In addition, this trait should be added as a bound to the `Float` and `Int`
traits for primitive numeric types.

The following function should also be added to `std::num`:

```rust
pub fn div_rem<T, U>(x: T, y: U)
        -> (<T as Div<U>>::Output, <T as Rem<U>>::Output) where T: DivRem<U> {
    x.div_rem(y)
}
```

# Drawbacks

There is some opportunity cost to accepting this proposal, if it turns out that
a different trait design might have been better.

# Alternatives

## Do nothing

We could postpone dealing with `div_rem`. However, this is functionality that is
very frequently used, and it is likely that leaving it out of `std` will simply
result in a large number of crates defining their own equivalent functions.

## Wait for further iteration on the `num` crate

The original `div_rem` was removed from `std::num` when the `Integer` trait was
moved to the `num` crate, because `div_rem` was a method on that trait. Whether
or not this is the right move depends on the purpose we expect `div_rem` to
fulfill.

There is an important identity that is specific to integers:

```rust
let (div, rem) = div_rem(x, y);
assert_eq!(div*y + rem, x);
```

One can argue that this property of integer division sets integers apart from
other types, such as floats and rationals, which have exact or nearly-exact
division. From this perspective, `div_rem` is primarily of interest for integer
types, and belongs on the `Integer` trait.

This RFC instead takes the view that the "meaning" of `div_rem` should simply be
to compute `(x / y, x % y)` by the fastest means possible for any given type,
without specifying whether or how the `Div` and `Rem` implementations for a
given type are related. Under this view, `div_rem` is most closely related to
`std::ops`, and is orthogonal to the numeric traits in the `num` crate.

## Add a `div_rem` method to `Int` and `Float`

This avoids an extra trait, but at the cost of further hampering abstraction
over numbers. It may interfere with redesigns of the traits under development in
the `num` crate, and also prevents use of the `div_rem` function (the one
defined above that is not a method).

## Define `div_rem` differently for non-integer arguments

If you call `div_rem` on floating point arguments, you're more likely to want
something like `((x/y).trunc(), x % y)` or `((x/y) as i64, x%y)` than `(x / y, x
% y)`. However, it would be potentially confusing if `div_rem` did this, since
from the name you would expect the result to be a simple combination of the two
operators.

## Add a new operator

In this case, `DivRem` would be added to `std::ops` instead of `std::num`, and
instead of adding a `div_rem` function, an operator would be provided (the most
obvious candidate being `/%`).

Pros:
 - This operation seems to be used at least as frequently as `%` by itself, and
   some form of `divmod` is one of the built-in or prelude functions for many
   languages.
 - Since both symbols are used only as binops currently, it seems that the `/%`
   symbol would not result in any syntax problems or ambiguities.
 - Under this RFC, `DivRem` has more to do with the `Div` and `Rem` traits
   anyway.

Cons:
 - Yet more complexity in the language syntax, which has to be taught to parsers
   and new users of the language, and accounted for in future syntax proposals.
 - Unlike `%`, most languages do not have a symbol for `divmod`, so new users
   won't expect there to be one.
 - If one uses `x/%y`, it looks somewhat like `%` is a prefix operator. `x%/y`
   may or may not be considered better, but is reversed from the usual
   pronunciation of such operators as `div-rem` or `div-mod`.
 - We can't really allow `x /%= y`, because `/%` will produce a tuple. (Well, we
   technically could allow it, but you would need a strange impl of `DivRem` on
   tuples.)
 - In languages that provide `divmod` as a function, there does not seem to be
   an overwhelming demand to shorten it further to an operator. We should
   probably have `div_rem` in the library, and *maybe* even in the prelude one
   day, but adding a new operator is arguably overkill.

# Unresolved questions

## Exactness of floating point arithmetic

It may be the case that, on some specific architecture, there is an
implementation of `div_rem` that is faster than simply using `(x / y, x % y)`,
but which is not exactly equal to it. In that case, we may want to allow such an
implementation where the returned value differs from the expected value only due
to the limitations of floating-point precision.

## Blanket implementation

If negative `where` bounds were added, there could be a blanket implementation,
such as:

```rust
impl<T, U> DivRem<U> for T
      where T: Clone+Div<U>+Rem<U>, U: Clone {
    fn div_rem(self, rhs: U)
           -> (<Self as Div<U>>::Output, <Self as Rem<U>>::Output) {
        let div = self.clone() / rhs.clone();
        (div, self % rhs)
    }
}
```
