- Feature Name: derive_unfinished
- Start Date: 2017-10-20
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Allows the user to derive any trait for any type with
`#[derive_unfinished(Trait1, Trait2, ...)]` by:

[`ConstDefault`]: https://github.com/Centril/rfcs/blob/rfc/const-default/text/0000-const-default.md
[`unimplemented`]: https://doc.rust-lang.org/nightly/std/macro.unimplemented.html

+ panicing via [`unimplemented`] in any method,
+ using [`ConstDefault`] for associated constants where implemented,
+ using defaults for associated types if such a default exists.

This desugars to a new `#[unfinished] impl Trait for Type { .. }` construct
where the user may leave out any method they wish to not give an implementation
for currently, as well as specify any associated type which does not have a
default in the trait or any associated constant for which the type is not
[`ConstDefault`].

The `#[unfinished]` construct in turn desugars into a full implementation of the
trait for the type which uses [`unimplemented`] for methods and uses
defaults where possible.

# Motivation
[motivation]: #motivation

The documentation of [`unimplemented`] reads:

> This can be useful if you are prototyping and are just looking to have your
> code typecheck, or if you're implementing a trait that requires multiple
> methods, and you're only planning on using one of them.

This RFC wholeheartedly agrees with this "typecheck first, implement later"
approach and aims to make it even easier and simpler to do so.

Let's start with the example that `unimplemented!` uses.
First, there are some preliminaries both this RFC and `unimplemented!` needs:

```rust
trait Foo {
    fn bar(&self);
    fn baz(&self);
}

struct Alice;
```

Now, the example implements `Foo` for `MyStruct`, which we will call `Alice`, as:

```rust
impl Foo for Alice {
    fn bar(&self) {
        // implementation goes here
    }

    fn baz(&self) {
        // let's not worry about implementing baz() for now
        unimplemented!();
    }
}
```

But we can do better! With this RFC, it is now possible to omit `baz` entirely.

```rust
impl Foo for Alice {
    #![unfinished]

    fn bar(&self) {}
}
```

or equivalently:

```rust
#[unfinished] impl Foo for Alice {
    fn bar(&self) {}
}
```

If we didn't want to implement `bar` either, we could have instead written:

```rust
#[unfinished] impl Foo for Alice {}
```

or:

```rust
impl Foo for Alice { #![unfinished] }
```

Let's now add a few more traits and implement them.

```rust
trait Plugh {
    fn wibble(&self);
    fn wobble(&self);
    fn wubble(&self);
}

trait Philosopher {
    fn eats(&mut self);
    fn thinks(&mut self);
}

trait Physicist {
    fn measures(&mut self);
}

#[unfinished] impl Plugh for Alice {}
#[unfinished] impl Philosopher for Alice {}
#[unfinished] impl Physicist for Alice {}
```

The attribute `#[unfinished]` also allows you to move from everything being
unimplemented to gradually more and more parts being implemented.

Using `#[unfinished]` is useful when we've already defined `Alice` in another
module. But this is in the same module, so we can do better still with:

```rust
#[derive_unfinished(Foo, Plugh, Philosopher, Physicist)]
struct Alice;
```

Some developers using Haskell may be familiar with this feature as the
`DeriveAnyClass` extension, which let's you do:

```haskell
{-# LANGUAGE DeriveAnyClass #-}

class Foo a where
    bar :: a -> IO ()
    baz :: a -> IO ()

data Alice = Alice
    deriving Foo
```

The `deriving Foo` clause here essentially expands into:

```haskell
instance Foo Alice where
    bar = undefined
    baz = undefined
```

This RFC is indeed heavily inspired by this Haskell extension, but with the
important difference that this RFC is purely for prototyping purposes while
Haskell uses the extension to derive traits where no items are left to be
specified for production code.

When typechecking before implementing, it is important that you're allowed to
be brief. As shown above, this ability is greatly enhanced by the RFC.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

[`Send`]: https://doc.rust-lang.org/nightly/std/marker/trait.Send.html
[`Sized`]: https://doc.rust-lang.org/nightly/std/marker/trait.Sized.html
[`Sync`]: https://doc.rust-lang.org/nightly/std/marker/trait.Sync.html
[`Unsize`]: https://doc.rust-lang.org/nightly/std/marker/trait.Unsize.html

An attribute `#[derive_unfinished(Trait1, Trait2, ...)]` is added to the
language. You can use this attribute on any `type` (`struct`/`enum`/`union`/..)
to derive any trait with the exception of auto traits such as
[`Send`], [`Sized`], [`Sync`], [`Unsize`].

As previously discussed in the [summary] as well as the [motivation], this is
equivalent to sprinkling a call to `unimplemented!()` in every method of the
trait as well as using [`ConstDefault`] for associated constants and given
defaults, if any, for associated types.

Let's now go through examples of these one by one.

## Implement gradually

We have already seen some basic examples used in [motivation]. Let's revisit
the trait `Plugh` again.

```rust
trait Plugh {
    fn wibble(&self);
    fn wobble(&self);
    fn wubble(&self);
}
```

Now you want add a new type called `Alonzo` and implement `Plugh` for it.

```rust
#[derive_unfinished(Plugh)]
struct Alonzo;
```

Some time passes, and now you are ready to implement `wibble`.

```rust
#[unfinished] impl Plugh for Alonzo {
    fn wibble(&self) {
        println!("Alonzo wibbles");
    }
}
```

But hmm... It's time for a snack. You return a while later and fix `wobble`.

```rust
#[unfinished] impl Plugh for Alonzo {
    fn wibble(&self) {
        println!("Alonzo wibbles");
    }

    fn wobble(&self) {
        println!("Alonzo wobbles");
    }
}
```

Finally, a while later, you are ready to define `wubble`:

