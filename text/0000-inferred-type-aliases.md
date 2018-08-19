- Feature Name: `inferred_type_aliases`
- Start Date: 2018-08-14
- RFC PR: _
- Rust Issue: _

# Summary
[summary]: #summary

Permit type aliases and associated types to have their
types be *[inferred]* in such a way that their nominal
types be *[transparent]* as opposed to *[opaque]* like so:

```rust
type Foo = _;

impl Iterator for Bar {
    type item = _;
}
```

You may also optionally constrain a type alias or associated type with a bound
by writing `type Alias: Bound = <definition>;`. This provides the *minimum*
capability of the alias as opposed to the *maximum* capability (see [RFC 2071]).

# Motivation
[motivation]: #motivation

[RFC 1522]: https://github.com/rust-lang/rfcs/pull/1522
[RFC 1951]: https://github.com/rust-lang/rfcs/pull/1951
[RFC 2071]: https://github.com/rust-lang/rfcs/pull/2071
[RFC 2250]: https://github.com/rust-lang/rfcs/pull/2250
[RFC 2515]: https://github.com/rust-lang/rfcs/pull/2515

The accepted [RFC 2071] introduced the ability to create what
it calls "existential type aliases" with the temporary syntax
`existential type Alias: Bar;`. These existential aliases exist
to satisfy two needs:

1. *Encapsulation* ("Abstraction").

   An `existential type Alias: Bar` only affords the operations that the bound
   `Bar` affords (+ `auto` trait leakage). This gives the user the ability
   to hide the concrete underlying type thereby making it possible to not
   overpromise and to change the underlying type at a later point in time.

2. Naming [unnameable types][unnameable type].

   Before [RFC 1522] it was impossible to name unnameable types such as
   closures. Thus users had to `Box` their `Iterator`s (or similar) and
   resort to dynamic dispatch. When the RFC was added, it became possible
   to say things like `fn foo() -> impl Iterator<Item = T> { .. }`.
   However, this ability was per-function and did not extend to associated types.
   This situation was later rectified by [RFC 2071].

This RFC does not aim to solve use case 1). In fact, the opposite is the case.
The mechanism proposed here should not be used for encapsulation.
What this RFC does aim to solve is:

## Better naming of [unnameable types][unnameable type]

When you use `existential type` to name a unnameable type you will be either
unable, due to generic parameters on the alias or type parameters on implemented
traits, to name all the traits that are needed to fully capture what the unnamed
type is capable of, or you may have to enumerate a non-trivial number of traits
to get the full list of capabilities.

For example, consider:

```rust
#![feature(existential_type)]

existential type IncIter: Iterator<Item = usize>;

fn make_iterator() -> IncIter {
    (0..10).map(|x: usize| x + 1)
}
```

The closure `|x: usize| x + 1` is a [unnameable type] and thus the entire
underlying return type of `make_iterator` is also unnameable.
The underlying type implements `Clone`, but the compiler will not permit
us to observe this fact from `make_iterator` because the fact that it
implements `Clone` has been hidden away. There are plenty more traits
that `IncIter` doesn't afford that it could:

```rust
#![feature(existential_type, trusted_len, try_from)]

use std::iter::{TrustedLen, FusedIterator};
use std::fmt::Debug;
use std::convert::TryFrom;

existential type IncIter:
    Clone +
    Debug +
    Iterator<Item = usize> +
    DoubleEndedIterator +
    TrustedLen +
    ExactSizeIterator +
    FusedIterator +
    'static;

fn make_iterator() -> IncIter {
    (0..10).map(|x: usize| x + 1)
}
```

However, naming all of the traits as done above does not scale well.
This RFC [proposes][the proposal] to rectify by using the
[transparent] and [inferred] `type IncIter = _;` instead.
Using this mechanism, we can truly get access to *all* the
operations that `(0..10).map(|x: usize| x + 1)` affords.
Exposing all operations of a type in this way works well for the
implementation details of a library or for an application since
there is no need to guarantee backwards compatibility for those details.

## Inference in `#[derive(MyTrait)]` macros

[proptest_drive]: https://github.com/AltSysrq/proptest/pull/79

Consider that we have some sort of custom derive macro for a trait with an
associated type. Also consider that the derive macro allows customization
of the derived implementation via attributes on the type definition.
As an example, we have `#[derive(Arbitrary)]` for [proptest_drive]:

