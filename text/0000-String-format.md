- Start Date: 2015-02-04
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Deprecate `std::fmt::format` (to be removed before 1.0) in favor of `String::format`.

# Motivation

It's available with libstd or just with libcollections.  Also, it's a more
descriptive name; it says what you get.

# Detailed design

What it says on the tin.

There's no visible change for users of the `format!` macro.

# Drawbacks

None! Perfect RFC!

# Alternatives

Do nothing. In [#21912](https://github.com/rust-lang/rust/pull/21912) I made
`std::fmt::format` a re-export of `collections::fmt::format`, which solved the
problem with `format!` in `no_std` crates. So this RFC is not a high priority,
but still seems like a clear improvement to the language.

# Unresolved questions

None! Perfect RFC!
