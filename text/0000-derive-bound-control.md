- Feature Name: derive_bound_control
- Start Date: 2017-02-26
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

This RFC gives users a way to control trait bounds on derived
implementations by allowing them to omit default bounds on type
parameters or add bounds for field types. This is achieved with
the two attributes `#[no_bound(Trait)]` and `#[field_bound(Trait)]`.

The semantics of `#[no_bound(Trait)]` for a type parameter `P` are:
> The type parameter `P` does not need to satisfy `Trait` for any field
> referencing it to be `Trait`

The semantics of `#[field_bound(Trait)]` on a field are that the type
of the field is added to the `where`-clause of the referenced `Trait`
as: `FieldType: Trait`.

# Motivation
[motivation]: #motivation

The deriving mechanism of Rust allows the author to prototype faster and reduce
pain by significantly reducing boilerplate in many cases. Deriving also allows
readers of code to easily see when a bunch of simple delegating `impl`s are
defined instead of reading such boilerplate as manual `impl`s.

Unfortunately, there are many cases where deriving fails to produce the code
indented by manual implementations. Either the `impl`s produced are too
restrictive by imposing bounds that shouldn't be there, which is solved by
`#[no_bound(..)]`, or not enough bounds are imposed. When the latter is the
case, deriving may fail entirely. This is solved by `#[bound(..)]`.

The crate `serde` provides the attribute `#[serde(bound = "T: MyTrait")]`.
This can be used solve the same issues as in this RFC. This RFC proposes a
common mechanism to be used for all derivable traits in the standard library,
as well as in custom derive macros. By doing so, a common language is given
to users who can now use this method regardless of what trait is being derived
in all of the ecosystem.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## Removing bounds in derive with `#[no_bound]`

Let's consider a simple new-type around an `Arc<T>`:

```rust
#[derive(Clone)]
struct MyArc<#[no_bound] T>(Arc<T>);
```

or, to apply `#[no_bound]` to all type parameters, which is in this case
equivalent:

```rust
#[derive(Clone)]
#[no_bound]
struct MyArc<T>(Arc<T>);
```

The resulting `impl` will be of the form:

```rust
// There is no bound T: Clone!
impl<T> Clone for MyArc<T> { /* .. */ }
```

We see that `#[no_bound]` on `T` is an instruction to the derive macro for
`Clone` that it should not add `T: Clone`. This applies to any trait being
derived and not just `Clone`. This works since `Arc<T>: Clone` holds regardless
of whether `T: Clone` or not.

But what if you want to differentiate between the deriving behavior of various
traits? Let's derive another trait, `PartialEq`, but still use `#[no_bound(..)]`:

```rust
#[derive(Clone, PartialEq)]
struct MyArc<#[no_bound(Clone)] T>(Arc<T>);
```

We can equivalently write:

```rust
#[derive(Clone, PartialEq)]
#[no_bound(Clone)]
struct MyArc<T>(Arc<T>);
```

Here, a meaningful `PartialEq` for `MyArc<T>` requires that `T: PartialEq`.
Therefore, we don't want that bound to be removed from the `impl` of `PartialEq`
for `MyArc<T>`. Instead, we use `#[no_bound(Clone)]` and the resulting `impl`s
will be:

```rust
// As before:
impl<T> Clone for MyArc<T> { /* .. */ }

// And `T: PartialEq` is there as expected!
impl<T: PartialEq> PartialEq for MyArc<T> { /* .. */ }
```

[proptest]: https://docs.rs/proptest/
[`Strategy`]: https://docs.rs/proptest/*/proptest/strategy/trait.Strategy.html

Let's consider this scenario in action with a real world example and create
a wrapper around a trait object of [`Strategy`] in the crate [proptest]:

```rust
#[derive(Clone, Debug)]
pub struct ArcStrategy<#[no_bound(Clone)] T> {
    source: Arc<Strategy<Value = Box<ValueTree<Value = T>>>>
}

// Debug is required as seen in these snippets:
pub trait ValueTree { type Value: Debug; }
pub trait Strategy: Debug { type Value: ValueTree; }
```

