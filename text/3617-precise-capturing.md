- Feature Name: `precise_capturing`
- Start Date: 2024-04-03
- RFC PR: [rust-lang/rfcs#3617](https://github.com/rust-lang/rfcs/pull/3617)
- Tracking Issue: [rust-lang/rust#123432](https://github.com/rust-lang/rust/issues/123432)

# Summary
[summary]: #summary

This RFC adds `use<..>` syntax for specifying which generic parameters should be captured in an opaque RPIT-like `impl Trait` type, e.g. `impl use<'t, T> Trait`.  This solves the problem of overcapturing and will allow the Lifetime Capture Rules 2024 to be fully stabilized for RPIT in Rust 2024.

# Motivation
[motivation]: #motivation

## Background

RPIT-like opaque `impl Trait` types in Rust *capture* certain generic parameters.

*Capturing* a generic parameter means that parameter can be used in the hidden type later registered for that opaque type.  Any generic parameters not captured cannot be used.

However, captured generic parameters that are *not* used by the hidden type still affect borrow checking.  This leads to the phenomenon of *overcapturing*.  Consider:

```rust
fn foo<T>(_: T) -> impl Sized {}
//                 ^^^^^^^^^^
//                 ^ The returned opaque type captures `T`
//                   but the hidden type does not.

fn bar(x: ()) -> impl Sized + 'static {
    foo(&x)
//~^ ERROR returns a value referencing data owned by the
//~|       current function
}
```

In this example, we would say that `foo` *overcaptures* the type parameter `T`.  The hidden type returned by `foo` does not *use* `T`, however it (and any lifetime components it contains) are part of the returned opaque type.  This leads to the error we see above.

Overcapturing limits how callers can use returned opaque types in ways that are often surprising and frustrating.  There's no good way to work around this in Rust today.

## Lifetime Capture Rules 2024

In Rust 2021 and earlier editions, all type parameters in scope are implicitly captured in RPIT-like `impl Trait` opaque types.  In these editions, lifetime parameters are not implicitly captured unless named in the bounds of the opaque.  This resulted, among other things, in the use of "the `Captures` trick".  See [RFC 3498][] for more details about this.

In RFC 3498, we decided to capture all in-scope generic parameters in RPIT-like `impl Trait` opaque types, across all editions, for new features we were stabilizing such as return position `impl Trait` in Trait (RPITIT) and associated type position `impl Trait` (ATPIT), and to capture all in-scope generic parameters for RPIT on bare functions and on inherent functions and methods starting in the Rust 2024 edition.  Doing this made the language more predictable and consistent, eliminated weird "tricks", and, by solving key problems, allowed for the stabilization of RPITIT.

However, the expansion of the RPIT rules in Rust 2024 means that some existing uses of RPIT, when migrated to Rust 2024, will now capture lifetime parameters that were not previously captured, and this may result in code failing to compile.  For example, consider:

```rust
//@ edition: 2021
fn foo<'t>(_: &'t ()) -> impl Sized {}

fn bar(x: ()) -> impl Sized + 'static {
    foo(&x)
}
```

Under the Rust 2021 rules, this code is accepted because `'t` is not implicitly captured in the returned opaque type.  When migrated to Rust 2024, the `'t` lifetime will be captured, and so this will fail to compile just as with the similar earlier example that had overcaptured a type parameter.

We need some way to migrate this kind of code.

[RFC 3498]: https://github.com/rust-lang/rfcs/blob/master/text/3498-lifetime-capture-rules-2024.md

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

In all editions, RPIT-like `impl Trait` opaque types may include `use<..>` in the bound to specify which in-scope generic parameters are captured or that no in-scope generic parameters are captured (with `use<>`).  If `use<..>` is provided, it entirely overrides the implicit rules for which generic parameters are captured.

One way to think about `use<..>` is that, in Rust `use` brings things *into scope*, and here we are bringing certain generic parameters into scope for the hidden type.

For example, we can solve the overcapturing in the original motivating example by writing:

```rust
fn foo<T>(_: T) -> impl use<> Sized {}
//                 ^^^^^^^^^^^^^^^^
//                 ^ Captures nothing.
```

Similarly, we can use this to avoid overcapturing a lifetime parameter so as to migrate code to Rust 2024:;

```rust
fn foo<'t>(_: &'t ()) -> impl use<> Sized {}
//                       ^^^^^^^^^^^^^^^^
//                       ^ Captures nothing.
```

We can use this to capture some generic parameters but not others:

```rust
fn foo<'t, T, U>(_: &'t (), _: T, y: U) -> impl use<U> Sized { y }
//                                         ^^^^^^^^^^^^^^^^^
//                                         ^ Captures `U` only.
```

## Generic const parameters

In addition to type and lifetime parameters, we can use this to capture generic const parameters:

```rust
fn foo<'t, const C: u8>(_: &'t ()) -> impl use<C> Sized { C }
//                                    ^^^^^^^^^^^^^^^^^
//                                    ^ Captures `C` only.
```

## Capturing from outer inherent impl

We can capture generic parameters from an outer inherent impl:

```rust
struct Ty<'a, 'b>(&'a (), &'b ());

impl<'a, 'b> Ty<'a, 'b> {
    fn foo(x: &'a (), _: &'b ()) -> impl use<'a> Sized { x }
    //                              ^^^^^^^^^^^^^^^^^^
    //                              ^ Captures `'a` only.
}
```

## Capturing from outer trait impl

We can capture generic parameters from an outer trait impl:

```rust
trait Trait<'a, 'b> {
    type Foo;
    fn foo(_: &'a (), _: &'b ()) -> Self::Foo;
}

impl<'a, 'b> Trait<'a, 'b> for () {
    type Foo = impl use<'a> Sized;
    //         ^^^^^^^^^^^^^^^^^^
    //         ^ Captures `'a` only.
    fn foo(x: &'a (), _: &'b ()) -> Self::Foo { x }
}
```

## Capturing in trait definition

We can capture generic parameters from the trait definition:

```rust
trait Trait<'a, 'b> {
    fn foo(_: &'a (), _: &'b ()) -> impl use<'a, Self> Sized;
    //                              ^^^^^^^^^^^^^^^^^^^^^^^^
    //                              ^ Captures `'a` and `Self` only.
}
```

## Capturing elided lifetimes

We can capture elided lifetimes:

```rust
fn foo(x: &()) -> impl use<'_> Sized { x }
//                ^^^^^^^^^^^^^^^^^^
//                ^ Captures `'_` only.
```

## Combining with `for<..>`

The `use<..>` specifier applies to the entire `impl Trait` opaque type.  In contrast, a `for<..>` binder applies to an individual *bound* within an opaque type.  Therefore, when both are used within the same type, `use<..>` always appears first.  E.g.:

```rust
fn foo<T>(_: T) -> impl use<T> for<'a> FnOnce(&'a ()) { |&()| () }
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Syntax

