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
programs to easily handle non-Unicode data received from the operating
system.  Currently, it is common for programs to just convert OS data
to `String`s, which leads to undesirable panics in the unusual case
where the input is not Unicode.  For example, currently, calling rustc
with a non-Unicode command line argument will result in an immediate
panic.  Fixing that in a way that actually handles non-Unicode data
correctly (as opposed to, for example, just interpreting it lossily)
would be very difficult with the current OS string API.

# Detailed design

The overall design of this API is to treat OS strings as mixtures of
Unicode code points and other system-specific things.  It allows the
Unicode portions to be manipulated as if they were part of a `str`,
treating the non-Unicode portions surrounding them as uninterpretable
objects.  A very limited set of operations are provided that can
examine and manipulate the non-Unicode portions, but it is expected
that any real interpretation of those sections will have to be done in
platform-specific code.

The method for deciding which portions of an `OsStr` correspond to
Unicode code points tries to be as inclusive as possible, treating a
section as Unicode if there is any possible interpretation of it in
the platform's standard Unicode encoding.

* In Windows, OS strings are sequences of ill-formed UTF-16 code
  units.  (Rust's internal representation is a WTF-8 encoded string,
  but, aside from determining what operations can be performed
  efficiently, this is not exposed in the interface.)  Unpaired
  surrogates are identified as non-Unicode, and everything else is
  treated as valid UTF-16.

* In Unix, OS strings are arbitrary byte sequences, which are often
  interpreted as UTF-8.  A byte is treated as being part of a Unicode
  section if there is any substring containing that byte that is a
  valid UTF-8 encoded character.  The self-synchronization property of
  UTF-8 guarantees that there can be at most one such substring for a
  given byte.  These code points are treated as Unicode characters,
  and all other bytes are treated as non-Unicode.  Note that this
  means that any byte with value less than 128 will be interpreted as
  Unicode.

## `OsString`

`OsString` will get the following new method:
```rust
/// Converts an `OsString` into a `String`, avoiding a copy if possible.
///
/// Any non-Unicode sequences are replaced with U+FFFD REPLACEMENT CHARACTER.
fn into_string_lossy(self) -> String;

```

This is analogous to the existing `OsStr::to_string_lossy` method, but
transfers ownership.  This operation can be done without a copy if the
`OsString` contains Unicode data or if the platform is Windows.

## `OsStr`

OsStr will get the following new methods (with supporting code
and explanations interspersed):
```rust
/// Returns an iterator over the Unicode and non-Unicode sections
/// of the string.  Sections will always be nonempty and Unicode
/// and non-Unicode sections will always alternate.
///
/// # Example
///
/// ```
/// use std::ffi::{OsStr, OsStrSection};
/// let string = OsStr::new("Hello!");
/// match string.split_unicode().next().unwrap() {
///     OsStrSection::Unicode(s) => assert_eq!(s, "Hello!"),
///     OsStrSection::NonUnicode(s) => panic!("Got non-Unicode: {:?}", s),
/// }
/// ```
fn split_unicode<'a>(&'a self) -> SplitUnicode<'a>;

struct SplitUnicode<'a> { ... }
impl<'a> Clone for SplitUnicode<'a> { ... }
impl<'a> Iterator for SplitUnicode<'a> {
    type Item = OsStrSection<'a>;
    ...
}
impl<'a> DoubleEndedIterator for SplitUnicode<'a> { ... }

#[derive(Debug, Clone, PartialEq, Eq)]
enum OsStrSection<'a> {
    Unicode(&'a str),
    NonUnicode(&'a OsStr),
}

```

This provides access to the Unicode and non-Unicode sections of the
string, as defined above.


```rust
/// Returns true if `needle` is a substring of `self`.
fn contains_os<S: AsRef<OsStr>>(&self, needle: S) -> bool;

/// Returns true if `needle` is a prefix of `self`.
fn starts_with_os<S: AsRef<OsStr>>(&self, needle: S) -> bool;

/// Returns true if `needle` is a suffix of `self`.
fn ends_with_os<S: AsRef<OsStr>>(&self, needle: S) -> bool;

/// Replaces all occurrences of one string with another.
fn replace<T: AsRef<OsStr>, U: AsRef<OsStr>>(&self, from: T, to: U) -> OsString;
```

These functions work with `OsStr` substrings of an `OsStr`, and ignore
any possible Unicode meanings.  They consider OS strings to be
composed of a sequence of platform-defined atomic objects (bytes for
Unix and code units for Windows), and then perform standard substring
operations with these "OS characters".

```rust
use std::str::pattern::{DoubleEndedSearcher, Pattern, ReverseSearcher};

/// An iterator over the non-empty substrings of `self` that
/// contain no whitespace and are separated by whitespace.
fn split_whitespace<'a>(&'a self) -> SplitWhitespace<'a>;

