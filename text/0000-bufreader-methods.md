- Feature Name: bufreader-methods 
- Start Date: 7-25-2015
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Add two new methods to the `BufReader` struct, `read_into_buf()` and `get_buf()`, which would allow users more control
over the buffer and when it should be read into, as well as to access it without having to perform I/O.

# Motivation

The current API of `BufReader` and its `BufRead` impl does not give the user any control over *when* the buffer should be read into,
at least not without consuming bytes from the buffer. `BufReader`'s implementation of `BufRead::fill_buf()` 
will only perform a read from the underlying source if the buffer is *empty*. This is problematic for users who want to work with
the data in the buffer but may need to pull in more data without emptying the buffer.

The author's use-case consists of a struct that wraps `BufReader` and uses it to search streaming HTTP request bodies for specific byte sequences.
The problem arises when a partial read from the underlying stream cuts off part of the target byte sequence; it is undesirable to consume
these bytes from the buffer, as they may or may not be a part of the byte sequence, but `BufReader` will not read more bytes until the buffer is empty.
The only alternative in this situation is to read into a temporary buffer, which violates the DRY (Don't Repeat Yourself) principle, 
in that a buffer is already available but its behavior does not suit the use-case.

# Detailed design

The solution the author proposes is to add two methods, `get_buf()` and `read_into_buf()` to the `BufReader` struct:

```rust
impl<R: Read> BufReader<R> {
    // Existing methods omitted for brevity.
    
    /// Read more data into the buffer without consuming the data in the buffer already.
    /// If successful, returns the number of bytes read.
    ///
    /// The returned byte count may be `0` for one of several reasons, including:
    /// 
    /// * The underlying source returned 0 bytes read.
    /// * The buffer was full.
    pub fn read_into_buf(&mut self) -> io::Result<usize> { /* ... */ }

    /// Get the buffer without modifying it.
    pub fn get_buf(&self) -> &[u8] { /* ... */ }
}
```

Unlike `BufReader`'s impl of `BufRead::fill_buf()`, which only reads into the buffer when it is empty,
`read_into_buf()` unconditionally reads into the buffer every time it is called. Thus, it would allow the user 
to control when `BufReader` reads into its buffer without removing bytes that are already in said buffer. 

Additionally, `get_buf()` would allow the user to access the buffer without calling `BufRead::fill_buf()`, which
might incur an expensive I/O operation if the buffer is empty. In contrast, `get_buf()` would simply return an empty slice.

[A previous RFC](https://github.com/rust-lang/rfcs/pull/1015), of which this RFC might be considered a spiritual successor,
proposed that these methods be added to the `BufRead` trait itself. This has some drawbacks, including, but not limited to,
breakage of downstream implementors because these methods could not have sane default impls in terms of the current API.
Since all changes to stable Rust APIs have to consider backwards-compatibility, this was deemed unacceptable.

Adding these methods as inherent on the `BufReader` struct itself avoids downstream breakage as well as the problem of having to address
the semantics of other `BufRead` impls, many of which don't necessarily treat buffering in the same way `BufReader` does.

# Drawbacks

* This RFC does not cover allowing the user to grow the buffer beyond its original allocation, which may be a desirable feature to accompany this API; buffer resizing can be discussed in its own RFC. For now, the user can set the buffer size with `BufReader::with_capacity()` if the default (64KiB at the time of this writing) isn't optimal for their use-case.

# Alternatives

None

# Unresolved questions

* Should `read_into_buf()` return the new length of the buffer instead of the number of bytes read?

    * This would be more ergonomic for the author's specific use-case but this might not always be the case.
    
    * Depending on the underlying `Read` impl's semantics, it might be useful to do `while try!(buf.read_into_buf()) > 0 {}` to make sure the buffer is filled.

* Should `read_into_buf()` move existing bytes down if there is room at the beginning of the buffer?
    
    * Perhaps *only* if there is more room at the beginning than at the end?

