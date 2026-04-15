- Feature Name: `peeked_entry_api`
- Start Date: 2026-04-15
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary
[summary]: #summary

Entry-like API for `Peekable`.

## Motivation
[motivation]: #motivation

`Peekable` provides several methods for conditional iteration such as [`next_if`](https://doc.rust-lang.org/stable/std/iter/struct.Peekable.html#method.next_if) and [`next_if_map`](`https://doc.rust-lang.org/stable/std/iter/struct.Peekable.html#method.next_if_map`).
However, those methods are not easily composable, especially when there are several `Peekable`s.
Sometimes it is easier to `peek` the value, do something with it, and then use `next` to extract the peeked value.
This approach leads to unnecessary `Option`s and `unwrap`s.
That is, `next` cannot return `None` if the corresponding `peek` returned `Some`.

Consider the following example.
Suppose we have two sorted iterators.
The keys are unique in each iterator.
We want to merge these iterators into a single iterator with the same properties.
That is, its values must be sorted and unique.
Equal values should be "merged" into a single one.
Implementing this with `Peekable` is straightforward.
We `peek` values from the underlying iterators, compare them, and either yield the lesser or the merged value:

```rust
struct MergeIter<
    T: Ord,
    A: Iterator<Item = T>,
    B: Iterator<Item = T>,
    M: FnMut(T, T) -> T,
> {
    a: Peekable<Fuse<A>>,
    b: Peekable<Fuse<B>>,
    merge: M,
}

impl<
    T: Ord,
    A: Iterator<Item = T>,
    B: Iterator<Item = T>,
    M: FnMut(T, T) -> T,
> Iterator for MergeIter<T, A, B, M> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let Some(a_val) = self.a.peek() else { return self.b.next(); };
        let Some(b_val) = self.b.peek() else { return self.a.next(); };
        match a_val.cmp(b_val) {
            Ordering::Less => self.a.next(),
            Ordering::Equal => Some(
                (self.merge)(self.a.next().unwrap(), self.b.next().unwrap())
                //                         ^^^^^^                  ^^^^^^
            ),
            Ordering::Greater => self.b.next(),
        }
    }
}
```

Note the two problems in this code.
First, we don't directly return the compared values.
This does not express our intent clearly and is bugprone.
Second, we use `unwrap` that cannot ever fail.
This is a sign of poorly expressed invariants.
We could use `?` instead, but, again, it would never actually shortcut, and it might be even more confusing.

The proposed entry-like API solves both of these problems. The code looks like this:

```rust
let Some(a_peeked) = self.a.peek_entry() else { return self.b.next(); };
let Some(b_peeked) = self.b.peek_entry() else { return self.a.next(); };
Some(
    match a_peeked.cmp(&b_peeked) {
        Ordering::Less => a_peeked.extract(),
        Ordering::Equal => (self.merge)(a_peeked.extract(), b_peeked.extract()),
        Ordering::Greater => b_extract.extract(),
    }
)
```

Here, `a_peeked` and `b_peeked` are proxies (entries) to the peeked values.
The entry type implements `Deref` and `DerefMut`.
The `extract` method consumes the entry and yields the value from the corresponding `Peekable`.

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

`Peekable<I>` has the following method:

```rust
fn peek_entry<'a>(&'a mut self) -> Option<PeekedEntry<'a, I>> { ... }
```

It is similar to `peek_mut`, but returns a proxy to the peeked object instead of a plain reference to it.

`PeekedEntry` implements `Deref` and `DerefMut` with `Target = I::Item`, providing an access to the peeked object.

`PeekedEntry` also has

```rust
fn extract(self) -> I::Item { ... }
```

The `extract` method returns the peeked value. It is equivalent to calling `next().unwrap()` on the `Peekable`.

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The implementation is straighforward.
`PeekedEntry` stores a mutable reference to the `Option<Option<I::Item>>` inside `Peekable`.
Both `Option`s are known to contain a value during the lifetime of `PeekedEntry`.
This allows to efficiently implement `Deref` and `DerefMut` without performing additional checks.
`extract` moves the value out and breaks the invariant.
Hence it consumes the `PeekedEntry` object.

Sample implementation: [`here`](https://github.com/NamorNiradnug/rust/commit/d423cb389e1dd32fafd9ed7b4d8f439801c09403)

## Drawbacks
[drawbacks]: #drawbacks

It seems like a niche feature.

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

An alternative is to not implement this in the standard library but outside.
The only difference would be the inability to use nice method-call syntax for `.peek_entry()`.
The implementation would also be a little less pretty.

## Unresolved questions
[unresolved-questions]: #unresolved-questions

Are `peek_entry()` and `PeekedEntry` good names?
They could be confusing, because `Peekable` has nothing to do with `[Hash]Set/Map`.
Although there is some similarity.

## Future possibilities
[future-possibilities]: #future-possibilities

This API could be a part of the `Peek` trait.
It is worth noting that `next_if`, `next_if_map` and other existing `Peekable`'s methods can be implemented in terms of `PeekedEntry`.

`PeekedEntry` could implement `DerefMove` instead of providing the `extract` method.
