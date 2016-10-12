- Feature Name: entry_into_owned
- Start Date: 2016-10-12
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Enable the map Entry API to take borrowed keys as arguments, cloning only when
necessary. The proposed implementation introduces a new trait
`std::borrow::IntoOwned` which enables the existing `entry` methods to accept
borrows. In effect, it makes the following possible:

```rust
  let string_map: HashMap<String, u64> = ...;
  let clone_map: HashMap<Cloneable, u64> = ...;
  let nonclone_map: HashMap<NonCloneable, u64> = ...;

  // ...

  *string_map.entry("foo").or_insert(0) += 1;  // Clones if "foo" not in map.
  *string_map.entry("bar".to_string()) += 1;   // By-value, never clones.

  clone_map.entry(&Cloneable::new());          // Clones if key not in map.
  clone_map.entry(Cloneable::new());           // By-value, never clones.

  nonclone_map.entry(NonCloneable::new());     // Can't and doesn't clone.
```

See [playground](https://is.gd/0lpGej) for a concrete demonstration.

# Motivation
[motivation]: #motivation

The motivation for this change is the same as the one laid out in [#1533](https://github.com/rust-lang/rfcs/pull/1533)
by @gereeter. Below is an adapted version of their `Motivation` section:

The Entry API for maps allows users to save time by allowing them to perform
arbitrary modifications at a given key dependent upon whether that key was
present and if it was, what value was previously there. However, although
insertion is the only action the user might take that requires a by-value key,
there is no way to create an Entry without a fully owned key. When the user only
has a by-reference key on hand, this is inefficient at best, requiring an
unnecessary .to_owned that may involve an expensive allocation and/or copy, and
unusable at worst, if the key cannot be cloned.

Consider a simple word count example:

```rust
fn word_count(text: &str) -> HashMap<String, u64> {
    let mut map = HashMap::new();
    for word in text.split_whitespace() {
        *map.entry(word.to_owned()).or_insert(0) += 1;
    }
    map
}
```

For a large enough text corpus, in the vast majority of cases the entry will be
occupied and the newly allocated owned string will be dropped right away,
wasting precious cycles. We would like the following to work.

```rust
fn word_count(text: &str) -> HashMap<String, u64> {
    let mut map = HashMap::new();
    for word in text.split_whitespace() {
        *map.entry(word).or_insert(0) += 1;
    }
    map
}
```

with a conditional `.to_owned` call inside the `Entry` implementation.
Specifically we're looking for a fix which supports the following cases

  1. `.entry(key)` with `key: K` where `K: !Clone`.
  2. `.entry(key)` with `key: K` where `K: Clone`.
  3. `.entry(&key)` with `key: Q` where `Q: ToOwned<Owned=K>`.

# Detailed design
[design]: #detailed-design

[Playground Proof of Concept](https://is.gd/0lpGej)

## Approach
To justify the approach taken by this proposal, first consider the following
(unworkable) solution:

```rust
  pub fn entry<'a, C, Q: ?Sized>(&'a self, k: C) -> Entry<'a, K, V>
        where K: Borrow<Q>,
              Q: Hash + Eq + ToOwned<Owned=K>
              C: Into<Cow<'a, Q>>
```

This would support (2) and (3) but not (1) because `ToOwned`'s blanket
implementation requires `Clone`. To work around this limitation we take a trick
out of `IntoIterator`'s book and add a new `std::borrow::IntoOwned` trait:

```rust
pub trait IntoOwned<T> {
    fn into_owned(self) -> T;
}

impl<T> IntoOwned<T> for T {
    default fn into_owned(self) -> T { self }
}

impl<T: RefIntoOwned> IntoOwned<T::Owned> for T {
    default fn into_owned(self) -> T::Owned { self.ref_into_owned() }
}

trait RefIntoOwned {
    type Owned: Sized;
    fn ref_into_owned(self) -> Self::Owned;
}

impl<'a, T: ?Sized + ToOwned> RefIntoOwned for &'a T {
    type Owned = <T as ToOwned>::Owned;
    fn ref_into_owned(self) -> T::Owned { (*self).to_owned() }
}
```

The auxilliary `RefIntoOwned` trait is needed to avoid the coherence issues
which an

```rust
impl<'a, T: ?Sized + ToOwned> IntoOwned<T::Owned> for &'a T {
    fn into_owned(self) -> T::Owned { (*self).to_owned() }
}
```

implementation would cause. Then we modify the `entry` signature to

```rust
  pub fn entry<'a, Q>(&'a self, k: Q) -> Entry<'a, K, V, Q>
        where Q: Hash + Eq + IntoOwned<K>
```

and add a new `Q: IntoOwned<K>` type parameter to `Entry`. This can be done
backwards-compatibly with a `Q=K` default. The new `Entry` type will store
`key: Q` and call `into_owned` on insert-like calls, while using Q directly on
get-like calls.

# Drawbacks
[drawbacks]: #drawbacks

1. The docs of `entry` get uglier and introduce two new traits the user
   never needs to manually implement. If there was support for `where A != B`
   clauses we could get rid of the `RefIntoOwned` trait, but that would still
   leave `IntoOwned` (which is strictly more general than the existing `ToOwned`
   trait).

2. It does not offer a way of recovering a `!Clone` key when no `insert`
   happens. This is somewhat orthogonal though and could be solved in a number
   of different ways eg. an `into_key` method on `Entry` or via an `IntoOwned`
   impl on a `&mut Option<T>`-like.

3. Further depend on specialisation in its current form for a public API. If the
   exact parameters of specialisation change, and this particular pattern
   doesn't work anymore, we'll have painted ourselves into a corner.

# Alternatives
[alternatives]: #alternatives

1. Keyless entries ([#1533](https://github.com/rust-lang/rfcs/pull/1533)):

     1. Con: An additional method to the Map API which is strictly more general,
        yet less ergonomic than `entry`.

     2. Con: The duplication footgun around having to pass in the same key twice
        or risk corrupting the map.

     3. Pro: Solves the recovery of `!Clone` keys.

# Unresolved questions
[unresolved]: #unresolved-questions

1. Should these traits ever be stabilised? `RefIntoOwned` in particular can go
   away with the inclusion of `where A != B` clauses:

   ```rust
   impl<'a, T: ?Sized + ToOwned> IntoOwned<T::Owned> for &'a T
       where T::Owned != &'a T
   {
       fn into_owned(self) -> T::Owned { (*self).to_owned() }
   }
   ```