```rust
#[unfinished] impl Plugh for Alonzo {
    fn wibble(&self) {
        println!("Alonzo wibbles");
    }

    fn wobble(&self) {
        println!("Alonzo wobbles");
    }

    fn wubble(&self) {
        println!("Alonzo wubbles");
    }
}
```

## An unnecessary `#[unfinished]`

You build your project with these changes. But the compiler isn't very happy
about it and greets you with the following warning:

```
WARNING: all trait items of `Plugh` for `Alice` are defined; remove `#[unfinished]`.
```

This is a gentle reminder from the compiler that you are finished with the
implementation of your trait since there are no more items the trait requires
you to define.

You may silence this warning with `#[allow(finished_unfinished)]`.

Likewise, if you attempt to derive a marker trait such as:

```rust
pub trait Copy: Clone { }
```

by doing:

```rust
#[derive_unfinished(Copy)]
struct Alice;
```

this will desugar into:

```rust
#[unfinished] impl Copy for Alice {}
```

All trait items (zero of zero) are already defined in this example, so the
compiler will emit a warning since you could just have written `#[derive(Copy)]`.
This warning looks like:

```
WARNING: all trait items of `Copy` for Alice are defined; The `Copy` trait is
a derivable marker trait. Change `#[derive_unfinished(Copy)]` into `#[derive(Copy)]`.
```

With a non-derivable marker trait as in the following example:

```rust
trait MyMarker {}

#[derive_unfinished(MyMarker)]
struct Alice;
```

you will instead get the following warning:

```
WARNING: all trait items of `MyMarker` for Alice are defined;
The `Copy` trait is a marker trait with no items.
Replace `#[derive_unfinished(MyMarker)]` with: `impl MyMarker for Alice {}`.
```

This warning can be silenced with: 

```rust
#[allow(finished_unfinished)]
#[derive_unfinished(MyMarker)]
struct Alice;
```

While this RFC allows you to silence these warnings globally for all items in
your module, you should only do this after careful consideration.

## Dealing with generics
[dealing with generics]: #dealing-with-generics

So far we've only dealt with types and traits without any generics. Now it's
time to see how to deal with types and traits with type parameters.

### In the type

Let's first look at a generic type `Two<T>` defined as.

```rust
trait Command { fn run(&self); }

#[derive_unfinished(Command)]
struct Two<T> {
    first: T,
    second: T,
}
```

Here `#[derive_unfinished(Command)]` expands to:

```rust
#[unfinished] impl<T> Command for Two<T> {}
```

You might be surprised that no `T: Command` bound is inserted as a real
implementation might look like:

```rust
impl<T: Command> Command for Two<T> {
    fn run(&self) {
        self.first.run();
        self.second.run();
    }
}
```

But since this desugars into:

```rust
impl<T> Command for Two<T> {
    fn run(&self) {
        unimplemented!()
    }
}
```

We see that no bound is necessary on `T`.

The compiler cound insert the bound nonetheless, but since it's impossible in
general to tell which type parameters of the type should have the bound and
which shouldn't, the most broad `impl` is generated to aid in prototyping.

If you want to be more conservative than the `#[unfinished] impl` produced by
deriving, you can always write in the relevant bounds you require.


However, if you write:

```rust
#[derive_unfinished(Command)]
struct Two<T: Command> {
    first: T,
    second: T,
}
```

[RFC 2089, Implied bounds]: https://github.com/rust-lang/rfcs/pull/2089

The compiler has no choice but to add the bound either explicitly or implicitly
with [RFC 2089, Implied bounds].

You are not limited to one type parameter on the type. Add as many type
parameters as you wish to your heart's content:

```rust
#[derive_unfinished(Command)]
struct H4<A, B, C, D>(A, B, C, D);

// first line expands to:

#[unfinished] impl<A, B, C, D> Command for H4<A, B, C, D> {}
```

The general rule here is that any type parameters of the type are added to the
`impl<..>` bit as well as to the type in the right-hand side (RHS) of `for`.

### In the trait

Let's say we have a velocity vector:

```rust
mod units {
    pub struct MetresPerSecond(f64);
}

struct Velocity {
    speed: MetresPerSecond,
    angle: f64,
}
```

Now we would like to provide `From<f64>` which gives a vector in the forward
direction and the speed given by the `f64` in the trait. We can do this with:

```rust
#[derive_unfinished(From<f64>)]
struct Velocity {
    // ..
}
```

As you may expect, this generates:

```rust
#[unfinished] impl From<f64> for Velocity {}
```

You may also be explicit about where the trait and any concrete types applied
to the type parameters are:

```rust
#[derive_unfinished(std::convert::From<units::MetresPerSecond>)]
struct Velocity {
    // ..
}
```

The first line generates:

```rust
#[unfinished] impl std::convert::From<units::MetresPerSecond> for Velocity {}
```

Let's now consider a trait with a type parameter and a type without one.
We would like to give the trait a generic parameter (not a concrete type) but
without giving it to the type. By applying `_` to the trait, this is possible:

```rust
use std::iter::FromIterator;

#[derive_unfinished(FromIterator<_>)]
struct Sink;
```

The second line will expand to:

```rust
#[unfinished] impl<F0> FromIterator<F0> for Sink {}
```

which in turn expands to:

```rust
impl<F0> FromIterator<F0> for Sink {
    fn from_iter<T>(iter: T) -> Self where T: IntoIterator<Item = F0> {
        unimplemented!()
    }
}
```

Where did `F0` come from? When you use `_`, the compiler will insert a new
generated generic type (free type variable) which does not conflict with any
other type names. Here, the compiler chose `F0` for you. You can use `_` as many
times as you like to get new free type variables. They can also be mixed with
concrete types as in:

```rust
#[derive_unfinished(Foo<usize, _, bool, _>)]
struct MyType;
```

