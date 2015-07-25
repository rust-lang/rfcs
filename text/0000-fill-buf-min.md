- Feature Name: fill-buf-min
- Start Date: 7-25-2015
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Add a new method to the `BufRead` trait, `BufRead::fill_buf_min()`, which would allow users to express 
to the buffer implementation that they need *at least* X bytes in the buffer.

# Motivation

The current API of `BufRead` does not give the user any control over *when* the buffer should be read into,
at least not without consuming bytes from the buffer.

This is evident with the reference implementation, the `BufReader` struct. `BufReader`'s implementation of `BufRead::fill_buf()` 
will only perform a read from the underlying source if the buffer is *empty*. This is problematic for users who want to work with
the data in the buffer but may need to pull in more data without emptying the buffer.

The author's use-case consists of a struct that wraps `BufReader` and uses it to search streaming HTTP request bodies for specific byte sequences.
The problem arises when a partial read from the underlying stream cuts off part of the target byte sequence; it is undesirable to consume
these bytes from the buffer, as they may or may not be a part of the byte sequence, but `BufReader` will not read more bytes until the buffer is empty.
The only alternative in this situation is to read into a temporary buffer, which violates the DRY (Don't Repeat Yourself) principle, 
in that a buffer is already available but its behavior does not suit the use-case.

# Detailed design

The solution the author proposes is to add a new method to the `BufRead` trait, `fill_buf_min()`. The proposed signature is as follows:

```rust
pub trait BufRead: Read {
    // Existing methods omitted for brevity.
    
    fn fill_buf_min(min: usize) -> io::Result<&[u8]>;
}
```

The semantics are similar to the existing `fill_buf()` method, with one exception: if the length of the buffer is less than `min`,
the implementation should perform another bulk read from the source. 

However, the implementation is not *required* to meet this minimum,
because it may not be able to for some reason, e.g. because the stream is at EOF or because it would take several expensive reads to do so. Thus,
the user should check to make sure the returned buffer is of the desired minimum length; if not, they may try another read by calling `fill_buf_min()`
again.



# Drawbacks

It would be another required method on `BufRead`, which would break downstream implementors that already fulfill the existing API contract.

Additionally, the author has not extensively considered what the semantics should be across all `BufRead` implementations; some may not be
able to fulfill the new requirements. This RFC primarily concerns `BufReader`'s impl, so it may not be suitable to make all others conform
to one impl's semantics.

This is a spiritual successor to [a previous RFC](https://github.com/rust-lang/rfcs/pull/1015) which proposed similar additions. However,
that RFC was closed because the new methods would break backwards compatibility.

All of these concerns are addressed in the next section.

# Alternatives

* Provide a default implementation of `BufRead::fill_buf_min()` that ignores its argument and simply calls `fill_buf()`. Downstream implementors
would then be able to override it at their leisure. Since `fill_buf_min()` explicitly *does not* guarantee the minimum lengh of the returned
buffer, this would not be a breach of the API contract.

* Implement `fill_buf_min()` as an inherent method on `BufReader`. This avoids breaking downstream implementors altogether, and also doesn't
require addressing the buffering semantics of other `BufRead` implementations. If it turns out this behavior is desired across all `BufRead` implementations,
the method can be moved to the trait relatively seamlessly, since almost all uses of `BufReader` are for its `BufRead` impl<sup>[citation needed]</sup>. The transition can be eased
by employing the previous alternative as well.

    * A preliminary implementation of this approach is available [here](https://github.com/cybergeek94/multipart/blob/boundary_fix/src/server/buf_read.rs#L34).

# Unresolved questions

* Is `fill_buf_min()` a descriptive enough method name?

* Should the implementation attempt more reads if it fails to meet the minimum in one? 
    * The author's guess is "no", because the user can simply call the method again to force another read. However, the author also believes this merits discussion.

* Is this behavior desirable in enough use cases to justify its addition?
