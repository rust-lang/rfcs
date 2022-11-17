- Feature Name: `mixed_utf8_literals`
- Start Date: 2022-11-15
- RFC PR: [rust-lang/rfcs#3349](https://github.com/rust-lang/rfcs/pull/3349)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Allow the exact same characters and escape codes in `"…"` and `b"…"` literals.

That is:

- Allow unicode characters, including `\u{…}` escape codes, in byte string literals. E.g. `b"hello\xff我叫\u{1F980}"`
- Also allow non-ASCII `\x…` escape codes in regular string literals, as long as they are valid UTF-8. E.g. `"\xf0\x9f\xa6\x80"`

# Motivation
[motivation]: #motivation

Byte strings (`[u8]`) are a strict superset of regular (utf-8) strings (`str`),
but Rust's byte string literals are currently not a superset of regular string literals:
they reject non-ascii characters and `\u{…}` escape codes.

```
error: non-ASCII character in byte constant
 --> src/main.rs:2:16
  |
2 |     b"hello\xff你\u{597d}"
  |                ^^ byte constant must be ASCII
  |

error: unicode escape in byte string
 --> src/main.rs:2:17
  |
2 |     b"hello\xff你\u{597d}"
  |                  ^^^^^^^^ unicode escape in byte string
  |
```

This can be annoying when working with "conventionally UTF-8" strings, such as with the popular [`bstr` crate](https://docs.rs/bstr/latest/bstr/).
For example, right now, there is no convenient way to write a literal like `b"hello\xff你好"`.

Allowing all characters and escape codes in both types of string literals reduces the complexity of the language.
We'd no longer have [different escape codes](https://doc.rust-lang.org/reference/tokens.html#characters-and-strings)
for different literal types. We'd only require regular string literals to be valid UTF-8.

If we can postpone the UTF-8 validation until the point where tokens are turned into literals, then this not only simplifies the job of the tokenizer,
but allows macros to take string literals with invalid UTF-8 (through `$_:tt` or `TokenTree`).
That can be useful for macros like `cstr!("…")` and `wide!("…")`, etc., which currently unnecessarily result in errors for non-UTF-8 data:

```
error: out of range hex escape
 --> src/main.rs:3:13
  |
3 |     cstr!("¿\xff");
  |             ^^^^ must be a character in the range [\x00-\x7f]
```

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Regular string literals (`""`) must be valid UTF-8. For example, valid strings are `"abc"`, `"🦀"`, `"\u{1F980}"` and `"\xf0\x9f\xa6\x80"`.
`"\x80"` is not valid, however, as that is not valid UTF-8.

Byte string literals (`b""`) may include non-ascii characters and unicode escape codes (`\u{…}`), which will be encoded as UTF-8.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The tokenizer should accept all escape codes in both `""` and `b""` literals.
Only a regular string literal is checked for invalid UTF-8, but only at the point where the token is converted to a string literal AST node.

Just like how `$_:tt` accepts a thousand-digit integer literal but `$_:literal` does not,
a `$_:tt` should accept `"\x80"`, but `$_:literal` should not.
Similar, proc macros should be able to consume invalid UTF-8 string literals as `TokenTree`.

# Drawbacks
[drawbacks]: #drawbacks

One might unintentionally write `\xf0` instead of `\u{f0}`.
However, for regular string literals that will result in an error in nearly all cases, since that's not valid UTF-8 by itself.

# Alternatives
[alternatives]: #alternatives

- Only extend `b""` (that is, accept `b"🦀"`), but still do not accept non-ASCII `\x` in regular string literals (that is, keep rejecting `"\xf0\x9f\xa6\x80"`).

- Stabilize `concat_bytes!()` and require writing `"hello\xff你好"` as `concat_bytes!(b"hello\xff", "你好")`.
  (Assuming we extend the macro to accept a mix of byte string literals and regular string literals.)

# Prior art
[prior-art]: #prior-art

- C and C++ do the same. (Assuming UTF-8 character set.)
- [The `bstr` crate](https://docs.rs/bstr/latest/bstr/)
- Python and Javascript do it differently: `\xff` mean `\u{ff}`, because their strings behave like UTF-32 or UTF-16 rather than UTF-8.
  (Also, Python's byte strings "accept" `\u` escape codes as just `'\\', 'u'`, without any warning or error.)

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Should `concat!("\xf0\x9f", "\xa6\x80")` work? (The string literals are not valid UTF-8 individually, but are valid UTF-8 after being concatenated.)

  (I don't care. I guess we should do whatever is easiest to implement.)

- How about single byte and character literals?

  - Should `b'\u{30}` work? (It's a unicode escape code, but it's still just one byte in UTF-8.)

    I think yes. I see no reason to disallow it.

  - Should `'\xf0\x9f\xa6\x80'` work? (It's multiple escape codes, but it's still just one character in UTF-8.)

    Probably not, since a `char` is not UTF-8 encoded; it's a single UTF-32 codepoint.
    _Decoding_ UTF-8 from `\x` escape codes back into UTF-32 would be a bit surprising.

# Future possibilities
[future-possibilities]: #future-possibilities

- Update the `concat!()` macro to accept `b""` strings and also not implicitly convert integers to strings, such that `concat!(b"", $x, b"\0")` becomes usable.
  (This would need to happen over an edition.)
