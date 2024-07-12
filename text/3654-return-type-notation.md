# Return type notation (RTN) in bounds and where-clauses

- Feature Name: `return_type_notation`
- Start Date: 2024-06-04
- RFC PR: [rust-lang/rfcs#3654](https://github.com/rust-lang/rfcs/pull/3654)
- Tracking Issue: [rust-lang/rust#109417](https://github.com/rust-lang/rust/issues/109417)

# Summary
[summary]: #summary

Return type notation (RTN) gives a way to reference or bound the type returned by a trait method. The new bounds look like `T: Trait<method(..): Send>` or `T::method(..): Send`. The primary use case is to add bounds such as `Send` to the futures returned by `async fn`s in traits and `-> impl Future` functions, but they work for any trait function defined with return-position impl trait (e.g., `where T: Factory<widgets(..): DoubleEndedIterator>` would also be valid).

This RFC proposes a new kind of type written `<T as Trait>::method(..)` (or `T::method(..)` for short). RTN refers to "the type returned by invoking `method` on `T`".

To keep this RFC focused, it only covers usage of RTN as the `Self` type of a bound or where-clause. The expectation is that, after accepting this RFC, we will gradually expand RTN usage to other places as covered under [Future Possibilities](#future-possibilities). As a notable example, supporting RTN in struct field types would allow constructing types that store the results of a call to a trait `-> impl Trait` method, making them [more suitable for use in public APIs](https://rust-lang.github.io/api-guidelines/future-proofing.html).

Examples of RTN usage allowed by this RFC include:

* `where <T as Trait>::method(..): Send`
    * (the base syntax)
* `where T: Trait<method(..): Send>`
    * (sugar for the base syntax with the (recently stabilized) [associated type bounds](https://github.com/rust-lang/rust/issues/52662))
* `where T::method(..): Send`
    * (sugar where `Trait` is inferred from the compiler)
* `dyn Trait<method(..): Send>`
    * (`dyn` types take lists of bounds)
* `impl Trait<method(..): Send>`
    * (...as do `impl` types)

# Motivation
[motivation]: #motivation

Rust now supports async fns and `-> impl Trait` in traits (acronymized as AFIT and RPITIT, respectively), but we currently lack the ability for users to declare additional bounds on the values returned by such functions. This is often referred to as the [Send bound problem][sbp], because the most acute manifestation is the inability to require that an `async fn` returns a `Send` future, but it is actually more general than both async fns and the `Send` trait (as discussed below).

[sbp]: https://smallcultfollowing.com/babysteps/blog/2023/02/01/async-trait-send-bounds-part-1-intro/

## The [send bound problem][sbp] blocks an interoperable async ecosystem

To create an interoperable async ecosystem, we need the ability to write a single trait definition that can be used across all styles of async exectutors (workstealing, thread-per-core, single-threaded, embedded, etc). One example of such a trait is the `Service` trait found in the `tower` crate, which defines a generic "service" that can process a `Request` and yield some `Response`. The [current `Service` trait](https://docs.rs/tower/latest/tower/trait.Service.html) is defined with a custom `poll` method and explicit usage of `Pin`, but the goal is to be able to define `Service` like so:

```rust
trait Service<Request> {
    type Response;

    // Invoke the service.
    async fn call(&self, req: Request) -> Self::Response;
}
```

This `Service` trait can then be used to define generic middleware that operate over any service. For example, we could write a `LogService` that wraps any service and emit logs to stderr:

```rust
pub struct LogService<S>(S);

impl<S, R> Service<R> for LogService<S>
where
    S: Service<R>,
    R: Debug,
{
    type Response = S::Response;

    async fn call(&self, request: R) -> S::Response {
        eprintln!("{request:?}");
        self.0.call(request).await
    }
}
```
### This definition today works only in some executors

Defining `Service` as shown above works fine in a thread-per-core or single-threaded executor, where spawned tasks do not move between threads. But it can encounter compilation errors with a work-stealing executor, such as the default Tokio executor, where all spawned futures must be `Send`. Consider this example:

```rust
async fn spawn_call<S>(service: S) -> S::Response
where
    S: Service<(), Response: Send> + Send + 'static,
{
    tokio::spawn(async move {
        service.call(()).await // <--- Error
    }).await
}
```

This code [will not compile][pgservice] because the future returned by invoking `S::call(..)` is not known to be `Send`:

[pgservice]: https://play.rust-lang.org/?version=nightly&mode=debug&edition=2021&gist=cc756422487005c51b65a9e53df2a7b9

```
error: future cannot be sent between threads safely
   --> src/lib.rs:6:5
    |
6   | /     tokio::spawn(async move {
7   | |         service.call(()).await // <--- Error
8   | |     }).await.unwrap()
    | |______^ future created by async block is not `Send`
    |
    = help: within `{async block@src/lib.rs:6:18: 8:6}`, the trait `Send` is not implemented for `impl Future<Output = <S as Service<()>>::Response>`, which is required by `{async block@src/lib.rs:6:18: 8:6}: Send`
note: future is not `Send` as it awaits another future which is not `Send`
   --> src/lib.rs:7:9
    |
7   |         service.call(()).await // <--- Error
    |         ^^^^^^^^^^^^^^^^ await occurs here on type `impl Future<Output = <S as Service<()>>::Response>`, which is not `Send`
```

The only way today to make this code compile is to modify the `Service` trait definition to *always* return a `Send` future, like so (and in fact if you [try the above example on the playground][pgservice], you will see the compiler suggests a change like this):

```rust
trait SendService<Request>: Send {
    type Response;

    // Invoke the service.
    fn call(
        &self,
        req: Request,
    ) -> impl Future<Output = Self::Response> + Send;
}
```

But this `SendService` trait is too strong for use outside a work-stealing setup. This leaves generic middleware like the `LogService` struct we saw earlier in a bind: should they use `Service` or `SendService`? Really, we want a single single `Service` trait that can be used in both contexts.

## Comparison to an analogous problem with `IntoIterator`

It is useful to compare this situation with analogous scenarios that arise elsewhere in Rust, such as with associated types. Imagine a function that takes an `I: IntoIterator` and which wishes to make use of the returned iterator in a separate thread:

```rust
fn into_iter_example<I: IntoIterator>(i: I) {
    let iter = i.into_iter();
    std::thread::spawn(move || {
        iter.next(); // <-- Error!
    });
}
```

This code will also [not compile][pgintoiter]:

[pgintoiter]: https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=ce95a4a98ce2dc3edd6ef6b1e49533c4


```
error[E0277]: `<I as IntoIterator>::IntoIter` cannot be sent between threads safely
   --> src/lib.rs:3:24
    |
3   |       std::thread::spawn(move || {
    |       ------------------ ^------
    |       |                  |
    |  _____|__________________within this `{closure@src/lib.rs:3:24: 3:31}`
    | |     |
    | |     required by a bound introduced by this call
4   | |         iter.next();
5   | |     });
    | |_____^ `<I as IntoIterator>::IntoIter` cannot be sent between threads safely
...
help: consider further restricting the associated type
    |
1   | fn into_iter_example<I: IntoIterator>(i: I)
    |   where <I as IntoIterator>::IntoIter: Send {
    |
```

There are two ways the function `into_iter_example` could be made to compile:

1. Modify the `IntoIterator` trait to require that the target iterator type is *always* `Send`
2. Modify the function to have a where-clause `I::IntoIter: Send`.

The 1st option is less flexible but more convenient; it is inappropriate in a highly generic trait like `IntoIterator` which is used in a number of scenarios. It would be fine for an application- or library-specific crate that is only used in narrow circumstances. Referring back to the compiler's error message, you can see that an additional where-clause is exactly what it suggested.

This is the challenge: **Rust does not currently have a way to write the equivalent of `where I::IntoIter: Send` for the futures returned by `async fn` (or the results of `-> impl Trait` methods in traits).** This creates a gap between the first `Service` example, which can only be resolved by modifying the trait, and `IntoIterator`, which can be resolved either by modifying the trait or by adding a where-clause to the function, whichever is more appropriate.

## Return type notation (RTN) permits the return type of AFIT and RPITIT to be bounded, closing the gap

The core feature proposed in this RFC is the ability to write a bound that bounds the return type of an AFIT/RPITIT trait method. This allows the `spawn_call` definition to be amended to require that `call()` returns a `Send` future:

```rust
async fn spawn_call<S>(service: S) -> S::Response
where
    S: Service<
        (),
        Response: Send,
        // "The method `call` returns a `Send` future."
        call(..): Send,
    > + Send + 'static,
{
    tokio::spawn(async move {
        service.call(()).await // <--- OK!
    }).await
}
```

A variant of the proposal in this RFC is already implemented, so you can [try this example on the playground and see that it works](https://play.rust-lang.org/?version=nightly&mode=debug&edition=2021&gist=46ba0640607762280ae2380ff0167edf).

## RTN is useful for more than `Send` bounds

RTN is useful for more than `Send` bounds. For example, consider the trait `Factory`, which contains a method that returns an `impl Iterator`:

```rust
trait Factory {
    fn widgets(&self) -> impl Iterator<Item = Widget>;
}
```

Now imagine that there are many `Factory` implementations, but only some of them return iterators that support `DoubleEndedIterator`.
Making use of RTN, we can write a "reverse factory" that can be used on precisely those instances ([playground](https://play.rust-lang.org/?version=nightly&mode=debug&edition=2021&gist=6d45f55355188001ea6499314ce30b4b)):

```rust
struct ReverseWidgets<F: Factory<widgets(..): DoubleEndedIterator>> {
    factory: F,
}

impl<F> Factory for ReverseWidgets<F>
where
    F: Factory<widgets(..): DoubleEndedIterator>,
{
    fn widgets(&self) -> impl Iterator<Item = Widget> {
        self.factory.widgets().rev()
        //                     ^^^ requires that the iterator be double-ended
    }
}
```

## RTN supports convenient trait aliases

The async WG conducted several [case studies][] to test the usefulness of RTN.
We found that RTN is very important for using async fn in practice,
but we also found that RTN alone can be repetitive in traits that have many methods.

We expect most users in the wild to define "trait aliases" to indicate cases where all methods in a trait are `Send` (and perhaps other traits). The (rust-lang supported) [trait-variant][] crate can automate this process. For example, the following code creates a `SendService` alias, which is automatically implemented by any type `T: Service` where `T: Send` and `T::call(..): Send`:

[case studies]: https://rust-lang.github.io/async-fundamentals-initiative/evaluation/case-studies.html
[trait-variant]: https://github.com/rust-lang/impl-trait-utils

```rust
#[trait_variant::make(SendService: Send)]
//                    -----------  ----
//                    |            |
//        name of the trait alias  |
//                                 |
//                    additional bound that must be met
//                    by async or `-> impl Trait` methods
trait Service<Request> {
    type Response;

    // Invoke the service.
    async fn call(&self, req: Request) -> Self::Response;
}
```

The expansion of this macro use RTN to create a trait that both (1) implies a `Service` whose methods return `Send` futures and (2) which is automatically implemented for all `Service` types whose methods are `Send` (this expansion could be altered to make use of [true trait aliases](https://github.com/rust-lang/rust/issues/41517) once those are stabilized):

```rust
trait SendService<R>:   // a `SendService` is...
    Service<            // ...a `Service`...
        R,
        call(..): Send, // ...where `call` returns
                        // a `Send` future...
    > +
    Send                // ...and which is itself `Send`.
{}

impl<S, R> SendService<R> for S
where
    S: Send + Service<R, call(..): Send>,
{}
```

The function `spawn_call` can then be written as follows:

```rust
async fn spawn_call<S>(service: S) -> S::Response
where
    S: SendService<(), Response: Send> + 'static,
    // ^^^^^^^^^^^ use the alias
{
    tokio::spawn(async move {
        service.call(()).await // <--- OK!
    }).await
}
```

This trait alias setup means that users (and middleware like `LogService`) **always** write impls for `Service`. Functions that consume a service can choose to use `SendService` if they require `Send` bounds. Without RTN, the best that can be done is to have two distinct traits, which forces middleware like `LogService` to choose which they will implement (as previously discussed).

(This RFC is not advocating for a particular naming convention. We use `Service` and `SendService` to make clear that there is a base trait to which additional bounds are being added. For Tower specifically, based on discussion with Tokio team, the most likely final setup is to call the base trait `LocalService` and the `Send`-variant simply `Service`; this would mean that users would implement `LocalService` always. The [future directions](#future-directions) includes some ways to make the `LocalService`/`Service` convention more transparent for users.)

## Expected usage pattern: "Trait aliases" for the common cases, explicit RTN for the exceptions

Our expectation is that most traits will make use of `trait_variant` to define trait aliases like `SendService`. This provides the best experience for trait consumers, since they can conveniently bound all methods in the trait at once.

However, even when such an alias exists, there are times when trait consumers may not want to use them. Consider a trait like `Backend`:

```rust
#[trait_variant::make(SendBackend: Send)]
trait Backend {
    async fn get(&self, key: Key) -> Value;
    async fn put(&self, key: Key, value: Value);
}
```

While `SendBackend` may be convenient most of the time, it is also stricter than necessary for functions that only invoke one of `get` or `put`. Now consider two backend types, `B1` and `B2`, where `B1` always returns `Send` futures, but only `B2::put(..)` operation on `B2` is `Send`, because `B2::get(..)` makes use of `Rc` for caching purposes. In that case, a generic function with a bound like `Backend<put(..): Send>` could be used on both `B1` and `B2`.

## Design axioms

* **Minimal bounds in trait defintion, consumers apply the bounds they need.** Rust's typical pattern is to have traits with minimal bounds (e.g., `IntoIterator` declares only that its `IntoIter` type will be an `Iterator`) and then to have consumers apply additional bounds when they need them (e.g., that `IntoIter: DoubleEndedIterator`). This makes for widely reusable traits.
* **Just say "async fn".** We want simply writing `async fn foo(&self)` to result in a maximally reusable trait (just as it results in a maximally reusable free function today); "best practice" trait definitions should still be simple to read and should not limit the trait's consumers or future uses.
* **Support both async fn and `-> impl Trait`.** The most pressing user need is for send bounds on async fns, but we want to add a primitive that will also address the limitations of `-> impl Trait` methods (both in traits and, eventually, outside of them).

# Guide-level explanation
[guide-level explanation]: #guide-level-explanation

Async functions can be used in many ways. The most common configuration is to use a *work stealing* setup, in which spawned tasks may migrate between threads. In this case, all futures have to be `Send` to ensure that this migration is safe. But many applications prefer to use a *thread-per-core* setup, in which tasks, once spawned, never move to another thread (one important special case is where the entire application runs on a single thread to begin with, common in embedded environments but also in e.g. Google's Fuchsia operating system).

For the most part, async functions today do not declare whether they are `Send` explicitly. Instead, when a future `F` is spawned on a multithreaded executor, the compiler determines whether it implements `Send`. So long as `F` results from an `async fn` that only calls other `async fn`s, the compiler can analyze the full range of possible executions. But there are limitations, especially around calls to async trait methods like `f.method()`. If the type of `f` is either a generic type or a `dyn` trait, the compiler cannot determine which impl will be used and hence cannot analyze the function body to see if it is `Send`. This can result in compilation errors.

## Example: `HealthCheck` and `SendHealthCheck`

For traits whose futures may or may not be `Send`, the recommend pattern is to leverage the (rust-lang provided) `trait_variant` crate, which can automatically declare two versions of the trait. The default trait, `HealthCheck`, returns a future from each method; the alias `SendHealthCheck` is used to indicate those cases where all futures are known to be `Send`:

```rust
#[trait_variant::make(SendHealthCheck: Send)]
trait HealthCheck {
    async fn check(&mut self, server: &Server) -> bool;

    async fn shutdown(&mut self, server: &Server);
}
```

## Most code can reference `HealthCheck` directly

The `HealthCheck` trait can now be implemented normally.
This includes cases, like `DummyCheck`, where the returned future will always be `Send`:

```rust
struct DummyCheck;

impl HealthCheck for DummyCheck {
    async fn check(&mut self, server: &Server) -> bool {
        true
    }

    async fn shutdown(&mut self, server: &Server) {}
}
```

But also cases like `LogCheck`, which return a `Send` future if and only if their generic type argument returns a `Send` future:

```rust
struct LogCheck<HC: HealthCheck> {
    hc: HC,
}

impl<HC: HealthCheck> HealthCheck for LogCheck<HC> {
    async fn check(&mut self, server: &Server) -> bool {
        self.hc.check(server).await
    }

    async fn shutdown(&mut self, server: &Server) {
        self.hc.shutdown(server).await
    }
}
```

## Generic code that needs `Send` can use `SendHealthCheck`

When writing generic functions that spawn tasks, invoking async functions can lead to compilation failures:

```rust
fn start_health_check<HC>(health_check: H, server: Server)
where
    HC: HealthCheck + Send + 'static,
{
    tokio::spawn(async move {
        while health_check.check(&server).await {
            //             ----- Error: Returned future must
            //             be Send because this code runs.
            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        emit_failure_log(&server).await;

        server.shutdown().await;
        //     ----- Error: Returned future must be Send
        //           because this code runs.
    });
}
```

The problem is that `tokio::spawn` requires a `Send` future,
but the future returned by `health_check.check` is not guaranteed to be `Send`.
To address this, refall that the `HealthCheck` trait also used the `trait_variant::make` macro to create an alias, `SendHealthCheck`, that required all futures to be `Send`:

```rust
#[trait_variant::make(SendHealthCheck: Send)]
trait HealthCheck {...}
```

Therefore you can change the `HC: HealthCheck` bound to `HC: SendHealthCheck`,
the alias that requires all of its futures to be `Send`:

```rust
fn start_health_check<HC>(health_check: H, server: Server)
where
    HC: SendHealthCheck + 'static,
{
    ...
}
```

## Bounding specific methods

Trait aliases like `SendHealthCheck` require all the async methods in the trait to return a `Send` future.
Sometimes that is too strict.
For example, the following function spawns a task to shutdown the server:

```rust
fn spawn_shutdown<HC>(health_check: H, server: Server)
where
    HC: SendHealthCheck + 'static,
    //  --------------- stricter than necessary
{
    tokio::spawn(async move {
        server.shutdown().await;
    });
}
```

Because `spawn_shutdown` only invokes `shutdown`, using `SendHealthCheck` is stricter than necessary.
It may be that there are types where the `check` method does not return a `Send` future
but `shutdown` does.
In this case, you can write a bound that specifically applies to the future returned by the `shutdown()` method, like so:

```rust
fn spawn_shutdown<HC>(health_check: H, server: Server)
where
    HC: HealthCheck<shutdown(..): Send> + Send + 'static,
    //              ------------------ "just right"
{
    tokio::spawn(async move {
        server.shutdown().await;
    });
}
```

The `shutdown(..)` notation acts like an associated type referring to the return type of the method.
The bound `HC: HealthCheck<shutdown(..): Send>` indicates that the `shutdown` method,
regardless of what arguments it is given,
will return a `Send` future.
These bounds do not have to be written in the `HealthCheck` trait, it could also be written as follows:

```rust
fn spawn_shutdown<HC>(health_check: H, server: Server)
where
    HC: HealthCheck + Send + 'static,
    HC::shutdown(..): Send,
```

## Guidelines and best practices

### Authoring async traits

When defining an async trait (a trait with async functions), best practice is to define a "send variant" with the `trait_variant` crate:

```rust
#[trait_variant::make(SendMyTrait: Send)]
trait MyTrait {
    async fn method1(&self);
    async fn method2(&self);
}
```

Defining a "send alias" in this way has advantages for users of your trait:

* Referencing `T: SendMyTrait` is shorter than using RTN if there are multiple functions
    * (compare to `T: Send + Mytrait<method1(..): Send, method2(..): Send>`)
* Referencing `T: SendMyTrait` is more forwards compatible:
    * If you add a new method to your trait (with a default impl), all users of the send alias will be able to call this new method. Users that have named individual methods will not (on the flip side)

But defining a "send alias" in this way comes with obligations for you:

* If you add a new default method to your trait, it must be "Send-preserving" (meaning that it will be `Send` if other functions return `Send` futures).
    * *Why?* If there is an existing function that requires `T: SendMyTrait` for some type `T`, then this must remain true even when `MyTrait` grows a new (defaulted) method, or else you will have broken your downstream clients.
    * On the flip side, if you don't define an alias, you can add new defaulted methods that are not Send. This won't break downstream crates but neither will they be able to use them.

### Using async traits

When using a trait `MyTrait` that defines a sendable alias `SendMyTrait`...

* Implement `MyTrait` directly. Your type will implement `SendMyTrait` automatically if appropriate.
* Prefer `T: SendMyTrait` over a more explicit, method-by-method bound like `T: MyTrait<method1(..): Send, method2(..): Send>` *unless you specifically want to "opt-out" from requiring a particular method is `Send`.*
    * Using the alias is shorter, but it also means that if the trait grows new default methods, they will be included in the alias by default, allowing you to call them.

# Reference-level explanation
[reference-level explanation]: #reference-level-explanation

## Background and running examples

### The `Widgets` trait

Throughout this section we will make use of the `Widgets` trait as a simple running example.

```rust
trait Widgets {
    fn widgets(&self) -> impl Iterator<Item = Widget>;
}
```

### Background: desugaring to associated types

Per [RFC 3425][], the return-position `impl Trait` types that appear in `Widgets` and `Log` are desugared by the compiler into generic associated types, roughly as follows:

[RFC 3425]: https://github.com/rust-lang/rfcs/pull/3425

```rust
trait Widgets { // desugared
    type $Widgets<'a>: Iterator<Item = u32>;
    fn widgets(&self) -> Self::$Widgets<'_>;
}
```

These desugarings are not exposed to users, so the associated types `$Widgets` and `$Log` are not directly nameable,
but we will use it to define the semantics of Return Type Notation.

## Grammar

### Return type notation

Return Type Notation extends the type grammar roughly as follows,
where `?` indicates an optional nonterminal and `,*` indicates a comma
separated list. These changes permit `where T::method(..): Send`.

```ebnf
Type = i32
     | u32
     | ...
     | Type "::" AssociatedTypeName
     | "<" Type as TraitName Generics? ">" "::" AssociatedTypeName
     | ...
     | Type "::" MethodName "(" ".." ")" // <--- new
     | "<" Type as TraitName Generics? ">" "::" MethodName "(" ".." ")" // <--- new

Generics = "<" Generic,* ">"
Generic = Type | Lifetime | ...
```

Examples: given the `Widgets` trait defined earlier in this section...

* `T::widgets(..)` is a valid RTN that refers to "`widgets` invoked with any arguments"
* `<T as Widgets>::widgets(..)` is a valid RTN that refers to "`widgets` invoked with any arguments"

To support the `()` notation for `Fn` trait bounds (e.g., `T: Fn(u8)`), the Rust grammar already permits `T::method_name(T0, T1)` to be parsed as a type ([example](https://play.rust-lang.org/?version=nightly&mode=debug&edition=2021&gist=354ec7908a44619145d2ce8d5296a4a2)), but those examples will result in a compiler error in later phases. This RFC requires them to be interpreted as RTN types instead.

### Associated type bounds

[Associated type bounds](https://github.com/rust-lang/rust/issues/52662) are a recently stabilized feature that permits `T: Trait<Type: Foo>` to be used to bound an associated type `T::Type`. The grammar for these trait references is extended to support RTN notation in this position:

```ebnf
TraitRef = TraitName "<" Generic,* AssociatedBound ">"

AssociatedBound = Identifier "=" Generic
                | Identifier ":" TraitRef // (from RFC #2289)
                | Identifier "(" ".." ")" ":" TraitRef // <--- new
```

Examples: given the `Widgets` trait defined earlier in this section...

* `T: Widgets<widgets(..): Send>` is a valid associated type bound

RTN bounds are internally desugared to an RTN in a standalone where-clause,
so e.g. `where T: Widgets<widgets(..): Send>` becomes `where <T as Widgets>::widgets(..): Send`.
We will not consider them further in this section.

## Where RTN can be used (for now)

Although RTN types extend the type grammar, the compiler will not allow them to appear in all positions. Positions where RTN is currently supported include:

* As a standalone type, RTN can only be used as the `Self` type of a where-clause, e.g., `where W::widgets(..): Send`.
* As an associated type bound, RTN can be used where associated type bounds appear, e.g.,
    * `trait SendWidgets: Widgets<widgets(..): Send>`
    * `fn foo<W: Widgets<widgets(..): Send>>()`
    * `dyn Widgets<widgets(..): Send>`
    * `impl Widgets<widgets(..): Send>`

> *Nonnormative:* The current set of allowed locations correspond to places where generics on the method (e.g., `widgets(..)`) can be converted into higher-ranked trait bounds, as described in the next section. We expect [future RFCs](#future-possibilities) to extend the places where RTN can appear. These RFCs will detail how to manage generic parameters in those functions. The expectation is that the behavior will generally match "whatever `'_` would do". For example, `let w: W::widgets(..) = ...` would be equivalent to `let w: W::$Widgets<'_> = ...`.

## Converting to higher-ranked trait bounds

The method named in an RTN type may have generic parameters (e.g., `fn widgets<'a>(&'a self)` has a lifetime parameter `'a`). Because RTN locations are limited to where-clauses and trait bounds in this RFC, these parameters can always be captured in a `for` to form a [higher-ranked trait bound](https://rust-lang.github.io/rfcs/0387-higher-ranked-trait-bounds.html).

The semantics are illustrated by the following examples which desugar references to `widgets(..)` into the (generic) associated type `$Widgets<'_>` described earlier:

* `<T as Widgets>::widgets(..): Send`
    * `where for<'a> <T as Widgets>::$Widgets<'a>: Send`
* `T: Widgets<widgets(..): Send`
    * Equivalent to `where T: Widgets<for<'a> $Widgets<'a>: Send>`
* `impl Widgets<widgets(..): Send>`
    * `impl for<'a> Widgets<$Widgets<'a>: Send>`
* `dyn Widgets<widgets(..): Send`
    * `dyn for<'a> Widgets<$Widgets<'a>: Send>`
    * But note that async fn and RPITIT are not yet dyn-safe; this is forward looking.

While all of these examples are using lifetimes, there is ongoing work to support higher-ranked trait bounds that are generic over types, and the expectation is that RTN will be extended to work over generic types and constants when possible.

### How this is implemented

The examples above illustrate the semantics but do not make clear how RTN can be implemented in the compiler. A RTN bound like `widgets(..)` is implemented internally via unification. To keep the RFC focused on how RTN feels to users, we defer a detailed description to reference material and a future stabilization report.

## RTN only applies to AFIT and RPITIT methods

Although conceptually RTN could be used for any trait method, we choose to limits its use to `async fn` and other methods that directly return an `-> impl Trait`. This limitation can be lifted in the future as we gain more experience.

* RTN may refer to the following examples:
    * `async fn method(&self)`
    * `fn method(&self) -> impl Iterator<Item = u32>`
* RTN may not presently refer to the following examples:
    * `fn method(&self) -> u32`
    * `fn method(&self) -> Option<impl Iterator<Item = u32>>`

# Drawbacks
[drawbacks]: #drawbacks

## Confusion about future type vs awaited type

When writing an async function, the future is implicit:

```rust
trait HealthCheck {
    async fn check(&mut self, server: Server);
}
```

It could be confusing that `HC::check(..)` refers to a future and not the `()` type that results from await. This is however consistent with expressions (i.e., `let c = hc.check(..)` will yield a future, not the result).

## Automatic impl of `Send` based on current method definition

Implementations of async functions automatically expose whether they are `Send` or not, limiting their future (semver-compatible) evolution. E.g., the following impl...

```rust
impl HealthCheck for MyType {
    async fn check(&mut self, server: Server) {
        return;
    }
}
```

...could not in the future be modified to reference an `Rc` internally. This is different from ordinary functions which can add references to `Rc`  transiently without an issue.

The fact that the `Send` requirement limits what values async functions can internally reference is not new, however, nor specific to trait functions.
It is a consequence of existing precedent:

* Async functions desugar to returning an `impl Future` value.
* Values are automatically `Send` based on their contents.

# Rationale and alternatives
[rationale and alternatives]: #rationale-and-alternatives

## What is the impact of not doing this?

The Async Working Group has performed [five case studies][cs] around the use of async functions in trait, covering usage in the following scenarios:

[cs]: https://rust-lang.github.io/async-fundamentals-initiative/evaluation/case-studies.html

* configuration and parameterization in the AWS SDK, such as providing a generic credentials provider ([link][awssdkcs]);
* redefining the `Service` trait defined by `tower` ([link][towercs]);
* usage in the Fuchsia Netstack3 socket handler developed at Google ([link][fuchsiacs]);
* usage in an internal Microsoft application ([link][msftcs]);
* usage in the embedded runtime [`embassy`], which targets simple processors without an operating system ([link][embassycs]).

[`embassy`]: https://github.com/embassy-rs/embassy
[awssdkcs]: https://rust-lang.github.io/async-fundamentals-initiative/evaluation/case-studies/builder-provider-api.html
[fuchsiacs]: https://rust-lang.github.io/async-fundamentals-initiative/evaluation/case-studies/socket-handler.html
[towercs]: https://rust-lang.github.io/async-fundamentals-initiative/evaluation/case-studies/tower.html
[msftcs]: https://rust-lang.github.io/async-fundamentals-initiative/evaluation/case-studies/microsoft.html
[embassycs]: https://rust-lang.github.io/async-fundamentals-initiative/evaluation/case-studies/embassy.html

We found that all of these key use cases required a way to handle send bounds, with only two exceptions:

* `embassy`, where the entire process is single-threaded (and hence `Send` is not important),
* Fuchsia, where the developers at first thought they needed `Send` bounds, but ultimately found they were able to refactor so that spawns did not occur in generic code ([link to the relevant section](https://rust-lang.github.io/async-fundamentals-initiative/evaluation/case-studies/socket-handler.html#send-bound-limitation)).

From this we conclude that offering async functions in traits without *some* solution to the "send bound problem" means it will not be usable for most Rust developers. The Fuchsia case also provides evidence that, even when workarounds exist, they are not obvious to Rust developers.

For most of the cases above, return-type notation as described in this RFC worked well. The major exception was the Microsoft application, which included a trait with many methods. Since doing this study we have developed the [trait-variant][] crate and thus the ability to define "send aliases", as described in this RFC, which addresses this ergonomic gap.

## How did you settle on this particular design?

The goal of this RFC is offer a

* **flexible primitive** that can support many use cases (including constructing aliases)
* and which is **ergonomic enough** to be useful directly when needed.

The primitive alone doesn't fill all needs as it doesn't address the need to create aliases,
but it provides the means for the `#[trait_variant::make]` procedural macro to be written as a stable crate;
in the future providing a more ergonomic syntax -- such as [trait transformers](#why-not-send-trait-transformers) -- for "all async functions return send futures" may be worthwhile.

## What are cases where that flexibility is useful?

Versus aliases that always bound every method, RTN can be used to

* bound individual methods
* introduce bounds for traits other than `Send`.

As [described in the motivation](#bounding-specific-methods), bounding individual methods allows for greater reuse.
For functions that only make use of a subset of the methods in a trait, RTN can be used to create a "maximally reusable" signature.

## What other syntax options were considered?

The lang team held a design meeting [reviewing RTN syntax options](https://hackmd.io/@rust-lang-team/ByUojGAn6) and covering the pros/cons for each of them in detail. The document also includes a detailed [evaluation and recommendations](https://hackmd.io/KPRLXXmISoWgX38alWUEnA?view#Evaluation-and-recommendation).

The document reviewed the following designs overall:

| Option | Bound |
| --- | --- |
| StatusQuo | `D: Database<items(): DoubleEndedIterator>` |
| DotDot | `D: Database<items(..): DoubleEndedIterator>` |
| Return | `D: Database<items::return: DoubleEndedIterator>` |
| Output | `D: Database<items::Output: DoubleEndedIterator>` |
| Fn | `D: Database<fn items(): DoubleEndedIterator>` |
| FnDotDot | `D: Database<fn items(..): DoubleEndedIterator>` |
| FnReturn | `D: Database<fn items::return: DoubleEndedIterator>` |
| FnOutput | `D: Database<fn items::Output: DoubleEndedIterator>` |

We briefly review the key arguments here:

* "StatusQuo": `D: Database<items(): DoubleEndedIterator>`
    * This notation is more concise and feels less heavy-weight. However, we expect users to primarily use aliases; also, the syntax "feels" surprising to many users, since Rust tends to use `..` to indicate elided items. The biggest concern here is a potential future conflict. If we (a) extend the notation to allow argument types to be specified ([as described in the future possibilities section](#future-possibilities)) AND (b) support some kind of variadic arguments, then `D::items()` would most naturally indicate "no arguments".
* "Return": `D: Database<items::return: DoubleEndedIterator>`
    * This notation avoids looking like a function call. Many team members found it dense and difficult to read. While intended to look more like an associated type, the use of a lower-case keyword still makes it feel like a new thing. The syntax does not support future extensions (e.g., specifying the value of argument types).
* "Output": `D: Database<items::Output: DoubleEndedIterator>` (see [this blog post](https://smallcultfollowing.com/babysteps/blog/2023/06/12/higher-ranked-projections-send-bound-problem-part-4/) for details)
    * This reuses associated types but, as both the function and future traits define an `Output` associated type, raises the potential for confusion about whether this notation means "the future that gets returned" or "the result of the future".
* "FnDotDot" and friends: `D: Database<fn items(..): DoubleEndedIterator>`
    * This notation was deemed too close to `fn` pointer types, particularly in stand-alone where-clauses.

## Why not use `typeof`, isn't that more general?

The compiler currently supports a `typeof` operation as an experimental feature (never RFC'd). The idea is that `typeof <expr>` type-checks `expr` and evaluates to the result of that expression. Therefore `typeof 22_i32` would be equivalent to `i32`, and `typeof x` would be equivalent to whatever the type of `x` is in that context (or an error if there is no identifier `x` in scope).

It might appear that `typeof`  can be used in a similar way to RTN, but in fact it is significantly more complex. Consider our first example, the `HealthCheck` trait:

```rust
trait HealthCheck {
    async fn check(&mut self, server: Server);
}
```

and a function bounding it

```rust
fn start_health_check<H>(health_check: H, server: Server)
where
    H: HealthCheck + Send + 'static,
    H::check(..): Send, // <--- How would we write this with `typeof`?
```

To write the above with `typeof`, you would do something like this

```rust
fn dummy<T>() -> T { panic!() }

fn start_health_check<H>(health_check: H, server: Server)
where
    H: HealthCheck + Send + 'static,
    for<'a> typeof H::check(
        dummy::<&'a mut H>(),
        dummy::<Server>(),
    ): Send,
```

Alternatively, one could write something like this

```rust
fn start_health_check<H>(health_check: H, server: Server)
where
    H: HealthCheck + Send + 'static,
    typeof {
        let hc: &'a mut H;
        let s: Server;
        H::check(hc, s)
    }: Send,
```

Note that we had to supply a callable expression (even if it will never execute), so we can't directly talk about the types of the arguments provided to `H::check`, instead we have to use the `dummy` function to produce a fake value of the type we want or introduce dummy let-bound variables.

Clearly, `typeof` on its own fails the "ergonomic enough to use for simple cases" threshold we were shooting for. But it's also a significantly more powerful feature that introduces a *lot* of complications. We were able to implement a minimal version of RTN in a few days, demonstrating that it fits relatively naturally into the compiler's architecture and existing trait system. In contrast, integrating `typeof` would be rather more complicated. To start, we would need to be running the type checker in new contexts (e.g., in a where clause) at large scale in order to normalize a type like `typeof H::check(x, y)` into the final type it represents.

With `typeof`, one would also expect to be able to reference local variables and parameters freely. This would bring Rust full on into dependent types, since one could have a variable whose type is something like `typeof x.method_call()`, which is clearly dependent on the type of `x`. This isn't an impossible thing to consider -- and indeed the same could be true of some extensions of RTN, if we chose to permit naming closures or other local variables -- but it's a significant bundle of work to sort it out.

Finally, while `typeof` clearly is a more general feature, it's not clear how well motivated that generality is. The main use cases we have in mind are more naturally and directly handled by RTN. To justify `typeof`, we'd want to have a solid rationale of use cases.

## Why not make *all* futures `Send`?

The `#[async_trait]` macro solves the send bounds problem by forcing the trait to declare up front whether it will require send or not. This is required by the desugaring that async-trait uses. For many users, this is a fine solution, since they always work with sendable futures. But there are a significant set of users that do not want send bounds, either because they are in an embedded context or because they are using a thread-per-core architecture. The widely used tokio runtime, for example, can be configured to either use work-stealing (which requires `Send` futures) or to be a single-threaded executor (which does not). The `glommio` executor does not require `Send` bounds on futures because it never moves tasks between threads. The Fuchsia project makes extensive use of single-threaded executors in their runtime, and hence they do not require `Send` bounds. The `embassy` runtime targets embedded environments that only have a uniprocessor and which have no need for `Send` bounds. All of these environments are disadvantaged by defaults that require send bounds.

One of our design goals with async-trait is to support core interoperability traits for things like reading, writing, HTTP, etc. The whole point of these traits is to be usable across many runtimes. If those traits forced `Send` bounds, that would be unnecessarily limiting, which would lead to users of non-Send-requiring runtimes to avoid them. If the traits did NOT force `Send` bounds, they would not be compatible with work stealing runtimes (the most popular choice) unless there was some additional feature to "opt-in" to needing send bounds -- which is exactly the gap RTN is looking to close.

## Why not create an associated type that represents the return type?

Early on in our design work, we expected to simply create an associated type within the trait to represent the return type. For example this trait:

```rust
trait Factory {
    fn widgets(&self) -> impl Iterator<Item = Widget>;
}
```

might have been desugared as follows:

```rust
trait Factory {
    type widgets<'a>: Iterator<Item = Widget>; // <--- implicitly introduced
    fn widgets(&self) -> Self::widgets<'_>;
}
```

This would mean that users could write a bound on `F::widgets` to bound the return type of `widgets`

```rust
fn use_factory<F>()
where
    F: Factory,
    for<'a> F::widgets<'a>: Send,
{}
```

We encountered a number of problems with this design.

### If the name is implicit, what name should we use?

The most impmediate problem with this proposal was trying to decide what name to use.

Using `Widgets` (capitalized) feels arbitrary and there is no precedent within Rust for automatically creating names with different case conventions in this way.

Using the same name as the method (`widgets`) results in an associated type that does not follow Rust's naming conventions.
It also introduces the potential for a shadowing conflict as today it is allowed to have methods and associated types with the same name:

```rust
trait Example {
    type method;
    fn method();
}
```

### Why not use an explicit name?

To address the challenge of an implicit name, we could allow people to explicitly annotate a name:

```rust
trait Factory {
    #[associated_return_type(Widgets)]
    fn widgets(&self) -> impl Iterator<Item = Widget>;
}
```

However, this has some downsides:

1. It goes against our design axiom that people should be able to "just write `async fn`".
   Now for maximum reuse the trait body requires extra annotations.
2. It means that trait authors must remember to add such an annotation or else their consumers will be limited in their ability to use the trait.
   Trait authors should expect a stream of PRs adding this annotation to most every `async fn` in their trait.

### What generic parameters should this associated type have?

Regardless of how it is named, it's not obvious what set of generic type parameters the function should have. In our example, there was only a single lifetime, but in other cases, functions can have a large number of implicit parameters. This occurs with anonymous lifetimes but also with argument-position impl Trait. We have so far avoided committing to a particular order or way of specifying those implicit parameters explicitly, but desugaring to a (user-visible) generic associated type would force us to make a commitment. Example:

```rust
trait Consumer {
    fn consume_elements(&mut self, context: &Context, widgets: &mut impl Iterator<Item = Widget>);
}
```

How many generic type parameters should `consume_elements` have, and in what order? There are at least three anonymous lifetimes mentioned, and one anonymous type parameter (the `impl Iterator`), but that's not enough to answer the question. First off, without seeing the definitions of `Context` and `Widget`, we do not know if they have lifetime parameters (although it's discouraged, Rust permits you to elide lifetime parameters from structs in function declarations). Second, all of the lifetime parameters we see appear in "variant" positions, and so we could get away with a single GAT parameter (simpler). But if (for example) `Context` were defined like so:

```rust
struct Context<'a> {
    x: &'a mut Vec<&'a u32>
}
```

then the function would require a separate lifetime parameter for `Context`. Committing to specific rules here limits us as language designers, but it's also a demands a deep understanding of the compiler and its desugaring to be successfully used and explained.

## Why not use a named associated type that represents the zero-sized method type?

In the previous question, we mentioned that every function in Rust has a unique zero-sized type associated with it, including methods. One natural desugaring then might be to introduce an associated type that represents the method type itself. One could then use the `Output` associated type to talk about the return type. Given the `Factory` trait we saw before:

```rust
trait Factory {
    fn widgets(&self) -> impl Iterator<Item = Widget>;
}
```

one might then take "any factory whose widgets iterator is sendable" like this:

```rust
fn use_factory<F>()
where
    F: Factory,
    for<'a> F::widgets<'a>::Output: Send,
    //      --------------  ------
    //      type of the     return type
    //      method
{}
```

This approach has an appealing generality to it, and it opens up some interesting possibilities. For example, one might consider a trait `Const` that is implemented by all function types which are `const fn` (discussed in withoutboats's [const as an auto trait][caa] blog post). Users could then write `for<'a> F::widgets<'a>: Const` to declare that the method is a const method. However, it's rather unergonomic for the common case. It also doesn't compose well with the associated type bounds notation -- i.e., would we write something like `F: Factory<widgets::Output: Send>`?

To resolve the ergonomic problems, our exporations of this future wound up proposing some form of sugar to reference the `Output` type -- for example, being able to write `F::widgets(..): Send`. But that is precisely what this RFC proposes! Indeed, in the [future possibilities][] section of the RFC, we discuss the possibility of giving users some way to name the type of the `widgets` method itself, and not just its return type.

So why not just start with this more general approach, if we think it might be a useful extension? First, it's not clear if it would be useful. We don't have to solve the question of "const as an auto trait" in order to address the send bounds problem. Second, this approach suffers from some of the complications mentioned in the previous question, such as needing to specify the order of arguments for anonymous lifetime or impl trait parameters, and having to deal with existing traits that may shadow the desired name. Lacking a strong motivation to have this much generality, it's hard to tell how to resolve those questions, since we don't really know where/when this more general form would be used.

[caa]: https://without.boats/blog/const-as-an-auto-trait/

## Why not make `trait_variant` crate magic?

With RTN, the `#[trait_variant::make]` macro can be defined in "user space".
It would also be possible to build it into the stdlib and have it defined "magically" through compiler intrinsics.
This would still allow async traits to be defined that can be used across all executors
(in roughly the same way as we recommend),
but it has several downsides.
First, it makes the stdlib more special, which works against the goals of Rust.
Second, it covers far fewer use cases than RTN: it cannot be used to express specifically which methods must be `Send`, nor can it be used for traits that were not "pre-imagined" by the trait author.

## Why not Send trait transformers?

[Trait transformers][] are a proposal to have "modifiers" on trait bounds that produce a derived version of the trait. For example, `T: async Iterator` might mean "T implements a version of `Iterator` where the `next` function is `async`". Following this idea, one can imagine `T: Send HealthCheck` to mean "implement a version of `HealthCheck` where every async fn returns a `Send` future". This idea is an ergonomic way to manage traits that have a lot of async functions, as [came up in the Microsoft case study](https://rust-lang.github.io/async-fundamentals-initiative/evaluation/case-studies/microsoft.html#send-bounds).

[Trait transformers]: https://smallcultfollowing.com/babysteps/blog/2023/03/03/trait-transformers-send-bounds-part-3/

It seems likely that trait transformers would be more ergonomic than RTN in practice, since they easily accommodate traits with many async functions. However, they are less flexible, as the current idea can only encode the case where you want to add the same auto trait to the return type of all async functions, whereas RTN can be used to encode all manner of patterns, as described in the [guide-level explanation][]. Furthermore, trait transformers are a more fundamental extension to Rust than RTN, and their design is tied up in questions of whether we should have other kinds of transformers, such as `async` or `const`. It is preferable to give time for exploration until we have a better handle on the motivation and use cases so that we can avoid constraining ourselves today in a way that we might not want in the future. In contrast, it's hard to imagine a future where we don't want *some* way to constrain or refer to the return types of individual methods within a trait.

# Prior art
[prior-art]: #prior-art

## C++

C++ has [`decltype`](https://en.cppreference.com/w/cpp/language/decltype) expressions which give the type of an expression and the type of a declaration, respectively. Some compilers (e.g., GCC) also support [`typeof`](https://gcc.gnu.org/onlinedocs/gcc/Typeof.html). The [drawbacks][] section listed reasons why we believe `typeof` is not a suitable primitive for us to build upon.

# Unresolved questions
[unresolved questions]: #unresolved-questions

## Does stabilizing `T::foo(..)` notation as a standalone type create a confusing inconsistency with `-> ()` shorthand?

Unlike a regular associated type, this RFC does not allow a trait bound that specifies the return type of a method, only the ability to put bounds on that return type.
rpjohnst suggested that we may wish to support a syntax like `T: Trait<method(..) -> T>`, perhaps in conjunction with specified argument types.
They further pointed out that permitting `T::method(..)` as a standalone type could be seen as inconsistent, given that `fn foo()` is normally shorthand for `-> ()`.
However, *not* supporting `T::method(..)` as a standalone type could also be seen as inconsistent, since normally `T: Trait<Bar: Send>` and `T::Bar: Send` are equivalent.
Prior to stabilizing the "associated type position" syntax, we should be sure we are comfortable with this.


# Future possibilities
[future possibilities]: #future-possibilities

## Implementing trait aliases

Referring to the `Service` trait specifically,
the Tokio developers expressed a preference to name the "base trait" `LocalService`
and to call the "sendable alias" `Service`.
This reflects the way that Tokio uses work-stealing executors by default.
This formulation can be done with `trait_variant` like so

```rust
#[trait_variant::make(Service: Send)]
trait LocalService<R> {
    type Response;

    async fn call(&self, request: R) -> Self::Response;
}
```

However, it carries the downside that users must implement `LocalService` and hence must be aware of the desugaring.
It would be nicer if users could choose to implement `Service` and then (in so doing) effectively assert that all their async functions are *always* `Send`.
This is not possible today due to the fact that `trait-variant` is emulating trait alias functioanlity with a blanket impl and supertraits; this is because true trait alias functionality is not yet stable.
[RFC 3437][] has proposed an extension to trait aliases that makes them implementable.
The combination of accepting [RFC 3437][] and stabilizing trait aliases would make these aliases nicer for users as a result.

[RFC 3437]: https://github.com/rust-lang/rfcs/pull/3437

## Permit RTN for more functions

RTN is currently limited to `async fn` and `-> impl Trait` methods in traits.
But the same syntax could be used for any methods as well as for free functions (e.g., `foo(..)` might refer to the return type of `fn foo()`).
One area that would be challenging to support is RTN for the return types of closures,
as that would introduce an element of dependent types that would complicate the type checker
(e.g., if `let y: x(..)` meant that `y` is the type returned from invoking the closure `x`, another local variable).

## Specifying the values for argument types

The `T::method(..): Send` notation we've been using so far means
"the return type of `method(..)` is `Send`, no matter what arguments you provide".
We could extend this notation to permit specifying the argument types explicitly.
For example, consider the `capture` method below, which takes a parameter of type `input`:

```rust
trait Capture {
    async fn capture<T>(&mut self, input: T) -> Option<T>;
}
```

and now consider a function that invokes `capture` with an `i32` value:

```rust
async fn capture_i32<C: Capture>(mut c: C) {
    c.capture(22_i32);
}
```

Now imagine we wanted to invoke `capture` on another thread,
and hence we need a where-clause indicating that the future
returned by `capture` will be `Send`:

```rust
async fn capture_i32<C: Capture + Send + 'static>(mut c: C)
where
    /* where-clause for C::check() needed here! */
{
    workstealing_runtime::spawn(async move {
        c.capture(22_i32);
    })
}
```

There are multiple ways we could write this where-clause, varying in their specificity...

* `where C::capture(..): Send` -- this indicates that `C::capture()` will return a `Send` value for any possible set of parameters
* `where C::capture(&mut C, i32): Send` -- this indicates that `C::capture()` will return a `Send` value when invoked specifically on a `&mut C` (for the `self` parameter) and an `i32`
* `where for<'a> C::capture(&'a mut C, i32): Send` -- same as the previous rule, but with the higher-ranked `'a` written explicitly
* `where C::capture::<i32>(..): Send` -- this indicates that `C::capture()` will return a `Send` value for any possible set of parameters, but with its `T` parameter set explicitly to `i32`
* `where C::capture::<i32>(&mut C, i32): Send` -- this indicates that `C::capture()` will return a `Send` value when its `T` parameter is `i32`
* `where for<'a> C::capture::<i32>(&'a mut C, i32): Send` -- same as the previous rule, but with the higher-ranked `'a` written explicitly

Possible rules for an RTN are as follows:

* Parameter types:
    * If parameter types are specified as `..` (e.g., `C::check(..)` or `C::check::<i32>(..)`), then the where-clause applies to any possible argument types
    * If parameter types are given, then the where-clause applies to those specific argument types
        * the `self` type must be given explicitly when using `C::check(..)` notation, just as it would in a function call (e.g., `let x = C::check(a, b)`)
        * elided lifetimes like (e.g., `C::check(&mut Self, i32)`) are translated to a higher-ranked lifetime (e.g., `for<'a> C::check(&'a mut Self, i32)`) covering the where-clause
* Turbofish:
    * If turbofish is not used, then the where-clause applies to any possible values for the type parameters
    * If turbofish is used, then the values for the type parameters are explicitly specified

## Supporting RTN in more locations

To contain the scope, this RFC only describes how RTN types work as the self type of a where-clause.
However, one advantage of RTNs is that they can be extended to work in more places.
This would address the gap that has existed in `-> impl Trait` (and hence in `async fn`) since it was introduced in [RFC 1522](./1522-conservative-impl-trait.md),
namely that there is no way to name the return type of such a function explicitly.
This in turn means that given a function like `fn odd_integers() -> impl Iterator<Item = u32>`, one cannot name
the iterator type that is returned.
For free functions, best practice today is to use a named return type; once [type alias impl trait](./2071-impl-trait-type-alias.md) is stabilized, that will also be an option.
But neither of these are practical for async functions that appear in traits.

RTN as specified in this RFC could be extended with relative ease to appear in any location where `'_` is accepted. For example:

```rust
trait DataFactory {
    async fn load(&self) -> Data;
}

fn load_data<D: DataFactory>(data_factory: D) {
    let load_future: D::load(..) = data_factory.load();
    //               -------
    //   Expands to `D::load(&'_ D)` -- in this context,
    //   `'_` means that the compiler will infer a suitable
    //   value.
    await_future(load_future);
}

fn await_future<D: DataFactory>(load_future: D::load(..)) -> Data {
    //                                       -------
    //                      As above, expands to `D::load(&'_ D)`, which
    //                      means "for some `_`".
    argument.await
}
```

The most useful place to use RTN, however, is likely struct fields, and in that location we do not accept `'_`.
We would therefore have to support specifying the types of arguments in RTN.
That would enable writing structs that wrap the future returned via some trait method:

```rust
struct Wrap<'a, D: DataFactory> {
    load_future: D::load(&'a D), // the future returned by `D::load`.
}
```

## Dyn support

We expect to  make traits with async functions and RPITIT dyn safe in the future. One benefit of the RTN design is that it continues to hide the presence and precise value of the associated types that define the return value of an async function. This means that given `HealthCheck`, we can later define the type of the future `<dyn HealthCheck>::check(..)` to be anything.

## Naming the zero-sized types for a method

Every function and method `f` in Rust has a corresponding zero-sized type that uniquely identifies `f`. The RTN notation `T::check(..)` refers to the return value of `check`; conceivably `T::check` (without the parens) could be used to refer the type of `check` itself. In this case, `T::check(..)` can be thought of as shorthand for `<T::check as Fn<_>>::Output`.
