- Feature Name: `new_range`
- Start Date: 2023-12-18
- RFC PR: [rust-lang/rfcs#3550](https://github.com/rust-lang/rfcs/pull/3550)
- Tracking Issue: [rust-lang/rust#123741](https://github.com/rust-lang/rust/issues/123741)

# Summary
[summary]: #summary

Change the range operators `a..b`, `a..`, and `a..=b` to resolve to new types `std::range::Range`, `std::range::RangeFrom`, and `std::range::RangeInclusive` in Edition 2024. These new types will not implement `Iterator`, instead implementing `Copy` and `IntoIterator`.

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

- `a..b` denotes a range from `a` (inclusive) to `b` (exclusive). It resolves to the type `std::range::Range`.
  The iterator for `Range` will yield values from `a` (inclusive) to `b` (exclusive) in steps of one.

- `a..=b` denotes a range from `a` (inclusive) to `b` (inclusive). It resolve to the type `std::range::RangeInclusive`.
  The iterator for `RangeInclusive` will yield values from `a` (inclusive) to `b` (inclusive) in steps of one.

- `a..` denotes a range from `a` (inclusive) with no upper bound. It resolves to the type `std::range::RangeFrom`.
  The iterator for `RangeFrom` will yield values starting with `a` and increasing in steps of one.

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

In Rust editions prior to 2024, `a..b`, `a..=b`, and `a..` resolved to a different set of types (now found in `std::range::legacy`). These legacy range types did not implement `Copy`, and implemented `Iterator` directly (rather than `IntoIterator`).

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
[migrating]: #migrating

In many cases, no changes need to be made at all. This includes most places where a `RangeBounds` or `IntoIterator` is expected:
```rust
pub fn takes_range(range: impl std::ops::RangeBounds<usize>) { ... }
takes_range(0..5); // No changes necessary

pub fn takes_iter(range: impl IntoIterator<usize>) { ... }
takes_iter(0..5); // No changes necessary
```

And most places where `Iterator` methods were used directly on the range:
```rust
for n in (0..5).rev() { ... } // No changes necessary
for n in (0..5).map(|x| x * 2) { ... } // No changes necessary
```

In other cases, `cargo fix --edition` will insert `.into_iter()` as necessary:

```rust
pub fn takes_iter(range: impl Iterator<usize>) { ... }
takes_iter((0..5).into_iter()); // Add `.into_iter()`

// Before
(0..5).for_each(...);
// After
(0..5).into_iter().for_each(...); // Add `.into_iter()`

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

Or fall back to converting to the legacy types:

```rust
// Before
pub fn takes_range(range: std::ops::Range<usize>) { ... }
takes_range(0..5);
// After
pub fn takes_range(range: std::range::legacy::Range<usize>) { ... }
takes_range((0..5).into());
```

## Migrating Libraries

Some libraries have range types in their public interface. To use the new range types with such a library, users will need to add explicit conversions.

To reduce the burden of explicit conversions, libraries should make the following backwards-compatible changes:

- Change any function parameters from legacy `Range*` types to `impl Into<Range*>`
  Or if applicable, `impl RangeBounds<_>`

```rust
// Before
pub fn takes_range(range: std::ops::Range<usize>) { ... }

// After
pub fn takes_range(range: impl Into<std::range::legacy::Range<usize>>) { ... }
// Or
pub fn takes_range(range: impl std::ops::RangeBounds<usize>) { ... }
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

- When your library implements a trait involving ranges, such as `std::ops::Index`, add impls for the new range types

```rust
// Before
use std::ops::{Index, Range};
impl Index<Range<usize>> for Bar { ... }

// After
use std::ops::{Index, Range};
impl Index<Range<usize>> for Bar { ... }
impl Index<std::range::Range<usize>> for Bar { ... }
```

**Note**
- These changes to libraries should happen when _users_ of a given library transition to the new edition
- These changes do not require the library itself to transition to the new edition

## Diagnostics

There is a substantial amount of educational material in the wild which assumes the range types implement `Iterator`. If a user references this outdated material, it is important that compiler errors guide them to the new solution.

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

**Note:** The exact names and module paths in this RFC are for demonstration purposes only, and can be finalized by _T-libs-api_ after the proposal is accepted.

Add replacement types only for the current `Range`, `RangeFrom`, and `RangeInclusive`.

The [**Range Expressions** page in the Reference](https://doc.rust-lang.org/reference/expressions/range-expr.html) will change to read as follows

> ## Edition 2024 and later
>
> The `..` and `..=` operators will construct an object of one of the `std::range::Range` (or `core::range::Range`) variants, according to the following table:
>
> | Production             | Syntax        | Type                         | Range                 |
> |------------------------|---------------|------------------------------|-----------------------|
> | _RangeExpr_            | start`..`end  | std::range::Range            | start &le; x &lt; end |
> | _RangeFromExpr_        | start`..`     | std::range::RangeFrom        | start &le; x          |
> | _RangeToExpr_          | `..`end       | std::range::RangeTo          |            x &lt; end |
> | _RangeFullExpr_        | `..`          | std::range::RangeFull        |            -          |
> | _RangeInclusiveExpr_   | start`..=`end | std::range::RangeInclusive   | start &le; x &le; end |
> | _RangeToInclusiveExpr_ | `..=`end      | std::range::RangeToInclusive |            x &le; end |
>
> **Note:** While `std::ops::RangeTo`, `std::ops::RangeFull`, and `std::ops::RangeToInclusive` are re-exports of `std::range::RangeTo`, `std::range::RangeFull`, and `std::ops::Range::RangeToInclusive` respectively, `std::ops::Range`, `std::ops::RangeFrom`, and `std::ops::RangeInclusive` are re-exports of the types under `std::range::legacy::` (NOT those directly under `std::range::`) for backwards-compatibility reasons.
>
> Examples:
>
> ```rust
> 1..2;   // std::range::Range
> 3..;    // std::range::RangeFrom
> ..4;    // std::range::RangeTo
> ..;     // std::range::RangeFull
> 5..=6;  // std::range::RangeInclusive
> ..=7;   // std::range::RangeToInclusive
> ```
>
> The following expressions are equivalent.
>
> ```rust
> let x = std::range::Range {start: 0, end: 10};
> let y = 0..10;
>
> assert_eq!(x, y);
> ```
>
> ## Prior to Edition 2024
>
> The `..` and `..=` operators will construct an object of one of the `std::range::legacy::Range` (or `core::range::legacy::Range`) variants, according to the following table:
>
> | Production             | Syntax        | Type                         | Range                 |
> |------------------------|---------------|------------------------------|-----------------------|
> | _RangeExpr_            | start`..`end  | std::range::legacy::Range            | start &le; x &lt; end |
> | _RangeFromExpr_        | start`..`     | std::range::legacy::RangeFrom        | start &le; x          |
> | _RangeToExpr_          | `..`end       | std::range::RangeTo          |            x &lt; end |
> | _RangeFullExpr_        | `..`          | std::range::RangeFull        |            -          |
> | _RangeInclusiveExpr_   | start`..=`end | std::range::legacy::RangeInclusive   | start &le; x &le; end |
> | _RangeToInclusiveExpr_ | `..=`end      | std::range::RangeToInclusive |            x &le; end |
>
> **Note:** `std::ops::Range`, `std::ops::RangeFrom`, and `std::ops::RangeInclusive` are re-exports of the respective types under `std::range::legacy::`. `std::ops::RangeTo`, `std::ops::RangeFull`, and `std::ops::RangeToInclusive` are re-exports of the respective types under `std::range::`.
>
> Examples:
>
> ```rust
> 1..2;   // std::range::legacy::Range
> 3..;    // std::range::legacy::RangeFrom
> ..4;    // std::range::RangeTo
> ..;     // std::range::RangeFull
> 5..=6;  // std::range::legacy::RangeInclusive
> ..=7;   // std::range::RangeToInclusive
> ```
>
> The following expressions are equivalent.
>
> ```rust
> let x = std::range::legacy::Range {start: 0, end: 10};
> let y = std::ops::Range {start: 0, end: 10};
> let z = 0..10;
>
> assert_eq!(x, y);
> assert_eq!(x, z);
> ```

## New paths

There is no language support for edition-dependent path resolution, so these types must continue to be accessible under their current paths. However, their canonical paths will change to live under `std::range::legacy`:

- `std::ops::Range` will be a re-export of `std::range::legacy::Range`
- `std::ops::RangeFrom` will be a re-export of `std::range::legacy::RangeFrom`
- `std::ops::RangeInclusive` will be a re-export of `std::range::legacy::RangeFrom`

In order to not break existing links to the documentation for these types, the re-exports must remain `doc(inline)`.

The replacement types will live under `range`:

- `std::range::Range` will be the Edition 2024 replacement for `std::range::legacy::Range`
- `std::range::RangeFrom` will be the Edition 2024 replacement for `std::range::legacy::RangeFrom`
- `std::range::RangeInclusive` will be the Edition 2024 replacement for `std::range::legacy::RangeFrom`

The `RangeFull`, `RangeTo`, and `RangeToInclusive` types will remain unchanged. But for consistency, their canonical paths will be changed to live under `range`:

- `std::ops::RangeFull` will be a re-export of `std::range::RangeFull`
- `std::ops::RangeTo` will be a re-export of `std::range::RangeTo`
- `std::ops::RangeToInclusive` will be a re-export of `std::range::RangeToInclusive`

## Iterator types

Because the three new types will implement `IntoIterator` directly, they need three new respective `IntoIter` types:

- `std::range::IterRange` will be `<range::Range<_> as IntoIterator>::IntoIter`
- `std::range::IterRangeFrom` will be `<range::RangeFrom<_> as IntoIterator>::IntoIter`
- `std::range::IterRangeInclusive` will be `<range::RangeInclusive<_> as IntoIterator>::IntoIter`

These iterator types will implement the same iterator traits (`DoubleEndedIterator`, `FusedIterator`, etc) as the legacy range types, with the following exceptions:
- `std::range::IterRange` will not implement `ExactSizeIterator` for `u32` or `i32`
- `std::range::IterRangeInclusive` will not implement `ExactSizeIterator` for `u16` or `i16`

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

`std::range::Range` and `std::range::RangeFrom` will have identical structure to the existing types, with public fields for the bounds. However, `std::range::RangeInclusive` will be changed:
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

The new types should have inherent methods to match the most common usages of `Iterator` methods. `map` and `rev` are the bare minimum; we leave the exact set to be finalized by _T-libs-api_ after the proposal is accepted.

```rust
impl<Idx> Range<Idx> {
    /// Shorthand for `.into_iter().map(...)`
    pub fn map<B, F>(self, f: F) -> iter::Map<<Self as IntoIterator>::IntoIter, F>
    where
        Self: IntoIterator,
        F: FnMut(Idx) -> B,
    {
        self.into_iter().map(f)
    }

    /// Shorthand for `.into_iter().rev()`
    pub fn rev(self) -> iter::Rev<<Self as IntoIterator>::IntoIter>
    where
        Self: IntoIterator,
        <Self as IntoIterator>::IntoIter: DoubleEndedIterator,
    {
        self.into_iter().rev()
    }
}
```

# Drawbacks
[drawbacks]: #drawbacks

This change has the potential to cause a significant amount of churn in the ecosystem. There are two main sources of churn:
- where ranges are assumed to be `Iterator`
- trait impls involving ranges, such as `Index<legacy::Range<_>>`

Changes will be required to support the new range types, even on older editions. See the [migrating section](#migrating) for specifics.

### Ranges assumed to be `Iterator`

This is not uncommon in the ecosystem. For instance, both [`rustc-rayon`](https://github.com/pitaj/rustc-rayon/commit/e76e554512ce25abb48f4118576ede5d7a457918) and [`quote`](https://github.com/pitaj/quote/commit/44feebf0594b255a511ff20890a7acbf4d6aeed1) needed patches for this during experimentation.

### `impl Index<Range<_>> for X`

A [Github search for this pattern](https://github.com/search?type=code&q=language%3Arust+NOT+is%3Afork+%28%22Index%3CRange%3C%22+OR+%22Index%3Cops%3A%3ARange%3C%22+OR+%22Index%3Cstd%3A%3Aops%3A%3ARange%3C%22+OR+%22Index%3Ccore%3A%3Aops%3A%3ARange%3C%22+OR+%22Index%3CRangeInclusive%3C%22+OR+%22Index%3Cops%3A%3ARangeInclusive%3C%22+OR+%22Index%3Cstd%3A%3Aops%3A%3ARangeInclusive%3C%22+OR+%22Index%3Ccore%3A%3Aops%3A%3ARangeInclusive%3C%22+OR+%22Index%3CRangeFrom%3C%22+OR+%22Index%3Cops%3A%3ARangeFrom%3C%22+OR+%22Index%3Cstd%3A%3Aops%3A%3ARangeFrom%3C%22+OR+%22Index%3Ccore%3A%3Aops%3A%3ARangeFrom%3C%22%29) yields 784 files, almost all of which appear to be true matches. It's hard to say how many of those are published libraries, but it does indicate that this could have a significant impact.

## Mitigation

To mitigate these drawbacks, we recommend introducing and stabilizing an MVP of the new types as soon as possible, well before Edition 2024 releases (even before the implementation of the syntax feature is complete). This will give libraries time to issue updates supporting the new range types.

Some users may depend on libraries that are not updated before Edition 2024. These users do not just have to accept adding explicit conversions to their code. They also have the option to stay on a prior edition.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Just implement `Copy` on the types as-is

`Copy` iterators are a large footgun. It was decided to [remove `Copy` from all iterators back in 2015](https://github.com/rust-lang/rust/pull/21809), and that decision is unlikely to be reversed.

That said, there are a few possibilities:
- Sophisticated lint to catch when an iterator is problematically copied
- Language or library feature to allow `Copy` structs to have certain non-`Copy` fields
- Specialize `IntoIterator` on these range types and lint whenever the `Iterator` impl is used

None of these approaches would resolve the following serious issues:
- `RangeInclusive` being larger than necessary for range purposes
- Incorrect `ExactSizeIterator` implementations

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

But the legacy range types have a much larger API surface than other `Iterator`s in the standard library, which typically only implement the various iterator traits and maybe have a `remainder` method. Specifically, there are no iterator types in the standard library which have public fields. Nor do any implement `PartialEq`, `Eq`, `Hash`, `Index`, or `IndexMut`.

`RangeInclusive` especially must take care with equality, hashing, and indexing because it can be exhausted. By removing those impls from the iterator for it, we can prevent that misuse entirely.

One of the strongest arguments for new types is the incorrect `ExactSizeIterator` implementations for `Range<u32 | i32>` and `RangeInclusive<u16 | i16>`. These can be excluded if new iterator types are introduced.

Finally, the cost of adding these iterator types is extremely low, given we're already adding a set of new types for the ranges themselves.

## Inherent `map` should map the bounds, not return an iterator

Some argue that inherent `map` should not return an iterator. Some say that they may expect it to map each bound individually (`(1..11).map(|x| x*2)` -> `2..22`). Others say these methods should return `IntoIterator` types instead.

However, making them return an iterator has many benefits:
- Matches existing behavior
- Reduces code churn
- Act as an entry point for other iterator methods

Adding these convenience methods is unlikely to cause confusion because of how common this pattern already is (if anything, the opposite is true). Plus, it's pretty easy to tell based on the function signature what is going on, and it's simple to document.

Changing the meaning of `(1..11).map(...)` is a huge hazard. There is a lot of existing code, documentation, etc that uses it in the `Iterator` sense. It would be incredibly confusing, especially to a newcomer, to have it do something totally different between editions. Especially since in many cases it could silently change meaning:

```rust
// Edition 2021
for n in (1..11).map(|n| n*2) {
    // n = 2, 4, 6, ...., 16, 18, 20
}
// Edition 2024?
for n in (1..11).map(|n| n*2) {
    // n = 2, 3, 4, 5, 6, 7, ...., 15, 16, 17, 18, 19, 20, 21
}
```

If there is demand for a method that maps the bounds, it should be added under a different name, such as `map_bounds` , perhaps even as a method on `RangeBounds`.

## Implicit conversions (coercions)

This proposal specifically avoids involving any form of implicit conversion. Adding coercions from the new to legacy types would have a few benefits:

- Avoid explicit conversions when migrating automatically to Edition 2024
- Few (if any) library changes needed to support the new types

Coercions would effectively eliminate the main drawback of this RFC. However, adding implicit conversions has severe drawbacks of its own:

- Makes it harder to reason about code
- Further blurs the line between language and library
- Affects type inference

In this specific case, the coercion would also need to be considered during trait resolution to be significantly useful, which is not currently done in other cases like deref coercion.

### Range literal

We could treat range expressions as a kind of literal, and only "coerce" them into the legacy range types at the point of the range syntax. Similar to integer literals, the concrete type would be chosen based on context, like how `4` can be used anywhere expecting any integer type.

This would have fewer serious downsides than coercions, but both approaches add a large cost for implementation in the compiler.

We don't consider the downsides of either approach to be justified given the relative rarity of libraries needing changes in the first place, the ease of adding explicit conversions when necessary, and the option for users to continue to use prior editions while waiting for library support.

# Prior art
[prior-art]: #prior-art

The [copy-range](https://docs.rs/copy-range) crate provides types similar to those proposed here.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

### Ecosystem Disruption

We must take into account the ecosystem impact of this change before stabilization.

- How do we properly document and execute the ecosystem transition?
- How much time will it take to propagate this change throughout the ecosystem?
- What degree of ecosystem saturation would we be satisfied with?
- How much time do we need with stable library types before making the lang change?
- What about libraries that wish to maintain a certain MSRV?
- Taking into account all of the mitigations (diagnostics, migrations, and lints but NOT language-level changes), is the level of ecosystem disruption acceptable?
- What is expected of new libraries? Should they continue to support both sets of ranges or only the new ones?
- Will new Rust users need to learn about older editions because of downstream users of their code?

### API

We leave the following items to be decided by the **libs-api** team after this proposal is accepted and before stabilization:

- The set of inherent methods copied from `Iterator` present on the new range types
- The exact module paths and type names
  + Should the new types live at `std::ops::range::` instead?
  + `IterRange`, `IterRangeInclusive` or just `Iter`, `IterInclusive`? Or `RangeIter`, `RangeInclusiveIter`, ...?
- Should other range-related items (like `RangeBounds`) also be moved under the `range` module?
- Should `RangeFrom` even implement `IntoIterator`, or should it require an explicit `.iter()` call? Using it as an iterator [can be a footgun](https://github.com/rust-lang/libs-team/issues/304), usually people want `start..=MAX` instead. Also, it is inconsistent with `RangeTo`, which doesn't implement `IntoIterator` either.
- Should there be a way to get an iterator that modifies the range in place, rather than taking the range by value? That would allow things like `range.by_ref().next()`.
- Should there be an infallible conversion from legacy to new `RangeInclusive`?
```rust
impl<Idx> From<legacy::RangeInclusive<Idx>> for RangeInclusive<Idx> {
    // How do we handle the `exhausted` case, set `end < start`?
}
```

# Future possibilities
[future-possibilities]: #future-possibilities

- Hide or deprecate range-related items directly under `ops` (without breaking existing links or triggering deprecation warnings on previous editions).
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
