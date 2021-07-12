- Feature Name: widening_mul
- Start Date: 2018-04-24
- RFC PR: 
- Rust Issue: 

# Summary
[summary]: #summary

Add an inherent method to integral types which does a double-wide multiplication.

# Motivation
[motivation]: #motivation

Double-wide multiplication is a prerequisite of arbitrary-precision multiplication. Many machine architectures specify an instruction for this operation, and it is cumbersome to otherwise define. It is also a prerequisite to completely define a `u2size` type of double the size of a machine word which, in the author's experience, is useful at times (e.g. modular arithmetic on offsets into large cyclic arrays of unusual length, smoothsort).

For an integral type of known width, other than the widest (now `u128`), one can define the operation in terms of a wider type, but `usize` has unknown width, and Rust may want to support 128-bit architectures (e.g. RV128) in future.

As the author writes, the "num-bigint" crate merely uses `u32` as its word type to avoid this difficulty, which is likely suboptimal on 64-bit architectures.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

In general, the product of an m-bit and an n-bit number has (m+n) bits. If one wishes to define arbitrary-precision arithmetic, one (usually) chooses a word size, and defines the multiple-precision operations in terms of primitive operations on this type. The `widening_mul` function allows one to do so conveniently, for example:

```rust
pub struct u2size { msw: usize, lsw: usize }

impl Mul for u2size {
    fn mul(self, other: Self) -> Self {
        let (lsw, c) = self.lsw.widening_mul(other.lsw);
        u2size { lsw, msw: c + self.msw * other.lsw
                             + self.lsw * other.msw }
    }
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

`pub fn widening_mul(self, other: Self) -> (Self, Self)`

Returns the low and high words of the product of `self` and `other`.

# Drawbacks
[drawbacks]: #drawbacks

None known

# Rationale and alternatives
[alternatives]: #alternatives

- We could not define this method, which means to do a double-wide multiplication, the user must use inline asm (unstable) or do an awkward dance of shifts and multiplications.
- We could define a `mul_high` method which merely returns the high word.

# Prior art
[prior-art]: #prior-art

This feature is already in many assembly languages (e.g. "mul" on x86, "mulh" on RISC-V M).

# Unresolved questions
[unresolved]: #unresolved-questions

- What should we call the method?
- What should the order of return values be?
