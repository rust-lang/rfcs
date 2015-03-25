- Feature Name: bufread_buf
- Start Date: 2015-03-25
- RFC PR:
- Rust Issue:

# Summary

Add two methods to BufRead, `read_into_buf` and `get_buf`. These would allow filling
up more of the buffer, and inspecting the buffered data without causing an implicit
read.

# Motivation

Currently, you can only view the buffer via `fill_buf`. Once the buffer has even a
single byte in it, `fill_buf` no longer fills up more of the buffer. It also causes
a read  implicitly if the buffer is empty, even if you only wanted to inspect the
buffer.

A use case is that of wanting to read more and more into the buffer without consuming
the read data. This could be because one requires a certain amount of bytes to be able
to determine what kind of input was received, and until that is determined, one doesn't
want to throw away the bytes.

A possible example:

```rust
loop {
    match try!(rdr.read_into_buf()) {
        0 => return Err(Incomplete),
        _ => {
            if is_foo(rdr.get_buf()) {
                rdr.consume(FOO_BYTES);
                return Ok(Foo)
            }
        }
    }
}

```

# Detailed design

Add the following methods to `BufRead`:

```rust
trait BufRead: Read {
    // ...

    /// This will read from the inner Read, and append the data onto the end of
    /// any currently buffered data.
    fn read_into_buf(&mut self) -> io::Result<usize>;

    /// Get a read-only view into the buffered data.
    ///
    /// NOTE: This method **will not** do an implicit read if the buffer is empty.
    /// It will just return an empty slice.
    fn get_buf(&self) -> &[u8];

}

```

# Drawbacks

Adds additional methods that a `BufRead` implementation must include.

# Alternatives

An alternative to this is to not add the `read_into_buf` method, and instead to
change `fill_buf` to act just like it. This may be more intuitive, or not, depending
on your assumptions.

# Unresolved questions