```rust
#[derive(Debug, Arbitrary)]
enum CitrusFruit {
    Orange {
        sort: String,
        #[proptest(strategy = "generate_origin_country()")]
        origin: Country,
        #[proptest(value = "47")]
        calories: usize,
    },
    Lemon {
        sweet: bool,
    },
    Lime {
        #[proptest(strategy = "3f32..=6")]
        diameter: f32,
    },
}
```

This would generate roughly the following (simplified for readability):

```rust
impl pt::Arbitrary for CitrusFruit {
    type Parameters = (
        (<String as pt::Arbitrary>::Parameters),
        (<bool as pt::Arbitrary>::Parameters),
    );

    type Strategy = pt::TupleUnion<(
        (
            u32,
            pt::Map<
                (
                    <String as pt::Arbitrary>::Strategy,
                    // Note this line!
                    pt::BoxedStrategy<Country>,
                    fn() -> usize,
                ),
                fn((String, Country, usize)) -> Self,
            >,
        ),
        (
            u32,
            pt::Map<(<bool as pt::Arbitrary>::Strategy,), fn((bool,)) -> Self>,
        ),
        (u32, pt::Map<(pt::BoxedStrategy<f32>,), fn((f32,)) -> Self>),
    )>;

    fn arbitrary_with(_top: Self::Parameters) -> Self::Strategy {
        let (param_0, param_1) = _top;
        pt::TupleUnion::new((
            (
                1u32,
                pt::Strategy::prop_map(
                    (
                        pt::any_with::<String>(param_0),
                        pt::Strategy::boxed(generate_origin_country()),
                        || 47,
                    ),
                    |(t0, t1, t2)| Orange { sort: t0, origin: t1, calories: t2, },
                ),
            ),
            (1u32, {
                let param_0 = param_1;
                pt::Strategy::prop_map(
                    (pt::any_with::<bool>(param_0),),
                    |(t0,)| CitrusFruit::Lemon { sweet: t0 }
                )
            }),
            (
                1u32,
                pt::Strategy::prop_map(
                    (pt::Strategy::boxed(3f32..=6),),
                    |(t0,)| CitrusFruit::Lime { diameter: t0 }
                ),
            ),
        ))
    }
}
```

The details of the code inside `arbitrary_with` are not very important.
Suffice it to say that the type of `Strategy` (and `Parameter`) is
*fully determined* by the code inside `arbitrary_with`.
However, the definition of `Strategy` is more or less an implementation
detail of the macro and not particularly relevant for anyone who wants
to understand what the implementation does.

To generate the definition of `type Strategy`,
100s of lines of code is required inside the procedural macro
corresponding to `#[derive(Arbitrary)]`.
This *greatly* complicates the development of the macro.

What's worse, if you note the line `pt::BoxedStrategy<Country>`,
this occurs because the procedural macro can't know the type of
`generate_origin_country()` because macros have no access to
type information beyond what can be inferred from the `TokenStream`
given to it corresponding to the text of the type definition itself.
As a result, the macro has no choice but to box the result of
`generate_origin_country()`.
This situation could be improved by writing (in the style of [RFC 2071]):

```rust
existential type Strategy: Strategy<Value = Self>;
```

However, once we do this, we have reintroduced the problems previously
discussed with respect to [opacity][opaque]. There are more operations
that the type of the returned expression inside `arbitrary_with` affords
such as being `Clone`able. The macro can't add a bound `+ Clone` to the
`Strategy` because there are might be situations where this does not hold.

By using `_` we can both simplify the procedural macro greatly as well
as exposing all operations the generated code affords:

```rust
type Strategy = _;
```

Simpler macros, including of the `macro_rules!` flavor,
can similarly be simplified and require less input from the user by using `_`.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## Vocabulary

First, let's get some useful vocabulary out of the way.

### `inferred`
[inferred]: #inferred

[type_inference]: https://en.wikipedia.org/wiki/Type_inference

By [*"inferred"*][type_inference] we refer to the compiler automatically
determining type of some object such as an expression for you.

For example, when you write:

```rust
let yummy_food = "Tikka masala";

// or equivalently:

let yummy_food: _ = "Tikka masala";
```

the compiler will automatically infer that the type of `yummy_food`
is `&'static str` here.

### `transparent`
[transparent]: #transparent

For something to be *"transparent"* means that you can see through it.

