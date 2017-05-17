- Feature Name: get_collection_keys
- Start Date: 2015-06-24
- RFC PR: https://github.com/rust-lang/rust/pull/26531
- Rust Issue:

# Summary

Get the original keys back from collections.

# Motivation

Removing a key from a HashMap or an item from a HashSet will drop the item.
This is usually acceptable for small clonable objects like numbers and strings,
but they are not always clonable, and even if they are it might be an expensive
operation.

# Detailed design

There are currently four operations that could have a new function to them:
`get`, `get_mut`, `insert` and `remove`. All of those methods receive a reference
to a key type and return reference to the value or a the value itself.

New analogous methods can be added to return not just the value but both the key
and the value. For `insert` and `remove` they can return the key and the value,
while `get` and `get_mut` will return a reference to them. Notice that `get_mut`
will return a mutable value but not a mutable key.

In the case of HashSet, the same concept applies for the items as if they were
keys in a hash map. It does not make sense to implement something like `get_mut`
in this case.

Structs that should implement these new methods are:

* std::collections::BTreeMap
* std::collections::HashMap
* std::collections::BTreeSet
* std::collections::HashSet


Some structs might implement these new methods, but their benefit besides
consistency is unclear since it would only apply for `usize`:

* std::collections::VecMap
* std::collections::BitSet

New API methods:

```rust
impl<K: Ord, V> BTreeMap<K, V> {
    // ...

    pub fn keyed_get<Q: ?Sized>(&self, key: &Q) -> Option<(&K, &V)> where K: Borrow<Q>, Q: Ord;
    pub fn keyed_get_mut<Q: ?Sized>(&mut self, key: &Q) -> Option<(&K, &mut V)> where K: Borrow<Q>, Q: Ord;
    pub fn keyed_insert(&mut self, mut key: K, mut value: V) -> Option<(K, V)>;
    pub fn keyed_remove<Q: ?Sized>(&mut self, key: &Q) -> Option<(K, V)> where K: Borrow<Q>, Q: Ord;
}

impl<T: Ord> BTreeSet<T> {
    // ...

    pub fn get<Q: ?Sized>(&self, value: &Q) -> Option<&T> where T: Borrow<Q>, Q: Ord;
    pub fn insert_item(&mut self, value: T) -> Option<T>;
    pub fn remove_item<Q: ?Sized>(&mut self, value: &Q) -> Option<T> where T: Borrow<Q>, Q: Ord;
}

impl<K, V, S> HashMap<K, V, S>
    where K: Eq + Hash, S: HashState
{
    // ...

    pub fn keyed_get<Q: ?Sized>(&self, k: &Q) -> Option<(&K, &V)>
        where K: Borrow<Q>, Q: Hash + Eq;
    pub fn get_mut<Q: ?Sized>(&mut self, k: &Q) -> Option<(&K, &mut V)>;
    pub fn keyed_insert(&mut self, k: K, v: V) -> Option<(K, V)>;
    pub fn keyed_remove<Q: ?Sized>(&mut self, k: &Q) -> Option<(K, V)>;
}

impl<T, S> HashSet<T, S>
    where T: Eq + Hash, S: HashState {
    // ...

    pub fn get<Q: ?Sized>(&self, value: &Q) -> Option<&T>
        where T: Borrow<Q>, Q: Hash + Eq;
    pub fn remove_item<Q: ?Sized>(&mut self, value: &Q) -> Option<T>
        where T: Borrow<Q>, Q: Hash + Eq;
    pub fn insert_item(&mut self, value: T) -> Option<T>;
}
```

The existing `OccupiedEntry` could benefit from these new methods and provide
its own pretty API on top:

```rust

impl<'a, K, V> OccupiedEntry<'a, K, V> {
    fn key(&self) -> &K;
    fn keyed_remove(self) -> (K, V);
    fn keyed_get_mut(&mut self) -> (&K, &mut V);
    fn keyed_get(&self) -> (&K, &V);
    fn keyed_into_mut(self) -> (&'a K, &'a mut V);
}

```

# Drawbacks

Add more code that will be used only sporadically.

# Alternatives

Keep the current design, since keys are immutables once in the collection use
an Rc.

Add methods to only fetch the keys, but most of the users will need both or
can easily discard the value.

# Unresolved questions

Naming. `Entry` is already taken for a similar purpose. Having the same name
for sets and maps is desirable, but not required. `keyed` and `item` might be
a good fit.