The [syntax for `impl Trait`][] is revised and extended as follows:

[syntax for `impl Trait`]: https://doc.rust-lang.org/nightly/reference/types/impl-trait.html

> _ImplTraitType_ :
> &nbsp;&nbsp; `impl` _UseCaptures_<sup>?</sup> [_TypeParamBounds_][]
>
> _ImplTraitTypeOneBound_ :
> &nbsp;&nbsp; `impl` _UseCaptures_<sup>?</sup> [_TraitBound_][]
>
> _UseCaptures_ :\
> &nbsp;&nbsp; `use` _UseCapturesGenericArgs_
>
> _UseCapturesGenericArgs_ :\
> &nbsp;&nbsp; &nbsp;&nbsp; `<` `>` \
> &nbsp;&nbsp; | `<` \
> &nbsp;&nbsp; &nbsp;&nbsp; ( _UseCapturesGenericArg_ `,`)<sup>\*</sup> \
> &nbsp;&nbsp; &nbsp;&nbsp; _UseCapturesGenericArg_ `,`<sup>?</sup> \
> &nbsp;&nbsp; &nbsp;&nbsp; `>`
>
> _UseCapturesGenericArg_ :\
> &nbsp;&nbsp; &nbsp;&nbsp; [LIFETIME_OR_LABEL][] \
> &nbsp;&nbsp; | [IDENTIFIER][]

