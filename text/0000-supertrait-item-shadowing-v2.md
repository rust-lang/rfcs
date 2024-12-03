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
    // ^ This is ambiguous: it could refer to Iterator::intersperse or Itertools::intersperse
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

When an ambiguity is resolved in this way, a lint warning is also emitted to warn the user about the potential ambiguity. The aim of this lint is to discourage reliance on this mechanism in normal code usage: it should only be used for backwards-compatibilty and the lint can be silenced by having users change their code. We can always later change this lint to be allowed by default if we consider that there are valid use cases for this feature other than backwards-compatiblity.

### Type inference

This change happens during name resolution and specifically doesn't interact with type inference. Consider this example:

```rust
trait Foo { fn method(&self) {} }
trait Bar: Foo { fn method(&self) {} }
impl<T> Foo for Vec<T> { }
impl<T: Copy> Bar for Vec<T> { }

fn main() {
    let x = vec![];
    x.method(); // which to call?
    x.push(Box::new(22)); // oh, looks like `Foo`
}
```

Today that example will give an ambiguity error because `method` is provided by multiple traits in scope. With this RFC, it will instead always resolve to the sub-trait method and then compilation will fail because `Vec` does not implement the `Copy` trait required by `Bar::method`.

# Drawbacks
[drawbacks]: #drawbacks

This behavior can be surprising: adding a method to a sub-trait can change which function is called in unrelated code. This is mitigated by the lint which warns users about the potential ambiguity.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

If we choose not to accept this RFC then there doesn't seem to be a reasonable path for adding new methods to the `Iterator` trait if such methods are already provided by `itertools` without a lot of ecosystem churn.

## Only doing this for specific traits

One possible alternative to a general change to the name resolution rules would be to only do so on a case-by-case basis for specific methods in standard library traits. This could be done by using a perma-unstable `#[shadowable]` attribute specifically on methods like `Iterator::intersperse`.

There are both advantages and inconvenients to this approach. While it allows most Rust users to avoid having to think about this issue for most traits, it does make the `Iterator` trait more "magical" in that it doesn't follow the same rules as the rest of the language. Having a consistent rule for how name resolution works is easier to teach people.

## Preferring the supertrait method instead

In cases of ambiguity between a subtrait method and a supertrait method, there are 2 ways of resolving the ambiguity. This RFC proposes to resolve in favor of the subtrait since this is most likely to avoid breaking changes in practice.

Consider this situation:

- library A has trait `Foo`
- crate B, depending on A, has trait `FooExt` with `Foo` as a supertrait
- A adds a new method to `Foo`, but it has a default implementation so it's not breaking. B has a pre-existing method with the same name.

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

Resolving in favor of `a` is a breaking change; in favor of `b` is not. The only other option is the status quo: not compiling. `a` simply cannot happen lest we violate backwards compatibility and the status quo is not ideal.

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
