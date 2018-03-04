- Feature Name: `ghost_busting`
- Start Date: 2018-03-04
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

[`PhantomData<T>`]: https://doc.rust-lang.org/nightly/nomicon/phantom-data.html
[variance]: https://doc.rust-lang.org/nightly/nomicon/subtyping.html
[drop checking]: https://doc.rust-lang.org/nightly/nomicon/dropck.html

1. Defines unused type parameters to act as if they are logically owned
   and covariant.

2. Introduces `phantom T` pseudo-fields which improves the ergonomics of
   changing [variance] and [drop checking] behavior when it is needed.

3. [`PhantomData<T>`] is redefined as `struct PhantomData<T: ?Sized>;`
   and deprecated.

4. The lang item `phantom_data` is removed.

5. Derive macros for derivable standard library traits will take advantage
   of statically known phantom types and fields to generate more permissive
   `impl`s.

# Motivation
[motivation]: #motivation

## Improving ergonomics and rapid prototyping

Today, it is impossible to define a type with unused type parameters:

```rust
struct Label<T>;
```

Instead, you must define the type as:

```rust
use std::marker::PhantomData;

struct Label<T> {
    marker: PhantomData<T>,
}
```

or equivalently:

```rust
struct Label<T>(PhantomData<T>);
```

This is a pain point since users must now:

1. import `std::marker::PhantomData`,

2. add a field of type [`PhantomData<T>`],

3. acknowledge the field by adding an expression `PhantomData`
   when a value of the type is to be constructed.

These steps add unnecessary boilerplate and is in the way of rapid prototyping
and developer flow when you are removing and adding fields.

By making `struct Label<T>;` be owning and covariant by default in `T`,
ergonomics of generics and label types are improved and `PhantomData<T>`
becomes just a library type.

For types where you really need to change the variance and drop checking
behavior such as for the following `MyVec<T>` type, we gain in ergonomics
by being able to skip steps 1 and 3 and instead write:

```rust
struct MyVec<T> {
    phantom T,
    data: *const T,
    len: usize,
    cap: usize,
}
```

## More permissive automatic deriving

Considering the `Label<T>` type above again, and a few derives:

```rust
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Label<T> {
    marker: PhantomData<T>,
}
```

we will now get get following `impl`s:

```rust
impl<T: Copy>       Copy       for Label<T> { /* .. */ }
impl<T: Clone>      Clone      for Label<T> { /* .. */ }
impl<T: Debug>      Debug      for Label<T> { /* .. */ }
impl<T: Default>    Default    for Label<T> { /* .. */ }
impl<T: PartialEq>  PartialEq  for Label<T> { /* .. */ }
impl<T: Eq>         Eq         for Label<T> { /* .. */ }
impl<T: PartialOrd> PartialOrd for Label<T> { /* .. */ }
impl<T: Ord>        Ord        for Label<T> { /* .. */ }
impl<T: Hash>       Hash       for Label<T> { /* .. */ }
```

Notice the bounds on the type parameter `T` in all of these `impl`s. They are
completely unnecessary and restrictive. Instead, we would like to generate:

```rust
impl<T> Copy       for Label<T> { /* .. */ }
impl<T> Clone      for Label<T> { /* .. */ }
impl<T> Debug      for Label<T> { /* .. */ }
impl<T> Default    for Label<T> { /* .. */ }
impl<T> PartialEq  for Label<T> { /* .. */ }
impl<T> Eq         for Label<T> { /* .. */ }
impl<T> PartialOrd for Label<T> { /* .. */ }
impl<T> Ord        for Label<T> { /* .. */ }
impl<T> Hash       for Label<T> { /* .. */ }
```

But deriving macros can't generate such `impl`s since it can't be completely
sure that `PhantomData<T>` really is `::core::marker::PhantomData<T>`.
Macros can chance it, but that is not reliable, which is why deriving macros
for derivable standard library traits don't assume getting `PhantomData<T>`
as a field type in the input token stream really is that type. Instead we
are faced with having to implement the traits manually for the type, which
is a waste. If however, a macros gets the following input:

