- Feature Name: keyless_entry
- Start Date: 2016-03-06
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add a variation to the entry API for maps that doesn't store the search key in the `Entry`.

# Motivation
[motivation]: #motivation

The `Entry` API for maps allows users to save time by allowing the user to perform arbitrary modifications
at a given key dependent upon whether that key was present and if it was, what value was previously there.
However, although insertion is the only action the user might take that requires a by-value key, there is
no way to create an `Entry` without a fully owned key. When the user only has a by-reference key on hand,
this is inefficient at best, requiring an unnecessary `.to_owned` that may involve an expensive allocation
and/or copy, and unusable at worst, if the key cannot be cloned.

It is a testament to the flexibility and control afforded by the `Entry` API that nearly every method on
a generic map can be conceptually implemented in terms of `entry`. However, this trick is not used
and cannot be used in practice because of the same issue - since, e.g., `remove` takes a by-reference key,
it cannot call `entry` and therefore cannot completely share its implementation with `entry`. To still recover
as much sharing as possible, map implementations seem to have converged on using an internal variation on the
`Entry` API, only differing in that the key itself is not stored with the `Entry`. For example, `HashMap` has
its `FullBucket`s and `VacantEntryState`s, and after rust-lang/rust#32058 will have an `InternalEntry` type,
while `BTreeMap` has its `Handle`s, and before the rewrite had `SearchStack`s. Making these modified entries
public gives the user exactly the control they need.

# Detailed design
[design]: #detailed-design

## API

Two new types are introduced on each map. `VacantKeylessEntry<'a, K, V>` is kept abstract, while
`KeylessEntry<'a, K, V>` is defined as follows:

```
pub enum KeylessEntry<'a, K, V> {
    Occupied(OccupiedEntry<'a, K, V>),
    Vacant(VacantKeylessEntry<'a, K, V>)
}
```

To obtain a `KeylessEntry`, a new method is added to the maps:

```
impl<K, V> Map<K, V> {
    /// Gets the given key's corresponding entry in the map for in-place manipulation.
    pub fn keyless_entry<'a, Q: ?Sized>(&'a mut self, key: &Q) -> KeylessEntry<'a, K, V> where K: Borrow<Q>;
}
```

with the appropriate `Ord` or `Eq + Hash` bound on `Q`.

The methods on `KeylessEntry`s mirror those on `Entry`s, only differing in that they take keys when doing
insertion:

```
impl<'a, K, V> KeylessEntry<'a, K, V> {
    /// Ensures a value is in the entry by inserting the default if empty, and returns
    /// a mutable reference to the value in the entry.
    pub fn or_insert(self, default_key: K, default: V) -> &'a mut V {
        match self {
            Occupied(entry) => entry.into_mut(),
            Vacant(entry) => entry.insert(default),
        }
    }

    /// Ensures a value is in the entry by inserting the result of the default function if empty,
    /// and returns a mutable reference to the value in the entry.
    pub fn or_insert_with<F: FnOnce() -> (K, V)>(self, default: F) -> &'a mut V {
        match self {
            Occupied(entry) => entry.into_mut(),
            Vacant(entry) => {
               let (key, value) = default();
               entry.insert(key, value)
            },
        }
    }
}

impl<'a, K: 'a, V: 'a> VacantKeylessEntry<'a, K, V> {
    /// Sets the value of the entry with the VacantKeylessEntry's key,
    /// and returns a mutable reference to it
    pub fn insert(self, key: K, value: V) -> &'a mut V;
}
```

`KeylessEntry` and `VacantKeylessEntry` also have methods for converting them into more traditional entries:

```
impl<'a, K, V> KeylessEntry<'a, K, V> {
    /// Associates a key with the `KeylessEntry`, making it no longer keyless and just an `Entry`.
    pub fn with_key(self, key: K) -> Entry<'a, K, V> {
        match self {
            Occupied(entry) => Entry::Occupied(entry),
            Vacant(entry) => Entry::Vacant(entry.with_key(key)),
        }
    }
}

impl<'a, K: 'a, V: 'a> VacantKeylessEntry<'a, K, V> {
    /// Associates a key with the `VacantKeylessEntry`, making it no longer keyless and just
    /// a `VacantEntry`.
    pub fn with_key(self, key: K) -> VacantEntry<'a, K, V>;
}
```

## Implementation

As mentioned in the motivation section of this RFC, the implementations of `BTreeMap` and `HashMap` already
align quite well with this API. `BTreeMap`'s `VacantKeylessEntry` can be implemented simply as a wrapper around
a `Handle<NodeRef<marker::Mut<'a>, K, V, marker::Leaf>, marker::Edge>`, while `HashMap`'s `VacantKeylessEntry`
can be a pair of a `SafeHash` and a `VacantEntryState`.

# Drawbacks
[drawbacks]: #drawbacks

Having the user plug the key in twice when doing an insertion makes it more easy to shoot oneself in the foot
by putting in two unequal keys. Although this can mess up the internal structure of the map, it cannot cause
memory unsafety, as this possibility was already allowed by having a query where `key` and `key.to_owned()`
are unequal.

# Alternatives
[alternatives]: #alternatives

- Replace the `Entry` API entirely. This would get rid of the inherent duplication of having two `Entry` APIs,
  but would be backwards incompatible.
- Create an `OccupiedKeylessEntry` that is separate from `OccupiedEntry` to help keep to two different `Entry`s
  straight. This might reduce confusion and wouldn't lock maps into dropping search keys immediately upon finding
  a match, but would add another type with identical behavior, which would be arguably even more confusing.
- Publicly expose an implementation of `VacantEntry` as a struct containing a key and a `VacantKeylessEntry`. This
  would help clarify the relationship between the two types and match more with the reuse of `OccupiedEntry` in
  both the keyed and keyless cases. However, it can be done backwards compatibly later and doesn't have any
  particularly compelling use cases.
- Make `Entry` hold a `Cow` and add an `entry_lazy` method for creating `Entry`s that don't own their keys, as
  (suggested)[https://internals.rust-lang.org/t/head-desking-on-entry-api-4-0/2156] by @Gankro. While this might
  work, it runs into issues of changing the signature of `Entry`, introducing a `Clone` bound on the non-lazy
  case even though the key will never be cloned. Additionally, it doesn't support some more exotic use cases in
  which the keys cannot be duplicated at all.
- Add an `entry_or_clone` method as suggested in rust-lang/rfcs#1203 that clones its key if the result is a
  `VacantEntry`. This is an improvement, but doesn't handle all use cases and still is needlessly inefficient, as
  the user might still not wish to do an insertion after finding a vacant slot.

# Unresolved questions
[unresolved]: #unresolved-questions

None
