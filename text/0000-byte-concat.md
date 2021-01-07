- Feature Name: byte_concat
- Start Date: 2018-07-31
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add a macro `concat_bytes!()` to join byte sequences onto an `u8` array,
the same way `concat!()` currently supports for `str` literals.

# Motivation
[motivation]: #motivation

`concat!()` is convenient and useful to create compile time `str` literals
from `str`, `bool`, numeric and `char` literals in the code. This RFC adds an
equivalent capability for `[u8]` instead of `str`.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The `concat_bytes!()` macro concatenates literals into a byte string literal
(an expression of the type `&[u8; N]`). The following literal types are
supported as inputs:

- byte string literals (`b"..."`)
- byte literals (`b'b'`)
- numeric literals – must fit in `u8`, any number larger than `u8::MAX` causes
  a compile time error like the following:

  ```
  error: cannot concatenate a non-`u8` literal in a byte string literal
    --> $FILE:XX:YY
     |
  XX |     concat_bytes!(256, b"val");
     |                   ^^^ this value is larger than `255`
  ```
- numeric array literals – if any literal is outside of `u8` range, it will
  cause a compile time error:

  ```
  error: cannot concatenate a non-`u8` literal in a byte string literal
    --> $FILE:XX:YY
     |
  XX |     concat_bytes!([300, 1, 2, 256], b"val");
     |                    ^^^        ^^^ this value is larger than `255`
     |                    |
     |                    this value is larger than `255`
  ```

For example, `concat_bytes!(42, b"va", b'l', [1, 2])` evaluates to
`[42, 118, 97, 108, 1, 2]`.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

<!-- TODO -->

# Drawbacks
[drawbacks]: #drawbacks

None known.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

`concat!` could instead be changed to sometimes produce byte literals instead of
string literals, like a previous revision of this RFC proposed. This would make
it hard to ensure the right output type is produced – users would have to use
hacks like adding a dummy `b""` argument to force a byte literal output.

# Prior art
[prior-art]: #prior-art

<!-- TODO -->

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Should additional literal types be supported? Byte string literals are
  basically the same thing as byte slice references, so it might make sense to
  support those as well (support `&[0, 1, 2]` in addition to `[0, 1, 2]`).
- What to do with string and character literals? They could either be supported
  with their underlying UTF-8 representation being concatenated, or rejected.
  - If supported, it would probably make sense to also support boolean literals
    so `concat_bytes!()` supports all inputs `concat!()` does.
  - If rejected, it would probably makes sense to also reject boolean literals
    to avoid any possible confusion about their representation (`b"true"` and
    `b"false"` vs. `1` and `0`).
