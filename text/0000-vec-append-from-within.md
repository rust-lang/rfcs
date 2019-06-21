- Feature Name: `vec_append_from_within`
- Start Date: 2019-06-21
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Provide a safe and efficient way append some of elements of a vector to itself. This is similar to the recently stabilized [`slice::copy_within()`](https://doc.rust-lang.org/std/primitive.slice.html#method.copy_within), but appends to the vector instead of copying to a location within it.

# Motivation
[motivation]: #motivation

Copying parts of a data stream to its end is an essential operation in decompressors and multimedia decoders - an area that could hugely benefit from Rust's performance and safety guarantees. Even though it requires just a little unsafe code, in practice people are struggling to implement it correctly. Motivating examples:

 * Relevant code in `inflate` crate was vulnerable (memory disclosure), [details here](https://www.reddit.com/r/rust/comments/8zpp5f/).
 * A vulnerability in such code in `libflate` is currently pending disclosure, see https://github.com/sile/libflate/issues/33.
 * **Rust standard library itself** had a buffer overflow bug in exactly this code. This is known as CVE-2018-1000810, [details here](https://blog.rust-lang.org/2018/09/21/Security-advisory-for-std.html).

This proposal is an attempt to provide the minimum viable building block that will allow safe and efficient implementations of RLE and similar predictive algorithms.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

```rust
assert_eq!(vec![3,5,7].append_from_within((..1)), vec![3,5,7,3]);
assert_eq!(vec![3,5,7].append_from_within((1..)), vec![3,5,7,5,7]);
assert_eq!(vec![3,5,7].append_from_within((..)), vec![3,5,7,3,5,7]);
vec![3,5,7].append_from_within((..1000)); // panic!
```

This is similar to the recently stabilized [`slice::copy_within()`](https://doc.rust-lang.org/std/primitive.slice.html#method.copy_within), but appends to the vector instead of copying to a location within it.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

```rust
append_from_within<R>(&mut self, src: R) where
    R: RangeBounds<usize>,
    T: Copy
```

This function appends copies of elements within the specified range to the end of the vector. Copying is done using `ptr::copy_nonoverlapping()`. Capacity of the vector will be increased to accommodate at least `src.len()` elements.

Specifying a range with `start > end` or `end > vec.len()` results in a panic. Specifying a range with `start = end` is a no-op. This matches behavior of `slice::copy_within()`.

[Prototype implementation in playground](https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=016ea345d36d1091e0925c320a1b99c9)

# Drawbacks
[drawbacks]: #drawbacks

 - This introduces yet another function on Vec data type.
 - It doesn't achieve highest possible performance in RLE decoding, since it will do capacity checks for every loop iteration (at least in the prototype implementation they're not elided by the compiler). However, for an exponential copying algorithm the cost should be negligible.
 - It could be made largely obsolete in the future by a hypothetical more general abstraction.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Alternatives:

- Do nothing. We will keep seeing vulnerabilities in real-world code.
- Provide such a function in an external crate. This will severely hurt discoverability, so people will probably keep hand-rolling this and getting it wrong. Also, one of the motivating vulnerabilities was in the standard library itself.
- Provide a function `Vec::repeat_part(&mut self, src: Range, times: usize)`, which would serve the described use case with `times` set to 1. For repeating more than once this would allow eliding capacity checks and bounds checks on every loop iteration, and that's exactly the desired behavior for RLE decoders. However, it's not clear whether this behavior is common enough to warrant inclusion in the stdlib.
- Provide a method similar to `slice::split_at_mut()` that splits a `Vec` into `(&mut [T], FixedCapacityVecView<T>)`. This makes it possible to append to the vector (up to its current capacity) while some of its elements are still mutably borrowed.
   -  This allows for more freedom in implementation of appending: this way you can use `right.extend()` on a chain of iterators over `left`.
   - The main downside is an increased entry barrier and cognitive load due to yet another stdlib entity with lots of methods on it.
   - A prototype of this [exists as a crate](https://github.com/WanzenBug/rust-fixed-capacity-vec).
   - We've learned from the prototype that an implementation relying on public interfaces of Vec cannot achieve performance on par with Vec: safety of this hinges on never triggering reallocation of the vector, and the reallocation behavior for Vec is defined rather loosely, so we had to implement a lot of additional checks. The only feasible way to implement this is within the stdlib, and introducing an entire new Vec-like entity in the stdlib for a fairly limited set of use case doesn't seem to be worth it.
   - A more in-depth discussion of this approach with more motivating examples and potential use cases can be found in the [Pre-RFC](https://internals.rust-lang.org/t/pre-rfc-fixed-capacity-view-of-vec/).
- Provide data type that is backed by a fixed-size memory, region like a slice, but keeps track of which parts of it are initialized, like a Vec.
   - This currently exists as a crate called [buffer](https://github.com/tbu-/buffer) which supports viewing `&[u8]`, `Vec<u8>` and `ArrayVec<u8>` this way. It supports a minimal set of expansion routines such as `.extend()` and appending to the end while a part of the initialized region is borrowed.
   - In the form of a crate it does not solve the motivating examples above because they have just one unsafe block with append-vec-to-itself code each, and taking on a large dependency with lots of unsafety to get rid of one unsafe block of your own is not worth the trouble. If uplifted into the stdlib, it could solve the motivating examples above as well as other motivating examples from the [Pre-RFC](https://internals.rust-lang.org/t/pre-rfc-fixed-capacity-view-of-vec/8413/20), and perhaps even solve the [long-standing issue](https://github.com/mozilla/mp4parse-rust/issues/172) with `Read` requring the output buffer to be initialized, which hurts performance.
   - The downside is yet another large entity in the stdlib. The implementation burden will probably be pretty large with many of the methods from `Vec` needing to be duplicated/reimplemented.
   - Even if this is implemented, it will not make `Vec::append_from_within()` entirely obsolete because it has the advantage of simplicity - it would not require the API user to grok yet another data type and perhaps a less obvious solution to the same problem.

# Prior art
[prior-art]: #prior-art

In Rust:
 - [rust-fixed-capacity-vec](https://github.com/WanzenBug/rust-fixed-capacity-vec) prototype, discussed in "Rationale and alternatives" above
 - [buffer](https://github.com/tbu-/buffer) crate, discussed in "Rationale and alternatives" above

In C `memcpy()` serves this purpose. It is, naturally, unsafe.

In Python the typical `.extend` method works even when passing a subslice of the same list because subslicing implicitly creates a copy, so there is no need for a separate function. Here is one of the usage examples of `.append_from_within()` adapted to Python:
```python3
>>> arr = [3,5,7]
>>> arr.extend(arr[1:])
>>> arr
[3,5,7,5,7]
```

# Unresolved questions
[unresolved-questions]: #unresolved-questions

 - Is it possible to elide capacity checks when calling this function in a loop if the capacity is preallocated?
 - Is the `Vec::repeat_part(&mut self, src: Range, times: usize)` alternative a common enough task to warrant inclusion instead of `.append_from_within(&mut self, src: Range)`?
 - That last alternative similar to `buffer` crate looks attractive, but would take a long time to design and stabilize. Should we stabilize `.append_from_within()` now even though a hypothetical stdlib equivalent of `buffer` would serve this use case eventually, although in a slightly less obvious way?

# Future possibilities
[future-possibilities]: #future-possibilities

A hypothetical stdlib equivalent of `buffer` crate discussed in "Rationale and alternatives" could solve this issue and many more that are out of scope of this proposal.
