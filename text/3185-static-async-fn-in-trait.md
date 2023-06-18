# Static async fn in traits

- Feature Name: `async_fn_in_trait`
- Start Date: 2021-10-13
- RFC PR: [rust-lang/rfcs#3185](https://github.com/rust-lang/rfcs/pull/3185)
- Rust Issue: [rust-lang/rust#91611](https://github.com/rust-lang/rust/issues/91611)

# Summary
[summary]: #summary

Support `async fn` in traits that can be called via static dispatch. These will desugar to an anonymous associated type.

# Motivation
[motivation]: #motivation

Async/await allows users to write asynchronous code much easier than they could before. However, it doesn't play nice with other core language features that make Rust the great language it is, like traits.

In this RFC we will begin the process of integrating these two features and smoothing over a wrinkle that async Rust users have been working around since async/await stabilized nearly 3 years ago.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

You can write `async fn` in traits and trait impls. For example:

```rust
trait Service {
    async fn request(&self, key: i32) -> Response;
}

struct MyService {
    db: Database
}

impl Service for MyService {
    async fn request(&self, key: i32) -> Response {
        Response {
            contents: self.db.query(key).await.to_string()
        }
    }
}
```

This is useful for writing generic async code.

Currently, if you use an `async fn` in a trait, that trait is not `dyn` safe. If you need to use dynamic dispatch combined with async functions, you can use the [`async-trait`] crate. We expect to extend the language to support this use case in the future.

Note that if a function in a trait is written as an `async fn`, it must also be written as an `async fn` in your implementation of that trait. With the above trait, you could not write this:

```rust
impl Service for MyService {
    fn request(&self, key: i32) -> impl Future<Output = Response> {
        async move {
            ...
        }
    }
}
```

Doing so will give you an "expected async fn" error. If you need to do this for some reason, you can use an associated type in the trait:

```rust
trait Service {
    type RequestFut<'a>: Future<Output = Response>
    where
        Self: 'a;
    fn request(&self, key: i32) -> RequestFut;
}

impl Service for MyService {
    type RequestFut<'a> = impl Future + 'a
    where
        Self: 'a;
    fn request<'a>(&'a self, key: i32) -> RequestFut<'a> {
        async move { ... }
    }
}
```

Note that in the impl we are setting the value of the associated type to `impl Future`, because async blocks produce unnameable opaque types. The associated type is also generic over a lifetime `'a`, which allows it to capture the `&'a self` reference passed by the caller.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## New syntax

We introduce the `async fn` sugar into traits and impls. No changes to the grammar are needed because the Rust grammar already support this construction, but async functions result in compilation errors in later phases of the compiler.

```rust
trait Example {
    async fn method(&self);
}

impl Example for ExampleType {
    async fn method(&self);
}
```

## Semantic rules

When an async function is present in a trait or trait impl...

### The trait is not considered dyn safe

This limitation is expected to be lifted in future RFCs.

### Both the trait and its impls must use `async` syntax

It is not legal to use an async function in a trait and a "desugared" function in an impl.

## Equivalent desugaring

### Trait

Async functions in a trait desugar to an associated function that returns a generic associated type (GAT):

* Just as with [ordinary async functions](https://rust-lang.github.io/rfcs/2394-async_await.html#lifetime-capture-in-the-anonymous-future), the GAT has a generic parameter for every generic parameter that appears on the fn, along with implicit lifetime parameters.
* The GAT has the complete set of where clauses that appear on the `fn`, including any implied bounds.
* The GAT is "anonymous", meaning that its name is an internal symbol that cannot be referred to directly. (In the examples, we will use `$` to represent this name.)


```rust
trait Example {
    async fn method<P0..Pn>(&self)
    where
        WC0..WCn;
}

// Becomes:

trait Example {
    type $<'me, P0..Pn>: Future<Output = ()>
    where
        WC0..WCn, // Explicit where clauses
        Self: 'me; // Implied bound from `&self` parameter

    fn method<P0..Pn>(&self) -> Self::$<'_, P0..Pn>
    where
        WC0..WCn;
}
```

`async fn` that appear in impls are desugared in the same general way as an [existing async function](https://doc.rust-lang.org/reference/items/functions.html#async-functions), but with some slight differences:

* The value of the associated type `$` is equal to an `impl Future` type, rather than the `impl Future` being the return type of the function
* The function returns `Self::$<...>` with all the appropriate generic parameters

Otherwise, the desugaring is the same. The body of the function becomes an `async move { ... }` block that both (a) captures all parameters and (b) contains the body expression.

```rust
impl Example for ExampleType {
    async fn method<P0..Pn>(&self) {
        ...
    }
}

impl Example for ExampleType {
    type $<'me, P0..Pn> = impl Future<Output = ()> + 'me
    where
        WC0..WCn, // Explicit where clauses
        Self: 'me; // Implied bound from `&self` parameter

    fn method<P0..Pn>(&self) -> Self::$<'_, P0..Pn> {
        async move { ... }
    }
}
```

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Why are we adding this RFC now?

This RFC represents the least controversial addition to async/await that we could add right now. It was not added before due to limitations in the compiler that have now been lifted – namely, support for [Generic Associated Types][gat] and [Type Alias Impl Trait][tait].

[gat]: https://github.com/rust-lang/generic-associated-types-initiative
[tait]: https://github.com/rust-lang/rust/issues/63063

## Why are the result traits not dyn safe?

Supporting async fn and dyn is a complex topic -- you can read the details on the [dyn traits](https://rust-lang.github.io/async-fundamentals-initiative/evaluation/challenges/dyn_traits.html) page of the async fundamentals evaluation doc.

## Can we add support for dyn later?

Yes, nothing in this RFC precludes us from making traits containing async functions dyn safe, presuming that we can overcome the obstacles inherent in the design space.

## What are users using today and why don't we just do that?

Users in the ecosystem have worked around the lack of support for this feature with the [async-trait] proc macro, which desugars into `Box<dyn Future>`s instead of anonymous associated types. This has the disadvantage of requiring users to use `Box<dyn>` along with all the [performance implications] of that, which prevent some use cases. It is also not suitable for users like [embassy](https://github.com/embassy-rs/embassy), which aim to support the "no-std" ecosystem.

[async-trait]: https://github.com/dtolnay/async-trait
[performance implications]: https://rust-lang.github.io/wg-async-foundations/vision/submitted_stories/status_quo/barbara_benchmarks_async_trait.html

## Will anyone use async-trait crate once this RFC lands?

The async-trait crate will continue to be useful after this RFC, because it allows traits to remain `dyn`-safe. This is a limitation in the current design that we plan to address in the future.

# Prior art
[prior-art]: #prior-art

## The `async-trait` crate

The most common way to use `async fn` in traits is to use the [`async-trait`] crate. This crate takes a different approach to the one described in this RFC. Async functions are converted into ordinary trait functions that return `Box<dyn Future>` rather than using an associated type. This means that the resulting traits are dyn safe and avoids a dependency on generic associated types, but it also has two downsides:

* Requires a box allocation on every trait function call; while this is often no big deal, it can be prohibitive for some applications.
* Requires the trait to state up front whether the resulting futures are `Send` or not. The [`async-trait`] crate defaults to `Send` and users write `#[async_trait(?Send)]` to disable this default.

Since the async function support in this RFC means that traits are not dyn safe, we do not expect it to completely displace uses of the `#[async_trait]` crate.

[`async-trait`]: https://crates.io/crates/async-trait

## The real-async-trait crate

The [`real-async-trait`] lowers `async fn` to use GATs and impl Trait, roughly as described in this RFC.

[`real-async-trait`]: https://crates.io/crates/real-async-trait

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- None.

# Future possibilities
[future-possibilities]: #future-possibilities

## Dyn compatibility

It is not a breaking change for traits to become dyn safe. We expect to make traits with async functions dyn safe, but doing so requires overcoming a number of interesting challenges, as described in the [async fundamentals evaluation doc][eval].

## Impl trait in traits

The [impl trait initiative] is expecting to propose "impl trait in traits" (see the [explainer](https://rust-lang.github.io/impl-trait-initiative/explainer/rpit_trait.html) for a brief summary). This RFC is compatible with the proposed design.

## Allowing sugared and desugared forms

In the current proposal, `async fn`s in traits must be implemented using `async fn`. Using a desugared form is not allowed, which can preclude implementations from doing things like doing some work at call time before returning a future. It would also be backwards-incompatible for library authors to move between the sugared and desugared form.

Once impl trait in traits is supported, we can redefine the desugaring of `async fn` in traits in terms of that feature (similar to how `async fn` is desugared for free functions). That provides a clear path to allowing the desugared form to be used interchangeably with the `async fn` form. In other words, you should be able to write the following:

```rust
trait Example {
    async fn method(&self);
}

impl Example for ExampleType {
    fn method(&self) -> impl Future<Output = ()> + '_ {}
}
```

It could also be made backward-compatible for the trait to change between the sugared and desugared form.

## Ability to name the type of the returned future

This RFC does not propose any means to name the future that results from an `async fn`. That is expected to be covered in a future RFC from the [impl trait initiative]; you can read more about the [proposed design](https://rust-lang.github.io/impl-trait-initiative/explainer/rpit_names.html) in the explainer.

[eval]: https://rust-lang.github.io/async-fundamentals-initiative/evaluation.html
[impl trait initiative]: https://rust-lang.github.io/impl-trait-initiative/