[IDENTIFIER]: https://doc.rust-lang.org/nightly/reference/identifiers.html
[LIFETIME_OR_LABEL]: https://doc.rust-lang.org/nightly/reference/tokens.html#lifetimes-and-loop-labels
[_TraitBound_]: https://doc.rust-lang.org/nightly/reference/trait-bounds.html
[_TypeParamBounds_]: https://doc.rust-lang.org/nightly/reference/trait-bounds.html

## Reference desugaring

Associated type position `impl Trait` (ATPIT) can also be used, more verbosely, to control capturing of generic parameters in opaque types.  We can use this to describe the semantics of `use<..>`.  If we consider the following code:

```rust
struct Ty<'u, U, const CU: u8>(&'u (), U);

impl<'u, U, const CU: u8> Ty<'u, U, CU> {
    pub fn f<'t, T, const CT: u8>(
        self, x: &'t (), y: T,
    ) -> impl use<'u, 't, U, T, CU, CT> Sized {
        (self, x, y, CU, CT)
    }
}
```

Then, using ATPIT, we could desugar this as follows while preserving equivalent semantics:

```rust
struct Ty<'u, U, const CU: u8>(&'u (), U);

impl<'u, U, const CU: u8> Ty<'u, U, CU> {
    pub fn f<'t, T, const CT: u8>(
        self, x: &'t (), y: T,
    ) -> <() as _0::H>::Opaque<'u, 't, U, T, CU, CT> {
        <() as _0::H>::f(self, x, y)
    }
}

mod _0 {
    use super::*;
    pub trait H {
        type Opaque<'u, 't, U, T, const CU: u8, const CT: u8>;
        fn f<'u, 't, U, T, const CU: u8, const CT: u8>(
            s: Ty<'u, U, CU>, x: &'t (), y: T,
        ) -> Self::Opaque<'u, 't, U, T, CU, CT>;
    }
    impl H for () {
        type Opaque<'u, 't, U, T, const CU: u8, const CT: u8>
            = impl Sized;
        #[inline(always)]
        fn f<'u, 't, U, T, const CU: u8, const CT: u8>(
            s: Ty<'u, U, CU>, x: &'t (), y: T,
        ) -> Self::Opaque<'u, 't, U, T, CU, CT> {
            (s, x, y, CU, CT)
        }
    }
}
```

## Avoiding capture of higher ranked lifetimes in nested opaques

