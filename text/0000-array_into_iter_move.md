- Feature Name: array_into_iter_move
- Start Date: 2017-10-21
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Implement into_iter on array types (not reference, so it moves).

# Motivation
[motivation]: #motivation

Arrays are particularly useful in `flat_map`, but currently a Vec is required as a movable
iterator because arrays only have slice iterator semantics. Obviously this is not optimal.

Now with `ManuallyDrop` implemented, it should be possible to implement moving into_iter on arrays.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

You can convert an array into an iterator with move, just like Vec:

```rust
for x in [1, 2] {
    // x is {integer} instead of &{integer}
}
```

```rust
let v: Vec<_> = (0..5).flat_map(|x| [x, x*2]).collect();
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Proposed implementation:

- a type `array::IntoIter<T>` is created. The `array::into_iter` method is implemented.
- 0..32 variant is first implemented using macros, then migrated to const generics once it lands.
- Inside the method:
  * Wrap the array with `ManuallyDrop`.
  * Create a `IntoIter` struct with contents moved.
- Inside the iterator:
  * Keep track of valid range (index) and move (`ptr::read`) items out as `next()` is called.
  * Don't forget to drop the items if the iterator itself is dropped in middle. This should be done with `drop_in_place`.

Before this lands, a deprecation warning should be implemented in either rustc or clippy to minimize the breakage. (See below for compatibility issues.)

We should also add a lint for redundant Vec usage (instead of array) in clippy to promote the use of this.

# Drawbacks
[drawbacks]: #drawbacks

This is not 100% backwards compatible as it changes the signature of into_iter (only if directly
called like below). A crater run is required.

```rust
[1, 2].into_iter();
// This was originally yielding references, but now values.
```

# Rationale and alternatives
[alternatives]: #alternatives

TBD

# Unresolved questions
[unresolved]: #unresolved-questions

TBD
