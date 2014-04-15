- Start Date: 2014-04-15
- RFC PR #:
- Rust Issue #:

# Summary

Add a method the `std::io::Writer` trait to inform writers
of an estimate of how many bytes we’re about to write "soon"
(though possibly in mutliple `write*` method calls).
Implementations can use this as they see fit,
including doing nothing (the default).

# Motivation

In cases where the user of a writer makes a number of calls to `write*` methods,
they be be able to estimate beforehand the total number of bytes.
Writers that need to "allocate space" somehow before they can write to it
could use that knowledge to pre-allocate a big chunck of space
rather than starting small and keep re-allocating as needed.

[rust-encoding](https://github.com/lifthrasiir/rust-encoding) is such a case,
with estimates based on the size of the input and encoding-specific knowledge.
At the moment, rust-encoding defines a custom `ByteWriter` trait
in order to support this.
Using libstd’s `Writer` instead would improve composability with other libraries.

# Detailed design

In the `std::io::Writer` trait, add:

```rust
    fn reserve_additional(n: uint) {}
```

Like `flush()`, this method defaults to a no-op
but may be overriden by implementations.

In the `MemWriter` implementation, add:

```rust
    fn reserve_additional(n: uint) {
        self.buf.reserve_additional(n)
    }
```

Usage example:

```rust
fn write_chars<I: Iterator<char>, W: Writer>(iter: I, output: W) -> IoError(()) {
    let chars_low, chars_high = iter.size_hint();

    // XXX Should this be used somehow?
    let bytes_high = chars_high.map(|h| h * 4)  // Only non-BMP code points

    let bytes_low = chars_low;  // Only ASCII code points
    output.reserve_additional(bytes_low);
    for c in iter {
        try!(output.write_char(c))
    }
}
```

# Alternatives

* Do nothing, if the allocation optimization is judged not worth the API complixity.
* Rather than a single number, (optinally?) provide a range for the estimate.
  See `Iterator::size_hint`.

# Unresolved questions

* `BufferedWriter` probably should also override this new method,
  but exact desired behavior is not obvious.
* If `reserve_additional` takes a single number
  but a user can estimate within a range,
  should they reserve the lower bound, upper bound, or something else?
  (`Vec::from_iter` currently only looks at the lower bound from `Iterator::size_hint`.)
