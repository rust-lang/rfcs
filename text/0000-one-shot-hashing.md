- Feature Name: one_shot_hashing
- Start Date: 2016-07-04
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Extend the `Hasher` trait with a `fn delimit` method.

# Motivation
[motivation]: #motivation

Streaming hashing is a way of hashing values of any type. Every significant
byte of the hashed value is included in a stream. The entire stream is hashed.
One-shot hashing is a simplification of streaming hashing. It is limited to a
single primitive value.

The current hashing architecture is used for streaming hashing. However, it is
unfit for optimal one-shot hashing. Consider the following interface for
one-shot hashing, based on Farmhash.

```rust
extern crate farmhash;

struct FarmHasher {
    result: u64
}

impl Hasher for FarmHasher {
    fn write(&mut self, msg: &[u8]) {
        self.result = farmhash::hash64(msg);
    }
    
    fn finish(&self) -> u64 {
        self.result
    }
}
```

This `FarmHasher` will work for constant-sized primitive types. That is:
integers, raw pointers, and `char`. It will give wrong results when hashing
`&str`, and may do unnecessary work when hashing `&[T]`. Why doesn't it work
for variable-sized types?

In general, for each type which implements Hasher, there cannot be two values
that produce the same stream. For example, hashing `("ab", "c")` and `("a",
"bc")` must produce different results. To ensure that, a special value is
inserted in the stream after the contents of every string. One-shot hashing
should be able to ignore such delimiters, because compound types can't even
be hashed in one shot.

# Detailed design
[design]: #detailed-design

A `delimit` method with default implementation is added to the `Hasher` trait as
follows.

```rust
trait Hasher {
    // ...

    /// Emit a delimiter.
    #[inline]
    #[unstable(feature = "hash_delimit", since = "...", issue="...")]
    fn delimit<T: Hash>(&mut self, delimiter: T) {
        delimiter.hash(self);
    }
}
```

Implementations of `Hash` for `str` and `[T]` are changed as follows.

```rust
impl Hash for str {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(self.as_bytes());
        state.delimit(0xff_u8);
    }
}

impl<T: Hash> Hash for [T] {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.delimit(self.len());
        Hash::hash_slice(self, state)
    }
}
```

The functionality of streaming hashers remains the same. One-shot hashing is
not yet in the standard library.

# Drawbacks
[drawbacks]: #drawbacks

* The `Hasher` trait becomes larger.

# Alternatives
[alternatives]: #alternatives

* Leaving out this, which means adaptive hashing may not work for
  string and slice types.
* Changing SipHash to ignore the first or the last delimiter.

# Unresolved questions
[unresolved]: #unresolved-questions

* Should `str` and `[u8]` get hashed the same way?
* Can streaming hashers such as SipHash ignore the first or the last delimiter?
