- Feature Name: `assoc_default_groups`
- Start Date: 2018-08-27
- RFC PR: _
- Rust Issue: _

# Summary
[summary]: #summary

[RFC 192]: https://github.com/rust-lang/rfcs/blob/master/text/0195-associated-items.md#defaults

1. [Resolve][changes] the design of associated type defaults,
   first introduced in [RFC 192],
   such that provided methods and other items may not assume type defaults.
   This applies equally to `default` with respect to specialization.

2. [Introduce][default_groups] the concept of `default { .. }` groups
   in traits and their implementations which may be used to introduce
   atomic units of specialization
   (if anything in the group is specialized, everything must be).

# Motivation
[motivation]: #motivation

## For associated type defaults

As discussed in the [background] and mentioned in the [summary],
associated type defaults were introduced in [RFC 192].
These defaults are valuable for a few reasons:

1. You can already provide defaults for `const`s and `fn`s.
   Allowing `type`s to have defaults adds consistency and uniformity
   to the language, thereby reducing surprises for users.

2. Associated `type` defaults in `trait`s simplify the grammar,
   allowing the grammar of `trait`s them to be more in line with
   the grammar of `impl`s. In addition, this brings `trait`s more in line
   with `type` aliases.

The following points were also noted in [RFC 192], but we expand upon them here:

3. Most notably, type defaults allow you to provide more ergonomic APIs.

   [proptest]: https://altsysrq.github.io/rustdoc/proptest/latest/proptest/arbitrary/trait.Arbitrary.html

   For example, we could change [proptest]'s API to be:

   ```rust
   trait Arbitrary: Sized + fmt::Debug {
       type Parameters: Default = ();
   
       fn arbitrary_with(args: Self::Parameters) -> Self::Strategy;

       fn arbitrary() -> Self::Strategy {
           Self::arbitrary_with(Default::default())
       }

       type Strategy: Strategy<Value = Self>;
   }
   ```

   Being able to say that the default of `Parameters` is `()` means that users
   who are not interested in this further detail may simply ignore specifying
   `Parameters`.

   The inability of having defaults results in an inability to provide APIs
   that are both a) simple to use, and b) flexible / customizable.
   By allowing defaults, we can have our cake and eat it too,
   enabling both a) and b) concurrently.

4. Type defaults also aid in API evolution.
   Consider a situation such as `Arbitrary` from above;
   The API might have originally been:

   ```rust
   trait Arbitrary: Sized + fmt::Debug {
       fn arbitrary() -> Self::Strategy;
   
       type Strategy: Strategy<Value = Self>;
   }
   ```

   with an implementation:

   ```rust
   impl Arbitrary for usize {
       fn arbitrary() -> Self::Strategy { 0..100 }

       type Strategy = Range<usize>;
   }
   ```

   By allowing defaults, we can transition to this more flexible API without
   breaking any consumers by simply saying:

   ```rust
   trait Arbitrary: Sized + fmt::Debug {
       type Parameters: Default = ();
   
       fn arbitrary() -> Self::Strategy {
           Self::arbitrary_with(Default::default())
       }

       fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
           Self::arbitrary()
           // This co-recursive definition will blow the stack.
           // However; since we can assume that previous implementors
           // actually provided a definition for `arbitrary` that
           // can't possibly reference `arbitrary_with`, we are OK.
           // You would only run into trouble for new implementations;
           // but that can be dealt with in documentation.
       }

       type Strategy: Strategy<Value = Self>;
   }
   ```

   The implementation `Arbitrary for usize` *remains valid* even after the change.

## For `default { .. }` groups

Finally, because we are making [changes] to how associated type defaults work
in this RFC, a new mechanism is required to regain the loss of expressive power
due to these changes. This mechanism is described in the section on
[`default { .. }` groups][default_groups] as alluded to in the [summary].

