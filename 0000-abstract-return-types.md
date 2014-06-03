- Start Date: 2014-06-02
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Allow functions to return _unboxed abstract types_, written `impl Trait` for
"some hidden type `T` such that `T: Trait`".

These types specify a trait bound, but hide the concrete type implementing the
bound. Unlike trait objects, but like generics, the actual type of an unboxed
abstract type is statically known and sized, and will thus generate
statically-dispatched code.

Optionally, allow unboxed abstract type syntax to be used elsewhere (function
arguments, structs) as a shorthand for generics.

# Motivation

In today's Rust, you can write a function signature like
````rust
fn consume_iter_static<I: Iterator<u8>>(iter: I)
fn consume_iter_dynamic(iter: Box<Iterator<u8>>)
````
In both cases, the function does not depend on the exact type of the argument.
The type is held "abstract", and is assumed only to satisfy a trait bound.
* In the `_static` version using generics,
each use of the function is specialized to a concrete, statically-known type,
giving static dispatch, inline layout, and other performance wins.
* In the `_dynamic` version using trait objects, the concrete argument type is
  only known at runtime using a vtable.

On the other hand, while you can write
````rust
fn produce_iter_dynamic() -> Box<Iterator<u8>>
````
you _cannot_ write something like
````rust
fn produce_iter_static() -> Iterator<u8>
````
That is, in today's Rust, abstract return types can only be written using trait objects, which
can be a significant performance penalty. This RFC proposes "unboxed abstract
types" as a way of achieving signatures like `produce_iter_static`. Like
generics, unboxed abstract types guarantee static dispatch and inline data
layout.

Here are some problems that unboxed abstract types solve or mitigate:

* _Returning unboxed closures_. The ongoing work on unboxed closures expresses
  closures using traits. Sugar for closures generates an anonymous type
  implementing a closure trait. Without unboxed abstract types, there is no way
  to use this sugar while returning the resulting closure unboxed, because there
  is no way to write the name of the generated type.

* _Leaky APIs_. Functions can easily leak implementation details in their return
  type, when the API should really only promise a trait bound. For example, a
  function returning `Rev<Splits<'a, u8>>` is revealing exactly how the iterator
  is constructed, when the function should only promise that it returns _some_
  type implementing `Iterator<u8>`. Using newtypes/structs with private fields
  helps, but is extra work. Unboxed abstract types make it as easy to promise only
  a trait bound as it is to return a concrete type.

* _Complex types_. Use of iterators in particular can lead to huge types:
  ````rust
  Chain<Map<'a, (int, u8), u16, Enumerate<Filter<'a, u8, vec::MoveItems<u8>>>>, SkipWhile<'a, u16, Map<'a, &u16, u16, slice::Items<u16>>>>
  ````
  Even when using newtypes to hide the details, the type still has to be written
  out, which can be very painful. Unboxed abstract types only require writing the
  trait bound.

* _Documentation_. In today's Rust, reading the documentation for the `Iterator`
  trait is needlessly difficult. Many of the methods return new iterators, but
  currently each one returns a different type (`Chain`, `Zip`, `Map`, `Filter`,
  etc), and it requires drilling down into each of these types to determine what
  kind of iterator they produce.

In short, unboxed abstract types make it easy for a function signature to
promise nothing more than a trait bound, and do not generally require the
function's author to write down the concrete type implementing the bound.

# Detailed design

> *Note*: this design borrows from
>
> * https://github.com/mozilla/rust/issues/11455,
> * https://github.com/mozilla/rust/issues/10448 and
> * https://github.com/mozilla/rust/issues/11196.

