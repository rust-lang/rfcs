- Feature Name: unified_machine_word_arithmetic
- Start Date: 2016-07-13
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

Unify functionality peculiar to `i8`…`i64` and `u8`…`u64` in a trait containing the
family of `overflowing`/`checked`/`wrapping`/`saturating` variants of arithmetic operations,
as well as a few new ones.

# Motivation
[motivation]: #motivation

The current design has the following functions replicated over eight different types:

- `checked_add`
- `checked_div`
- `checked_mul`
- `checked_neg`
- `checked_rem`
- `checked_shl`
- `checked_shr`
- `checked_sub`
- `count_ones`
- `count_zeros`
- `leading_zeros`
- `overflowing_add`
- `overflowing_div`
- `overflowing_mul`
- `overflowing_neg`
- `overflowing_rem`
- `overflowing_shl`
- `overflowing_shr`
- `overflowing_sub`
- `rotate_left`
- `rotate_right`
- `saturating_add`
- `saturating_mul`
- `saturating_sub`
- `trailing_zeros`
- `wrapping_add`
- `wrapping_div`
- `wrapping_mul`
- `wrapping_neg`
- `wrapping_rem`
- `wrapping_shl`
- `wrapping_shr`
- `wrapping_sub`

Further functions that are replicated include:

- `from_be`
- `from_le`
- `swap_bytes`
- `to_be`
- `to_le`

But some of those should get their own trait dealing with endianness, which is not the
focus of this RFC.

Generalizing the arithmetic functionality into a trait, will have the following benefits:

- Enable generic code working with arithmetic overflow — a clunky process at present.
- Pave the way for general-case software implementations of large integer sizes on platforms
    without support.
- Make it easy to implement `i128`/`u128` (and larger types) when compiler/hardware support permits, letting
    generic code predating the addition of the larger types work with it.
- Generic implementation of `std::ops::*` traits with the usual panic-on-overflow semantics, using the
    `checked_*` functions.

Philosophically, one might argue that a solid trait for machine-arithmetic might pave the way
for saner arithmetic hierarcies: machine-level arithmetic is seldom easily expressible in terms of
abstract algebra; indeed this RFC may be said to enforce a separation of concerns in library design.

# Detailed design
[design]: #detailed-design

I propose the following trait to be added to the standard library where convenient:

    trait MachineArith : Copy + Eq {
      fn abs(self) -> Self;
      fn bitsize() -> u32; // new
      fn bit_test(self, u32) -> bool; // new
      fn bit(u32) -> Self; // new
      fn bitwise_and(self, Self) -> Self; // new
      fn bitwise_not(self) -> Self; // new
      fn bitwise_or(self, Self) -> Self; // new
      fn bitwise_xor(self, Self) -> Self; // new
      fn bitwise_zeros() -> Self; // new
      fn bitwise_ones() -> Self; // new
      fn checked_add(self, Self) -> Option<Self>;
      fn checked_div(self, Self) -> Option<Self>;
      fn checked_mul(self, Self) -> Option<Self>;
      fn checked_neg(self) -> Option<Self>;
      fn checked_pow(self, u32) -> Option<Self>; // new
      fn checked_rem(self, Self) -> Option<Self>;
      fn checked_shl(self, u32) -> Option<Self>;
      fn checked_shr(self, u32) -> Option<Self>;
      fn checked_sub(self, Self) -> Option<Self>;
      fn count_ones(self) -> u32;
      fn count_zeros(self) -> u32;
      fn is_negative(self) -> bool;
      fn is_positive(self) -> bool;
      fn leading_zeros(self) -> u32;
      fn leading_zeros_at_least(self, u32) -> bool; // new
      fn max_value() -> Self;
      fn min_value() -> Self;
      fn overflowing_add(self, Self) -> (Self, bool);
      fn overflowing_div(self, Self) -> (Self, bool);
      fn overflowing_mul(self, Self) -> (Self, bool);
      fn overflowing_neg(self) -> (Self, bool);
      fn overflowing_pow(self, u32) -> (Self, bool); // new
      fn overflowing_rem(self, Self) -> (Self, bool);
      fn overflowing_shl(self, u32) -> (Self, bool);
      fn overflowing_shr(self, u32) -> (Self, bool);
      fn overflowing_sub(self, Self) -> (Self, bool);
      fn pow(self, u32) -> Self; // alias for wrapping_pow
      fn rotate_left(self, u32) -> Self;
      fn rotate_right(self, u32) -> Self;
      fn saturating_add(self, Self) -> Self;
      fn saturating_mul(self, Self) -> Self;
      fn saturating_pow(self, u32) -> Self; // new
      fn saturating_sub(self, Self) -> Self;
      fn signum(self) -> i8;
      fn trailing_zeros(self) -> u32;
      fn trailing_zeros_at_least(self, u32) -> bool; // new
      fn wrapping_add(self, Self) -> Self;
      fn wrapping_div(self, Self) -> Self;
      fn wrapping_mul(self, Self) -> Self;
      fn wrapping_neg(self) -> Self;
      fn wrapping_pow(self, u32) -> Self; // new
      fn wrapping_rem(self, Self) -> Self;
      fn wrapping_shl(self, u32) -> Self;
      fn wrapping_shr(self, u32) -> Self;
      fn wrapping_sub(self, Self) -> Self;
    }

For the most part, these functions mirror the expected and well-known semantics
of the existing `impl`s for the machine integer types; I have taken the liberty of
adding a number of functions to cover otherwise missing functionality:

- The `bitsize` function is a constant, usable in generic programming. An important invariant is
    that
        
        let a : A;
        assert_eq!(a.count_ones() + a.count_zeros(), A::bitsize());

- The `bit` and `bit_test` functions should exist to simplify working with bit masks, as per the
    Haskell module `Data.Bits` .

- The `bitwise` function family exists to allow more rigorously machine-level bitwise arithmetic.
    Rust has its syntactical ancestry from C, but the mere existence of `std::ops` devalues the
    utility in having bitwise operations exist only as infix operators.


- The `pow` function should be expanded to cover a `wrapping`, `overflowing`, `checked` and
    `saturating` variety.

- The `leading_zeros_at_least`/`trailing_zeros_at_least` represents a subtler functionality
    that is present in the form of processor flags set by shifting instructions on some architectures:
    should set bits be shifted out of register by a shift operation, a flag will be set. This
    essentially provides a second ‘overflow’ functionality to the shifting functions; the ability
    to check if the shift is equivalent to ideal multiplication by a power of two.

As for default implementations, I suggest using `overflowing_*` as the basis for the other functions,
and having `checked_div` and `checked_rem` also do a zero-check of the divisor, using `bitwise_zeros`.

# Drawbacks
[drawbacks]: #drawbacks

There are a number of drawbacks to the above proposition.

One of the least pressing ones is that the introduction of this trait clobbers other concepts
of functionality:

- A separate trait for bit set-like structures that work similar to machine words may be desirable.
- A separate trait for types with a negative/positive may be desirable.
- A separate trait for the `pow` function, similar to `std::ops::*`.

# Alternatives
[alternatives]: #alternatives

The current implementation is perfectly usable, if a little more unweildy for generics.

# Unresolved questions
[unresolved]: #unresolved-questions

The proposed trait is a grab-bag of functionality associated with the way processors and bit-twiddling
languages handle machine words. The principal question is if it is wise to have a monolithic trait like that.
(See Drawbacks.)
