- Feature Name: `const_char_encode_utf8`
- Start Date: 2024-09-17
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

`char::encode_utf8` should be marked const to allow for compile-time conversions.

# Motivation
[motivation]: #motivation

The `encode_utf8` method (in `char`) is currently **not** marked as "const" and is therefore rendered unusable in scenarios that require const-compatibility.

With the recent stabilisation of [`const_mut_refs`](https://github.com/rust-lang/rust/issues/57349/), implementing `encode_utf8` with the same parameters is trivial would yield no incompatibilities with existing code.

I expect that implementing this RFC &ndash; despite its limited scope &ndash; will however prove useful in supporting compile-time string handling in the future.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Currently, the `encode_utf8` method has the following prototype:

```rust
pub fn encode_utf8(self, dst: &mut [u8]) -> &mut str;
```

This is to simply be marked as const:

```rust
pub const fn encode_utf8(self, dst: &mut [u8]) -> &mut str;
```

This is not a breaking change.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Other than just adding the `const` qualifier to the function signature, the function body would have to be changed due to some constructs currently not being supported in constant expressions.

A working implementation can be found at [`bjoernager/rust:const-char-encode-utf8`](https://github.com/bjoernager/rust/tree/const-char-encode-utf8).

# Drawbacks
[drawbacks]: #drawbacks

Implementing this RFC at the current moment could degenerate diagnostics as the `assert` call in `encode_utf8_raw` relies on formatters that are non-const.

The reference implementation resolves this by instead using a generic message, although this may not be desired.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

If the initial diagnostics are deemed to be worth more than const-compatibility then an `encode_utf8_unchecked` method could be considered instead:

```rust
pub const unsafe fn encode_utf8_unchecked(self, dst: &mut [u8]) -> &mut str;
```

This function would perform the same operation but without testing the length of `dst`.
This would in turn allow const conversions &ndash; if very needed &ndash; without changing diagonstic messages.

# Prior art
[prior-art]: #prior-art

Currently none that I know of.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None at the moment.

# Future possibilities
[future-possibilities]: #future-possibilities