The basic idea is to allow code like the following:
````rust
pub fn produce_iter_static() -> impl Iterator<int> {
    range(0, 10).rev().map(|x| x * 2).skip(2)
}
````
where `impl Iterator<int>` should be understood as "some type `T` such that `T:
Iterator<int>`.  Notice that the function author does not have to write down any
concrete iterator type, nor does the function's signature reveal those details
to its clients. But the type promises that _there exists_ some concrete type.

This code is roughly equivalent to
````rust
pub struct Result_produce_iter_static(
    iter::Skip<iter::Map<'static,int,int,iter::Rev<iter::Range<int>>>>
);

impl Iterator<int> for Result_produce_iter_static {
    fn next(&mut self) -> Option<int> {
        match *self {
            Result_produce_iter_static(ref mut r) => r.next()
        }
    }
}

pub fn produce_iter_static() -> Result_produce_iter_static {
    Result_produce_iter_static(
        range(0, 10).rev().map(|x| x * 2).skip(2)
    )
}
````

That is, using an unboxed abstract type is _semantically_ equivalent to
introducing an _anonymous_ newtype where:
* the representation is private
* the newtype implements the given trait bound by delegating to the representation

An unboxed abstract return type `impl Trait` has several implications for the compiler:

* _Typechecking the function_: The compiler should check that the inferred actual
  return type `T` of the function satisfies the trait bound `Trait`. This is a local check.

* _Typechecking the clients_: The compiler should treat each use of `impl Trait`
  as a fresh unknown type that satisfies the bound `Trait`. Thus, function
  clients cannot depend on the concrete return type, only the bound, which
  retains local typechecking.

* _Code generation_: The compiler should record the concrete return type `T` for
  the function for the purposes of specialization. Clients that use the return
  type should generate the same code as if they has used the concrete type `T`,
  despite that typechecking prevents them from relying on anything about `T`
  except the trait bound `Trait`.  (This can be viewed as a form of
  monomorphization for abstract types, which stand for a single type, in contrast
  to generics (universal types), which stand for a family of types.)

So, for example, given the function signatures
````rust
fn produce_iter_static() -> impl Iterator<int>
fn consume_iter_static<I: Iterator<u8>>(iter: I);
````
the expression
````rust
consume_iter_static(produce_iter_static())
````
should typecheck, and should compile using static dispatch. On the other hand,
````rust
let iter: iter::Skip<_> = produce_iter_static();
````
should fail to typecheck, since it relies on the concrete return type of
`produce_iter_static`, which is hidden behind the unboxed abstract return
type.

The subsections below fill in several details necessary for implementing this
high-level design.

## Generics and lifetime parameters

A function returning an unboxed abstract type might also use generic types or
lifetime parameters:

````rust
impl<T> SomeCollectionType<T> {
    fn iter<'a>(&'a self) -> impl Iterator<&'a T> { ... }
}
````

In general, the concrete return type may be parameterized by any type or
lifetime variable in scope. These parameters may also be mentioned in the trait
bound for the unboxed abstract type.

Just as is currently done for trait objects, the typechecker must ensure that
lifetime parameters are not stripped when using an unboxed abstract type.
For example (adapted from @glaebhoerl):
````rust
fn evil<T, Iter: Iterator<T>>(iter: Iter) -> Box<Iterator<T>> {
    box iter as Box<Iterator<T>>
}
````
generates the error "value may contain references; add `'static` bound to
`Iter`".  Since unboxed abstract types are the statically-dispatched equivalent
to returning trait objects, the same logic should apply:
````rust
fn evil<T, Iter: Iterator<T>>(iter: Iter) -> Box<impl Iterator<T>> {
    box iter as Box<Iterator<T>>
}
````
should not be allowed for the same reason.

## Possible add-on: unboxed abstract types in non-return positions

_**Note**: this is an optional feature._

So far, the RFC has assumed that the new use of `impl` for unboxed abstract types
is only allowed for the return type of a function. In principle, it makes sense
to allow unboxed abstract types anywhere that a type is allowed.

**Note**: the extensions below are somewhat orthogonal. For example, the
  extension to include function arguments does not depend on including `struct`
  types.

### Function arguments

Instead of writing
````rust
fn extend<I: Iterator<T>>(&mut self, iterator: I)
````
we could write
````rust
fn extend(&mut self, iterator: impl Iterator<T>)
````
These two signatures are almost equivalent, but the first has an _explicit_ type
argument (which can be provided using `::` syntax) while the second has an
_implicit_ type argument.

Using unboxed abstract types in arguments makes (simple) static and dynamic
dispatch syntactically closer:
````rust
fn extend_static(&mut self, iterator: impl Iterator<T>)
fn extend_dynamic(&mut self, iterator: &Iterator<T>)
````

It may be especially useful for passing unboxed closures as arguments in a
pleasant way:
````rust
fn use_fn<T: Fn<u8, ()>>(f: T)
````
versus
````rust
fn use_fn(f: impl Fn<u8, ()>)
````

### Nested results

Rather than only supporting

````rust
pub fn produce_iter_static() -> impl Iterator<int>
````

unboxed abstract types could also be permitted in nested form:

````rust
pub fn produce_iter_box() -> Box<impl Iterator<int>>
pub fn produce_iters() -> (impl Iterator<int>, impl Iterator<int>)
````

Each use of `impl` would mark a distinct abstract type, so `produce_iters`
could provide different concrete iterator types for the first and second
components of the tuple.

### Structs and other compound types

**Note**: the motivation for this extension is purely consistency &mdash;
  allowing `impl Trait` wherever a type is allowed. There is no known concrete
  use case, which is perhaps a strong argument against including the
  feature. (Personally, I think this is probably a bad idea.)

