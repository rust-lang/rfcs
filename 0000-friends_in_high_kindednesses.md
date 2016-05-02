- Feature Name: associated_type_constructors
- Start Date: 2016-04-29
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Allow type constructors to be associated with traits. This is an incremental
step toward a more general feature commonly called "higher-kinded types," which
is often ranked highly as a requested feature by Rust users. This specific
feature (associated type constructors) resolves one of the most common use
cases for higher-kindedness, is a relatively simple extension to the type
system compared to other forms of higher-kinded polymorphism, and is forward
compatible with more complex forms of higher-kinded polymorphism that may be
introduced in the future.


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
associated type constructors. This feature has a number of applications, but
the primary application is along the same lines as the `StreamingIterator`
trait: defining traits which yield types which have a lifetime tied to the
local borrowing of the receiver type.

# Detailed design
[design]: #detailed-design

## Background: What is kindedness?

"Higher-kinded types" is a vague term, conflating multiple language features
under a single banner, which can be inaccurate. As background, this RFC
includes a brief overview of the notion of kinds and kindedness. Kinds are
often called 'the type of a type,' the exact sort of unhelpful description that
only makes sense to someone who already understands what is being explained.
Instead, let's try to understand kinds by analogy to types.

In a well-typed language, every expression has a type. Many expressions have
what are sometimes called 'base types,' types which are primitive to the
language and which cannot be described in terms of other types. In Rust, the
types `bool`, `i64`, `usize`, and `char` are all prominent examples of base
types. In contrast, there are types which are formed by arranging other types -
functions are a good example of this. Consider this simple function:

```rust
fn not(x: bool) -> bool {
   !x
}
```

