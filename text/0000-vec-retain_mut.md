- Feature Name: filter_in_place
- Start Date: 2015-11-6
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add `filter_in_place` and `filter_in_place_unordered` methods to `Vec` and
`VecDeque` which allow iterating through all elements mutably while deciding
whether to remove an element from the container.

The existing `retain` method will be deprecated since its functionality is
superseded by the two new methods.

# Motivation
[motivation]: #motivation

A common pattern with vectors is to look at each element, modify it and
remove it from the vector if a certain condition is met. The `retain` method
allows elements to be removed from a vector in-place but does not allow existing
elements to be modified.

The lack of a `retain_mut` method means that users have to do 2 passes, one with
`iter_mut` and one with `retain`. The resulting code is harder to read and
slower. Another, more efficient, way of doing this would be to work with vector
indices directly and swap elements into place while mutating them, similar to
what `retain` already does.

`Vec` and `VecDequeue` also lack a way of filtering elements without preserving
the order of elements in the vector, which can be implemented more efficiently
by simply swaping with the last element when removing from the vector.

# Detailed design
[design]: #detailed-design

Two new methods are added to `Vec` and `VecDequeue`:

```rust
pub fn filter_in_place<F>(&mut self, f: F)
    where F: FnMut(&mut T) -> bool;

pub fn filter_in_place_unordered<F>(&mut self, f: F)
    where F: FnMut(&mut T) -> bool;
```

The `filter_in_place` method is similar to `retain` in that it calls the given
closure for each element in the vector in their original order, and removes the
element from the vector if the closure returns `false`. Unlike `retain` however,
the closure is given a `&mut T` instead of a `&T`, which allows it to modify
elements before deciding whether to remove them from the vector.

The `filter_in_place_unordered` method is similar to `filter_in_place` but does
not preserve the order of elements in the vector. The closure is only guaranteed
to be called once for all elements in the original vector, but not necessarily
in the original order.

The new `retain` method will be deprecated from both `Vec` and `VecDeque` since
it only provides a subset of the functionality of `filter_in_place`.

Here is an example which will decrement each element of a vector and filter out
elements that have reached a value of zero.

```rust
let mut vec = vec![7, 1, 3, 10];
vec.filter_in_place(|x| {
    *x -= 1;
    *x != 0
});
assert_eq!(vec, [6, 2, 9]);
```

# Drawbacks
[drawbacks]: #drawbacks

This deprecates `retain`, which is a stable API.

# Alternatives
[alternatives]: #alternatives

Changing the existing `retain` method to take a `FnMut(&mut T)` was considered
in rust-lang/rust#25477 but this is likely to break a lot of existing code which
passes a closure defined using `|&x| {...}`.

Another alternative is to not do anything. Users can implement their own version
of `filter_in_place` or they can restructure their code into an `iter_mut` pass
on the vector followed by a `retain` pass.

# Unresolved questions
[unresolved]: #unresolved-questions

None