In the context of Rust and in this RFC in particular we mean that for the
`type` of some object / expression to be transparent means that its true
nominal type can be obtained by the user. This entails that if you have
some expression of that type, you can perform all operations the type affords.

An example:

```rust
let foo: _     = make_some_number();
// The compiler permit this because the type of `foo` is transparent:
let bar: usize = foo;
```

### `opaque`
[opaque]: #opaque

The opposite of something being transparent is for something to be *"opaque"*.
This means that you can *not* see through it. In the context of Rust,
this refers to that the true nominal type of some object or expression
is hidden from the user. Instead only a particular interface is exposed.
This entails that if you have such an expression, only the operations
afforded by the interface may be performed. This has been provided by
the `impl Trait` (see [RFC 1522], [RFC 1951], [RFC 2071], [RFC 2250],
and currently proposed [RFC 2515]) mechanism in Rust.

An example:

```rust
use std::ops::Range;

fn one_to_hundred() -> impl Iterator<Item = u8> {
    let range: Range<u8> = 0..100;
    range
}

fn main() {
    let _numbers: Range<u8> = one_to_hundred();
}
```

This results in:

```rust
error[E0308]: mismatched types
 --> src/main.rs:9:30
  |
9 |     let _numbers: Range<u8> = one_to_hundred();
  |                               ^^^^^^^^^^^^^^^^ expected struct `std::ops::Range` found anonymized type
  |
  = note: expected type `std::ops::Range<u8>`
             found type `impl std::iter::Iterator`

error: aborting due to previous error
```

Here *"anonymized"* and *"opaque"* mean the same thing.
As we can see from the error message, even though we know that the nominal type
of `_numbers` is `Range<u8>`, the compiler has prevented us from concluding this.

### `unnameable type`
[unnameable type]: #unnameable-type

A *"unnameable type"* in Rust refers to types which *can not be named*.
They are also sometimes called *"voldemort types"*. The canonical example of
such unnameable types are the types of closures.
Each closure has a distinct type, which the compiler knows about,
but you can't write their name in code.

An example:

```rust
#![feature(core_intrinsics)]

use std::intrinsics::type_name;

fn name_of_type<T>(_: &T) -> &str {
    unsafe { type_name::<T>() }
}

fn main() {
    let closure_one = |x: usize| x + 1;
    let closure_two = |x: usize| x + 1;
    println!("{}", name_of_type(&closure_one));
    // output: [closure@src/main.rs:10:23: 10:39]
    println!("{}", name_of_type(&closure_two));
    // output: [closure@src/main.rs:11:23: 11:39]
}
```

As you can see from the output, the type of these two closures are *not* the same.

## The Proposal
[the proposal]: #the-proposal

Now that we've defined the core concepts this RFC deals with,
let's deal with what is actually proposed.

This part is rather simple.
What this RFC proposes is that you should be able to have the actual type of a
`type` alias as well as the associated type of an implementation be [inferred]
and that these inferred types should be [transparent] as opposed to [opaque].
We propose to do this by using the familiar symbol `_` which means
*"please infer the type"* in type contexts.

For example, you will be allowed to write:

```rust
use std::ops::Range;

type MyIterator = _;

fn one_to_hundred() -> MyIterator {
    let range: Range<u8> = 0..100;
    range
}

fn main() {
    // OK! because this is transparent inference.
    let _numbers: Range<u8> = one_to_hundred();
}
```

You can also optionally enforce that `MyIterator` must at least be an `Iterator`:

```rust
type MyIterator: Iterator<Item = u8> = _;
```

This works the same way in associated types:

```rust
// Let's pretend this type is more interesting in actuality.
#[derive(Debug)]
struct SomeImportantStuff;
struct MyType;
trait MyTrait {
    type MyImplementationDetail;
    fn my_public_api(&self) -> Self::MyImplementationDetail;    
}

// Our implementation:
impl MyTrait for MyType {
    // Here we employ type inference to our aid:
    type MyImplementationDetail = _;

    fn my_public_api(&self) -> Self::MyImplementationDetail {
        // Here goes logic that determines the
        // actual type of `MyImplementationDetail`...
        SomeImportantStuff
    }
}

fn main() {
    // OK! Inferred type is transparent.
    let _detail: SomeImportantStuff = MyType.my_public_api();
}
```

Same as with type aliases, you may also enforce a trait bound:

