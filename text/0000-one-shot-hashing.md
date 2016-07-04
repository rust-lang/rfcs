- Feature Name: one_shot_hashing
- Start Date: 2016-07-04
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Extend the `Hasher` trait with a `fn delimit` method.

# Motivation
[motivation]: #motivation

The current hashing architecture is suitable for streaming hashing.

In general, for each type which implements Hasher, there cannot be two values
that produce the same stream. Delimiters are inserted so that values of
compound types produce unique streams. For example, hashing `("ab", "c")` and
`("a", "bc")` must produce different results.

Hashing in one shot is possible even today with a custom hasher for constant-
sized types. However, HashMap keys are often strings and slices. In order to
allow fast, specialized hashing for variable-length types, we need a clean way
of handling single writes. Hashing of strings and slices performs two writes
to a stream: one for a delimiter and the other for the content. We need a way
of conveying the distinction between the delimiter and actual content. In the
case of one-shot hashing, the delimiter can be ignored.

# Detailed design
[design]: #detailed-design

The functionality of streaming hashers remains the same.

A `delimit` method with default implementation is added to the `Hasher` trait as
follows.

```rust
trait Hasher {
    // ...

    /// Emit a delimiter for an array of length `len`.
    #[inline]
    #[unstable(feature = "hash_delimit", since = "...", issue="...")]
    fn delimit(&mut self, len: usize) {
        self.write_usize(len);
    }
}
```

# Drawbacks
[drawbacks]: #drawbacks

* The `Hasher` trait becomes larger.

# Alternatives
[alternatives]: #alternatives

* Leaving out this, which means adaptive hashing may not work for
  string and slice types.
* Changing SipHash to ignore the first delimiter.

# Unresolved questions
[unresolved]: #unresolved-questions

* Should `str` and `[u8]` get hashed the same way?
* Can streaming hashers such as SipHash ignore the first or the last delimiter?
