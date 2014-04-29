- Start Date: 2014-04-15
- RFC PR #:
- Rust Issue #:

# Summary

Add a `size_hint` method to `Reader` that
returns an estimated range of how many bytes are remaining to be read,
similar to the existing `size_hint` method on iterators.
Add a `reserve` method to `Writer` that
takes an estimated range of how many bytes will “soon” be written.


# Motivation

Just like `Iterator::size_hint` allows e.g. `Vec::from_iter`
to pre-allocate a big chunk of memory rather than keep reallocating as needed,
this would help reader users and writer implementations that can similarly
pre-allocate space.

For writers, the caller may have information about amount of data being processed
across many calls to `write*` methods.
For example, [rust-encoding](https://github.com/lifthrasiir/rust-encoding)
processes one code point at a time (calling `write_char` repeatedly),
but can estimate based on the size of the input and encoding-specific knowledge.

At the moment, rust-encoding defines a custom `ByteWriter` trait
in order to support this.
Using libstd’s `Writer` instead would improve composability with other libraries.

Simplified usage example:

```rust
fn write_chars<I: Iterator<char>, W: Writer>(iter: I, output: W) -> IoError(()) {
    let (chars_low, chars_high) = iter.size_hint();
    let bytes_low = chars_low;  // Only ASCII code points
    let bytes_high = chars_high.map(|h| h * 4)  // Only non-BMP code points
    output.reserve(bytes_low, bytes_high);
    for c in iter {
        try!(output.write_char(c))
    }
    Ok(())
}
```

# Detailed design

The `std::io::Reader` trait gets a new default method:

```rust
    /// Return a lower bound and upper bound on the estimated
    /// remaining number of bytes until EOF.
    ///
    /// Note: This estimate may be wrong.
    /// There is no guarantee that EOF will actually be reach within this range.
    ///
    /// The common use case for the estimate is pre-allocating space to store the results.
    #[inline]
    fn size_hint(&self) -> (uint, Option<uint>) { (0, None) }
```

This is identical to the `std::iter::Iterator<u8>::size_hint` method.

The `Reader::read_to_end` default method is updated
to pre-allocate the new vector’s capacity based on `self.size_hint()`.

`size_hint` is overriden as appropriate in libstd implementors.
For example, in `MemReader`:

```rust
    #[inline]
    fn size_hint(&self) -> (uint, Option<uint>) {
        let exact = self.buf.len() - self.pos;
        (exact, Some(exact))
    }
```

The `std::io::Writer` trait gets a new default method:

```rust
    /// Inform the writer that of the lower bound and upper bound
    /// on the estimated number of bytes that will be written “soon”
    /// (though possibly in multiple `write*` method calls).
    /// Return a lower bound and upper bound on the remaining number of bytes until EOF.
    ///
    /// Note: this estimate may be wrong.
    /// It is valid to write a number of bytes outside the given range.
    ///
    /// Implementations can use this information as they see fit,
    /// including doing nothing (the default).
    /// The common use case for the estimate is pre-allocating space to store the results.
    #[inline]
    fn reserve(&self, _low: uint, _high: Option<uint>) {}
```

Like `flush()`, this method defaults to a no-op
but is meant be overridden by implementations.

Override `size_hint` as appropriate in libstd implementors.
For example, in `MemWriter` (modeled after `Vec::from_iter`):

```rust
    #[inline]
    fn size_hint(&self, low: uint, _high: Option<uint>) {
        self.buf.reserve_additional(low)
    }
```

`std::io::fs::File::size_hint` could use
[`fallocate`](http://man7.org/linux/man-pages/man2/fallocate.2.html)
with `FALLOC_FL_KEEP_SIZE` on Linux,
or equivalent on other systems.
(This is supposed to be only a hint,
we probably don’t want to change the apparent size of the file.)

# Alternatives

* Do nothing, if the allocation optimization is judged not worth the API complexity.
* A previous version of this RFC used a single integer as an "estimate" instead
  of the current lower bound and optional upper bound (like `Iterator::size_hint`).
* A draft of this version used the `size_hint` name for both readers and writers,
  but that would have prevented anything to implement both traits, like `File` does.
* Define these methods on new, special purpose traits.
  This is only practical if we also have specialization: mozilla/rust#7059

# Unresolved questions

* It’s unclear whether or how `BufferedWriter` should override `size_hint`.
* Should `File::size_hint` really call `fallocate`,
  or is that better left to a more explicitly-name API?
