- Feature Name: expressive-rem
- Start Date: 2019-02-23
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add additional implementations of `std::ops::Rem` to `uX/iX` and `usize/isize` where `RHS` is `uY/iY` with `Y < X` and `type Output = uY/iY`. This is correct because `a % b < b` is always `true`. Doing this allows for code which is both safer and more expressive.

# Motivation
[motivation]: #motivation

`std::ops::Rem` is currently only implemented when `typeof(RHS) == typeof(LHS)`. There are some situations where the added implementations enable safer and simpler code.

```rust
let big_num: u128 = 12315463445234525245;
let small: u8 = 16;
// now one can write
let new: u8 = big_num % small;
// instead of
let old: u8 = (big_num % small as u128) as u8;
```
In case `small` ends up getting changed to a number which is bigger than `std::u8::MAX` the new version causes a compile time error while the old one could lead to a bug, as `big_num % small` might not fit into a `u8` anymore.

As the new implementations return a smaller type, this could also allow for more optimizations.



# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

`std::ops::Rem` is implemented for all integers where maximum value size of the dividend is greater than the maximum value of the divisor, returning an integers with the type of the divisor. Using these new implementations should be preferred to casting before performing said operation. See motivation for an example.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Add the following to `std/core::ops::Rem`.

```rust
// example implementation
macro_rules! impl_rem_small {
    ($X:ty, $Y:ty) => {
        impl Rem<$Y> for $X {
            type Output = $Y;
        
            fn rem(self, modulus: $Y) -> $Y {
                use std::mem::size_of;
                assert!(size_of::<$X> > size_of::<$Y>);
                (self % modulus as $X) as $Y;
            }
        }
    }
}

// uX, uY
impl_rem_small!(u128, u64);
impl_rem_small!(u128, u32);
impl_rem_small!(u128, u16);
impl_rem_small!(u128, u8);
impl_rem_small!(u64, u32);
impl_rem_small!(u64, u16);
impl_rem_small!(u64, u8);
impl_rem_small!(u32, u16);
impl_rem_small!(u32, u8);
impl_rem_small!(u16, u8);

// iX, iY
impl_rem_small!(i128, i64);
impl_rem_small!(i128, i32);
impl_rem_small!(i128, i16);
impl_rem_small!(i128, i8);
impl_rem_small!(i64, i32);
impl_rem_small!(i64, i16);
impl_rem_small!(i64, i8);
impl_rem_small!(i32, i16);
impl_rem_small!(i32, i8);
impl_rem_small!(i16, i8);

// usize: still uncertain about this
impl_rem_small!(u128, usize);
impl_rem_small!(u64, usize);
impl_rem_small!(u32, usize);
impl_rem_small!(u16, usize);
impl_rem_small!(u8, usize);


// isize: still uncertain about this
impl_rem_small!(i128, isize);
impl_rem_small!(i64, isize);
impl_rem_small!(i32, isize);
impl_rem_small!(i16, isize);
impl_rem_small!(i8, isize);
```

# Drawbacks
[drawbacks]: #drawbacks

- causes problems with type inference
- slightly more complexity in the standard library

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- do nothing: this feature is not that important, so doing nothing is also possible

- implement rem for all possible `X, Y` instead: this should not be needed, as the currently requested implementations cover most cases, and seem to be more prone to bugs. One might also add the currently missing implementations later on. 

# Prior art
[prior-art]: #prior-art

The `DIV` instruction of [x86-Assembly][1] uses a dividend twice the size of the remainder and the divisor.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Which functions should be implemented for `usize/isize`

# Future possibilities
[future-possibilities]: #future-possibilities

Add more implementations to `Div/Rem` for `NonZeroT` to remove other edge cases of division.

[1]: https://www.felixcloutier.com/x86/div