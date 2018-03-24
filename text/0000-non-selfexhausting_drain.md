- Feature Name: non-selfexhausting_drain
- Start Date:
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add `*_nonexhausting()` variants for every `drain()` that do not eagerly consume residual items on drop of the `DrainNonexhausting` struct.

# Motivation
[motivation]: #motivation

The `drain` API is a specialized operation that combines two unrelated tasks:
1. Moving elements out of a collection without consuming it
2. Clearing the range or the entire collection, regardless of iteration

You could call it `drain_clearing`. The forced consumption isn't necessary for a safe drain nor is it necessarily faster. Because of this coupling, there is currently no efficient way of moving a subset of elements out while keeping the collection, when one does not know in advance how many elements to remove.

The `drain_filter` methods recognize the need for selective removal by allowing on-the-fly decisions.
However, `DrainFilter`, too, will eagerly exhaust itself on drop with no way of stopping.
Excess elements can be kept by hacking some state awareness into the conditional closure and always returning `false` after some point, but this is both unnecessary computation and tedious for the programmer.

More generally speaking, it's uncharacteristic for an iterator to behave in this (semi-)eager fashion by default.
`drain` is stable and so cannot be changed, but we should have a conforming iterator.
The behaviour of a clearing drain can be gained by the combination of two more generalized and orthogonal APIs:
* The non-selfexhausting drain proposed in this RFC
* An iterator adapter for self-exhaustion as proposed [here](https://github.com/Emerentius/rfcs/blob/selfexhausting_iter_adapter/text/0000-selfexhausting_iter_adapter.md).

Although with less leakage on panics, as the repair code will still be called if a panic occurs during self-exhaustion.

```rust
// take only what's needed
for element in dont_waste_me.drain_nonexhausting(..) {
    /* do stuff */
    if condition {
        break
    }
}

let cherrypicked = vec.drain_filter_nonexhausting(condition)
    .take(10)
    .collect();
```

# Implementation

With a non-selfexhausting drain, the collection's internal structure needs to be repaired afterwards.
This is already required for `drain_filter()`.
As mentioned in the Motivation, `drain_filter_nonexhausting` can be emulated with `drain_filter(condition)` by returning `false` from `condition` for every element after some point. The regular `drain` can be emulated with `drain_filter(|_| true)`. Therefore, any collection for which `drain_filter` can exist, can also have nonexhausting drains with small adaptions.
Given that no new challenges arise, this RFC doesn't lay out a detailed plan for the implementation.

Several collections are still lacking `drain` and/or `drain_filter`, but there is a [desire to add them](https://github.com/rust-lang/rfcs/issues/2140). In their case, `drain_filter_nonexhausting()` should be implemented first as a basis for the other drains.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation
`drain_nonexhausting` is like `drain` but does not remove items from the collection that were not consumed through the iterator.
The difference between `drain_filter_nonexhausting` and `drain_filter` is the same.

# Drawbacks
[drawbacks]: #drawbacks
* Additional API surface. Internally a lot of code can be reused.

# Rationale and alternatives
[alternatives]: #alternatives
* Instead of putting methods on the collections directly, an adapter can be provided for `Drain*` structs that changes its behaviour on drop. This lowers the number of methods on the collections directly, but would be less discoverable. It requires that `Drain` and `DrainNonexhausting` iterate in identical fashion and only differ on `Drop`. This is probably not an issue and our current drains could internally use `DrainNonexhausting`.

* Make `drain_filter` nonexhaustive and don't add any `_nonexhausting` variants at all. This minimizes API surface, but the discrepancy between `drain` and `drain_filter` will be surprising.

* The proposed name is chosen for its symmetry to the `.exhausting()` adapter proposed [here](https://github.com/Emerentius/rfcs/blob/selfexhausting_iter_adapter/text/0000-selfexhausting_iter_adapter.md)
  It could also be called `*_lazy` or `*_lazy_drop`. The `lazy` part may be confusing because the iterator is already lazy apart from `Drop`. Bikeshedding welcome afterwards.

# Prior art
[prior-art]: #prior-art

The drain API was first proposed in not much detail in the [Collections Reform Part 2](https://github.com/rust-lang/rfcs/pull/509).
In the [RFC 574](https://github.com/rust-lang/rfcs/pull/574) the details were worked out and the current semantics proposed. [RFC 570] proposed the same functionality a bit earlier for `String` under the name `remove_range` without the iterator part. It wasn't accepted in favor of RFC 574. [RFC 1257](https://github.com/rust-lang/rfcs/pull/1257) expanded `drain()` to more collections.

The decision to make these APIs a `remove_range()` first, iterator second, wasn't questioned in any of the mentioned RFCs nor in their tracking issues. When `drain_filter()` was introduced it followed `drain()`'s example and was implemented as `retain_mut()` first, iterator second. If there is a discussion of this design choice somewhere in the issue trackers, I didn't find it.

# Unresolved questions
[unresolved]: #unresolved-questions
