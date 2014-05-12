- Start Date: 2014-05-11
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Iterator trait allows infinite sequences of values, but some of its functions
clearly don't make sense for sequences that are not finite (e.g. last, len).
These functions should be moved to a separate trait.

# Motivation

Currently it's possible to write and compile the following code:

```rust
let s = Repeat::new('z').len();
```

It obviously introduces an infinite loop. It shouldn't be that easy to use iterators incorrectly.

# Drawbacks

Yet another \*Iterator trait to implement.
But it doesn't introduce new functions, only shuffles existing ones.

# Detailed design

The following list summarizes current behaviour of each funcion from Iterator trait when used on 
an *infinite* iterator.

1. `next`: Never returns None
2. `size_hint`: Counter, Repeat and Cycle with non-empty iterator return `(uint::MAX, None)`
3. `chain`: Returns an infinite iterator
4. `zip`: Returns a possibly ifinite iterator (depends on iterator supplied as an argument)
5. `map`: Returns an infinite iterator
6. `filter`: Returns an infinite iterator
7. `filter_map`: Returns an infinite iterator
8. `enumerate`: Returns an infinite iterator (possible overflow in uint)
9. `peekable`: Returns an infinite iterator
10. `skip_while`: Returns a possibly infinite iterator (depeneds on predicate)
11. `take_while`: Returns a possibly infinite iterator (depeneds on predicate)
12. `skip`: Returns an infinite iterator
13. `take`: Returns a finite iterator
14. `scan`: Returns an ifinite iterator
15. `flat_map`: Returns an ifinite iterator
16. `fuse`: Returns an infinite iterator
17. `inspect`: Returns an infinite iterator
18. `by_ref`: Returns an infinite iterator
19. `advance`: May not fall into an infinite loop (depends on predicate)
20. `collect`: May not fall into an infinite loop (depends on FromIterator implementation)
21. `nth`: Always returns
22. `last`: Always falls into an infinite loop
23. `fold`: Always falls into an infinite loop
24. `len`: Always falls into an infinite loop
25. `all`: May not fall into an infinite loop (depends on predicate)
26. `any`: May not fall into an infinite loop (depends on predicate)
27. `find`: May not fall into an infinite loop (depends on predicate)
28. `position`: May not fall into an infinite loop (depends on predicate)
29. `count`: Always falls into an infinite loop
30. `max_by`: Always falls into an infinite loop
31. `min_by`: Always falls into an infinite loop


Create a new trait and move into it at least these functions that
today result in an *uncoditional* infinite loop:
`last`, `fold`, `len`, `count`, `min_by`, `max_by`.
All these functions greedily reduce an iterator into a single value and clearly
shouldn't be allowed on iterators that are guaranteed to be infinite.

```rust
trait<A> MeaningfulName<A> : Iterator<A> {
    fn last(&mut self) -> Option<A> { ... }
    fn fold<B>(&mut self, init: B, f: |B, A| -> B) -> B { ... }
    fn len(&mut self) -> uint { ... }
    fn count(&mut self, predicate: |A| -> bool) -> uint { ... }
    fn max_by<B: TotalOrd>(&mut self, f: |&A| -> B) -> Option<A> { ... }
    fn min_by<B: TotalOrd>(&mut self, f: |&A| -> B) -> Option<A> { ... }
}
```

Default implementations for these functions would be the same as in current Iterator trait.
After this change:
* `Iterator` trait doesn't assume anything about finiteness of generated sequence of values
* Proposed trait *does* assume that `next` eventually returns `None`.

Iterator trait implementations and trait bounds that include Iterator should be modified
accordingly. Among others:

* Change a trait bound in `AdditiveIterator` implementation
* Change a trait bound in `MultiplicativeIterator` implementation
* Change a trait bound in `OrdIterator` implementation

Trait should *not* be implemented for types representing endless
iterator, such as `Counter` or `Repeat`.

There is at least one problematic edge case. `Cycle` iterator is infinite unless original,
internal iterator is empty, in which case `Cycle` is also empty and therefore finite.
That's very specific and uninteresting case. I think `Cycle` should simply be considered
infinite (i.e. it should not implement proposed trait).

# Alternatives

Not doing this would mean that it's possible to write code that results in an infinite
loop, even though that could be prevented at compile time.

# Unresolved questions

* Which functions should be allowed for possibly infinite iterators.
It's a trade-off between flexibility and infinite loops at run time.
* Consider better wording. Choose a meaningful name for proposed trait.