```rust
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Label<T>;
```

it can know statically that `T` is a phantom type. This also extends to
`phantom <type>`. Let's consider a simple refl encoding in Rust, which
requires invariance in the parameters `A` and `B`:

```rust
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Id<A: ?Sized, B: ?Sized> {
    // fn(A) -> A makes A invariant:
    phantom (fn(A) -> A, fn(B) -> B)
}
```

since `phantom` is syntactically special here, a derive macro can statically
know that `A` and `B` are phantom types and the following `impl`s can be
generated automatically:

```rust
impl<A, B> Copy       for Id<A, B> { /* .. */ }
impl<A, B> Clone      for Id<A, B> { /* .. */ }
impl<A, B> Debug      for Id<A, B> { /* .. */ }
impl<A, B> Default    for Id<A, B> { /* .. */ }
impl<A, B> PartialEq  for Id<A, B> { /* .. */ }
impl<A, B> Eq         for Id<A, B> { /* .. */ }
impl<A, B> PartialOrd for Id<A, B> { /* .. */ }
impl<A, B> Ord        for Id<A, B> { /* .. */ }
impl<A, B> Hash       for Id<A, B> { /* .. */ }
```

## Suggested in part by [RFC 738]

[future possibilities]: https://github.com/rust-lang/rfcs/blob/master/text/0738-variance.md#future-possibilities

In discussing [future possibilities], RFC 738 suggests the exact syntax of
`phantom` fields proposed in this RFC and then goes on to say that:

> This would improve the usability of phantom markers greatly.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## Recommended background reading

Before reading this RFC, it is highly recommended that you familiarize
yourself with the basics of [`PhantomData<T>`], [drop checking] and
[variance] in the Rustonomicon.

## Unused type parameters are allowed, logically owned and covariant

If you define a type in Rust today such as:

```rust
struct Label<T>;
```

or:

```rust
struct Foo<S, T> {
    bar: S,
    baz: usize,
}
```

the compiler will greet you with the error messages:

```rust_errors
error[E0392]: parameter `T` is never used
 --> src/main.rs:3:15
  |
3 | struct Foo<S, T> {
  |               ^ unused type parameter
  |
  = help: consider removing `T` or using a marker such as `std::marker::PhantomData`
```

and:

```rust_errors
error[E0392]: parameter `T` is never used
 --> src/main.rs:3:14
  |
3 | struct Label<T>;
  |              ^ unused type parameter
  |
  = help: consider removing `T` or using a marker such as `std::marker::PhantomData`
```

With this RFC implemented, you will stop receiving the error message `E0392`
(unused parameter) for type parameters and the type definitions above will act
as if they logically own a `T` both in terms of being [covariant][variance] and
in terms of [drop checking]. In other words, for each unused type parameter `P`,
it will be as if you had added a marker field with `PhantomData<P>` to the type.

Therefore, the type definitions above will be legal and you will not need
to use [`PhantomData<T>`], unless you actually need to change the variance
to contravariant or invariant and instruct [drop checking] to not see `T`
as logically owned.

As a consequence of not needing [`PhantomData<T>`] anymore, the type will be
deprecated and simply defined trivially as: `struct PhantomData<T: ?Sized>;`.
The lang item `phantom_data` will also be removed.

To construct a `Label<T>` or `Foo<S, T>` as defined above, you can simply write:

```rust
fn main() {
    let lab: Label<u8> = Label;
    let foo: Foo<u8, ()> = Foo {
        bar: 1u8,
        baz: 2usize,
    };
}
```

Note in particular that we didn't have to, and indeed can't, set a field
`marker: PhantomData` in the struct literals above.

## `phantom` fields

Given the following type `MyVec<T>`, `T` will be covariant but also not counted
as logically owned by the drop checker. Therefore, `MyVec<T>` is not sound.

