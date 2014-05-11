- Start Date: 2014-05-11
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Iterator trait allows infinite sequences of values, but some of its functions
clearly don't make sense for infinite sequences (e.g. last, len).
These functions should be moved to a separate trait.

# Motivation

Makes it impossible to write incorrect code.
Currently it's possible to write and compile the following code:

```rust
let s = Repeat::new('z').len();
```

It obviously introduces an infinite loop. It shouldn't be that easy to use iterators
incorrectly. Even Rust source code contains complains about current Iterator trait.
Example from trait implementation for Counter structure:

```rust
fn size_hint(&self) -> (uint, Option<uint>) {
    (uint::MAX, None) // Too bad we can't specify an infinite lower bound
}
```

# Drawbacks

Yet another \*Iterator trait to implement.
But it doesn't introduce new functions, only shuffles existing ones.

# Detailed design

There is a bunch of functions in Iterator trait that don't work well on infinite sequences.

1. Functions that always introduce an infinite loop:
`collect`, `last`, `fold`, `len`, `count`, `max_by`, `min_by`.
2. Functions that may return even for an infinite sequence:
`advance`, `all`, `any`, `find`, `position`.
3. Other functions that make little sense for an infinite iterator: `size_hint`.

Move at least 1. and 3. functions from current Iterator trait into new `FiniteIterator` trait:

```rust
trait<A> FiniteIterator<A> : Iterator<A> {
    fn size_hint(&self) -> (uint, Option<uint>);
    fn collect<B: FromIterator<A>>(&mut self) -> B;
    fn last(&mut self) -> Option<A>;
    fn fold<B>(&mut self, init: B, f: |B, A| -> B) -> B;
    fn len(&mut self) -> uint;
    fn count(&mut self, predicate: |A| -> bool) -> uint;
    fn max_by<B: TotalOrd>(&mut self, f: |&A| -> B) -> Option<A>;
    fn min_by<B: TotalOrd>(&mut self, f: |&A| -> B) -> Option<A>;
}
```

Default implementations, here omitted for readability, are the same as in current Iterator trait.
This way `Iterator` trait doesn't assume anything about finiteness of generated sequence of
values and `FiniteIterator` assumes that `next` will eventually return `None`.

Meaning of `size_hint` changes slightly. `None` for upper bound could mean
*"not possible to represent by uint"* instead of *"there is no upper bound"*.
Iterator trait implementations and trait bounds that include Iterator should be modified
accordingly. Trait should *not* be implemented for types representing endless
iterator, such as Counter or Repeat.

There is at least one problematic edge case. `Cycle` iterator is infinite unless original,
internal iterator is empty, in which case `Cycle` is also empty and therefore finite.
That's very specific and uninteresting case so I think `Cycle` should simply be considered
infinite (i.e. it should not implement `FiniteIterator`).

# Alternatives

Not doing this would mean that it's possible to write generic code that uses Iterator trait bound
and works perfectly fine for some types and generates infinite loop for others.

# Unresolved questions

* Whether functions from group 2. are appropriate for use on infinite sequences.
* Consider better wording.