For implementation reasons, Rust does not yet support higher ranked lifetime bounds on nested opaque types (see [#104288][]).  However, according to the Lifetime Capture Rules 2024, a nested `impl Trait` opaque type *must* capture all generic parameters in scope, including higher ranked ones.  Therefore, in Rust 2024, this code fails to compile:

```rust
trait Trait { type Ty; }
impl<F> Trait for F { type Ty = (); }

fn foo() -> impl for<'a> Trait<Ty = impl Sized> {
    //~^ ERROR `impl Trait` cannot capture higher-ranked lifetime
    //~|        from outer `impl Trait`
    fn f(_: &()) -> &'static () { &() }
    f
}
```

With `use<..>`, we can avoid capturing this higher ranked lifetime, allowing compilation:

```rust
fn foo() -> impl for<'a> Trait<Ty = impl use<> Sized> {
    //                              ^^^^^^^^^^^^^^^^
    //                              ^ Captures nothing.
    fn f(_: &()) -> &'static () { &() }
    f
}
```

[#104288]: https://github.com/rust-lang/rust/issues/104288

## Capturing higher ranked lifetimes in nested opaques

Once higher ranked lifetime bounds on nested opaque types are supported in Rust (see [#104288][]), we'll be able to use `use<..>` specifiers to capture lifetime parameters from higher ranked `for<..>` binders on outer opaque types:

```rust
trait Trait<'a> { type Ty; }
impl<'a, F: Fn(&'a ()) -> &'a ()> Trait<'a> for F { type Ty = &'a (); }

fn foo() -> impl for<'a> Trait<'a, Ty = impl use<'a> Sized> {
    //                                  ^^^^^^^^^^^^^^^^^^
    //                                  ^ Captures `'a`.
    fn f(x: &()) -> &() { x }
    f
}
```

## Refinement

If we write a trait such as:

```rust
trait Trait {
    type Foo<'a>: Sized where Self: 'a;
    fn foo(&self) -> Self::Foo<'_>;
}
```

...then an impl of this trait can provide a type for the associated type `Foo` that uses the `&'_ self` lifetime:

```rust
struct A;
impl Trait for A {
    type Foo<'a> = &'a Self; // Or, e.g.: `impl use<'a> Sized`
    fn foo(&self) -> Self::Foo<'_> { self }
}
```

However, such an impl may also provide a type that does *not* use the lifetime:

```rust
struct B;
impl Trait for B {
    type Foo<'a> = (); // Or, e.g.: `impl use<> Sized`
    fn foo(&self) -> Self::Foo<'_> {}
}
```

If we only know that the value is of some type that implements the trait, then we must assume that the type returned by `foo` *might* have used the lifetime:

```rust
fn test_trait<T: Trait + 'static>(x: T) -> impl Sized + 'static {
    x.foo()
//~^ ERROR cannot return value referencing function parameter `x`
}
```

However, if we know we have a value of type `B`, we can *rely* on the fact that the lifetime was not used:

```rust
fn test_b(x: B) -> impl Sized + 'static {
    x.foo() //~ OK.
}
```

We would say that the impl for `B` is *refining* in that it offers more to or demands less of callers than the minimum the trait could offer or the maximum it could demand.  Associated type definitions are always refining in this way.

RPITIT desugars into associated types similar to those above, but here we've currently decided to lint against this refinement, e.g.:

```rust
trait Trait {
    fn foo(&self) -> impl Sized;
}

impl Trait for () {
    fn foo(&self) -> () {}
//~^ WARN impl trait in impl method signature does not match
//~|      trait method signature
//~| NOTE add `#[allow(refining_impl_trait)]` if it is intended
//~|      for this to be part of the public API of this crate
//~| NOTE we are soliciting feedback, see issue #121718
//~|      <https://github.com/rust-lang/rust/issues/121718>
//~|      for more information
}
```

Similarly, for consistency, we'll lint against RPITIT cases where less is captured by RPIT in the impl as compared with the trait definition when using `use<..>`.  E.g.:

```rust
trait Trait {
    fn foo(&self) -> impl Sized;
}

impl Trait for () {
    fn foo(&self) -> impl use<> Sized {}
//~^ WARN impl trait in impl method signature does not match
//~|      trait method signature
//~| NOTE add `#[allow(refining_impl_trait)]` if it is intended
//~|      for this to be part of the public API of this crate
//~| NOTE we are soliciting feedback, see issue #121718
//~|      <https://github.com/rust-lang/rust/issues/121718>
//~|      for more information
}
```

## Argument position impl Trait

Note that for a generic type parameter to be captured with `use<..>` it must have a name.  Anonymous generic type parameters introduced with argument position `impl Trait` (APIT) syntax don't have names, and so cannot be captured with `use<..>`.  E.g.:

```rust
fn foo(x: impl Sized) -> impl use<> Sized { x }
//                       ^^^^^^^^^^^^^^^^
//                       ^ Captures nothing.
```

## Migration strategy for Lifetime Capture Rules 2024

The migration lints for Rust 2024 will insert `use<..>` as needed so as to preserve the set of generic parameters captured by each RPIT opaque type.  That is, we will convert, e.g., this:

```rust
//@ edition: 2021
fn foo<'t, T>(_: &'t (), x: T) -> impl Sized { x }
```

...into this:

```rust
//@ edition: 2024
fn foo<'t, T>(_: &'t (), x: T) -> impl use<T> Sized { x }
```

Note that since generic type parameters must have names to be captured with `use<..>`, some uses of APIT will need to be converted to named generic parameters.  E.g., we will convert this:

```rust
//@ edition: 2021
fn foo<'t>(_: &'t (), x: impl Sized) -> impl Sized { x }
```

...into this:

```rust
//@ edition: 2024
fn foo<'t, T: Sized>(_: &'t (), x: T) -> impl use<T> Sized { x }
```

As we're always cognizant of adding noise during migrations, it's worth mentioning that this will also allow noise to be *removed*.  E.g., this code:

```rust
#[doc(hidden)]
pub trait Captures<'t> {}
impl<T: ?Sized> Captures<'_> for T {}