```rust
    // Still transparent and will leak everything about the type:
    type MyImplementationDetail: Debug = _;
```

Another way to think of the feature proposed in this RFC is that it is the
`_` inside `type Alias = _;` is the [transparent] version of the [opaque]
and [inferred] `type Alias = impl Trait;` ([RFC 2515]) or equivalently
`existential type Alias: Trait;` ([RFC 2071]).

To make matters more concrete, `type Alias = impl Trait;`,
can be thought of as semantically equivalent to:

```rust
type AliasRepr: Bar = _;

#[repr(transparent)]
struct Alias {
    representation: AliasRepr
}

impl Bar for Alias {
    // delegate everything to self.representation
}
```

In this case, the inference properties of [RFC 2071] are retained
since `type Alias = impl Trait` has the same properties.
This example does not take privacy into account,
so it is not a literal desugaring, but it does illustrate
how `existential type` behaves beyond that.

## Dos and Don’ts
[do_dont]: #dos-and-don'ts

1. **Do not** use `_` inside type aliases as a means of *abstraction*.
  Aliases that use `_` inside them will *leak* all information about the
  exact type that is inferred. Changes to the capabilities / affordances /
  operations offered by the underlying inferred type will cause breakage in
  consumers of your APIs that depend on those affordances. Therefore, be extra
  careful when using `pub` on `type Alias = _;` or when using `_` for an
  associated type of an implementation for a public trait + public type.

2. **Do** use `_` as a means of eliding complex implementation details that
  are not important to the users of your API.
  This applies in particular to custom derive macros for traits with
  associated types.

3. **Do**, assuming that you keep in mind 1), use `_` as a means to name the
   type of an expression with a [unnameable type] somewhere inside it.

## Teaching this

The teaching of the [transparent] and [inferred] types can be explained in
conjunction with explaining the usual `_` feature in other places.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Grammar

The grammar of `type` aliases and associated `type`s are extended to permit
an optional bound (*"trait ascription"* if you will).
To do this, we replace the production:
```rust
impl_type : attrs_and_vis maybe_default item_type ;

item_type
: TYPE ident generic_params maybe_where_clause '=' ty_sum ';' ;
```

with:

```rust
impl_type : attrs_and_vis maybe_default item_type ;

item_type
: TYPE ident generic_params maybe_bounds maybe_where_clause '=' ty_sum ';' ;
```

## Type checking

Currently, if a user writes (associated type or type alias does not matter):

```rust
type Foo = _;
```

[`E0121`]: https://doc.rust-lang.org/stable/error-index.html#E0121

they are presented with the error [`E0121`]:

```rust
error[E0121]: the type placeholder `_` is not allowed within types on item signatures
  --> src/main.rs:11:12
   |
11 | type Foo = _;
   |            ^ not allowed in type signatures
```

This restriction is lifted by this RFC in `type` items.

Instead, the `_` "type" is permitted within `type`s.
However, each `type` item that uses `_` in any part of its definition
must be constrained by at least one `fn` body, or `const`/`static` initializer
(henceforth: *"determinant"*). All of these determinants must be in the same
module as the `type` item inferred and must independently place constraints
on the `type` item such that they determine the same underlying exact type for it.
A body or initializer must either fully constrain or place no constraints upon a given `type` item with `_` inside it.

A `type` item that uses `_` within its definition exposes its
full type identity globally. In other words, `_` acts exactly like
`existential type` (as specified in [RFC 2071]) does,
with the exception of [transparency][transparent].

Another difference between `_` in `type` and `existential type` is that
the former may be used in `impl` blocks.

[`E0277`]: https://doc.rust-lang.org/stable/error-index.html#E0277

When a `<bound>` is present on `type <name> <generics>?: <bound> = <definition>`
the type checker will check that `<definition>` satisfies the specified `<bound>`
or will issue [`E0277`] otherwise.

# Drawbacks
[drawbacks]: #drawbacks

The primary drawback to this feature is that can be *misused* for abstraction
(which it should *not* be used for) instead of using the feature for what it
was meant for (naming [unnameable types][unnameable type] and eliding
unimportant implementation details).

To mitigate this possibly, the use cases of using `_` should be highlighted
and it should be stressed that the feature is not meant for abstraction purposes.
The [guide-level-explanation] provides a series of [dos and don’ts][do_dont]
that may be used in teaching material.

