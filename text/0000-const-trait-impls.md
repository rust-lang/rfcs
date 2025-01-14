- Feature Name: `const_trait_methods`
- Start Date: 2024-12-13
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#67792](https://github.com/rust-lang/rust/issues/67792)

# Summary
[summary]: #summary

Make trait methods callable in const contexts. This includes the following parts:

* Allow marking `trait` declarations as const implementable.
* Allow marking `trait` impls as `const`.
* Allow marking trait bounds as `const` to make methods of them callable in const contexts.

Fully contained example ([Playground of currently working example](https://play.rust-lang.org/?version=nightly&mode=debug&edition=2021&gist=2ab8d572c63bcf116b93c632705ddc1b)):

```rust
const trait Default {
    fn default() -> Self;
}

impl const Default for () {
    fn default() {}
}

const fn default<T: ~const Default>() -> T {
    T::default()
}

fn compile_time_default<T: const Default>() -> T {
    const { T::default() }
}

const _: () = Default::default();

fn main() {
    let () = default();
    let () = compile_time_default();
    let () = Default::default();
}
```

# Motivation
[motivation]: #motivation

Const code is currently only able to use a small subset of Rust code, as many standard library APIs and builtin syntax things require calling trait methods to work.
As an example, in const contexts you cannot use even basic equality on anything but primitives:

```rust
const fn foo() {
    let a = [1, 2, 3];
    let b = [1, 2, 4];
    if a == b {} // ERROR: cannot call non-const operator in constant functions
}
```

## Background

This RFC requires familarity with "const contexts", so you may have to read [the relevant reference section](https://doc.rust-lang.org/reference/const_eval.html#const-context) first.

Calling functions during const eval requires those functions' bodies to only use statements that const eval can handle. While it's possible to just run any code until it hits a statement const eval cannot handle, that would mean the function body is part of its semver guarantees. Something as innocent as a logging statement would make the function uncallable during const eval.

Thus we have a marker (`const`) to add in front of functions that requires the function body to only contain things const eval can handle. This in turn allows a `const` annotated function to be called from const contexts, as you now have a guarantee it will stay callable.

When calling a trait method, this simple scheme (that works great for free functions and inherent methods) does not work.

Throughout this document, we'll be revisiting the example below. Method syntax and `dyn Trait` problems all also exist with static method calls, so we'll stick with the latter to have the simplest examples possible.

```rust
const fn default<T: Default>() -> T {
    T::default()
}

// Could also be `const fn`, but that's an orthogonal change
fn compile_time_default<T: Default>() -> T {
    const { T::default() }
}
```

Neither of the above should (or do) compile.
The first, because you could pass any type T whose default impl could

* mutate a global static,
* read from a file, or
* just allocate memory,

which are all not possible right now in const code, and some can't be done in Rust in const code at all.

It should be possible to write `default` in a way that allows it to be called in const contexts
for types whose `Default` impl's `default` method satisfies all rules that `const fn` must satisfy
(including some annotation that guarantees this won't break by accident).
It must always be possible to call `default` outside of const contexts with no limitations on the generic parameters that may be passed.

Similarly it should be possible to write `compile_time_default` in a way that also requires calls
outside of const contexts to only pass generic parameters whose `Default::default` method satisifies
the usual `const fn` rules. This is necessary in order to allow a const block
(which can access generic parameters) in the function body to invoke methods on the generic parameter.

So, we need some annotation that differentiates a `T: Default` bound from one that gives us the guarantees we're looking for.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## Nomenclature and new syntax concepts

### Const trait impls

It is now allowed to prefix a trait name in an impl block with `const`, marking that this `impl`'s type is now allowed to
have methods of this `impl`'s trait to be called in const contexts (if all where bounds hold, like ususal, but more on this later).

An example looks as follows:

```rust
impl const Trait for Type {}
```

Such impls require that the trait is a `const trait`.

All method bodies in a const trait impl are [const contexts](https://doc.rust-lang.org/reference/const_eval.html#const-context).

### Const traits

Traits need to opt-in to being allowed to have const trait impls. Thus you need to declare your traits by prefixing the `trait` keyword with `const`:

```rust
const trait Trait {}
```

This in turn checks all methods' default bodies as if they were `const fn`, making them callable in const contexts.
Impls can now rely on the default methods being const, too, and don't need to override them with a const body.

We may add an attribute later to allow you to mark individual trait methods as not-const so that when creating a const trait, one can
add (defaulted or not) methods that cannot be used in const contexts.

It is possible to split up a trait into the const an non-const parts as discussed [here](#cant-have-const-methods-and-nonconst-methods-on-the-same-trait).

All default method bodies of const trait declarations are [const contexts](https://doc.rust-lang.org/reference/const_eval.html#const-context).

Note that on nightly the syntax is

```rust
#[const_trait]
trait Trait {}
```

and a result of this RFC would be that we would remove the attribute and add the `const trait` syntax.

### Const trait bounds

Any item that can have trait bounds can also have `const Trait` bounds.

Examples:

* `T: const Trait`, requiring any type that `T` is instantiated with to have a const trait impl.
* `dyn const Trait`, requiring any type that is unsized to this dyn trait to have a const trait impl.
    * These are not part of this RFC because they require `const fn` function pointers. See [the Future Possibilities section](#future-possibilities).
* `impl const Trait` (in all positions).
    * These are not part of this RFC because they require `const fn` function pointers. See [the Future Possibilities section](#future-possibilities).
* `trait Foo: const Bar {}`, requiring every type that has an impl for `Foo` (even a non-const one), to also have a const trait impl for `Bar`.

Such an impl allows you to use the type that is bound within a const block or any other const context, because we know that the type has a const trait impl and thus
must be executable at compile time. The following function will invoke the `Default` impl of a type at compile time and store the result in a constant. Then it returns that constant instead of computing the value every time.

```rust
fn compile_time_default<T: const Default>() -> T {
    const { T::default() }
}
```

### Conditionally-const trait bounds

Many generic `const fn` and especially many const trait impls do not actually require a const trait impl for their generic parameters.
As `const fn` can also be called at runtime, it would be too strict to require it to only be able to call things with const trait impls.
Picking up the example from [the beginning](#summary):

```rust
const trait Default {
    fn default() -> Self;
}

impl const Default for () {
    fn default() {}
}

impl<T: Default> Default for Box<T> {
    fn default() -> Self { Box::new(T::default()) }
}

// This function requires a `const` impl for the type passed for T,
// even if called from a non-const context
const fn default<T: const Default>() -> T {
    T::default()
}

const _: () = default();

fn main() {
    let _: Box<u32> = default();
    //~^ ERROR: <Box<u32> as Default>::default cannot be called at compile-time
}
```

What we instead want is that, just like `const fn` can be called at runtime and compile time, we want their trait bounds' constness
to mirror that behaviour. So we're introducing `~const Trait` bounds, which mean "const if called from const context" (slight oversimplifcation, but read on).

The only thing we need to change in our above example is the `default` function, changing the `const Default` bound to a `~const Default` one.

```rust
const fn default<T: ~const Default>() -> T {
    T::default()
}
```

`~const` is derived from "approximately", meaning "conditionally" in this context, or specifically "const impl required if called in const context".
It is the opposite of `?` (prexisting for `?Sized` bounds), which also means "conditionally", but from the other direction: `?const` (not proposed here, see the alternatives section for why it was rejected) would mean "no const impl required, even if called in const context".
See [this alternatives section](#make-all-const-fn-arguments-const-trait-by-default-and-require-an-opt-out-const-trait) for an explanation of why we do not use a `?const` scheme.

### Const fn

`const` fn have always been and will stay "always const" functions.

It may appear that a function is suddenly "not a const fn" if it gets passed a type that doesn't satisfy
the constness of the corresponding trait bound. E.g.

```rust
struct Foo;

impl Clone for Foo {
    fn clone(&self) -> Self {
        Foo
    }
}

const fn bar<T: ~const Clone>(t: &T) -> T { t.clone() }
const BAR: Foo = bar(Foo); // ERROR: `Foo`'s `Clone` impl is not for `const Clone`.
```

But `bar` is still a `const` fn and you can call it from a const context, it will just fail some trait bounds. This is no different from 

```rust
const fn dup<T: Copy>(a: T) -> (T, T) {(a, a)}
const FOO: (String, String) = dup(String::new());
```

Here `dup` is always const fn, you'll just get a trait bound failure if the type you pass isn't `Copy`.

This may seem like language lawyering, but that's how the impl works and how we should be talking about it.

It's actually important for inference and method resolution in the nonconst world today.
You first figure out which method you're calling, then you check its bounds.
Otherwise it would at least seem like we'd have to allow some SFINAE or method overloading style things,
which we definitely do not support and have historically rejected over and over again.


### `~const Destruct` trait

The `Destruct` trait enables dropping types within a const context.

```rust
const fn foo<T>(t: T) {
    // `t` is dropped here, but we don't know if we can evaluate its `Drop` impl (or that of its fields' types)
}
const fn baz<T: Copy>(t: T) {
    // Fine, `Copy` implies that no `Drop` impl exists
}
const fn bar<T: ~const Destruct>(t: T) {
    // Fine, we can safely invoke the destructor of `T`.
}
```

When a value of a generic type goes out of scope, it is dropped and (if it has one) its `Drop` impl gets invoked.
This situation seems no different from other trait bounds, except that types can be dropped without implementing `Drop`
(as they can contain types that implement `Drop`). In that case the type's drop glue is invoked.

The `Destruct` trait is a bound for whether a type has drop glue. This is trivally true for all types.

`~const Destruct` trait bounds are satsifed only if the type has a `const Drop` impl or all of the types of its components
are `~const Destruct`.

While this means that it's a breaking change to add a type with a non-const `Drop` impl to a type,
that's already true and nothing new:

```rust
pub struct S {
    x: u8,
    y: Box<()>, // adding this field breaks code.
}

const fn f(_: S) {}
//~^ ERROR destructor of `S` cannot be evaluated at compile-time
```

## Trivially enabled features

You can use `==` operators on most types from libstd from within const contexts.

```rust
const _: () = {
    let a = [1, 2, 3];
    let b = [4, 5, 6];
    assert!(a != b);
};
const _: () = {
    let a = Some(42);
    let b = a;
    assert!(a == b);
};
```

Note that the use of `assert_eq!` is waiting on `Debug` impls becoming `const`, which
will likely be tracked under a separate feature gate under the purview of T-libs.
Similarly other traits will be made `const` over time, but doing so will be
unblocked by this feature.

## Crate authors: Making your own custom types easier to use

You can write const trait impls of many standard library traits for your own types.
While it was often possible to write the same code in inherent methods, operators were
covered by traits from `std::ops` and thus not avaiable for const contexts.
Most of the time it suffices to add `const` before the trait name in the impl block.
The compiler will guide you and suggest where to also
add `~const` bounds for trait bounds on generic parameters of methods or the impl.

Similarly you can make your traits available for users of your crate to implement constly.
Note that this will change your semver guarantees: you are now guaranteeing that any future
methods you add don't just have a default body, but a `const` default body. The compiler will
enforce this, so you can't accidentally make a mistake, but it may still limit how you can
extend your trait without having to do a major version bump.
Most of the time it suffices to add `const` before the `trait` declaration. The compiler will
guide you and suggest where to also add `~const` bounds for super trait bounds or trait bounds
on generic parameters of your trait or your methods.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## How does this work in the compiler?

These `const` or `~const` trait bounds desugar to normal trait bounds without modifiers, plus an additional constness bound that has no surface level syntax.

A much more detailed explanation can be found in https://hackmd.io/@compiler-errors/r12zoixg1l#What-now

We generate a `ClauseKind::HostEffect` for every `const` or `~const` bound.
To mirror how some effectful languages represent such effects,
I'm going to use `<Type as Trait>::k#host` to allow setting whether the `host` effect is "const" (disabled) or "conditionally" (generic).
This is not comparable with other associated bounds like type bounds or const bounds, as the values the associated host effect can
take do neither have a usual hierarchy nor a concrete single value we can compare due to the following handling of those bounds:

* There is no "always" (enabled), as that is just the lack of a host effect, meaning no `<Type as Trait>::k#host` bound at all.
* In contrast to other effect systems, we do not track the effect as a true generic parameter in the type system,
  but instead just ignore all `Conditionally` bounds in host environments and treat them as `Const` in const environments.

While this could be modelled with generic parameters in the type system, that:

* Has been attempted and is really complex (fragile) on the impl side and on the reasoning about things side.
* Appears to permit more behaviours than are desirable (multiple such parameters, math on these parameters, ...), so they need to be prevented, adding more checks.
* Is not necessary unless we'd allow much more complex kinds of bounds. So it can be kept open as a future possibility, but for now there's no need.
* Does not quite work in Rust due to the constness then being early bound instead of late bound, cause all kinds of problems around closures and function calls.
* Technically cause two entirely separate MIR bodies to be generated, one for where the effect is on and one where it is off. On top of that it then theoretically allows you to call the const MIR body from non-const code.

Thus that approach was abandoned after proponents and opponents cooperated in trying to make the generic parameter approach work, resulting in all proponents becoming opponents, too.

### `const` desugaring

```rust
fn compile_time_default<T: const Default>() -> T {
    const { T::default() }
}
```

desugars to

```rust
fn compile_time_default<T>() -> T
where
    T: Default,
    <T as Default>::k#host = Const,
{
    const { T::default() }
}
```

### `~const` desugaring

```rust
const fn default<T: ~const Default>() -> T {
    T::default()
}
```

desugars to

```rust
const fn default<T>() -> T
where
    T: Default,
    <T as Default>::k#host = Conditionally,
{
    T::default()
}
```

### Why not both?


```rust
const fn checked_default<T>() -> T
where
    T: const Default,
    T: ~const Default,
    T: ~const PartialEq,
{
    let a = const { T::default() };
    let b = T::default();
    if a == b {
        a
    } else {
        panic!()
    }
}
```

Has a redundant bound. `T: const Default` implies `T: ~const Default`, so while the desugaring will include both (but may filter them out if we deem it useful on the impl side),
there is absolutely no difference (just like specifying `Fn() + FnOnce()` has a redundant `FnOnce()` bound).

## Precedence of `~`

The `~` sigil applies to the `const`, not the `const Trait`, so you can think of it as `(~const) Trait`, not `~(const Trait)`.
This is both handled this way by the parser, and semantically what is meant here. The constness of the trait bound is affected,
the trait bound itself exists either way.

## Why do traits need to be marked as "const implementable"?

### Default method bodies

Adding a new method with a default body would become a breaking change unless that method/default body
would somehow be marked as `const`, too. So by marking the trait, you're opting into the requirement that all default bodies are const checked,
and thus neither `impl const Trait for Type` items nor `impl Trait for Type` items will be affected if you add a new method with a default body.
This scheme avoids adding a new kind of breaking change to the Rust language,
and instead allows everyone managing a public trait in their crate to continue relying on the
previous rule "adding a new method is not a breaking change if it has a default body".

### `~const Destruct` super trait

Traits that have `self` (by ownership) methods, will almost always drop the `self` in these methods' bodies unless they are simple wrappers that just forward to the generic parameters' bounds.

The following never drops `T`, because it's the job of `<T as Add>` to handle dropping the values.

```rust
struct NewType<T>(T);

impl<T: ~const Add<Output = T>> const Add for NewType<T> {
    type Output = Self,
    fn add(self, other: Self) -> Self::Output {
        NewType(self.0 + other.0)
    }
}
```

But if any code path could drop a value...

```rust
struct NewType<T>(T, bool);

struct Error;

impl<T: ~const Add<Output = T>> const Add for NewType<T> {
    type Output = Result<Self, Error>;
    fn add(self, other: Self) -> Self::Output {
        if self.1 {
            Ok(NewType(self.0 + other.0, other.1))
        } else {
            // Drops both `self.0` and `self.1`
            Err(Error)
        }
    }
}
```

... then we need to add a `~const Destruct` bound to `T`, to ensure
`NewType<T>` can be dropped.

This bound in turn will be infectious to all generic users of `NewType` like

```rust
const fn add<T: ~const Add>(
    a: NewType<T>,
    b: NewType<T>,
) -> Result<NewType<T::Output>, Error> {
    a + b
}
```

which now need a `T: ~const Destruct` bound, too.
In practice we have noticed that a large portion of APIs will have a `~const Destruct` bound.
This bound has little value as an explicit bound that appears almost everywhere.
Especially since it is a fairly straight forward assumption that a type that has const trait impls will also have a `const Drop` impl or only contain `const Destruct` types.

Thus we give all `const trait`s a `~const Destruct` super trait to ensure users don't need to add `~const Destruct` bounds everywhere.
We may offer an opt out of this behaviour in the future, if there are convincing real world use cases.

### `~const` bounds on `Drop` impls

It is legal to add `~const` to `Drop` impls' bounds, even thought the struct doesn't have them:

```rust
const trait Bar {
    fn thing(&mut self);
}

struct Foo<T: Bar>(T);

impl<T: ~const Bar> const Drop for Foo<T> {
    fn drop(&mut self) {
        self.0.thing();
    }
}
```

There is no reason (and no coherent representation) of adding `~const` trait bounds to a type.
Our usual `Drop` rules enforce that an impl must have the same bounds as the type.
`~const` modifiers are special here, because they are only needed in const contexts.
While they cause exactly the divergence that we want to prevent with the `Drop` impl rules:
a type can be declared, but not dropped, because bounds are unfulfilled, this is:

* Already the case in const contexts, just for all types that aren't trivially free of `Drop` types.
* Exactly the behaviour we want.

Extraneous `~const Trait` bounds where `Trait` isn't a bound on the type at all are still rejected:

```rust
impl<T: ~const Bar + ~const Baz> const Drop for Foo<T> {
    fn drop(&mut self) {
        self.0.thing();
    }
}
```

errors with

```
error[E0367]: `Drop` impl requires `T: Baz` but the struct it is implemented for does not
  --> src/lib.rs:13:22
   |
13 | impl<T: ~const Bar + ~const Baz> const Drop for Foo<T> {
   |                      ^^^^^^^^^^
   |
note: the implementor must specify the same requirement
  --> src/lib.rs:8:1
   |
8  | struct Foo<T: Bar>(T);
   | ^^^^^^^^^^^^^^^^^^
```

# Drawbacks
[drawbacks]: #drawbacks

## Adding any feature at all around constness

I think we've reached the point where all critics have agreed that this one kind of effect system is unavoidable since we want to be able to write maintainable code for compile time evaluation.

So the main drawback is that it creates interest in extending the system or add more effect systems, as we have now opened the door with an effect system that supports traits.
Even though I personally am interested in adding an effect for panic-freedom, I do not think that adding this const effect system should have any bearing on whether we'll add
a panic-freedom effect system or other effect systems in the future. This feature stands entirely on its own, and even if we came up with a general system for many effects that is (e.g. syntactically) better in the
presence of many effects, we'll want the syntax from this RFC as sugar for the very common and simple case.

## It's hard to make constness optional with `#[cfg]`

One cannot `#[cfg]` just the `const` keyword in `const Trait`, and even if we made it possible by sticking with `#[const_trait]` attributes, and also adding the equivalent for impls and functions,
`~const Trait` bounds cannot be made conditional with `#[cfg]`. The only real useful reason to have this is to support newer Rust versions with a cfg, and allow older Rust versions to compile
the traits, just without const support. This is surmountable with proc macros that either generate two versions or just generate a different version depending on the Rust version.
Since it's only necessary for a transition period while a crate wants to support both pre-const-trait Rust and
newer Rust versions, this doesn't seem too bad. With a MSRV bump the proc macro usage can be removed again.

## Can't have const methods and nonconst methods on the same trait

If a trait has methods that don't make sense for const contexts, but some that do, then right now it is required to split that
trait into a nonconst trait and a const trait and "merge" them by making one of them be a super trait of the other:

```rust
const trait Foo {
    fn foo(&self);
}
trait Bar: Foo {
    fn bar(&self);
}

impl const Foo for () {
    fn foo(&self) {}
}
impl Bar for () {
    fn bar(&self) {
        println!("writing to terminal is not possible in const eval");
    }
}
```

Such a split is not possible without a breaking change, so splitting may not be feasible in some cases.
Especially since we may later offer the ability to have const and nonconst methods on the same trait, then allowing
the traits to be merged again. That's churn we'd like to avoid.

Note that it may frequently be that such a trait should have been split even without constness being part of the picture.

# Alternatives
[alternatives]: #alternatives

## use `const Trait` bounds for conditionally-const, invent new syntax for always-const

It may seem tempting to use `const fn foo<T: const Trait>` to mean what in this RFC is `~const Trait`, and then add new syntax for bounds that allow using trait methods in const blocks.

Examples of possible always const syntax:

* `=const Trait`
* `const const Trait` (lol)
* `const(always) Trait` (`pub` like)
* `const<true> Trait` (effect generic like)
* `const! Trait`

## use `Trait<const>` or `Trait<bikeshed#effect: const>` instead of `const Trait`

To avoid new syntax before paths referring to traits, we could treat the constness as a generic parameter or an associated type.
While an associated type is very close to how the implementation works, neither `effect = const` nor `effect: const` are representing the logic correctly,
as `const` implies `~const`, but `~const` is nothing concrete, it's more like a generic parameter referring to the constness of the function.
Fully expanded one can think of

```rust
const fn foo<T: ~const Trait + const OtherTrait>(t: T) { ... }
```

to be like

```rust
const<const C: bool> fn foo<T>(t: T)
where
    T: Trait + OtherTrait,
    <T as Trait>::bikeshed#effect = const<C>,
    <T as OtherTrait>::bikeshed#effect = const<true>,
{
    ...
}
```

Note that `const<true>` implies `const<false>` and thus also `for<C> const<C>`, just like `const Trait` implies `~const Trait`.

We do not know of any cases where such an explicit syntax would be useful (only makes sense if you can do math on the bool),
so a more reduced version could be

```rust
const fn foo<T>(t: T)
where
    T: Trait + OtherTrait,
    <T as Trait>::bikeshed#effect = ~const,
    <T as OtherTrait>::bikeshed#effect = const,
{
    ...
}
```

or

```rust
const fn foo<T: Trait<bikeshed#effect = ~const> + OtherTrait<bikeshed#effect = const>>(t: T) { ... }
```

## Make all `const fn` arguments `~const Trait` by default and require an opt out `?const Trait`

We could default to making all `T: Trait` bounds be const if the function is called from a const context, and require a `T: ?const Trait` opt out
for when a trait bound is only used for its associated types and consts.

This requires a new `~const fn` syntax (sigils or syntax bikesheddable), as the existing `const fn` already has trait bounds that
do not require const trait impls even if used in const contexts.

An example from libstd today is [the impl block of Vec::new](https://github.com/rust-lang/rust/blob/1ab85fbd7474e8ce84d5283548f21472860de3e2/library/alloc/src/vec/mod.rs#L406) which has an implicit `A: Allocator` bound from [the type definition](https://github.com/rust-lang/rust/blob/1ab85fbd7474e8ce84d5283548f21472860de3e2/library/alloc/src/vec/mod.rs#L397).

A full example how how things would look then

```rust
const trait Foo: Bar + ?const Baz {}

impl const Foo for () {}

const fn foo<T: Foo>() -> T {
    // cannot call `Baz` methods
    <T as Bar>::bar()
}

const _: () = foo();
```

This can be achieved across an edition by having some intermediate syntax like prepending `#[next_const]` attributes to all const fn that are using the new syntax, and having a migration lint that suggests adding it to every `const fn` that has trait bounds.

Then in the following edition, we can forbid the `#[next_const]` attribute and just make it the default.

The disadvantage of this is that by default, it creates stricter bounds than desired.

```rust
const fn foo<T: Foo>() {
    T::ASSOC_CONST
}
```

compiles today, and allows all types that implement `Foo`, irrespective of the constness of the impl.
With the opt-out scheme that would still compile, but suddenly require callers to provide a const impl.

The safe default (and the one folks are used to for a few years now on stable), is that trait bounds just work, you just
can't call methods on them.
This is both useful in

* nudging function authors to using the minimal necessary bounds to get their function
body to compile and thus requiring as little as possible from their callers,
* ensuring our implementation is correct by default.

The implementation correctness argument is partially due to our history with `?const` (see https://github.com/rust-lang/rust/issues/83452 for where we got it wrong and thus decided to stop using opt-out), and partially with our history with `?` bounds not being great either (https://github.com/rust-lang/rust/issues/135229, https://github.com/rust-lang/rust/pull/132209). An opt-in is much easier to make sound and keep sound.

To get more capabilities, you add more syntax. Thus the opt-out approach was not taken.

## Per-method constness instead of per-trait

We could require trait authors to declare which methods can be const:

```rust
trait Default {
    const fn default() -> Self;
}
```

This has two major advantages:

* you can now have const and non-const methods in your trait without requiring an opt-out
* you can add new methods with default bodies and don't have to worry about new kinds of breaking changes

The specific syntax given here may be confusing though, as it looks like the function is always const, but
implementations can use non-const impls and thus make the impl not usable for `T: ~const Trait` bounds.

Though this means that changing a non-const fn in the trait to a const fn is a breaking change, as the user may
have that previous-non-const fn as a non-const fn in the impl, causing the entire impl now to not be usable for
`T: ~const Trait` anymore.

See also: out of scope RTN notation in [Unresolved questions](#unresolved-questions)

## Per-method and per-trait constness together:

To get the advantages of the per-method constness alternative above, while avoiding the new kind of breaking change, we can require per-method and per-trait constness:

A mixed version of the above could be

```rust
const trait Foo {
    const fn foo();
    fn bar();
}
```

where you still need to annotate the trait, but also annotate the const methods.

# Prior art
[prior-art]: #prior-art

* I tried to get this accepted before under https://github.com/rust-lang/rfcs/pull/2632.
    * While that moved to [FCP](https://github.com/rust-lang/rfcs/pull/2632#issuecomment-481395097), it had concerns raised.
    * [T-lang discussed this](https://github.com/rust-lang/rfcs/pull/2632#issuecomment-567699174) and had the following open concerns:
        * This design has far-reaching implications and we probably aren't going to be able to work them all out in advance. We probably need to start working through the implementation.
        * This seems like a great fit for the "const eval" project group, and we should schedule a dedicated meeting to talk over the scope of such a group in more detail.
        * Similarly, it would be worth scheduling a meeting to talk out this RFC in more detail and make sure the lang team is understanding it well.
        * We feel comfortable going forward with experimentation on nightly even in advance of this RFC being accepted, as long as that experimentation is gated.
    * All of the above have happened in some form, so I believe it's time to have the T-lang meeting again.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- What parts of the design do you expect to resolve through the RFC process before this gets merged?
    * Whether to pick an alternative syntax (and which one in that case).
- What parts of the design do you expect to resolve through the implementation of this feature before stabilization?
    * We've already handled this since the last RFC, there are no more implementation concerns.
- What related issues do you consider out of scope for this RFC that could be addressed in the future independently of the solution that comes out of this RFC?
    * This RFC's syntax is entirely unrelated to discussions on `async Trait`.
        * `async Trait` can be written entirely in user code by creating a new trait `AsyncTrait`; there is no workaround for `const`.
    * This RFC's syntax is entirely unrelated to discussions on effect syntax.
        * If we get an effect system, it may be desirable to allow expressing const traits with the effect syntax, this design is forward compatible with that.
        * If we get an effect system, we will still want this shorthand, just like we allow you to write:
            * `T: Iterator<Item = U>` and don't require `where T: Iterator, <T as Iterator>::Item = U`.
            * `T: Iterator<Item: Debug>` and don't require `where T: Iterator, <T as Iterator>::Item: Debug`.
    * RTN for per-method bounds: `T: Trait<some_fn(..): ~const Fn(A, B) -> C>` could supplement this feature in the future.
        * Alternatively `where <T as Trait>::some_fn(..): ~const` or `where <T as Trait>::some_fn \ {const}`.
        * Very verbose (need to specify arguments and return type).
        * Want short hand sugar anyway to make it trivial to change a normal function to a const function by just adding some minor annotations.
        * Significantly would delay const trait stabilization (by years).
        * Usually requires editing the trait anyway, so there's no "can constify impls without trait author opt in" silver bullet.
    * New RTN-like per-method bounds: `T: Trait<some_fn(_): ~const>`.
        * Unclear if soundly possible.
        * Unclear if possible without incurring significant performance issues for all code (may need tracking new information for all functions out there).
        * Still requires editing traits.
        * Still want the `~const Trait` sugar anyway.

## Should we start out by allowing only const trait declarations and const trait impls

We do not need to immediately allow using methods on generic parameters of const fn, as a lot of const code is nongeneric.

The following example could be made to work with just const traits and const trait impls.

```rust
const fn foo() {
    let a = [1, 2, 3];
    let b = [1, 2, 4];
    if a == b {}
}
```

Things like `Option::map` could not be made const without const trait bounds, as they need to actually call the generic `FnOnce` argument.


# Future possibilities
[future-possibilities]: #future-possibilities

## Migrate to `~const fn`

`const fn` and `const` items have slightly different meanings for `const`:

`const fn` can also be called at runtime just fine, while the others are always const
contexts and need to be evaluated by the const evaluator.

Additionally `const Trait` bounds have a third meaning (the same as `const Trait` in `impl const Trait for Type`):

They can be invoked at compile time, but also in `const fn`.

While all these meanings are subtly different, making their differences more obvious will not make them easier to understand.
All that changing to `~const fn` would achieve is that folk will add the sigil when told by the compiler, and complain about
having to type a sigil, when there is no meaning for `const fn` without a sigil.

While I see the allure from a language nerd perspective to give every meaning its own syntax, I believe it is much more practical to
just call all of these `const` and only separate the `~const Trait` bounds from `const Trait` bounds.

## `const fn()` pointers

Just like `const fn foo(x: impl ~const Trait) { x.method() }` and `const fn foo(x: &dyn ~const Trait) { x.method() }` we want to allow
`const fn foo(f: const fn()) { f() }`.

There is nothing design-wise blocking function pointers and calling them, they mainly require implementation work and extending the
compiler's internal type system representation of a function signature to include constness.
