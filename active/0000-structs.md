- Start Date: 2014-06-30
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Allow empty structs with braces.

# Motivation

`struct X;` is an exception that was necessary because of ambiguous code such as `if x == X { } { ... }`.
With [this PR](https://github.com/rust-lang/rust/pull/14885) the ambiguity no longer exists.

## Definitive list of reasons to do this.

- 64% (or so) of those who voted want this (+1 vs. -1).
- Macros without special cases for zero elements.
- Ease the transition between empty and non-empty structs: `struct X { _sigh: () }`.
- Consistency with C code. People find this weird when learning Rust.
- Consistency with Rust code: `trait X { }`, `enum X { }`, `fn x() { }`, `mod X { }`.
- Clarity: `let x = X { };` is a struct.

# Detailed design

Replace `;` by `{ }` everywhere.

# Drawbacks

None.

# Alternatives

N/A

# Unresolved questions

TIOOWTDI (with a majority in favor)
