- Feature Name: `type_alias_impl_trait`
- Start Date: 2018-08-03
- RFC PR: [rust-lang/rfcs#2515](https://github.com/rust-lang/rfcs/pull/2515)
- Rust Issue: [rust-lang/rust#63063](https://github.com/rust-lang/rust/issues/63063)

# Summary
[summary]: #summary

Allow type aliases and associated types to use `impl Trait`, replacing the prototype `existential type` as a way to declare type aliases and associated types for opaque, uniquely inferred types.

# Motivation
[motivation]: #motivation

[RFC 2071](https://github.com/rust-lang/rfcs/blob/master/text/2071-impl-trait-existential-types.md) described a method to define opaque types satisfying certain bounds (described in RFC 2071 and elsewhere as *existential types*). It left open the question of what the precise concrete syntax for the feature should be, opting to use a placeholder syntax, `existential type`. Since then, a clearer picture has emerged as to how to rephrase `impl Trait` in terms of type inference, rather than existentially-quantified types, which also provides new motivation for a proposed concrete syntax making use of the existing and familiar syntax `impl Trait`.

In essence, this RFC proposes that the syntax:

```rust
type Foo = impl Bar;
```

be implemented with the same semantics as:

```rust
existential type Foo: Bar;
```

both as the syntax for type aliases and also for associated types, and that existing placeholder be removed.

Furthermore, this RFC proposes a strategy by which the terminology surrounding `impl Trait` might be transitioned from existentially-type theoretic terminology to type inference terminology, reducing the cognitive complexity of the feature.

## Semantic Justification
Currently, each occurrence `impl Trait` serves two complementary functional purposes.
1. It defines an opaque type `T` (that is, a new type whose precise identification is hidden) satisfying (trait) bounds.
2. It infers the precise type for `T` (that must satisfy the bounds for `T`), based on its occurrences.

Thus, the following code:

```rust
fn foo() -> impl Bar {
    // return some type implementing `Bar`
}
```

is functionally equivalent to:

```rust
struct __foo_return(/* some inferred type (2) */); // (1)

fn foo() -> __foo_return {
    // return some type implementing `Bar` wrapped in `__foo_return` (3)
}
```

The generated type `__foo_return` is not exposed: it is automatically constructed from any valid type (as in `(3)`).

Note that, in order for the type inference to support argument-position `impl Trait`, which may be polymorphic (just like a generic parameter), the inference used here is actually a more expressive form of type inference similar to ML-style let polymorphism. Here, the inference of function types may result in additional generic parameters, specifically relating to the occurrences of argument-position `impl Trait`.

RFC 2071 proposed a new construct for declaring types acting like `impl Trait`, but whose actual type was not hidden (i.e. a method to expose the `__foo_return` above), to use such types in positions other than function arguments and return-types (for example, at the module level).

If the semantics of `impl Trait` are justified from the perspective of existentially-quantified types, this new construct is a sensible solution as re-using `impl Trait` for this purpose introduces additional inconsistency with the existential quantifier scopes. (See [here](https://varkor.github.io/blog/2018/07/03/existential-types-in-rust.html) for more details on this point.)

However, if we justify the semantics of `impl Trait` solely using type inference (as in point 2 above, expounded below) then we can re-use `impl Trait` for the purpose of `existential type` consistently, leading to a more unified syntax and lower cognitive barrier to learning.

Here, we define the syntax:

```rust
type Foo = impl Bar;
```

to represent a type alias to a generated type:

```rust
struct __Foo_alias(/* some inferred type */);
type Foo = __Foo_alias;
```

This is functionally identical to `existential type`, but remains consistent with `impl Trait` where the original generated type is technically still hidden (exposed through the type alias).

### Aliasing `impl Trait` in function signatures
Note that though the type alias above is not contextual, it can be used to alias any existing occurrence of `impl Trait` in return position, because the type it aliases is inferred.

```rust
fn foo() -> impl Bar {
    // return some type implementing `Bar`
}
```

can be replaced by:

```rust
type Baz = impl Bar;

fn foo() -> Baz {
    // return some type implementing `Bar`
}
```

However, if the function is parameterised, it may be necessary to add explicit parameters to the type alias (due to the return-type being within the scope of the function's generic parameters, unlike the type alias).

Using `Baz` in multiple locations constrains all occurrences of the inferred type to be the same, just as with `existential type`.

Notice that we can describe the type alias syntax using features that are already present in Rust, rather than introducing any new constructs.

## Learnability Justification

###  Reduced technical and theoretic complexity
As a relatively recently stabilised feature, there is not significant (official) documentation on `impl Trait` so far. Apart from the various RFC threads and internal discussions, `impl Trait` [is described in a blog post](https://blog.rust-lang.org/2018/05/10/Rust-1.26.html) and in the [Rust 2018 edition guide](https://rust-lang-nursery.github.io/edition-guide/2018/transitioning/traits/impl-trait.html). The edition guide primary describes `impl Trait` intuitively, in terms of use cases. It does however contain the following:

> `impl Trait` in argument position are universal (universally quantified types). Meanwhile, `impl Trait` in return position are existentials (existentially quantified types).

[This is incorrect](https://varkor.github.io/blog/2018/07/03/existential-types-in-rust.html#confusion-2-return-position-impl-trait-vs-argument-position-impl-trait) (albeit subtly): in fact, the distinction between argument-position and return-position `impl Trait` is the scope of their existential quantifier. This (understandable) mistake is pervasive and it's not alone (the fact that those documenting the feature missed this is indicative of the issues surrounding this mental model). The problem stems from a poor understanding of what "existential types" are — which is entirely unsurprising: existential types are a technical type theoretic concept that are not widely encountered outside type theory (unlike universally-quantified types, for instance). In discussions about existential types in Rust, these sorts of confusions are endemic.

In any model that does not unify the meaning of `impl Trait` in various positions, these technical explanations are likely to arise, as they provide the original motivation for treating `impl Trait` nonhomogeneously. From this perspective, it is valuable from documentation and explanatory angles to unify the uses of `impl Trait` so that these types of questions never even arise. Then we would have the ability to transition entirely away from the topic of existentially-quantified types.

### Natural syntax
Having explained `impl Trait` solely in terms of type inference (or less formal equivalent explanations), the syntax proposed here is the only natural syntax. Indeed, while discussing the syntax here, many express surprise that this syntax has ever been under question (often from people who think of `impl Trait` from an intuition about the feature's behaviour, rather than thinking about the existential type perspective).

The argument that is occasionally put forward: that this syntax makes type aliases (or their uses) somehow contextual, is also addressed by the above interpretation. Indeed, every use of an individual `impl Trait` type alias refers to the same type. This argument is [detailed and addressed further in **Drawbacks**](#drawbacks).

The following section provides a documentation-style introductory explanation for `impl Trait` that justifies the type alias syntax proposed here.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

[Adapted from the [Rust 2018 edition guide](https://rust-lang-nursery.github.io/edition-guide/2018/transitioning/traits/impl-trait.html#more-details).]

`impl Trait` provides a way to specify unnamed concrete types with specific bounds. You can currently use it in three places (to be extended in future versions of Rust: see [the tracking issue](https://github.com/rust-lang/rust/issues/34511) for more details):
- Argument position
- Return position
- Type aliases

```rust
trait Trait {}

// Argument-position
fn foo(arg: impl Trait) {
    // ...
}

// Return-position
fn bar() -> impl Trait {
    // ...
}

// Type alias
type Baz = impl Trait;
```

## How does `impl Trait` work?
Whenever you write `impl Trait`, in any of the three places, you're saying that you have *some type* that implements `Trait`, but you don't want to expose any more information than that. The concrete type that implements `Trait` will be hidden, but you'll still be able to treat the type as if it implements `Trait`: calling trait methods and so on.

The compiler will infer the concrete type, but other code won't be able to make use of that fact. This is straightforward to describe, but it manifests a little differently depending on the place it's used, so let's take a look at some examples.

## Argument-position
```rust
trait Trait {}

fn foo(arg: impl Trait) {
    // ...
}
```

Here, we're saying that `foo` takes an argument whose type implements `Trait`, but we're not saying exactly what it is. Thus, the caller can pass a value of any type, as long as it implements `Trait`.

You may notice this sounds very like a generic type parameter. In fact, functionally, using `impl Trait` in argument position is almost identical to a generic type parameter.

```rust
fn foo(arg: impl Trait) {
    // ...
}

// is almost the same as:

fn foo<T: Trait>(arg: T) {
    // ...
}
```

The only difference is that you can't use turbo-fish syntax for the first definition (as turbo-fish syntax only works with explicit generic type parameters). Thus, it's worth being mindful that switching between `impl Trait` and generic type parameters can consistute a breaking change for users of your code.

## Return-position
```rust
trait Trait {}

impl Trait for i32 {}

fn bar() -> impl Trait {
    5
}
```

Using `impl Trait` as a return type is more useful, as it enables us to do things we weren't able to before. In this example, `bar` returns some type that's not specified: it just asserts that the type implements `Trait`. Inside the function, we can return any type that fits, but from the caller's perspective, all they know is that the type implements the trait.

This is useful especially for two things:
- Hiding (potentially complex) implementation details
- Referring to types that were previously unnameable, such as closures

[Here, we would also provide a more useful example, as in the [Rust 2018 edition guide](https://rust-lang-nursery.github.io/edition-guide/2018/transitioning/traits/impl-trait.html#impl-trait-and-closures).]

## Type alias
```rust
trait Trait {}

type Baz = impl Trait;
```

`impl Trait` type aliases are useful for declaring types that are constrained by traits, but whose concrete type should be a hidden implementation detail. We can use it in place of return-position `impl Trait` as in the previous examples.

```rust
trait Trait {}

type Baz = impl Trait;

// The same as `fn bar() -> impl Baz`
fn bar() -> Baz {
    // ...
}
```

However, if we use `Baz` in multiple locations, we constrain the concrete type referred to by `Baz` to be the same, so we get a type that we know will be the same everywhere and will satisfy specific bounds, whose concrete type is hidden. This can be useful in libraries where you want to hide implementation details.

```rust
trait Trait {}

type Baz = impl Trait;

impl Trait for u8 {}

fn foo() -> Baz {
    let x: u8;
    // ...
    x
}

fn bar(x: Baz, y: Baz) {
    // ...
}

struct Foo {
    a: Baz,
    b: (Baz, Baz),
}
```

In this example, the concrete type referred to by `Baz` is guaranteed to be the same wherever `Baz` occurs.

Note that using `Baz` as an argument type is *not* the same as argument-position `impl Trait`, as `Baz` refers to a unique type, whereas the concrete type for argument-position `impl Trait` is determined by the caller.

```rust
trait Trait {}

type Baz = impl Trait;

fn foo(x: Baz) {
    // ...
}

// is *not* the same as:

fn foo(x: impl Trait) {
    // ...
}
```

Just like with any other type alias, we can use `impl Trait` to specify associated types for traits, as in the following example.

```rust
trait Trait {
    type Assoc;
}

struct Foo {}

impl Trait for Foo {
    type Assoc = impl Debug;
}
```

Here, anything that makes use of `Foo` knows that `Foo::Assoc` implements `Debug`, but has no knowledge of its concrete type.

[Eventually, we would also describe the use of `impl Trait` in `let`, `const` and `static` bindings, but as they are as-yet unimplemented and function the same as return-type `impl Trait`, they haven't been included here.]

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Since RFC 2071 was accepted, the initial implementation of `existential type` [has already been completed](https://github.com/rust-lang/rust/pull/52024). This RFC would replace the syntax of `existential type`, from:

```rust
existential type Foo: Bar;
```

to:

```rust
type Foo = impl Bar;
```

In addition, having multiple occurrences of `impl Trait` in a type alias or associated type is now permitted, where each occurrence is desugared into a separate inferred type. For example, the following alias:

```rust
type Foo = Arc<impl Iterator<Item = impl Debug>>;
```

would be desugared to the equivalent of:

```rust
existential type _0: Debug;
existential type _1: Iterator<Item = _0>;
type Foo = Arc<_1>;
```

Furthermore, when documenting `impl Trait`, explanations of the feature would avoid type theoretic terminology (specifically "existential types") and prefer type inference language (if any technical description is needed at all).

`impl Trait` type aliases may contain generic parameters just like any other type alias. The type alias must contain the same type parameters as its concrete type, except those implicitly captured in the scope (see [RFC 2071](https://github.com/rust-lang/rfcs/blob/master/text/2071-impl-trait-existential-types.md) for details).

```rust
// `impl Trait` type aliases may contain type parameters...
#[derive(Debug)]
struct DebugWrapper<T: Debug>(T);

type Foo<T> = impl Debug;

fn get_foo<T: Debug>(x: T) -> Foo<T> { DebugWrapper(x) }

// ...and lifetime parameters (and so on).
#[derive(Debug)]
struct UnitRefWrapper<'a>(&'a ());

type Bar<'a> = impl Debug;

fn get_bar<'a>(y: &'a ()) -> Bar<'a> { UnitRefWrapper(y) }
```

# Drawbacks
[drawbacks]: #drawbacks

This feature has already been accepted under a placeholder syntax, so the only reason not to do this is if another syntax is chosen as a better choice, from an ergonomic and consistency perspective.

There is one critique of the type alias syntax proposed here, which is frequently brought up in discussions, regarding referential transparency.

Consider the following code:

```rust
fn foo() -> impl Trait { /* ... */ }
fn bar() -> impl Trait { /* ... */ }
```

A user who has not come across `impl Trait` before might imagine that the return type of both functions is the same (as synactically, they are). However, because each occurrence of `impl Trait` defines a new type, the return types are potentially distinct.

This is a problem inherent with `impl Trait` (and any other syntax that determines a type contextually) and thus `impl Trait` type aliases have the same caveat.

A user unaware of the behaviour of `impl Trait` might try refactoring this example into the following:

```rust
type SharedImplTrait = impl Trait;

fn foo() -> SharedImplTrait { /* ... */ }
fn bar() -> SharedImplTrait { /* ... */ }
```

This evidently means something different to what the user intended, because here `SharedImplTrait` is inferred as a single type, shared with `foo` and `bar`.

However, this problem is specifically with the behaviour of `impl Trait` and not with the type aliases, whose behaviour is not altered. Specifically note that, after this RFC, it is still true that for any type alias:

```rust
type Alias = /* ... */;
```

all uses of `Alias` refer to the same unique type. The potential confusion is rather with whether all uses of `impl Trait` refer to the same unique type (which is, of course, false).

It is likely that a misunderstanding of the nature of `impl Trait` in argument or return position will lead to similar confusion as to the role of `impl Trait` in type aliases, and vice versa. By clearly teaching the behaviour of `impl Trait`, we should be able to eliminate most of these conceptual difficulties.

Since we will teach `impl Trait` cohesively (that is, argument-position, return-position and type alias `impl Trait` at the same time), it is unlikely that users who understand `impl Trait` will be confused about `impl Trait` type aliases. (What's more, examples in the reference will illustrate this clearly.)

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives
The justification for the type alias syntax proposed here comes down to two key motvations:
- Consistency
- Minimality

Ideally a language should provide as small a surface area as possible. New keywords or constructs add to the cognitive complexity of a language, requiring users to look more concepts up or read larger guides to understand code they read and want to write. If it is possible to add new capabilities to the language that fit into the existing syntax and concepts, this generally increases cohesion.

The syntax proposed here is a natural extension of the existing `impl Trait` syntax and it is felt that, should users encounter it after seeing argument-position and return-position `impl Trait`, its meaning will be immediately clear. On the other hand, new keywords or syntax will require the user to investigate further and provide more questions:
- "Why can't I use `impl Trait` here?"
- "What's the difference between `impl Trait` and X?"

Using different syntax, and then trying to justify the differences between `impl Trait` and some new feature, seems likely to lead into conversations about existential types, which are almost always unhelpful for understanding.

`type Foo = impl Bar;` has the additional benefit that it's easy to search for and can appear alongside documentation for other uses of `impl Trait`.

The syntax `existential type` was intended to be a placeholder, so we need to pick a syntax eventually for this feature. Justification for why this is the best syntax, given the existing syntax in Rust, has been included throughout the RFC.

The other alternatives commonly given are:
- `type Foo: Bar;`, which suffers from complete and confusing inconsistency with associated types. Although on the surface, they can appear similar to existential types, by virtue of being a declaration that "some type exists [that will be provided]", they are more closely related to type parameters (which also declare that "some type exists that will be provided"), though type parameters with [Haskell-style functional dependencies](https://wiki.haskell.org/Functional_dependencies). This is sure to lead to confusions as users wonder why two features with identical syntax turn out to behave so differently.
- Some other, new syntax for declaring a new type that acts in the same way as `existential type`. Though a new syntax would not be inconsistent, it would not be minimal, given that we can achieve the functionality using existing syntax (`impl Trait`). What's more, if the syntax proposed here were *not* added alongside this new syntax, this would lead to inconsistencies with `impl Trait`.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None
