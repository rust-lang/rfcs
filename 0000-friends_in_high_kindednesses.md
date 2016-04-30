- Feature Name: associated_type_operators
- Start Date: 2016-04-29
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Allow type operators to be associated with traits. This is an incremental step
toward a more general feature commonly called "higher-kinded types," which is
often ranked highly as a requested feature by Rust users. This specific feature
(associated type operators) resolves one of the most common use cases for
higher-kindedness, is a relatively simple extension to the type system compared
to other forms of higher-kinded polymorphism, and is forward compatible with
more complex forms of higher-kinded polymorphism that may be introduced in the
future.


# Motivation
[motivation]: #motivation

Consider the following trait as a representative motivating example:

```rust
trait StreamingIterator {
    type Item<'a>;
    fn next<'a>(&'a mut self) -> Option<Self::Item<'a>>;
}
```

This trait is very useful - it allows for a kind of Iterator which yields
values which have a lifetime tied to the lifetime of the reference passed to
`next`. A particular obvious use case for this trait would be an iterator over
a vector which yields overlapping, mutable subslices with each iteration. Using
the standard `Iterator` interface, such an implementation would be invalid,
because each slice would be required to exist for as long as the iterator,
rather than for as long as the borrow initiated by `next`.

This trait cannot be expressed in Rust as it exists today, because it depends
on a sort of higher-kinded polymorphism. This RFC would extend Rust to include
that specific form of higher-kinded polymorphism, which is refered to here as
associated type operators. This feature has a number of applications, but the
primary application is along the same lines as the `StreamingIterator` trait:
defining traits which yield types which have a lifetime tied to the local
borrowing of the receiver type.

# Detailed design
[design]: #detailed-design

## Background: What is kindedness?

"Higher-kinded types" is a vague term, conflating multiple language features
under a single inaccurate banner. Let us discuss specifically the notion of a
'kind' as background for this RFC. Kinds are often called 'the type of a type',
the exact sort of unhelpful description that only makes sense to someone who
already understands what is being explained. We'll take a different approach.

In a well-typed language, every expression has a type. Many expressions have
what are sometimes called 'base types,' types which are primitive to the
language and which cannot be described in terms of other types. In Rust, the
types `bool`, `i64`, `usize`, and `char` are all prominent examples of base
types. In contrast, there are other types which are formed by arranging other
types - functions are a good example of this. Consider this simple function:

```rust
fn not(x: bool) -> bool {
   !x
}
```

`not has the type `bool -> bool` (my apologies for using a syntax different
from Rust's). Note that this is different from the type of `not(true)`, which
is `bool`. This difference is important, by way of analogy, to understanding
higher-kindedness.

In the analysis of kinds, all of these types - `bool`, `char`, `bool -> bool`
and so on - have the kind `type`, which is often written `*`. This is a base
kind, just as `bool` is a base type. In contrast, there are more complex kinds,
such as `* -> *`. An example of an term of this kind is `Vec`, which takes a
type as a parameter and evalues to a type. The difference between the kind of
`Vec` and the kind of `Vec<i32>` (which is `*`) is analogous to the difference
between the type of `not` and `not(true)`. Note that `Vec<T>` has the kind `*`,
just like `Vec<i32>`: even though `T` is a type parameter, `Vec` is still being
applied to a type, just like `not(x)` still has the type `bool` even if `x` is
dynamically determined.

A relatively uncommon feature of Rust is that it has _two_ base kinds, whereas
many languages which deal with higher-kindedness only have the base kind `*`.
The other base kind of Rust is the lifetime parameter, which for conveniences
sake we will represent as `&`. For a type `Foo<'a>`, the kind of `Foo` is
`& -> *`.

Terms of a higher kind are often called 'type operators'; type operators which
evaluate to a type are called 'type constructors.' The concept of
'higher-kinded types' usually refers to the ability to write code which is
polymorphic over type operators in some way, such as implementing a trait for a
type operator. This proposal is to allow a type operator to be associated with
a trait, in the same way that a type or a const can be associated with a trait
today.

## The basic requirements of associated type operators

Adding associated type operators to the language requires the introduction of
four discrete constructs:

1. In a definition of a trait, an associated type operator can be declared.
2. In the position of any type within the definition of that trait, the
   associated type operator can be applied to a type parameter or a concrete
   type which is in scope.
3. In the implementation of that trait, a type operator of the correct kind can
   be assigned to the declared associated type operator.
4. When bounding a type parameter by that trait, that trait can be bound to
   have a concrete type operator as this associated type operator.

## Partial application

In order for this feature to be useful, we will have to allow for type
operators to partially applied. Many languages with higher-kinded polymorphism
use currying as an alternative to partial application. Rust does not have
currying at the level of expressions, and currying would not be sufficient
to enable the use cases that exist for type operators, so this RFC does not
propose using currying for higher-kinded polymorphism.

As an example, the reference operator has the kind `&, * -> *`, taking both
a lifetime and a type to produce a new type. With currying, the two parameters
to the reference operator would have to have a defined order, and it would
be possible to partially apply only one of the parameters to the reference
operator. That is, if it were `& -> * -> *`, one could apply it to a lifetime
to produce a `* -> *` operator, but one could not apply it to a type to produce
a `& -> *` operator. If it were defined as `* -> & -> *`, it would be 
restricted in the opposite way. Because Rust makes use of two base kinds,
currying would severely restrict the forms of abstraction enabled by Rust.

