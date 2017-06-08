- Feature Name: A type representing an owned C-compatible wide string
- Start Date: 2016-10-20
- RFC PR:
- Rust Issue:

# Summary

Add CWideString/CWideStr for more simple interaction with not well-formed UTF-16
external API (for example: with Windows API).

# Motivation

This RFC born from issue: [rust-lang/rust#36671](https://github.com/rust-lang/rust/issues/36671)

Many of Windows API use not well-formed UTF-16 strings. Some of this API use
null-terminated strings.

Rust lack simple null-terminated UTF-16 string conversions and now working with this
API need too many boilerplate copy-paste code like:
```
unsafe fn from_wide_null(ptr: *const u16) -> OsString {
    let mut len = 0;
    while *ptr.offset(len) != 0 {
        len += 1;
    }
    OsStringExt::from_wide(slice::from_raw_parts(ptr, len as usize))
}

fn to_wide_null(s: &str) -> Vec<u16> {
    self.encode_utf16().chain(Some(0)).collect()
}
```

OsString also can't be used with Windows API directly, because it stores string internally
in WTF-8 encoding.

So, this RFC try to add simple and effective way to work with UTF-16 string.

Also it can be usefull:

 * Inside Rust OsString implementatino on Windows platform;
 * With Java FFI (Java internally use UTF-16 strings).

# Detailed design

Copy CStr/CString as CWideStr/CWideString with using u16 instead of u8.

A preliminary implementation can be found at: https://github.com/bozaro/rust-cwstring

# Drawbacks

This classes is not generally platform specific, but it mostly useful on Windows.

Also CWideString name can be confused, because it use differ element size then
`std::wstring` in C++ world (`std::wstring` type defined as `std::basic_string<wchar_t>`,
but `sizeof(wchar_t)` is platform depended: 4 bytes on Linux, 2 bytes on Windows).

# Alternatives

Keep all as is.

# Unresolved questions

I try implement CWideStr/CWideString and got some issues/questions:

 * NulError is copied as WideNulError, but it breaking currect code or name convensions.
 * into_string method is removed, because unlike CString, he does not give the performance
   profit. Also IntoStringError is not copied from c_str.
 * I don't find good name for `u16` method like `to_bytes`.
 * memchr and strlen replaced by wmemchr and wstrlen failback implementation.
 * May be need better implementation fmt::Debug for CWideStr.
 