In this case, the following is generated for the first line:

```rust
#[unfinished] impl<F0, F1> Foo<usize, F0, bool, F1> for MyType {}
```

### In relation to const generics

[RFC 2000, const_generics]: https://github.com/rust-lang/rfcs/blob/master/text/2000-const-generics.md

With [RFC 2000, const_generics] the `impl` can contain `const` variables as in:

```rust
trait Foo { fn bar(&self); }

struct Matrix<T, const M: usize, const N: usize> {
    array: [[T; M]; N]
}

impl<T, const M: usize, const N: usize> Foo for Matrix<T, M, N> {
    fn foo(&self) {
        unimplemented!()
    }
}
```

When implementing `Foo for Matrix<..>` we can do:

```rust
#[derive_unfinished(Foo)]
struct Matrix<T, const M: usize, const N: usize> {
    array: [[T; M]; N],
}
```

This generates the `impl` above.

Once the language also allows `const` generics in traits, you will be able to do:

```rust
trait Foo<const N: usize> { fn bar(&self); }

#[derive_unfinished(Foo<42>)]
struct Alice;

#[derive_unfinished(Foo<N>)]
struct Bob<const N: usize> {
    skills: [Skill; N],
}

#[derive_unfinished(Foo<_>)]
struct Ada<const N: usize> {
    skills: [Skill; N],
}
```

These attributes will generate:

```rust
impl Foo<42> for Alice {
    fn foo(&self) { unimplemented!() }
}

impl<const N: usize> Foo<N> for Bob<N> {
    fn foo(&self) { unimplemented!() }
}

impl<const N: usize, const F0: usize> Foo<F0> for Ada<N> {
    fn foo(&self) { unimplemented!() }
}
```

### In the type and the trait

Consider an `impl` where both the trait and type is generic.
Let's consider the `Option` type and the `FromIterator` traits.

```rust
#[derive_unfinished(FromIterator<T>)]
enum Option<T> {
    None,
    Some(T),
}
```

In this example, the type variable `T` in `FromIterator` refers to the same
type variable in `Option`. Therefore the first line expands into:

```rust
#[unfinished] impl<T> FromIterator<T> for Option<T> {}
```

which in turn expands to:

```rust
impl<T> FromIterator<T> for Option<T> {
    fn from_iter<I>(iter: I) -> Self where I: IntoIterator<Item = T> {
        unimplemented!()
    }
}
```

At this point, it is important to note that `from_iter` brought a new type
variable `T` into scope originally, which the `impl` also did. Had the compiler
not handled this case properly, the result would have been:

```rust
impl<T> FromIterator<T> for Option<T> {
    fn from_iter<T>(iter: T) -> Self where T: IntoIterator<Item = T> {
        unimplemented!()
    }
}
```

Here, the only way to solve `T: IntoIterator<Item = T>` is to pick an iterator
which returns itself as the item. To resolve this conflict, the compiler, in
desugaring the`#[unfinished]` attribute, had to rename the variable `T` that
`from_iter` introduces to a fresh name, in this case `I`.

As a final note on generics, you are of course free to mix the all of the
features and concepts enumerated in this section. The following is a contrived
example of this:

```rust
trait Foo<D, E, F> {
    fn bar(d: D, e: E) -> F;
}

mod path_to {
    pub struct Unit;
}

#[derive_unfinished(Foo<A, path_to::Unit, _>)]
union Baz<A, B, C> {
    field: PhantomData<(A, B, C)>,
}
```

Here, `derive_unfinished` desugars into:

```rust
#[unfinished] impl<A, B, C, F0> Foo<A, path_to::Unit, F0> for Baz<A, B, C> {}
```

### Lifetimes for types and traits

Deriving also works fine with lifetimes, both in the type and the trait.

Let's consider three short examples.

#### When the derived-for type has a lifetime

```rust
trait Foo { fn bar(&self); }

#[derive_unfinished(Foo)]
struct A<'a>(&'a str);
```

The attribute desugars into:

```rust
#[unfinished] impl<'a> Foo for A<'a> {}
```

#### When the trait and type has a lifetime

```rust
trait Foo<'a> { fn bar(&'a self); }

#[derive_unfinished(Foo<'a>)] // Refers to 'a in the type.
struct A<'a>(&'a str);
```

The attribute desugars into:

```rust
#[unfinished] impl<'a> Foo<'a> for A<'a> {}
```

#### When only the trait has a lifetime

```rust
trait Foo<'a> { fn bar(&'a self); }

#[derive_unfinished(Foo<'_>)] // '_ generates a fresh generic lifetime.
struct A;
```

The attribute desugars into:

```rust
#[unfinished] impl<'l0> Foo<'l0> for A {}
```

## With associated types

Let's now say that our trait has an associated type, how can we simplify this
case? Consider the example of an `Iterator`:

```rust
pub trait Iterator {
    type Item;

    fn next(&mut self) -> Option<Self::Item>;
}
```

There's no way for the compiler to know what type to pick for `Iterator::Item`,
therefore, `#[derive_unfinished(Iterator)]` can't be used. Instead, you have to
write the following:

```rust
#[unfinished] impl<A> Iterator for Two<A> {
    type Item = A;

    // fn next(..) is still added for you.
}
```

which in turn expands into:

```rust
impl<A> Iterator for Two<A> {
    type Item = A;

    fn next(&mut self) -> Option<Self::Item> {
        unimplemented!()
    }
}
```

If you try to write:

```rust
#[derive_unfinished(Iterator)]
struct Two<T> {
    first: T,
    second: T,
}
```

the compiler will greet you with an error message:

```
ERROR: could not derive `Iterator` for `Two`. The associated type `Iterator::Item`

    pub trait Iterator {
        type Item;
            // ^ Item does not have a default type.

        // ...
    
    }

was not defined with a default type which can be used when deriving `Iterator`.
```

