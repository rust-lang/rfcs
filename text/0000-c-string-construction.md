- Feature Name: c_string_from_iter
- Start Date: 2015-02-26
- RFC PR:
- Rust Issue:

# Summary

This is currently a fallback proposal in case
[generic conversion traits](https://github.com/rust-lang/rfcs/pull/529)
are not adopted. The changes proposed here amend the methods available to
construct a `CString` for more flexibility and better possibilities for
optimization.

# Motivation

The implementation of RFCs [#592][rfc 592] and [#840][rfc 840] has resolved
most of the issues with the design of `std::ffi::CString`, but some new APIs
have been created at the same time, needing stabilization. This proposal
aims at addressing the following issues:

1. The `IntoBytes` trait does not seem wholly justified: it falls short of
   supporting `IntoIterator`, and has become yet another special-interest trait
   to care about when designing APIs producing string-like data. 

2. The exposure of `Vec` as an intermediate type for all conversions
   precludes small-string optimizations, such as an in-place variant
   implemented in [c_string](https://github.com/mzabaluev/rust-c-str).

[rfc 592]: https://github.com/rust-lang/rfcs/pull/592
[rfc 840]: https://github.com/rust-lang/rfcs/pull/840

# Detailed design

Replace `IntoBytes` with trait `IntoCString`, with the return type
of the conversion method changed to `Result<CString, NulError>`.
All implementations of `IntoBytes` are converted to `IntoCString`,
and the generic bound on parameter of `CString::new` is changed to
`IntoCString`.

A constructor accepting `IntoIterator` should also be added,
following the `from_iter` pattern in collection types:

```rust
impl CString {
    pub fn from_iter<I>(iterable: I) -> Result<CString, NulError>
        where I: IntoIterator<Item=u8>
    { ... }
}
```

# Proof of concept

As usual for my RFCs concerning `CString`, most of the proposed changes are
implemented on its workalike `CStrBuf` in crate
[c_string](https://github.com/mzabaluev/rust-c-str).

# Drawbacks

None identified.

# Alternatives

Implement [generic conversion traits](https://github.com/rust-lang/rfcs/pull/529).

Living with `IntoBytes` is tolerable as it is.

# Unresolved questions

Stylistic issue: `Result` as return value type of `new` feels a bit too
'loaded'. It's used in some other places, but the general expectation on `new`
is to be the most straightforward way to obtain a value of the type, while more
involved failure modes tend to be more typical on `from_*` constructors
and the like.