If we truly want to allow `impl Trait` everywhere, that would include `struct` fields as well.
````rust
struct Foo {
    s: impl Set<u8>;
}
````
would be equivalent to
````rust
struct Foo<T: Set<u8>> {
    s: T;
}
````
except that the type argument would be treated as _implicit_, meaning that
you would write
````rust
fn use_foo(f: &Foo) { ... }
````
and the function `use_foo` would itself implicitly parameterize over the
`Set<u8>` implementation.  Similarly,
````rust
fn make_foo() -> Foo { ... }
````
would implicitly treat the `Set<u8>` field as an abstract type, which can be
implemented by an arbitrary (but hidden) concrete type.

## Possible add-on: type equality and `Self`

_**Note**: this is an optional feature._

Although the clients of an unboxed abstract types do not know its concrete type,
they can rely on there being _some_ consistent concrete type. For example, given
a function
````rust
fn collect_to_set<T, I: Iterator<T>>(iter: I) -> impl Set<T>
````
the code
````rust
let s1 = collect_to_set(iter1);
let s2 = collect_to_set(iter2);
s1.is_subset(s2)
````
should typecheck as long as `iter1` and `iter2` are iterators over the same
type, since the underlying concrete type implementing `Set<T>` is guaranteed to
be the same.

More generally, the typechecker could treat each unboxed abstract type as
implicitly parameterized over all of the in-scope type and lifetime variables.
Two uses of the same existential type with the same parameters should be treated
as the same type.  In terms of the newtype expansion suggested earlier, this is the
equivalent of having the newtype take all of the in-scope type and lifetime
parameters.

## Staging

This RFC can be accepted and/or implemented in stages, in the following order of priority:

* Unboxed abstract return types, by far the most important use-case
* Unboxed abstract argument types
* Unboxed abstract types in structs/enums/tuples
* Equality/Self for unboxed abstract types

The first two bullets would affect library API stabilization; the others are nice-to-haves.

# Drawbacks

The main downside is complexity. It can already be difficult to understand the
distinction between generics and trait objects, and this RFC proposes another
generics-like mechanism for unboxed abstract return types. This drawback is
discussed in more detail in the following section on Alternatives.

Other drawbacks are specific to the optional add-ons. In particular, adding
`impl Trait` fields to `struct`s adds a somewhat scary form of implicitness,
since code using the `struct` will be silently monomorphized without any
explicit use of generics.

# Alternatives

The proposal here, with all the add-ons, ultimately allows `impl Trait` to be
used wherever a type can be used.  This is partly for consistency and partly as
a lighter weight way to write generics. The downside is that we would be adding
yet another distinction to the type system, and quite a bit of implicit
parameterization/monomorphization.

## A very conservative alternative

A more targeted approach would be to _just_ tackle return types, and try to make
them look more akin to generics:
````rust
fn produce_iter_static() -> _ : Iterator<u8>
````
This more conservative proposal would be easier to implement, and would add less
complexity to the language, while still addressing the primary use case for the
RFC.

## A somewhat conservative alternative

Finally, a midpoint would be to use `impl Trait` but restrict it to function
signatures, not allowing it in `struct` fields. That design:

1. retains the benefit of syntactically lightweight static dispatch
2. keeps the nice property that, by looking at a function signature alone, it is
   possible to tell exactly where monomorphization will happen.

This design might also make it possible for the implicit type parameters for
`impl Trait` to be provided using `::` syntax.

# Unresolved questions

## Multiple trait bounds

The design should _definitely_ be generalized to support multiple trait bounds
(`impl Trait1 + Trait2`), since it is common for a return type to implement
several traits of interest. (Iterator return types are a good example). This
should not be any harder to implement than the basic design, but the syntax may
need some thought.

## Naming an abstract return type

The above proposal for type equality allows multiple uses of the same
existential to be treated as equal types. However, there is no way to name
that _concrete_ type, so it cannot be stored in a `struct` or passed as an
argument.

**Note**: this is different from using `impl Trait` in a `struct` field, which
  allows _any_ type implementing `Trait`. What we cannot do is have a `struct`
  field whose type is "whatever concrete type function `foo` returns".

Thinking again of the signature
````rust
fn collect_to_set<T, I: Iterator<T>>(iter: I) -> impl Set<T>
````
we could allow naming the concrete result type by a path like
`collect_to_set::<T, I>::impl`. The only way to get a value of this type is by
calling `collect_to_set`. Thinking in terms of the newtype encoding of unboxed
abstract types, this is just a way to _name_ the concrete newtype used.

This should be considered as part of any design for associated types, which also
involve paths to types.