### Specifying the associated type

Finally, we can also specify the associated type in the attribute directly:

```rust
#[derive_unfinished(Iterator<Item = T>)]
struct Two<T> {
    first: T,
    second: T,
}
```

The rule for what can be used on the RHS of the equality is the same as those
enumerated in [dealing with generics] except for using `_`.

So, this is valid:

```rust
#[derive_unfinished(Iterator<Item = u8>)]
struct Two(u8, u8);
```

But not this:

```rust
#[derive_unfinished(Iterator<Item = _>)]
struct Empty;
```

Why? Because in the latter case, the attribute would expand into:

```rust
impl<F0> Iterator for Empty {
    type Item = F0;

    fn next(&mut self) -> Option<Self::Item> {
        unimplemented!()
    }
}
```

which the compiler today greets with:

```
error[E0207]: the type parameter `F0` is not constrained by the impl trait, self type, or predicates
 --> src/main.rs:<line>:<column>
  |
4 | impl<F0> Iterator for Empty {
  |      ^^ unconstrained type parameter
```

## With `#![feature(associated_type_defaults)]`

If trait has a default for an associated type, as in the following example, then
that type need not be mentioned.

```rust
#![feature(associated_type_defaults)]

trait WithOneDefault {
    type First = u32;
    type Second;

    fn foo(&self);
}
```

We have to use `#[unfinished]`:

```rust
#[unfinished] impl WithOneDefault for Alice {
    type Second = bool;

    // rest is added for us...
}
```

which expands into:

```rust
impl WithOneDefault for Alice {
    type First = u32;
    type Second = bool;

    fn foo(&self) { unimplemented!(); }
}
```

If all associated types have defaults, then `#[derive_unfinished(Trait)]`
can be used:

```rust
#![feature(associated_type_defaults)]

trait WithAllDefault {
    type First = u32;
    type Second = bool;

    fn bar(&self);
}

#[derive_unfinished(WithAllDefault)]
struct Bob;
```

where the attribute expands into:

```rust
#[unfinished] impl WithOneDefault for Bob {}
```

which in turn expands into:

```rust
impl WithOneDefault for Alice {
    type First = u32;
    type Second = bool;

    fn bar(&self) { unimplemented!(); }
}
```

## With associated constants

Let's say our trait has an associated constant as in:

```rust
enum SumType {
    A, B, C
}

trait WithConst {
    const ID: SumType;

    fn baz(&self);
}
```

Since there's no way for the compiler to tell which variant of `Sum` to use,
you can't write:

```rust
#[derive_unfinished(WithConst)]
enum Colors { Red, Green, Blue }
```

If you would try to do this, the compiler would greet you with the error:

```
Could not derive WithConst for Colors. The associated constant WithConst::ID
does not have a default value which can be used.
```

Instead, you must write:

```rust
#[unfinished] impl WithConst for Colors {
    const ID: SumType = SumType::A;

    // The rest is added for you...
}
```

which expands to:

```rust
impl WithConst for Colors {
    const ID: SumType = SumType::A;

    fn baz(&self) { unimplemented!(); }
}
```

### Specifying the associated constant

As with associated types, you may also specify the associated constant directly
as done in the following example:

```rust
#[derive_unfinished(WithConst<ID = SumType::A>)]
enum Colors { Red, Green, Blue }
```

### With defaults

Any associated constants with defaults that the trait has can however be omitted.
If all associated constants have defaults, then you can use `derive_unfinished`:

```rust
trait WithConst {
    const ID: SumType = SumType::A;

    fn baz(&self) { unimplemented!(); }
}

#[derive_unfinished(WithConst)]
enum Colors { Red, Green, Blue }
```

where the attribute expands to:

```rust
#[unfinished] impl WithConst for Colors {}
```

which in turn expands to:

```rust
#[unfinished] impl WithConst for Colors {
    const ID: SumType = SumType::A;

    fn baz(&self) { unimplemented!(); }
}
```

### With `ConstDefault`

As an optional feature of this RFC in conjunction with the [`ConstDefault`] RFC,
iff the type of the associated constant is `ConstDefault` as `usize` would with:

```rust
impl ConstDefault for usize {
    const DEFAULT: Self = 0;
}
```

then the compiler will use `<AssociatedConstant as ConstDefault>::DEFAULT`
letting you write:

```rust
trait WithConstDefault {
    const ID: usize;

    fn quux(&self) { unimplemented!(); }
}

#[derive_unfinished(WithConstDefault)]
struct Ada;
```

where the attribute expands to:

```rust
#[unfinished] impl WithConstDefault for Ada {}
```

which further expands to:

```rust
#[unfinished] impl WithConstDefault for Ada {
    const ID: usize = usize::DEFAULT;

    fn quux(&self) { unimplemented!(); }
}
```

With the [`ConstDefault`] proposal, the error message above would be:

```
Could not derive WithConst for Colors. The associated constant WithConst::ID
does not have a default value which can be used. The constant WithConst::ID
is of type usize, which does not implement ConstDefault.
```

## When can `#[derive_unfinished(Trait)]` be used?

The exact semantics are that:

If you can write:

```rust
#[unfinished] impl Trait for MyType {}
```

then you may write:

```rust
#[derive_unfinished(Trait)]
<Type declaration>
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

In this section, we will, bit by bit, discuss how the features discussed
previously may be achieved technically.

## `#[derive_unfinished(..)]`

The attribute `#[derive_unfinished(Trait..)]` can be used on `struct`, `enum`,
and `union` type definitions to quickly prototype an `impl` of the `Trait` for
the type it is used on.