In this case, the generated code will be:

```rust
impl<T> Clone for ArcStrategy<T> { /* .. */ }
impl<T: Debug> Debug for ArcStrategy<T> { /* .. */ }
```

We have so far considered a single type parameter. Let's now add another.
We consider a `Refl` encoding in Rust:

```rust
use std::marker::PhantomData;

/// A proof term that `S` and `T` are the same type (type identity).
/// This type is only every inhabited when `S` is nominally equivalent to `T`.
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[no_bound]
pub struct Id<S: ?Sized, T: ?Sized>(PhantomData<(*mut S, *mut T)>);

// ..
```

This will generate the following `impl`s:

```rust
impl<S: ?Sized, T: ?Sized> Copy       for Id<S, T> { /* .. */ }
impl<S: ?Sized, T: ?Sized> Clone      for Id<S, T> { /* .. */ }
impl<S: ?Sized, T: ?Sized> Debug      for Id<S, T> { /* .. */ }
impl<S: ?Sized, T: ?Sized> Hash       for Id<S, T> { /* .. */ }
impl<S: ?Sized, T: ?Sized> PartialEq  for Id<S, T> { /* .. */ }
impl<S: ?Sized, T: ?Sized> Eq         for Id<S, T> { /* .. */ }
impl<S: ?Sized, T: ?Sized> PartialOrd for Id<S, T> { /* .. */ }
impl<S: ?Sized, T: ?Sized> Ord        for Id<S, T> { /* .. */ }
```

In this case in particular, we've reduced a lot of clutter as well as
unnecessary typing.

Why do we need to be able to remove bounds on different parameters
independently? Because their behavior may diverge. Let's consider such
a type where this is the case:

```rust
#[derive(Clone)]
struct Foo<#[no_bound] S, T> {
    bar: Arc<S>,
    baz: T,
}
```

The generated code in this case is:

```rust
impl<S, T: Clone> Clone for Foo { /* .. */ }
```

With an even more complex scenario we have:

```rust
#[derive(Clone, PartialEq)]
struct Foo<#[no_bound(Clone)] S, T, #[no_bound(Clone, PartialEq)] U> {
    bar: Arc<S>,
    baz: T,
    quux: PhantomData<U>
}
```

and the generated code is:

```rust
impl<S, T: Clone, U> Clone for Foo { /* .. */ }
impl<S: PartialEq, T: PartialEq, U> Clone for Foo { /* .. */ }
```

### `#[no_bound]` is not `#[ignore]`

Consider the case of `Filter<I, P>` as in:

```rust
/// An iterator that filters the elements of `iter` with `predicate`.
#[derive(Clone)]
pub struct Filter<I, P> {
    iter: I,
    predicate: P,
}
```

This type provides the `impl`:
```rust
impl<I: Debug, P> Debug for Filter<I, P>
```

Notice in particular that `P` lacks the bound `Debug`.
To derive `Debug` instead, you might want to reach for `#[no_bound]`
on `P` in this case as in:

```rust
#[derive(Clone, Debug)]
pub struct Filter<I, #[no_bound] P> {
    iter: I,
    predicate: P,
}
```

This however, does not work! Why? Because `#[no_bound]` on `P` means that:
> The parameter `P` does not __need__ to satisfy `Trait` for any field
> referencing it to be `Trait`

It does not mean that:
> Ignore the field `predicate`

Therefore, deriving `Debug` will not work as above since the deriving
mechanism of `Debug` will try to generate an `impl` which does not work:

```rust
impl<I: Debug, P> Debug for Filter<I, P> {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        f.debug_struct("Filter")
         .field("iter", &self.iter)
         .field("predicate", &self.predicate) // <-- Not OK!
         .finish()
    }
}
```

Instead the proper `impl`:

```rust
impl<I: Debug, P> Debug for Filter<I, P> {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.debug_struct("Filter")
            .field("iter", &self.iter)
            .finish()
    }
}
```

## Adding bounds on field types with `#[field_bound]`