`not` has the type `bool -> bool` (my apologies for using a syntax different
from Rust's). Note that this is different from the type of `not(true)`, which
is `bool`. This difference is important to understanding higher-kindedness.

In the analysis of kinds, all of these types - `bool`, `char`, `bool -> bool`
and so on - have the kind `type`. Every type has the kind `type`. However,
`type` is a base kind, just as `bool` is a base type, and there are terms with
more complex kinds, such as `type -> type`. An example of a term of this kind
is `Vec`, which takes a type as a parameter and evaluates to a type. The
difference between the kind of `Vec` and the kind of `Vec<i32>` (which is
`type`) is analogous to the difference between the type of `not` and
`not(true)`. Note that `Vec<T>` has the kind `type`, just like `Vec<i32>`: even
though `T` is a type parameter, `Vec` is still being applied to a type, just
like `not(x)` still has the type `bool` even though `x` is a variable.

A relatively uncommon feature of Rust is that it has _two_ base kinds, whereas
many languages which deal with higher-kindedness only have the base kind
`type`. The other base kind of Rust is the lifetime parameter. If you have a
type like `Foo<'a>`, the kind of `Foo` is `lifetime -> type`.

Higher-kinded terms can take multiple arguments as well, of course. `Result`
has the kind `type, type -> type`. Given `vec::Iter<'a, T>` `vec::Iter` has the
kind `lifetime, type -> type`.

Terms of a higher kind are often called 'type operators'; the type operators
which evaluate to a type are called 'type constructors'. There are other type
operators which evaluate to other type operators, and there are even higher
order type operators, which take type operators as their argument (so they have
a kind like `(type -> type) -> type`). This RFC doesn't deal with anything as
exotic as that.

Specifically, the goal of this RFC is to allow type constructors to be
associated with traits, just as you can currently associate functions, types,
and consts with traits. There are other forms of polymorphism involving type
constructors, such as implementing traits for a type constructor instead of a
type, which are not a part of this RFC.

## Features of associated type constructors

### Declaring an associated type constructor

This RFC proposes a very simple syntax for defining an associated type
constructor, which looks a lot like the syntax for creating aliases for type
constructors. The goal of using this syntax is to avoid to creating roadblocks
for users who do not already understand higher kindedness.

```rust
trait StreamingIterator {
   type Item<'a>;
}
```

Here, it is clear that `Item` is a type constructor, because it carries a
parameter. Associated type constructors can carry any number of type and
lifetime parameters, as in:

```rust
trait FooBar {
    type Baz<'a, T, U>;
}
```

Associated type constructors can be followed by `where` clauses, which place
trait bounds on the types constructed by this constructor. For example:

```rust
trait Collection<T> {
    type Iter<'a> where for<'a> Self::Iter<'a>: Iterator<Item=&'a T>;
    type IterMut<'a> where for<'a> Self::IterMut<'a>: Iterator<Item=&'a mut T>;
    type IntoIter: Iterator<Item=T>;
}
```

A `where` clause is used to avoid the impression that this is providing a
bound on the constructor itself. Note the contrast to `IntoIter`, which is
not a type constructor. Also note that this involves an extension to HRTB,
which is discussed later in this RFC.

As a last note, these `where` clauses do not need to involve HRTB, but can
instead apply type/lifetime parameters or concrete types/lifetimes that are
in scope to the type constructor, as in:

```rust
trait Foo<T> {
    type Bar<X> where Self::Bar<T>: Display;
    type Baz<'a> where Self::Baz<'static>: Send;
}
```

### Assigning an associated type constructor

Assigning associated type constructors in impls is very similar to the syntax
for assigning associated types:

```rust
impl<T> StreamingIterator for StreamIterMut<T> {
    type Item<'a> = &'a mut [T];
    ...
}
```

Note that this example makes use of partial application (see the later section
on partial application for more information about this feature). The parameter
to this argument is quite clear, because it is the argument associated with
the type constructor. If there were multiple lifetimes involved, it would still
be unambiguous which was being applied and which isn't, for example:

```rust
impl<'a> StreamingIterator for FooStreamIter<'a> {
    type Item<'b> = &'b mut [Foo<'a>];
}
```

### Using an associated type constructor to construct a type

Once a trait has an associated type constructor, it can be applied to any
type/lifetime parameters or concrete types/lifetimes that are in scope. This
can be done both inside the body of the trait and outside of it, using syntax
which is analogous to the syntax for using associated types. Here are some
examples:

```rust
trait StreamingIterator {
    type Item<'a>;
    // Applying the lifetime parameter `'a` to `Self::Item` inside the trait.
    fn next<'a>(&'a self) -> Option<Self::Item<'a>>;
}

struct Foo<T: StreamingIterator> {
    // Applying a concrete lifetime to the constructor outside the trait.
    bar: <T as StreamingIterator>::Item<'static>;
}
```

Associated type constructors can also be used to construct other type
constructors through partial application (see the later section on partial
application for more information about this feature).

```rust
trait Foo {
    type Bar<'a, T>;
}

trait Baz {
    type Quux<'a>;
}

impl<T> Baz for T where T: Foo {
    type Quux<'a> = <T as Foo>::Bar<'a, usize>;
}
```

Lastly, lifetimes can be elided in associated type constructors in the same
manner that they can be elided in other type constructors. Considering lifetime
ellision, the full definition of `StreamingIterator` is:

```rust
trait StreamingIterator {
    type Item<'a>;
    fn next(&mut self) -> Option<Self::Item>;
}
```

### Using associated type constructors in bounds

Users can bound parameters by the type constructed by that trait's associated
type constructor of a trait using HRTB. Both type equality bounds and trait
bounds of this kind are valid:

```rust
fn foo<T: for<'a> StreamingIterator<Item<'a>=&'a [i32]>>(iter: T) { ... }

fn foo<T>(iter: T) where T: StreamingIterator, for<'a> T::Item<'a>: Display { ... }
```

See the section on extending HRTBs for more information about that aspect of
this feature.

This RFC does not propose allowing any sort of bound by the type constructor
itself, whether an equality bound or a trait bound (trait bounds of course are
also impossible). That is, one can do the former but not the latter here:

```rust
// Valid
fn foo<T: for<X> Foo<Bar<X>=Vec<X>>>(x: T) { ... }

