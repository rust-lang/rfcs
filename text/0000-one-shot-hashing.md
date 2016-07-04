- Feature Name: one_shot_hashing
- Start Date: 2016-07-04
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Extend the `Hasher` trait with a `fn delimit` method. Add an unstable Farmhash
implementation to the standard library.

# Motivation
[motivation]: #motivation

The current hashing architecture is suitable for streaming hashing.

In general, for each type which implements Hasher, there cannot be two values
that produce the same stream. Delimiters are inserted so that values of
compound types produce unique streams. For example, hashing `("ab", "c")` and
`("a", "bc")` must produce different results.

Hashing in one shot is possible even today with a custom hasher for constant-
sized types. However, HashMap keys are often strings and slices. In order to
allow fast, specialized hashing for more types, we need a clean way of
handling single writes. Hashing of strings and slices performs two writes to a
stream: one for a delimiter and the other for the content. We need a way of
conveying the distinction between the delimiter and actual content. In the
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

Farmhash is introduced as an unstable struct at `core::hash::FarmHasher`. It
should not be exposed in to users of stable Rust.

It may be implemented in the standard library as follows.

```rust
struct FarmHasher {
    hash: u64
}

impl Hasher for FarmHasher {
    fn write(&mut self, input: &[u8]) {
        self.hash = farmhash::hash64(input);
    }

    fn delimit(&mut self, _len: usize) {
        // Nothing to do.
    }

    fn finish(&mut self) -> u64 {
        self.hash
    }
}
```

# Drawbacks
[drawbacks]: #drawbacks

* There will be yet another hashing algorithm to maintain in the standard library.
* The `Hasher` trait becomes larger.

# Alternatives
[alternatives]: #alternatives

* Leaving out either or both of these. This means adaptive hashing won't work for
  string and slice types.
* Introducing Farmhash as an unstable function.
* Adding the `fn delimit` method, but leaving out Farmhash.
* Using MetroHash or some other algorithm instead of Farmhash.
* Changing SipHash to ignore the first delimiter.

# Unresolved questions
[unresolved]: #unresolved-questions

* Should `str` and `[u8]` get hashed the same way?
* Can streaming hashers such as SipHash ignore the first or the last delimiter?