To gain more exact control of the bounds put on `impl`s generated by
deriving macros you can also use the `#[field_bound(..)]` attribute.

A simple example is:

```rust
#[derive(Clone, PartialEq, PartialOrd)]
struct Foo<S, T> {
    #[field_bound]
    bar: Bar<S>,
    baz: Baz<T>
}
```

This will generate the following `impl`s:

```rust
impl<S: Clone, T: Clone> Clone for Foo<S, T>
where Bar<S>: Clone { /* .. */ }

impl<S: PartialEq, T: PartialEq> Clone for Foo<S, T>
where Bar<S>: PartialEq { /* .. */ }

impl<S: PartialOrd, T: PartialEq> Clone for Foo<S, T>
where Bar<S>: PartialEq { /* .. */ }
```

We can also apply this to a specific trait `impl`:

```rust
#[derive(Clone, PartialEq, PartialOrd)]
struct Foo<S, T> {
    #[field_bound(Clone)]
    bar: Bar<S>,
    #[field_bound(Clone)]
    baz: Baz<T>
}
```

This will generate the following `impl`s:

```rust
impl<S: Clone, T: Clone> Clone for Foo<S, T>
where Bar<S>: Clone, Baz<T>: Clone { /* .. */ }

impl<S: PartialEq, T: PartialEq> Clone for Foo<S, T> { /* .. */ }

impl<S: PartialOrd, T: PartialEq> Clone for Foo<S, T> { /* .. */ }
```

We can simplify the definition above to:

```rust
#[derive(Clone, PartialEq, PartialOrd)]
#[field_bound(Clone)]
struct Foo<S, T> {
    bar: Bar<S>,
    baz: Baz<T>
}
```

or if we want to do this for all derived traits:

```rust
#[derive(Clone, PartialEq, PartialOrd)]
#[field_bound]
struct Foo<S, T> {
    bar: Bar<S>,
    baz: Baz<T>
}
```

### A note on visibility

It is important to note that the following generated `impl`:

```rust
impl<S: Clone, T: Clone> Clone for Foo<S, T> where Bar<S>: Clone { /* .. */ }
```

only works if `Foo<S, T>` is at least as visible as `Bar<S>`.
In particular, a Rust compiler will reject the `impl` above
if `Bar<S>` is private and `Foo<S, T>` is `pub`.

## Guidance to custom derive macro authors

The concepts in this RFC should be taught to derive macro users, by explaining
how the attributes work with derivable traits in the standard library.
These are fairly advanced concepts. As such, they should be deferred to the end
of the book's explanation of Derivable Traits in the appendix section `21.3`.

For users looking to implement custom derive macros, these concepts should
be explained in conjunction with guides on how to implement these macros.

Ideally, the `syn` crate, or crates in the same space such as `synstructure`,
should also facilitate handling of the proposed attributes.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The attributes `#[no_bound(..)]` and `#[field_bound(..)]` for controlling
how bounds are used by derive macros for standard library traits and should
be used for those outside in custom derive macros.

## `#[no_bound(..)]`

### Grammar

The attribute `#[no_bound(..)]` can be placed on type definitions directly
(`struct`, `enum`, `union`) or on formal type parameters. The attribute has
the following grammar:

```enbf
no_bound_attr : "#" "[" "no_bound" no_bound_traits? "]" ;
no_bound_traits : "(" trait_list ","? ")" ;
trait_list : ident | ident "," trait_list ;
```

### Semantics - on a formal type parameter

Formally: Assuming a formal type parameter `P`, and the attribute
`#[no_bound(Trait)]` on `P` for a given specific trait `Trait`, specifying
the attribute `#[derive(Trait)]` shall **NOT** add a bound `P: Trait` to
either the `where`-clause or directly where `P` is brought into scope
(`impl<P: Bound..>`) in the `impl<.., P, ..> Trait<..> for Type<.., P, ..>`
generated by a derive macro for `Trait`. This does not necessarily mean that
the field which in some way references `P` does not need to implement the
`Trait` in question.

When `#[no_bound(..)]` contains a comma separated list of traits,
these semantics will apply to each trait referenced but not other traits.

