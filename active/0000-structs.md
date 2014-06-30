- Start Date: 2014-06-30
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Allow empty structs with braces.

# Motivation

`struct X;` is an exception that was necessary because of ambiguous code such as `if x == X { } { ... }`.
With [this PR](https://github.com/rust-lang/rust/pull/14885) the ambiguity no longer exists.

# Detailed design

Allow `struct X { }`.
Remove or keep `struct X;`.
Some people might want to keep `let x = X;`.

# Drawbacks

None that I know of.

# Alternatives

N/A

# Unresolved questions

None that I know of.
