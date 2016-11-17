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

### Declaring & assigning an associated type constructor

This RFC proposes a very simple syntax for defining an associated type
constructor, which looks a lot like the syntax for creating aliases for type
constructors. The goal of using this syntax is to avoid to creating roadblocks
for users who do not already understand higher kindedness.

```rust
trait StreamingIterator {
   type Item<'a>;
}
```

It is clear that the `Item` associated item is a type constructor, rather than
a type, because it has a type parameter attached to it.

Associated type constructors can be bounded, just like associated types can be:

```rust
trait Iterable {
    type Item<'a>;
    type Iter<'a>: Iterator<Item = Item<'a>>;
    
    fn iter<'a>(&'a self) -> Self::Iter<'a>;
}
```

This bound is applied to the "output" of the type constructor, and the parameter
is treated as a higher rank parameter. That is, the above bound is roughly
equivalent to adding this bound to the trait:

```rust
for<'a> Self::Iter<'a>: Iterator<Item = Self::Item<'a>>
```

Currently, this RFC only proposes adding associated type constructor of **lifetime**
arguments, but it is intended to be extended to type arguments once higher rank
type parameters are included.

Assigning associated type constructors in impls is very similar to the syntax
for assigning associated types:

```rust
impl<T> StreamingIterator for StreamIterMut<T> {
    type Item<'a> = &'a mut [T];
    ...
}
```

### Using an associated type constructor to construct a type

Once a trait has an associated type constructor, it can be applied to any
parameters or concrete term that are in scope. This can be done both inside the
body of the trait and outside of it, using syntax which is analogous to the
syntax for using associated types. Here are some examples:

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
constructors:

```rust
trait Foo {
    type Bar<'a, 'b>;
}

trait Baz {
    type Quux<'a>;
}

impl<T> Baz for T where T: Foo {
    type Quux<'a> = <T as Foo>::Bar<'a, 'static>;
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

This RFC does not propose allowing any sort of bound by the type constructor
itself, whether an equality bound or a trait bound (trait bounds of course are
also impossible). 

#### `let` introduction of parameters in bound

The `for` syntax for HRTB is widely considered inaccessible and difficult to learn.
The problem here is not that the underlying concept of a higher rank parameter is
particularly challenging, but that the syntax introduces too much jargony syntax
which distracts from the underlying idea.

This RFC proposes adding a new syntax for higher rank parameters which appear in the
type position of the bound. A user can simply introduce a new parameter with the `let`
keyword:

```rust
where T: Iterable, T::Item<let 'a>: &'a str
//equivalent to
where T: Iterable, for<'a> T::Item<'a>: &'a str
```

The variable introduced by the `let` is scoped only to this bound. Shadowing existing
type variables in scope is not permitted by a `let`, just as it is not permitted with
the existing `for` syntax. The `let` keyword is necessary to make it unambiguous that
the user intends to introduce a new variable here.

The `let` syntax is valid for any type constructor being bound, including those which
are not associated items. As an arbitrary example:

```
where vec::Iter<let 'a, T>: ExactSizeIterator
```

Hypothetically, the `let` syntax could be expanded to positions outside of bounds, but
this RFC proposes no such extension.

## Restrictions on ATCs

In order to be forward compatible with higher order type constructors - which is commonly
called "full higher kinded types" - this RFC imposes certain restrictions on the kinds of
constructors which can be used as associated items. Background on this reasoning can be
found here: http://smallcultfollowing.com/babysteps/blog/2016/11/09/associated-type-constructors-part-4-unifying-atc-and-hkt/

The restriction is simple: Each lifetime argument of the constructor must be applied, in
order, and must be the left-most arguments of the type constructor. So all of these are
valid:

```rust
impl Trait for Type {
    type Foo<'a> = &'a u32;
    type Bar<'a> = SomeType<'a, 'static>;
    type Baz<'a, 'b> = SomeType<'a, 'b>;
    type Quux<'a> = Self::Foo<'a>;
}
```

But these are not valid:

```rust
impl Trait for Type {
    type Foo<'a> = String; // ERROR! Argument never used.
    type Bar<'a> = SomeType<'static, 'a>; // ERROR! Argument must be left-most.
    type Baz<'a, 'b> = SomeType<'b, 'a>; // ERROR! Arguments are used in wrong order.
    type Quux<'a> = (&'a i32, &'a i32); // ERROR! Argument is used more than once.
}
```

All of these restrictions can be avoided (unpleasantly) with newtypes and phantomdata:

```rust
struct ValidFoo<'a>(String, PhantomData<&'a ()>);
struct ValidBar<'a>(SomeType<'static, 'a>);
struct ValidBaz<'a, 'b>(SomeType<'a, 'b>);
struct Quux<'a>(&'a i32, &'a i32);
```

If this feature is extended to type arguments, the restriction remains the same within each
kind - so the same restriction with "lifetime" replaced with "type" is also added, but they do
not intermingle.

## Future extensions

The most immediate future extension to this feature is extending it to type arguments.

For example:

```rust
trait Foo {
    type Bar<T>;
}
```

This sort of extension would enable to this feature to encode all forms of higher kinded
polymorphism, with some boilerplate, using the "family" pattern:

```rust
trait PointerFamily {
    type Pointer<T>: Deref<Target = T>;
    fn new<T>(value: T) -> Self::Poiner<T>;
}

struct Foo<P: PointerFamily> {
    bar: P::Pointer<String>,
}
```

Beyond this, this feature is intended to be compatible with extensions to "full HKT"
in the future.

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

## Do not impose restrictions on associated type constructors

The restrictions imposed on this feature are only to be forward compatible
with other forms of higher kinded polymorphism. If we decided that we didn't
want to include those features ever, or that we were fine with those features
being totally disjoint from this one, we could not include those restrictions
in this RFC.

# Unresolved questions
[unresolved]: #unresolved-questions

This design does not resolve the question of introducing more advanced forms of
higher-kinded polymorphism.
