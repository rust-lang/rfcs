- Feature Name: visible_trait_where_clause
- Start Date: (2017-02-24)
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Today when you write `T: SomeTrait`, you are also implicitly allowed to rely on
`T: AnySuperTraits`. However, if `SomeTrait` has additional constraints besides
supertraits, they are must be repeated every time `T: SomeTrait` is written.
This RFC allows `T: SomeTrait` to implicitly inherit any constraints of
`SomeTrait` that are on a concrete type.

# Motivation
[motivation]: #motivation

For traits which have type parameters, the parameter is often as important as
the type itself. For example, consider that `T: Into<U>` and `U: From<T>` are
functionally equivalent. `T` and `U` can be considered to be of equal
importance.

However, they are not equivalent when it comes to trait definitions. If you were
to write `trait Foo where Self: From<Bar>`, then writing `T: Foo` implicitly
also means `T: From<Bar>`. If you change that to the functionally equivalent
form of `trait Foo where Bar: Into<Self>`, then every place that `T: Foo` is
written is required to specify that `Bar: Into<T>` as well. There is no reason
that these should be treated differently.

As a concrete example of where this has caused pain in the wild, Diesel went so
far as to change a trait from `T: NativeSqlType<DB>` to `DB: HasSqlType<T>`,
simply so that [they could constraint that a "backend" supports the ANSI
standard types](https://github.com/diesel-rs/diesel/commit/b2476d1d). That crate
also has had to avoid adding constraints like `String: FromSql<Text, Self>` as
doing a similar re-organization would cause coherence issues.

# Detailed design
[design]: #detailed-design

For the purposes of this RFC, a "constraint" is defined as an item which appears
in the where clause in the form `target: obligation`. A constraint is considered
"inherited" if `T: SomeTrait` implicitly adds constraints from the definition of
`SomeTrait`.

For all examples given, assume that an item has been fully desugared (`trait
Foo<Bar: Baz>` is expanded to `trait Foo<Bar> where Bar: Baz`, `trait Foo: Bar`
is expanded to `trait Foo where Self: Bar`, and `where T: Foo + Bar` is expanded
to `where T: Foo, T: Bar`).

There is no RFC that specifically lays out when constraints are inherited, or
what the specific reasoning was.  However, the design appears to be intended to
treat traits and structs similarly. Constraints on structs are assumed to be
based around the type parameters. This is a reasonable assumption for structs,
since any reference to the `Self` type by definition includes the type
parameters.

The reasoning behind requiring that `T: SomeTrait<U>` also repeat the
constraints that `SomeTrait` laid out for `U` is likely to ensure that
[loosening bounds is not a breaking change](https://github.com/rust-lang/rfcs/blob/master/text/1105-api-evolution.md#minor-change-loosening-bounds),
as any caller relying on those bounds would have had to state them explicitly.

However, traits differ due to the fact that the type of `Self` is not known.
Referencing `Self` does not necessarily mean that we are referencing the type
parameters of the trait. For this reason, `T: SomeTrait` inherits any any
constraints that `SomeTrait` places on `Self`.

Supertraits also differ from normal constraints in that they affect the vtable
for trait objects. This RFC does not intend to change what is defined as a
supertrait, or in any way affect trait objects or object safety. This RFC only
seeks to expand when a constraint is inherited by `T: SomeTrait`.

The current rules for whether a constraint is inherited are simply "the target
of the constraint is `Self`". This RFC proposes that a constraint is inherited
when one of the following is true:

- The target of the constraint is `Self`
- The type parameters of the trait do not appear in the target and `Self`
  appears in either the target or the obligation

Here are some concrete examples (all assume the trait `Foo<T>`)

- `Self: Bar`
  - Before: inherited
  - After: inherited
- `Self: Bar<T>`
  - Before: inherited
  - After: inherited
- `T: Bar<Self>`
  - Before: not inherited
  - After: not inherited
- `Vec<Self>: Bar`
  - Before: not inherited
  - After: inherited
- `i32: Bar<Self>`
  - Before: not inherited
  - After: inherited
- `Result<Self, T>: Bar`
  - Before: not inherited
  - After: not inherited
- `i32: Bar<T, Self>`
  - Before: not inherited
  - After: inherited

This RFC does *not* propose any changes in what is required in the actual trait
declaration. For example, if writing `trait Foo where Bar: Baz<Self>` required
that `Self: Sized`, that constraint would still need to be explicitly added to
the definition of `Foo`. However, `Baz` may have other constraints that fall
under these new rules, which one could now assume about `Bar`.

# How We Teach This
[how-we-teach-this]: #how-we-teach-this

The term "constraint" is already pretty well established with the meaning used
in this RFC. I haven't found the terminology "inherited" used in reference to
constraints (but actual documentation around them is surprisingly sparse). Since
this is giving a name to something which currently only occurs for supertraits
(which emulate a subtype relationship) and lifetime requirements (which is an
actual subtype relationship), the term "inherited" seems fitting.

This idea is a continuation of an existing Rust pattern, but one that is mostly
undocumented so would need to be essentially introduced as a new one.

This feature is introduced to new and existing users simply by the compiler
telling them what to do. Other than concerns regarding backwards compatibility,
it doesn't need to be explicitly taught.

_The Rust Programming Language_ does not cover supertraits or where clauses, and
I do not think that we need to add it. _Rust by Example_ would benefit from
supertraits being mentioned in its section on "bounds", but I do not think it
needs to lay out the full set of rules.

The Rust Reference does not currently have a section where this addition would
fit. A new section should be added about constraints in general which covers:

- The implicit `T: Sized`
- Constraints of structs
- Supertraits
- These new rules

# Drawbacks
[drawbacks]: #drawbacks

This change potentially causes loosening constraints to be considered a major
change under [RFC #1105]. The RFC currently lists loosening constraints as a
minor change, but it does so under "Signatures in type definitions" which
doesn't actually cover traits. Supertraits are never explicitly mentioned in
that document. If 1105 is interpreted to have removing a supertrait be
considered a major change, this RFC expands the potential for a major change.

(1105 should be amended to clarify whether removing a supertrait is a major
change, regardless of whether this RFC is accepted).

[RFC #1105]: https://github.com/rust-lang/rfcs/blob/master/text/1105-api-evolution.md

However, there is likely very little code in the wild which has constraints that
would be inherited by these new rules today. The type of constraint covered by
this RFC is significantly less useful if it is not inherited. Users who do wish
to add this sort of constraint in a way that is not inherited can place the
constraint on specific methods instead of on the trait itself.

# Alternatives
[alternatives]: #alternatives

We could also have the simpler rule of "target is a type other than `Self` and
no type parameters from the trait appear in the constraint". However, that
leaves the case of `Self: Bar<T>` as a weird inconsistency. (It should be noted
that a constraint that doesn't include the type parameters by definition must
include `Self` to be valid, otherwise it is simply an assertion which is always
true or false)

Another potential design would be to allow the `pub` keyword to appear in where
clauses, allowing authors to be very specific about whether a constraint is
inherited or not.

# Unresolved questions
[unresolved]: #unresolved-questions

None
