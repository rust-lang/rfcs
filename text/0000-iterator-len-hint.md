- Feature Name: iterator_len_hint
- Start Date: 2015-04-05
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Deprecate `Iterator::size_hint` for `Iterator::len_hint`, and `ExactSizeIterator` for `ExactLengthIterator`, gaining consistency with the standard collections' naming convention.

# Motivation

Currently, methods returning the numbers of elements in containers are conventionally named `len` in Rust, even for non-linear collections like [`BTreeMap`](http://doc.rust-lang.org/1.0.0-beta/std/collections/btree_map/struct.BTreeMap.html#method.len). But the `Iterator` trait, which represents entities each yielding a sequence of elements, has a `size_hint` method, instead of the more consistent `len_hint`.

The standard library documentation says about [`size_hint`](http://doc.rust-lang.org/1.0.0-beta/std/iter/trait.Iterator.html#tymethod.size_hint):

> Returns a lower and upper bound on the remaining *length* of the iterator.

Additionally, there is an [`ExactSizeIterator`](http://doc.rust-lang.org/1.0.0-beta/std/iter/trait.ExactSizeIterator.html) trait that also has an inconsistent name (but the sole method of `ExactSizeIterator` is named `len`, which is consistent).

So, some names should be changed. However, Rust 1.0 beta has already been released, which means deprecation should be favoured over direct renaming.

# Detailed design

1. Add `core::iter::Iterator::len_hint`, a method with an inlined default implementation simply calling `self.size_hint()`.
2. Add `core::iter::ExactLengthIterator`, a re-export of `core::iter::ExactSizeIterator`.
3. Deprecate `core::iter::Iterator::size_hint` and `core::iter::ExactSizeIterator`.
4. Adjust the `std` re-exports accordingly.
5. Deprecate the implementations of `size_hint`.

Later, in Rust 2.x series, remove `size_hint` and `ExactSizeIterator`.

This design is fully backwards compatible with Rust 1.0.0-beta.

# Drawbacks

Having deprecated items in the first stable release of Rust is a bit weird. However, this is only a minor drawback.

# Alternatives

#### A. Directly rename `size_hint` to `len_hint`, and `ExactSizeIterator` to `ExactLengthIterator` during the 1.0 beta cycle.

This alternative is clearer than the main candidate.

However, this means late breaking changes, which are generally undesirable. If other breaking changes happen during the 1.0 beta cycle, then this alternative can "piggyback" on those changes.

#### B. Keep the status quo.

Other than "not introducing deprecations now", the status quo doesn't have any advantage over the main candidate.

# Unresolved questions

None.
