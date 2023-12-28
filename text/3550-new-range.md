- Feature Name: `new_range`
- Start Date: 2023-12-18
- RFC PR: [rust-lang/rfcs#3550](https://github.com/rust-lang/rfcs/pull/3550)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Change the range operators `a..b`, `a..`, and `a..=b` to resolve to new types `ops::range::Range`, `ops::range::RangeFrom`, and `ops::range::RangeInclusive` in Edition 2024. These new types will not implement `Iterator`, instead implementing `Copy` and `IntoIterator`.

# Motivation
[motivation]: #motivation

The current iterable range types ([`Range`], [`RangeFrom`], [`RangeInclusive`]) implement `Iterator` directly. This is now widely considered to be a mistake, because it makes implementing `Copy` for those types hazardous due to how the two traits interact.

```rust
for x in it.take(3) {  // a *copy* of the iterator is used here
    // ..
}

match it.next() {  // the original iterator (not advanced) is used here
    // ..
}
```

[`Range`]: https://doc.rust-lang.org/stable/core/ops/struct.Range.html
[`RangeFrom`]: https://doc.rust-lang.org/stable/core/ops/struct.RangeFrom.html
[`RangeInclusive`]: https://doc.rust-lang.org/stable/core/ops/struct.RangeInclusive.html

However, there is considerable demand for `Copy` range types for multiple reasons:
- ergonomic use without needing explicit `.clone()`s or rewriting the `a..b` syntax repeatedly
- use in `Copy` types (currently people work around this by using a tuple instead)

Another primary motivation is the extra size of `RangeInclusive`. It uses an extra `bool` field to keep track of when the upper bound has been yielded by the iterator, but this extra size is useless when the type is not used as an iterator.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Rust has several different types of "range" syntax, including the following:

- `a..b` denotes a range from `a` (inclusive) to `b` (exclusive). It resolves to the type `std::ops::range::Range`.  
  The iterator for `Range` will yield values from `a` (inclusive) to `b` (exclusive) in steps of one.

- `a..=b` denotes a range from `a` (inclusive) to `b` (inclusive). It resolve to the type `std::ops::range::RangeInclusive`.
  The iterator for `RangeInclusive` will yield values from `a` (inclusive) to `b` (inclusive) in steps of one.

- `a..` denotes a range from `a` (inclusive) with no upper bound. It resolves to the type `std::ops::range::RangeFrom`.  
  The iterator for `RangeFrom` will yield values starting with `a` and increasing in steps on one.

These types implement the `IntoIterator` trait, enabling their use directly in a `for` loop:
```rust
for n in 0..5 {
    // `n` = 0, 1, 2, 3, 4
}
```

All range types are `Copy` when the bounds are `Copy`, allowing easy reuse:
```rust
let range = 0..5;

if a_slice[range].contains(x) {
    // ...
}
if b_slice[range].contains(y) {
    // ...
}
```

For convenience, several commonly-used methods from `Iterator` are present as inherent functions on the range types:
```rust
for n in (1..).map(|x| x * 2) {
    // n = 2, 4, 6, 8, 10, ...
}
for n in (0..5).rev() {
    // n = 4, 3, 2, 1, 0
}
```

## Legacy Range Types

In Rust editions prior to 2024, `a..b`, `a..=b`, and `a..` resolved to a different set of types (now found in `std::ops::range::legacy`). These legacy range types did not implement `Copy`, and implemented `Iterator` directly (rather than `IntoIterator`).

This meant that any `Iterator` method could be called on those range types:
```rust
let mut range = 0..5;
assert_eq!(range.next(), Some(0));
range.for_each(|n| {
    // n = 1, 2, 3, 4
});
```

There exist `From` impls for converting from the new range types to the legacy range types.

### Migrating

`cargo fix --edition 2024` can handle migrating many use cases to the new range types. This includes most places where a `RangeBounds` or `Iterator` is expected:
```rust
pub fn takes_range(range: impl std::ops::RangeBounds<usize>) { ... }
takes_range(0..5); // No changes necessary
```

And most places where `Iterator` methods were used directly on the range:
```rust
for n in (0..5).rev() { ... } // No changes necessary

// Before
(0..5).for_each(...);
// After
(0..5).into_iter().for_each(...); // Add `.into_iter()`
```

But it is impossible to generally handle everything, and there are cases which fall back to converting to the legacy types:

```rust
// Before
pub fn takes_range(range: std::ops::Range<usize>) { ... }
takes_range(0..5);
// After
pub fn takes_range(range: std::ops::range::legacy::Range<usize>) { ... }
takes_range(std::ops::range::legacy::Range::from(0..5));
```

To reduce the need for conversions, we recommend the following:

- Change any function parameters from legacy `Range*` types to `impl RangeBounds<_>`  
  Or where `RangeBounds` is not applicable, `impl Into<Range*>`

```rust
// Before
pub fn takes_range(range: std::ops::Range<usize>) { ... }

// After
pub fn takes_range(range: std::ops::RangeBounds<usize>) { ... }
// Or
pub fn takes_range(range: impl Into<std::ops::Range<usize>>) { ... }
```

- Change any trait bounds that assume `Range*: Iterator` to use `IntoIterator` instead  
  This is fully backwards-compatible, thanks to the [blanket `impl<I: Iterator> IntoIterator for I`](https://doc.rust-lang.org/stable/std/iter/trait.IntoIterator.html#impl-IntoIterator-for-I)

```rust
pub struct Wrapper<T> {
    range: Range<T>
};

// Before
impl<T> IntoIterator for Wrapper<T>
where Range<T>: Iterator
{
    type Item = <Range<T> as Iterator>::Item;
    type IntoIter = Range<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.range
    }
}
// After
impl<T> IntoIterator for Wrapper<T>
where Range<T>: IntoIterator
{
    type Item = <Range<T> as IntoIterator>::Item;
    type IntoIter = <Range<T> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.range.into_iter()
    }
}
```

- When the range is treated as mutable iterator, call `.into_iter()` before using it

```rust
// Before
let mut range = 0..5;
assert_eq!(range.next(), Some(0));
range.for_each(|n| {
    // n = 1, 2, 3, 4
});

// After
let mut range = (0..5).into_iter();
assert_eq!(range.next(), Some(0));
range.for_each(|n| {
    // n = 1, 2, 3, 4
});
```

## Diagnostics

There is a substantial amount of educational material in the wild which assumes the the range types implement `Iterator`. If a user references this outdated material, it is important that compiler errors guide them to the new solution.

```
error[E0599]: `Range<usize>` is not an iterator
 --> src/main.rs:4:7
  |
4 |     a.sum()
  |       ^^^ `Range<usize>` is not an iterator
  |
  = note: the Edition 2024 range types implement `IntoIterator`, not `Iterator`
  = help: convert to an iterator first: `a.into_iter().sum()`
  = note: the following trait bounds were not satisfied:
          `Range<usize>: Iterator`
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Add replacement types only for the current `Range`, `RangeFrom`, and `RangeInclusive`.

The [**Range Expressions** page in the Reference](https://doc.rust-lang.org/reference/expressions/range-expr.html) will change to read as follows

> ## Edition 2024 and later
>
> The `..` and `..=` operators will construct an object of one of the `std::ops::range::Range` (or `core::ops::range::Range`) variants, according to the following table:
> 
> | Production             | Syntax        | Type                         | Range                 |
> |------------------------|---------------|------------------------------|-----------------------|
> | _RangeExpr_            | start`..`end  | std::ops::range::Range            | start &le; x &lt; end |
> | _RangeFromExpr_        | start`..`     | std::ops::range::RangeFrom        | start &le; x          |
> | _RangeToExpr_          | `..`end       | std::ops::range::RangeTo          |            x &lt; end |
> | _RangeFullExpr_        | `..`          | std::ops::range::RangeFull        |            -          |
> | _RangeInclusiveExpr_   | start`..=`end | std::ops::range::RangeInclusive   | start &le; x &le; end |
> | _RangeToInclusiveExpr_ | `..=`end      | std::ops::range::RangeToInclusive |            x &le; end |
> 
> **Note:** While `std::ops::RangeTo`, `std::ops::RangeFull`, and `std::ops::RangeToInclusive` are re-exports of `std::ops::range::RangeTo`, `std::ops::range::RangeFull`, and `std::ops::Range::RangeToInclusive` respectively, `std::ops::Range`, `std::ops::RangeFrom`, and `std::ops::RangeInclusive` are re-exports of the types under `std::ops::range::legacy::` (NOT those directly under `std::ops::range::`) for backwards-compatibility reasons.
> 
> Examples:
> 
> ```rust
> 1..2;   // std::ops::range::Range
> 3..;    // std::ops::range::RangeFrom
> ..4;    // std::ops::range::RangeTo
> ..;     // std::ops::range::RangeFull
> 5..=6;  // std::ops::range::RangeInclusive
> ..=7;   // std::ops::range::RangeToInclusive
> ```
>
> The following expressions are equivalent.
> 
> ```rust
> let x = std::ops::range::Range {start: 0, end: 10};
> let y = 0..10;
> 
> assert_eq!(x, y);
> ```
>
> ## Prior to Edition 2024
>
> The `..` and `..=` operators will construct an object of one of the `std::ops::range::legacy::Range` (or `core::ops::range::legacy::Range`) variants, according to the following table:
> 
> | Production             | Syntax        | Type                         | Range                 |
> |------------------------|---------------|------------------------------|-----------------------|
> | _RangeExpr_            | start`..`end  | std::ops::range::legacy::Range            | start &le; x &lt; end |
> | _RangeFromExpr_        | start`..`     | std::ops::range::legacy::RangeFrom        | start &le; x          |
> | _RangeToExpr_          | `..`end       | std::ops::range::RangeTo          |            x &lt; end |
> | _RangeFullExpr_        | `..`          | std::ops::range::RangeFull        |            -          |
> | _RangeInclusiveExpr_   | start`..=`end | std::ops::range::legacy::RangeInclusive   | start &le; x &le; end |
> | _RangeToInclusiveExpr_ | `..=`end      | std::ops::range::RangeToInclusive |            x &le; end |
>
> **Note:** `std::ops::Range`, `std::ops::RangeFrom`, and `std::ops::RangeInclusive` are re-exports of the respective types under `std::ops::range::legacy::`. `std::ops::RangeTo`, `std::ops::RangeFull`, and `std::ops::RangeToInclusive` are re-exports of the respective types under `std::ops::range::`.
>
> Examples:
> 
> ```rust
> 1..2;   // std::ops::range::legacy::Range
> 3..;    // std::ops::range::legacy::RangeFrom
> ..4;    // std::ops::range::RangeTo
> ..;     // std::ops::range::RangeFull
> 5..=6;  // std::ops::range::legacy::RangeInclusive
> ..=7;   // std::ops::range::RangeToInclusive
> ```
> 
> The following expressions are equivalent.
> 
> ```rust
> let x = std::ops::range::legacy::Range {start: 0, end: 10};
> // Or: let x = std::ops::Range {start: 0, end: 10};
> let y = 0..10;
> 
> assert_eq!(x, y);
> ```

## New paths

There is no language support for edition-dependent path resolution, so these types must continue to be accessible under their current paths. However, their canonical paths will change to live under `ops::range::legacy`:

- `ops::Range` will be a re-export of `ops::range::legacy::Range`
- `ops::RangeFrom` will be a re-export of `ops::range::legacy::RangeFrom`
- `ops::RangeInclusive` will be a re-export of `ops::range::legacy::RangeFrom`

In order to not break existing links to the documentation for these types, the re-exports must remain `doc(inline)`.

The replacement types will live under `ops::range`:

- `ops::range::Range` will be the Edition 2024 replacement for `ops::range::legacy::Range`
- `ops::range::RangeFrom` will be the Edition 2024 replacement for `ops::range::legacy::RangeFrom`
- `ops::range::RangeInclusive` will be the Edition 2024 replacement for `ops::range::legacy::RangeFrom`

The `RangeFull`, `RangeTo`, and `RangeToInclusive` types will remain unchanged. But for consistency, their canonical paths will be changed to live under `ops::range`:

- `ops::RangeFull` will be a re-export of `ops::range::RangeFull`
- `ops::RangeTo` will be a re-export of `ops::range::RangeTo`
- `ops::RangeToInclusive` will be a re-export of `ops::range::RangeToInclusive`

## Iterator types

Because the three new types will implement `IntoIterator` directly, they need three new respective `IntoIter` types:

- `ops::range::IterRange` will be `<ops::range::Range<_> as IntoIterator>::IntoIter`
- `ops::range::IterRangeFrom` will be `<ops::range::RangeFrom<_> as IntoIterator>::IntoIter`
- `ops::range::IterRangeInclusive` will be `<ops::range::RangeInclusive<_> as IntoIterator>::IntoIter`

These iterator types will implement the same iterator traits (`DoubleEndedIterator`, `FusedIterator`, etc) as the legacy range types, with the following exceptions:
- `ops::range::IterRange` will not implement `ExactSizeIterator` for `u32` or `i32`
- `ops::range::IterRangeInclusive` will not implement `ExactSizeIterator` for `u16` or `i16`

Those `ExactSizeIterator` impls on the legacy range types are [known to be incorrect](https://github.com/rust-lang/rust/blob/495203bf61efabecc2c460be38e1eb0f9952601b/library/core/src/iter/range.rs#L903-L936).

These iterator types should each feature an associated function for getting the remaining range back:

```rust
impl<Idx> IterRange<Idx> {
    pub fn remainder(self) -> Range<Idx>;
}
impl<Idx> IterRangeFrom<Idx> {
    pub fn remainder(self) -> RangeFrom<Idx>;
}
impl<Idx> IterRangeInclusive<Idx> {
    // `None` if the iterator was exhausted
    pub fn remainder(self) -> Option<RangeInclusive<Idx>>;
}
```

## Changed structure and API

`ops::range::Range` and `ops::range::RangeFrom` will have identical structure to the existing types, with public fields for the bounds. However, `ops::range::RangeInclusive` will be changed:
- `start` and `end` will be changed to public fields
- `exhausted` field will be removed entirely

This makes the new `RangeInclusive` the same size as `Range`.

All three new types will have the same trait implementations as the legacy types, with the following exceptions:
- NOT implement `Iterator`
- implement `IntoIterator` directly (when `Idx: Step`)
- implement `Copy` (when `Idx: Copy`)

The following conversions between the new and legacy types will be implemented:
```rust
impl<Idx> From<range::Range<Idx>> for range::legacy::Range<Idx>
impl<Idx> From<range::RangeFrom<Idx>> for range::legacy::RangeFrom<Idx>
impl<Idx> From<range::RangeInclusive<Idx>> for range::legacy::RangeInclusive<Idx>

impl<Idx> From<range::legacy::Range<Idx>> for range::Range<Idx>
impl<Idx> From<range::legacy::RangeFrom<Idx>> for range::RangeFrom<Idx>
// Fallible because legacy RangeInclusive can be exhausted
impl<Idx> TryFrom<range::legacy::RangeInclusive<Idx>> for range::RangeInclusive<Idx>
```

# Drawbacks
[drawbacks]: #drawbacks

This change has the potential to cause a significant amount of churn in the ecosystem. The largest source of this churn will be cases where ranges are assumed to be `Iterator`. While experimenting with implementing the library part of these changes, I 

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Just implement `Copy` on the types as-is

`Copy` iterators are a large footgun. It was decided to [remove `Copy` from all iterators back in 2015](https://github.com/rust-lang/rust/pull/21809), and that decision is unlikely to be reversed.

Another spin on this is to specialize `IntoIterator` on these range types and lint whenever the `Iterator` impl is used. However, that would likely be a breaking change and this kind of specialization may never be stabilized (and might not even be sound).

Neither of these approaches would resolve the issue of `RangeInclusive` being larger than necessary for range purposes.

## Name the new types something besides `Range`

We could choose to introduce these new types with a name other than `Range`. Some alternatives that have been proposed:
- Interval
- Span
- Bounds

We believe that it is best to keep the `Range` naming for several reasons:
- Existing `Range*` types that implement `Copy` and not `Iterator` that won't be touched by this change
- Large amount of legacy educational material and code using the `Range` naming
- It's best to match the name of the syntax (["range expressions"](https://doc.rust-lang.org/reference/expressions/range-expr.html))

## Use legacy range types as the iterators for the new range types

We could choose to make `new_range.into_iter()` resolve to a legacy range type. This would reduce the number of new types we need to add to the standard library.

But the legacy range types have a much larger API surface than other `Iterator`s in the standard library, which typically only implement the various iterator traits and maybe have a `remainder` method. This can reduce optimization potential for the iterators, possibly limiting performance.

# Prior art
[prior-art]: #prior-art

The [copy-range](https://docs.rs/copy-range) crate provides types similar to those proposed here.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

The set of inherent methods copied from `Iterator` present on the new range types is left open for the **libs-api** team to decide after this proposal is accepted and before stabilization.

- Should other range-related items (like `RangeBounds`) also be moved under `std::ops::range`?
- Should `RangeFrom` even implement `IntoIterator`, or should it require an explicit `.iter()` call? Using it as an iterator [can be a footgun](https://github.com/rust-lang/libs-team/issues/304), usually people want `start..=MAX` instead.
- Should there be a way to get an iterator that modifies the range in place, rather than taking the range by value? That would allow things like `range.by_ref().next()`.
- Should there be an infallible conversion from legacy to new `RangeInclusive`?
```rust
impl<Idx> From<legacy::RangeInclusive<Idx>> for RangeInclusive<Idx> {
    // How do we handle the `exhausted` case, set `end < start`?
}
```

# Future possibilities
[future-possibilities]: #future-possibilities

- Hide or deprecate range-related items directly under `ops`, without breaking existing links or triggering deprecation warnings on previous editions.
- `RangeTo(Inclusive)::rev()` that returns an iterator?
- `IterRangeInclusive` can be optimized to take advantage of the case where the bounds don't occupy the full domain of the index type:

```rust
enum IterRangeInclusiveImpl<Idx> {
    // Used when `end < Idx::MAX`
    // Works like `start..(end + 1)`
    Exclusive { start: Idx, end: Idx },
    // Used when `end == Idx::MAX && start > Idx::MIN`
    // Works like `((start - 1)..end).map(|i| i + 1)`
    ExclusiveOffset { start: Idx, end: Idx },
    // Only used when `start == Idx::MIN` and `end == Idx::MAX`
    // Works like `start..=end` does now
    // No need for `exhausted` flag, uses `start < end` instead
    Inclusive { start: Idx, end: Idx },
}

pub struct IterRangeInclusive<Idx> {
    inner: IterRangeInclusiveImpl<Idx>,
}
```

[playground](https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=dc5a5009cd311a86d54d258a8471cf88)