```rust
struct MyVec<T> {
    data: *const T,
    len: usize,
    cap: usize,
}
```

Before this RFC, this was solved by adding a marker field with the type
`PhantomData<T>` as in:

```rust
use std::marker::PhantomData;

struct MyVec<T> {
    marker: PhantomData<T>,
    data: *const T,
    len: usize,
    cap: usize,
}
```

This made `T` logically owned and covariant. With this RFC, you will
instead write:

```rust
struct MyVec<T> {
    phantom T,
    data: *const T,
    len: usize,
    cap: usize,
}
```

Another example of using `phantom` fields which was discussed in the
[motivation] is:

```rust
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Id<A: ?Sized, B: ?Sized> {
    // fn(A) -> A makes A invariant:
    phantom (fn(A) -> A, fn(B) -> B)
}

impl<A: ?Sized> Id<A, A> { pub const REFL: Self = Id; }

impl<A: ?Sized, B: ?Sized> Id<A, B> {
    pub fn cast(self, value: A) -> B where A: Sized, B: Sized {
        unsafe {
            let cast_value = mem::transmute_copy(&value);
            mem::forget(value);
            cast_value
        }
    }
}
```

In this particular example, and to maintain type and memory safety, we need to
change the default covariant variance to invariant because we would otherwise
introduce the ability to transmute lifetimes in safe Rust with:

```rust
fn transmute_lifetime<'a, 'b, T: 'a + 'b>(r: &'a T) -> &'b T {
    Id::REFL.cast(r)
}
```

### Not nameable

It is important to note that the introduced `phantom` fields are truly
phantom, unlike with `marker: PhantomData<T>`, since you can't reference
or name a `phantom` field either during construction in a literal expression
or with field access notation `expr.field`.

A consequence of this is that given the following type, you can coerce `Foo`
into a function pointer of type `fn(u8) -> Foo<u8, SomeType>`:

```rust
struct Foo<X, Y>(X, phantom Y);
```

### Visibility

[visibility modifiers]: https://doc.rust-lang.org/nightly/reference/visibility-and-privacy.html

While `phantom` fields are not nameable, you can put [visibility modifiers] on
them or none, in which case the default private visibility is used. Visibility
on a `phantom` field has the standard effect that the type it is in can't be
constructed unless the "field" is visible in the module in question.

This property is crucial because if `phantom` fields always were public, then
the a value of type `Id<A, B>` where `A != B` could be constructed with `Id {}`
and now we've introduced unsoundness into the type system. Always-private
visibility would also be undesirable since it would be too restrictive.

[screeps_api]: https://dabo.guru/rust/screeps-api/src/screeps_api/endpoints/recent_pvp.rs.html

Respecting privacy also allows us to encode the [following idiom][screeps_api]:

```rust
pub struct Thing {
    pub foo: usize,
    pub bar: String,
    /// Permits adding fields in the future since () is private.
    phantom (),
}
```

### Attributes

The keen reader will also have noticed here that attributes are permitted on
`phantom` fields as doc comments `/// Some documentation...` are [just sugar]
for `#[doc="Some documentation..."]`.

[just sugar]: https://doc.rust-lang.org/book/first-edition/documentation.html#doc-attributes

## Unused lifetimes

Note that `E0392` will still be issued for unused lifetimes since given
`struct Foo<'a, T, U>;`, the compiler can't decide, in a way that is
intuitive for a reader, whether `&'a ()`, `&'a T`, `&'a U`, or some
combination thereof is logically owned. Any rule for how to handle
`struct Foo<'a> {..}`, `struct Foo<'a, T> {..}`, .., would likely
be too complex for a user's mental model.

Instead, unused lifetimes becomes more ergonomic to handle with
`phantom` fields since we can write:

