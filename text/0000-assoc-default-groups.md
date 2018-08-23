- Feature Name: `assoc_default_groups`
- Start Date: 2018-08-23
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

   For example, we may provide an API (due to [proptest]):

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

With these changes,
we consider the design of associated type defaults to be *finalized*.

## `default` specialization groups
[default_groups]: #default-specialization-groups

Now, you might be thinking: - *"Well, what if I __do__ need to assume that
my defaulted associated type is what I said in a provided method,
what do I do then?"*. Don't worry; We've got you covered.

To be able to assume that `Self::Bar` is truly `u8` in snippets (2) and (5),
you may henceforth use `default { .. }` to group items into atomic units of
specialization. This means that if one item in `default { .. }` is overridden
in an implementation, then all all the items must be. An example (6):

```rust
trait ComputerScientist {
    default {
        type Bar = u8;
        const QUUX: Self::Bar = 42u8; // OK!
        fn wibble(x: Self::Bar) -> u8 { x } // OK!
    }
}

struct Alan;
struct Alonzo;
struct Kurt;
struct Per;

impl ComputerScientist for Alan {
    type Bar = Vec<u8>;

    // ERROR! You must override QUUX and wibble.
}

impl ComputerScientist for Alonzo {
    const QUUX: u8 = 21;
    fn wibble(x: Self::Bar) -> u8 { 4 }

    // ERROR! You must override Bar.
}

impl ComputerScientist for Kurt {    
    type Bar = (u8, u8);
    const QUUX: Self::Bar = (0, 1);
    fn wibble(x: Self::Bar) -> u8 { x.0 }

    // OK! We have overridden all items in the group.
}

impl ComputerScientist for Per {
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

However, please note that for use of `default { .. }` inside implementations,
you will still need actual support for [specialization].

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

trait_default : DEFAULT '{' (trait_item)* '}' ;
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

Any item in `$other_items`,
which have any provided definitions (henceforth: *"default"*),
may only assume that the type of `Self::Bar` is `Self::Bar`.
They may *not* assume that the underlying type of `Self::Bar` is `$default_type`.
This property is essential for the soundness of the type system.
When an associated type default exists in a `trait` definition,
it need not be specified in the implementations of that `trait`.

This applies generally to any item inside a `trait`.
You may only assume the signature of an item, but not any default,
in defaults of other items. This also includes `impl` items for that trait.
For example, this means that you may not assume the value of an
associated `const` item in other item with a default.

### Specialization groups

Implementations of a `trait` as well as `trait`s themselves may now
contain *"specialization default groups"* (henceforth: *"group"*) as
defined by the [grammar].

Such a group is considered an *atomic unit of specialization* and
each item in such a group may be specialized / overridden.
This means that if *one* item is overridden in a group,
*all* items must be overridden in a group.

Items inside a group may assume the defaults inside the group.
Items outside of that group may not assume the defaults inside of it.

#### Nesting

There applies no restriction on the nesting of groups.
This means that you may nest them arbitrarily.
When nesting does occur, the atomicity applies as if the nesting was flattened.
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

2. secondarily, if you have implementations where it is commonly needed
   to write `default { .. }` because you need to assume the type of an
   associated type default in a provided method, then the solution proposed
   in this RFC is less ergonomic.

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

As Rust traits are a form of type classes,
we naturally look for prior art from were they first were introduced.
That language, being Haskell, permits a user to specify associated type defaults.
For example, we may write:

```haskell
{-# LANGUAGE TypeFamilies #-}

class Foo x where
  type Bar x :: *
  -- A default:
  type Bar x = Int

  -- Provided method:
  baz :: x -> Bar x -> Int
  baz _ _ = 0
```

In this case, we are not assuming that `Bar x` unifies with `Int`.
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
