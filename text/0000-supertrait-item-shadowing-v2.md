- Feature Name: `supertrait_item_shadowing`
- Start Date: 2024-05-04
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

When name resolution encounters an ambiguity between 2 trait methods when both traits are in scope, if one trait is a sub-trait of the other then select that method instead of reporting an ambiguity error.

# Motivation
[motivation]: #motivation


The libs-api team would like to stabilize `Iterator::intersperse` but has a problem. The `itertools` crate already has:

```rust
// itertools
trait Itertools: Iterator {
    fn intersperse(self, element: Self::Item) -> Intersperse<Self>;
}
```

This method is used in crates with code similar to the following:

```rust
use core::iter::Iterator; // Implicit import from prelude

use itertools::Itertools as _;

fn foo() -> impl Iterator<Item = &'static str> {
    "1,2,3".split(",").intersperse("|")
    // ^ This is ambiguious: it could refer to Iterator::intersperse or Itertools::intersperse
}
```

This code actually works today because `intersperse` is an unstable API, which works because the compiler already has [logic](https://github.com/rust-lang/rust/pull/48552) to prefer stable methods over unstable methods when an amiguity occurs.

Attempts to stabilize `intersperse` have failed with a large number of regressions [reported by crater](https://github.com/rust-lang/rust/issues/88967) which affect many popular crates. Even if these were to be manually corrected (since ambiguity is considered allowed breakage) we would have to go through this whole process again every time a method from `itertools` is uplifted to the standard library.

# Proposed solution
[proposed-solution]: #proposed-solution

This RFC proposes to change name resolution to resolve the ambiguity in the following specific circumstances:
- All method candidates are trait methods. (Inherent methods are already prioritized over trait methods)
- One trait is transitively a sub-trait of all other traits in the candidate list.

When this happens, the sub-trait method is selected instead of reporting an ambiguity error.

Note that this only happens when *both* traits are in scope since this is required for the ambiguity to occur in the first place.

# Drawbacks
[drawbacks]: #drawbacks

This behavior can be surprising: adding a method to a sub-trait can change which function is called in unrelated code. A lint could be emitted to warn users about the potential ambiguity.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

If we choose not to accept this RFC then there doesn't seem to be a reasonable path for adding new methods to the `Iterator` trait if such methods are already provided by `itertools` without a lot of ecosystem churn.

# Prior art
[prior-art]: #prior-art

### RFC 2845

RFC 2845 was a previous attempt to address this problem, but it has several drawbacks:
- It doesn't fully address the problem since it only changes name resolution when trait methods are resolved due to generic bounds. In practice, most of the amiguity from stabilizing `intersperse` comes from non-generic code.
- It adds a lot of complexity because name resolution depends on the specific trait bounds that have been brought into scope.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None

# Future possibilities
[future-possibilities]: #future-possibilities

None
