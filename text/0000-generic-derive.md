- Feature Name: `generic_derive`
- Start Date: 2019-11-09
- RFC PR: [rust-lang/rfcs#2811](https://github.com/rust-lang/rfcs/pull/2811)
- Rust Issue: 

# Summary
[summary]: #summary

Add ability to pass generic parameters of the impl to the derive macros,
greatly increasing the flexibility of the `derive` attribute.

# Motivation
[motivation]: #motivation

Derive macros are a very convenient way to generating trait impls based on
the definition item of a type. However, the ability to use `#[derive(Trait)]`
is denied when the impl must have generic parameters that need to be defined
and bound in a more customized way than what the derive macro could generate
automatically based on the definition item of the `Self` type.

Consider The Most Annoying Problem of `#[derive(Clone)]`:

```rust
#[derive(Clone)]
pub struct WaitingForGodot<T> {
    // ...
    _phantom_godot: PhantomData<T>
}
```

The use of `derive` here is often a convenient pitfall that generates this impl:

```rust
impl<T: Clone> Clone for WaitingForGodot<T> {
    //  ^---- Oops, did not really need this bound
    // ...
}
```

This can be easily solved by customizing the impl parameter:

```rust
#[derive(<T> Clone)]
pub struct WaitingForGodot<T> {
    // ...
    _phantom_godot: PhantomData<T>
}
```

More traits could be made conveniently derivable with custom generics than
is feasible now:

```rust
use derive_unpin::Unpin;

#[derive(<St: Unpin, F> Unpin)]
pub struct MyFold<St, F> {
    #[unsafe_pinned]
    stream: St,
    #[unsafe_unpinned]
    op: F,
}
```

In tandem with more elaborate helper attributes, it could be even more powerful:

```rust
// A not-yet-written library providing the derive macro
use async_state_machine::Future;
use futures::future::{TryFuture, IntoFuture, MapOk};

#[derive(
    <
        Fut1: TryFuture,
        Fut2: TryFuture<Error = Fut1::Error>,
        F: FnOnce(<Fut1 as TryFuture>::Ok) -> Fut2,
    > Future
)]
enum AndThen<Fut1, Fut2, F> {
    First(MapOk<Fut1, F>),
    #[after(First)]
    #[future(output)]
    Then(IntoFuture<Fut2>),
}
```

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The trait name in a `derive` attribute can be adorned with generic parameters
that specify the generics of the generated `impl` item:

```rust
#[derive(<T: Bound1, U: Bound2> Frob<T>)]
struct Foo<U> {
    // ...
}
```

The derive macro for `Frob` is expected to generate an implementation item
with these generic parameters:

```rust
impl<T: Bound1, U: Bound2> Frob<T> for Foo<U> {
    // ...
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The syntax of an item in the `derive` attribute is extended to a subset of the
language that can occur in a trait implementation item between the keywords
`impl` and `for`:

> _DeriveItem_ :\
> &nbsp;&nbsp; _Generics_<sup>?</sup> _TypePath_

The procedural macro can optionally support generic parameters to `derive` by
defining an entry point annotated with the `proc_macro_derive_with_generics`
attribute:

```rust
extern crate proc_macro;
use proc_macro::TokenStream;

#[proc_macro_derive_with_generics(Frob)]
pub fn derive_frob_with_generics(
    generics: TokenStream,
    trait_args: Option<TokenStream>,
    item: TokenStream,
) -> TokenStream {
    // ...
}
```

Invoked in the example above, the function will receive the token stream of
`<T: Bound1, U: Bound2>` as the first argument, a `Some` value with the token
stream of `<T>` as the second argument, and the token stream with the
`struct Foo` item as the third.

If the compiler does not find a matching `proc_macro_derive_with_generics`
symbol in the procedural macro crate that it has resolved for a `derive` item
that features generics, an error is reported stating that the macro does not
support generics. A plain old `derive` item can be processed with
a function annotated as `proc_macro_derive_with_generics` if no function
is annotated as `proc_macro_derive` for the same trait, otherwise the other
function gets called.

# Drawbacks
[drawbacks]: #drawbacks

This extension complicates the syntax of the `derive` attribute.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Extending `derive` this way, we can solve its current shortcomings and
open it to more uses and experimentation. The proposed syntax should be
familiar to the developers, as it forms a part of the syntax of the intended
trait impl item.

An [earlier proposal][rust-lang/rfcs#2353] to control generic bounds on
derived items introduces two attributes used on the generic parameters of
the type definition item, the whole item, or its fields. Using separate
attributes, however, visually distances the declaration from its effect
on the behavior on the `derive` attribute, and in many cases would be
more verbose. It also splits the solution across multiple attributes, whereas
the extended `derive` syntax proposed here is holistic, consistent with the
syntax of the generated impl item to the extent of being a literal
subsequence of it, and may allow further extension also in holistic ways.
The extension proposed here is opted into by the macro authors if and when
they wish to do so, while the solution proposed in RFC 2353 expects all
macro authors to implement support for the new attributes "so that a consistent
experience is maintained in the ecosystem".

[rust-lang/rfcs#2353]: https://github.com/rust-lang/rfcs/pull/2353

An alternative has been proposed in the pre-RFC discussion to enable custom
bounds by trait-specific inert attributes. This has some disadvantages of
the alternative above, furthermore, it bifurcates the solution into mostly
similar custom attributes that add to cognitive load and may lead to
maintenance trouble if the preferred syntax is changed again.

Everything proposed here is also possible to implement with custom attribute
macros instead of `derive` macros. But this would unnecessarily multiply
mechanisms for generating a trait implementation for a type. Plugging into a
well-defined syntax of the `derive` attribute would make the macro more
memorable for the users and may be more friendly to automatic analysis
than freeform attribute macros.

# Prior art
[prior-art]: #prior-art

The analysis done in the [previous proposal][rust-lang/rfcs#2353] is
sufficient for this RFC as well.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

## The syntax

- Is it advisable, or even possible syntactically, to extend the general
  `derive` syntax with optional generics for each comma-separated item,
  or should this be only permitted as an alternative form of `derive`
  with a single item? An alternative combining syntax
  `#[derive(<T: Bound> Trait1 + Trait2 + Trait3)]` is also possible,
  either standalone or as an item in a comma-separated list.
- It's possible to extend the syntax even further by supporting a `where`
  clause, allowing more complex bounds, or just as a more readable alternative
  to bounds in the angle bracket syntax:

  ```rust
    #[derive(<Fut1, Fut2, F> Future where
        Fut1: TryFuture,
        Fut2: TryFuture<Error = Fut1::Error>,
        F: FnOnce(<Fut1 as TryFuture>::Ok) -> Fut2,
    )]
    enum AndThen<Fut1, Fut2, F> {
        // ...
    }
  ```

  The `where` clause syntax could be chosen as the only available way to
  specify generics in preference to the angle bracketed parameter list.
  If so, unbounded parameters would look a little weird, though permitted
  in the current syntax for `where` clauses:

  ```rust
    #[derive(Unwrap where St: Unwrap, F:)]
    struct MyFold<St, F> {
        // ...
    }
  ```

  This form would also be harder on the macro implementation, which would
  not get a list of parameters to paste directly into the generated impl item,
  but would have to assemble them from the type definition item and the
  possible trait parameters.

## The extended macro entry point

- Should it be permitted to have two derive macros in scope for the
  same trait, one with a `proc_macro_derive_with_generics` entry point
  and the other with a plain `proc_macro_derive`? Conversely, should having
  both kinds of entry points for the same trait in one procedural macro crate
  be disallowed?
- Should the `proc_macro_derive` annotation be reused for the extended
  function signature, rather than introducing `proc_macro_derive_with_generics`
  and needing a policy on coexistence of the two kinds as per the questions
  above (that is, disallow coexistence by uniting both kinds under a single
  `proc_macro_derive` registry)?
  This may lead to confusion, as the only distinguishing factor here would be
  the number of parameters and their types, and two-to-three `TokenStream`
  parameters do not exactly jump out and say "generics be here". A more
  disciplined struct could be added to the `proc_macro` API for the new
  function signature.

# Future possibilities
[future-possibilities]: #future-possibilities

Extending `derive` with generics would open this language extension mechanism
to far wider use and experimentation than what is possible today; the
[motivational section](#motivation) provides only a few beneficial examples.

# Acknowledgements
[acknowledgements]: #acknowledgements

Thanks to David Tolnay [@dtolnay](https://github.com/dtolnay) for suggesting
alternative ideas and offering constructive criticism.
