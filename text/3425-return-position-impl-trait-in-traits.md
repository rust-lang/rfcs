- Feature Name: return_position_impl_trait_in_traits
- Start Date: 2023-04-27
- RFC PR: [rust-lang/rfcs#3425](https://github.com/rust-lang/rfcs/pull/3425)
- Rust Issue: [rust-lang/rust#91611](https://github.com/rust-lang/rust/issues/91611)
- Initiative: [impl trait initiative](https://github.com/rust-lang/impl-trait-initiative)

# Summary
[summary]: #summary

* Permit `impl Trait` in fn return position within traits and trait impls.
* Allow `async fn` in traits and trait impls to be used interchangeably with its equivalent `impl Trait` desugaring.
* Allow trait impls to `#[refine]` an `impl Trait` return type with added bounds or a concrete type.[^refine]

# Motivation
[motivation]: #motivation

The `impl Trait` syntax is currently accepted in a variety of places within the Rust language to mean "some type that implements `Trait`" (for an overview, see the [explainer] from the impl trait initiative). For function arguments, `impl Trait` is [equivalent to a generic parameter][apit] and it is accepted in all kinds of functions (free functions, inherent impls, traits, and trait impls).

In return position, `impl Trait` [corresponds to an opaque type whose value is inferred][rpit]. This is necessary for returning unnameable types, like those created by closures and `async` blocks, and also a convenient way to avoid naming complicated types like nested iterators. In return position, `impl Trait` is currently accepted only in free functions and inherent impls. This RFC extends the support to traits and trait impls.

[explainer]: https://rust-lang.github.io/impl-trait-initiative/explainer.html
[apit]: https://rust-lang.github.io/impl-trait-initiative/explainer/apit.html
[rpit]: https://rust-lang.github.io/impl-trait-initiative/explainer/rpit.html

## Example use case

The use case for `-> impl Trait` in trait functions is similar to its use in other contexts: traits often wish to return "some type" without specifying the exact type. As a simple example that we will use through the RFC, consider the `NewIntoIterator` trait, which is a variant of the existing `IntoIterator` that uses `impl Iterator` as the return type:

```rust
trait NewIntoIterator {
    type Item;
    fn into_iter(self) -> impl Iterator<Item = Self::Item>;
}
```

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

*This section assumes familiarity with the [basic semantics of impl trait in return position][rpit].*

When you use `impl Trait` as the return type for a function within a trait definition or trait impl, the intent is the same: impls that implement this trait return "some type that implements `Trait`", and users of the trait can only rely on that.

<!--However, the desugaring to achieve that effect looks somewhat different than other cases of impl trait in return position. This is because we cannot desugar to a type alias in the surrounding module; we need to desugar to an associated type (effectively, a type alias in the trait).-->

Consider the following trait:

```rust
trait IntoNumIterator {
    fn into_int_iter(self) -> impl Iterator<Item = u32>;
}
```

The semantics of this are analogous to introducing a new associated type within the surrounding trait;

```rust
trait IntoNumIterator { // desugared
    type IntoNumIter: Iterator<Item = u32>;
    fn into_int_iter(self) -> Self::IntoNumIter;
}
```

When using `-> impl Trait`, however, there is no associated type that users can name.

By default, the impl for a trait like `IntoNumIterator` must also use `impl Trait` in return position.

```rust
impl IntoNumIterator for Vec<u32> {
    fn into_int_iter(self) -> impl Iterator<Item = u32> {
        self.into_iter()
    }
}
```

It can, however, give a more specific type with `#[refine]`:[^refine]

```rust
impl IntoNumIterator for Vec<u32> {
    #[refine]
    fn into_int_iter(self) -> impl Iterator<Item = u32> + ExactSizeIterator {
        self.into_iter()
    }

    // ..or even..

    #[refine]
    fn into_int_iter(self) -> std::vec::IntoIter<u32> {
        self.into_iter()
    }
}
```

Users of this impl are then able to rely on the refined return type, as long as the compiler can prove this impl specifically is being used. Conversely, in this example, code that is generic over the trait can only rely on the fact that the return type implements `Iterator<Item = u32>`.

### async fn desugaring

`async fn` always desugars to a regular function returning `-> impl Future`. When used in a trait, the `async fn` syntax can be used interchangeably with the equivalent desugaring in the trait and trait impl:

```rust
trait UsesAsyncFn {
    // Equivalent to:
    // fn do_something(&self) -> impl Future<Output = ()> + '_;
    async fn do_something(&self);
}

// OK!
impl UsesAsyncFn for MyType {
    fn do_something(&self) -> impl Future<Output = ()> + '_ {
        async {}
    }
}
```
```rust
trait UsesDesugaredFn {
    // Equivalent to:
    // async fn do_something(&self);
    fn do_something(&self) -> impl Future<Output = ()> + '_;
}

// Also OK!
impl UsesDesugaredFn for MyType {
    async fn do_something(&self) {}
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Equivalent desugaring for traits

Each `-> impl Trait` notation appearing in a trait fn return type is effectively desugared to an anonymous associated type. In this RFC, we will use the placeholder name `$` when illustrating desugarings and the like.

As a simple example, consider the following (more complex examples follow):

```rust
trait NewIntoIterator {
    type Item;
    fn into_iter(self) -> impl Iterator<Item = Self::Item>;
}

// becomes

trait NewIntoIterator {
    type Item;

    type $: Iterator<Item = Self::Item>;

    fn into_iter(self) -> <Self as NewIntoIterator>::$;
}
```

## Equivalent desugaring for trait impls

Each `impl Trait` notation appearing in a trait impl fn return type is desugared to the same anonymous associated type `$` defined in the trait along with a function that returns it. The value of this associated type `$` is an `impl Trait`.

```rust
impl NewIntoIterator for Vec<u32> {
    type Item = u32;

    fn into_iter(self) -> impl Iterator<Item = Self::Item> {
        self.into_iter()
    }
}

// becomes

impl NewIntoIterator for Vec<u32> {
    type Item = u32;

    type $ = impl Iterator<Item = Self::Item>;

    fn into_iter(self) -> <Self as NewIntoIterator>::$ {
        self.into_iter()
    }
}
```

The desugaring works the same for provided methods of traits.

## Scoping rules for generic parameters

We say a generic parameter is "in scope" for an `impl Trait` type if the actual revealed type is allowed to name that parameter. The scoping rules for return position `impl Trait` in traits are the same as [those for return position `impl Trait` generally][scoping]: All type and const parameters are considered in-scope, while lifetime parameters are only considered in-scope if they are mentioned in the `impl Trait` type directly.

Formally, given a trait method with a return type like `-> impl A + ... + Z` and an implementation of that trait, the hidden type for that implementation is allowed to reference:

* Concrete types, constant expressions, and `'static`
* Any generic type and const parameters in scope, including:
    * Type and const parameters on the impl
    * Explicit type and const parameters on the method
    * Implicit type parameters on the method (argument-position `impl Trait` types)
* Lifetime parameters that appear anywhere in the `impl A + ... + Z` type, including elided lifetimes

[scoping]: https://rust-lang.github.io/rfcs/1951-expand-impl-trait.html#scoping-for-type-and-lifetime-parameters

Lifetime parameters not in scope may still be indirectly named by one of the type parameters in scope.

_Note_: The term "captured" is sometimes used as an alternative to "in scope".

When desugaring, captured parameters from the method are reflected as generic parameters on the `$` associated type. Furthermore, the `$` associated type brings whatever where clauses are declared on the method into scope, excepting those which reference parameters that are not captured.

This transformation is precisely the same as the one which is applied to other forms of `-> impl Trait`, except that it applies to an associated type and not a top-level type alias.

Example:

```rust
trait RefIterator for Vec<u32> {
    type Item<'me>
    where
        Self: 'me;

    fn iter<'a>(&'a self) -> impl Iterator<Item = Self:Item<'a>>;
}

// Since 'a is named in the bounds, it is captured.
// `RefIterator` thus becomes:

trait RefIterator for Vec<u32> {
    type Item<'me>
    where
        Self: 'me;

    type $<'a>: impl Iterator<Item = Self::Item<'a>>
    where
        Self: 'a; // Implied bound from fn

    fn iter<'a>(&'a self) -> Self::$<'a>;
}
```

## Validity constraint on impls

Given a trait method where `impl Trait` appears in return position,

```rust
trait Trait {
    fn method() -> impl T_0 + ... + T_m;
}
```

where `T_0 + ... + T_m` are bounds, for any impl of that trait to be valid, the following conditions must hold:

* The return type named in the corresponding impl method must implement all bounds `T_0 + ... + T_m` specified in the trait.
    * This must be proven using only the information in the signature, with the exception that if the impl uses `impl Trait` syntax for the return type, the usual auto trait leakage rules apply.
* Either the impl method must have `#[refine]`,[^refine] OR
    * The impl must use `impl Trait` syntax to name the corresponding type, and
    * The return type in the trait must implement all bounds `I_0 + ... + I_n` specified in the impl return type. (Taken with the first outer bullet point, we can say that the bounds in the trait and the bounds in the impl imply each other.)

[^refine]: `#[refine]` was added in [RFC 3245: Refined trait implementations](https://rust-lang.github.io/rfcs/3245-refined-impls.html). This feature is not yet stable. Examples in this RFC requiring the use of `#[refine]` will not work until that feature is stabilized.

```rust
trait NewIntoIterator {
    type Item;
    fn into_iter(self) -> impl Iterator<Item = Self::Item>;
}

// OK:
impl NewIntoIterator for Vec<u32> {
    type Item = u32;
    fn into_iter(self) -> impl Iterator<Item = u32> {
        self.into_iter()
    }
}

// OK:
impl NewIntoIterator for Vec<u32> {
    type Item = u32;
    #[refine]
    fn into_iter(self) -> impl Iterator<Item = u32> + DoubleEndedIterator {
        self.into_iter()
    }
}

// OK:
impl NewIntoIterator for Vec<u32> {
    type Item = u32;
    #[refine]
    fn into_iter(self) -> std::vec::IntoIter<u32> {
        self.into_iter()
    }
}

// Not OK (requires `#[refine]`):
impl NewIntoIterator for Vec<u32> {
    type Item = u32;
    fn into_iter(self) -> std::vec::IntoIter<u32> {
        self.into_iter()
    }
}
```

Additionally, using `-> impl Trait` notation in an impl is only legal if the trait also uses that notation. Each occurrence of `impl Trait` in an impl must unify with an occurrence of `impl Trait` in the trait.

```rust
trait Trait {
    fn foo() -> i32;
    fn bar() -> impl Sized;
}

impl Trait for () {
    // Not OK
    fn foo() -> impl Sized { 0 }

    // Not OK
    fn bar() -> Result<impl Sized, impl Sized> { Ok::<(), ()>(()) }
}
```

An interesting consequence of auto trait leakage is that a trait is allowed to specify an auto trait in its return type bounds, but the impl does not have to _repeat_ that auto trait in its signature, as long as its return type actually implements the required bound. For example:

```rust
/// Converts `self` into an iterator that is always `Send`.
trait IntoSendIterator {
    type Item;
    fn into_iter(self) -> impl Iterator<Item = Self::Item> + Send;
}

// OK (signatures match exactly):
impl IntoSendIterator for Vec<u32> {
    type Item = u32;
    fn into_iter(self) -> impl Iterator<Item = u32> + Send {
        self.into_iter()
    }
}

// OK (auto traits leak, so adding `+ Send` here is NOT required):
impl IntoSendIterator for Vec<u32> {
    type Item = u32;
    fn into_iter(self) -> impl Iterator<Item = u32> {
        self.into_iter()
    }
}

// OK:
impl<T: Send> IntoSendIterator for Vec<T> {
    //  ^^^^ Required for our iterator to be Send!
    type Item = T;
    fn into_iter(self) -> impl Iterator<Item = T> {
        self.into_iter()
    }
}

// Not OK (returned iterator is not known to be `Send`):
impl<T> IntoSendIterator for Vec<T> {
    type Item = T;
    fn into_iter(self) -> impl Iterator<Item = T> {
        self.into_iter()
    }
}
```

### Interaction with `async fn` in trait

This RFC modifies the “Static async fn in traits” RFC so that async fn in traits may be satisfied by implementations that return `impl Future<Output = ...>` as long as the return-position impl trait type matches the async fn's desugared impl trait with the same rules as above.

```rust
trait Trait {
  async fn async_fn(&self);

  async fn async_fn_refined(&self);
}

impl Trait for MyType {
  fn async_fn(&self) -> impl Future<Output = ()> + '_ { .. }

  #[refine]
  fn async_fn_refined(&self) -> BoxFuture<'_, ()> { .. }
}
```

Similarly, the equivalent `-> impl Future` signature in a trait can be satisfied by using `async fn` in an impl of that trait.

## Legal positions for `impl Trait` to appear

`impl Trait` can appear in the return type of a trait method in all the same positions as it can in a free function.

For example, return position impl trait in traits may be nested in associated types bounds:

```rust
trait Nested {
    fn deref(&self) -> impl Deref<Target = impl Display> + '_;
}

// This desugars into:

trait Nested {
    type $1<'a>: Deref<Target = Self::$2> + 'a;

    type $2: Display;

    fn deref(&self) -> Self::$1<'_>;
}
```

It may also be used in type argument position of a generic type, including tuples:

```rust
trait Foo {
    fn bar(&self) -> (impl Debug, impl Debug);
}

// This desugars into:

trait Foo {
    type $1: Debug;
    type $2: Debug;

    fn bar(&self) -> (Self::$1, Self::$2);
}
```

But following the same rules as the allowed positions for return-position impl trait, it is not allowed to be nested in trait generics:

``` rust
trait Nested {
    fn deref(&self) -> impl AsRef<impl Sized>; // ❌
}
```

## Dyn safety

To start, traits that use `-> impl Trait` will not be considered dyn safe, *unless the method has a `where Self: Sized` bound*. This is because dyn types currently require that all associated types are named, and the `$` type cannot be named. The other reason is that the value of `impl Trait` is often a type that is unique to a specific impl, so even if the `$` type *could* be named, specifying its value would defeat the purpose of the `dyn` type, since it would effectively identify the dynamic type.

On the other hand, if the method has a `where Self: Sized` bound, the method will not exist on `dyn Trait` and therefore there will be no type to name.

### Dyn safety for `async fn` in trait

This RFC modifies the "Static async fn in traits" RFC to allow traits with `async fn` to be dyn-safe if the method has a `where Self: Sized` bound. This is consistent with how `async fn foo()` desugars to `fn foo() -> impl Future`.

# Drawbacks
[drawbacks]: #drawbacks

This section discusses known drawbacks of the proposal as presently designed and (where applicable) plans for mitigating them in the future.

## Cannot migrate off of impl Trait

In this RFC, if you use `-> impl Trait` in a trait definition, you cannot "migrate away" from that without changing all impls. In other words, we cannot evolve:

```rust
trait NewIntoIterator {
    type Item;
    fn into_iter(self) -> impl Iterator<Item = Self::Item>;
}
```

into

```rust
trait NewIntoIterator {
    type Item;
    type IntoIter: Iterator<Item = Self::Item>;
    fn into_iter(self) -> Self::IntoIter;
}
```

without breaking semver compatibility for your trait. The [future possibilities](#future-possibilities) section discusses one way to resolve this, by permitting impls to elide the definition of associated types whose values can be inferred from a function return type.

## Clients of the trait cannot name the resulting associated type, limiting extensibility

[As @Gankra highlighted in a comment on a previous RFC][gankra], the traditional `IntoIterator` trait permits clients of the trait to name the resulting iterator type and apply additional bounds:

[gankra]: https://github.com/rust-lang/rfcs/pull/3193#issuecomment-965505149

```rust
fn is_palindrome<Iter, T>(iterable: Iter) -> bool
where
    Iter: IntoIterator<Item = T>,
    Iter::IntoIter: DoubleEndedIterator,
    T: Eq;
```

The `NewIntoIterator` trait used as an example in this RFC, however, doesn't support this kind of usage, because there is no way for users to name the `IntoIter` type (and, as discussed in the previous section, there is no way for users to migrate to a named associated type, either!). The same problem applies to async functions in traits, which sometimes wish to be able to [add `Send` bounds to the resulting futures](https://rust-lang.github.io/async-fundamentals-initiative/evaluation/challenges/bounding_futures.html).

The [future possibilities](#future-possibilities) section discusses a planned extension to support naming the type returned by an impl trait, which could work to overcome this limitation for clients.

## Difference in scoping rules from `async fn`

`async fn` behaves [slightly differently][ref-async-captures] than return-position `impl Trait` when it comes to the scoping rules defined above. It considers _all_ lifetime parameters in-scope for the returned future.

[ref-async-captures]: https://doc.rust-lang.org/reference/items/functions.html#async-functions

In the case of there being one lifetime in scope (usually for `self`), the desugaring we've shown above is exactly equivalent:

```rust
trait Trait {
    async fn async_fn(&self);
}

impl Trait for MyType {
    fn async_fn(&self) -> impl Future<Output = ()> + '_ { .. }
}
```

It's worth taking a moment to discuss _why_ this works. The `+ '_` syntax here does two things:

1. It brings the lifetime of the `self` borrow into scope for the return type.
2. It promises that the return type will outlive the borrow of `self`.

In reality, the second point is not part of the `async fn` desugaring, but it does not matter: We can already reason that because our return type has only one lifetime in scope, it must outlive that lifetime.[^OutlivesProjectionComponents]

[^OutlivesProjectionComponents]: After all, the return type cannot possibly reference any lifetimes *shorter* than the one lifetime it is allowed to reference. This behavior is specified as the rule `OutlivesProjectionComponents` in [RFC 1214](https://rust-lang.github.io/rfcs/1214-projections-lifetimes-and-wf.html#outlives-for-projections). Note that it only works when there are no type parameters in scope.

When there are multiple lifetimes however, writing an equivalent desugaring becomes awkward.

```rust
trait Trait {
    async fn async_fn(&self, num: &u32);
}
```

We might be tempted to add another outlives bound:

```rust
impl Trait for MyType {
    fn async_fn<'b>(&self, num: &'b u32) -> impl Future<Output = ()> + '_ + 'b { .. }
}
```

But this signature actually promises *more* than the original trait does, and would require `#[refine]`. The `async fn` desugaring allows the returned future to name both lifetimes, but does not promise that it *outlives* both lifetimes.[^intersection]

[^intersection]: Technically speaking, we can reason that the returned future outlives the *intersection* of all named lifetimes. In other words, when all lifetimes the future is allowed to name are valid, we can reason that the future must also be valid. But at the time of this RFC, Rust has no syntax for intersection lifetimes.

One way to get around this is to "collapse" the lifetimes together:

```rust
impl Trait for MyType {
    fn async_fn<'a>(&'a self, num: &'a u32) -> impl Future<Output = ()> + 'a { .. }
}
```

In most cases[^lifetime-collapse] the type system actually recognizes these signatures as equivalent. This means it should be possible to write this trait with RPITIT now and move to async fn in the future. In the general case where these are not equivalent, it is possible to write an equivalent desugaring with a bit of a hack:

[^lifetime-collapse]: Both lifetimes must be [late-bound] and the type checker must be able to pick a lifetime that is the intersection of all input lifetimes, which is the case when either both are [covariant] or both are contravariant. The reason for this is described in more detail in [this comment](https://github.com/rust-lang/rust/issues/32330#issuecomment-202536977). In practice the equivalence can be checked [using the compiler](https://play.rust-lang.org/?version=nightly&mode=debug&edition=2021&gist=56faadfc236bb9acfb4af1b51a214a79). (Note that at the time of writing, a bug in the nightly compiler prevents it from accepting the example.)

[late-bound]: https://rust-lang.github.io/rfcs/0387-higher-ranked-trait-bounds.html#distinguishing-early-vs-late-bound-lifetimes-in-impls
[covariant]: https://doc.rust-lang.org/reference/subtyping.html#variance

```rust
trait Trait {
    async fn async_fn(&self, num_ref: &mut &u32);
    //                                     ^^^^
    // The lifetime of this inner reference is invariant!
}

impl Trait for MyType {
    // Let's say we do not want to use `async fn` here.
    // We cannot use the `+ 'a` syntax in this case,
    // so we use `Captures` to bring the lifetimes in scope.
    fn async_fn<'a, 'b>(&'a self, num_ref: &'a mut &'b u32)
    -> impl Future<Output = ()> + Captures<(&'a (), &'b ())> { .. }
}

trait Captures<T> {}
impl<T, U> Captures<T> for U {}
```

Note that the `Captures` trait doesn't promise anything at all; its sole purpose is to give you a place to name lifetime parameters you would like to be in scope for the return type.

This difference is pre-existing, but it's worth highlighting in this RFC the implications for the adoption of this feature. If we stabilize this feature first, people will use it to emulate `async fn` in traits. Care will be needed not to create forward-compatibility hazards for traits that want to migrate to `async fn` later. The best strategy for someone in that situation might be to simulate such a migration with the nightly compiler.

We leave open the question of whether to stabilize these two features together. In the future we can provide a nicer syntax for dealing with these cases, or remove the difference in scoping rules altogether.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

### Does auto trait leakage still occur for `-> impl Trait` in traits?

Yes, so long as the compiler has enough type information to figure out which impl you are using. In other words, given a trait function `SomeTrait::foo`, if you invoke a function `<T as SomeTrait>::foo()` where the self type is some generic parameter `T`, then the compiler doesn't really know what impl is being used, so no auto trait leakage can occur. But if you were to invoke `<u32 as SomeTrait>::foo()`, then the compiler could resolve to a specific impl, and hence a specific [impl trait type alias][tait], and auto trait leakage would occur as normal.

[tait]: https://rust-lang.github.io/impl-trait-initiative/explainer/tait.html

### Can traits migrate from a named associated type to `impl Trait`?

Not compatibly, no, because they would no longer have a named associated type. The "future directions" section discusses the possibility of allowing users to explicitly give a name for the associated type created, which would enable this use case.

### Can traits migrate from `impl Trait` to a named associated type?

Generally yes, but all impls would have to be rewritten to include the definition of the associated type. In many cases, some form of type-alias impl trait (or impl trait in associated type values) would also be required.

For example, if we changed the `IntoNumIterator` trait from the motivation to use an explicit associated type..

```rust
trait IntoNumIterator {
    type IntIter: Iterator<Item = u32>;
    fn into_iter(self) -> Self::IntIter;
}
```

...then impls like...

```rust
impl IntoNumIterator for MyType {
    fn into_int_iter(self) -> impl Iterator<Item = u32> {
        (0..self.len()).map(|x| x * 2)
    }
}
```

...would no longer compile, because they are not specifying the value of the `IntIter` associated type. Moreover, the value for this type would be impossible to express without `impl Trait` notation, as it embeds a closure type.

### Would there be any way to make it possible to migrate from `impl Trait` to a named associated type compatibly?

Potentially! There have been proposals to allow the values of associated types that appear in function return types to be inferred from the function declaration. So, using the example from the previous question, the impl for `IntoNumIterator` could infer the value of `IntIter` based on the return type of `into_int_iter`. This may be a good idea, but it is not proposed as part of this RFC.

### What about using an implicitly-defined associated type?

One alternative under consideration was to use a named associated type instead of the anonymous `$` type. The name could be derived by converting "snake case" methods to "camel case", for example. This has the advantage that users of the trait can refer to the return type by name.

We decided against this proposal:

* Introducing a name by converting to camel-case feels surprising and inelegant.
* Return position impl Trait in other kinds of functions doesn't introduce any sort of name for the return type, so it is not analogous.
* We would like to allow `-> impl Trait` methods to work with dynamic dispatch (see [Future possibilities][future-possibilities]). `dyn` types typically require naming all associated types of a trait. That would not be desirable for this feature, and these associated types would therefore not be consistent with other named associated types.

There is a need to introduce a mechanism for naming the return type for functions that use `-> impl Trait`; we plan to introduce a second RFC addressing this need uniformly across all kinds of functions.

As a backwards compatibility note, named associated types could likely be introduced later, although there is always the possibility of users having introduced associated types with the same name.

### What about using a normal associated type?

Giving users the ability to write an explicit `type Foo = impl Bar;` is already covered as part of the `type_alias_impl_trait` feature, which is not yet stable at the time of writing, and which represents an extension to the Rust language both inside and outside of traits. This RFC is about making trait methods consistent with normal free functions and inherent methods.

There are different situations where you would want to use an explicit associated type:

1. The type is central to the trait and deserves to be named.
1. You want to give users the ability to use concrete types without `#[refine]`.
1. You want to give generic users of your trait the ability specify a particular type, instead of just bounding it.
1. You want to give users the ability to easily name and bound the type without using (to-be-RFC'd) special syntax to name the type.
1. You want the trait to work with dynamic dispatch today.
1. In the future, you want the associated type to be specified as part of `dyn Trait`, instead of using dynamic dispatch itself.

Using our hypothetical `NewIntoIterator` example, most of these are not met for the `IntoIter` type:

1. While the `Item` type is pretty central to users of the trait, the specific iterator type `IntoIter` is usually not.
1. The concrete type of an impl may or may not be useful, but usually what's important is the specific extra bounds like `ExactSizeIterator` that a user can use. Using `#[refine]` to explicitly choose to expose this (or a fully concrete type) is not overly burdensome.
1. Rarely does a function taking `impl IntoIterator` specify a particular iterator type; it would be rare to see a function like this, for example:
   ```rust
   fn iterate_over_anything_as_long_as_it_is_vec<T>(
       vec: impl IntoIterator<IntoIter = std::vec::IntoIter<T>, Item = T>
   )
   ```
1. Bounding the iterator by adding extra bounds like `DoubleEndedIterator` is useful, but not the common case for `IntoIterator`. It therefore shouldn't be overly burdensome to use a (reasonably ergonomic) special syntax in the cases where it's needed.
1. Using `IntoIterator` with dynamic dispatch would be surprising; more common would be to call `.into_iter()` using static dispatch and then pass the resulting iterator to a function that uses dynamic dispatch.
1. If we did use `IntoIterator` with dynamic dispatch, the resulting iterator being dynamically dispatched would make the most sense.

Therefore, if we were writing `IntoIterator` today, it would probably use `-> impl Trait` in return position instead of having an explicit `IntoIter` type.

The same is not true for `Iterator::Item`: because `Item` is so central to what an `Iterator` is, and because it rarely makes sense to use an opaque type for the item, it would remain an explicit associated type.

# Prior art

## Should library traits migrate to use `impl Trait`?

Potentially, but not necessarily. Using `impl Trait` in traits imposes some limitations on generic code referencing those traits. While `impl Trait `desugars internally to an associated type, that associated type is anonymous and cannot be directly referenced by users, which prevents them from putting bounds on it or naming it for use in struct declarations. This is similar to `-> impl Trait` in free and inherent functions, which also returns an anonymous type that cannot be directly named. Just as in those cases, this likely means that widely used libraries should avoid the use of `-> impl Trait` and prefer to use an explicit named associated type, at least until some of the "future possibilities" are completed. However, this decision is best reached on a case-by-case basis: the real question is whether the bounds named in the trait will be sufficient, or whether users will wish to put additional bounds. In a trait like `IntoIterator`, for example, it is common to wish to bound the resulting iterator with additional traits, like `ExactLenIterator`. But given a trait that returns `-> impl Debug`, this concern may not apply.
[prior-art]: #prior-art

There are a number of crates that do desugaring like this manually or with procedural macros. One notable example is [real-async-trait](https://crates.io/crates/real-async-trait).

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Should we stabilize this feature together with `async fn` to mitigate hazards of writing a trait that is not forwards-compatible with its desugaring? (See [drawbacks].)
- Resolution of [#112194: RPITIT is allowed to name any in-scope lifetime parameter, unlike inherent RPIT methods](https://github.com/rust-lang/rust/issues/112194)
- Should we limit the legal positions for `impl Trait` to positions that are nameable using upcoming features like return-type notation (RTN)? (See [this comment](https://github.com/rust-lang/rfcs/pull/3425#pullrequestreview-1467880633) for an example.)

# Future possibilities
[future-possibilities]: #future-possibilities

### Naming return types

This RFC does not include a way for generic code to name or bound the result of `-> impl Trait` return types. This means, for example, that for the `IntoNumIterator` trait introduced in the motivation, it is not possible to write a function that takes a `T: IntoNumIterator` which returns an `ExactLenIterator`; for async functions, the most common time this comes up is code that wishes to take an async function that returns a `Send` future. We expect future RFCs will address these use cases.

### Dynamic dispatch

Similarly, we expect to introduce language extensions to address the inability to use `-> impl Trait` types with dynamic dispatch. These mechanisms are needed for async fn as well. A good writeup of the challenges can be found on the "challenges" page of the [async fundamentals initiative](https://rust-lang.github.io/async-fundamentals-initiative/evaluation/challenges/dyn_traits.html).

### Migration to associated type

It would be possible to introduce a mechanism that allows users to migrate from an `impl Trait` to a named associated type.

Existing users of the trait won't specify an associated type bound for the new associated type, nor will existing implementers of the trait specify the type. This can be fixed with [associated type defaults](https://github.com/rust-lang/rfcs/blob/master/text/2532-associated-type-defaults.md). So given a trait like `NewIntoIterator`, we could choose to introduce an associated type for the iterator like so:

```rust
// Now old again!
trait NewIntoIterator {
    type Item;
    type IntoIter = impl Iterator<Item = Self::Item>;
    fn into_iter(self) -> Self::IntoIter;
}
```

The only problem remaining is with `#[refine]`. If an existing implementation refined its return value of an RPITIT method, we would need the existing `#[refine]` attribute to stand in for an overriding of the associated type default.

Whatever rules we decide to make this work, they will interact with some ongoing discussions of proposals for `#[defines]` or `#[defined_by]` attributes on `type_alias_impl_trait`. We therefore leave the details of this to a future RFC.

### Adding new occurrences of `impl Trait` in refinements

We may want to allow the following pattern:

```rust
trait Trait {
    fn test() -> impl Sized;
}

impl Trait for () {
    #[refine]
    fn test() -> Result<impl Sized, impl Sized> { Ok::<(), ()>(()) }
}
```

Then uses of `impl Trait` in a trait impl would not necessarily correspond to a use of `impl Trait` in the trait.