```rust
#[derive(Clone)] // We gained the ability to derive Clone!
pub struct Iter<'a, T: 'a> {
    phantom &'a T
    ptr: *const T,
    end: *const T,
}

pub struct IterMut<'a, T: 'a> {
    phantom &'a mut T,
    ptr: *mut T,
    end: *mut T,
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

[RFC 738]: https://github.com/rust-lang/rfcs/blob/master/text/0738-variance.md

This RFC supercedes [RFC 738] in some aspects.

## `PhantomData<T>` is deprecated

The lang item `phantom_data` is removed and the type `PhantomData<T>` in
`core::marker` (re-exported in `std::marker`) is deprecated and redefined as:

```rust
struct PhantomData<T: ?Sized>;
```

## Unused type parameters are allowed, logically owned and covariant

Any unused type (*not lifetime*) parameter `P` of a type definition `Type` is
permitted, covariant, and counted as logically owned by the drop checker.

Any unused type parameter `P` is treated semantically as equivalent to adding a
`phantom` field of type `P` to the type definition with `pub` visibility. That
is, the following type:

```rust
struct Foo<P> {
    // other fields not referencing P
}
```

is equivalent to:

```rust
struct Foo<P> {
    phantom P,
    // other fields not referencing P
}
```

## `phantom` fields

### Grammar

Assuming a production:

1. `ty_sum` for types permitted in Rust.
2. `attrs_and_vis` for attributes and visibility modifiers,
3. `ident` for identifiers,
4. `expr` for expressions,

The productions:

```
struct_decl_field : attrs_and_vis ident ':' ty_sum ;

struct_tuple_field : attrs_and_vis ty_sum ;

enum_args : '{' struct_decl_fields '}'
          | '{' struct_decl_fields ',' '}'
          | '(' maybe_ty_sums ')'
          | '=' expr
          | %empty
          ;
```

are redefined as:

```
struct_decl_field : attrs_and_vis named_field ;
struct_tuple_field : attrs_and_vis ty_sum ;

enum_args : '{' struct_decl_fields '}'
          | '{' struct_decl_fields ',' '}'
          | '(' maybe_tuple_fields ')'
          | '=' expr
          | %empty
          ;

phantom : 'phantom' ty_sum ;

named_field : phantom | ident ':' ty_sum ;
tuple_field : phantom | ty_sum ;

tuple_fields : tuple_field | tuple_fields ',' tuple_field ;
maybe_tuple_fields : tuple_fields | tuple_fields ',' | %empty ;
```

### Semantics

A `phantom` field is an unnameable pseudo field which can be added as a field
to a type definition in the case of `struct`s and `union`s or variants in the
case of an `enum`.

#### No representation in memory - only in logic

These `phantom` fields do not contribute to the memory layout or representation
of a type but only to the variance of type parameters and how the drop checker
sees type parameters in terms of ownership. For each phantom field of form
`phantom T` where `T` is some type, `T` is seen as logically owned by the drop
checker and `T` will contribute to the variance of any, by `T` referenced,
formal type parameter `P` or lifetime `'lt`.

#### Unnameable

Being unnameable means in this context that `phantom` fields do not have names
and as such, they can't be referred to with field access syntax `<expr>.<field>`
as well as in struct literal expressions for structs with named fields as in:
`Type { field: <expr> }` or for tuple structs `Type(<expr>)`. As a consequence, 
you can coerce the following type `Foo` into a function pointer of type
`fn(u8) -> Foo<u8, SomeType>`:

```rust
struct Foo<X, Y>(X, phantom Y);
```

#### Phantom values

Since `phantom` fields have no representation in memory, there is also no
"phantom value" which you may construct. Instead, the valid way to construct a
`Foo<T, SomeType>` is `Foo(val)` where `val : T`.

#### Visibility and attributes

While `phantom` fields are not nameable, they permit visibility modifiers on
them. The default visibility of a `phantom` field is private as with other
fields. Therefore, having a type definition with no fields but private `phantom`
fields will cause other modules not to be able to construct the type.

