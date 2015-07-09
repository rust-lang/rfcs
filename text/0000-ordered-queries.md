- Feature Name: ordered-queries
- Start Date: 2015-09-25
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Add the following to BTreeMap

* pred_inc
* pred_exc
* succ_inc
* succ_exc
* first
* last
* pred_inc_mut
* pred_exc_mut
* succ_inc_mut
* succ_exc_mut
* first_mut
* last_mut

and to BTreeSet:

* pred_inc
* pred_exc
* succ_inc
* succ_exc
* first
* last




# Motivation

Currently the only option people have to do ordered queries on a BTreeMap is to make a full-blown
`range` iterator with carefully selected bounds. However constructing an iterator can require
a significantly larger amount of state construction. In particular our current BTreeMap
implementation suffers greatly due to its expensive iterator design (allocates a VecDeque for the
whole search path). A BTreeMap with parent pointers could potentially avoid any performance hit
modulo, but this is a more general problem for the *ordered map* API. There are surely types for
which a straight-up query will be cheaper than iterator initialization.

It is also siginificantly more ergonomic/discoverable to have `pred_inc_mut(&K)` over
`range_mut(Bound::Unbounded, Bound::Inclusive(&K)).next_back()`.




# Detailed design


The BTreeMap APIs are as follows:

(pred|succ)_(inc|exc):

```rust
fn pred_inc<Q: ?Sized>(&self, &Q) -> Option<(&K, &V)>
    where K: Borrow<Q>;
```

(pred|succ)_(inc|exc)_mut:

```rust
fn pred_inc_mut<Q: ?Sized>(&mut self, &Q) -> Option<(&K, &mut V)>
    where K: Borrow<Q>;
```

first|last:

```rust
fn first(&self) -> Option<(&K, &V)>;
```

(first|last)_mut:

```rust
fn first_mut(&mut self) -> Option<(&K, &mut V)>;
```


BTreeSet gets the equivalent APIs with the value part of the return removed.


Note that in contrast to `get` the key is yielded because the key that matched the query is
unquestionably new information for a querier. Also we don't want to have to add _keyed variants
of all these *later*.




# Drawbacks

Weep before the might of combinatorics, destroyer of API designs.




# Alternatives

```
fn pred(&self, Bound<Q>)
fn succ(&self, Bound<Q>)
fn pred_mut(&self, Bound<Q>)
fn succ_mut(&self, Bound<Q>)
```

where `pred(Unbounded)` is max, and `succ(Unbounded)` in min by assuming you're getting the
predecessor and successor of positive and negative infinity. This RFC does not propose this
API because it is crazy-pants and would make our users cry.

--------

We could also give `&mut` access to the keys in the `_mut` variants. This would enable
changing "unimportant" information in the keys without resorting to interior mutability
mechanisms. It would allow BTreeSet to have _mut variants of all these methods. This RFC
does not propose this because it's probably a really big footgun while also being quite niche.





# Unresolved questions

Nothing. This is pretty straightforward.