pub fn foo<'a, 'b, 'c>(
    x: &'a (), y: &'b (), _: &'c (),
) -> impl Sized + Captures<'a> + Captures<'b> {
    (x, y)
}
```

...can be replaced with this:

```rust
pub fn foo<'a, 'b, 'c>(
    x: &'a (), y: &'b (), _: &'c (),
) -> impl use<'a, 'b> Sized {
    (x, y)
}
```

As an example of what migrating to explicit `use<..>` captures looks like within `rustc` itself (without yet migrating to the Lifetime Capture Rules 2024 which would simplify many cases further), see [this diff][].

[this diff]: https://github.com/rust-lang/rust/compare/efd136e5cd57789834c7555eed36c490b7be6fe7...0d15c5c62d2a6f46269e5812653900e0945738bf?expand=1

## Stabilization strategy

Due to implementation considerations, it's likely that the initial stabilization of this feature will be partial.  We anticipate that partial stabilization will have these restrictions:

- `use<..>`, if provided, must include all in-scope type and const generic parameters.
- In RPIT within trait definitions, `use<..>`, if provided, must include all in-scope generic parameters.

We anticipate lifting these restrictions over time.

Since all in-scope type and const generic parameters were already captured in Rust 2021 and earlier editions, and since RPITIT already adheres to the Lifetime Capture Rules 2024, these restrictions do not interfere with the use of this feature to migrate code to Rust 2024.

# Alternatives
[alternatives]: #alternatives

## ATPIT / TAIT

As we saw in the reference desugaring above, associated type position `impl Trait` (ATPIT), once stabilized, can be used to effect precise capturing.  Originally, we had hoped that this (particularly once expanded to full type alias `impl Trait` (TAIT)) might be sufficient and that syntax such as that in this RFC might not be necessary.

As it turned out, there are four problems with this:

1. ATPIT/TAIT is too indirect a solution.
2. They might not be stabilized in time.
3. They would lead to a worse migration story.
4. We would want this syntax anyway.

Taking these in turn:

One, as can be seen in the reference desugaring, using ATPIT/TAIT in this way can be rather indirect, and this was confirmed in our practical experience when migrating code.  ATPIT and TAIT are good tools, but they weren't designed to solve this particular problem.  This problem calls for a more direct solution.

Two, while ATPIT is nearing stabilization, there are yet some type systems details being resolved.  For TAIT, there is much work yet to do.  Putting these features in the critical path would add risk to the edition, to the Lifetime Capture Rules 2024, and to these features.

Three, as a practical matter, an explicit `impl use<..> Trait` syntax lets us write much better automatic migration lints and offers a much more straightforward migration story for our users.

Four, the set of generic parameters that are captured by an opaque type is a fundamental and practical property of that opaque type.  In a language like Rust, it *feels* like there ought to be an explicit syntax for it.  We probably want this in any world.

## Inferred precise capturing

We had hoped that we might be able to achieve something with a similar effect to precise capturing at the cost of an extra generic lifetime parameter in each signature with improvements to the type system.  The goal would be to allow, e.g., this code to work rather than error:

```rust
fn foo<'o, T>(_: T) -> impl Sized + 'o {}

fn bar(x: ()) -> impl Sized + 'static {
    foo(&x)
