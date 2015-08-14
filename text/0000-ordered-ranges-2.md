- Feature Name: ordered-ranges-2
- Start Date: 2015-08-13
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Replace `range(Bound::Excluded(&min), Bound::Included(&max))` with
`range().ge(&min).lt(&max)`.

Flesh out the general "ordered query" story.




# Motivation

There are roughly 3 classes of collection in Rust today to which the notion
of a "range" can apply:

* Unordered (Hash*)
* Sequential (Vec, VecDeque, LinkedList)
* Ordered (BTree*)

When you want to make queries on Unordered collections, it doesn't make sense
to talk about ranges of values.

When you want to make queries on Sequential collections, the `x..y` range
syntax is completely sufficient, even though it's confined to
`(inclusive, exclusive)`. If you want to fiddle with inclusivity of the bounds,
you can always just add 1 to either bound. This is because sequences are always
"keyed" by integers, which have a successor function. Also even for
ZSTs `usize::MAX` is never a valid index (`len` restricts us to storing
`usize::MAX` elements, and index `i` is item `i + 1`).

When you want to make queries on Ordered collections, things start to get
really complicated. All these collections require is that there is *some*
total ordering of the keys. There is no successor function to be had. Even if
there *were* a successor function, we *couldn't* and *shouldn't* use it.

We *couldn't* use such a function because we do not have the "`usize::MAX` is
never valid" assumption (a `BTreeSet<usize>` may contain `usize::MAX`). We also
*shouldn't* use such a function because it can be supremely expensive. For
instance, a BigInt could indeed support the API we want, but to get successors
would involve allocating an entire new BigInt, which is essentially a `Vec<u32>`
-- arbitrarily expensive.

Instead, we would like to instead enable the user to specify the inclusivity
of both bounds. This problem was "solved" by an earlier RFC via the `range`
iterator method, and the `Bound` enum. A `Bound` has the following definition:

```rust
enum Bound<T> {
    Included(T),
    Excluded(T),
    Unbounded,
}
```

Which is used with `range` as follows:

```rust
// Only import Bound:
let range = btree.range(Bound::Included(&min), Bound::Excluded(&max));

// With variants imported:
let range = btree.range(Included(&min), Excluded(&max));

// Open ended:
let range = btree.range(Unbounded, Excluded(&max));
```

It is this RFC's opinion that this is an awful API to use:

1. It requires importing an enum (and really, its variants for decency) that
  will *never* be in the prelude.

2. It then also requires you to pass your arguments in enums, which is a verbose
  and awkward calling convention.

3. It requires passing Unbounded as an explicit argument, rather than simply
  an omission as is the case for sequences (`..a`).



Whatever API we settle on is needed to handle `range`, `range_mut`, and `drain`.

However this also influences the ultimate API design we settle on for performing
ordered queries (min, max, successor, predecessor). [A previous rfc][ordered]
tried to introduce a large gamut of functions to the tune of `get_le_mut(&key)`
to address this issue, but was rejected due to its combinatoric nature, and the
fact that in principle, the `range` API already supports these queries by
constructing a range and then requesting an element.

However `btree.range(Unbounded, Included(&max)).next_back()` is arguably a very
confusing (and verbose!) way to get the inclusive predecessor of an `max`. Also
due to implementation details of std's BTree, this is an order of magnitude
slower than it needs to be. There is a rewrite underway that would in principle
solve this, but it does indicate that there are at very least collections
*in the abstract* that would incur serious performance penalties from only
exposing such an API.

`std::collections` specifies the general interfaces that all external collections
should endeavour to provide, so it would be unfortunate to force them to use
a suboptimal or non-standard API for this behaviour.





# Detailed design

The solution to all these problems is a *builder* API. You create a range,
you build the bounds on it, and then you convert into the thing you want.

A range starts out as (Unbounded, Unbounded). If the LHS is Unbounded, one can
call `gt` or `ge` on it to bound it. If the RHS is Unbounded, one can call
`lt` or `le` on it to bound it. It is statically impossible to bound anything
other than Unbounded.

