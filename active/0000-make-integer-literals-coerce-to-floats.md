- Start Date: 2014-07-23
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Currently, numeric literals like "1" can become either signed or unsigned integers of any time, depending on how they're used. For example, in `let x = 3; let y = x + 1u32;`, `x` is deduced to have type `u32`. However, replacing `1u32` with `1f32` leads to a compile error unless `3` is replaced with `3.`. `3` is a perfectly valid floating point number, so it should be able to be a floating point number under type deduction without any explicit suffixes.

# Motivation

Having unnecessary type annotations adds noise to code and makes it harder to follow. Rust's type deduction mechanism greatly reduces the number of type annotations needed for a program, but there are still some spots where it misses.

# Detailed design

This is the bulk of the RFC. Explain the design in enough detail for somebody familiar
with the language to understand, and for somebody familiar with the compiler to implement.
This should get into specifics and corner-cases, and include examples of how the feature is used.

# Drawbacks

Increased complexity in the type deduction system.

People who expect unsuffixed integer literals to always be integers will be confused.

# Alternatives

Leaving it as it is.

# Unresolved questions

Should a literal still be coerced if it's too big for any float to exactly equal it?

Should a literal that is very large be coerced if it does happen to have an exact float representation? 
