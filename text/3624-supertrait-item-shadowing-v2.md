- Feature Name: `supertrait_item_shadowing`
- Start Date: 2024-05-04
- RFC PR: [rust-lang/rfcs#3624](https://github.com/rust-lang/rfcs/pull/3624)
- Tracking Issue: [rust-lang/rust#89151](https://github.com/rust-lang/rust/issues/89151)

# Summary
[summary]: #summary

When method selection encounters an ambiguity between two trait methods when both traits are in scope, if one trait is a subtrait of the other then select the method from the subtrait instead of reporting an ambiguity error.

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
    // ^ This is ambiguous: it could refer to Iterator::intersperse or Itertools::intersperse
}
```

This code actually works today because `intersperse` is an unstable API, and the compiler already has [logic](https://github.com/rust-lang/rust/pull/48552) to prefer stable methods over unstable methods when an ambiguity occurs.

Attempts to stabilize `intersperse` have failed with a large number of regressions [reported by crater](https://github.com/rust-lang/rust/issues/88967) which affect many popular crates. Even if these were to be manually corrected (since ambiguity is considered allowed breakage) we would have to go through this whole process again every time a method from `itertools` is uplifted to the standard library.

# Proposed solution
[proposed-solution]: #proposed-solution

This RFC proposes to change method selection to resolve the ambiguity in the following specific circumstances:

- All method candidates are trait methods (inherent methods are already prioritized over trait methods).
- One trait is transitively a subtrait of all other traits in the candidate list.

When this happens, the subtrait method is selected instead of reporting an ambiguity error.

Note that this only happens when *both* traits are in scope since this is required for the ambiguity to occur in the first place.

We will provide an allow-by-default lint to let users opt in to being notified when an ambiguity is resolved in this way.

# Drawbacks
[drawbacks]: #drawbacks

This behavior might be surprising as adding a method to a subtrait can change which function is called in unrelated code. This is somewhat mitigated by the opt-in lint which, when enabled, warns users about the potential ambiguity.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

If we choose not to accept this RFC then there doesn't seem to be a reasonable path for adding new methods to the `Iterator` trait if such methods are already provided by `itertools` without a lot of ecosystem churn.

## Only doing this for specific traits

One possible alternative to a general change to the method selection rules would be to only do so on a case-by-case basis for specific methods in standard library traits. This could be done by using a perma-unstable `#[shadowable]` attribute specifically on methods like `Iterator::intersperse`.

There are both advantages and inconveniences to this approach. While it allows most Rust users to avoid having to think about this issue for most traits, it does make the `Iterator` trait more "magical" in that it doesn't follow the same rules as the rest of the language. Having a consistent rule for how method selection works is easier to teach people.

## Preferring the supertrait method instead

In cases of ambiguity between a subtrait method and a supertrait method, there are two ways of resolving the ambiguity. This RFC proposes to resolve in favor of the subtrait since this is most likely to avoid breaking changes in practice.

Consider this situation:

- Library A has trait `Foo`.
- Crate B, depending on A, has trait `FooExt` with `Foo` as a supertrait.
- A adds a new method to `Foo`, but it has a default implementation so it's not breaking. B has a preexisting method with the same name.

In this general case, the reason this cannot be resolved in favor of the supertrait is that the method signatures are not necessarily compatible.

[In code](https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=b3919f7a8480c445d40b18a240936a07):

```rust
#![allow(unused)]

mod a {
    pub trait Int {
        // fn call(&self) -> u32 {
        //     0
        // }
    }
    impl Int for () {}
}

mod b {
    pub trait Int: super::a::Int {
        fn call(&self) -> u8 {
            0
        }
    }
    impl Int for () {}
}

use a::Int as _;
use b::Int as _;

fn main() {
    let val = ().call();
    println!("{}", std::any::type_name_of_val(&val));
}
```

Resolving in favor of `a` is a breaking change; resolving in favor of `b` is not. The only other option is the status quo -- not compiling. Resolving to `a` simply cannot happen lest we violate backwards compatibility, and the status quo is not ideal.

# Prior art
[prior-art]: #prior-art

### RFC 2845

RFC 2845 was a previous attempt, but it did not fully address the problem since it only changes method selection when trait methods are resolved due to generic bounds. In practice, most of the ambiguity from stabilizing `intersperse` comes from non-generic code.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Should we have a warn-by-default lint that fires at the definition-site of a subtrait that shadows a supertrait item?