In this context, prototyping refers to the concept of
"typecheck first, implement later". This is achieved with the attribute by
generating `impl`s which uses `unimplemented!()` on all non-default methods.
For associated types, any defaults are also used. The user can also specify
the associated type in the `derive_unfinished` attribute as well as associated
consts with a limited form of constant expressions. The `derive_unfinished`
attribute is sugar for `#[unfinished] impl ..` which is explained later.

### Grammar of the attribute

Before going deeper into these details, the grammar, given in a dialect of EBNF,
for the attribute is:

```
derive_unfinished : "derive_unfinished" "(" derive_list ")" ;

derive_list : derive_list_item [ "," derive_list_item ]* ","? ;
derive_list_item : derived_trait [ "<" trait_arguments? ">" ]? ;

derived_trait : "::"? ident ["::" ident]* ;

trait_arguments : trait_arguments_item [ "," trait_arguments_item ]* ","? ;

trait_arguments_item : specified_associated_item
                     | type_expr
                     ;

specified_associated_item : ident "=" type_expr_assoc ;

type_expr_with_const : type_expr
                     | const_expr_limited
                     ;

type_expr : fresh_lifetime
          | explicit_lifetime
          | fresh_type
          | unit_type
          | array_type
          | tuple_type
          | ref_type
          | type_path
          ;

fresh_lifetime : "'_" ;
explicit_lifetime : "'" ident ;

fresh_type : "_" ;

ref_type : "&" lifetime "mut" type_expr_assoc
         | "&" lifetime type_expr
         | "*" "const" type_expr
         | "*" "mut" type_expr
         ;

array_type : "[" type_expr ";" const_expr "]" ;

tuple_type : "(" tuple_type_list ")" ;
tuple_type_list : type_expr ","
                | type_expr [ "," type_expr ]+ ","?
                ;

type_path : "::"? ident [ type_path_tail ] + ;
type_path_tail : '<' type_expr_with_const [ ',' type_expr_with_const ] + '>'
               | "::" type_path_tail
               ;

type_expr_assoc : explicit_lifetime
                | unit_type
                | array_type_assoc
                | tuple_type_assoc
                | ref_type_assoc
                | type_path_assoc
                | const_expr_limited
                ;

ref_type_assoc : "&" lifetime "mut" type_expr_assoc
               | "&" lifetime type_expr_assoc
               | "*" "const" type_expr_assoc
               | "*" "mut" type_expr_assoc
               ;

array_type_assoc : "[" type_expr_assoc ";" const_size_expr "]" ;
tuple_type_assoc : "(" tuple_type_assoc_list ")" ;
tuple_type_assoc_list : type_expr_assoc ","
                      | type_expr_assoc [ "," type_expr_assoc ]+ ","?
                      ;

type_path_assoc : "::"? ident [ type_path_assoc_tail ] + ;
type_path_assoc_tail : '<' type_expr_assoc [ ',' type_expr_assoc ] + '>'
                     | "::" type_path_assoc_tail
                     ;

const_expr_limited : literal
                   | unit_expr
                   | tuple_expr_limited
                   | array_expr_limited
                   | expr_path_limited
                   ;

const_size_expr : literal ;
unit_expr : "()" ;

array_expr_limited : "[" const_expr_limited ";" const_size-expr "]" ;

tuple_expr_limited : "(" tuple_expr_limited_list ")" ;
tuple_expr_limited_list : const_expr_limited ","
                        | const_expr_limited [ "," const_expr_limited ]+ ","?
                        ;

tuple_struct_init_limited : expr_path_limited apply_expr_limited? ;

apply_expr_limited : "(" apply_expr_limited_list ")" ;
apply_expr_limited_list : const_expr_limited [ "," const_expr_limited ]* ","? ;

expr_path_limited : "::"? ident ["::" ident]* ;
```

Once const generics are allowed in a `trait` definition, `type_expr` will be
changed into:

```
type_expr : fresh_lifetime
          | explicit_lifetime
          | fresh_type
          | unit_type
          | array_type
          | tuple_type
          | ref_type
          | type_path
          | const_expr_limited
          ;
```

If `const_expr_limited` is too allowing as `pos` in
`impl<..> Trait<pos> for Type`, it will be further limited to fit the RFC
which allows const generics in `pos`. This does not apply to `type_expr_assoc`,
which is only used in associated item position, since associated consts are
allowed in Rust today.

### Lowering to `#[unfinished]`

After parsing `#[derive_unfinished(..)`, further transformations and analysis
must be done once all traits have been resolved. At this point, hence referred
to as `traits_resolved`, both the AST of the type definition and the trait in
question are available, which is required to proceed further.

#### Deriving derivable marker traits

Before `traits_resolved`, the compiler will check to see if `derived_trait`
resolves to one of the derivable marker traits which currently consists of:
+ `Copy`

If it does, and unless `#[allow(finished_unfinished)]` is in effect either on
the type, module, or the crate, the following warning is raised:

```
WARNING: all trait items of $TRAIT for $TYPE are defined; The $TRAIT trait is
a derivable marker trait. Change $ATTRIBUTE into `#[derive($TRAIT)]`.
```

In this warning, the meta variables are:
+ `$TRAIT`, the trait being derived which is the same as `derived_trait`.
+ `$ATTRIBUTE`, the attribute declaration in its entirety.
+ `$TYPE`, the name of the type the attribute is used on.

The note regarding `#[allow(finished_unfinished)]` also applies to all raisable
warnings to be enumerated. A further note is that the compiler may delay analysis
for these warnings until after generation and unify them into a more common
mechanism, as long as the semantics of the warnings stay the same.

#### Deriving marker traits

After `traits_resolved`, the compiler checks first if `$TRAIT` resolves
to a trait with zero items such as `trait MyMarker {}`, if so, the following
warning is raised:

