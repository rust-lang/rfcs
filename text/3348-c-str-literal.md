- Feature Name: `c_str_literal`
- Start Date: 2022-11-15
- RFC PR: [rust-lang/rfcs#3348](https://github.com/rust-lang/rfcs/pull/3348)
- Rust Issue: [rust-lang/rust#105723](https://github.com/rust-lang/rust/issues/105723)

# Summary
[summary]: #summary

`c"…"` string literals.

# Motivation
[motivation]: #motivation

Looking at the [amount of `cstr!()` invocations just on GitHub](https://cs.github.com/?scopeName=All+repos&scope=&q=cstr%21+lang%3Arust) (about 3.2k files with matches) it seems like C string literals
are a widely used feature. Implementing `cstr!()` as a `macro_rules` or `proc_macro` requires non-trivial code to get it completely right (e.g. refusing embedded nul bytes),
and is still less flexible than it should be (e.g. in terms of accepted escape codes).

In Rust 2021, we reserved prefixes for (string) literals, so let's make use of that.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

`c"abc"` is a [`&CStr`](https://doc.rust-lang.org/stable/core/ffi/struct.CStr.html). A nul byte (`b'\0'`) is appended to it in memory and the result is a `&CStr`.

All escape codes and characters accepted by `""` and `b""` literals are accepted, except nul bytes.
So, both UTF-8 and non-UTF-8 data can co-exist in a C string. E.g. `c"hello\x80我叫\u{1F980}"`.

The raw string literal variant is prefixed with `cr`. For example, `cr"\"` and `cr##"Hello "world"!"##`. (Just like `r""` and `br""`.)

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Two new [string literal types](https://doc.rust-lang.org/reference/tokens.html#characters-and-strings): `c"…"` and `cr#"…"#`.

Accepted escape codes: [Quote](https://doc.rust-lang.org/reference/tokens.html#quote-escapes) & [Unicode](https://doc.rust-lang.org/reference/tokens.html#unicode-escapes) & [Byte](https://doc.rust-lang.org/reference/tokens.html#byte-escapes).

Nul bytes are disallowed, whether as escape code or source character (e.g. `"\0"`, `"\x00"`, `"\u{0}"` or `"␀"`).

Unicode characters are accepted and encoded as UTF-8. That is, `c"🦀"`, `c"\u{1F980}"` and `c"\xf0\x9f\xa6\x80"` are all accepted and equivalent.

The type of the expression is [`&core::ffi::CStr`](https://doc.rust-lang.org/stable/core/ffi/struct.CStr.html). So, the `CStr` type will have to become a lang item.
(`no_core` programs that don't use `c""` string literals won't need to define this lang item.)

Interactions with string related macros:

- The [`concat` macro](https://doc.rust-lang.org/stable/std/macro.concat.html) will _not_ accept these literals, just like it doesn't accept byte string literals.
- The [`format_args` macro](https://doc.rust-lang.org/stable/std/macro.format_args.html) will _not_ accept such a literal as the format string, just like it doesn't accept a byte string literal.

(This might change in the future. E.g. `format_args!(c"…")` would be cool, but that would require generalizing the macro and `fmt::Arguments` to work for other kinds of strings. (Ideally also for `b"…"`.))

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

* No `c""` literal, but just a `cstr!()` macro. (Possibly as part of the standard library.)

  This requires [complicated machinery](https://github.com/rust-lang/rust/pull/101607/files) to implement correctly.

  The trivial implementation of using `concat!($s, "\0")` is problematic for several reasons, including non-string input and embedded nul bytes.
  (The unstable `concat_bytes!()` solves some of the problems.)

  The popular [`cstr` crate](https://crates.io/crates/cstr) is a proc macro to work around the limitations of a `macro_rules` implementation, but that also has many downsides.

  Even if we had the right language features for a trivial correct implementation, there are many code bases where C strings are the primary form of string,
  making `cstr!("..")` syntax quite cumbersome.

- No `c""` literal, but make it possible for `""` to implicitly become a `&CStr` through magic.

  We already allow integer literals (e.g. `123`) to become one of many types, so perhaps we could do the same to string literals.

  (It could be a built-in fixed set of types (e.g. just `str`, `[u8]`, and `CStr`),
  or it could be something extensible through something like a `const trait FromStringLiteral`.
  Not sure how that would exactly work, but it sounds cool.)

* Allowing only valid UTF-8 and unicode-oriented escape codes (like in `"…"`, e.g. `螃蟹` or `\u{1F980}` but not `\xff`).

  For regular string literals, we have this restriction because `&str` is required to be valid UTF-8.
  However, C literals (and objects of our `&CStr` type) aren't necessarily valid UTF-8.

* Allowing only ASCII characters and byte-oriented escape codes (like in `b"…"`, e.g. `\xff` but not `螃蟹` or `\u{1F980}`).

  While C literals (and  `&CStr`) aren't necessarily valid UTF-8, they often do contain UTF-8 data.
  Refusing to put UTF-8 in it would make the feature less useful and would unnecessarily make it harder to use unicode in programs that mainly use C strings.

* Having separate `c"…"` and `bc"…"` string literal prefixes for UTF-8 and non-UTF8.

  Both of those would be the same type (`&CStr`). Unless we add a special "always valid UTF-8 C string" type, there's not much use in separating them.

* Use `z` instead of `c` (`z"…"`), for "zero terminated" instead of "C string".

  We already have a type called `CStr` for this, so `c` seems consistent.

- Also add `c'…'` as [`c_char`](https://doc.rust-lang.org/stable/core/ffi/type.c_char.html) literal.

  It'd be identical to `b'…'`, except it'd be a `c_char` instead of `u8`.

  This would easily lead to unportable code, since `c_char` is `i8` or `u8` depending on the platform. (Not a wrapper type, but a direct type alias.)
  E.g. `fn f(_: i8) {} f(c'a');` would compile only on some platforms.

  An alternative is to allow `c'…'` to implicitly be either a `u8` or `i8`. (Just like integer literals can implicitly become one of many types.)

# Drawbacks
[drawbacks]: #drawbacks

- The `CStr` type needs some work. `&CStr` is currently a wide pointer, but it's supposed to be a thin pointer. See https://doc.rust-lang.org/1.65.0/src/core/ffi/c_str.rs.html#87

  It's not a blocker, but we might want to try to fix that before stabilizing `c"…"`.

# Prior art
[prior-art]: #prior-art

- C has C string literals (`"…"`). :)
- Nim has `cstring"…"`.
- COBOL has `Z"…"`.
- Probably a lot more languages, but it's hard to search for. :)

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Also add `c'…'` C character literals? (`u8`, `i8`, `c_char`, or something more flexible?)

- Should we make `&CStr` a thin pointer before stabilizing this? (If so, how?)

- Should the (unstable) [`concat_bytes` macro](https://github.com/rust-lang/rust/issues/87555) accept C string literals? (If so, should it evaluate to a C string or byte string?)

# Future possibilities
[future-possibilities]: #future-possibilities

(These aren't necessarily all good ideas.)

- Make `concat!()` or `concat_bytes!()` work with `c"…"`.
- Make `format_args!(c"…")` (and `format_args!(b"…")`) work.
- Improve the `&CStr` type, and make it FFI safe.
- Accept unicode characters and escape codes in `b""` literals too: [RFC 3349](https://github.com/rust-lang/rfcs/pull/3349).
- More prefixes! `w""`, `os""`, `path""`, `utf16""`, `brokenutf16""`, `utf32""`, `wtf8""`, `ebcdic""`, …
- No more prefixes! Have `let a: &CStr = "…";` work through magic, removing the need for prefixes.
  (That won't happen any time soon probably, so that shouldn't block `c"…"` now.)
