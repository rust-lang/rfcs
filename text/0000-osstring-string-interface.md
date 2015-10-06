- Feature Name: osstring_string_interface
- Start Date: 2015-10-05
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Add a string-like API to the `OsString` and `OsStr` types.  This RFC
focuses on creating a string-like interface, as opposed to RFC #1307,
which focuses more on container-like features.

# Motivation

As mentioned in the `std::ffi::os_str` documentation: "**Note**: At
the moment, these types are extremely bare-bones, usable only for
conversion to/from various other string types. Eventually these types
will offer a full-fledged string API."  This is intended as a step in
that direction.

Having an ergonomic way to manipulate OS strings is needed to allow
programs to easily handle non-UTF-8 data received from the operating
system.  Currently, it is common for programs to just convert OS data
to `String`s, which leads to undesirable panics in the unusual case
where the input is not UTF-8.  For example, currently, calling rustc
with a non-UTF-8 command line argument will result in an immediate
panic.  Fixing that in a way that actually handles non-UTF-8 data
correctly (as opposed to, for example, just interpreting it lossily as
UTF-8) would be very difficult with the current OS string API.  Most
of the functions proposed here were motivated by the OS string
processing needs of rustc.

# Detailed design

## `OsString`

`OsString` will get the following new method:
```rust
/// Converts an `OsString` into a `String`, avoiding a copy if possible.
///
/// Any non-Unicode sequences are replaced with U+FFFD REPLACEMENT CHARACTER.
pub fn into_string_lossy(self) -> String;

```

This is analogous to the existing `OsStr::to_string_lossy` method, but
transfers ownership.  This operation can be done without a copy if the
`OsString` contains UTF-8 data or if the platform is Windows.

## `OsStr`

OsStr will get the following new methods:
```rust
/// Returns true if the string starts with a valid UTF-8 sequence
/// equal to the given `&str`.
fn starts_with_str(&self, prefix: &str) -> bool;

/// If the string starts with the given `&str`, returns the rest
/// of the string.  Otherwise returns `None`.
fn remove_prefix_str(&self, prefix: &str) -> Option<&OsStr>;

/// Retrieves the first character from the `OsStr` and returns it
/// and the remainder of the `OsStr`.  Returns `None` if the
/// `OsStr` does not start with a character (either because it it
/// empty or because it starts with non-UTF-8 data).
fn slice_shift_char(&self) -> Option<(char, &OsStr)>;

/// If the `OsStr` starts with a UTF-8 section followed by
/// `boundary`, returns the sections before and after the boundary
/// character.  Otherwise returns `None`.
fn split_off_str(&self, boundary: char) -> Option<(&str, &OsStr)>;

/// Returns an iterator over sections of the `OsStr` separated by
/// the given character.
///
/// # Panics
///
/// Panics if the boundary character is not ASCII.
fn split<'a>(&'a self, boundary: char) -> Split<'a>;
```

These methods fall into two categories.  The first four
(`starts_with_str`, `remove_prefix_str`, `slice_shift_char`, and
`split_off_str`) interpret a prefix of the `OsStr` as UTF-8 data,
while ignoring any non-UTF-8 parts later in the string.  The last is a
restricted splitting operation.

### `starts_with_str`

`string.starts_with_str(prefix)` is logically equivalent to
`string.remove_prefix_str(prefix).is_some()`, but is likely to be a
common enough special case to warrant it's own clearer syntax.

### `remove_prefix_str`

This could be used for things such as removing the leading "--" from
command line options as is common to enable simpler processing.
Example:
```rust
let opt = OsString::from("--path=/some/path");
assert_eq!(opt.remove_prefix_str("--"), Some(OsStr::new("path=/some/path")));
```

### `slice_shift_char`

This performs the same function as the similarly named method on
`str`, except that it also returns `None` if the `OsStr` does not
start with a valid UTF-8 character.  While the `str` version of this
function may be removed for being redundant with `str::chars`, the
functionality is still needed here because it is not clear how an
iterator over the contents of an `OsStr` could be defined in a
platform-independent way.

An intended use for this function is for interpreting bundled
command-line switches.  For example, with switches from rustc:

