- Feature Name: `const_char_encode_utf8`
- Start Date: 2024-09-17
- RFC PR: [rust-lang/rfcs#3696](https://github.com/rust-lang/rfcs/pull/3696)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

`char::encode_utf8` should be marked const to allow for compile-time conversions.
Considering mutable references now being stable in const environments, this implementation would be trivial even without compiler magic.

# Motivation
[motivation]: #motivation

The `encode_utf8` method (in `char`) is currently **not** marked as "const" and is therefore rendered unusable in scenarios that require const-compatibility.

With the recent stabilisation of [`const_mut_refs`](https://github.com/rust-lang/rust/issues/57349/), implementing `encode_utf8` with the current signature is trivial and would (in practice) yield no incompatibilities with existing code.

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

Other than just adding the `const` qualifier to the function prototype, the function body would have to be changed due to some constructs currently not being supported in constant expressions.

A working implementation can be found at [`bjoernager/rust:const-char-encode-utf8`](https://github.com/bjoernager/rust/tree/const-char-encode-utf8).
Required changes are in [`/library/core/src/char/methods.rs`](https://github.com/bjoernager/rust/blob/const-char-encode-utf8/library/core/src/char/methods.rs/).

Note that this implementation assumes [`const_slice_from_raw_parts_mut`](https://github.com/rust-lang/rust/issues/67456/).

# Drawbacks
[drawbacks]: #drawbacks

Implementing this RFC at the current moment could degenerate diagnostics as the `assert` call in the `encode_utf8_raw` function relies on formatters that are non-const.

The reference implementation resolves this by instead using a generic message, although this may not be desired:

```
encode_utf8: buffer does not have enough bytes to encode code point
```

This *could* be changed to have the number of bytes required hard-coded, but doing so may instead sacrifice code readability.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

If the initial diagnostics are deemed to be worth more than const-compatibility then an `encode_utf8_unchecked` method could be considered instead:

```rust
pub const unsafe fn encode_utf8_unchecked(self, dst: &mut [u8]) -> &mut str;

// ... or...

pub const unsafe fn encode_utf8_unchecked(self, dst: *mut u8) -> *mut str;
```

This function would perform the same operation but without testing the length of `dst`, allowing for const conversions at least in the short-term (until formatters are stabilised).

# Prior art
[prior-art]: #prior-art

Currently none that I know of.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

The problem with diagnostic degeneration could be solved by allowing the used formatters in const environments.
I do not know if there already exists such a feature for use by the standard library.

# Future possibilities
[future-possibilities]: #future-possibilities

I suspect that having a similar `decode_utf8` method may be desired.