// Invalid
fn foo<T: Foo<Bar=Vec>>(x: T) { ... }
```

HRTBs allow us to express the same bounds without adding quite as radical a
new feature as adding bounds by equality of type constructors.

## Partial Application

In order for this feature to be useful, we will have to allow for type
constructors to partially applied. Many languages with higher-kinded
polymorphism use currying as an alternative to partial application. Rust does
not have currying at the level of expressions, and currying would not be
sufficient to enable the use cases that exist for type constructors, so this
RFC does not propose using currying for higher-kinded polymorphism.

As an example, the reference operator has the kind `lifetime, type -> type`,
taking both a lifetime and a type to produce a new type. With currying, the two
parameters to the reference operator would have to have a defined order, and it
would be possible to partially apply only one of the parameters to the
reference operator. That is, if it were `lifetime -> type -> type`, one could
apply it to a lifetime to produce a `type -> type` operator, but one could not
apply it to a type to produce a `lifetime -> type` operator. If it were defined
as `type -> lifetime -> type`, it would be  restricted in the opposite way.
Because Rust makes use of two base kinds, currying would severely restrict the
forms of abstraction enabled by Rust.

Instead, when defining an associated type constructor, an anonymous type
constructor can be constructed from a type constructor with more parameters by
applying any of the parameters to that type constructor. The syntax discussed
below makes it unambiguous and easy to see which parameters remain undetermined
at the point of assigning the associated type constructor to a concrete type
constructor.

When used in a type position, of course, all of the parameters to an associated
type constructor must have been applied to concrete types or type parameters that
are in scope, so that it can be evaluated to a proper type.

## Extending HRTBs

Providing bounds on the types constructed by associated type constructors
requires heavy use of HRTBs, or higher-ranked trait bounds. This exists in Rust
today, but in a limited form, and it is an obscure feature primarily used in
the background to make function traits behave as expected.

In brief, a higher-ranked trait bound is one in which a type or lifetime
parameter is introduced only for the scope of that trait bound. A classic
example of how this can be useful is in the contrast between these two
functions:

```rust
fn foo1<T, F>(x: T, id: F) -> T where F: Fn(T) -> T {
    id(x)
}

fn foo2<T, F>(x: T, id: F) -> T where F: for<X> Fn(X) -> X {
    id(x)
}

// Valid (evaluates to 4)
foo1::<i32, _>(2, |x| x + x)

// Invalid (type error)
foo2::<i32, _>(2, |x| x + x)
```

In the second function, we _guarantee_ that the `id` argument is the identity
function (ignoring side effects), because it must be a valid function of `X`
to `X` for _all_ `X`, whereas the first can be specialized to only be a valid
function for the type `T`, in this case `i32`.

Higher-ranked trait bounds have several other use cases. Currently, Rust uses
them to declare that arguments with different lifetimes can be passed to
function types, by requiring that that function be valid for all lifetimes,
rather than just for some single lifetime parameter.

In order to bound associated type constructors, we use higher-ranked types to
require that the type constructor constructs type which meet some bound. This
can be done both in the declaration and when bounding a type parameter by a
trait, and can be both a trait bound and a type equality bound. Here are
examples in code, with their meanings written out:

```rust
trait Sequence<T> {
    // For every lifetime, this constructor applied to that lifetime must
    // produce a type which is an iterator of references of that lifetime
    type Iter<'a> where for<'a> Iter<'a>: Iterator<Item=&'a T>;
}