//~^ ERROR returns a value referencing data owned by the
//~|       current function
}
```

The idea is that, even though the opaque type returned by `foo` does capture the generic type parameter `T`, since the opaque type is explicitly bounded by `'o` and the signature does not assert `T: 'o`, we know that the hidden type cannot actually use `T`.

As it turns out, making full use of this observation is challenging (see [#116040][] and [#116733][]).  While we did make improvements to the type system here, and while more might be possible, this does not solve the problem today in all important cases (including, e.g., avoiding the capture of higher ranked lifetimes in nested opaque types) and will not for the foreseeable future.

Moreover, even with the fullest possible version of these improvements, whether or not a generic parameter is captured by an opaque type would remain observable.  Having an explicit syntax to control what is captured is more direct, more expressive, and leads to a better migration story.

See [Appendix G][] in [RFC 3498][] for more details.

[#116040]: https://github.com/rust-lang/rust/pull/116040
[#116733]: https://github.com/rust-lang/rust/pull/116733
[Appendix G]: https://github.com/rust-lang/rfcs/blob/master/text/3498-lifetime-capture-rules-2024.md#appendix-g-future-possibility-inferred-precise-capturing

## Syntax

We considered a number of different possible syntaxes before landing on `impl use<..> Trait`.  We'll discuss each considered.

### `impl use<..> Trait`

This is the syntax chosen in this RFC.

Using a separate keyword makes this syntax more scalable in the sense that we can apply `use<..>` in other places.

Conveniently, the word "use" is quite appropriate here, since we are *using* the generic parameters in the type of the opaque type and allowing the generic parameters to be *used* in the hidden type.  That is, with `use`, we are bringing the generic parameters *into scope* for the hidden type, and `use` is the keyword in Rust for bringing things into scope.

Picking an existing keyword allows for this syntax, including extensions to other positions, to be allowed in older editions.  Because `use` is a full keyword, we're not limited in where it can be placed.

By not putting the generic parameters on `impl<..>`, we reduce the risk of confusion that we are somehow introducing generic parameters here rather than using them.

We put `impl` before `use<..>` because `use<..>` is a property of the opaque type and we're *applying* the generic *parameters* as generic *arguments* to this opaque type.  In `impl Trait` syntax, the `impl` keyword is the stand-in for the opaque type itself.  Viewed this way, `impl use<..> Trait` maintains the following order, which is seen throughout Rust: *type*, *generic arguments*, *bounds*.

Using angle brackets, rather than parenthesis or square brackets, is consistent with other places in the language where type parameters are applied to a type.

At three letters, the `use` keyword is short enough that it doesn't feel too noisy or too much like a burden to use this, and it's parsimonious with other short keywords in Rust.

Overall, naming is hard, but on average, people seemed to dislike this choice the least.

### `impl<..> Trait`

The original syntax proposal was `impl<..> Trait`.  This has the benefit of being somewhat more concise than `impl use<..> Trait` but has the drawback of perhaps suggesting that it's introducing generic parameters as other uses of `impl<..>` do.  Many preferred to use a different keyword for this reason.

Decisive to some was that we may want this syntax to *scale* to other uses, most particularly to controlling the set of generic parameters and values that are captured by closure-like blocks.  As we discuss in the future possibilities, it's easy to see how `use<..>` can scale to address this in a way that `impl<..> Trait` cannot.

### `use<..> impl Trait`

Putting the `use<..>` specifier *before* the `impl` keyword is potentially appealing as `use<..>` applies to the entire `impl Trait` opaque type rather than to just one of the bounds, and this ordering might better suggest that.

However, this visual association might also *prove too much*.  That is, it could make the `use<..>` look more like a *binder* (like `for<..>`) rather than like a *property* of the opaque type.

The `use<..>` syntax *applies* the listed generic *parameters* as generic *arguments* to the opaque type.  It's analogous, e.g., with the generic arguments here:

```rust
impl Trait for () {
    type Opaque<'t, T> = Concrete<'t, T>
    //                   ^^^^^^^^|^^^^^
    //                     Type  | Generic Arguments
    where Self: 'static;
    //    ^^^^^^^^^^^^^
    //       Bounds
}
```

Just as the above *applies* `<'t, T>` to `Concrete`, `use<..>` applies its arguments to the opaque type.