Another possible mitigation strategy is to lint in the case of public aliases
which include `_` in their definitions. However, this will not help for
associated types.

[semverver]: https://crates.io/crates/semverver

Finally, another drawback is the somewhat increased risk of breaking changes
if and when users mistake `_` as something [opaque] or if they are simply
not careful enough when making changes. However, [interesting work][semverver]
is being done in the area of detecting breaking changes. Such a tool could
potentially also be used for detecting breakage where `_` is involved.

# Rationale and alternatives
[alternatives]: #rationale-and-alternatives

## Use cases

The use cases this RFC is intended to facilitate are discussed in the [motivation].
It follows then that if this RFC is not accepted, those use cases will either
have to be supported by some other means (such as `typeof`, see below)
or they will remain unsupported.

In particular, do note that this RFC enables things that are simply *impossible*
with today's Rust. In other words, what is suggested here is not mere syntactic
sugar, but represents instead an increase in the capabilities of the language.
To see examples of what is impossible, please refer back to the [motivation].

## On the globality of inference

One might get the impression that this RFC introduced some sort of
type inference that is *more global* than what exists in Rust today.
However, this is not the case. This RFC introduces inference that is
*as global* as `existential type` ([RFC 2071]) provides, *no more, no less*.

For example, the following snippet would not be accepted by the type checker
according to the specification of this RFC:

```rust
mod foo {
    // There is no function in the same module that fully determines what
    // the type of `Alias` is.
    pub type Alias = _;
}

// `Alias` is not defined in the same module as `bar` is.
fn bar() -> Alias {
    42usize
}
```

This is by design so that `type Foo = _;` is consistent with
how `existential type` works.

However, there is a choice here.
One alternative design, that is still not full global type inference,
could be to allow all determinants in a module to contribute to
the unification effort. They must however not contradict each other.
The design in this RFC should be forward compatible with such an
alternative as it would accept strictly more programs as well-formed.

## The rationale for `_` as a syntax

Simply put, we use `_` as a syntax because it is already permitted in `let`
bindings (as seen in the [guide-level-explanation]) as well as in turbofish
contexts (e.g. `iter.collect::<Vec<_>>()`). Using any other syntax would be
inconsistent and would unnecessarily complicate the language and the
understanding of it.

## Complexity increase?

We also note that because `_` is already permitted in the places aforementioned
as well as the introduction of `existential type Foo: Bar;` ([RFC 2071]),
or the proposed `type Foo = impl Trait;` syntax ([RFC 2515]), permitting `_`
inside `type` can be understood both in terms of how `_` works in other places
as well as the [transparent] version of the [opaque] `existential type`.

Thus, it could be argued that this RFC constitutes a net complexity decrease
or at the very least not a substantial increase in complexity.

## `typeof`

The main alternative to permitting `_` in type aliases is introducing
a `typeof` construct. This is possible since `typeof` is a reserved keyword.
The semantics of such a construct would be to take an expression and
evaluate the type of that expression. This corresponds to `decltype` in C++.

However, this alternative does not work well to allow us to elide
implementation details in associated types because you would either have to 
repeat the expression which determines the type, or you would, if possible,
have to refer to the return type of some method in the type explicitly by
writing something akin to:

```rust
<(typeof Self::the_method) as FnOnce>::Output
```

This construct can then be macro-ized as a `return_type!(..)` type macro
which may then be used as `return_type!(Self::the_method)`.
To be able to do this you would need to be able to annotate the return
type of a function with `typeof expr` where `expr` refers to the innards
of the function.

In either case, for this use case, `typeof`, or derived constructs,
will never be as ergonomic as the simple `_` construct because it will
require the user to provide the path of the function to get the type of.
For some, this explicit nature may be considered a feature.

Furthermore, as previously noted, we already permit `_` in other places
in the language. As such, the complexity cost of introducing `_` in one
more place is negligible relative to introducing `typeof`.

## In relation to [RFC 2515]

RFC 2515 changes the syntax of `existential type Foo: Bar;`
into `type Foo = impl Bar;` and proposes that we should think
of `impl Trait` in terms of [opaque] [inferred] types.
Such a step would work well with this RFC as this one provides
[transparent] [inferred] types.
They also work well on a purely syntactic level;
while [RFC 2515] lets `impl Trait` work in more places,
this RFC lets `_` work in more places.
Thus, the RFCs increase syntactic uniformity in much the same way.

