- Start Date: 2014-11-09
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

This RFC is a proposal to allow any Unicode character in the byte literals.

# Motivation

[RFC
69](https://github.com/rust-lang/rfcs/blob/master/text/0069-ascii-literals.md)
rules that the byte literals must contain ASCII characters only. By relieving
this restriction, we can code the UTF-8 byte literals directly.

Currently, there are at least three ways to generate a byte literal that
contains non-ASCII characters.

1. `bytes!("안녕")`
2. `"안녕".as_bytes()`
3. `b"\xec\x95\x88\xeb\x85\x95"`

The first method is now deprecated, by introducing the `b"literal"` notation.
The second one works, though it needs an additional function call. It may get
some optimization in near future, but is still a bit bothersome. The last one
has no performance impact, but is too hard to type.

# Detailed design

Every non-ASCII character in the byte literals will be interpreted as UTF-8,
rather than emitting an error. This change is backward compatible because the
existing ASCII-only byte literals will work as before.

As the encoding of the source code is forced to UTF-8, we can directly know the
representation of the non-ASCII characters in the byte literals. It is similar
to the string literals, except the byte literals can include non-UTF-8 data
using the \xXX escape sequences.

The biggest concern is that we are assuming the non-ASCII characters in the byte
literals to be UTF-8. It may be other encodings. But, the same UTF-8 assumption
is made on the string literals. The only difference is that they are
UTF-8-ensured.

It seems that the idea of ASCII-only originates from
[rust-lang/rust#4334](https://github.com/rust-lang/rust/issues/4334). One of the
advantages it says is that by allowing only ASCII characters, there will be a
very clear distinction between the string literals and the byte literals. But,
there are two concerns:

1. The string literals and the byte literals should be distinguished through the
   type system, not by preventing some characters.
2. If the literal has only ASCII characters, the clear distinction is still not
   possible.

# Drawbacks

- The assumption of the UTF-8 encoding may seem too brave.

# Alternatives

- Leave `b"literal"` as is, and make the compiler optimize
  `"literal".as_bytes()` to a byte literal. The typing is still a bit
  bothersome, but the choice is safer than this RFC.

# Unresolved questions

None.