As `phantom` fields contribute logically to type definitions, attributes are
permitted on them including doc comments in the form of `/// Documentation..`.

## Deriving with `phantom` fields and unused type parameters

Given a `phantom Type` field in a type definition, where `Type` is a type
referencing a type parameter `Param` of that type definition, a derive macro
for a derivable standard library trait `Trait`, which receives the type
definition as its input, will not bound `Param: Trait` when it is decidable
that `Param` is not referenced in a normal field of the type definition.
If such analysis is undecidable, then a bound `Param: Trait` will be enforced.

Unused type parameters `Param` are treated in this regard as if `phantom Param`
was added for each parameter.

Crates in the ecosystem are *encouraged* to adopt a similar approach as the
derive macros in the standard library.

# Drawbacks
[drawbacks]: #drawbacks

## Of allowing unused type parameters

+ Allowing users to elide `phantom T` will delay how long it takes before
aspiring new rustaceans will have to learn about `phantom T` or indeed 
`PhantomData<T>` if `phantom` fields are not introduced. Forcing the user
to learn earlier and continuously think about these things might be good.

+ Fields will no longer be purely additive in terms of variance, because
having zero fields would start with a logically owned covariant `T` instead
of a bivariant and non-owned `T`. This drawback seems however mostly academic
and it seems there is no or little practical value of maintaining this additive
property.

*Note: Bivariance means that both subtypes and supertypes are allowed,
which you never want.*

Allowing unused type parameters is expected to decrease instead of increase
mental complexity as users won't have to think about `PhantomData<T>` or
`phantom T` unless it is really needed.

## Of introducing `phantom` fields

The drawback here is mainly that of increased compiler complexity. A quick
search showed possible refactorings with respect to not having to provide
`PhantomData` as an expression can offset this drawback.

# Rationale and alternatives
[alternatives]: #alternatives

This RFC is purely about ergonomics. Everything proposed in the RFC is already
expressible today, but not in an ergonomic way.

The reasoning behind an unused type parameter `T` being counted as owned and
covariant is that this is the more common case as well as being more intuitive.
This also has the benefit that `PhantomData<T>` becomes a pure library type
without the lang item `phantom_data`.

## Alternative: Do nothing

We can always do nothing and decide that the status quo is fine and that
there's no problem in need of solving.

## Alternative: No unused type parameters

We could decide to have `phantom` fields but no unused type parameters and give
up on the majority of the gains in ergonomics in this RFC.

## Alternative: No `phantom` fields

We could decide to not have `phantom` fields but allow unused type parameters.
The drawback of this is that `PhantomData<T>` where it really matters isn't
made more ergonomic.

## Alternative: Invariance by default

Another, more conservative, option is that a unused type parameter `T` should
be defined as invariant by default.

It would be a breaking change to change invariance to covariance since if we
consider the type `struct Id<S, T>(());`, which allows casting values of type
`S` to type `T`, then changing invarance to covariance allows you to transmute
lifetimes in safe code. However, a user may always manually change to logically
owned covariance in this case with `PhantomData<T>`. 

Assuming that we decide not to introduce `phantom` fields, another downside is
that the lang item `phantom_data` can't be removed and that `PhantomData<T>`
can't be defined as a pure library type.

A downside to invariance by default is that it isn't tuned to what the
user usually wants. We conjecture that there are more cases where owned
covariance is used in the ecosystem than invariance. There are a few reasons
why this is a reasonable conjecture:

1. Invariance tends to be important and come up when you are dealing with
   `unsafe` code, which turns out to be rather rare. In these cases, there
   are particular reasons why a parameter `P` can't be covariant.

2. Unconstrained type parameters are solved with `PhantomData<T>`.
   An example of this is `pub struct Empty<T>(marker::PhantomData<T>);` in
   `std::iter`. This tends to show up quite often. Another case of this is
   `pub struct BuildHasherDefault<H>(marker::PhantomData<H>);`.

