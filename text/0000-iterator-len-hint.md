- Feature Name: iterator_len_hint
- Start Date: 2015-04-05
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Add a method `len_hint` to the `Iterator` trait as a synonym for `size_hint`, and deprecate `size_hint`.

# Motivation

Currently, methods returning the numbers of elements in containers are conventionally named `len` in Rust, even for non-linear collections like [`BTreeMap`](http://doc.rust-lang.org/std/collections/btree_map/struct.BTreeMap.html#method.len). But the `Iterator` trait, being conceptually linear, has a `size_hint` method, instead of the more consistent `len_hint`.

The standard library documentation says about `size_hint`:

> Returns a lower and upper bound on the remaining *length* of the iterator.

This method should be renamed.

# Detailed design

A method `len_hint` would be added to the `std::iter::Iterator` trait, which has the following inlined default implementation:

```rust
fn len_hint(&self) -> (usize, Option<usize>) {
    self.size_hint();
}
```

The `size_hint` method would be marked `deprecated` now.

In Rust 2.x, `len_hint` would become a required method and `size_hint` would be removed.

# Drawbacks

Rust 1.0 final is yet to be released, having something deprecated now but not removed until 2.x is a bit weird.

# Alternatives

#### A. Rename `size_hint` to `len_hint` during the Rust 1.0 beta cycle.

Instead of removing `size_hint` in Rust 2.x, remove it before 1.0 final.

The disadvantage: this is a late breaking change (though only a minor naming correction at that).

The advantage: it is clearer than having `len_hint` as a synonym for `size_hint` in Rust 1.x.

#### B. Keep the status quo.

`size_hint` is not too bad after all, though it is inconsistent, especially when even non-linear collections use methods named `len` instead of `size`.

# Unresolved questions

None.