Example usage:

```rust
// Make an iterator
let range = btree.range().gt(&min).le(&max).into_iter()

// In a loop:
for (k, v) in btree.range().gt(&min).le(&max) {
    // ...
}

// Excluding a bound:
for (k, v) in btree.range().lt(&max) {
    // ...
}

// Mutable
for (k, v) in btree.range_mut().ge(&min).lt(&max) {
    // ...
}

// Drain (note, this is the one case where no modifiers is "useful")
for (k, v) in btree.drain().ge(&min) {
    // ...
}
```

Compared to today (with everything maximally imported!):

```rust
// Full
let range = btree.range(Excluded(&min), Included(&max))

// In a loop:
for (k, v) in btree.range(Excluded(&min), Included(&max)) {
    // ...
}

// Excluding a bound:
for (k, v) in btree.range(Unbounded, Included(&max)) {
    // ...
}

// Mutable
for (k, v) in btree.range_mut(Included(&min), Excluded(&max)) {
    // ...
}

// Drain
// Doesn't exist
```


The signatures of everything is actually pretty hairy for today's impl:

```rust
// Note these are public for API reasons, but not intended for direct use
pub struct Unbounded<'a, Q: ?Sized + 'a>(PhantomData<&'a Q>);
pub struct Inclusive<'a, Q: ?Sized + 'a>(&'a Q);
pub struct Exclusive<'a, Q: ?Sized + 'a>(&'a Q);

pub struct Range<'a, K: 'a, V: 'a, L, R>(&'a BTreeMap<K, V>, L, R);

impl<'a, K, V, R> Range<'a, K, V, Unbounded<'a, K>, R> {
    pub fn gt<Q:?Sized>(self, min: &Q) -> Range<'a, K, V, Exclusive<Q>, R> {
        Range(self.0, Exclusive(min), self.2)
    }
    pub fn ge<Q:?Sized>(self, min: &Q) -> Range<'a, K, V, Inclusive<Q>, R> {
        Range(self.0, Inclusive(min), self.2)
    }
}
impl<'a, K, V, L> Range<'a, K, V, L, Unbounded<'a, K>> {
    pub fn lt<Q:?Sized>(self, max: &Q) -> Range<'a, K, V, L, Exclusive<Q>> {
        Range(self.0, self.1, Exclusive(max))
    }
    pub fn le<Q:?Sized>(self, max: &Q) -> Range<'a, K, V, L, Inclusive<Q>> {
        Range(self.0, self.1, Inclusive(max))
    }
}

impl<K: Ord, V> BTreeMap<K, V> {
    fn range(&self) -> Range<K, V, Unbounded<K>, Unbounded<K>> {
        Range(self, Unbounded(PhantomData), Unbounded(PhantomData))
    }
}

impl<'map, K, V, L, R> IntoIterator for Range<'map, K, V, L, R> {
    // Implementation specific bounds, see appendix
}
```

This would be duplicated with `&mut BTreeMap` as necessary for RangeMut and
Drain.



## Ordered queries

With this builder pattern, it is now easy to graft on the ordered query API
as different finalizers from `into_iter`: `min` and `max`

```rust
// get maximum key
btree.range().max()

// get_mut successor of `key`
btree.range_mut().ge(&key).min()

// remove predecessor of `min` if it's > `max`
btree.drain().gt(&min).le(&max).max()
```

With the general signature being

`fn min(self) -> Option<(K, V)>`

The reference-ness and mutability of `K` and `V` being determined by whether it's
a Range, RangeMut, or Drain.

Whether this is implemented by converting IntoIter or just hard-coding All The
Combinations is an implementation detail.

This resolves the issue of creating a full iterator being more expensive:
specialized search-only functionality can be provided here. Note that this is
*not* a problem specialization itself can solve. It would have to be able to realize
that you're only calling `next`/`next_back` once, and on a special configuration.