struct SplitWhitespace<'a> { ... }
impl<'a> Clone for SplitWhitespace<'a> { ... }
impl<'a> Iterator for SplitWhitespace<'a> {
    type Item = &'a OsStr;
    ...
}
impl<'a> DoubleEndedIterator for SplitWhitespace<'a> { ... }

/// An iterator over the lines of `self`, separated by `\n` or
/// `\r\n`.  This does not return an empty string after a trailing
/// `\n`.
fn lines<'a>(&'a self) -> Lines<'a>;

struct Lines<'a> { ... }
impl<'a> Clone for Lines<'a> { ... }
impl<'a> Iterator for Lines<'a> {
    type Item = &'a OsStr;
    ...
}
impl<'a> DoubleEndedIterator for Lines<'a> { ... }

/// Returns true if `self` matches `pat`.
///
/// Note that patterns can only match Unicode sections of the `OsStr`.
fn contains<'a, P>(&'a self, pat: P) -> bool where P: Pattern<'a> + Clone;

/// Returns true if the beginning of `self` matches `pat`.
///
/// Note that patterns can only match Unicode sections of the `OsStr`.
fn starts_with<'a, P>(&'a self, pat: P) -> bool where P: Pattern<'a>;

/// Returns true if the end of `self` matches `pat`.
///
/// Note that patterns can only match Unicode sections of the `OsStr`.
fn ends_with<'a, P>(&'a self, pat: P) -> bool
    where P: Pattern<'a>, P::Searcher: ReverseSearcher<'a>;

/// An iterator over substrings of `self` separated by characters
/// matched by a pattern.  See `str::split` for details.
///
/// Note that patterns can only match Unicode sections of the `OsStr`.
fn split<'a, P>(&'a self, pat: P) -> Split<'a, P> where P: Pattern<'a>;

struct Split<'a, P> where P: Pattern<'a> { ... }
impl<'a, P> Clone for Split<'a, P>
    where P: Pattern<'a> + Clone, P::Searcher: Clone { ... }
impl<'a, P> Iterator for Split<'a, P> where P: Pattern<'a> + Clone {
    type Item = &'a OsStr;
    ...
}
impl<'a, P> DoubleEndedIterator for Split<'a, P>
    where P: Pattern<'a> + Clone, P::Searcher: DoubleEndedSearcher<'a> { ... }

/// An iterator over substrings of `self` separated by characters
/// matched by a pattern, in reverse order.  See `str::rsplit` for
/// details.
///
/// Note that patterns can only match Unicode sections of the `OsStr`.
fn rsplit<'a, P>(&'a self, pat: P) -> RSplit<'a, P> where P: Pattern<'a>;

struct RSplit<'a, P> where P: Pattern<'a> { ... }
impl<'a, P> Clone for RSplit<'a, P>
    where P: Pattern<'a> + Clone, P::Searcher: Clone { ... }
impl<'a, P> Iterator for RSplit<'a, P>
    where P: Pattern<'a> + Clone, P::Searcher: ReverseSearcher<'a> {
    type Item = &'a OsStr;
    ...
}
impl<'a, P> DoubleEndedIterator for RSplit<'a, P>
    where P: Pattern<'a> + Clone, P::Searcher: DoubleEndedSearcher<'a> { ... }

/// Equivalent to `split`, except the trailing substring is
/// skipped if empty.  See `str::split_terminator` for details.
///
/// Note that patterns can only match Unicode sections of the `OsStr`.
fn split_terminator<'a, P>(&'a self, pat: P) -> SplitTerminator<'a, P>
    where P: Pattern<'a>;

struct SplitTerminator<'a, P> where P: Pattern<'a> { ... }
impl<'a, P> Clone for SplitTerminator<'a, P>
    where P: Pattern<'a> + Clone, P::Searcher: Clone { ... }
impl<'a, P> Iterator for SplitTerminator<'a, P> where P: Pattern<'a> + Clone {
    type Item = &'a OsStr;
    ...
}
impl<'a, P> DoubleEndedIterator for SplitTerminator<'a, P>
    where P: Pattern<'a> + Clone, P::Searcher: DoubleEndedSearcher<'a> { ... }

/// Equivalent to `rsplit`, except the trailing substring is
/// skipped if empty.  See `str::rsplit_terminator` for details.
///
/// Note that patterns can only match Unicode sections of the `OsStr`.
fn rsplit_terminator<'a, P>(&'a self, pat: P) -> RSplitTerminator<'a, P>
    where P: Pattern<'a>;

