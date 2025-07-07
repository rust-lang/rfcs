- Feature Name: 'iter_find_many'
- Start Date: 2022-11-18
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This feature would allow finding, returning, and potentially mapping, multiple &muts within a single iterator using a convenient const generic array syntax

# Motivation
[motivation]: #motivation

Credit to @cuviper on the rust internals forum for building out most of this implementation.

This is a very general addition and could support many use cases. Currently it is rather difficult and cumbersome to find multiple &muts from an iterator.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This feature would introduce 2 new methods on `Iterator<Item = &mut T>`.

`find_many` takes an iterator and return multiple mutable references to the items which match the predicate.

Example
```rust
let mut v = vec![(0, 1), (1, 2), (2, 3), (3, 4), (4, 5)];
let [left, right] =
v.find_many([&2, &3], |item, key| &item.0 == key).unwrap();
assert_eq!(*left, (2, 3));
assert_eq!(*right, (3, 4));
```

`find_map_many` takes an iterator and returns multiple mutable references to the items which match the predicate. The items which match the predicate are then mapped to a different &mut. This is particularly useful when doing something similar to a key value search.

Example
```rust
let mut v = vec![(0, 1), (1, 2), (2, 3), (3, 4), (4, 5)];
let [left, right] =
v.find_map_many(&mut v, [&2, &3], |item, key| &item.0 == key, |item| &mut item.1).unwrap();
assert_eq!(*left, 3);
assert_eq!(*right, 4);
```

For both methods an option containing an array to the &muts would be returned. The None variant would represent cases where there are no matching items in the iterator for every key. Each key provided requires a unique Item.

This feature will make make handling mutable iterators much more ergonomic and should prevent programmers from rearranging code in unintuitive ways just to make the borrow checker happy. This should result in more natural layout of functions.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

These methods rely on two nightly features of rust to be enabled. `array_try_map` and `inline_const`.

```rust
#![feature(array_try_map, inline_const)]

pub fn find_many<'a, I, T, F, K, const LEN: usize>(collection: I, keys: [&K; LEN], mut predicate: F) -> Option<[&'a mut T; LEN]>
where
    I: IntoIterator<Item = &'a mut T>,
    F: FnMut(&T, &K) -> bool,
{
    let mut remaining = LEN;
    let mut output = [const { None::<&'a mut T> }; LEN];

    'collection: for elem in collection {
        for (key, out) in std::iter::zip(&keys, &mut output) {
            if out.is_none() && predicate(elem, key) {
                *out = Some(elem);
                remaining -= 1;
                if remaining == 0 {
                    break 'collection;
                }
                break;
            }
        }
    }

    output.try_map(|opt| opt)
}
```
```rust
#![feature(array_try_map, inline_const)]

pub fn find_map_many<'a, I, T, U, F, M, K, const LEN: usize>(
    collection: I, keys: [&K; LEN], mut predicate: F, mut map: M,
) -> Option<[&'a mut U; LEN]>
where
    I: IntoIterator<Item = &'a mut T>,
    T: 'a,
    F: FnMut(&T, &K) -> bool,
    M: FnMut(&'a mut T) -> &'a mut U,
{
    let mut remaining = LEN;
    let mut output = [const { None::<&'a mut U> }; LEN];

    'collection: for elem in collection {
        for (key, out) in std::iter::zip(&keys, &mut output) {
            if out.is_none() && predicate(elem, key) {
                *out = Some(map(elem));
                remaining -= 1;
                if remaining == 0 {
                    break 'collection;
                }
                break;
            }
        }
    }

    output.try_map(|opt| opt)
}
```

# Drawbacks
[drawbacks]: #drawbacks

There aren't many drawbacks to this other than perhaps filling up the standard library a little more. I think these functions provide plenty of utility.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

I think overall this makes rust code easier to read and write. This could potentially go into an external crate, but I feel it is useful enough to belong in the standard library. I don't think there are many alternative ways of doing this.

# Prior art
[prior-art]: #prior-art

I am not aware of how other languages have solved this problem. My experience with using programming languages that enforce strict ownership rules is limited to rust only.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

I am not 100% sold on the API of find_map_many. The predicate closure and the map closure had to be separated in order to appease the borrow checker. Perhaps there is a way around this that would make it more ergonomic. I'm open to suggestions there.

# Future possibilities
[future-possibilities]: #future-possibilities

These functions could also be implemented for `Iterator<Item = &T>` though I don't see them as useful or as necessary as they are for &mut Iterators. Would like to hear other ideas about that. 