When `#[no_bound]` is used (with no traits referenced), these rules will
apply to all derived traits.

#### An example

Given the following type definition:

```rust
#[derive(Clone)]
struct Foo<#[no_bound] S, T> {
    bar: Arc<S>,
    baz: T,
}
```

The generated `impl` is:

```rust
impl<S, T: Clone> // <-- S: Clone is missing
Clone for Foo { /* .. */ }
```

### Semantics - on a type

When `#[no_bound(..)]` is applied directly on a type, this is equivalent to
specifying the identical attribute on each formal type parameter of the type.

#### An example

Consider a `Refl` encoding in Rust:

```rust
use std::marker::PhantomData;

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[no_bound]
pub struct Id<S: ?Sized, T: ?Sized>(PhantomData<(*mut S, *mut T)>);
```

The generated `impl`s are:

```rust
impl<S: ?Sized, T: ?Sized> Copy       for Id<S, T> { /* .. */ }
impl<S: ?Sized, T: ?Sized> Clone      for Id<S, T> { /* .. */ }
impl<S: ?Sized, T: ?Sized> Debug      for Id<S, T> { /* .. */ }
impl<S: ?Sized, T: ?Sized> Hash       for Id<S, T> { /* .. */ }
impl<S: ?Sized, T: ?Sized> PartialEq  for Id<S, T> { /* .. */ }
impl<S: ?Sized, T: ?Sized> Eq         for Id<S, T> { /* .. */ }
impl<S: ?Sized, T: ?Sized> PartialOrd for Id<S, T> { /* .. */ }
impl<S: ?Sized, T: ?Sized> Ord        for Id<S, T> { /* .. */ }
```

## `#[field_bound(..)]`

### Grammar

The attribute `#[field_bound(..)]` can be placed on type definitions directly
(`struct`, `enum`, `union`) or on their fields. Note in particular that they may
not be specified on variants of `enum`s. The attribute has the following grammar:

```enbf
field_bound_attr : "#" "[" "field_bound" field_bound_traits? "]" ;
field_bound_traits : "(" trait_list ","? ")" ;
trait_list : ident | ident "," trait_list ;
```

### Semantics - on a field

Formally: Assuming a field `F`, either named or unnamed, of type `FieldType`,
and the attribute `#[field_bound(Trait)]` on `F` for a specific trait `Trait`,
specifying the attribute `#[derive(Trait)]` shall add a bound `FieldType: Trait`
in the `where`-clause in the `impl<..> Trait<..> for Type<..>` generated by a
derive macro for `Trait`.

When `#[field_bound(..)]` contains a comma separated list of traits,
these semantics will apply to each trait referenced but not other traits.

When `#[field_bound]` is used (with no traits referenced), these rules
will apply to all derived traits.

#### An example

Given the following type definition:

```rust
#[derive(Clone, PartialEq, PartialOrd)]
struct Foo<S, T> {
    #[field_bound(Clone)]
    bar: Bar<S>,
    baz: Baz<T>
}
```

The generated `impl`s are:

```rust
impl<S: Clone, T: Clone> Clone for Foo<S, T>
where Bar<S>: Clone { /* .. */ } // <-- Note the where clause!

impl<S: PartialEq, T: PartialEq> Clone for Foo<S, T> { /* .. */ }

impl<S: PartialOrd, T: PartialEq> Clone for Foo<S, T> { /* .. */ }
```

### Semantics - on a type

When `#[field_bound(..)]` is applied directly on a type, this is equivalent
to specifying the identical attribute on each field of the type.

#### An example

Given the following type definition:

```rust
#[derive(Clone, PartialEq, PartialOrd)]
#[field_bound(Clone)]
struct Foo<S, T> {
    bar: Bar<S>,
    baz: Baz<T>
}
```

The generated `impl`s are:

```rust
impl<S: Clone, T: Clone> Clone for Foo<S, T>
where Bar<S>: Clone, Baz<T>: Clone { /* .. */ } // <-- Note!

impl<S: PartialEq, T: PartialEq> Clone for Foo<S, T> { /* .. */ }

impl<S: PartialOrd, T: PartialEq> Clone for Foo<S, T> { /* .. */ }
```

