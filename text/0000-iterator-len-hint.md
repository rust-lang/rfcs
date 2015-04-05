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
