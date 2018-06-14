- Feature Name: Add `delete` and `delete_by` to `Iterator`
- Start Date: 2018-06-14
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary
This RFC asks to add two methods to `std::iter::Iterator`, namely `delete` and `delete_by`.

`delete` would delete the first occurrence of a value in an iterator.
`delete_by` acts like `delete`, but also lets the user specify their own comparator predicate.
This is different from `filter` which removes all occurrences that match the predicate.

# Motivation
[motivation]: #motivation

The motivation for this is simply to be able to filter out single copies of values, rather than all of them.

I was in this exact situation myself, and had to make use of the destructive `Vec::remove` method.
Having a non-destructive way of doing something similar that fits within the ethos of `Iterator` seems nicer.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The `delete` method removes the first occurrence of its input from the iterator.
Example 1:
```rust
let v = vec![1, 1, 2, 3, 4, 4, 5, 6];
let result = v.iter().delete(4); // `result` now contains [1, 1, 2, 3, 4, 5, 6]
```
Example 2:
Suppose you're making a dice roller that needs to be able to subtract a single die from the result, this could be easily done as so:
```rust
let outcomes = vec![5, 2, 2, 6];
let smallest = outcomes.iter().min().unwrap();
let result = outcomes.delete(*smallest); // result now contains [5, 2, 6]
```

If the required value doesn't already exist in the iterator, nothing happens; it's already gone.

`delete_by` would work like `delete`, but also take a binary predicate: 
```rust
// `result` contains [1, 2, 3, 5, 6, 7, 8, 9], skipping 4
let result = (1 .. 10).delete_by(4, |x, y| x <= y); 
```

In fact, `delete` can be implemented by means of `delete_by`, as `delete` only requires checking for equality

# Drawbacks
[drawbacks]: #drawbacks

Some could argue that this solves a very strange small edge case, and the inclusion would bloat the standard library.
After all, the language is already expressive enough for the user to be able to implement this on a case-by-case basis.

That being said, to get a somewhat minimal implementation of this as it stands using built in methods, 
turning the iterator into a vector and then destructively altering it seems less than ideal.

# Rationale and alternatives
[alternatives]: #alternatives

As it stands, there is no _nice_ way of non-destructively removing a single element from an iterator.
There might be a more general way of implementing it that could be more useful in general; however, 
as it stands simply using `filter`, `map`, or `fold` won't do the trick when wanting to afect a single item in an unkown spot in the iterator.

# Prior art
[prior-art]: #prior-art

These methods are directly inspired by Haskell's functions of the same name 
(`Data.List.delete` and `Data.List.deleteBy` found [here](http://hackage.haskell.org/package/base-4.11.1.0/docs/src/Data.OldList.html#delete)),
which fits right into `Iterator` like the other Haskell-inspired methods like `take_while` and `skip_while` (called `dropWhile` in Haskell).

# Unresolved questions
[unresolved]: #unresolved-questions

- Maybe `delete` sounds too destructive, could there be a better name?