```
WARNING: all trait items of $TRAIT for $TYPE are defined;
The $TRAIT trait is a marker trait with no items.
Replace $ATTRIBUTE with: $REPLACEMENT_IMPL.
```

With added meta variable:
+ `$REPLACEMENT_IMPL`, the `impl` that `$ATTRIBUTE` desugars to, minus the
`#[unfinished]` modifier. For example: `impl MyMarker for Alice {}`.

#### Deriving traits with no items to finish

If the trait had some items, but all were one of:

+ associated type with default
+ associated const with default
+ associated const where `typeof(ITEM): ConstDefault`
+ specified in the attribute via `Item = <type_expr_assoc>`

Then the following warning was raised:

```
WARNING: all trait items of $TRAIT for $TYPE are defined.
Replace $ATTRIBUTE with $REPLACEMENT_IMPL
```

This step can be deferred to when `#[unfinished]` is dealt with, an exact
mechanism for the equivalent warning is given for that attribute.

#### Generation

Now, the compiler enters a phase hence referred to as `lower_derive_unfinished`
the goal of which is to lower `#[derive_unfinished(..)]` into an `impl` which is
`#[unfinished]`. To accomplish this, for each `derive_list_item`, the compiler:

1. Determines generic type parameters
    1. Accumulates the generic types `$TYPE_PARAMS` of `$TYPE` into `$PARAMS`.
    2. Recursively accumulates all simple `type_path`s (those which lack `::`) into
    `$SIMPLE_TYPE_PATHS`.
    3. Joins `$PARAMS` together with `$SIMPLE_TYPE_PATHS` into a set `$USED_NAMES`.
    4. Finds an `ident` prefix `$FRESH_TYPE_PREFIX` that is not used in `$USED_NAMES`.
    5. Starts a counter `FT` at zero.
    6. Recursively finds any usage of `fresh_type` in `type_expr` and:
        1. Reads `FT` and adds it as a suffix of `$FRESH_TYPE_PREFIX`. This new type
        variable is called `$FRESH_TYPE_VAR`.
        2. Adds `$FRESH_TYPE_VAR` to `$PARAMS`.
        3. Replaces the current `fresh_type` with `$FRESH_TYPE_VAR`.
        4. Increments `FT` by one.
2. Determines generic lifetime parameters
    1. Accumulates the generic lifetimes `$TYPE_LIFETIMES` of `$TYPE` into `$LIFETIMES`.
    2. Joins `$LIFETIMES` + `{'static}` into a set `$USED_LIFETIMES`.
    3. Finds an `ident` prefix `$FRESH_LIFETIME_PREFIX` that is not used in `$USED_LIFETIMES`.
    4. Starts a counter `FL` at zero.
    5. Recursively finds any usage of `fresh_lifetime` in `type_expr` and:
        1. Reads `FL` and adds it as a suffix of `$FRESH_LIFETIME_PREFIX`.
        This new lifetime is called `$FRESH_LIFETIME`.
        2. Adds `$FRESH_LIFETIME` to `$LIFETIMES`.
        3. Replaces the current `fresh_lifetime` with `$FRESH_LIFETIME`.
        4. Increments `FT` by one.
3. Saves the modified (with respect to fresh type variables and lifetimes)
`derive_list_item` as `$IMPL_TRAIT`.
4. Concatenates `$LIFETIMES` and `$PARAMS` into `$VARS_IMPL`
5. Concatenates `$TYPE_LIFETIMES` and `$TYPE_PARAMS` into `$VARS_TYPE`
6. Accumulates all `specified_associated_item` in `trait_arguments` as:
`$SPECIFIED_ASSOCIATED_ITEMS`. If an associated item is repeated more than once
an error is raised.
7. Adds to the module:
```rust
#[unfinished] impl<$VARS_IMPL> $IMPL_TRAIT for $TYPE<VARS_TYPE> {
    $SPECIFIED_ASSOCIATED_ITEMS
}
```

These steps are executed in order, but the compiler is free to reorder as long
as the end result is semantically the same.

The generation of the `#[unfinished] impl` is now done.

## `#![unfinished]`

The attribute `#![unfinished]` can be placed before all other items inside a
trait `impl` as in:

```rust
impl<$PARAMS> $TRAIT_WITH_PARAMS for $TYPE_WITH_PARAM {
    #![unfinished]

    $OTHER_STUFF
}
```

After `lower_derive_unfinished` has executed this is translated into
`#[unfinished] impl` as in:

```rust
#[unfinished] impl<$PARAMS> $TRAIT_WITH_PARAMS for $TYPE_WITH_PARAM {
    $OTHER_STUFF
}
```

## `#[unfinished]`

### High level description

The attribute `#[unfinished]` can be placed on a trait `impl` as in:

```rust
#[unfinished] impl<$PARAMS> $TRAIT_WITH_PARAMS for $TYPE_WITH_PARAM {
    $OTHER_STUFF
}
```

In `$OTHER_STUFF` the developer may specify only those items they wish to
implement at the present time or none at all. Two the "none at all" rule there
are two exceptions:

+ Associated types with no default type must always be specified.
+ Associated `const`s which have both no default value and where `typeof(item)`
is not `ConstDefault` must always be specified.

Those associated items not specified in the `impl` get assigned as follows:
+ For associated types the default for the trait is used.
+ For associated `const`s the default for the trait is first used if any, 
if none exists the `ConstDefault` implementation of the `const`s type is used.

For `fn` items in the `impl`, the methods are copied from the trait and the
semantic equivalent of `unimplemented!()` is used.

### Before generation: Dealing with `ConstDefault`

Prior to desugaring of `#[unfinished]` the compiler will accumulate all `impl`s
of the `ConstDefault` trait. The `impl` itself need not be typechecked at this
point - the knowledge that the `impl` exists is sufficient. Accumulating all
`ConstDefault` `impl`s is done before such that the compiler may check if a
certain type is `ConstDefault` or not when desugaring `#[unfinished]`.

