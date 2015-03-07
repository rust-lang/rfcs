- Feature Name: inclusive_ranges
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Change the Range struct to allow all combinations of inclusive/exclusive ranges.

# Motivation

Regardless of how the range syntax will eventually work, the underlying data
structure will need to support both inclusive and exclusive ranges. If we don't
do this now, libraries will define their own way of specifying ranges (as the
rand crate and the BTreeMap collection have already done) and these custom
implementations may become entrenched. 

# Detailed design

The design is simply to:

  1. Remove the `Unbounded` variant of the `Bound<Idx>` enum and move it to the
     ops module.
  2. Change `Range` (and all variations there of) to use `Bound<Idx>` instead of
     `Idx` for start and end.

```rust
pub enum Bound<Idx> {
    Inclusive(Idx),
    Exclusive(Idx),
}
pub struct Range<Idx> {
    pub start: Bound<Idx>,
    pub end: Bound<Idx>,
}
pub struct RangeFrom<Idx> {
    pub start: Bound<Idx>,
}
pub struct RangeTo<Idx> {
    pub end: Bound<Idx>,
}
pub struct RangeFull;
```

# Drawbacks

* The Range struct becomes larger.
* When checking the bounds, you have to check if they are inclusive/exclusive.

# Alternatives

One obvious alternative is the following:

```rust
pub enum Bound<Idx> {
    Inclusive(Idx),
    Exclusive(Idx),
    Unbounded,
}
pub struct Range<Idx> {
    pub start: Bound<Idx>,
    pub end: Bound<Idx>,
}
```

However, this would make it impossible to do things like only allowing full
ranges (see slicing OsString).

# Unresolved questions

We might want some way to extract inclusive bounds from any *integral* range to
make bounds checking easier.
