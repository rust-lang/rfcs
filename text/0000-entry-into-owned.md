- Feature Name: entry_into_owned
- Start Date: 2016-10-12
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Enable the map Entry API to take borrowed keys as arguments, cloning only when
necessary (in `VacantEntry::insert`). The proposed implementation introduces a
new trait `std::borrow::AsBorrowOf` which enables the existing `entry` methods
to accept borrows. In effect, it makes the following possible:

```rust
  let string_map: HashMap<String, u64> = ...;
  let clone_map: HashMap<Cloneable, u64> = ...;
  let nonclone_map: HashMap<NonCloneable, u64> = ...;

  // ...

  *string_map.entry("foo").or_insert(0) += 1;                  // Clones if "foo" not in map.
  *string_map.entry("bar".to_string()).or_insert(0) += 1;      // By-value, never clones.

  *clone_map.entry(&Cloneable::new()).or_insert(0) += 1;       // Clones if key not in map.
  *clone_map.entry(Cloneable::new()).or_insert(0) += 1;        // By-value, never clones.

  *nonclone_map.entry(NonCloneable::new()).or_insert(0) += 1;  // Can't and doesn't clone.
```

See [playground](https://is.gd/XBVgDe) and [prototype
implementation](https://github.com/rust-lang/rust/pull/37143).

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
implementation requires `Clone`. To work around this limitation we define a
different trait `std::borrow::AsBorrowOf`:

```rust
pub trait AsBorrowOf<T, B: ?Sized>: Sized where T: Borrow<B> {
    fn into_owned(self) -> T;
    fn as_borrow_of(&self) -> &B;
}

impl<T> AsBorrowOf<T, T> for T {
    fn into_owned(self) -> T { self }
    fn as_borrow_of(&self) -> &Self { self }
}

impl<'a, B: ToOwned + ?Sized> AsBorrowOf<B::Owned, B> for &'a B {
    fn into_owned(self) -> B::Owned { self.to_owned() }
    fn as_borrow_of(&self) -> &B { *self }
}
```

This trait defines a relationship between three types `T`, `B` and `Self` with
the following properties:

  1. There is a by-value conversion `Self` -> `T`.
  2. Both `T` and `Self` can be borrowed as `&B`.

These properties are precisely what we need an `entry` query: we need (2) to
hash and/or compare the query against exiting keys in the map and we need (1) to
convert the query into a key on vacant insertion.

The two impl-s capture that

  1. `T` can always be converted to `T` and borrowed as `&T`. This enables
     by-value keys.
  2. `&B` can be converted to `B::Owned` and borrowed as `&B`, when B:
     `ToOwned`. This enables borrows of `Clone` types.

Then we modify the `entry` signature (for `HashMap`, but similar for `BTreeMap`)
to

```rust
pub fn entry<'a, Q, B>(&'a self, k: Q) -> Entry<'a, K, V, Q>
      where Q: AsBorrowOf<K, B>
            K: Borrow<B>,
            B: Hash + Eq {
    // use `hash(key.as_borrow_of())` and `key.as_borrow_of() == existing_key.borrow()`
    // for comparisions and `key.into_owned()` on `VacantEntry::insert`.
}
```

## Detailed changes:

Also see [working implementation](https://github.com/rust-lang/rust/pull/37143)
for diff.

  1. Add `std::borrow::Borrow` as described in previous section.
  2. Change `Entry` to add a `Q` type parameter defaulted to `K` for backwards
     compatibility (for `HashMap` and `BTreeMap`).
  3. `Entry::key`, `VacantEntry::key` and `VacantEntry::into_key` are moved to a
     separate `impl` block to be implemented only for the `Q=K` case.
  4. `Entry::or_insert`, `Entry::or_insert_with` and `VacantEntry::insert` gain
     a `B` type parameter and appropriate constraints: `where Q: AsBorrowOf<K, B>, K: Borrow<B>, B: Hash + Eq`.


# Drawbacks
[drawbacks]: #drawbacks

1. The docs of `entry` get uglier and introduce a new trait the user
   never needs to manually implement.

2. It does not offer a way of recovering a `!Clone` key when no `insert`
   happens. This is somewhat orthogonal though and could be solved in a number
   of different ways eg. an `into_query` method on `Entry`.

4. The changes to `entry` would be insta-stable (not the new traits). There's
   no real way of feature-gating this.

5. May break inference for uses of maps where `entry` is the only call (`K` can
   no longer be necessarily inferred as the arugment of `entry`). May also hit
   issue [#37138](https://github.com/rust-lang/rust/issues/37138).

6. The additional `B` type parameter on `on_insert_with` is a backwards
   compatibility hazard, since it breaks explicit type parameters
   (e.g. `on_insert_with::<F>` would need to become `on_insert_with::<F, _>`).
   This seems very unlikely to happen in practice: F is almost always a closure
   and even when it isn't `on_insert_with` can always infer the type of `F`.

# Alternatives
[alternatives]: #alternatives

1. Keyless entries ([#1533](https://github.com/rust-lang/rfcs/pull/1533)):

     1. Con: An additional method to the Map API which is strictly more general,
        yet less ergonomic than `entry`.

     2. Con: The duplication footgun around having to pass in the same key twice
        or risk corrupting the map.

     3. Pro: Solves the recovery of `!Clone` keys.

2. Add a `entry_or_clone` with an `Q: Into<Cow<K>>` bound.

     1. Con: Adds a new method as well as new `Entry` types for all maps.

     2. Con: Passes on the problem to any generic users of maps with every layer
        of abstraction needing to provide an `or_clone` variant.

     3. Pro: probably clearest backwards compatible solution, doesn't introduce
        any new traits.

3. Split `AsBorrowOf` into `AsBorrowOf` and `IntoOwned`. This is closer to the
   original proposal:

     1. Con: Requires introducing three new traits.

     2. Con: Requires specialisation to implement a public API, tying us closer
        to current parameters of specialisation.

     3. Pro: `IntoOwned` may be independently useful as a more general
        `ToOwned`.

     4. Pro: no additional `B` type parameter on `on_insert` and
        `on_insert_with`.

Code:
```rust
pub trait IntoOwned<T> {
    fn into_owned(self) -> T;
}

impl<T> IntoOwned<T> for T {
    default fn into_owned(self) -> Self {
        self
    }
}

impl<T> IntoOwned<T::Owned> for T
    where T: RefIntoOwned
{
    default fn into_owned(self) -> T::Owned {
        self.ref_into_owned()
    }
}

pub trait AsBorrowOf<T, B: ?Sized>: IntoOwned<T> where T: Borrow<B> {
    fn as_borrow_of(&self) -> &B;
}

impl<T> AsBorrowOf<T, T> for T {
    default fn as_borrow_of(&self) -> &Self {
        self
    }
}

impl<'a, B: ToOwned + ?Sized> AsBorrowOf<B::Owned, B> for &'a B {
    default fn as_borrow_of(&self) -> &B {
        *self
    }
}

// Auxilliary trait to get around coherence issues.
pub trait RefIntoOwned {
    type Owned: Sized;

    fn ref_into_owned(self) -> Self::Owned;
}

impl<'a, T: ?Sized> RefIntoOwned for &'a T
    where T: ToOwned
{
    type Owned = <T as ToOwned>::Owned;

    fn ref_into_owned(self) -> T::Owned {
        (*self).to_owned()
    }
}

```

# Unresolved questions
[unresolved]: #unresolved-questions

1. Are the backwards compatibility hazards acceptable?
2. Is the `IntoOwned` version preferable?
