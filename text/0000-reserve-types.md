- Start Date: 2014-06-25
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Reserve more numeric types.

# Motivation

It is conceivable that Rust will gain support for types such as `f128`, `f16`,
or `u128`. In the interest of backwards compatability, extend the grammar to
reserve these.

Adding them is backwards incompatible because `type PRIMITIVE = T;` is an error.

# Detailed design

Reserve the following type names: `fN`, `uN`, `iN` for `N` a multiple of 8.

Reserve additionally `dN` for decimal floating point numbers.

Reserve additionally `mN` for SSE.

# Drawbacks

Makes the grammar larger for types which we may never use.

# Alternatives

New types could require a flag to be enabled.

C99 uses `_Bool` instead of `bool` because `_T` is reserved.
