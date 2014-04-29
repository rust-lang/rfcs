- Start Date: 2014-04-15
- RFC PR #:
- Rust Issue #:

# Summary

Add `TextReader` and `TextWriter` traits to `std::io` for Unicode text-oriented streams,
like `Reader` and `Writer` are for byte-oriented streams.
The API design of text-oriented streams guarantees well-formed Unicode scalar values (characters),
so that there is no need to deal with e.g. errors caused by invalid UTF-8 in an input byte sequence.


# Motivation

When dealing with a potentially large amount of data,
we prefer doing so incrementally rather than having the data set
and all of its intermediate representations entirely in memory.
This is why `Reader` and `Writer` were added.

Additionally, experience in other programming language has taught us of
[the Unicode sandwich](http://nedbatchelder.com/text/unipain.html):
when dealing with text, the best practice is to handle Unicode only internally
(in Rust: `char`, `str` and `StrBuf`; as opposed to `u8` and `[u8]`),
and convert to or from bytes at the program’s boundaries, when doing I/O.
Byte-oriented streams are good, but we also need text-oriented streams.

For example, JSON is defined in terms of Unicode code points.
Encoding these code points to UTF-8 for transmission is completely orthogonal
to JSON itself.
Our `serialize::json` module could be based on text streams,
and avoid [the redundant UTF-8 valitiy check](https://github.com/mozilla/rust/blob/30e373390f1a2f74e78bf9ca9c8ca68451f3511a/src/libserialize/json.rs#L329)
that’s involved when getting a `~str` from a byte stream.

[rust-encoding](https://github.com/lifthrasiir/rust-encoding)
will provide wrappers to "convert" between byte streams and text streams.
For example, one that takes a `Writer`, an encoding, and an error handling behavior,
and provides a `TextWriter`.

Eventually, we could open a file directly in text mode with a given encoding
and obtain a text stream.


# Detailed design


```rust
/// A minimal implementation only needs `write_str`.
/// However, a writer that is not based on UTF-8 may prefer
/// to override `write_char` as their "most fundamental" method,
/// and implement `write_str` with:
///
///
///     fn write_str(&mut self, buf: &str) -> IoResult<()> {
///         for c in buf.chars {
///             try!(write_char(c))
///         }
///         Ok(())
///     }
pub trait TextWriter {
    fn write_str(&mut self, buf: &str) -> IoResult<()>;

    // These are similar to Writer, but based on `write_str` instead of `write`.
    fn write_char(&mut self, c: char) -> IoResult<()> { ... }
    fn write_line(&mut self, s: &str) -> IoResult<()> { ... }
    fn write_uint(&mut self, n: uint) -> IoResult<()> { ... }
    fn write_int(&mut self, n: int) -> IoResult<()> { ... }

    // These are similar to Writer
    fn flush(&mut self) -> IoResult<()> { ... }
    fn by_ref<'a>(&'a mut self) -> RefTextWriter<'a, Self> { ... }
}
```

Other than `write_char`, the set of default methods is just an idea.

If and when [#7771](https://github.com/mozilla/rust/issues/7771) is implemented,
`write_str` can have a default implementation based on `write_char`
with `#[requires(one_of(write_str, write_char)]` on the trait.



```rust
pub trait TextReader {
    fn read(&mut self, buf: &mut StrBuf, max_bytes: uint) -> IoResult<uint>;

    // These are similar to Reader
    fn read_to_end(&mut self) -> IoResult<~str> { ... }
    fn bytes<'r>(&'r mut self) -> Bytes<'r, Self> { ... }
    fn by_ref<'a>(&'a mut self) -> RefReader<'a, Self> { ... }

    // These are similar to Buffer
    fn read_line(&mut self) -> IoResult<~str> { ... }
    fn lines<'r>(&'r mut self) -> Lines<'r, Self> { ... }
    fn read_until<C: CharEq>(&mut self, char: C) -> IoResult<~str> { ... }
    fn read_char(&mut self) -> IoResult<char> { ... }
    fn chars<'r>(&'r mut self) -> Chars<'r, Self> { ... }
}
```

The set of default methods here is just an idea.


# Alternatives

* Let rust-encoding define `TextReader` and `TextWriter` itself and revisit later.
* We may want `TextReader` to be closer to `std::io::Buffer` (which requires `Reader`) rather than just `Reader`


# Unresolved questions

* Which of these things should have text-oriented equivalents?
  The `Buffer`, `Seek`, and `Stream` traits,
  their buffered wrapper implementations,
  the readers and writers in `std::io::util`.
