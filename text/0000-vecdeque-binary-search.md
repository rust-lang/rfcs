- Feature Name: TBD
- Start Date: 2020-09-30
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Implement binary search functions for `VecDeque`:
 - `binary_search()`
 - `binary_search_by()`
 - `binary_search_by_key()`

# Motivation
[motivation]: #motivation

These functions are already implemented for slice and, by virtue of `Defer`-to-slice, for `Vec` as well.
`VecDeque` is a linear storage with `Index` implementation much like `Vec`,
having these functions would make just as much sense as it does for `Vec` and slice.

My use-case is using `VecDeque` for in-memory time-series-like data store where new data are added
in the front as time progresses and old data are added in the back from persistent storage
when requested for reading. I found the binary search fns useful for looking up data (or empty slots)
based on a timestamp.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The `binary_search()` function (as well as `binary_search_by()` and `binary_search_by_key()`) may be
used when a `VecDeque` contains sorted elements to quickly (in logarithmic time) locate an element in the deque.

These functions return an `Ok(index)` if the deque contains the element we were looking for,
or an `Err(index)` if the deque doesn't have that element, in which case the `index` is a position
where such an element may be inserted such that the deque remains sorted.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Internally, the implementation would use the `as_slices()` function,
first comparing against the first element of the 'back' slice and then:
- In case of `Greater`, delegate to the 'front' slice
- In case of `Equal`, return the index of this element
- In case of `Less`, delegate to the rest of the 'back' slice

# Drawbacks
[drawbacks]: #drawbacks

- New functions on a std collection: Some backcompat hazard, maintenance burden.
- Maybe a more generic solution would be preferable instead, see the next paragraph.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The benefit of this design is that it's very simple and just follows what's
already established for slice and `Vec`.

On the other hand, a more generic solution might be better from a long term
point of view. For example, C++ standard library has [`random_access_iterator`](https://en.cppreference.com/w/cpp/iterator/random_access_iterator)
and [`std::binary_search`](https://en.cppreference.com/w/cpp/algorithm/binary_search).

Rust used to have [`RandomAccessIterator`](https://doc.rust-lang.org/1.0.0/std/iter/trait.RandomAccessIterator.html),
but it was effectively split into `Index`/`IndexMut` and `ExactSizeIterator`.
Perhaps a trait representing binary search ops could be added instead
and default-implemented for anything `Index` + `ExactSizeIterator`
as a more generic solution.

# Prior art
[prior-art]: #prior-art

I don't know of there being a prior proposal for binary search for `VecDeque`
or the former `RandomAccessIterator`.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None other than the one in [Rationale and alternatives](#rationale-and-alternatives).

# Future possibilities
[future-possibilities]: #future-possibilities

I can't think of anything.
