- Feature Name: ordered-queries
- Start Date: 2015-09-25
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Add the following to BTreeMap

* min
* max
* get_le
* get_lt
* get_ge
* get_gt

* min_mut
* max_mut
* get_le_mut
* get_lt_mut
* get_ge_mut
* get_gt_mut


and to BTreeSet:

* min
* max
* get_le
* get_lt
* get_ge
* get_gt



# Motivation

Currently the only option people have to do ordered queries on a BTreeMap is to make a full-blown
`range` iterator with carefully selected bounds. However constructing an iterator can require
a significantly larger amount of state construction. In particular our current BTreeMap
implementation suffers greatly due to its expensive iterator design (allocates a VecDeque for the
whole search path). A BTreeMap with parent pointers could potentially avoid any performance hit
modulo, but this is a more general problem for the *ordered map* API. There are surely types for
which a straight-up query will be cheaper than iterator initialization.

It is also significantly more ergonomic/discoverable to have `get_le_mut(&K)` over
`range_mut(Bound::Unbounded, Bound::Inclusive(&K)).next_back()`.




# Detailed design


The BTreeMap APIs are as follows:

get_(le|lt|gt|ge):

```rust
fn get_le<Q: ?Sized>(&self, &Q) -> Option<(&K, &V)>
    where K: Borrow<Q>;
```

get_(le|lt|gt|ge)_mut:

```rust
fn get_le_mut<Q: ?Sized>(&mut self, &Q) -> Option<(&K, &mut V)>
    where K: Borrow<Q>;
```

min|max:

```rust
fn min(&self) -> Option<(&K, &V)>;
```

(min|max)_mut:

```rust
fn min_mut(&mut self) -> Option<(&K, &mut V)>;
```


BTreeSet gets the equivalent APIs with the value part of the return removed.


Note that in contrast to `get` the key is yielded because the key that matched the query is
unquestionably new information for a querier. Also we don't want to have to add _keyed variants
of all these *later*.




# Drawbacks

Weep before the might of combinatorics, destroyer of API designs.




# Alternatives

## Use Bounds to unify inc/exc/extreme

```rust
fn pred(&self, Bound<&Q>)
fn succ(&self, Bound<&Q>)
fn pred_mut(&self, Bound<&Q>)
fn succ_mut(&self, Bound<&Q>)
```

where `pred(Unbounded)` is max, and `succ(Unbounded)` in min by assuming you're getting the
predecessor and successor of positive and negative infinity. This RFC does not propose this
API because it is in the author's opinion awful and would make our users cry.



## Use a custom enum to capture all variability

Take enums instead of having many methods:

```rust
enum Query<T> {
    Min,
    Lt(T),
    Le(T),
    Ge(T),
    Gt(T),
    Max,
}

impl<K, V> Map<K, V> {
    fn query<Q: ?Sized>(&self, query: Query<&Q>) -> Option<(&K, &V)>
       where K: Borrow<Q>;
       
    fn query_mut<Q: ?Sized>(&self, query: Query<&Q>) -> Option<(&K, &mut V)>
       where K: Borrow<Q>;
}
```

But this is just shuffling around the complexity, and making a more painful calling convention
that involves importing names:

```rust
let result = map.query(Query::Lt("hello"));
let result = map.query_mut(Query::Max);

// pulled in enum
let result = map.query(Lt("hello"));
let result = map.query_mut(Max);
```

vs

```rust
let result = map.get_lt("hello");
let result = map.max_mut();
```

##




# Unresolved questions

Nothing. This is pretty straightforward.