```rust
let mut opts = &OsString::from("vL/path")[..]; // Leading '-' has already been removed
while let Some((ch, rest)) = opts.slice_shift_char() {
    opts = rest;
    match ch {
        'v' => { verbose = true; }
        'L' => { /* interpret remainder as a link path */ }
        ....
    }
}
```

### `split_off_str`

This is intended for interpreting "tagged" OS strings, for example
rustc's `-L [KIND=]PATH` arguments.  It is expected that such tags
will usually be UTF-8.  Example:
```rust
let s = OsString::from("dylib=/path");

let (name, kind) = match s.split_off_str('=') {
    None => (&*s, cstore::NativeUnknown),
    Some(("dylib", name)) => (name, cstore::NativeUnknown),
    Some(("framework", name)) => (name, cstore::NativeFramework),
    Some(("static", name)) => (name, cstore::NativeStatic),
    Some((s, _)) => { error(...) }
};
```

### `split`

This is similar to the similarly named function on `str`, except the
splitting boundary is restricted to be an ASCII character instead of a
general pattern.  ASCII characters have well-defined meanings in both
flavors of OS string, and the portions before and after such a
character are always well-formed OS strings.

This is intended for interpreting OS strings containing several paths.
Using this function will generally restrict the allowed paths to those
not containing the separator, but this is a common limitation already
in such interfaces.  For example, rustc's `--emit dep-info=bar.d,link`
could be processed as:
```rust
let arg = OsString::from("dep-info=bar.d,link");

for part in arg.split(',') {
    match part.split_off_str('=') {
        ...
    }
}
```

## `SliceConcatExt`

Implement the trait
```rust
impl<S> SliceConcatExt<OsStr> for [S] where S: Borrow<OsStr> {
    type Output = OsString;
    ...
}
```

This has the same behavior as the `str` version, except that it works
on OS strings.  It is intended as a more convenient and efficient way
of building up an `OsString` from parts than repeatedly calling
`push`.

# Drawbacks

This is a somewhat unusual string interface in that much of the
functionality either accepts or returns a different type of string
than the one the interface is designed to work with (`str` instead of
the probably expected `OsStr`).

# Alternatives

## Interfaces without `str`

Versions of the `*_str` functions that take or return `&OsStr`s seem
more natural, but in at least some of the cases it is not possible to
implement such an interface.  For example, on Windows, the following
should hold using a hypothetical `remove_prefix(&self, &OsStr) ->
Option<&OsStr>`:

```rust
let string = OsString::from("😺"); // [0xD83D, 0xDE3A] in UTF-16
let prefix: OsString = OsStringExt::from_wide(&[0xD83D]);
let suffix: OsString = OsStringExt::from_wide(&[0xDE3A]);

assert_eq!(string.remove_prefix(&prefix[..]), Some(&suffix[..]));
```

However, the slice `&suffix[..]` (internally `[0xED, 0xB8, 0xBA]`)
does not occur anywhere in `string` (internally `[0xF0, 0x9F, 0x98,
0xBA]`), so there would be no way to construct the return value of
such a function.

## Different forms for `split`

The restriction of the argument of `split` to ASCII characters is a
very conservative choice.  It would be possible to allow any Unicode
character as the divider, at the expense of creating somewhat strange
situations where, for example, applying `split` followed by `concat`
produces a string containing the divider character.  As any interface
manipulating OS strings is generally non-Unicode, needing to split on
non-ASCII characters is likely rare.

In some ways, it would be more natural to split on bytes in Unix and
16-bit code units in Windows, but it would be difficult to present a
cross-platform interface for such functionality and implementations on
Windows would have similar issues to those in the `remove_prefix`
example above.

# Unresolved questions

It is not obvious that the `split` function's restriction to ASCII
dividers is the correct interface.

There are many directions this interface could be extended in.  It
would be possible to proved a subset of this functionality using
`OsStr` rather than `str` in the interface, and it would also be
possible to create functions that interacted with non-prefix portions
of `OsStr`s.  It is not clear whether the usefulness of these
interfaces is high enough to be worth pursuing them at this time.