## Warnings

A warning should be issued if:

1. `#[no_bound]` is specified on a type definition without type parameters.

2. `#[no_bound(Trait)]` is specified on a type definition which does not derive
   `Trait`.

3. `#[no_bound]` is specified on a type definition which does not derive any
   trait.

4. `#[field_bound]` is specified on a type without fields.

5. `#[field_bound]` is specified on a field which is less visible than the type
   which contains the field.

6. `#[field_bound(Trait)]` is specified on a field of a type definition which
   does not derive `Trait`.

7. `#[field_bound]` is specified on a field of a type definition which do not
   derive any trait.

## Deriving of standard library traits

Deriving any standard library trait will obey the semantics here specified.

## Custom derive macros

All custom derive macros as **encouraged** to follow the semantics here
specified so that a consistent experience is maintained in the ecosystem.

# Drawbacks
[drawbacks]: #drawbacks

1. It imposes expectations upon custom derive macro authors which they do
   not have time for. This can be mitigated by helper crates.

2. Flexible deriving is a nice-to-have feature but does not enable users to
   express things otherwise not expressible. Arguably, the now-derivable
   `impl`s should be implemented manually.

3. The complexity of the derive system is increased.

# Rationale and alternatives
[alternatives]: #alternatives

The designs proposed by this RFC aims to make deriving cover `impl`s
that are not derivable today. The design has been considered against
real world scenarios. Some trade-offs and choices are discussed in the
[Unresolved questions][unresolved] section.

As with any RFC, an alternative is to say that the status quo is good enough,
but for the reasons mentioned in the [motivation], steps should be taken to
make the derive system of Rust more flexible.

# Prior art
[prior-art]: #prior-art

[RFC 534]: https://github.com/rust-lang/rfcs/blob/master/text/0534-deriving2derive.md

The deriving mechanism of Rust was inspired by Haskell, a fact evidenced by
the change in [RFC 534] where `#[deriving(..)]` became `#[derive(..)]`.

As Haskell does not have a feature similar to Rust's attributes, it is not
possible to configure deriving mechanisms in Haskell. Therefore, there is no
prior art. The features proposed here would be unique to Rust.

# Unresolved questions
[unresolved]: #unresolved-questions

## 1. Should `#[no_bound]` be permitted on fields?

Let's reconsider this example:

```rust
#[derive(Clone, PartialEq)]
struct Foo<#[no_bound(Clone)] S, T, #[no_bound(Clone, PartialEq)] U> {
    bar: Arc<S>,
    baz: T,
    quux: PhantomData<U>
}
```

We could also permit `#[no_bound(..)]` on fields as well and reformulate
the above snippet as:

```rust
#[derive(Clone, PartialEq)]
struct Foo<S, T, U> {
    #[no_bound(Clone)]
    bar: Arc<S>,
    baz: T,
    #[no_bound(Clone, PartialEq)]
    quux: PhantomData<U>
}
```

This is arguably more readable, but hinges on the semantics that bounds are
added by performing name resolution on each field's type and searching for
type parameters in those for usage. This behavior, while not very complex to
encode using visitors the `syn` crate, is not used by derivable traits in the
standard library. Therefore, the experience would not be uniform across traits.

Such behavior will also handle type macros poorly. Given the type position
macro `Foo` and type `Bar`:

```rust
macro_rules! Foo { () => { T } }
struct Bar<T>(Foo!())
```

macros have no way to expand `Foo!()`. Arguably, using type position macros
are rare, but for standardization, a more robust approach is probably preferred.
A possibly path ahead is to provide the API proposed  in [RFC 2320], in which
case using the field based approach becomes more robust.

[RFC 2320]: https://github.com/rust-lang/rfcs/pull/2320

## 2. Should `#[field_bound]` and `#[no_bound]` be combinable?

Consider the following snippet:

```rust
#[derive(Clone, PartialEq, PartialOrd)]
struct Foo<T> {
    #[field_bound]
    #[no_bound(Clone)]
    field: Bar<T>
}
```

