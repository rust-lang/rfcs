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
    #[try_step]
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

A `where` clause is also permitted in the single-item form, allowing
bounds that do not apply directly to the parameters, or just as a more
readable alternative to giving bounds in the angle bracket syntax:

```rust
#[derive(<T, U> Frob<T> where T: Bound1, Bar<U>: Bound2)]
struct Foo<U> {
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

In the single-item form of the `derive` attribute, the item may be
appended by a `where` clause:

> _DeriveAttrInputWithWhere_ :\
> &nbsp;&nbsp; _Generics_ _TypePath_ _WhereClause_

The overall `derive` attribute syntax is:

> _DeriveAttrInput_:\
> &nbsp;&nbsp; _DeriveItem_ (`,` _DeriveItem_)<sup>\*</sup> `,`<sup>?</sup>\
> &nbsp;&nbsp; | _DeriveAttrInputWithWhere_

A procedural macro can optionally support generic parameters to `derive` by
defining an entry point annotated with the `proc_macro_derive_with_generics`
attribute:

```rust
extern crate proc_macro;
use proc_macro::{DeriveGenerics, TokenStream};

#[proc_macro_derive_with_generics(Frob)]
pub fn derive_frob_with_generics(
    generics: DeriveGenerics,
    item: TokenStream,
) -> TokenStream {
    // ...
}
```

The `DeriveGenerics` struct is provided by `proc_macro` as follows:

```rust
pub struct DeriveGenerics {
    /// List of impl parameters, including the enclosing angle brackets.
    /// Empty if the derive attribute item has no generics.
    pub impl_generics: TokenStream,
    /// Generic arguments of the trait path including the angle brackets
    /// or functional syntax, or empty if the trait has no generic parameters.
    pub trait_args: TokenStream,
    /// Where clause, if present.
    pub where_clause: Option<TokenStream>,
}
```

Invoked in the example featuring the `where` clause above,
the `DeriveGenerics` parameter of the function will receive:
- the token stream of `<T, U>` in the `impl_generics` member,
- the token stream of `<T>` in the `trait_args` member,
- `Some` with the token stream of `where T: Bound1, Bar<U>: Bound2`
  as the `where_clause` member.

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
familiar to the developers, as it forms parts of the syntax of the intended
trait impl item. The same property makes the extended attribute input data
easier to use in the derive macros.

An [earlier proposal][rust-lang/rfcs#2353] to control generic bounds on
derived items introduces two attributes used on the generic parameters of
the type definition item, the whole item, or its fields. Using separate
attributes, however, visually distances the declaration from its effect
on the behavior on the `derive` attribute, and in many cases would be
more verbose. It also splits the solution across multiple attributes, whereas
the extended `derive` syntax proposed here is holistic, consistent with the
syntax of the generated impl item to the extent of informing literal parts
of it, and may allow further extension in similarly holistic ways.
The extension proposed here is opted into by the macro authors if and when
they wish to do so, while the solution proposed in [rust-lang/rfcs#2353]
expects all macro authors to implement support for the new attributes
"so that a consistent experience is maintained in the ecosystem".

[rust-lang/rfcs#2353]: https://github.com/rust-lang/rfcs/pull/2353

An alternative has been proposed in the pre-RFC discussion to customize
bounds by trait-specific helper attributes. This is already a practice in
some projects, including Servo. It has some disadvantages of the alternative
above, furthermore, it bifurcates the solution into mostly similar custom
attributes that add to cognitive load and may lead to maintenance trouble
if the preferred syntax is changed again. The proposal discussed here, however,
does not exclude augmentation with helper attributes, which may help further
reduce boilerplate in deriving traits within a large codebase, or in a
particularly popular API. A more systematic approach like all or part of
[rust-lang/rfcs#2353] is also not incompatible with this one.

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

- A combining syntax `#[derive(<T: Bound> Trait1 + Trait2 + Trait3)]` is also
  possible, either standalone or as an item in a comma-separated list.
  Should it be included while we are at radically extending `derive`, or
  should it wait for another stabilization round just to be careful?
- The `where` clause syntax could be chosen as the only available way to
  specify generics in preference to the angle bracketed parameter list.
  If so, unbounded parameters would look a little weird, though permitted
  in the current syntax for `where` clauses (and hey, we have a chance to
  legitimize smileys in Rust here):

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
  and the other with a plain `proc_macro_derive`? Conversely, should it be
  disallowed to have both kinds of entry points for the same trait in one
  procedural macro crate?
- Should the `proc_macro_derive` annotation be reused for the extended
  function signature, rather than introducing `proc_macro_derive_with_generics`
  and needing a policy on coexistence of the two kinds as per the questions
  above (that is, disallow coexistence by uniting both kinds under a single
  `proc_macro_derive` registry)?

# Future possibilities
[future-possibilities]: #future-possibilities

Extending `derive` with generics would open this language extension mechanism
to far wider use and experimentation than what is possible today; the
[motivational section](#motivation) provides only a few beneficial examples.

# Acknowledgements
[acknowledgements]: #acknowledgements

Thanks to David Tolnay [@dtolnay](https://github.com/dtolnay) for proposing
the `where` clause, suggesting alternative ideas and offering constructive
criticism.