struct RSplitTerminator<'a, P> where P: Pattern<'a> { ... }
impl<'a, P> Clone for RSplitTerminator<'a, P>
    where P: Pattern<'a> + Clone, P::Searcher: Clone { ... }
impl<'a, P> Iterator for RSplitTerminator<'a, P>
    where P: Pattern<'a> + Clone, P::Searcher: ReverseSearcher<'a> {
    type Item = &'a OsStr;
    ...
}
impl<'a, P> DoubleEndedIterator for RSplitTerminator<'a, P>
    where P: Pattern<'a> + Clone, P::Searcher: DoubleEndedSearcher<'a> { ... }

/// An iterator over substrings of `self` separated by characters
/// matched by a pattern, restricted to returning at most `count`
/// items.  See `str::splitn` for details.
///
/// Note that patterns can only match Unicode sections of the `OsStr`.
fn splitn<'a, P>(&'a self, count: usize, pat: P) -> SplitN<'a, P>
    where P: Pattern<'a>;

struct SplitN<'a, P> where P: Pattern<'a> { ... }
impl<'a, P> Clone for SplitN<'a, P>
    where P: Pattern<'a> + Clone, P::Searcher: Clone { ... }
impl<'a, P> Iterator for SplitN<'a, P> where P: Pattern<'a> + Clone {
    type Item = &'a OsStr;
    ...
}

/// An iterator over substrings of `self` separated by characters
/// matched by a pattern, in reverse order, restricted to returning
/// at most `count` items.  See `str::rsplitn` for details.
///
/// Note that patterns can only match Unicode sections of the `OsStr`.
fn rsplitn<'a, P>(&'a self, count: usize, pat: P) -> RSplitN<'a, P>
    where P: Pattern<'a>;

struct RSplitN<'a, P> where P: Pattern<'a> { ... }
impl<'a, P> Clone for RSplitN<'a, P>
    where P: Pattern<'a> + Clone, P::Searcher: Clone { ... }
impl<'a, P> Iterator for RSplitN<'a, P>
    where P: Pattern<'a> + Clone, P::Searcher: ReverseSearcher<'a> {
    type Item = &'a OsStr;
    ...
}

/// An iterator over matches of a pattern in `self`.  See
/// `str::matches` for details.
///
/// Note that patterns can only match Unicode sections of the `OsStr`.
fn matches<'a, P>(&'a self, pat: P) -> Matches<'a, P> where P: Pattern<'a>;

struct Matches<'a, P> where P: Pattern<'a> { ... }
impl<'a, P> Clone for Matches<'a, P>
    where P: Pattern<'a> + Clone, P::Searcher: Clone { ... }
impl<'a, P> Iterator for Matches<'a, P> where P: Pattern<'a> + Clone {
    type Item = &'a str;
    ...
}
impl<'a, P> DoubleEndedIterator for Matches<'a, P>
    where P: Pattern<'a> + Clone, P::Searcher: DoubleEndedSearcher<'a> { ... }

/// An iterator over matches of a pattern in `self`, in reverse
/// order.  See `str::rmatches` for details.
///
/// Note that patterns can only match Unicode sections of the `OsStr`.
fn rmatches<'a, P>(&'a self, pat: P) -> RMatches<'a, P> where P: Pattern<'a>;

struct RMatches<'a, P> where P: Pattern<'a> { ... }
impl<'a, P> Clone for RMatches<'a, P>
    where P: Pattern<'a> + Clone, P::Searcher: Clone { ... }
impl<'a, P> Iterator for RMatches<'a, P>
    where P: Pattern<'a> + Clone, P::Searcher: ReverseSearcher<'a> {
    type Item = &'a str;
    ...
}
impl<'a, P> DoubleEndedIterator for RMatches<'a, P>
    where P: Pattern<'a> + Clone, P::Searcher: DoubleEndedSearcher<'a> { ... }

/// Returns a `&OsStr` with leading and trailing whitespace removed.
fn trim(&self) -> &OsStr;

/// Returns a `&OsStr` with leading whitespace removed.
fn trim_left(&self) -> &OsStr;

/// Returns a `&OsStr` with trailing whitespace removed.
fn trim_right(&self) -> &OsStr;

/// Returns a `&OsStr` with leading and trailing matches of `pat`
/// repeatedly removed.
fn trim_matches<'a, P>(&'a self, pat: P) -> &'a OsStr
    where P: Pattern<'a> + Clone, P::Searcher: DoubleEndedSearcher<'a>;

