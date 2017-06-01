- Start Date: 2014-07-23
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Currently, numeric literals like "1" can become either signed or unsigned integers of any time, depending on how they're used. For example, in `let x = 3; let y = x + 1u32;`, `x` is deduced to have type `u32`. However, replacing `1u32` with `1f32` leads to a compile error unless `3` is replaced with `3.`. `3` is a perfectly valid floating point number, so it should be able to be a floating point number under type deduction without any explicit suffixes.

# Motivation

Having unnecessary type annotations adds noise to code and makes it harder to follow. Rust's type deduction mechanism greatly reduces the number of type annotations needed for a program, but there are still some spots where it is overly conservative.

# Detailed design

In the current compiler, any integer literal without specific type annotation is deduced to have type "generic integer", which is then narrowed as required by its interaction with parts of the program that do have their types defined, such as function parameters and variables that do have explicit types. It can be deduced to be any of `i8, i16, i32, i64, int, u8, u16, u32, u64, uint`.

This RFC propses adding `f32` and `f64` to that list. The "generic integer" will be renamed "generic number", and it will have its type determined by the rest of its interactions, as it is now. The difference is that when an operation like `+ 3.0` or `cos` is applied to it, its type will be narrowed to "generic float" instead of having the compiler give an error.

The compiler will still warn when a literal is assigned to a type that cannot exactly represent that value, the same way that it does on `let x: u8 = 256;`.

This RFC does not propose allowing generic floats to coerce into integers. Adding the decimal point is an explicit annotation that the number is not intended to be integral.

# Drawbacks

Increased complexity in the type deduction system.

People who expect unsuffixed integer literals to always be integers will be confused.

# Alternatives

Leaving it as it is.

# Unresolved questions

Should a literal still be coerced without warning if it's too big for any float to exactly equal it? For example, `2^60 + 1` cannot be exactly represented as a `f32` or `f64`, but unlike `1000u8` it does have a very close approximation.

Should a literal that is very large be coerced without warning if it does happen to have an exact float representation? For example, `2^60` does have an exact `f32` representation, but `2^60 + 1` and `2^60 - 1` do not.
