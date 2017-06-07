- Feature Name: heterogeneous_comparisons
- Start Date: 2017-06-06
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Allow to compare integer values of different signedness and size.

# Motivation
[motivation]: #motivation

Right now every comparison between different integral types requires a cast. These casts not only clutter the code, but also encourage writing incorrect code.

The easiest way to compare signed and unsigned values is to cast unsigned value to signed type. It works most of the time, but it will silently ignore overflows.

Comparison between values of different integer types is always well-defined. There is only one correct result and only one way to get it. Allowing compiler to perform these comparisons will reduce both code clutter and count of hidden overflow/underflow errors users make.

# Detailed design
[design]: #detailed-design

`Ord` and `Eq` traits should be modified to allow `Rhs` type not equal to `Self`.

After that `PartialEq`, `Eq`, `PartialOrd` and `Ord` should be implemented for all pairs of signed/unsigend 8/16/32/64/(128) bit integers and `isize`/`usize` variants.

Implementation for signed-singed and unsigned-unsigned pairs should promote values to larger type first, then perform comparison.

Implementation for signed-unsigned pairs should first check if signed value less than zero. If not, then it should promote both values to signed type with the same size as larger argument type and perform comparison.

# How We Teach This
[how-we-teach-this]: #how-we-teach-this

No changes I can find or think of.

# Drawbacks
[drawbacks]: #drawbacks

* It might break some code relying on return type polymorphism. It won't be possible to infer type of the second argument from type of the first one for `Eq` and `Ord`.
* Correct signed-unsigned comparison requires one more operation than regular comparison. Proposed change hides this performance cost. If user doesn't care about correctness in his particular use case, then cast and comparison is faster.
* The rest of rust math prohibits mixing different types and requires explicit casts. Allowing heterogeneous comparisons (and only comparisons) makes rust math somewhat inconsistent.

# Alternatives
[alternatives]: #alternatives

* Keep things as is.
* Add generic helper function for cross-type comparisons.

# Unresolved questions
[unresolved]: #unresolved-questions

Is `Ord` between float and int values as bad idea as it seems?