Since `ConstDefault` is used to determine a value for associated `const`s in
some cases, the compiler will not allow any of the following:

+ `#[unfinished] impl<..> ConstDefault for $TYPE {}`
+ `impl<..> ConstDefault for $TYPE { #![unfinished] }`
+ `#[derive_unfinished(ConstDefault)] $TYPE`

If the compiler did allow this, a cycle would arise.

If the user does attempt to `#[derive_unfinished(ConstDefault)]` then the
following error will be raised:

```
ERROR: `#[derive_unfinished(ConstDefault)]` is not possible as the trait
`ConstDefault` is not derivable for any type including `$TYPE`.
```

If `#[unfinished]` or `#![unfinished]` is instead used, the following error
will be raised:

```
ERROR: `$ATTRIBUTE` is not allowed on `$IMPL`.
An `impl` of the trait `ConstDefault` may never be `$ATTRIBUTE`;
```

where `$ATTRIBUTE` is either `#[unfinished]` or `#![unfinished]`.

### Before generation: Warnings for marker traits

If a derivable marker trait as specified previously is the trait for which
`#[unfinished] impl` is being done, then a warning is raised as follows:

```
WARNING: all trait items of $TRAIT for $TYPE are defined; The $TRAIT trait is
a derivable marker trait. Change $IMPL into `#[derive($TRAIT)]`.
```

### Generation

#### 1. Dealing with associated types

Associated types are treated as is done with normal `impl`s. The compiler looks
at the `#[unfinished] impl` and the trait. If an associated type is specified
that does not exist, the normal error is produced . If an associated type without
a default is not specified, the error:

```
error[E0046]: not all trait items implemented, missing: `$ASSOCIATED_ITEM`
```

is raised.

#### 2. Dealing with associated `const`s

The compiler looks at the `#[unfinished] impl` and the trait. Foreach associated
`const` of the trait, the following cases are matched in order:

1. If it is explicitly specified in the `impl`, then that is used.
2. If it is not explicitly specified and there is a default, then that default
is used.
3. If it is not explicitly specified and no default exists, then
`<$TYPEOF_ITEM as ConstDefault>::DEFAULT` is used and added to the `impl`.
An example is: `<usize as ConstDefault>::DEFAULT`.
4. Otherwise, the following error is raised:

```
ERROR: the associated constant `$ITEM` can not be left unspecified as the trait
neither provides a default value for the item, nor does the type of the item,
`$TYPEOF_ITEM`, implement the trait `ConstDefault`.

Suggestions:
+ Modify the trait by adding a default to for the constant.
+ Implement `ConstDefault` for the type of the constant.
+ Specify the value of the constant explicitly.
```

If the situation arises from `#[derive_unfinished(..)]` then the following error
is raised instead:

```
ERROR: Can not `#[derive_unfinished($TRAIT)]` for `$TYPE`. The associated
constant `$ITEM` can not be left unspecified) as the trait neither provides a
default value for the item, nor does the type of the item, `$TYPEOF_ITEM`,
implement the trait `ConstDefault`.

Suggestions:
+ Modify the trait by adding a default to for the constant.
+ Implement `ConstDefault` for the type of the constant.
+ Specify the value of the constant explicitly with
    `#[derive_unfinished($TRAIT<$ITEM = the value>)]`.
```

#### 3. Dealing with functions

The compiler starts with setting `$FN_COUNT = 0`.
For all other function in the trait, the compiler, for each `fn`:

1. (There are three cases matched top down)
    1. If the function was explicitly given by the user, then nothing happens
    here and the function is typechecked according to normal rules.
    2. If the function has a default implementation in the trait, then that is
    copied.
    3. If the no default exists, the function is copied and given a body which
    is semantically equivalent to `unimplemented!()`. Here, `$FN_COUNT` is
    incremented.

    For the two latter cases, all type variables introduced by the function are
    alpha converted with fresh names that do not conflict with those type
    variables introduced by the `impl`.

2. Adds the modified function to the `impl`.

After going through all functions, if `$FN_COUNT == 0` then the following
warning is raised:

```
WARNING: all trait items of `$TRAIT` for `$TYPE` are defined; remove `#[unfinished]`.
```

The desugaring is now done.

## Restriction for `impl Trait`

Consider the following case:

```rust
#![feature(conservative_impl_trait, never_type)]

trait Foo {}

fn bar() -> impl Foo {
    unimplemented!()
}
```

If this is attempted today, the compiler responds with:

```
error[E0277]: the trait bound `!: Foo` is not satisfied
 --> src/main.rs:5:13
  |