Instead, when defining an associated type operator, an anonymous type operator
can be constructed from a type operator with more parameters by applying any
of the parameters to that operator. The syntax discussed below makes it
unambiguous and easy to see which parameters remain undetermined at the point
of assigning the associated type operator to a concrete type operator.

When used in a type position, of course, all of the parameters to an associated
type operator must have been applied to concrete types or type parameters that
are in scope.

## Associated type operators in bounds

This RFC proposes making associated type operators available in bounds only
as concrete type operators. Because higher-kinded traits cannot be defined, and
traits cannot be implemented for type operators, it is not possible to bound
associated type operators by traits.

Even without higher-kinded traits, it could be useful to bound associated type
operators with some sort of higher-rank syntax, as in:

```rust
T where T: StreamingIterator, for<'a> T::Item<'a>: Display
```

However, this RFC does not propose adding this feature.

## Benefits of implementing only this feature before other higher-kinded polymorphisms

This feature is the first 20% of higher-kinded polymorphism which is worth 50%
of the full implementation. It is the ideal starting point, as it will enable
many constructs while adding relatively few complicates to the type system. By
implementing only associated type operators, we sidestep several issues:

* Defining higher-kinded traits
* Implementing traits for type operators
* Higher order type operators
* Type operator parameters bound by higher-kinded traits
* Type operator parameters applied to a given type or type parameter

## Proposed syntax

The syntax proposed in this RFC is very similar to the syntax of associated
types and type aliases. An advantage of this is that users less familiar with
the intimate details of kindedness will hopefully find this feature intuitive.

To declare an associated type operator, simply declare an associated type
with parameters on the type name, as in:

```rust
trait StreamingIterator {
    type Item<'a>;
    ...
}
```

Here `Item` is an associated type operator of the kind `& -> *`.

To apply the associated type operator, simply use it in the position where
a normal type operator would be used instead, as in:

```rust
trait StreamingIterator {
    ...
    fn next<'a>(&'a mut self) -> Option<Self::Item<'a>>;
}
```

To assign the associated type operator, use the parameters in the type
declaration on the right-hand side of the type expression, as in:

```rust
impl<T> StreamingIterator for StreamIter<T> {
   type Item<'a> = &'a [T];
   ...
}
```

Note here that a slice reference has the kind `&, * -> *`, but the local type
parameter `T` is applied to it through partial application to form a type
operator `& -> *`. The syntax makes it clear that the unapplied parameter is
the lifetime `'a`, because `'a` is introduced on the type Item.

This has the same appearance as the declaration of type operator aliases which
are not associated with the trait.

To add a concrete bound as an associated type operator, the syntax is the same
as adding a concrete bound of an associated type. Here, any types or lifetimes
which are parameters to the associated type operator are omitted (not elided):

```rust
where T: StreamingIterator<Item=&[u8]>
```

`&[u8]` is not an elided form of some `&'a [u8]`, but a type operator of the
kind `& -> *`.


However, life time parameters can be elided when applied to associated type
operators in the type position just as they can be elided for concrete type
operators, as in this case, providing a full definition of `StreamingIterator`:

```rust
trait StreamingIterator {
   type Item<'a>;
   fn next(&mut self) -> Option<Self::Item>;
}
```


# Drawbacks
[drawbacks]: #drawbacks

## Drawbacks to the concept

This adds complexity to the language, and implements a part of higher-kinded
polymorphism without all of the benefits that come along with it. There are
valid arguments in favor of waiting until additional forms of higher-kinded
polymorphism have been worked out, as well as in favor of never implementing
higher-kinded polymorphism at all.

## Drawbacks to the syntax

Though this syntax is a natural fit for associated type operators, it is not
a natural syntax for other forms of higher-kinded polymorphism. As a result,
the syntaxes of two related forms of polymorphism will be significantly
different. We believe this cost is justified by the advantages of making the
syntax similar to associated types.

# Alternatives
[alternatives]: #alternatives

An alternative is to push harder on higher-ranked lifetimes, possibly
introducing some elision that would make them easier to use.

Currently, an approximation of `StreamingIterator` can be defined like this:

```rust
trait StreamingIterator<'a> {
   type Item: 'a;
   fn next(&'a self) -> Option<Self::Item>;
}
```

You can then bound types as `T: for<'a> StreamingIterator<'a>` to avoid the
lifetime parameter infecting everything `StreamingIterator` appears.

However, this only partially prevents the infectiveness of `StreamingIterator`,
only allows for some of the types that associated type operators can express,
and is in generally a hacky attempt to work around the limitation rather than
an equivalent alternative.

# Unresolved questions
[unresolved]: #unresolved-questions

This design does not resolve the question of introducing more advanced forms of
higher-kinded polymorphism. This document does not describe the details of
implementing this RFC in terms of rustc's current typeck, because the author
is not familiar with that code. This document is certainly inadequate in its
description of this feature, most likely in relation to partial application,
because of the author's ignorance and personal defects.
