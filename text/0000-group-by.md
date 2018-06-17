- Feature Name: group_by
- Start Date: 2018-06-15
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

Provide an `Iterator` over a slice that produce non-overlapping runs of elements separated by a given predicate.

# Motivation
[motivation]: #motivation

Adding this `Iterator` to the standard library will help people split slices by using a custom predicate!
This `Iterator` is implemented on generic slices to provide performances and flexibility, `GroupBy` implements `DoubleEndedIterator` without any overhead and it does not need any allocation.

There is a similar method that already exists in [the standard library called `split`](https://doc.rust-lang.org/std/primitive.slice.html#method.split) but it will remove the element that does the separation.
This behavior is not always wanted and could have been achieved by using `group_by` skipping the first element of each groups but the first.

In short it should be added to the standard library because it is a more generic `split` method that cover more use cases.

This method does not fit in the `itertools` library, as the `itertools` description say: _Extra iterator adaptors, functions and macros_. And this function is really optimized for slices/contiguous data.

Here is a loop that return the first element of each group based on the equality predicate:

```rust
let mut previous = None;
let mut iter = slice.iter();
while let Some(elem) = iter.next() {
    if previous.is_none() || previous != Some(elem) {
        previous = Some(elem);

        // do something here with `elem`: the first element of each group
    }
}
```

Using the `GroupBy` `Iterator` here return all the elements which are in the same group, it gives a slice of a complete group with less boilerplate:

```rust
for group in slice.group_by(|a, b| a == b) {
    // do something here with the `group` slice
}
```

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

If you want to split a slice into groups of elements you can use the `GroupBy` `Iterator`. It provides you the ability to specify if two elements that follow each other must be in the same group or not, if the predicate you specify returns `false` so the slice must be split at this point and a new group is returned to the user. A group is no more than a slice of the base slice.

```rust
struct Human {
    age: u32,
    is_cool: bool,
}

let slice = /* a slice of humans */;

// we first group humans by coolness
for coolness_group in slice.group_by(|a, b| a.is_cool == b.is_cool) {
    // and we then group humans by age
    for age_group in coolness_group.group_by(|a, b| a.age == b.age) {
        // ...
    }
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

[A basic implementation is available](http://github.com/Kerollmops/group-by). Note that it implement `DoubleEndedIterator` and so the `next_back` and the `rev` methods.

The implementation that is specified here is only available on slices, the reason is because it is less efficient to do that on any possible `Iterator`, much less optimizations are available to us with simple `Iterator`. It will probably be painful to implement `DoubleEndedIterator` on it.

# Drawbacks
[drawbacks]: #drawbacks

It will add a new type to the slice and it will make the standard library grow.

# Rationale and alternatives
[alternatives]: #alternatives

The current design will make no real overhead compared to one based only on generic `Iterator`s, it does not need allocation at all. The `GroupBy` `Iterator` will have a friend named `GrouByMut` and both will provide a `remainder` method ([following the same borrowing rules has the `ExactChunks/ExactChunksMut`](https://github.com/rust-lang/rust/pull/51339)) that will give the remaining elements.

[The generic implementation on `Iterator` has been tested](https://git.phaazon.net/phaazon/group-by-rs/src/commit/3d3c6d80c02f1813ecc001b110a90392899d0f68) and performances are not here compared to the slice based one.

# Prior art
[prior-art]: #prior-art

This is a useful function that is already present in most of the other language libraries (e.g. [Haskell has `groupBy`](http://hackage.haskell.org/package/base-4.11.1.0/docs/Data-List.html#v:groupBy]).

The good thing that Haskell provide in relation with the `groupBy` function is a `group` function for elements that implement `Eq`. The same behavior can be achieved:

```rust
fn group_by_eq<T: Eq>(slice: &[T]) -> impl Iterator<Item=&[T]> {
    GrouBy::new(slice, PartialEq::eq)
}
```

# Unresolved questions
[unresolved]: #unresolved-questions

In the standard library, when two implementation are near the same, macros are used to remove code duplication, we will need to declare a macro for `GroupBy` and `GroupByMut` that will be generic over the pointer type used (e.g. `*const T` and `*mut T`).
