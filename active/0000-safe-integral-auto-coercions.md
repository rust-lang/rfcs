- Start Date: 2014-09-03
- RFC PR #:
- Rust Issue #:

# Summary

Enable implicit coercion from some integral type `A` to some integral type `B` if type `B` can represent all the values type `A` can.

# Motivation

This would improve both programming convenience and code readability.

# Detailed design

All unsigned integral types should auto-coerce to a wider integral type. All signed integral types should auto-coerce to a wider signed integral type. For example, `u16` would auto-coerce to any of the following: `u32`, `u64`, `i32`, `i64`. But `i16` would auto-coerce only to `i32` and `i64`.

# Drawbacks

?

# Alternatives

?

# Unresolved questions

What about auto-coercion to `int` and `uint`?
