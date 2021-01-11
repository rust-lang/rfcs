- Feature Name: try_trait_v2
- Start Date: 2020-12-12
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Replace [RFC #1859, `try_trait`](https://rust-lang.github.io/rfcs/1859-try-trait.html),
with a new design for the currently-unstable [`Try` trait](https://doc.rust-lang.org/nightly/std/ops/trait.Try.html)
and corresponding desugaring for the `?` operator.

The new design supports all the currently-stable conversions (including the accidental ones),
while addressing the discovered shortcomings of the currently-implemented solution,
as well as enabling new scenarios.

*This is forward-looking to be compatible with other features,
like [`try {}`](https://doc.rust-lang.org/nightly/unstable-book/language-features/try-blocks.html) blocks
or [`yeet e`](https://twitter.com/josh_triplett/status/1248658754976927750) expressions
or [`Iterator::try_find`](https://github.com/rust-lang/rust/issues/63178),
but the statuses of those features are **not** themselves impacted by this RFC.*

# Motivation
[motivation]: #motivation

The motivations from the previous RFC still apply (supporting more types, and restricted interconversion).
However, new information has come in since the previous RFC, making people wish for a different approach.

- An [experience report](https://github.com/rust-lang/rust/issues/42327#issuecomment-366840247) in the tracking issue mentioned that it's annoying to need to make a residual type.
- The `try {}` conversations have wished for more source information to flow through `?` so that fewer annotations would be required.
- Similarly, it's no longer clear that `From` should be part of the `?` desugaring for _all_ types.  It's both more flexible -- making inference difficult -- and more restrictive -- especially without specialization -- than is always desired.
- Various library methods, such as `try_map` for arrays ([PR #79713](https://github.com/rust-lang/rust/pull/79713#issuecomment-739075171)), would like to be able to do HKT-like things to produce their result types.  (For example, `Iterator::try_find` wants to be able to return a `Foo<Option<Item>>` from a predicate that returned a `Foo<bool>`.)
- Using the "error" terminology is a poor fit for other potential implementations of the trait.
- It turned out that the current solution accidentally stabilized more interconversion than expected, so a more restricted form may be warranted.

This RFC proposes a solution that _mixes_ the two major options considered last time.

- Like the _reductionist_ approach, this RFC proposes an unparameterized trait with an _associated_ type for the "ok" part, so that the type produced from the `?` operator on a value is always the same.
- Like the [_essentialist_ approach](https://github.com/rust-lang/rfcs/blob/master/text/1859-try-trait.md#the-essentialist-approach), this RFC proposes a trait with a _generic_ parameter for "error" part, so that different types can be consumed.

<!--
Why are we doing this? What use cases does it support? What is the expected outcome?
-->

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## The `ops::ControlFlow` type

This is a simple enum:
```rust
enum ControlFlow<B, C = ()> {
    Break(B),
    Continue(C),
}
```

Its purpose is to clearly communicate the desire to either short-circuit what's happening (`Break`), or just to go on as normal (`Continue`).

For example, it can be used to early-exit in `Iterator::try_for_each`:
```rust
let y: ControlFlow<i32> = it.try_for_each(|x| {
    if x % 100 == 99 {
        return ControlFlow::Break(x);
    }

    ControlFlow::Continue(())
});
```
While one could also use `Result` to do this, it can be confusing to use `Err` for what one would mentally consider a _successful_ early exit.  Using a different type without those extra associations can help avoid mental dissonance while reading the code.

You might also use it when exposing similar things yourself, such as a graph traversal or visitor, where you want the user to be able to choose to break early.

## Defining your own `Result`-like type

We've seen elsewhere in the book that `Result` is just an enum.  Let's define our own to learn more about how `?` works.

To start with, let's use this type:
```rust
enum MyResult<T, U> {
    Terrific(T),
    Unfortunate(U)
}
```

That lets us do all the pattern matching things, but let's implement some more traits to support additional operators.

### Supporting `?` via `Bubble`

`Bubble` lets us define which values of our type let execution go on normally, and which should result in a short circuit.

Here's a full implementation:
```rust
use std::ops::{ControlFlow, Bubble};
impl<T, U> Bubble for MyResult<T, U> {
    type Continue = T;
    type Holder = <Result<T, U> as Bubble>::Holder;
    fn branch(self) -> ControlFlow<Self::Holder, T> {
        match self {
            MyResult::Terrific(v) => ControlFlow::Continue(v),
            MyResult::Unfortunate(e) => ControlFlow::Break(Err(e)),
        }
    }
    fn continue_with(v: T) -> Self {
        MyResult::Terrific(v)
    }
}
```

Taking each of those associated items individually:
- The `Continue` type is the type that comes out when applying the `?` operator.  For us it's just one of our generic types.  If there was only one value that represented success, though, it might just be `()`.
- The `Holder` type represents the other possible states.  For now we'll just use `Result`'s holder type, but will come back to it in a future section.
- The `branch` method tells the `?` operator whether or not we need to early-exit for a value.  Here we've said that `?` should produce the value from the `Terrific` variant and short circuit for `Unfortunate` values.
- One can also create an instance of our type from a value of the `Continue` type using the `continue_with` constructor.

Because we used `Result`'s holder type, this is enough to use `?` on our type in a method that returns an appropriate `Result`:
```rust
fn foo() -> Result<(), f32> {
    let _: () = MyResult::Unfortunate(1.1)?;
    Ok(())
}
```

### Consuming `?` via `Try`

If we change that function to return `MyResult`, however, we'll get an error:
```rust
error[E0277]: the `?` operator can only be used in a function that returns `Result` or `Option` (or another type that implements `Try`)
  --> C:\src\rust\src\test\ui\try-operator-custom-bubble-and-try.rs:29:17
   |
LL | / fn foo() -> MyResult<(), f32> {
LL | |     let _: () = MyResult::Unfortunate(1.1)?;
   | |                 ^^^^^^^^^^^^^^^^^^^^^^^^ cannot use the `?` operator in a function that returns `MyResult<(), f32>`
LL | |     MyResult::Terrific(())
LL | | }
   | |_- this function should return `Result` or `Option` to accept `?`
   |
   = help: the trait `Try<std::result::Result<!, {float}>>` is not implemented for `MyResult<(), f32>`
   = note: required by `from_holder`
```

So let's implement that one:
```rust
use std::ops::Try;
impl<T, U> Try for MyResult<T, U> {
    fn from_holder(h: Self::Holder) -> Self {
        match h {
            Err(e) => MyResult::Unfortunate(e),
            Ok(v) => match v {},
        }
    }
}
```

This is much simpler, with just the one associated function.  Because the holder is always an error, we'll always produce a `Unfortunate` result.  (The extra `match v {}` is because that's uninhabited, but [`exhaustive_patterns`](https://github.com/rust-lang/rust/issues/51085) is not yet stable, so we can't just omit the `Ok` arm.)

With this we can now use `?` on both `MyResult`s and `Result`s in a function returning `MyResult`:
```rust
fn foo() -> MyResult<(), f32> {
    let _: () = MyResult::Unfortunate(1.1)?;
    MyResult::Terrific(())
}

fn bar() -> MyResult<(), f32> {
    let _: () = Err(1.1)?;
    MyResult::Terrific(())
}
```

### Avoiding interconversion with a custom `Holder`

While interconversion isn't a problem for our custom result-like type, one might not always want it.  For example, you might be making a type that short-circuits on something you think of as success, or just doesn't make sense as pass/fail so there isn't a meaningful "error" to provide.  So let's see how we'd make a custom holder to handle that.

As we saw in the `Bubble::branch` implementation, the holder type preserves the values that weren't returned in the `Continue` type.  Thus for us it'll depend only on `U`, never on `T`.

Also, a holder type can always be associated back to its canonical `Try` (and `Bubble`) type.  This allows generic code to keep the "result-ness" or "option-ness" of a type while changing the `Continue` type.  So while we only need to store a `U`, we'll need some sort of wrapper around it to keep its "myresult-ness".

Conveniently, though, we don't need to define a new type for that: we can use our enum, but with an uninhabited type on one side.  As `!` isn't stable yet, we'll use `std::convert::Infallible` as a canonical uninhabited type.  (You may have seen it before in `TryFrom`, with `u64: TryFrom<u8, Error = Infallible>` since that conversion cannot fail.)

First we need to change the `Holder` type in our `Bubble` implementation, and change the body of `branch` accordingly:
```rust
use std::convert::Infallible;
impl<T, U> Bubble for MyResult<T, U> {
    ... no changes here ...
    type Holder = MyResult<Infallible, U>;
    fn branch(self) -> ControlFlow<Self::Holder, T> {
        match self {
            MyResult::Terrific(v) => ControlFlow::Continue(v),
            MyResult::Unfortunate(e) => ControlFlow::Break(MyResult::Unfortunate(e)),
        }
    }
    ... no changes here ...
}
```

As well as update our `Try` implementation for the new holder type:
```rust
impl<T, U> Try for MyResult<T, U> {
    fn from_holder(h: Self::Holder) -> Self {
        match h {
            MyResult::Unfortunate(e) => MyResult::Unfortunate(e),
            MyResult::Terrific(v) => match v {},
        }
    }
}
```

We're not quite done, though; the compiler will let us know that we have more work to do:
```rust
error[E0277]: the trait bound `MyResult<Infallible, U>: BreakHolder<T>` is not satisfied
  --> C:\src\rust\src\test\ui\try-operator-custom-v2.rs:17:5
   |
LL |     type Holder = MyResult<Infallible, U>;
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ the trait `BreakHolder<T>` is not implemented for `MyResult<Infallible, U>`
   |
  ::: C:\src\rust\library\core\src\ops\try.rs:102:18
   |
LL |     type Holder: BreakHolder<Self::Continue>;
   |                  --------------------------- required by this bound in `std::ops::Bubble::Holder`
```

But that's a simple one:
```rust
impl<T, U> BreakHolder<T> for MyResult<Infallible, U> {
    type Output = MyResult<T, U>;
}
```

You can think of this trait as bringing back together the `T` and the `U` that `Bubble` had split into different associated types.

With that we can still use `?` in both directions as in `foo` previously.  And it means that `Iterator::try_find` will give us back a `MyResult` if we return it in the predicate:
```rust
let x = [1, 2].iter().try_find(|&&x| {
    if x < 0 {
        MyResult::Unfortunate("uhoh")
    } else {
        MyResult::Terrific(x % 2 == 0)
    }
});
assert!(matches!(x, MyResult::Terrific(Some(2))));
```

As expected, the mixing in `bar` no longer compiles:
```
help: the trait `Try<std::result::Result<!, {float}>>` is not implemented for `MyResult<(), f32>`
```

### Enabling `Result`-like error conversion

`Result` allows mismatched error types so long as it can convert the source one into the type on the function.  But if we try that with our current type, it won't work:
```rust
fn qux() -> MyResult<(), i64> {
    let _: () = MyResult::Unfortunate(3_u8)?;
    MyResult::Terrific(())
}
```

That help message in the error from the previous section gives us a clue, however.  Thus far we've been using the default generic parameter on `Try<H>` which only allows an exact match to our declared `Bubble::Holder` type.  But we can be more general if we want.  Let's use `From`, like `Result` does:
```rust
impl<T, U, V: From<U>> Try<MyResult<Infallible, U>> for MyResult<T, V> {
    fn from_holder(h: MyResult<Infallible, U>) -> Self {
        match h {
            MyResult::Unfortunate(e) => MyResult::Unfortunate(From::from(e)),
            MyResult::Terrific(v) => match v {},
        }
    }
}
```

With that the `qux` example starts compiling successfully.

(This can also be used to allow interconversion with holder types from other type constructors, but that's left as an exercise for the reader.)

<!--
Explain the proposal as if it was already included in the language and you were teaching it to another Rust programmer. That generally means:

- Introducing new named concepts.
- Explaining the feature largely in terms of examples.
- Explaining how Rust programmers should *think* about the feature, and how it should impact the way they use Rust. It should explain the impact as concretely as possible.
- If applicable, provide sample error messages, deprecation warnings, or migration guidance.
- If applicable, describe the differences between teaching this to existing Rust programmers and new Rust programmers.

For implementation-oriented RFCs (e.g. for compiler internals), this section should focus on how compiler contributors should think about the change, and give examples of its concrete impact. For policy RFCs, this section should provide an example-driven introduction to the policy, and explain its impact in concrete terms.
-->

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## `ops::ControlFlow`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlFlow<B, C = ()> {
    /// Exit the loop, yielding the given value
    Break(B),
    /// Continue in the loop, using the given value for the next iteration
    Continue(C),
}
```

## The traits

```rust
trait Bubble {
	type Continue;
	type Holder: BreakHolder<Self::Continue>;
	fn continue_with(c: Self::Continue) -> Self;
	fn branch(self) -> ControlFlow<Self::Holder, Self::Continue>;
}

trait BreakHolder<T> {
    type Output: Try<Continue = T, Holder = Self>;
}

trait Try<H = <Self as Bubble>::Holder>: Bubble {
	fn from_holder(h: H) -> Self;
}
```

## Desugaring `?`

The previous desugaring of `x?` was

```rust
match Try::into_result(x) {
	Ok(v) => v,
	Err(e) => return Try::from_error(From::from(e)),
}
```

The new one is very similar:

```rust
match Bubble::branch(x) {
	ControlFlow::Continue(v) => v,
	ControlFlow::Break(h) => return Try::from_holder(h),
}
```

It's just left conversion (such as `From::from`) up to the implementation instead of forcing it in the desugar.

## Standard implementations

### `Result`

```rust
impl<T, E> ops::Bubble for Result<T, E> {
    type Continue = T;
    type Holder = Result<!, E>;

    fn continue_with(c: T) -> Self {
        Ok(c)
    }

    fn branch(self) -> ControlFlow<Self::Holder, T> {
        match self {
            Ok(c) => ControlFlow::Continue(c),
            Err(e) => ControlFlow::Break(Err(e)),
        }
    }
}

impl<T, E> ops::BreakHolder<T> for Result<!, E> {
    type Output = Result<T, E>;
}

#[unstable(feature = "try_trait_v2", issue = "42327")]
impl<T, E, F: From<E>> ops::Try<Result<!, E>> for Result<T, F> {
    fn from_holder(x: Result<!, E>) -> Self {
        match x {
            Err(e) => Err(From::from(e)),
        }
    }
}
```

### `Option`

```rust
impl<T> ops::Bubble for Option<T> {
    type Continue = T;
    type Holder = Option<!>;

    #[inline]
    fn continue_with(c: T) -> Self {
        Some(c)
    }

    #[inline]
    fn branch(self) -> ControlFlow<Self::Holder, T> {
        match self {
            Some(c) => ControlFlow::Continue(c),
            None => ControlFlow::Break(None),
        }
    }
}

impl<T> ops::BreakHolder<T> for Option<!> {
    type Output = Option<T>;
}

impl<T> ops::Try for Option<T> {
    fn from_holder(x: Self::Holder) -> Self {
        match x {
            None => None,
        }
    }
}
```

### `Poll`

These reuse `Result`'s holder type, so don't need `BreakHolder` implementations of their own, nor additional `Try` implementations on `Result`.

```rust
impl<T, E> ops::Bubble for Poll<Result<T, E>> {
    type Continue = Poll<T>;
    type Holder = <Result<T, E> as ops::Bubble>::Holder;

    fn continue_with(c: Self::Continue) -> Self {
        c.map(Ok)
    }

    fn branch(self) -> ControlFlow<Self::Holder, Self::Continue> {
        match self {
            Poll::Ready(Ok(x)) => ControlFlow::Continue(Poll::Ready(x)),
            Poll::Ready(Err(e)) => ControlFlow::Break(Err(e)),
            Poll::Pending => ControlFlow::Continue(Poll::Pending),
        }
    }
}

impl<T, E, F: From<E>> ops::Try<Result<!, E>> for Poll<Result<T, F>> {
    fn from_holder(x: Result<!, E>) -> Self {
        match x {
            Err(e) => Poll::Ready(Err(From::from(e))),
        }
    }
}
```

```rust
impl<T, E> ops::Bubble for Poll<Option<Result<T, E>>> {
    type Continue = Poll<Option<T>>;
    type Holder = <Result<T, E> as ops::Bubble>::Holder;

    fn continue_with(c: Self::Continue) -> Self {
        c.map(|x| x.map(Ok))
    }

    fn branch(self) -> ControlFlow<Self::Holder, Self::Continue> {
        match self {
            Poll::Ready(Some(Ok(x))) => ControlFlow::Continue(Poll::Ready(Some(x))),
            Poll::Ready(Some(Err(e))) => ControlFlow::Break(Err(e)),
            Poll::Ready(None) => ControlFlow::Continue(Poll::Ready(None)),
            Poll::Pending => ControlFlow::Continue(Poll::Pending),
        }
    }
}

impl<T, E, F: From<E>> ops::Try<Result<!, E>> for Poll<Option<Result<T, F>>> {
    fn from_holder(x: Result<!, E>) -> Self {
        match x {
            Err(e) => Poll::Ready(Some(Err(From::from(e)))),
        }
    }
}
```

### `ControlFlow`

```rust
impl<B, C> ops::Bubble for ControlFlow<B, C> {
    type Continue = C;
    type Holder = ControlFlow<B, !>;
    fn continue_with(c: C) -> Self {
        ControlFlow::Continue(c)
    }
    fn branch(self) -> ControlFlow<Self::Holder, C> {
        match self {
            ControlFlow::Continue(c) => ControlFlow::Continue(c),
            ControlFlow::Break(b) => ControlFlow::Break(ControlFlow::Break(b)),
        }
    }
}

impl<B, C> ops::BreakHolder<C> for ControlFlow<B, !> {
    type Output = ControlFlow<B, C>;
}

impl<B, C> ops::Try for ControlFlow<B, C> {
    fn from_holder(x: Self::Holder) -> Self {
        match x {
            ControlFlow::Break(b) => ControlFlow::Break(b),
        }
    }
}
```

## Making the accidental `Option` interconversion continue to work

This is done with an extra implementation:
```rust
mod sadness {
    use super::*;

    /// This is a remnant of the old `NoneError`, and is never going to be stabilized.
    /// It's here as a snapshot of an oversight that allowed this to work in the past,
    /// so we're stuck supporting it even though we'd really rather not.
    #[unstable(feature = "legacy_try_trait", issue = "none")]
    #[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
    pub struct PleaseCallTheOkOrMethodToUseQuestionMarkOnOptionsInAMethodThatReturnsResult;

    #[unstable(feature = "try_trait_v2", issue = "42327")]
    impl<T, E> ops::Try<Option<!>> for Result<T, E>
    where
        E: From<PleaseCallTheOkOrMethodToUseQuestionMarkOnOptionsInAMethodThatReturnsResult>,
    {
        fn from_holder(x: Option<!>) -> Self {
            match x {
                None => Err(From::from(
                    PleaseCallTheOkOrMethodToUseQuestionMarkOnOptionsInAMethodThatReturnsResult,
                )),
            }
        }
    }
}
```

## Use in `Iterator`

The provided implementation of `try_fold` is already just using `?` and `try{}`, so doesn't change.  The only difference is the name of the associated type in the bound:
```rust
fn try_fold<B, F, R>(&mut self, init: B, mut f: F) -> R
where
    Self: Sized,
    F: FnMut(B, Self::Item) -> R,
    R: Try<Continue = B>,
{
    let mut accum = init;
    while let Some(x) = self.next() {
        accum = f(accum, x)?;
    }
    try { accum }
}
```

The current (unstable) `try_find` allows any `Try<Ok = bool>` type as the return type of its predicate, but always returns a `Result`.

One can take advantage of `BreakHolder` to still return `Result` when the predicate returned `Result`, but also to return `Option` when the predicate returned `Option` (and so on):
```rust
fn try_find<F, R>(
    &mut self,
    f: F,
) -> <R::Holder as ops::BreakHolder<Option<Self::Item>>>::Output
where
    Self: Sized,
    F: FnMut(&Self::Item) -> R,
    R: ops::Try<Continue = bool>,
    R::Holder: ops::BreakHolder<Option<Self::Item>>,
{
    #[inline]
    fn check<F, T, R>(mut f: F) -> impl FnMut((), T) -> ControlFlow<Result<T, R::Holder>>
    where
        F: FnMut(&T) -> R,
        R: Try<Continue = bool>,
    {
        move |(), x| match f(&x).branch() {
            ControlFlow::Continue(false) => ControlFlow::CONTINUE,
            ControlFlow::Continue(true) => ControlFlow::Break(Ok(x)),
            ControlFlow::Break(h) => ControlFlow::Break(Err(h)),
        }
    }

    match self.try_fold((), check(f)) {
        ControlFlow::Continue(()) => ops::Bubble::continue_with(None),
        ControlFlow::Break(Ok(x)) => ops::Bubble::continue_with(Some(x)),
        ControlFlow::Break(Err(h)) => Try::from_holder(h),
    }
}
```

<!--
This is the technical portion of the RFC. Explain the design in sufficient detail that:

- Its interaction with other features is clear.
- It is reasonably clear how the feature would be implemented.
- Corner cases are dissected by example.

The section should return to the examples given in the previous section, and explain more fully how the detailed proposal makes those examples work.
-->

# Drawbacks
[drawbacks]: #drawbacks

- While this handles a known accidental stabilization, it's possible that there's something else unknown that will keep this from being doable while meeting Rust's stringent stability guarantees.
- The extra complexity of this approach, compared to either of the alternatives considered the last time around, might not be worth it.
- This is the fourth attempt at a design in this space, so it might not be the right one either.
- As with all overloadable operators, users might implement this to do something weird.
- In situations where extensive interconversion is desired, this requires more implementations.

<!--
Why should we *not* do this?
-->

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Why `ControlFlow` pulls its weight

The previous RFC discussed having such a type, but ended up deciding that defining a new type for the desugar wasn't worth it, and just used `Result`.

This RFC does use a new type because one already [exists in nightly](https://doc.rust-lang.org/nightly/std/ops/enum.ControlFlow.html) under [the `control_flow_enum` feature gate](https://github.com/rust-lang/rust/issues/75744).
It's being used in [the library](https://github.com/rust-lang/rust/blob/fd34606ddf02d1e9364e459b373a6ad665c3d8a4/library/core/src/iter/traits/iterator.rs#L2239-L2252) and [the compiler](https://github.com/rust-lang/rust/blob/c609b2eaf323186a1167ec1a9ffa69a7d4a5b1b9/compiler/rustc_middle/src/ty/fold.rs#L184-L206), demonstrating that it's useful beyond just this desugaring, so the desugar might as well use it too for extra clarity.
There are also [ecosystem changes waiting on something like it](https://github.com/rust-itertools/itertools/issues/469#issuecomment-677729589), so it's not just a compiler-internal need.

## Methods on `ControlFlow`

On nightly there are are a [variety of methods](https://doc.rust-lang.org/nightly/std/ops/enum.ControlFlow.html#implementations) available on `ControlFlow`.  However, none of them are needed for the stabilization of the traits, so they left out of this RFC.  They can be considered by libs at a later point.

There's a basic set of simple ones that could be included if desired, though:
```rust
impl<B, C> ControlFlow<B, C> {
	fn is_break(&self) -> bool;
	fn is_continue(&self) -> bool;
	fn break_value(self) -> Option<B>;
	fn continue_value(self) -> Option<C>;
}
```

## Traits for `ControlFlow`

`ControlFlow` derives a variety of traits where they have obvious behaviour.  It does not, however, derive `PartialOrd`/`Ord`.  They're left out as it's unclear which order, if any, makes sense between the variants.

For `Option`s, `None < Some(_)`, but for `Result`s, `Ok(_) < Err(_)`.  So there's no definition for `ControlFlow` that's consistent with the isomorphism to both types.

Leaving it out also leaves us free to change the ordering of the variants in the definition in case doing so can allow us to optimize the `?` operator.  (For a similar previous experiment, see [PR #49499](https://github.com/rust-lang/rust/pull/49499).)

## Was this considered last time?

Interestingly, a [previous version](https://github.com/rust-lang/rfcs/blob/f89568b1fe5db4d01c4668e0d334d4a5abb023d8/text/0000-try-trait.md#using-an-associated-type-for-the-success-value) of RFC #1859 _did_ actually mention a two-trait solution, splitting the "associated type for ok" and "generic type for error" like is done here.  It's no longer  mentioned in the version that was merged.  To speculate, it may have been unpopular due to a thought that an extra traits just for the associated type wasn't worth it.

Current desires for the solution, however, have more requirements than were included in the RFC at the time of that version.  Notably, the stabilized `Iterator::try_fold` method depends on being able to create a `Try` type from the accumulator.  Including such a constructor on the trait with the associated type helps that separate trait provide value.

Also, ok-wrapping was decided [in #70941](https://github.com/rust-lang/rust/issues/70941), which needs such a constructor, making this ["much more appealing"](https://github.com/rust-lang/rust/issues/42327#issuecomment-379882998).

## Trait naming

Bikeshed away!

## Why a "holder" type is better than an "error" type

Most importantly, for any type generic in its "continue type" it's easy to produce the holder type using an uninhabited type.  That works for `Option` -- no `NoneError` residual type needed -- as well as for the `StrandFail<T>` type from the experience report.  And thanks to enum layout optimizations, there's no space overhead to doing this: `Option<!>` is a ZST, and `Result<!, E>` is no larger than `E` itself.  So most of the time one will not need to define anything additional.

In those cases where a separate type *is* needed, it's still easier to make a holder type because they're transient and thus can be opaque: there's no point at which a user is expected to *do* anything with a holder type other than convert it back into a known `Try` type.  This is different from the previous design, where less-restrictive interconversion meant that anything could be exposed via a `Result`.  That has lead to requests, [such as for `NoneError` to implement `Error`](https://github.com/rust-lang/rust/issues/46871#issuecomment-618186642), that are perfectly understandable given that the instances are exposed in `Result`s.  As holder types aren't ever exposed like that, it would be fine for them to implement nothing but `BreakHolder` (and probably `Debug`), making them cheap to define and maintain.

## Use of `!`

This RFC uses `!` to be concise.  It would work fine with `convert::Infallible` instead if `!` has not yet stabilized, though a few more match arms would be needed in the implementations.  (For example, `Option::from_holder` would need `Some(c) => match c {}`.)

## Moving away from the `Option`→`Result` interconversion

We could consider doing an edition switch to make this no longer allowed.

For example, we could have a different, never-stable `Try`-like trait used in old editions for the `?` desugaring.  It could then have a blanket impl, plus the extra interconversion one.

It's unclear that that's worth the effort, however, so this RFC is currently written to continue to support it going forward.  Notably, removing it isn't enough to solve the annotation requirements, so the opportunity cost feels low.

## Bounds on `Bubble::Holder`

The bound for this associated type could be tightened as follows:
```rust
type Holder: BreakHolder<Self::Continue, Output = Self>;
```

That forces every type to be in bijection with its holder; however it's not clear that such a restriction is valuable.

The implementation for `Poll<Result<T, E>>`, for example, works well with reusing `Result`'s holder type.  The type *wants* to support the interconversion anyway, so forcing it to define a newtype for the holder is extra busywork.  It might not even be possible for types outside `core` due to coherence.

The advantage of the bijection, though, is that methods like `Iterator::try_find` would always return the "same" thing conceptually.  But in generic code that's mostly irrelevant, and even with known types it's not a big deal.  The holder type will be the same, so `?` would still work to go back to the original type if needed.  And of course the type can always define a distinct holder if they're worried about it.

## `BreakHolder<T>` vs using GATs

Generic Associated Types (GATs) may not be stable at the time of writing this RFC, but it's natural to ask if we'd like to use them if available, and thus should wait for them.

Types like `Result` and `Option` support any (sized) type as their continue type, so GATs would be reasonable for them.  However, with other possible types that might not be the case.  For example, imagine supporting `?` on `process::ExitStatus`.  Its holder type would likely only be `BreakHolder<()>`, since it cannot hold a custom payload and for it "success is defined as a zero exit status" (per `ExitStatus::success`).  So requiring that these types support an arbitrary generic continue type may be overly restrictive, and thus the trait approach -- allowing bounding to just the types needed -- seems reasonable.

(Note that RFC #1598 only [included lifetime-GATs, not type-GATs](https://rust-lang.github.io/rfcs/1598-generic_associated_types.html#associated-type-constructors-of-type-arguments).  So it's likely that it would also be a very long wait for type-GATs.)

## Default `H` on `Try`

The default here is provided to make the homogeneous case simple.  Along with its supertrait, that greatly simplifies the bound on `Iterator::try_fold`, for example.  Just requiring `R: Try` is enough to break apart and rebuild a value of that type (as was the case with the previous trait).

It's also convenient when implementing the trait in cases that don't need conversion, as seen with `Option`.

## `Try::from_holder` vs `Holder::into_try`

Either of these directions could be made to work.  Indeed, an early experiment while drafting this had a method on `BreakHolder` that created the `Bubble` type (not just the associated type).  However that was removed as unnecessary once `Try::from_holder` was added.

A major advantage of the `Try::from_holder` direction is that it's more flexible with coherence when it comes to allowing other things to be converted into a new type being defined.  That does come at the cost of higher restriction on allowing the new type to be converted into other things, but reusing a holder can also be used for that scenario.

Converting a known holder into a generic `Bubble` type seems impossible (unless it's uninhabited), but consuming arbitrary holders could work -- imagine something like
```rust
impl<H: std::fmt::Debug + BreakHolder<()>> Try<H> for LogAndIgnoreErrors {
    fn from_holder(h: H) -> Self {
        dbg!(h);
        Self
    }
}
```
(Not that that's necessarily a good idea -- it's plausibly *too* generic.  This RFC definitely isn't proposing it for the standard library.)

And, ignoring the coherence implications, a major difference between the two sides is that the target type is typically typed out visibly (in a return type) whereas the source type (going into the `?`) is often the result of some called function.  So it's preferrable for any behaviour extensions to be on the type that can more easily be seen in the code.

<!--
- Why is this design the best in the space of possible designs?
- What other designs have been considered and what is the rationale for not choosing them?
- What is the impact of not doing this?
-->

# Prior art
[prior-art]: #prior-art

Previous approaches used on nightly
- The original [`Carrier` trait](https://doc.rust-lang.org/1.16.0/core/ops/trait.Carrier.html)
- The next design with a [`Try` trait](https://doc.rust-lang.org/1.32.0/core/ops/trait.Try.html) (different from the one here)

Thinking from the perspective of a [monad](https://doc.rust-lang.org/1.32.0/core/ops/trait.Try.html), `Bubble::continue_with` is similar to `return`, and `<H as BreakHolder<T>>::Output` is its type constructor.

<!--
Discuss prior art, both the good and the bad, in relation to this proposal.
A few examples of what this can include are:

- For language, library, cargo, tools, and compiler proposals: Does this feature exist in other programming languages and what experience have their community had?
- For community proposals: Is this done by some other community and what were their experiences with it?
- For other teams: What lessons can we learn from what other communities have done here?
- Papers: Are there any published papers or great posts that discuss this? If you have some relevant papers to refer to, this can serve as a more detailed theoretical background.

This section is intended to encourage you as an author to think about the lessons from other languages, provide readers of your RFC with a fuller picture.
If there is no prior art, that is fine - your ideas are interesting to us whether they are brand new or if it is an adaptation from other languages.

Note that while precedent set by other languages is some motivation, it does not on its own motivate an RFC.
Please also take into consideration that rust sometimes intentionally diverges from common language features.
-->

# Unresolved questions
[unresolved-questions]: #unresolved-questions

Scope:
- Is `BreakHolder<T>` worth it?  There are a bunch of scenarios that might be interested in it, but they're all either currently-unstable or just future possibilities.
- Should this bring in anything from the future section, such as more things about `try {}`?

Bikesheds:
- I've long liked [parasyte's "bubble" suggestion](https://internals.rust-lang.org/t/bikeshed-a-consise-verb-for-the-operator/7289/29?u=scottmcm) as a name, but maybe there's a better option.
- The "holder" name is really vague, and `BreakHolder` isn't much better.
- This uses `Try` mostly because that meant not touching all the `try_fold` implementations in the prototype.  It's possible that name fits better on a different trait, or none of them.  That trait as `from_holder`, which is most related to "yeet", so a name related to that might fit better for it.

<!--
- What parts of the design do you expect to resolve through the RFC process before this gets merged?
- What parts of the design do you expect to resolve through the implementation of this feature before stabilization?
- What related issues do you consider out of scope for this RFC that could be addressed in the future independently of the solution that comes out of this RFC?
-->

# Future possibilities
[future-possibilities]: #future-possibilities

## Possibilities for `try{}`

A core problem with [try blocks](https://doc.rust-lang.org/nightly/unstable-book/language-features/try-blocks.html) as implemented in nightly, is that they require their contextual type to be known.

That is, the following never compiles, no matter the types of `x` and `y`:
```rust
let _ = try {
	foo(x?);
	bar(y?);
	z
};
```

This usually isn't a problem on stable, as the `?` usually has a contextual type from its function, but can still happen there in closures.

But with the design in this RFC, an alternative desugaring becomes available which takes advantage of how the holder type preserves the "result-ness" (or whatever-ness) of the original value.  That might turn the block above into something like the following:
```rust
fn helper<C, H: BreakHolder<C>>(h: H) -> <H as BreakHolder<C>>::Output { Try::from_holder(h) }

'block: {
	foo(match Bubble::branch(x) {
		ControlFlow::Continue(c) => c,
		ControlFlow::Break(h) => break 'block helper(h),
	});
	bar(match Bubble::branch(y) {
		ControlFlow::Continue(c) => c,
		ControlFlow::Break(h) => break 'block helper(h),
	});
	Bubble::continue_with(z)
}
```
(It's untested whether the inference engine is smart enough to pick the appropriate `C` with just that -- the `Output` associated type is constrained to have a `Continue` type matching the generic parameter, and that `Continue` type needs to match that of `z`, so it's possible.  But hopefully this communicates the idea, even if an actual implementation might need to more specifically introduce type variables or something.)

That way it could compile so long as the output types of the holders matched.  For example, [these uses in rustc](https://github.com/rust-lang/rust/blob/7cf205610e1310897f43b35713a42459e8b40c64/compiler/rustc_codegen_ssa/src/back/linker.rs#L529-L573) would work without the extra annotation.

Now, of course that wouldn't cover anything.  It wouldn't work with anything needing error conversion, for example, but annotation is also unavoidable in those cases -- there's no reasonable way for the compiler to pick "the" type into which all the errors are convertible.

So a future RFC could define a way (syntax, code inspection, heuristics, who knows) to pick which of the desugarings would be best.  This RFC declines to even brainstorm possibilities for doing so.

*Note that the `?` desugaring in nightly is already different depending whether it's inside a `try {}` (since it needs to block-break instead of `return`), so making it slightly more different shouldn't have excessive implementation cost.*

## Possibilities for `yeet`

As previously mentioned, this RFC neither defines nor proposes a `yeet` operator.  However, like the previous design could support one with its `Try::from_error`, it's important that this design would be sufficient to support it.

Because this "holder" design carries along the "result-ness" or "option-ness" or similar, it means there are two possibilities for a desugaring.

- It could directly take the holder type, so `yeet e` would desugar directly to `Try::from_holder(e)`.
- It could put the argument into a special holder type, so `yeet e` would desugar to something like `Try::from_holder(Yeet(e))`.

These have various implications -- like `yeet None`/`yeet`, `yeet Err(ErrorKind::NotFound)`/`yeet ErrorKind::NotFound.into()`, etc -- but thankfully this RFC doesn't need to discuss those.  (And please don't do so in the comments either.) 

<!--
Think about what the natural extension and evolution of your proposal would
be and how it would affect the language and project as a whole in a holistic
way. Try to use this section as a tool to more fully consider all possible
interactions with the project and language in your proposal.
Also consider how this all fits into the roadmap for the project
and of the relevant sub-team.

This is also a good place to "dump ideas", if they are out of scope for the
RFC you are writing but otherwise related.

If you have tried and cannot think of any future possibilities,
you may simply state that you cannot think of anything.

Note that having something written down in the future-possibilities section
is not a reason to accept the current or a future RFC; such notes should be
in the section on motivation or rationale in this or subsequent RFCs.
The section merely provides additional information.
-->
