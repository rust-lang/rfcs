- Feature Name: iterator_len_hint
- Start Date: 2015-04-05
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Rename the `size_hint` method of the `Iterator` trait to `len_hint`.

# Motivation

Currently, methods returning the numbers of elements in containers are conventionally named `len` in Rust, even for non-linear collections like [`BTreeMap`](http://doc.rust-lang.org/1.0.0-beta/std/collections/btree_map/struct.BTreeMap.html#method.len). But the `Iterator` trait, which represents entities each yielding a sequence of elements, has a `size_hint` method, instead of the more consistent `len_hint`.

The standard library documentation says about [`size_hint`](http://doc.rust-lang.org/1.0.0-beta/std/iter/trait.Iterator.html#tymethod.size_hint):

> Returns a lower and upper bound on the remaining *length* of the iterator.

This method should be renamed.

# Detailed design

Rename the `size_hint` method of `std::iter::Iterator` to `len_hint` during the Rust 1.0 beta cycle.

A new, inlined, but deprecated `size_hint` method may be added temporarily to ease the transition. That new `size_hint` would be implemented as:

```rust
fn size_hint(&self) -> (usize, Option<usize>) {
    self.len_hint()
}
```

Depending on the scale of impact of breaking changes planned during the beta cycle, it may or may not be desirable to release a new beta, reflecting the new breaking changes, so that more people can provide feedbacks about the changes before they get set in the stone.

Ideally, this change should be completed just before a release (either the new beta, if it is decided to be released, or the final, if no new beta gets released). Also, the transition period should be as short as possible. 

The reason is that: If this change happens too early, library authors (that are affected by this change) would have to maintain at least two separate branches of their libraries, one for the old beta, the other for nighties, if they want the widest possible audience. On the other hand, if the change starts and finishes just before a new release, library authors could abandon the old beta immediately and focus on the new release and later nighties.

# Drawbacks

This is a late breaking change (though only a minor naming correction at that).

# Alternatives

#### A. Add `len_hint` as a synonym for `size_hint`, and deprecate `size_hint`.

A `len_hint` method would be added to the `std::iter::Iterator` trait, which has the following inlined default implementation:

```rust
fn len_hint(&self) -> (usize, Option<usize>) {
    self.size_hint()
}
```

The `size_hint` method would be marked `deprecated` now.

In Rust 2.x, `len_hint` would become a required method and `size_hint` would be removed.

The advantage of this alternative is that it is not a breaking change.

However, Rust 1.0 final is yet to be released, having something deprecated now but not removed until 2.x is weird.

#### B. Keep the status quo.

`size_hint` is not too bad after all, though it is inconsistent, especially when even non-linear collections use methods named `len` instead of `size`.

# Unresolved questions

None.