/// Returns a `&OsStr` with leading matches of `pat` repeatedly
/// removed.
fn trim_left_matches<'a, P>(&'a self, pat: P) -> &'a OsStr
    where P: Pattern<'a>;

/// Returns a `&OsStr` with trailing matches of `pat` repeatedly
/// removed.
fn trim_right_matches<'a, P>(&'a self, pat: P) -> &'a OsStr
    where P: Pattern<'a>, P::Searcher: ReverseSearcher<'a>;
```

These functions implement a subset of the string pattern matching
functionality of `str`.  They act the same as the `str` versions,
except that some of them require an additional `Clone` bound on the
pattern (because patterns are single-use objects and each Unicode
segment must be treated separately).  Patterns can only match Unicode
sections of the `OsStr`, but operations such as `split` can return
partially non-Unicode data.

### Methods not included

Most of he `str` methods not proposed for `OsStr` are those that take
or return indexes into the `str`.  Additionally, `slice_shift_at` was
left out due to its instability and likely upcoming removal from
`str`; `chars` and `parse` were left out because they don't make sense
(although a `chars_lossy` or something returning `u8`/`u16` newtype on
Unix/Windows would be possible); and `to_lowercase` and `to_uppercase`
were left out on the grounds that applying Unicode transformations to
an `OsStr` seems likely to be an unusual operation (and they can be
easily written in terms of existing functionality if someone needs
them).

Some kind of escaping function (along the lines of
`str::escape_default` or `str::escape_unicode`) might be useful, but
the correct form of such a function is unclear.

## `SliceConcatExt`

Implement the trait
```rust
impl<S> SliceConcatExt<OsStr> for [S] where S: Borrow<OsStr> {
    type Output = OsString;
    ...
}
```

This has the same behavior as the `str` version, except that it works
on OS strings.  It is a more convenient and efficient way of building
up an `OsString` from parts than repeatedly calling `push`.

# Drawbacks

This is a somewhat unusual string interface in that many of the
functions only accept Unicode data, while the type can encode more
general strings.  Unfortunately, in many cases it is not possible to
generalize the interface to accept non-Unicode input.  For example, on
Windows, the following should hold using a hypothetical `split(&self,
&OsStr) -> Split`:

```rust
let string = OsString::from("😺"); // [0xD83D, 0xDE3A] in UTF-16
let prefix: OsString = OsStringExt::from_wide(&[0xD83D]);
let suffix: OsString = OsStringExt::from_wide(&[0xDE3A]);

assert_eq!(string.split(&suffix[..]).next(), Some(&prefix[..]));
```

However, `string` is represented internally as the WTF-8 bytes `[0xF0,
0x9F, 0x98, 0xBA]`, and the slice `&prefix[..]` would be represented
as `[0xED, 0xA0, 0xBD]`.  Since this sequence of bytes does not occur
anywhere in `string`, there is no way to construct the borrowed return
value.

It would be possible to design an interface that returned
`Cow<OsStr>`, but this would be a significant departure from the `str`
interface.  If such functions are determined to be sufficiently useful
they can be added at a later time.

# Alternatives

Create a new API without copying `str` as closely as possible.

## Stricter bounds on the pattern-accepting iterator constructors

The proposed bounds on the pattern-accepting functions are the weakest
possible.  This means that one can often construct an "iterator" that
does not actually implement the `Iterator` trait.  For example, one
can call `split` with any `P: Pattern<'a>`, but the resulting `Split`
struct only implements the `Iterator` trait if `P` is additionally
`Clone`. This is likely to be confusing, so tightening the bounds may
be desirable.

# Unresolved questions

The correct behavior of `split`, `matches`, and similar functions with
a pattern that matches the empty string is not clear.  Possibilities
include:

* panic
* match on "character boundaries", probably defined as the ends of the
  string and adjacent to each Unicode character.
* define the behavior to commute with `to_string_lossy` (assuming the
  pattern does not match anything including the replacement character)

In any case, care should be taken to handle patterns that can match
both the empty string and non-empty strings correctly.

# Future work

There are many common operations that, while possible to perform using
this interface, are still undesirably difficult.  It may be desirable
to add functions to simplify these operations, but such a proposal
should consider modifying the `str` interface at the same time, and so
is out of scope of this RFC.

(An example of such a difficult operation is reading and removing a
pattern match from the start of a string.  For an `OsStr` this will
most likely be performed by using both `matches` and `splitn`, which
duplicates the work of performing the pattern matching.  For `str`
this operation can be performed using a single search followed by
slicing.)
