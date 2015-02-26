- Feature Name: c_string_from_iter
- Start Date: 2015-02-26
- RFC PR:
- Rust Issue:

# Summary

Amend the methods available to construct a `CString` to improve composability
and follow the conventions emerging elsewhere in the standard library.

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

3. Stylistic: `Result` as return value type of `new` feels a bit too 'loaded'.
   It's used in some other places, but the general expectation on `new` is to
   be the most straightforward way to obtain a value of the type, while more
   involved failure modes tend to be more typical on `from_*` constructors
   and the like.

[rfc 592]: https://github.com/rust-lang/rfcs/pull/592
[rfc 840]: https://github.com/rust-lang/rfcs/pull/840

# Detailed design

Replace the constructor accepting `IntoBytes` with one accepting
`IntoIterator`, following the `from_iter` pattern in collection types:

```rust
impl CString {
    pub fn from_iter<I>(iterable: I) -> Result<CString, NulError>
        where I: IntoIterator<Item=u8>
    { ... }
}
```

`CString::from_vec` should be reinstated as an optimized special case.
`CString::from_cow_string` can be added later on.

# Proof of concept

As usual for my RFCs concerning `CString`, the proposed changes are
implemented on its workalike `CStrBuf` in crate
[c_string](https://github.com/mzabaluev/rust-c-str).

# Drawbacks

`IntoIterator` is slightly less convenient than `IntoBytes` for converting
from standard Rust strings and byte slices.
This can be bridged over by providing an auxiliary trait as described in
the [Unresolved questions](#unresolved-questions) section below.

# Alternatives

None put forward so far. Living with `IntoBytes` is tolerable.

# Unresolved questions

An auxiliary trait could be provided to take over the convenience and
optimization aspects of `IntoBytes`, but changed to return a
`Result<CString, NulError>` directly.