## Justification for a minimal `Bound`

As noted in the [summary] and elsewhere, this RFC permits the user to
optionally specify a *minimal* bound on type aliases and associated types
in trait implementations.

It has been argued that this would be a comparatively large extension of
Rust's grammar for little practical benefit. However, in this section,
we outline a few benefits this extension does give us.

### Grammatical simplification

It might seem that this feature complicates the grammar and thereby the parser.
However, we observe that given associated type defaults (in nightly),
as well as `where` clauses on associated `type`s inside `trait`s (RFC 1598),
we can achieve a degree of grammatical unification.
For example, before this RFC, we had:

```
eq_type : '=' ty_sum ;
common : TYPE ident generic_params ;

trait_type
: maybe_outer_attrs
  common maybe_ty_param_bounds maybe_where_clause eq_type? ';' ;

impl_type
: attrs_and_vis maybe_default
  common eq_type ';'

item_type
: common maybe_where_clause eq_type ';' ;
```

With this RFC, we can simplify by moving `maybe_ty_param_bounds` into `common`:

```
eq_type : '=' ty_sum ;
common : TYPE ident generic_params maybe_ty_param_bounds ;

trait_type
: maybe_outer_attrs
  common maybe_where_clause eq_type? ';' ;

impl_type
: attrs_and_vis maybe_default
  common eq_type ';'

item_type
: common maybe_where_clause eq_type ';' ;
```

What is left then is introducing `maybe_where_clause` to trait implementations
which would give us:

```
eq_type : '=' ty_sum ;
common : TYPE ident generic_params maybe_ty_param_bounds maybe_where_clause ;
trait_type : maybe_outer_attrs common eq_type? ';' ;
impl_type : attrs_and_vis maybe_default item_type ';'
item_type : common eq_type ';' ;
```

This is the maximal unification possible.
However, this final step is left as possible future work.

The benefit of this increased grammatical unification, however small,
is that the language becomes more uniform, which is useful to reduce
surprises for users.

### Useful to explain [RFC 2071]

In the notes about [the proposal] we used the fact that this RFC proposes
a minimum bound to explain how one can understand `type Foo = impl Bar;`
in terms of `type AliasRepr: Bar = _;` as well as a new type wrapper.
If we don't have this capability, then the explanation does also not work.
However, this is a rather minor point.

### Useful for documentation

Even when you want to expose all the operations of some type,
there often is a key trait that is used around the trait.
We have already seen examples of this in the [motivation] with
the case of `Iterator` and `Strategy`. Documenting this key trait
can serve as a useful hint for users that the major role of the
alias is so and so.

### Useful as a guard against breakage

As we've previously discussed in the section on [drawbacks],
one problem with publicly exposing the underlying types of
these type aliases or associated types is that breakage may
become more likely.

As a guard against this risk, we can use these bounds to denote
the aforementioned key traits. This can help ensure that at least
for some key parts of the API, breakage can be detected by the
crate author as opposed to by the dependents of the crate.

### Alternative syntax

One possible alternative syntax instead of `type Foo: Bound = _` is to extend
the type grammar to allow a sort of "trait ascription". We do this by introducing
the following alternative to the `type` production:

```
type : type ":" bound ;
```

It then becomes possible to state things such as:

```rust
type Foo = _ : Debug;

// Assuming we allow `_` in this context which this RFC does not.
fn bar() -> _ : Debug {
    ...
}
```

While this alternative is a more composable construct, we instead propose
`type Foo: Bound = <definition>;` because it is more readable.

# Prior art
[prior-art]: #prior-art

To our knowledge, there exists no prior art in other languages for the contents
proposed in this RFC.

# Unresolved questions
[unresolved]: #unresolved-questions

1. Should a (warn-by-default) lint be emitted in the case of specifying
   `pub` on a type alias which has `_` somewhere in its definition?

# Possible future work
[future work]: #possible-future-work

Possible extensions to this RFC are:

+ allowing `_` in the type of `const` items.
+ allowing `_` in the type of `static` items.
+ allowing `_` in the return type of `fn` items.

These are possible to achieve indirectly with the feature proposed in this RFC. 
However, it is unclear whether this will be beneficial or not.
One advantage to `type Foo = _;` is that it at least gives a name
that can describe to other humans what the alias's role is.
Just writing `fn foo() -> _ { .. }` does not make this aspect clear.
