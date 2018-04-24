- Feature Name: carrying_mul
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

One can, for example, define 
Explain the proposal as if it was already included in the language and you were teaching it to another Rust programmer. That generally means:

- Introducing new named concepts.
- Explaining the feature largely in terms of examples.
- Explaining how Rust programmers should *think* about the feature, and how it should impact the way they use Rust. It should explain the impact as concretely as possible.
- If applicable, provide sample error messages, deprecation warnings, or migration guidance.
- If applicable, describe the differences between teaching this to existing Rust programmers and new Rust programmers.

For implementation-oriented RFCs (e.g. for compiler internals), this section should focus on how compiler contributors should think about the change, and give examples of its concrete impact. For policy RFCs, this section should provide an example-driven introduction to the policy, and explain its impact in concrete terms.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

`pub fn carrying_mul(self, other: Self) -> (Self, Self)`

Returns the low and high words of the product of `self` and `other`.

# Drawbacks
[drawbacks]: #drawbacks

None known

# Rationale and alternatives
[alternatives]: #alternatives

The alternative is to not define this method, which means to do a double-wide multiplication, the user must use inline asm (unstable) or do an awkward dance of shifts and multiplications.

# Prior art
[prior-art]: #prior-art

This feature is already in many assembly languages (e.g. "mul" on x86, "mulh" on RISC-V M).

# Unresolved questions
[unresolved]: #unresolved-questions

- What should we call the method?
- What should the order of return values be?