Although the btree rewrite would in principle nullify any performance gains
from this design for `min`/ `max`, it's less clear what the consequences of
the rewrite are for `drain().ge(&key).min()` which is an ordered remove.
See the appendix for details.

Note that this is a pure *addition* on top of the basic Range API, meaning
it can be accepted, rejected, or deferred separately. However it is the author's
belief that it is important that these two APIs be worked out together.






# Drawbacks

## Crazy API

This API is atrociously undiscoverable and unparseable. BTreeMap's docs will
need to basically fully describe the API in high-level terms, with the actual
API docs tucked away in a corner. Thankfully, it *will* be tucked away in a
corner, as it will be on the Range type. The RFC author is more than happy
to write these high-level docs, which should exist *anyway*.


## Double Everything for Sets

Sets would have to re-implement all of the builder methods, rather than just
mapping over the Map's iterator, as is done for everything else today. Although
the builders could of course just defer to the Map builder, all the methods
need to be re-declared.


## Oopsies

The builder pattern enables some confusing errors such as:

```rust
btree.range().lt(&min).ge(&max)
```

This is an empty range, because they got less/greater backwards. This is one
advantage of the current API: it's positional. That said, it's not clear how
common this error will be. It's also quite easy to do with the old API as
`range(Included(&max), Excluded(&min))`, especially since it's unlikely that
these bounds will be called min/max.

A lint might be desirable for not using lt/gt positionally.



## So much monomorph

This will generate a ton of monomorphized code for the builders that the current
Range API in principle avoids via enums. The builder stuff should be trivially
inlined away anyway, and the old API is already generic over `K, V, Q`.



## Referring to lt/ge/etc may be confusing

If you have a custom comparison then ordering may be reversed from the "actual"
ordering of the keys. The RFC's author also has a very academic background so
talking about "exclusivity"/"inclusivity" and "predecessor"/"sucessor". Feels
more natural. We could consider more human names like "from"/"after", "to"/"before",
but it's not clear that this really improves the situation.

~bikeshedding~



## Confusing Errors

Since so much functionality is conditional on certain state in the Range, you
can get a lot of confusing "method doesn't exist" errors. For instance if you
call `get` at the wrong time or try to `.lt().le()`







# Alternatives



## Bounded `..`

Instead of the builder pattern, just make a slight twist on `range`:

```rust
enum Bound<T> {
    Included(T),
    Excluded(T),
}

use std::collections::Bound::*;

let range = btree.range(..Excluded(&max))
```

This solves only issue 3, though. It also is abusing the semantics of `..` a
bit.

However it does avoid the positional confusion issues with the builders, and is
an intimately familiar API. The simple case for drain -- `btree.drain(..)` --
would require importing no enums, but the user would experience friction as soon
as they want to reach for more advanced functionality.

It's a bit more discoverable also, because its signature makes it clear that
it can be constrained, where one might see `.range()` and not realize it can
be modified.

If this implements IntoIterator, and is not simply an Iterator, it would also
enable the ordered query API to still be built on top of it.

This is a very strong candidate!




## Drain as an adapter on range_mut

Rather than `btree.drain().lt(&min).gt(&max)`, have
`btree.range_mut().lt(&min).gt(&max).drain()`. This in principle involves less
range builder boiler-plate, but at the cost of making `drain` more verbose and
"hidden" under range_mut, when really it should be first-class.

Also not clear if `.drain()` would produce an Interator. If it does, this would
remove the ability to do an ordered remove.



## `get` instead of `min` and `max`

`min` and `max` are maximally flexible, but most of the good bits can be
captured by a `get` method on Ranges that have exactly one constraint. If
that constraint is a lower bound, it would resolve to `min`. If it's an upper
bound, it would resolve to `max`:

```rust
// proposed: get_mut successor of `key`
btree.range_mut().ge(&key).min()

// alternative: get_mut successor of `key`
btree.range_mut().ge(&key).get()
```

This obviates the need to remember if you want `min` or `max` -- you just specify
the bound and ask for the first thing next to that bound inside the range.

However this removes this functionality:

