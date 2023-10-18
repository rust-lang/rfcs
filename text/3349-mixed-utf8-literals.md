- Feature Name: `mixed_utf8_literals`
- Start Date: 2022-11-15
- RFC PR: [rust-lang/rfcs#3349](https://github.com/rust-lang/rfcs/pull/3349)
- Tracking Issue: [rust-lang/rust#116907](https://github.com/rust-lang/rust/issues/116907)

# Summary
[summary]: #summary

Relax the restrictions on which characters and escape codes are allowed in string, char, byte string, and byte literals.

Most importantly, this means we accept the exact same characters and escape codes in `"…"` and `b"…"` literals. That is:

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

Allowing all characters and all known escape codes in both types of string literals reduces the complexity of the language.
We'd no longer have [different escape codes](https://doc.rust-lang.org/reference/tokens.html#characters-and-strings)
for different literal types. We'd only require regular string literals to be valid UTF-8.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Regular string literals (`""` and `r""`) must be valid UTF-8.
For example, valid strings are `"abc"`, `"🦀"`, `"\u{1F980}"` and `"\xf0\x9f\xa6\x80"`.
`"\xff"` is not valid, however, as that is not valid UTF-8.

Byte string literals (`b""` and `br""`) may include non-ascii characters and unicode escape codes (`\u{…}`), which will be encoded as UTF-8.

The `char` type does not store UTF-8, so while `'\u{1F980}'` is valid, trying to encode it in UTF-8 as in `'\xf0\x9f\xa6\x80'` is not accepted.
In a char literal (`''`), `\x` may only be used for values 0 through 0x7F.

Similarly, in a byte literal (`b''`), `\u` may only be used for values 0 through 0x7F, since those are the only code points that are unambiguously represented as a single byte.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The ["characters and strings" section in the Rust Reference](https://doc.rust-lang.org/reference/tokens.html#characters-and-strings)
is updated with the following table:

|                 | Example     | Characters  | Escapes                   | Validation               |
|-----------------|-------------|-------------|---------------------------|--------------------------|
| Character       | 'H'         | All Unicode | ASCII, unicode            | Valid unicode code point |
| String          | "hello"     | All Unicode | ASCII, high byte, unicode | Valid UTF-8              |
| Raw string      | r#"hello"#  | All Unicode | -                         | Valid UTF-8              |
| Byte            | b'H'        | All ASCII   | ASCII, high byte          | -                        |
| Byte string     | b"hello"    | All Unicode | ASCII, high byte, unicode | -                        |
| Raw byte string | br#"hello"# | All Unicode | -                         | -                        |

With the following definitions for the escape codes:

- ASCII: `\'`, `\"`, `\n`, `\r`, `\t`, `\\`, `\0`, `\u{0}` through `\u{7F}`, `\x00` through `\x7F`
- Unicode: `\u{80}` and beyond.
- High byte: `\x80` through `\xFF`

Compared to before, the tokenizer should start accepting:
- unicode characters in `b""` and `br""` literals (which will be encoded as UTF-8),
- all `\x` escapes in `""` literals,
- all `\u` escapes in `b""` literals (which will be encoded as UTF-8), and
- ASCII `\u` escapes in `b''` literals.

Regular string literals (`""`) are checked to be valid UTF-8 afterwards.
(Either during tokenization, or at a later point in time. See future possibilities.)

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
- Python and Javascript do it differently: `\xff` means `\u{ff}`, because their strings behave like UTF-32 or UTF-16 rather than UTF-8.
  (Also, Python's byte strings "accept" `\u` as just `'\\', 'u'`, without any warning or error.)

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Should `concat!("\xf0\x9f", "\xa6\x80")` work? (The string literals are not valid UTF-8 individually, but are valid UTF-8 after being concatenated.)

  (I don't care. I guess we should do whatever is easiest to implement.)

# Future possibilities
[future-possibilities]: #future-possibilities

- Postpone the UTF-8 validation to a later stage, such that macros can accept literals with invalid UTF-8. E.g. `cstr!("\xff")`.

  - If we do that, we could also decide to accept _all_ escape codes, even unknown ones, to allow things like `some_macro!("\a\b\c")`.
    (The tokenizer would only need to know about `\"`.)

- Update the `concat!()` macro to accept `b""` strings and also not implicitly convert integers to strings, such that `concat!(b"", $x, b"\0")` becomes usable.
  (This would need to happen over an edition.)