In the above example and throughout Rust, we observe the following order: *type*, *generic arguments* (applied to the type), *bounds*.  In `impl Trait` syntax, the `impl` keyword is the stand-in for the opaque type itself.  The `use<..>` specifier lists the generic arguments to be applied to that type.  Then the bounds follow.  Putting `use<..>` after `impl` is consistent with this rule, but the other way would be inconsistent.

This observation, that we're applying generic *arguments* to the opaque type and that the `impl` keyword is the stand-in for that type, is also a strong argument in favor of `impl<..> Trait` syntax.  It's conceivable that we'll later, with more experience and consistently with [Stroustrup's Rule][], decide that we want to be more concise and adopt the `impl<..> Trait` syntax after all.  One of the advantages of placing `use<..>` after `impl` is that there would be less visual and conceptual churn in later making that change.

Finally, there's one other practical advantage to placing `impl` before `use<..>`.  If we were to do it the other way and place `use<..>` before `impl`, we would need to make a backward incompatible change to the `ty` macro matcher fragment specifier.  This would require us to migrate this specifier according to our policy in [RFC 3531][].  This is something we could do, but it is a cost on us and on our users, and combined with the other good reasons that argue for `impl use<..> Trait` (or even `impl<..> Trait`), it doesn't seem a cost that's worth paying.