```
// get maximum key
btree.range().max()

// remove predecessor of `min` if it's > `max`
btree.drain().gt(&min).le(&max).max()
```

The latter is arguably not that interesting. You can do the check yourself
easily, except for in the case of `drain`, which is destructive.
It's kind've nice to not have to support handling that.

This alternative would add inherent `min` and `max` methods to the BTreeMap
itself to compensate for the former.

It's however a bit confusing what's being "gotten". Why is the constrained bound
so special?



## Don't have `min` and `max` consume the range

They could all take `&self` or `&mut self` as appropriate, to enable query `min`
and `max`. This would require limiting the lifetimes of the references from
RangeMut to that of the RangeMut itself, and not the map. Also, having to
support being able to call `min` again could be very expensive for `Drain`. At
the limit, you should really just be using `into_iter`.

Finally, it's "free" to construct a range (it's lazy), so having to construct
it twice to query min and max isn't particularly arduous.







# Unresolved questions

This is largely addressed by other sections. There are API design tradeoffs
throughout.




# Appendix: Implementation Details

This RFC refers to some implementation details of BTreeMap. This section
elaborates on them, and also discusses how the guts of this functionality
would be implemented.



# Background: The impl of Today and Tomorrow

Today's BTreeMap is implemented *without* parent pointers. This is really nice
for modifying internal nodes, because it means you don't need to descend into
all of the node's children and modify their parent pointers. Great for caching
(because every descent would presumably be a miss).

However this comes at the cost of having to maintain a *stack* of all the nodes
that were traversed on the way to the node in order to ascend up the tree.
There are 3 cases today where this stack is necessary:

* insert
* remove
* iteration

All of these operations require ascending in the tree. As such, all of these
operations are penalized with the cost of allocating and maintaining a stack.
In principle, the allocation could be avoided by determining a maximum depth and
statically allocating an array of that size. For a BTree this may not be so bad,
since it's very balanced and has a pretty high branching rate. 32 would probably
be sufficient. This of course means that BTree iterators would be huge.