// For every lifetime, the associated type constructor Item applied to
// that lifetime produces a reference of that lifetime to a slice of bytes.
struct Foo<T> where T: for<'a> StreamingIterator<Item=&'a mut [u8]> {
   ...
}
```

Enabling this requires extending HRTBs to support type parameters as well as
lifetime parameters. This would also imply that HRTBs could introduce type
parameters that themselves have bounds. The syntax for this is left to another
RFC.

## Benefits of implementing only this feature before other higher-kinded polymorphisms

This feature is not full-blown higher-kinded polymorphism, and does not allow
for the forms of abstraction that are so popular in Haskell, but it does
provide most of the unique-to-Rust use cases for higher-kinded polymorphism,
such as streaming iterators and collection traits. It is probably also the
most accessible feature for most users, being somewhat easy to understand
intuitively without understanding higher-kindedness.

This feature has several tricky implementation challenges, but avoids all of
these features that other kinds of higher-kinded polymorphism require:

* Defining higher-kinded traits
* Implementing higher-kinded traits for type operators
* Higher order type operators
* Type operator parameters bound by higher-kinded traits
* Type operator parameters applied to a given type or type parameter

## Advantages of proposed syntax

The advantage of the proposed syntax is that it leverages syntax that already
exists. Type constructors can already be aliased in Rust using the same syntax
that this used, and while type aliases play no polymorphic role in type
resolution, to users they seem very similar to associated types. A goal of this
syntax is that many users will be able to use types which have assocaited type
constructors without even being aware that this has something to do with a type
system feature called higher-kindedness.

# Drawbacks
[drawbacks]: #drawbacks

## Adding language complexity

This would add a somewhat complex feature to the language, being able to
polymorphically resolve type constructors, and requires several extensions to
the type system which make the implementation more complicated.

Additionally, though the syntax is designed to make this feature easy to learn,
it also makes it more plausible that a user may accidentally use it when they
mean something else, similar to the confusion between `impl .. for Trait` and
`impl<T> .. for T where T: Trait`. For example:

```rust
// The user means this
trait Foo<'a> {
    type Bar: 'a;
}

// But they write this
trait Foo<'a> {
    type Bar<'a>;
}
```

## Not full "higher-kinded types"

This does not add all of the features people want when they talk about higher-
kinded types. For example, it does not enable traits like `Monad`. Some people
may prefer to implement all of these features together at once. However, this
feature is forward compatible with other kinds of higher-kinded polymorphism,
and doesn't preclude implementing them in any way. In fact, it paves the way
by solving some implementation details that will impact other kinds of higher-
kindedness as well, such as partial application.

## Syntax isn't like other forms of higher-kinded polymorphism

Though the proposed syntax is very similar to the syntax for associated types
and type aliases, it is probably not possible for other forms of higher-kinded
polymorphism to use a syntax along the same lines. For this reason, the syntax
used to define an associated type constructor will probably be very different
from the syntax used to e.g. implement a trait for a type constructor.

However, the syntax used for these other forms of higher-kinded polymorphism
will depend on exactly what features they enable. It would be hard to design
a syntax which is consistent with unknown features.

# Alternatives
[alternatives]: #alternatives

## Push HRTBs harder without associated type constructors

An alternative is to push harder on HRTBs, possibly introducing some elision
that would make them easier to use.

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
only allows for some of the types that associated type constructors can
express, and is in generally a hacky attempt to work around the limitation
rather than an equivalent alternative.

## Only add associated type constructors whose arguments are lifetimes

If associated type constructors could only take lifetime arguments, much of the
work extending HRTBs would not be necessary. Associated type constructors with
lifetime parameters only covers the primary known use cases for this feature.
Though it is inelegant to treat lifetime parameters differently from type
parameters here, at least as an implementation strategy it may make sense to
first implement this feature with lifetime parameters, and later extend it to
type parameters as well.

# Unresolved questions
[unresolved]: #unresolved-questions

This design does not resolve the question of introducing more advanced forms of
higher-kinded polymorphism. This document does not describe the details of
implementing this RFC in terms of rustc's current typeck, because the author
is not familiar with that code. This document is certainly inadequate in its
description of this feature, most likely in relation to partial application,
because of the author's ignorance and personal defects.