This could be interpreted as an instruction to provide the following `impl`s:

```rust
impl<T> Clone for Foo<T> {..}
impl<T: PartialEq> PartialEq for Foo<T> where Bar<T>: PartialEq {..}
impl<T: PartialOrd> PartialOrd for Foo<T> where Bar<T>: PartialOrd {..}
```

This is currently not proposed as it is deemed unnecessary, but the mechanism
should be considered.

## 3. Should `#[field_bound]` be named just `#[bound]`?

The latter is shorter, but less legible, wherefore we've opted to use
`#[field_bound]` at the moment.

## 4. Should the attributes be prefixed with `derive_`?

While this makes the attributes more legible on types and reduces the
chance of conflict, prefixing the attributes with `derive_` can become
overly verbose, wherefore the RFC currently does not propose prefixing.
Such prefixing can become especially verbose when applied on type parameters.

## 5. Permit `field: Vec<#[field_bound] Arc<T>>`?

If so, `#[bound]` is a more correct name. However, the current thinking
is that this requires parsing changes while also looking weird. This may
be a step too far - in such cases, manual `impl`s are probably better.
For these reasons, the RFC does not propose this mechanism currently.

## 6. Permit `#[bound(<List of traits>, T: <Bound>)]`?

[serde_bound_desc]: https://serde.rs/container-attrs.html#serdebound--t-mytrait

Last but not least, the crate `serde` allows the attribute
`#[serde(bound = "T: MyBound")]` which replaces the `where`
clause of the `impl` generated by `serde`. This attribute
is [described][serde_bound_desc] as follows:

> Where-clause for the `Serialize` and `Deserialize` impls.
> This replaces any trait bounds inferred by Serde.

We could standardize this concept in the form of an attribute
`#[bound(..)]` put on types with a syntax permitting:

+ Replace bounds on impl of `Clone` and `PartialEq` with `T: Sync`

```rust
#[bound(Clone, PartialEq, T: Sync)]
```

+ Replace bounds on impl of `Clone` with `T: Sync + 'static`

```rust
#[bound(Clone, T: Sync + 'static)]
```

+ Replace bounds on all derived traits with `T: Copy`

```rust
#[bound(T: Copy)]
```

+ No bounds on impl of `Clone` and `PartialEq`

```rust
#[bound(Clone, PartialEq)]
```

+ No bounds on impl of `Clone`

```rust
#[bound(Clone)]
```

+ No bounds on all derived traits:

```rust
#[bound]
```

The syntax `TyVar: Bound` is however not allowed in attributes currently.
Changing this would require a language change. Another option is to quote the
bound as `"TyVar: Bound"` as done by `serde`. This requires no larger changes,
but is brittle, strange, and would require of syntax highlighters to understand
`#[bound]` specially. Therefore, a more permissible attribute syntax might be a
good thing and can have positive effects elsewhere.

[A real world example]: https://github.com/ppedrot/kravanenn/blob/61f089e2091d1f0c4eb57b2617532e7bee63508d/src/ocaml/values.rs#L10

[A real world example] of how `serde`'s attribute is used is:
```rust
#[derive(Debug, Clone, DeserializeState, Hash, PartialEq, Eq)]
#[serde(deserialize_state = "Seed<'de>")]
#[serde(bound(deserialize =
    "T: serde::de::DeserializeState<'de, Seed<'de>> + Send + Sync + 'static"))]
pub enum List<T> {
    Nil,
     Cons(#[serde(deserialize_state)] ORef<(T, List<T>)>),
}
```

with `#[bound]`, this is rewritten as:

```rust
#[derive(Debug, Clone, DeserializeState, Hash, PartialEq, Eq)]
#[serde(deserialize_state = "Seed<'de>")]
#[bound(Deserialize,
    T: serde::de::DeserializeState<'de, Seed<'de>> + Send + Sync + 'static)]
pub enum List<T> {
    Nil,
     Cons(#[serde(deserialize_state)] ORef<(T, List<T>)>),
}
```