However [Google's BTree][cpp-btree] demonstrates that superior performance can
be had by simply using parent pointers. The conventional wisdom of not using
parent pointers in BTrees is derived from the back-ground as *external memory*
data structures, where a descent isn't just a cache miss to RAM, it's a cache
miss to Disk! Also external memory BTrees tend to have a far higher branching
factor (you want a node to fit on but fill a page).

Basically, for an external memory btree, the cost of hitting the disk dominates
everything else so completely that it makes sense to pay any cost to avoid it.
For an in memory btree, hitting memory (or maybe just not the L1 cache) isn't
nearly as costly, so allocating search stacks or manipulating large nodes is not
as obviously worth it. Indeed a value of `B ~= 6` (5-13 children) has generally
been measured to be as good as we can do for our implementation.

This rewrite is under way. Once it is done, all of these methods could ditch
the explicit stacks in favour of a single handle into a node. This has the
advantage of also making everything a little more *compositional*, as you can
just have the logic for finding a handle, and then figure out what to do with
that handle independently. The current design of BTreeMap endeavours to do this
a bit, but there's currently a schism between lookup and insert/remove/iter due
to the need to build a stack.

*However*, even with the handle code, it is the author's understanding that there
is no decent way to describe all of eq/gt/ge/lt/le/min/max compactly. They all
have subtle logical quirks. See the [ordered query impl PR][ordered-pr] for
details on this. Of course, `<` and `>` are totally symmetric, but that just
means they can be described identically. You still need separate final code.

The handle code just means not having to duplicate this logic for "needs a stack"
and "doesn't need a stack". If the handles use raw pointers, it may also be
possible to eliminate any duplication for `&` vs `&mut`. How to interpret the
handle can be specialized by the consumer of the handle. Note that a handle
is not just a pointer, it's a (pointer, index) pair. Parent pointers would
also be of this form (which is why they would need more fixups).

Note that the absence of parent pointers was *never* intended to make BTree any
safer. BTree still has to pervasively utilize raw pointers to maintain the
search stack, and related book-keeping.

Also note that while the parent pointers will bloat up the nodes a bit, the nodes
are already bloated with storing `B` and their `len` as full usizes. A previous PR
to make `B` a global const and shrink down the `len` found no performance gains
(it was closed pending more work, and was simply forgotten about). It is
possible to add parent pointers without incurring any additional bloat by
replacing `B` with the parent pointer, and storing the parent index and len as
`u8`'s (more than enough).




# Implementing the proposed Range API

(Note: this is totally agnostic to Builder vs `..`)

With today's API, the new API would basically have to defer to the current
API (which would become an implementation detail). Here is a complete
implementation of the basic Range and RangeMut that works outside std today:

https://play.rust-lang.org/?gist=4c4bcaa7e093af911d8a&version=stable

```rust
#![feature(collections, collections_bound, btree_range)]

use std::collections::Bound;
use std::borrow::Borrow;
use std::collections::btree_map::{self, BTreeMap};
use std::marker::PhantomData;

pub struct Unbounded<'a, Q: ?Sized + 'a>(PhantomData<&'a Q>);
pub struct Inclusive<'a, Q: ?Sized + 'a>(&'a Q);
pub struct Exclusive<'a, Q: ?Sized + 'a>(&'a Q);

trait IntoBound<'a> {
    type Bound: ?Sized + 'a;
    fn into_bound(self) -> Bound<&'a Self::Bound>;
}

impl<'a, Q: ?Sized + 'a> IntoBound<'a> for Inclusive<'a, Q> {
    type Bound = Q;
    fn into_bound(self) -> Bound<&'a Self::Bound> { Bound::Included(self.0) }
}
impl<'a, Q: ?Sized + 'a> IntoBound<'a> for Exclusive<'a, Q> {
    type Bound = Q;
    fn into_bound(self) -> Bound<&'a Self::Bound> { Bound::Excluded(self.0) }
}
impl<'a, Q: ?Sized + 'a> IntoBound<'a> for Unbounded<'a, Q> {
    type Bound = Q;
    fn into_bound(self) -> Bound<&'a Self::Bound> { Bound::Unbounded }
}






// Range

trait Ranged<K, V> {
    fn ranged(&self) -> Range<K, V, Unbounded<K>, Unbounded<K>>;
}

impl<K, V> Ranged<K, V> for BTreeMap<K, V> {
    fn ranged(&self) -> Range<K, V, Unbounded<K>, Unbounded<K>> {
        Range(self, Unbounded(PhantomData), Unbounded(PhantomData))
    }
}

pub struct Range<'a, K: 'a, V: 'a, L, R>(&'a BTreeMap<K, V>, L, R);

impl<'a, K, V, R> Range<'a, K, V, Unbounded<'a, K>, R> {
    pub fn gt<Q:?Sized>(self, min: &Q) -> Range<'a, K, V, Exclusive<Q>, R> {
        Range(self.0, Exclusive(min), self.2)
    }
    pub fn ge<Q:?Sized>(self, min: &Q) -> Range<'a, K, V, Inclusive<Q>, R> {
        Range(self.0, Inclusive(min), self.2)
    }
}
impl<'a, K, V, L> Range<'a, K, V, L, Unbounded<'a, K>> {
    pub fn lt<Q:?Sized>(self, max: &Q) -> Range<'a, K, V, L, Exclusive<Q>> {
        Range(self.0, self.1, Exclusive(max))
    }
    pub fn le<Q:?Sized>(self, max: &Q) -> Range<'a, K, V, L, Inclusive<Q>> {
        Range(self.0, self.1, Inclusive(max))
    }
}

impl<'map, K, V, L, R> IntoIterator for Range<'map, K, V, L, R>
    where L: IntoBound<'map> + 'map,
          R: IntoBound<'map> + 'map,
          L::Bound: Ord + 'map,
          R::Bound: Ord + 'map,
          K: Ord + Borrow<L::Bound> + Borrow<R::Bound>,
{
    type IntoIter = btree_map::Range<'map, K, V>;
    type Item = (&'map K, &'map V);

    fn into_iter(self) -> Self::IntoIter {
        let Range(map, l, r) = self;
        map.range(l.into_bound(), r.into_bound())
    }
}




// RangeMut

trait RangedMut<K, V> {
    fn ranged_mut(&mut self) -> RangeMut<K, V, Unbounded<K>, Unbounded<K>>;
}

impl<K, V> RangedMut<K, V> for BTreeMap<K, V> {
    fn ranged_mut(&mut self) -> RangeMut<K, V, Unbounded<K>, Unbounded<K>> {
        RangeMut(self, Unbounded(PhantomData), Unbounded(PhantomData))
    }
}

pub struct RangeMut<'a, K: 'a, V: 'a, L, R>(&'a mut BTreeMap<K, V>, L, R);

impl<'a, K, V, R> RangeMut<'a, K, V, Unbounded<'a, K>, R> {
    pub fn gt<Q:?Sized>(self, min: &Q) -> RangeMut<'a, K, V, Exclusive<Q>, R> {
        RangeMut(self.0, Exclusive(min), self.2)
    }
    pub fn ge<Q:?Sized>(self, min: &Q) -> RangeMut<'a, K, V, Inclusive<Q>, R> {
        RangeMut(self.0, Inclusive(min), self.2)
    }
}
impl<'a, K, V, L> RangeMut<'a, K, V, L, Unbounded<'a, K>> {
    pub fn lt<Q:?Sized>(self, max: &Q) -> RangeMut<'a, K, V, L, Exclusive<Q>> {
        RangeMut(self.0, self.1, Exclusive(max))
    }
    pub fn le<Q:?Sized>(self, max: &Q) -> RangeMut<'a, K, V, L, Inclusive<Q>> {
        RangeMut(self.0, self.1, Inclusive(max))
    }
}

impl<'map, K, V, L, R> IntoIterator for RangeMut<'map, K, V, L, R>
    where L: IntoBound<'map> + 'map,
          R: IntoBound<'map> + 'map,
          L::Bound: Ord + 'map,
          R::Bound: Ord + 'map,
          K: Ord + Borrow<L::Bound> + Borrow<R::Bound>,
{
    type IntoIter = btree_map::RangeMut<'map, K, V>;
    type Item = (&'map K, &'map mut V);

    fn into_iter(self) -> Self::IntoIter {
        let RangeMut(map, l, r) = self;
        map.range_mut(l.into_bound(), r.into_bound())
    }
}







fn main() {
    let mut map = BTreeMap::new();
    map.insert(0, 0);
    map.insert(10, 10);
    map.insert(20, 20);

    for (k, v) in map.ranged().lt(&20) {
        println!("{} {}", k, v);
    }

    for (k, v) in map.ranged_mut().ge(&10).lt(&30) {
        println!("{} {}", k, v);
    }
}
```

The *extended* API would have to defer to essentially the implementations
proposed by the [previous implementation PR][ordered-pr] (the author did
not bother to proof this out beyond verifying `get` could be forwarded to).

With a handle-based API, instead of dispatching to a (trivial) IntoBound trait,
we would instead dispatch to an IntoHandle trait which does the full search for
the node. Handles point between elements, so that the next or previous one can
be requested (and to encode the start and end of the tree).

`min` and `max` would just resolve that handle into a `(K, V)` pair
by telling it to `(get|get_mut|read)_(prev|next) as appropriate, while `into_iter`
would stuff the two handles into an iterator struct which would know how to
drive them appropriately. If the two handles are equal, then the range is empty.

This would all be unsafe, but BTree is full of unsafe code already (though we
try pretty hard -- perhaps too hard -- to minimize the scope of it today).



[ordered]: https://github.com/rust-lang/rfcs/pull/1195
[ordered-pr]: https://github.com/rust-lang/rust/pull/27135
[cpp-btree]: https://code.google.com/p/cpp-btree/
