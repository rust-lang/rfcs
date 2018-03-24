- Feature Name: selfexhausting_iter_adapter
- Start Date:
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add an adapter `.exhausting()` to `Iterator` which causes the iterator to be driven to its end on drop. Before dropping it will act exactly like the source iterator.

# Motivation
[motivation]: #motivation

This RFC is part of two RFCs for splitting up the functionality of `drain()` into orthogonal APIs. It's related to [the RFC for non-selfexhausting drains](https://github.com/Emerentius/rfcs/blob/non-selfexhausting_drain/text/0000-non-selfexhausting_drain.md), but not dependent on it.
The current `drain` APIs run the following code on drop before repairing the collection's state:

```rust
for _ in &mut self {}
```

That is, they run themselves to the end for the side-effect of calling the destructor on each element to be removed from the collection.

This showcases the use of self-exhausting iterators. The principle can apply to any side effecting iterator where all side effects are needed but not (all) the elements it returns. An `.exhausting()` adapter allows adding self-exhaustion on drop to arbitrary iterators.
The behaviour of `drain` could be attained by combining two separate functions:
`drain_nonexhausting().exhausting()`

Iteration through `.by_ref()` and subsequent consumption can achieve the same result, but only when one is holding the iter. With `.exhausting()`, the iter can be passed to a function or returned from one. For this reason, it also works better with method chaining.
Note that returning a self-exhausting iterator from a function should mostly be limited to callback situations. Hardcoding self-exhaustion is mixing concerns and needlessly limiting.
Adding `exhausting` to the std library should make it easy for users to gain this behaviour where necessary and avert more non-lazy iterator APIs in the std library and outside of it.

## Examples
```rust
// pass side effecting iter away
iter_of_iters.flat_map(|iter| {
        iter.map(side_effects)
            .exhausting() // finish what you've started
            .take_while(condition)
    });

// return self-exhausting iter from function
fn drain(&mut self) -> Drain {
    // wrapper can forward all iterator methods to internal iter
    Drain(
        self.drain_nonexhausting()
            .exhausting()
    )
}

// -------------------- Aesthetic improvements only ---------------------
// Current: manual exhaustion with by-ref
let mut iter = iter.some()
    .adapter()
    .chain();
let val = iter.by_ref() // chain breaking indirection
    .map(func)
    find(condition);
iter.for_each(|_| {}); // explicitly consume iterator
                       // must have access to iter

// With proposal:
let val = iter.some()   // all of this
    .adapter()          // will run
    .chain()            // for all elements in iter
    .exhausting()
    .map(func)          // runs only until an element
    .find(condition);   // is found or iter is exhausted
//----------------------------------------------------------------------
```

# Implementation

The `exhausting()` method should take the iterator by value.
During iteration, `Exhausting` is a trivial wrapper that acts like `&mut Self`, meaning it implements all the Iterator traits that the contained iter implements and will always do external iteration. On drop, it runs `for _ in self {}`.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The iterator created by `iter.exhausting()` behaves in the same fashion as `iter` and will on drop be automatically driven to its end. This is useful, if `iter` causes side-effects and is passed away so that one can't exhaust it explicitly with `.for_each(drop)`.

# Drawbacks
[drawbacks]: #drawbacks
* `.exhausting()` may be too niche a usecase with `drain` already including the behaviour.

* The `Exhausting` adapter has a corner case on finished, non-fused iterators. On drop, it will attempt iteration again which will result in implementation dependent behaviour unless guarded against with a flag and a comparison on every `.next()`.

# Rationale and alternatives
[alternatives]: #alternatives
* Add the adapter to itertools.

* Add `exhausting` to the `FusedIterator` trait to side-step the issue of non-fused iterators altogether.

# Prior art
[prior-art]: #prior-art

The drain APIs are as far as I'm aware the only place where iterators show deferred self-exhaustion. I did not find any proposals of this kind before.
Eager iterator consumers were proposed multiple times before. So far, `for_each()` has been added and a closure-less, by-ref version of it has been proposed as well, but not added. The latter goes under the name of `exhaust()` or `drain()`: [1](https://github.com/rust-lang/rust/pull/45168), [2](https://github.com/rust-lang/rust/issues/44546), [3](https://github.com/rust-lang/rust/pull/48945).

`exhausting()` iterates by reference like `exhaust()`, but it can't be written through currently existing adapters.
# Unresolved questions
[unresolved]: #unresolved-questions

* Self-exhausting iterators that can panic during `next()` can easily run into double panics. If `next()` panics during normal program execution,
  then the drop of `Exhausting` will cause `next()` to be called again which is not unlikely to produce another panic, resulting in the whole program to abort.

  We could guard against aborts, by not self-exhausting when the panic occured during iteration (communicated via a flag) or, alternatively, not to self-exhaust under any panic (with `std::thread::panicking()`). This is a choice between possibly unnecessary leaks and a higher likelihood of accidentally tearing down the whole process. An explicit iteration through `by_ref()` followed by exhaustion through `for_each()` would also skip the exhaustion if a panic occurs at any point.

  Example of how this would look like:

```rust
impl<T: Iterator> Iterator for Exhausting<T> {
    type Item = ...;
    fn next(...) -> ... {
        // no additional branching
        self.currently_iterating = true;   // no double iteration
        let next = self.iter.next();
        self.current_iterating = false;    // no double iteration
    }
}

impl<T: Iterator> Drop for Exhausting<T> {
    fn drop(&mut self) {
        // if !std::thread::panicking() {  // no panicking iteration at all
        if !self.currently_iterating {     // no double iteration
            for _ in self.iter {}
        }
    }
}
```