These groups not only retain the expressive power due to [RFC 192] but extend
power such that users get fine grained control over what things may and may not
be overridden together. In addition, these groups allow users to assume the
definition of type defaults in other items in a way that preserves soundness.

Examples where it is useful for other items to assume the default of an
associated type include:

[issue#29661]: https://github.com/rust-lang/rust/issues/29661

[comment174527854]: https://github.com/rust-lang/rust/issues/29661#issuecomment-174527854
[comment280944035]:https://github.com/rust-lang/rust/issues/29661#issuecomment-280944035

1. [A default method][comment174527854] whose
   [return type is an associated type:][comment280944035]

   ```rust
   /// "Callbacks" for a push-based parser
   trait Sink {
       fn handle_foo(&mut self, ...);
   
       default {
           type Output = Self;
   
           // OK to assume what `Output` really is because any overriding
           // must override both `Outout` and `finish`.
           fn finish(self) -> Self::Output { self }
       }
   }
   ```

2. There are plenty of other examples in [rust-lang/rust#29661][issue#29661].

[issue#31844]: https://github.com/rust-lang/rust/issues/31844

3. Other examples where `default { .. }` would have been useful can be found
   in the [tracking issue][issue#31844] for [specialization]:

   + <https://github.com/rust-lang/rust/issues/31844#issuecomment-198853202>

     You can see `default { .. }` being used
     [here](https://github.com/rust-lang/rust/issues/31844#issuecomment-249355377).

   + <https://github.com/rust-lang/rust/issues/31844#issuecomment-230093545>
   + <https://github.com/rust-lang/rust/issues/31844#issuecomment-247867693>
   + <https://github.com/rust-lang/rust/issues/31844#issuecomment-263175793>
   + <https://github.com/rust-lang/rust/issues/31844#issuecomment-279350986>

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## Associated type defaults

### Background and The status quo
[background]: #background-and-the-status-quo

Let's consider a simple trait with an associated type and another item (1):

```rust
trait Foo {
    type Bar;

    const QUUX: Self::Bar;

    fn wibble(x: Self::Bar) -> u8;
}
```

Ever since [RFC 192],
Rust has been capable of assigning default types to associated types as in (2):

```rust
#![feature(associated_type_defaults)]

trait Foo {
    type Bar = u8;

    const QUUX: Self::Bar = 42u8;

    fn wibble(x: Self::Bar) -> u8 { x }
}
```

However, unlike as specified in [RFC 192], which would permit (2),
the current implementation rejects (2) with the following error messages (3):

```rust
error[E0308]: mismatched types
 --> src/lib.rs:6:29
  |
6 |     const QUUX: Self::Bar = 42u8;
  |                             ^^^^ expected associated type, found u8
  |
  = note: expected type `<Self as Foo>::Bar`
             found type `u8`

error[E0308]: mismatched types
 --> src/lib.rs:8:37
  |
8 |     fn wibble(x: Self::Bar) -> u8 { x }
  |                                --   ^ expected u8, found associated type
  |                                |
  |                                expected `u8` because of return type
  |
  = note: expected type `u8`
             found type `<Self as Foo>::Bar`
```

The compiler rejects snippet (2) to preserve the soundness of the type system.
It must be rejected because a user might write (4):

```rust
struct Bar { ... }

impl Foo for Bar {
    type Bar = Vec<u8>;
}
```

Given snippet (4), `Self::Bar` will evaluate to `Vec<u8>`,
which is therefore the type of `<Bar as Foo>::QUUX`.
However, we have not given a different value for the constant,
and so it must be `42u8`, which has the type `u8`.
Therefore, we have reached an inconsistency in the type system:
`<Bar as Foo>::QUUX` is of value `42u8`, but of type `Vec<u8>`.
So we may accept either `impl Foo for Bar` as defined in (4),
or the definition of `Foo` as in (2), but not *both*.

[RFC 192] solved this dilemma by rejecting the implementation
and insisting that if you override *one* associated type,
then you must override *all* other defaulted items.
Or stated in its own words:

> + If a trait implementor overrides any default associated types,
>   they must also override all default functions and methods.
> + Otherwise, a trait implementor can selectively override individual
>   default methods/functions, as they can today.

Meanwhile, as we saw in the error message above (3),
the current implementation takes the alternative approach of accepting
`impl Foo for Bar` (4) but not the definition of `Foo` as in (2).

### Changes in this RFC
[changes]: #changes-in-this-rfc

In this RFC, we change the approach in [RFC 192] to the currently implemented
approach. Thus, you will continue to receive the error message above
and you will be able to provide associated type defaults.

[specialization]: https://github.com/rust-lang/rfcs/pull/1210

With respect to [specialization], the behaviour is the same.
That is, if you write (5):

```rust
#![feature(specialization)]

trait Foo {
    type Bar;

    fn quux(x: Self::Bar) -> u8;
}

struct Wibble<T>;

impl<T> Foo for Wibble<T> {
    default type Bar = u8;

    default fn quux(x: Self::Bar) -> u8 { x }
}
```

The compiler will reject this because you are not allowed to assume,
just like before, that `x: u8`. The reason why is much the same as
we have previously discussed in the [background].

[current_impl_diverge]: https://play.rust-lang.org/?gist=30e01d77f7045359e30c7d3f3144e984&version=nightly&mode=debug&edition=2015

One place where this proposal diverges from what is currently implemented
is with respect to the [following example][current_impl_diverge] (6):

```rust
#![feature(associated_type_defaults)]

trait Foo {
    type Bar = usize;

    fn baz(x: Self::Bar) -> usize;
}

impl<T> Foo for Vec<T> {
    fn baz(x: Self::Bar) -> usize { x }
}
```

In the current implementation, (6) is rejected because the compiler will not
let you assume that `x` is of type `usize`. But in this proposal, you would be
allowed to assume this. To permit this is not a problem because `Foo for Vec<T>`
is not further specializable since `baz` in the implementation has not been
marked as `default`.

With these changes,
we consider the design of associated type defaults to be *finalized*.

## `default` specialization groups
[default_groups]: #default-specialization-groups

Note: Everything in this section assumes actual support for [specialization].

Now, you might be thinking: - *"Well, what if I __do__ need to assume that
my defaulted associated type is what I said in a provided method,
what do I do then?"*. Don't worry; We've got you covered.

To be able to assume that `Self::Bar` is truly `u8` in snippets (2) and (5),
you may henceforth use `default { .. }` to group associated items into atomic
units of specialization. This means that if one item in `default { .. }` is
overridden in an implementation, then all all the items must be. An example (7):

```rust
struct Country(&'static str);

struct LangSec { papers: usize }
struct CategoryTheory { papers: usize }

trait ComputerScientist {
    default {
        type Details = Country;
        const THE_DETAILS: Self::Details = Country("Scotland"); // OK!
        fn papers(details: Self::Details) -> u8 { 19 } // OK!
    }
}

// https://en.wikipedia.org/wiki/Emily_Riehl
struct EmilyRiehl;

// https://www.cis.upenn.edu/~sweirich/
struct StephanieWeirich;

// http://www.cse.chalmers.se/~andrei/
struct AndreiSabelfeld;

// https://en.wikipedia.org/wiki/Conor_McBride
struct ConorMcBride;

impl ComputerScientist for EmilyRiehl {
    type Details = CategoryTheory;

    // ERROR! You must override THE_DETAILS and papers.
}

impl ComputerScientist for StephanieWeirich {
    const THE_DETAILS: Country = Country("USA");
    fn papers(details: Self::Details) -> u8 { 86 }

    // ERROR! You must override Details.
}

impl ComputerScientist for AndreiSabelfeld {    
    type Details = LangSec;
    const THE_DETAILS: Self::Details = LangSec { papers: 90 };
    fn papers(details: Self::Details) -> u8 { details.papers }

    // OK! We have overridden all items in the group.
}

impl ComputerScientist for ConorMcBride {
    // OK! We have not overridden anything in the group.
}
```

You may also use `default { .. }` in implementations.
When you do so, everything in the group is automatically overridable.
For any items outside the group, you may assume their signatures,
but not the default definitions given. An example:

```rust
use std::marker::PhantomData;

trait Fruit {
    type Details;
    fn foo();
    fn bar();
    fn baz();
}

struct Citrus<S> { species: PhantomData<S> }
struct Orange<V> { variety: PhantomData<V> }
struct Blood;
struct Common;

impl<S> Fruit for Citrus<S> {
    default {
        type Details = bool;
        fn foo() {
            let _: Self::Details = true; // OK!
        }
        fn bar() {
            let _: Self::Details = true; // OK!
        }
    }

    fn baz() { // Removing this item here causes an error.
        let _: Self::Details = true;
        // ERROR! You may not assume that `Self::Details == bool` here.
    }
}

impl<V> Fruit for Citrus<Orange<V>> {
    default {
        type Details = u8;
        fn foo() {
            let _: Self::Details = 42u8; // OK!
        }
    }

    fn bar() { // Removing this item here causes an error.
        let _: Self::Details = true;
        // ERROR! You may not assume that `Self::Details == bool` here,
        // even tho we specified that in `Fruit for Citrus<S>`.
        let _: Self::Details = 22u8;
        // ERROR! Can't assume that it's u8 either!
    }
}

impl Fruit for Citrus<Orange<Common>> {
    default {
        type Details = f32;
        fn foo() {
            let _: Self::Details = 1.0f32; // OK!
        }
    }
}

impl Fruit for Citrus<Orange<Blood>> {
    default {
        type Details = f32;
    }

    fn foo() {
        let _: Self::Details = 1.0f32;
        // ERROR! Can't assume it is f32.
    }
}
```

So far our examples have always included an associated type.
However, this is not a requirement.
We can also group associated `const`s and `fn`s together or just `fn`s.
An example:

```rust
trait Foo {
    default {
        const BAR: usize = 3;

        fn baz() -> [u8; Self::BAR] {
            [1, 2, 3]
        }
    }
}

trait Quux {
    default {
        fn wibble() {
            ...
        }

        fn wobble() {
            ...
        }

        // For whatever reason; The crate author has found it imperative
        // that `wibble` and `wobble` always be defined together.
    }
}
```

### Case study
[case study]: #case-study

[RFC 2500]: https://github.com/rust-lang/rfcs/pull/2500

One instance where default groups could be useful to provide a more ergonomic
API is to improve upon [RFC 2500]. The RFC proposes the following API:

```rust
trait Needle<H: Haystack>: Sized {
    type Searcher: Searcher<H::Target>;
    fn into_searcher(self) -> Self::Searcher;

    type Consumer: Consumer<H::Target>;
    fn into_consumer(self) -> Self::Consumer;
}
```

However, it turns out that usually, `Consumer` and `Searcher` are
the same underlying type. Therefore, we would like to save the user
from some unnecessary work by letting them elide parts of the required
definitions in implementations.

One might imagine that we'd write:

```rust
trait Needle<H: Haystack>: Sized {
    type Searcher: Searcher<H::Target>;
    fn into_searcher(self) -> Self::Searcher;

    default {
        type Consumer: Consumer<H::Target> = Self::Searcher;
        fn into_consumer(self) -> Self::Consumer { self.into_searcher() }
    }
}
```

However, the associated type `Searcher` does not necessarily implement
`Consumer<H::Target>`. Therefore, the above definition would not type check.

However, we can encode the above construct by rewriting it slightly,
using the concept of partial implementations from [RFC 1210]:

```rust
default impl<H: Haystack> Needle for T
where Self::Searcher: Consumer<H::Target> {
    default {
        type Consumer = Self::Searcher;
        fn into_consumer(self) -> Self::Consumer { self.into_searcher() }
    }
}
```

Now we have ensured that `Self::Searcher` is a `Consumer<H::Target>`
and therefore, the above definition will type check.
Having done this, the API has become more ergonomic because we can
let users define instances of `Needle<H>` with half as much requirements.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Grammar
[grammar]: #grammar

The production `trait_item` is changed from:

```
trait_item
: trait_const
| trait_type
| trait_method
| maybe_outer_attrs item_macro
;
```

to:

```
trait_item
: trait_default
| trait_const
| trait_type
| trait_method
| maybe_outer_attrs item_macro
;

trait_default : DEFAULT '{' trait_item* '}' ;
```

Associated type defaults are already in the grammar.

## Semantics and type checking

### Associated type defaults

This section supersedes [RFC 192] with respect to associated type defaults.

Associated types can be assigned a default type in a `trait` definition:

```rust
trait Foo {
    type Bar = $default_type;

    $other_items
}
```

Any item in `$other_items`, which have any provided definitions,
may only assume that the type of `Self::Bar` is `Self::Bar`.
They may *not* assume that the underlying type of `Self::Bar` is `$default_type`.
This property is essential for the soundness of the type system.

When an associated type default exists in a `trait` definition,
it need not be specified in the implementations of that `trait`.
If implementations of that `trait` do not make that associated type
available for specialization, the `$default_type` may be assumed
in other items specified in the implementation.
If an implementation does make the associated type available for
further specialization, then other definitions in the implementation
may not assume the given underlying specified type of the associated type
and may only assume that it is `Self::TheAsociatedType`.

This applies generally to any item inside a `trait`.
You may only assume the signature of an item, but not any provided definition,
in provided definitions of other items.
For example, this means that you may not assume the value of an
associated `const` item in other items with provided definition
in a `trait` definition.

#### Interaction with `existential type`

[RFC 2071]: https://github.com/rust-lang/rfcs/blob/master/text/2071-impl-trait-existential-types.md#reference-existential-types

[RFC 2071] defines a construct `existential type Foo: Bar;` which is permitted
in associated types and results in an opaque type. This means that the nominal
type identity is hidden from certain contexts and only `Bar` is extensionally
known about the type wherefore only the operations of `Bar` is afforded.
This construct is sometimes written as `type Foo = impl Bar;` in conversation
instead.

[RFC 1210]: https://github.com/rust-lang/rfcs/blob/master/text/1210-impl-specialization.md#default-impls

With respect to this RFC, the semantics of `type Assoc = impl Bar;`
inside a trait definition, where `Assoc` is the name of the associated type,
is understood as what it means in terms of `default impl ..` as discussed
in [RFC 1210]. What this entails means in concrete terms is that given:

```rust
trait Foo {
    type Assoc = impl Bar;

    ...
}
```

the underlying type of `Assoc` stays the same for all implementations which
do not change the default of `Assoc`. The same applies to specializations.
With respect to type opacity, it is the same as that of `existential type`.

### Specialization groups

Implementations of a `trait` as well as `trait`s themselves may now
contain *"specialization default groups"* (henceforth: *"groups"*) as
defined by the [grammar].

Such a group is considered an *atomic unit of specialization*
and each item in such a group may be specialized / overridden.
This means that if *one* item is overridden in a group,
*all* items must be overridden in that group.

Items inside a group may assume the definitions inside the group.
Items outside of that group may not assume the definitions inside of it.

The parser will accept items inside `default { .. }` without a body.
However, such an item will later be rejected during type checking.

#### Nesting

There applies no restriction on the nesting of groups.
This means that you may nest them arbitrarily.
When nesting does occur, the atomicity applies as if the nesting were flattened.
However, with respect to what may be assumed, the rule above applies.
For example, you may write:

```rust
trait Foo {
    default {
        type Bar = u8;
        fn baz() {
            let _: Self::Bar = 1u8;
        }

        default {
            const SIZE: usize = 3;
            fn quux() {
                let_: [Self::Bar; Self::SIZE] = [1u8, 2u8, 3u8];
            }
        }
    }
}

impl Foo for () {
    type Bar = Vec<u8>;
    fn baz() {}
    const SIZE: usize = 5;
    fn quux() {}
}
```

#### Linting redundant `default`s

When in source code (but not as a consequence of macro expansion),
the following occurs, a warn-by-default lint (`redundant_default`) will be emitted:

```rust
default {
    ...

    default $item
//  ^^^^^^^ warning: Redundant `default`
//          hint: remove `default`.

    ...
}
```

# Drawbacks
[drawbacks]: #drawbacks

The main drawbacks of this proposal are that:

1. `default { .. }` is introduced, adding to the complexity of the language.

   However, it should be noted that token `default` is already accepted for
   use by specialization and for `default impl`.
   Therefore, the syntax is only partially new.

2. if you have implementations where you commonly need to write `default { .. }`
   because you need to assume the type of an associated type default in a
   provided method, then the solution proposed in this RFC is less ergonomic.

   However, it is the contention of this RFC that such needs will be less common.
   This is discussed below.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Alternatives

The main alternative is to retain the behaviour in [RFC 192] such that
you may assume the type of associated type defaults in provided methods.
As noted in the [drawbacks] section,
this would be useful for certain types of APIs.
However, it is more likely than not that associated type defaults will
be used as a mechanism for code reuse than for other constructs.
As such, we consider the approach in this RFC to be the more ergonomic approach.

Another alternative to the mechanism proposed in this RFC is to somehow
track which methods rely on which associated types as well as constants.
However, we have historically had a strong bias toward being explicit
in signatures about such things, avoiding to infer them.
With respect to semantic versioning, such an approach may also cause
surprises for crate authors and their dependents alike.

## Consistency with associated `const`s

Consider the following valid example from stable Rust:

```rust
trait Foo {
    const BAR: usize = 1;

    fn baz() { println!("Hi I'm baz."); }
}

impl Foo for () {
    fn baz() { println!("Hi I'm () baz."); }
}
```

As we can see, you are permitted to override `baz` but leave `BAR` defaulted.
This is consistent with the behaviour in this RFC in that it has the same
property: *"you don't need to override all items if you override one"*.

Consistency and uniformity of any programming language is vital to make
its learning easy and to rid users of surprising corner cases and caveats.
By staying consistent, as shown above, we can reduce the cost to our complexity
budget that associated type defaults incur.

## Overriding everything is less ergonomic

We have already discussed this to some extent.
Another point to consider is that Rust code frequently sports traits such as
`Iterator` and `Future` that have many provided methods and few associated types.
While these particular traits may not benefit from associated type defaults,
many other traits, such as `Arbitrary` defined in the [motivation], would.

## `default { .. }` is syntactically light-weight

When you actually do need to assume the underlying default of an associated type
in a provided method, `default { .. }` provides a syntax that is comparatively
not *that* heavy weight.

In addition, when you want to say that multiple items are overridable,
`default { .. }` provides less repetition than specifying `default` on
each item would. Thus, we believe the syntax is ergonomic.

Finally, `default { .. }` works well and allows the user a good deal of control
over what can and can't be assumed and what must be specialized together.
The grouping mechanism also composes well as seen in
[the section where it is discussed][default_groups].

# Prior art
[prior-art]: #prior-art

## Haskell

[associated type defaults]: https://www.microsoft.com/en-us/research/wp-content/uploads/2005/01/at-syns.pdf

As Rust traits are a form of type classes,
we naturally look for prior art from were they first were introduced.
That language, being Haskell,
permits a user to specify [associated type defaults].
For example, we may write the following legal program:

```haskell
{-# LANGUAGE TypeFamilies #-}

class Foo x where
  type Bar x :: *
  -- A default:
  type Bar x = Int

  -- Provided method:
  baz :: x -> Bar x -> Int
  baz _ _ = 0

data Quux = Quux

instance Foo Quux where
  baz _ y = y
```

As in this proposal, we may assume that `y :: Int` in the above snippet.

In this case, we are not assuming that `Bar x` unifies with `Int` in the `class`.
Let's try to assume that now:

```haskell
{-# LANGUAGE TypeFamilies #-}

class Foo x where
  type Bar x :: *
  -- A default:
  type Bar x = Int

  -- Provided method:
  baz :: x -> Bar x -> Int
  baz _ barX = barX
```

This snippet results in a type checking error (tested on GHC 8.0.1):

```
main.hs:11:16: error:
    • Couldn't match expected type ‘Int’ with actual type ‘Bar x’
    • In the expression: barX
      In an equation for ‘baz’: baz _ barX = barX
    • Relevant bindings include
        barX :: Bar x (bound at main.hs:11:9)
        baz :: x -> Bar x -> Int (bound at main.hs:11:3)
<interactive>:3:1: error:
```

The thing to pay attention to here is:
> Couldn't match expected type ‘`Int`’ with actual type ‘`Bar x`’

We can clearly see that the type checker is now allowing us to assume
that `Int` and `Bar x` are the same type.
This is consistent with the approach this RFC proposes.

To our knowledge, Haskell does not have any means such as `default { .. }`
to change this behaviour. Presumably, this is the case because Haskell
preserves parametricity and lacks specialization,
wherefore `default { .. }` might not carry its weight.

## Idris

[idris_interface]: http://docs.idris-lang.org/en/latest/tutorial/interfaces.html
[coherence]: http://blog.ezyang.com/2014/07/type-classes-confluence-coherence-global-uniqueness/

Idris has a concept it calls [`interface`s][idris_interface].
These resemble type classes in Haskell, and by extension traits in Rust.
However, unlike Haskell and Rust, these `interface`s do not have the property
of [coherence] and will permit multiple implementations of the same interface.

Since Idris is language with full spectrum dependent types,
it does not distinguish between terms and types, instead, types are terms.
Therefore, there is really not a distinct concept called "associated type".
However, an `interface` may require certain definitions to be provided
and this includes types. For example, we may write:

```idris
interface Iterator self where
    item : Type
    next : self -> Maybe (self, item)

implementation Iterator (List a) where
    item = a
    next [] = Nothing
    next (x :: xs) = Just (xs, x)
```

Like in Haskell, in Idris, a function or value in an interface may be given a
default definition. For example, the following is a valid program:

```idris
interface Foo x where
    bar : Type
    bar = Bool

    baz : x -> bar

implementation Foo Int where
    baz x = x == 0
```

However, if we provide a default for `baz` in the `interface` which assumes
the default value `Bool` of `bar`, as with the following example:

```idris
interface Foo x where
    bar : Type
    bar = Bool

    baz : x -> bar
    baz _ = True
```

then we run into an error:

```
Type checking .\foo.idr
foo.idr:6:13-16:
  |
6 |     baz _ = True
  |             ~~~~
When checking right hand side of Main.default#baz with expected type
        bar x _

Type mismatch between
        Bool (Type of True)
and
        bar x _ (Expected type)
```

The behaviour here is exactly as in Haskell and as proposed in this RFC.

## C++

In C++, it is possible to provide associated types and specialize them as well.
This is shown in the following example:

```cpp
#include <iostream>
#include <string>

template<typename T> struct wrap {};

template<typename T> struct foo { // Unspecialized.
    using bar = int;

    bar make_a_bar() { return 0; };
};

template<typename T> struct foo<wrap<T>> { // Partial specialization.
    using bar = std::string;

    bar make_a_bar() { return std::string("hello world"); };
};

int main() {
    foo<void> a_foo;
    std::cout << a_foo.make_a_bar() << std::endl;

    foo<wrap<void>> b_foo;
    std::cout << b_foo.make_a_bar() << std::endl;
}
```

You will note that C++ allows us to assume in both the base template class,
as well as the specialization, that `bar` is equal to the underlying type.
This is because one cannot specialize any part of a class without specializing
the whole of it. It's equivalent to one atomic `default { .. }` block.

## Swift

[swift_assoc]: https://docs.swift.org/swift-book/LanguageGuide/Generics.html

One language which does have [associated types][swift_assoc] and defaults but
which does not have provided definitions for methods is Swift.
As an example, we may write:

```swift
protocol Foo {
    associatedtype Bar = Int

    func append() -> Bar
}

struct Quux: Foo {
    func baz() -> Bar {
        return 1
    }
}
```

However, we may not write:

```swift
protocol Foo {
    associatedtype Bar = Int

    func append() -> Bar { return 0 }
}
```

This would result in:

```
main.swift:4:23: error: protocol methods may not have bodies
    func baz() -> Bar { return 0 }
```

## Scala

Another language which allows for these kinds of type projections and defaults
for them is Scala. While Scala does not have type classes like Rust and Haskell
does, it does have a concept of `trait` which can be likened to a sort of
incoherent "type class" system. For example, we may write:

```scala
trait Foo {
    type Bar = Int

    def baz(x: Bar): Int = x
}

class Quux extends Foo {
    override type Bar = Int
    override def baz(x: Bar): Int = x
}
```

There are a few interesting things to note here:

1. We are allowed to specify a default type `Int` for `Bar`.

2. A default definition for `baz` may be provided.

3. This default definition may assume the default given for `Bar`.

4. However, we *must* explicitly state that we are overriding `baz`.

5. If we change the definition of of `override type Bar` to `Double`,
   the Scala compiler will reject it.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

1. Should trait objects default to the one specified in the trait if
   an associated type is omitted? In other words, given:

   ```rust
   trait Foo {
       type Bar = usize;
       fn baz(&self) -> Self::Bar;
   }

   type Quux = Box<dyn Foo>;
   ```

   Should `Quux` be considered well-formed and equivalent to the following?

   ```rust
   type Quux = Box<dyn Foo<Bar = usize>>;
   ```

   This question may be left as future work for another RFC or resolved
   during this RFC as the RFC is forward-compatible with such a change.

2. Should groups be arbitrarily nestable?

   On the one hand, permitting arbitrary nesting is simpler from a grammatical
   point of view and makes the language simpler by having *fewer rules*.
   It also allows the user more fine grained control.

   On the other hand, it is not clear to what use such fine grained control
   would be. Nested groups may also be less understandable and lead to confusion.

   To resolve this issue, some usage experience may be required.
   Thus, it might be a good idea to defer such a choice until after the RFC.

# Future work

## `where` clauses on `default { .. }` groups

From our [case study], we noticed that we had to depart from our `trait`
definition into a separate `default impl..` to handle the conditionality
of `Self::Searcher: Consumer<H::Target>`. However, one method to regain
the locality provided by having `default { .. }` inside the `trait` definition
is to realize that we could attach an optional `where` clause to the group.
This would allow us to write:

```rust
trait Needle<H: Haystack>: Sized {
    type Searcher: Searcher<H::Target>;
    fn into_searcher(self) -> Self::Searcher;

    default where
        Self::Searcher: Consume<H::Target>
    {
        type Consumer: Consumer<H::Target> = Self::Searcher;
        fn into_consumer(self) -> Self::Consumer { self.into_searcher() }
    }
}
```

The defaults in this snippet would then be equivalent to the `default impl..`
snippet noted in the [case study].

This `default where $bounds` construct should be able to
subsume common cases where you only have a single `default impl..`
but provide comparatively better local reasoning.

However, we do not propose this at this stage because it is unclear how
common `default impl..` will be in practice.
