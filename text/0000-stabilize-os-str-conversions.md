- Feature Name: `convert` (the `OsStr` ones)
- Start Date: 2015-08-13
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Tweak convenience methods for converting between OS strings and Rust's utf-8
string types, placing them on track for stabilization.

# Motivation

Dealing with OS strings in a cross-platform fashion can sometimes be inherently
unergonomic. To recap, the fundamental problem is that on Unix an OS string is
basically `Vec<u8>` where on Windows it's `Vec<u16>`. There's no possible way to
convert between these two types without performing some form of interpretation
of the contents (e.g.  considering them unicode).

Much effort has been put into the standard library to never require viewing the
contents of an OS string. There are many high-level functions on the OS string
types themselves as well as the wrappers found in `std::path`, for example. In
many cases this means that programs never need to actually interpret the
contents of an OS string, and can happily ship around the bits as necessary.
There are situations, however, where the contents need to either be interpreted
or an OS string needs to be manufactured from some contents. For example:

* Files on all platforms are byte oriented, so storing an OS string (e.g. a
  path) in a file requires viewing the path's contents as an array of bytes.
* Various protocols and formats which exist today are structured such that path
  names are encoded as an array of bytes. For example tarballs do this as well
  as scp. This also requires viewing OS strings as an array of bytes and
  sometimes constructing an OS string from an array of bytes.
* Many C libraries are not written with the same OS string abstraction that the
  standard library has, so they ubiquitously use `char*` for paths. This means
  that FFI bindings in Rust wanting to use `Path` instead must convert a list of
  bytes to an from an OS string.
* On Windows most robust APIs take a wide string (e.g. `&[u16]`) and for any of
  the situations above producers need to transform a `&[u8]` path into a wide
  string somehow.

The crux of these scenarios is that an OS string needs to either be converted to
`&[u8]`/`&[u16]` or it needs to be constructed from these contents. If code is
mostly written for one platform then there normally isn't a problem. For example
on Unix OS strings are freely convertible between byte arrays and back, and on
Windows the same is true for `u16` arrays. Problems can arise, however, when a
library wants to perform these operations across all platforms.

The functions being stabilized in this RFC are intended to provide convenient,
yet fallible helper methods for performing these conversions. Currently
byte-oriented, on Unix the methods simply pass through all contents (as
everything is bytes) and on Windows crossing the `u16` to `u8` boundary involves
interpreting the contents as valid unicode. The methods are all fallible as the
unicode interpretation on Windows may fail.

These convenience functions enable crates to avoid brittle `#[cfg]` logic while
supporting a large number of cases right off the bat for both Unix and Windows.

# Detailed design

For converting an array of bytes into an OS string the following functions will
be provided.

```rust
impl OsString {
    fn from_narrow(bytes: Vec<u8>) -> Result<OsString, FromNarrowError>;
    fn from_narrow_lossy(bytes: &[u8]) -> Cow<OsStr>;
}

impl OsStr {
    fn from_narrow(bytes: &[u8]) -> Option<&OsStr>;
}

impl FromNarrowError {
    fn into_bytes(self) -> Vec<u8>;
}
```

> Note: the `OsString::from_bytes` function today has been renamed here and the
> generics have been removed.

* On Unix, simply transmute the provided bytes and always succeed.
* On Windows, the fallible variants will only succeed if the bytes are valid
  utf-8 and the lossy case is the same as `String::from_utf8_lossy`.

Next, the following methods will be available for extracting a sequence of bytes
out of an OS string.

```rust
impl OsStr {
    fn to_narrow(&self) -> Option<&[u8]>;
    fn to_narrow_lossy(&self) -> Cow<[u8]>;
    fn to_cstring(&self) -> Result<CString, ToCStringError>;
    fn to_cstring_lossy(&self) -> Result<CString, NulError>;
}

impl ToCStringError {
    fn nul_error(&self) -> Option<&NulError>;
}
```

The semantics of these functions will be:

* On Unix always succeed by just working on the internal list of bytes.
* On Windows, attempt to interpret the `&[u16]` as utf-16, and if successful
  convert to utf-8 and perform the same as Unix. If the utf-16 interpretation
  fails an error is returned. The lossy functions will be equivalent to using
  `String::from_utf16_lossy` to get a list of bytes.

# Drawbacks

The platform-specific behavior of these functions can be surprising to some
programs. It's relatively easy to leverage these functions and witness that they
never fail in a Unix environment, allowing use of `unwrap` to go unnoticed and
discouraging proper error handling. There is, however, a general culture of
avoiding `unwrap` in robust code in Rust.

Additionally, although these functions have platform-specific behavior, they're
enabling as many successful use cases on each platform as possible. The failure
mode of `u16` to `u8` conversion on Windows is the only case where `None` is
returned, and it's inevitable that applications need to make a decision of what
to do in this case regardless (e.g. apply a lossy conversion or return an
error).

# Alternatives

Outlined in [rust-lang/rust#27657][pr], an alternative would be to provide
infallible interpretations of OS strings as either `[u16]` or `[u8]`. In each
direction either Unix or Windows would be lossless but the opposite platform
would use the `from_utf8_lossy` family of functions on strings to replace
ill-formed unicode with unicode replacement characters.

[pr]: https://github.com/rust-lang/rust/pull/27657

The downside of this approach, however, is that an infallible conversion implies
some form of lossiness across platforms, which is easy to forget and start
silently relying on in applications. Additionally, it's possible to build the
lossy version on top of the non-lossy versions proposed in this RFC.

# Unresolved questions

* Is `platform` an appropriate name to have in these methods? Does it correctly
  convey the platform-specific functionality?
