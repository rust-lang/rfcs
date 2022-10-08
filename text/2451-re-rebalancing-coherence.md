- Feature Name: `re_rebalancing_coherence`
- Start Date: 2018-05-30
- RFC PR: [rust-lang/rfcs#2451](https://github.com/rust-lang/rfcs/pull/2451)
- Rust Issue: [rust-lang/rust#55437](https://github.com/rust-lang/rust/issues/55437)

# Summary
[summary]: #summary

This RFC seeks to clarify some ambiguity from [RFC #1023], and expands it to
allow type parameters to appear in the type for which the trait is being
implemented, regardless of whether a local type appears before them. More
concretely, it allows `impl<T> ForeignTrait<LocalType> for ForeignType<T>` to be
written.

# Motivation
[motivation]: #motivation

For better or worse, we allow implementing foreign traits for foreign types. For
example, `impl From<Foo> for Vec<i32>` is something any crate can write, even
though `From` is a foreign trait, and `Vec` is a foreign type. However, under
the current coherence rules, we do not allow `impl<T> From<Foo> for Vec<T>`.

There's no good reason for this restriction. Fundamentally, allowing `for
Vec<ForeignType>` requires all the same restrictions as allowing `Vec<T>`.
Disallowing type parameters to appear in the target type restricts how crates
can be extended.

Consider an example from Diesel. Diesel constructs an AST which represents a SQL
query, and then provides a trait to construct the final SQL. Because different
databases have different syntax, this trait is generic over the backend being
used. Diesel wants to support third party crates which add new AST nodes, as
well as crates which add support for new backends. The current rules make it
impossible to support both.

The Oracle database requires special syntax for inserting multiple records in a
single query. However, the impl required for this is invalid today. `impl<'a, T,
U> QueryFragment<Oracle> for BatchInsert<'a, T, U>`. There is no reason for this
impl to be rejected. The only impl that Diesel could add which would conflict
with it would look like `impl<'a, T> QueryFragment<T> for BatchInsert<'a, Type1,
Type2>`. Adding such an impl is already considered a major breaking change by
[RFC #1023], which we'll expand on below.

For some traits, this can be worked around by flipping the self type with the
type parameter to the trait. Diesel has done that in the past (e.g.
`T: NativeSqlType<DB>` became `DB: HasSqlType<T>`). However, that wouldn't work
for this case. A crate which adds a new AST node would no longer be able to
implement the required trait for all backends. For example, a crate which added
the `LOWER` function from SQL (which is supported by all databases) would not be
able to write `impl<T, DB> QueryFragment<Lower<T>> for DB`.

Unless we expand the orphan rules, use cases like this one will never be
possible, and a crate like Diesel will never be able to be designed in a
completely extensible fashion.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## Definitions

Local Trait: A trait which was defined in the current crate. Whether a trait is
local or not has nothing to do with type parameters. Given `trait Foo<T, U>`,
`Foo` is always local, regardless of the types used for `T` or `U`.

Local Type: A struct, enum, or union which was defined in the current crate.
This is not affected by type parameters. `struct Foo` is considered local, but
`Vec<Foo>` is not. `LocalType<ForeignType>` is local. Type aliases and trait
aliases do not affect locality.

Covered Type: A type which appears as a parameter to another type. For example,
`T` is uncovered, but the `T` in `Vec<T>` is covered. This is only relevant for
type parameters.

Blanket Impl: Any implementation where a type appears uncovered. `impl<T> Foo
for T`, `impl<T> Bar<T> for T`, `impl<T> Bar<Vec<T>> for T`, and `impl<T> Bar<T>
for Vec<T>` are considered blanket impls. However, `impl<T> Bar<Vec<T>> for
Vec<T>` is not a blanket impl, as all instances of `T` which appear in this impl
are covered by `Vec`.

Fundamental Type: A type for which you cannot add a blanket impl backwards
compatibly. This includes `&`, `&mut`, and `Box`. Any time a type `T` is
considered local, `&T`, `&mut T`, and `Box<T>` are also considered local.
Fundamental types cannot cover other types. Any time the term "covered type" is
used, `&T`, `&mut T`, and `Box<T>` are not considered covered.

## What is coherence and why do we care?

Let's start with a quick refresher on coherence and the orphan rules. Coherence
means that for any given trait and type, there is one specific implementation
that applies. This is important for Rust to be easy to reason about. When you
write `<Foo as Bar>::trait_method`, the compiler needs to know what actual
implementation to use.

In languages without coherence, the compiler has to have some way to choose
which implementation to use when multiple implementations could apply. Scala
does this by having complex scope resolution rules for "implicit" parameters.
Haskell (when a discouraged flag is enabled) does this by picking an impl
arbitrarily.

Rust's solution is to enforce that there is only one impl to choose from at all.
While the rules required to enforce this are quite complex, the result is easy
to reason about, and is generally considered to be quite important for Rust.
New features like specialization allow more than one impl to apply, but for any
given type and trait, there will always be exactly one which is most specific,
and deterministically be chosen.

An important piece of enforcing coherence is restricting "orphan impls". An impl
is orphaned if it is implementing a trait you don't own for a type you don't
own. Rust's rules around this balance two separate, but related goals:

- Ensuring that two crates can't write impls that would overlap (e.g. no crate
  other than `std` can write `impl From<usize> for Vec<i32>`. If they could,
  your program might stop compiling just by using two crates with an overlapping
  impl).
- Restricting the impls that can be written so crates can add implementations
  for traits/types they do own without worrying about breaking downstream
  crates.

## Teaching users

This change isn't something that would end up in a guide, and is mostly
communicated through error messages. The most common one seen is [E0210]. The
text of that error will be changed to approximate the following:

[E0210]: https://doc.rust-lang.org/error-index.html#E0210

> Generally speaking, Rust only permits implementing a trait for a type if either
> the trait or type were defined in your program. However, Rust allows a limited
> number of impls that break this rule, if they follow certain rules. This error
> indicates a violation of one of those rules.
>
> A trait is considered local when {definition given above}. A type is considered
> local when {definition given above}.
>
> When implementing a foreign trait for a foreign type, the trait must have one or
> more type parameters. A type local to your crate must appear before any use of
> any type parameters. This means that `impl<T> ForeignTrait<LocalType<T>, T> for
> ForeignType` is valid, but `impl<T> ForeignTrait<T, LocalType<T>> for
> ForeignType` is not.
>
> The reason that Rust considers order at all is to ensure that your
> implementation does not conflict with one from another crate. Without this rule,
> you could write `impl<T> ForeignTrait<T, LocalType> for ForeignType`, and
> another crate could write `impl<T> ForeignTrait<TheirType, T> for ForeignType`,
> which would overlap. For that reason, we require that your local type come
> before the type parameter, since the only alternative would be disallowing these
> implementations at all.

Additionally, the case of `impl<T> ForeignTrait<LocalType> for T` should be
special cased, and given its own error message, which approximates the
following:

> This error indicates an attempt to implement a trait from another crate for a
> type parameter.
>
> Rust requires that for any given trait and any given type, there is at most one
> implementation of that trait. An important piece of this is that we disallow
> implementing a trait from another crate for a type parameter.
>
> Rust's orphan rule always permits an impl if either the trait or the type being
> implemented are local to the current crate. Therefore, we can't allow `impl<T>
> ForeignTrait<LocalTypeCrateA> for T`, because it might conflict with another crate
> writing `impl<T> ForeignTrait<T> for LocalTypeCrateB`, which we will always
> permit.

Finally, [RFC #1105] states that implementing any non-fundamental trait for an
existing type is not a breaking change. This directly condradicts [RFC #1023],
which is entirely based around "blanket impls" being breaking changes.
Regardless of whether the changes proposed to the orphan rules in this proposal
are accepted, a blanket impl being a breaking change *must* be true today. Given
that the compiler currently accepts `impl From<Foo> for Vec<Foo>`, adding
`impl<T> From<T> for Vec<T>` must be considered a major breaking change.

As such, [RFC #1105] is amended to remove the statement that implementing a
non-fundamental trait is a minor breaking change, and states that adding any
blanket impl for an existing trait is a major breaking change, using the
definition of blanket impl given above.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Concrete orphan rules

Assumes the same definitions [as above](#definitions).

Given `impl<P1..=Pn> Trait<T1..=Tn> for T0`, an impl is valid only if at
least one of the following is true:

- `Trait` is a local trait
- All of
  - At least one of the types `T0..=Tn` must be a local type. Let `Ti` be the
    first such type.
  - No uncovered type parameters `P1..=Pn` may appear in `T0..Ti` (excluding
    `Ti`)

The primary change from the rules defined in in [RFC #1023] is that we only
restrict the appearance of *uncovered* type parameters. Once again, it is
important to note that for the purposes of coherence, `#[fundamental]` types are
special. `Box<T>` is not considered covered, and `Box<LocalType>` is considered
local.

Under this proposal, the orphan rules continue to work generally as they did
before, with one notable exception; We will permit `impl<T>
ForeignTrait<LocalType> for ForeignType<T>`. This is completely valid under the
forward compatibility rules set in [RFC #1023]. We can demonstrate that this is
the case with the following:

- Any valid impl of `ForeignTrait` in a child crate must reference at least one
  type that is local to the child crate.
- The only way a parent crate can reference the type of a child crate is with a
  type parameter.
- For the impl in child crate to overlap with an impl in parent crate, the type
  parameter must be uncovered.
- Adding any impl with an uncovered type parameter is considered a major
  breaking change.

We can also demonstrate that it is impossible for two sibling crates to write
conflicting impls, with or without this proposal.

- Any valid impl of `ForeignTrait` in a child crate must reference at least one
  type that is local to the child crate.
- The only way a local type of sibling crate A could overlap with a type used in
  an impl from sibling crate B is if sibling crate B used a type parameter
- Any type parameter used by sibling crate B must be preceded by a local type
- Sibling crate A could not possibly name a type from sibling crate B, thus that
  parameter can never overlap.

## Effects on parent crates

[RFC #1023] is amended to state that adding a new impl to an existing trait is
considered a breaking change unless, given `impl<P1..=Pn> Trait<T1..=Tn> for
T0`:

- At least one of the types `T0..=Tn` must be a local type, added in this
  revision. Let `Ti` be the first such type.
- No uncovered type parameters `P1..=Pn` appear in `T0..Ti` (excluding `Ti`)

The more general way to put this rule is: "Adding an impl to an existing trait
is a breaking change if it could possibly conflict with a legal impl in a
downstream crate".

This clarification is true regardless of whether the changes in this proposal
are accepted or not. Given that the compiler currently accepts `impl From<Foo> for
Vec<Foo>`, adding the impl `impl<T> From<T> for Vec<T>` *must* be considered a
major breaking change.

To be specific, the following adding any of the following impls would be
considered a breaking change:

- `impl<T> OldTrait<T> for OldType`
- `impl<T> OldTrait<AnyType> for T`
- `impl<T> OldTrait<T> for ForeignType`

However, the following impls would not be considered a breaking change:

- `impl NewTrait<AnyType> for AnyType`
- `impl<T> OldTrait<T> for NewType`
- `impl<T> OldTrait<NewType, T> for OldType`

# Drawbacks
[drawbacks]: #drawbacks

The current rules around coherence are complex and hard to explain. While this
proposal feels like a natural extension of the current rules, and something many
expect to work, it does make them slightly more complex.

The orphan rules are often taught as "for an impl `impl Trait for Type`, either
Trait or Type must be local to your crate". While this has never been actually
true, it's a reasonable hand-wavy explanation, and this gets us even further
from it. Even though `impl From<Foo> for Vec<()>` has always been accepted,
`impl<T> From<Foo> for Vec<T>` *feels* even less local. While `Vec<()>` only
applies to `std`, `Vec<T>` now applies to types from `std` and any other crate.

# Rationale and alternatives
[alternatives]: #alternatives

- Rework coherence even more deeply. The rules around the orphan rule are
  complex and hard to explain. Even `--explain E0210` doesn't actually try to
  give the rationale behind them, and just states the fairly arcane formula from
  the original RFC. While this proposal is a natural extension of the current
  rules, and something that many expect to "just work", it ultimately makes them
  even more complex.

  In particular, this keeps the "ordering" rule. It still serves *a* purpose
  with this proposal, but much less of one. By keeping it, we are able to allow
  `impl<T> SomeTrait<LocalType, T> for ForeignType`, because no sibling crate
  can write an overlapping impl. However, this is not something that the
  majority of library authors are aware of, and requires API designers to order
  their type parameters based on how likely they are to be overridden by other
  crates.

  We could instead provide a mechanism for traits to opt into a redesigned
  coherence system, and potentially default to that in a future edition.
  However, that would likely cause a lot of confusion in the community. This
  proposal is a strict addition to the set of impls which are allowed with the
  current rules, without an increase in risk or impls which are breaking
  changes. It seems like a reasonably conservative move, even if we eventually
  want to overhaul coherence.

- Get rid of the orphan rule entirely. A long standing pain point for crates
  like Diesel has been integration with other crates. Diesel doesn't want to
  care about chrono, and chrono doesn't want to care about Diesel. A database
  access library shouldn't dictate your choice of time libraries, vice versa.

  However, due to the way Rust works today, one of them has to. Nobody can
  create a `diesel-chrono` crate due to the orphan rule. Maybe if we just
  allowed crates to have incompatible impls, and set a standard of "don't write
  orphan impls unless that's the entire point of your crate", it wouldn't
  actually be that bad.

# Unresolved questions
[unresolved]: #unresolved-questions

- Are there additional implementations which are clearly acceptable under the
  current restrictions, which are disallowed with this extension? Should we
  allow them if so?

[RFC #1023]: https://github.com/rust-lang/rfcs/blob/master/text/1023-rebalancing-coherence.md
[RFC #1105]: https://github.com/rust-lang/rfcs/blob/master/text/1105-api-evolution.md