6 | fn bar() -> impl Foo {
  |             ^^^^^^^^ the trait `Foo` is not implemented for `!`
```

This is the case because currently, `!` (the `never_type`) does not implement
all traits automatically. While this is true, and since `unimplemented!()` will
yield a panic of type `!`, once it is possible for a trait to have existential
types in return position of an `fn` either directly or via an associated type,
then neither `#[unfinished]` nor `#[derive_unfinished(..)` will work for such a
trait and will therefore not be allowed. Existentials in argument position are
however not a problem as `unimplemented!()` only affects the return value.

The user may work around this problem by declaring that:

```rust
impl Foo for ! {}
```

This assumes that this `impl` follows the coherence rules. Some traits in the
standard library already have such an `impl`.

# Drawbacks
[drawbacks]: #drawbacks

This can be considered quite a large addition to the language. It could also be
argued that `unimplemented!()` is sufficient for prototyping purposes, but as
the [motivation] has shown, a lot less typing can be had with the changes
proposed.

# Rationale and alternatives
[alternatives]: #alternatives

One of the impacts of not doing this is that the ergonomics of prototyping via
"typecheck first, implement later" is not enhanced.

One of the alternatives considered to the `#[derive_unfinished(Trait)]`
attribute was to modify the `#[derive(..)]` attribute with the syntax:
`#[derive(Clone, Display => !)]` where `Clone` would be derived as normal and 
`Display` with the semantics of this RFC. A different syntax for this also
considered was: `#[derive(Clone, unfinished Display)]`. However, these
alternative syntaxes have worse greppablity compared to `#[debug_unimpl(..)]`.
The new attribute is comparatively much more obvious, which helps the user to
notice that that particular piece of code is unfinished.

Another alterative to the changes proposed here is to do just `#[unfinished]` or
just `#[derive_unfinished(..)]` or not both. However, this RFC argues that
there's a smooth transition between the latter to the former which creates a
nice stepping-stone effect.

An alternative to adding any of the functionality proposed in this RFC is to
let RLS, in a joint venture with your favourite text editor / IDE, generate a
skeleton trait `impl` for the user and paste it into their code. However, one
can not assume that RLS is present and that it works with the editor. And even
when it does, generated skeletons will add a lot of distracting clutter that the
user could be without "right now". For developers with shorter attention spans,
the method proscribed by this RFC can be better.

# Formerly unresolved questions

+ What should the `#{unfinished]` attribute be called?

Between the alternatives `#[unimpl]` and `#[unimplemented]`, which one is better?
This depends on if your bias and perspective is from `unimplemented!()` or
`impl` and `#[derive_unimpl(..)]`. If your perspective is from the point of view
of `unimplemented!()`, then `#[unimplemented]` is better, while `#[unimpl]` is
better from the other POV. Since `#[derive_unimplemented(Trait)]` is a bridge
too far, and hurts rapid prototyping, that should not be changed. For the
purposes of enhancing rapid prototyping maximally, `#[unimpl]` is better since
it is sufficiently clear while also being shorter.

An alternative to `#[unimpl]` would be to name the attribute `#[partial]`
which reads well with `impl Foo for Bar {}`, as does `#[unfinished]` and
`#[incomplete]`. These are all worthy of consideration since they read
substantially better compared to `#[unimpl] impl ..` and
`#[unimplemented] impl ..`. The names `#[unimpl]` and `#[unimplemented]` do also
not convey semantic intent as well as `#[partial]`, `#[unfinished]`, and
`#[incomplete]` since an "unimplemented implementation" is an oxymoron. Of the
three last names, `#[unfinished]` and `#[incomplete]` convey intent slightly
better since the `impl` is literally unfinished or incomplete.
Since an English speaker to "finish X" instead of "complete X", `#[unfinished]`
is the best choice linguistically. This is also the rationale the RFC uses to
pick `#[unfinished]` as it's current choice of attribute name.

It follows from this choice that `#[derive_unimpl(..)]` should and will be named
`#[derive_unfinished(..)]` instead.

+ An addendum to the previous question is if the form `#![unfinished]`
(note the `!`) should be allowed as in this example:

```rust
impl Foo for Bar {
    #![unfinished]

    // other stuff...
}
```

Using the attribute `#[unfinished]` as above follows the normal rules of
using attributes in the language. To not allow it would be to surprise users.
It is probably also technically cheaper not to have a special case for this
attribute. Futhermore, it may be ergonomic to write an impl, and then decide
that "I don't want to write the rest right now" and then drop in an
`#![unfinished]` at the start. Therefore, this RFC allows the use of
`#![unfinished]` inside an `impl`.

+ Should the user be allowed to write the following?

```rust
#[derive_unfinished(Copy)]
struct Alice;
```

This desugars into:

```rust
#[unfinished] impl Copy for Alice {}
```

which is just:

```rust
impl Copy for Alice {}
```

which is equivalent to:

```rust
#[derive(Copy)]
struct Alice;
```

In general, if the user can write `#[derive(Trait)]`, should they be allowed to
write `#[derive_unfinished(Trait)]`? If yes, should this generate a warning?

Checking whether `#[derive(Trait)]` can be hard to figure out, especially with
custom derived traits. The compiler might need to first compile with
`#[derive(Trait)]` first, then check if an error was generated, and if so, undo
the derive and save that it wasn't possible or that it was, this might lead to
many tricky corner cases and bad compile performance.

It is however probably not even desirable to emit a warning let alone an error.
An example of where it is not is for the trait `Clone`. It may be possible that
while  `#[derive(Clone)]` works, the user might not want the derived `impl` but
rather a custom one. In this case, the user might want to gradually implement
the trait and not give the implementation of `.clone()` immediately. This RFC
takes the view that no error or warning should be emitted.

+ What should the effect of `#[unfinished]` be when all the trait items are
defined as in the example above with `Copy`? Should an error be raised or is a
warning enough?

A possible error message would be:

```
ERROR: all trait items of Copy for Alice are defined; remove `#[unfinished]`.
```

If `#![unfinished]` is allowed and used, the error message would reflect this.

Not emitting an error or warning may be even worse for an example such as:

```rust
#[unfinished] unsafe impl<T> std::iter::TrustedLen for MyContainer<T> {}
```

However, it could be argued that the user is to blame for this since `unsafe`
was used.

A less extreme approach than an error, i.e: a warning, may be better. Since
a hard error may be in the way of rapid prototyping, a warning is more likely
be optimal for productivity as well as tell the user that `#[unfinished]` has
no effect. This is also more in line with what the compiler does in other
contexts when dealing with unnecessary things the user has written. An example
of this is how the compiler deals with unnecessary parenthesis. This rationale
is used as motivation for the current choice: to emit a warning.

# Unresolved questions
[unresolved]: #unresolved-questions

+ The optionality of `ConstDefault` should be resolved.