3. `PhantomData<T>` is often used when building abstractions that really own
   `T` in memory such as `pub struct RawTable<K, V>` in the standard library.
   And if these data structures do not really own a value of type `T`, they
   often can or will produce owned values of type `T` as in the case of
   iterators (`Empty<T>`), futures, or strategies to randomly generate a value.
   This means that owned `T`s are reachable from the type which contained the
   `PhantomData<T>`.

## Alternative: Allow filling unspecified fields with `Default`

Simply put, this would mean that a struct literal such as `Foo { .. }` would
be a shorthand for the FRU `Foo { .. Default::default()`. This makes dealing
with `PhantomData` more ergonomic since expressions involving those do not
have to write out `field: PhantomData`. However, this does not solve the issue
of pain when dealing with type definitions.

# Prior art
[prior-art]: #prior-art

To some extent, languages such as Haskell and Idris provide prior art in the
sense that you never need to use all type parameters in a `data` definition.
The following constant combinator type is permitted by GHC and does not use
`b` on the right hand side:

```haskell
newtype K a b = K a
```

or in Idris:

```idris
data K a b = K a
```

However, neither Haskell or Idris have subtyping.

Let's now instead consider a different language, Java, which does have subtyping
but of a different kind than Rust's flavor of limited subtyping:

```java
class Main {
  public static void main(String[] args) {
    // Invariance by default:
    // OK! A type X can be substituted for itself.
    K<S> k1 = new K<S>();
    // Err! Not covariant, T <: S =/> K<T> <: K<S>.
    K<S> k2 = new K<T>();
    // Err! Not contravariant, S <: R =/> K<R> <: K<S>.
    K<S> k3 = new K<R>();

    // Opt-in Covariance:
    // OK! Subtyping is reflexive.
    K<? extends S> k4 = new K<S>();
    // OK! T <: S => K<T> <: K<? extends S>.
    K<? extends S> k5 = new K<T>();
    // Err! Not contravariant, S <: R =/> K<R> <: K<? extends S>.
    K<? extends S> k6 = new K<R>();

    // Opt-in Contravariance:
    // OK! Reflexivity, S <: S.
    K<? super S> k7 = new K<S>();
    // OK! S <: R => K<R> <: K<? super S>.
    K<? super S> k8 = new K<R>();
    // Err! Not covariant, T <: S =/> K<T> <: K<S>
    K<? super S> k9 = new K<T>();
  }
}

class R {}
class S extends R {}
class T extends S {}
class K<A> {}
```

We see that generic parameters are invariant by default and that you change the
variance at use site of types rather than on definition site which you can't in
Rust. We also see that Java permtis "unused" type parameters.

For C#, invariance is the default in generic interfaces, which you may
change at the interface definition site with by annotating type parameters
with [`out`](put) which makes them covariant and `in`(put) which makes them
contravariant. [For arrays], covariance is the default.

[For arrays]: https://docs.microsoft.com/en-us/dotnet/csharp/programming-guide/concepts/covariance-contravariance/index

[`out`]: https://docs.microsoft.com/en-us/dotnet/csharp/programming-guide/concepts/covariance-contravariance/creating-variant-generic-interfaces

It is important to keep in mind that compared to Java and C#, Rust's notion
of subtyping derives soley from our notion of lifetimes and their partial
ordering. For Rust, questions are more about whether `Ctor<&'static T>` can
be coerced to `Ctor<&'a T>` where `'static <: 'a` or not.

# Unresolved questions
[unresolved]: #unresolved-questions

1. Should the default for unused type parameters be invariance instead of
   covariance?

2. Do we permit visibility on `phantom` fields?

3. Should `PhantomData<T>` be deprecated? There might be some cases where you
   actually want to pass around a `PhantomData` value as a proxy; but in that
   case, a better named type such as `Proxy<T>` is more apt. Deprecation also
   has the benefit that it will drive users towards adopting `phantom T` or
   eliding the phantom entirely.