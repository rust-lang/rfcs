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

There's no visible change for users of the `format!` macro, except that it
works without libstd (see [#21912](https://github.com/rust-lang/rust/pull/21912)).

# Drawbacks

None! Perfect RFC!

# Alternatives

`std::fmt::format` is the only bit that's not a facade of `core::fmt`. It could
be a facade of `collections::fmt` instead, in which case the `format!` macro
would use `$crate::fmt::format`.  This was acrichto's suggestion.

# Unresolved questions

None! Perfect RFC!