[RFC 3531]: https://github.com/rust-lang/rfcs/blob/master/text/3531-macro-fragment-policy.md
[Stroustrup's Rule]: https://www.thefeedbackloop.xyz/stroustrups-rule-and-layering-over-time/

### `impl Trait & ..`

In some conceptions, the difference between `impl Trait + 'a + 'b` and `impl use<'a, 'b> Trait` is the difference between capturing the union of those lifetimes and capturing the intersection of them.  This inspires syntax proposals such as `impl Trait & 't & T` or `impl Trait & ['t, T]` to express this intersection.

One problem with the former of these is that it gives no obvious way to express that the opaque type captures nothing.  Another is that it would give `AsRef &T` a valid but distinct meaning to `AsRef<&T>` which might be confusing.

For either of these, appearing later in the type would put these after higher ranked `for<..>` lifetimes may have been introduced.  This could be confusing, since `use<..>` (with any syntax) captures generic parameters for the entire type where `for<..>` applies individually to each bound.

Overall, nobody seemed to like this syntax.

### `impl k#captures<..> Trait`

We could use a new and very literal keyword such as `captures` rather than `use`.  There are three main drawbacks to this:

1. There are limits to how this could be used in older editions.
2. There's a cost to each new keyword, and `use` is probably good enough.
3. It's somewhat long.

Taking these in turn:

One, while `captures` could be reserved in Rust 2024 and used in any position in that edition, and in Rust 2021 could be used as `k#captures` in any position, on older editions, it would only be able to be used where it could be made contextual.  This could limit how we might be able to scale this syntax to handle other use cases such as controlling the capturing of generic parameters and values in closure-like blocks (as discussed in the future possibilities).

Two, each keyword takes from the space of names that users have available to them, and it increases the number of keywords with which users must be familiar (e.g. so as to not inadvertently trip over when choosing a name).  That is, each keyword has a cost.  If an existing keyword can reasonably be used in more places, then we get more benefit for that cost.  In this case, `use` is probably a strong enough choice that paying the cost for a new keyword doesn't seem worth it.

Three, `captures` would be a somewhat long keyword, especially when we consider how we might scale the use of this syntax to other places such as closure-like blocks.  We don't want people to feel punished for being explicit about the generics that they capture, and we don't want them to do other worse things (such as overcapturing where they should not) just to avoid visual bloat in their code, so if we can be more concise here, that seems like a win.

### `impl move<'t, T> Trait`

We could use the existing `move` keyword, however the word "move" is semantically worse.  In Rust, we already *use* generic parameters in types, but we don't *move* any generic parameters.  We move only *values*, so this could be confusing.  The word "use" is better.

### `impl k#via<'t, T> Trait`

We could use a new short keyword such as `via`.  This has the number 1 and 2 drawbacks of `k#captures` mentioned above.  As with `move`, it also seems a semantically worse word.  With `use<..>`, we can explain that it means the opaque type *uses* the listed generic parameters.  In contrast, it's not clear how we could explain the word "via" in this context.

### Using parenthesis or square brackets

We could say `use('t, T)` or `use['t, T]`.  However, in Rust today, generic parameters always fall within angle brackets, even when being applied to a type.  Doing something different here could feel inconsistent and doesn't seem warranted.

# Future possibilities
[future-possibilities]: #future-possibilities

## Opting out of captures

There will plausibly be cases where we want to capture many generic parameters and not capture only smaller number.  It could be convenient if there were a way to express this without listing out all of the in-scope type parameters except the ones not being captured.

The way we would approach this with the `use<..>` syntax is to add some syntax that means "fill in all in-scope generic parameters", then add syntax to remove certain generic parameters from the list.  E.g.:

```rust
fn foo<'a, A, B, C, D>(
    _: &'a A, b: B, c: C, d: D,
) -> impl use<.., !'a, !A> Sized {}
//   ^^^^^^^^^^^^^^^^^^^^^^^^^^^
//   ^ Captures `B`, `C`, and `D` but not `'a` or `A`.
```

Here, the `..` means to include all in-scope generic parameters and `!` means to exclude a particular generic parameter even if previously included.

We leave this to future work.

## Explicit capturing for closure-like blocks

Closures and closure-like blocks (e.g. `async`, `gen`, `async gen`, `async` closures, `gen` closures, `async gen` closures, etc.) return opaque types that capture both *values* and *generic parameters* from the outer scope.

### Specifying captured generics for closures-like blocks

The capturing of outer generics in closure-like blocks can lead to overcapturing, as in [#65442][].  Consider:

```rust
trait Trait {
    type Ty;
    fn define<T>(_: T) -> Self::Ty;
}

impl Trait for () {
    type Ty = impl Fn();
    fn define<T>(_: T) -> Self::Ty {
        || ()
    //~^ ERROR type parameter `T` is part of concrete type but not
    //~|       used in parameter list for the `impl Trait` type alias
    }
}
```

Here, the opaque type of the closure is capturing `T`.  We may want a way to specify which outer generic parameters are captured by closure-like blocks.  We could apply the `use<..>` syntax to closure-like blocks to solve this, e.g.:

```rust
trait Trait {
    type Ty;
    fn define<T>(_: T) -> Self::Ty;
}

impl Trait for () {
    type Ty = impl Fn();
    fn define<T>(_: T) -> Self::Ty {
        use<> || ()
    //  ^^^^^^^^^^^
    //  ^ Captures no generic parameters.
    }
}
```

We leave this to future work, but this demonstrates how the `use<..>` syntax can scale to solve other problems.

[#65442]: https://github.com/rust-lang/rust/issues/65442

### Specifying captured values for closure-like blocks

Closure-like blocks capture values either by *moving* them or by *referencing* them.  How Rust decides whether values should be captured by move or by reference is implicit and can be a bit subtle.  E.g., this works:

```rust
fn foo<T>(x: T) -> impl FnOnce() -> T {
    || x
}
```

...but this does not:

```rust
fn foo<T: Copy>(x: T) -> impl FnOnce() -> T {
    || x
//~^ ERROR may outlive borrowed value `x`
}
```

While in simple cases like this we can apply `move` to the entire closure-like block to get the result that we want, in other cases other techniques are needed.

We might want a syntax for specifying which values are captured by the closure-like block and how each value is captured.  We could apply the `use` syntax to solve this.  E.g.:

```rust
fn foo<A, B, C, D>(a: A, b: B, mut c: C, _: D) {
    let f = use(a, ref b, ref mut c) || {
        //      ^  ^^^^^  ^^^^^^^^^
        //      |  |      ^ Captures `c` by mutable reference.
        //      |  ^ Captures `b` by immutable reference.
        //      ^ Captures `a` by move.
        todo!()
    }
    todo!()
}
```

This could be combined with specifying which outer generic parameters to capture, e.g. with `use<A, B, C>(a, ref b, ref mut c)`.

We leave this to future work, but this demonstrates how the `use<..>` syntax can scale to solve other problems.
