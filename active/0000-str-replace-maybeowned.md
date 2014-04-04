- Start Date: 2014-04-04
- RFC PR #:
- Rust Issue #:

# Summary

Strings have a `fn replace(&self, from: &str, to: &str) -> ~str` method.
The return value type could be changed to `MaybeOwned`.

# Motivation

When `from` does not appear at all in `self`, the return value equals `self`.
This is an allocation and a copy that are not strictly necessary.
With a `MaybeOwned` return value type, we could just return a slice in this case.

# Drawbacks

The API becomes more complex.
For example, multiple `.replace()` calls can not be chained anymore
without adding `.as_slice()` calls in-between.
This could be addressed by implementing string methods for `MaybeOwned`.

# Detailed design

The `MaybeOwned` type already exists in `std::str`
and is used for the return value of `std::str::from_utf8_lossy`.
It is either a slice or an owned string.

The change itself is easy enough
(see my [work-in-progress branch](https://github.com/SimonSapin/rust/compare/str-replace-maybeowned)),
but see Unresolved questions.

# Alternatives

Status quo: keep `~str` as the return value type of `replace()` for simplicity,
and accept the cost on unnecessary allocations and copies.

# Unresolved questions

The compiler uses `replace()`.
How can this change be made without breaking bootstraping?
