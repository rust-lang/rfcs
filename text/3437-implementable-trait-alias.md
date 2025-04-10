- Feature Name: `trait_alias_impl`
- Start Date: 2023-05-24
- RFC PR: [rust-lang/rfcs#3437](https://github.com/rust-lang/rfcs/pull/3437)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary

Extend `#![feature(trait_alias)]` to permit `impl` blocks for most trait aliases.
Also support fully-qualified method call syntax with such aliases.

Additionally, allow trait aliases to have bodies, which can contain `type`s,
`const`s, and.or `fn`s.

# Motivation

Often, one desires to have a "weak" version of a trait, as well as a "strong"
one providing additional guarantees. Specifically, this RFC addresses trait
relationships with the following properties:

- For any implementation of the "strong" variant, there is exactly one way to
  implement the "weak" variant.
- For any implementation of the "weak" variant, there is at most one way to
  implement the "strong" variant.

Subtrait relationships are commonly used to model this, but this often leads to
coherence and backward compatibility issues.

In addition, sometimes one may wish to split a trait into two parts; however,
this is impossible to accomplish backward-compatibly at present.

It is also impossible to rename trait items in a backward-compatible way.

## AFIT `Send` bound aliases

Imagine a library, `frob-lib`, that provides a trait with an async method.

```rust
//! crate `frob-lib`
pub trait Frob {
    async fn frob(&self);
}
```

Most of `frob-lib`'s users will need `Frob::frob`'s return type to be `Send`, so
the library wants to make this common case as painless as possible. But
non-`Send` usage should be supported as well.

### MVP: `trait_variant`

Because Return Type Notation isn't supported yet, `frob-lib` follows the
recommended practice of using the
[`trait-variant`](https://docs.rs/trait-variant/) crate to have `Send` and
non-`Send` variants.

```rust
//! crate `frob-lib`

#[trait_variant::make(Frob: Send)]
pub trait LocalFrob {
    async fn frob(&mut self);
}
```

However, this API has limitations. Fox example, `frob-lib` may want to offer a
`DoubleFrob` wrapper:

```rust
pub struct DoubleFrob<T: LocalFrob>(T);

impl<T: LocalFrob> LocalFrob for DoubleFrob<T> {
    async fn frob(&mut self) {
        self.0.frob().await;
        self.0.frob().await;
    }
}
```

As written, this wrapper only implements `LocalFrob`, which means that it's not
fully compatible with work-stealing executors. So `frob-lib` tries to add a
`Frob` implementation as well:

```rust
impl<T: Frob> Frob for DoubleFrob<T> {
    async fn frob(&mut self) {
        self.0.frob().await;
        self.0.frob().await;
    }
}
```

Coherence, however, rejects this.

```
error[E0119]: conflicting implementations of trait `LocalFrob` for type `DoubleFrob<_>`
 --> src/lib.rs:1:1
  |
1 | #[trait_variant::make(Frob: Send)]
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ conflicting implementation for `DoubleFrob<_>`
...
8 | impl<T: LocalFrob> LocalFrob for DoubleFrob<T> {
  | ---------------------------------------------- first implementation here
  |
  = note: this error originates in the attribute macro `trait_variant::make` (in Nightly builds, run with -Z macro-backtrace for more info)

For more information about this error, try `rustc --explain E0119`.
```

With the `trait_variant`-based design, it's impossible to support both `Send`
and non-`Send` usage in the same `DoubleFrob` type.

### Migrating to Return Type Notation

A few Rust releases later, Return Type Notation is stabilized. `frob-lib` wants
to migrate to it in order to address the issues with the `trait_variant`
solution:

```rust
//! crate `frob-lib``

pub trait LocalFrob {
    async fn frob(&self);
}

// or whatever RTN syntax is decided on
pub trait Frob: LocalFrob<frob(..): Send> + Send {}
impl<T: ?Sized> Frob for T where T: LocalFrob<frob(..): Send> + Send {}
```

However, this is an incompatible change; all implementations of `Frob` are
broken!

```rust
//! crate `downstream`
use frob_lib::Frob;

struct MyType;

impl Frob for MyType {
    // Now an error, "trait `Frob` has no method `frob`"
    async fn frob(&self) { /* ... */ }
}
```

All `impl` blocks for `Frob` must be migrated to reference `LocalFrob` instead.

```rust
//! crate `downstream`
use frob_lib::LocalFrob;

struct MyType;

impl LocalFrob for MyType {
    async fn frob(&self) { /* ... */ }
}
```

Not only is this change disruptive, it also results in more confusing code.
`downstream` is written for work-stealing executors, but needs to reference
`LocalFrob` anyway.

### With today's `#![feature(trait_alias)]`

What if `frob-lib` looked like this instead?

```rust
//! crate `frob-lib`
#![feature(trait_alias)]

pub trait LocalFrob {
    async fn frob(&self);
}

pub trait Frob = LocalFrob<frob(..): Send> + Send;
```

With today's `trait_alias`, it wouldn't make much difference for `downstream`.
`impl` blocks for `Frob` would still be broken.

## Splitting a trait

To exemplify this use-case, we will use [Niko Matsakis’s proposal to split the
`Deref`
trait](https://github.com/rust-lang/rust/pull/135881#issuecomment-2718417230).

`Deref` currently looks like this:

```rust
pub trait Deref {
    type Target: ?Sized;

    fn deref(&self) -> &Self::Target;
}
```

Niko wants to split off the `type Target` part into a separate `Receiver`
supertrait. But there is no backward-compatible way to do this at present.

## Removing a `Sized` bound

Consider this humble library:

```rust
trait Frob {
   type Frobber:

   fn frob(&self, frobber: &Self::Frobber);
}
```

Currently, `Frob::Frobber` has a `Sized` bound, but the signature of `frob()`
doesn't require it. However, there is no backward-compatible way at present for
`frob-lib` to remove the bound.

## Renaming trait items

Consider this verbose library:

```rust
trait TraitForFrobbing {
   type TypeThatEnablesFrobbing:

   fn perform_the_frobbing_operation_posthaste(&self, frobber: &Self::TypeThatEnablesFrobbing);
}
```

The library author may want to rename the trait and its items to something less
unwieldy. Unfortunately, he has no good way to accomplish this at present.

# Guide-level explanation

## `impl` blocks for trait aliases

With `#![feature(trait_alias)]` (RFC #1733), one can define trait aliases, for
use in bounds, trait objects, and `impl Trait`. This feature additionally allows
writing `impl` blocks for a subset of trait aliases.

Let's rewrite our AFIT example from before using this feature. Here's what it
looks like now:

```rust
//! crate `frob-lib`
#![feature(trait_alias)]

pub trait LocalFrob {
    async fn frob(&self);
}

pub trait Frob = LocalFrob<frob(..): Send> 
where
    // not `+ Send`!
    Self: Send;
```

```rust
//! crate `downstream`
#![feature(trait_alias_impl)]

use frob_lib::Frob;

struct MyType;

impl Frob for MyType {
    async fn frob(&self) { /* ... */ }
}
```

`impl`s of `Frob` now Just Work.

## Bodies for trait aliases

Trait aliases can also now sepcify an optional body, which can contain various
items. These items are themselves aliases for items defined on the respective
traits.

```rust
//! crate `foolib`

trait Foo {
    type AssocTy;

    const ASSOC: i32;

    fn method(&self);
    fn another_method(&self);
}

trait QuiteVerboseAlias = Foo {
    type TypeThatIsAssociated = Self::AssocTy;

    const ASSOCIATED_CONSTANT: i32 = Self::ASSOC;

    fn a_method_you_can_call = Self::method;
}
```

You can then refer to these associated items wherever the alias is in
scope:

```rust
fn do_thing<T: QuiteVerboseAlias>(arg: T, another_arg: T::TypeThatIsAssociated) {
    arg.a_method_you_can_call();

    // You can also still use the original names from the aliased trait
    arg.method();
    arg.another_method();
}
```

You can also use the alias names when implementing the trait alias:

```rust
impl QuiteVerboseAlias for () {
    type TypeThatIsAssociated = i32;

    const ASSOC: i32 = 42;

    fn a_method_you_can_call(&self) {
        println!("foo")
    }

    fn another_method(&self) {
        println!("bar")
    }
}
```

## Implementing trait aliases for multiple traits

Trait aliases that combine multiple traits with `+` are also implementable:

```rust
trait Foo {
    fn foo();
}

trait Bar {
    fn bar();
}

trait FooBar = Foo + Bar;

impl FooBar for () {
    fn foo() {
        println!("foo");
    }

    fn bar() {
        println!("bar");
    }
}
```

However, be careful: if both traits have an item of the same name, you won’t be
able to disambiguate, and will have to split the `impl` block into separate
impls for the two underlying traits. Or, alternatively, you can give the trait
alias a body, and define item aliases with distinct names for each of the
conflicting items.

We can use this to split the `Deref` trait, as suggested in the motivation section:

```rust
//! New `Deref`

pub trait Reciever {
    type Target: ?Sized;
}

pub trait DerefToTarget: Reciever {
    fn deref(&self) -> &Self::Target;
}

pub trait Deref = Receiver + DerefToTarget;
```

# Reference-level explanation

## Implementing trait aliases

A trait alias is considered implementable if it includes at least one trait
reference before the `where` keyword. (Henceforth, these are the “primary
traits” of the alias.)`impl`ementing the alias implements these primary traits,
and only these traits. The alias’s `where` clauses are enforced as requirements
that the `impl`ing type must meet—just like `where` clauses in trait
declarations are treated.

```rust
pub trait CopyIterator = Iterator<Item: Copy> where Self: Send;

struct Foo;

impl CopyIterator for Foo {
    type Item = i32; // Would be an error if this was `String`

    fn next(&mut self) -> Self::Item {
        42
    }
}

struct Bar;
impl !Send for Bar;

//impl CopyIterator for Bar { /* ... */ } // ERROR: `Bar` is not `Send`
```

```rust
trait Foo {}
trait Bar = Foo where Self: Send;
//impl<T> Bar for T {} // ERROR: Need to add `T: Send` bound
```

```rust
#![feature(trivial_bounds)]
trait Foo {}
trait Bar = Foo where String: Copy;
//impl Bar for () {} // ERROR: `String: Copy` not satisfied
```

Bounds on generic parameters are also enforced at the `impl` site.

```rust
trait Underlying<T> {}

trait Alias<T: Send> = Underlying<T>;

impl Alias<*const i32> for i32 {} // Error: `*const i32` is not `Send`
```

If the trait alias uniquely constrains a portion of the `impl` block, that part
can be omitted.

```rust
pub trait IntIterator = Iterator<Item = i32> where Self: Send;

struct Baz;

impl IntIterator for Baz {
    // The alias constrains `Self::Item` to `i32`, so we don't need to specify it
    // (though we are allowed to do so if desired).

    fn next(&mut self) -> Option<i32> {
        Some(-27)
    }
}
```

Such constraints can be inferred indirectly:

```rust
trait Bar: Iterator<Item = i32> {}
pub trait IntIterator = Iterator where Self: Bar;

struct Baz;

impl Bar for Baz {}

impl IntIterator for Baz {
    // `IntIterator` requires `Bar`,
    // which requires `Iterator<Item = i32>`,
    // so `Item` must be `i32`
    // and we don't need to specify it.

    fn next(&mut self) -> Option<i32> {
        Some(-27)
    }
}
```

Alias `impl`s also allow omitting implied `#[refine]`s:

```rust
//! crate frob-lib
#![feature(trait_alias)]

pub trait LocalFrob {
    async fn frob(&self);
}

// not `+ Send`!
pub trait Frob = LocalFrob<frob(..): Send> where Self: Send;
```

```rust
//! crate joes-crate
#![feature(trait_alias_impl)]

use frob_lib::Frob;

struct MyType;

impl Frob for MyType {
    // The return future of this method is implicitly `Send`, as implied by the alias.
    // No `#[refine]` is necessary.
    async fn frob(&self) { /* ... */ }
}
```

Trait aliases are `unsafe` to implement iff one or more primary traits are
marked `unsafe`.

## Usage in paths

Trait aliases can also be used with trait-qualified and fully-qualified method
call syntax, as well as in paths more generally. When used this way, they are
treated equivalently to the underlying primary trait(s), with the additional
restriction that all `where` clauses and type parameter/associated type bounds
must be satisfied.

```rust
use std::array;

trait IntIter = Iterator<Item = u32> where Self: Clone;

let iter = [1_u32].into_iter();
let _: IntIter::Item = IntIter::next(&mut iter); // works
let _: <array::IntoIter as IntIter>::Item = <array::IntoIter as IntIter>::next(); // works
IntIter::clone(&iter);
let dyn_iter: &mut dyn Iterator<Item = u32> = &mut iter;
//IntIter::next(dyn_iter); // ERROR: `dyn Iterator<Item = u32>` does not implement `Clone`
let signed_iter = [1_i32].into_iter();
//IntIter::next(&mut signed_iter); // ERROR: Expected `<Self as Iterator>::Item` to be `u32`, it is `i32`
```

Implementable trait aliases can also be used with associated type bounds.

```rust
trait IteratorAlias = Iterator;
let _: IteratorAlias<Item = u32> = [1_u32].into_iter();

trait IntIter = Iterator<Item = u32> where Self: Clone;
let _: IntIter<Item = u32> = [1_u32].into_iter(); // `Item = u32` is redundant, but allowed
//let _: IntIter<Item = f64> = [1.0_f64].into_iter(); // ERROR: `Item = f64` conflicts with `Item = u32`
```

Items from traits in `where` clauses of the alias are accessible, unless
shadowed by items in the primary trait(s):

```rust
trait Foo {
    fn frob();
    fn frit();
}

trait Bar {
    fn frit();
    fn bork();
}

trait FooBar = Bar where Self: Foo;

fn example<T: FooBar>() {
    T::frob(); // resolves to `<T as Foo>::frob`
    T::frit(); // resolves to `<T as Bar>::frit`
    T::bork(); // resolves to `<T as Bar>::bork`
}
```

## Aliases with multiple primary traits

A trait alias with multiple primary traits can be implemented, unless one of the
primary traits requires specifying an item that conflicts with an item of the
same name in a different primary trait.

```rust
trait Foo {
    fn frob();
}

trait Bar {
    fn frob() {}
}

// This isn't implementable, due to conflict between `Foo::frob` and `Bar::frob`
trait FooBar = Foo + Bar;
```

If the confliting items all have defaults, the alias will be implementable, but
overriding the defaults will not be possible.

```rust
trait Foo {
    fn frob() {}
}

trait Bar {
    fn frob() {}
}

// This is implementable, but the `impl` block won't be able
// to override the default bodies of the `frob()` functions.
trait FooBar = Foo + Bar;
```

Name conflicts of this sort also cause ambiguity when using the alias:

```rust
fn example<T: FooBar>() {
    T::frob(); // ERROR: ambiguous
}
```

To resolve these conflicts, you can use trait alias bodies, as described below.

## Bodies for trait aliases

Trait aliases can now optionally contain a body, specifying aliases for various
items. These can be types, constants, or functions.

### `type`s and `const` items in trait alias bodies

```rust
trait Foo {
    type Assoc;
    const ASSOC: i32;
}

trait Alias = Foo {
    type AssocVec = Vec<Self::Assoc>;
    const ASSOC_PLUS_1: i32 = Self::ASSOC + 1;
}
```

`<T as Alias>::AssocVec` means the same thing as `Vec<<T as Foo>::Assoc>`, and
`<T as Alias>::ASSOC_PLUS_1` is equivalent to `const { <T as Foo>::ASSOC + 1 }`.

To be implementable, a `type` or `const` alias item must obey certain
restrictions. It must either be set equal to an item of a primary trait of the
alias:

```rust
trait Foo {
    type Assoc;
    const ASSOC: i32;
}

trait Alias = Foo {
    type Associated = Self::Assoc;
    const ASSOCIATED: i32 = Self::ASSOC;
}

impl Alias for () {
    type Associated = i32; // Equivalent to `type Assoc = i32;`
    const ASSOCIATED: i32 = 42; // Equivalent to `const ASSOC: i32 = 42;`
}
```

Or, the trait alias must set an associated type of the primary trait equal to a
generic type, with the alias item as a generic parameter of that type. For
example, here is
[`TryFuture`](https://docs.rs/futures-core/latest/futures_core/future/trait.TryFuture.html)
as an implementable trait alias:

```rust
/// This means:
/// "A `TryFuture` is a `Future` where there exist
/// unique types `Self::Ok` and `Self::Error` such that
/// `Self: Future<Output = Result<Self::Ok, Self::Error>>`."
pub trait TryFuture = Future<Output = Result<Self::Ok, Self::Error>> {
    // The values of these `type`s are defined by the `Output = ...` above.
    // So there is no need for `= ...` RHS
    type Ok;
    type Error;
}

// Example impl

struct AlwaysFails;

impl TryFuture for AlwaysFails {
    type Ok = !;
    type Error = ();

    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<!, ()>> {
        Poll::Ready(Err(()))
    }
}
```

The generic parameter can also be nested:

```rust
trait Foo {
    type Assoc;
}

trait Bar = Foo<Assoc = Result<Self::Foo, Vec<Self::Foo>>> {
    type Foo;
}
```

Items defined in a trait alias body shadow items of the same name in a primary
trait.

```rust
trait Foo {
    type Assoc;
}

trait Bar {
    type Assoc;
}

trait FooBar = Foo + Bar {
    type Assoc = <Self as Foo>::Assoc; // `FooBar::Assoc` will resolve to `Foo::Assoc`
    type BarAssoc = <Self as Bar>::Assoc;
}
```

#### GATs in type alias bodies

Type alias bodies can also contain GATs. These are also subject to the
implementability rules, though reordering generic parameters does not inhibit
implementability.

```rust
trait Foo {
    type Gat<'a, T, U>
    where
       Self: 'a;
}

trait Alias = Foo {
    type Tag<'a, U, T> = Self::Gat<'a, T, U>; // Implementable
    type GatVec<'a, T, U> = Self::Gat<'a, Vec<T>, U>; // Not implementable
    type GatSame<'a, T> = Self::Gat<'a, T, T>; // Not implementable
}
```

### `fn`s in type alias bodies

#### Implementable `fn`s

Trait alias bodies can also contain aliases for methods of its primary trait(s).
This involves a new syntax form for implementable function aliases:

```rust
trait Frob {
    fn frob(&self);
}

trait Alias = Frob {
    fn method = Self::frob; // `Alias::method()` is equivalent to `Frob::frob()`
}
```

Effect keywords like `const`, `async`, or `unsafe` do not need to be specified.

You are allowed to specify generic parameters, in order to reorder them. But you
don't have to:

```rust
trait Frob {
    fn frob<T, U>(&self);
}

trait Alias = Frob {
    fn alias = Self::frob; // OK
    fn alias2<T, U> = Self::frob<U, T>; // Also OK
    //fn alias3<T> = Self::frob<T, T>; // ERROR, as would not be implementable
}
```

#### Non-implementable `fn`s

A trait alias body can also contain non-alias `fn`s, with bodies. These are not
implementable:

```rust
trait Frob {
    fn frob(&self) -> i32;
}

trait Alias = Frob {
    #[must_use]
    fn frob_twice(&self) -> i32 {
        self.frob() + self.frob()
    }
}
```

This is similar to defining an extension trait like
[`Itertools`](https://docs.rs/itertools/latest/itertools/trait.Itertools.html).
(One difference from extension traits is that trait aliases do not create their
own `dyn` types.)

## Interaction with `dyn`

Trait aliases do not define their own `dyn` types. This RFC does not change that
pre-existing behavior. However, we do make one change to which trait aliases
also define a type alias for a trait object. If a trait alias contains multiple
non-auto traits (primary or not), but one of them is a subtrait of all the
others, then the corresponding `dyn` type for that trait alias is now an alias
for the `dyn` type for that subtrait.

This is necessary to support the `Deref` example from earlier.

```rust
trait Foo {
    fn foo(&self);
}
trait Bar: Foo {
    fn bar(&self);
}

trait FooBar = Foo + Bar; // `dyn FooBar` is an alias of `dyn Bar`
trait FooBar2 = Foo
where
    Self: Bar; // `dyn FooBar2` is also an alias of `dyn Bar`
```

N.B.: when using implementable trait aliases to split a trait into two parts
*without* a supertrait/subtrait relationship between them, you have to be
careful in order to preserve `dyn` compatiblilty.

```rust
trait Foo {
    fn foo(&self);
}
trait Bar {
    fn bar(&self);
}

trait FooBar = Foo + Bar; // `dyn FooBar` is not a valid type!
```

To make it work, you can do:

```rust
trait Foo {
    fn foo(&self);
}

trait Bar {
    fn bar(&self);
}

#[doc(hidden)]
trait FooBarDyn: Foo + Bar {}
impl<T: Foo + Bar + ?Sized> FooBarDyn for T {}

trait FooBar = Foo + Bar
where
    Self: FooBarDyn; // `dyn FooBar` now works just fine
```

# Drawbacks

- The fact that `trait Foo = Bar + Send;` means something different than `trait
  Foo = Bar where Self: Send;` will likely be surprising to many.
- Adds complexity to the language. In particular, trait alias bodies introduce a
  large amount of new syntax and complexity, but will likely be rarely used.
- There is a lot of overlap between trait alias bodies and extension traits.

# Rationale and alternatives

## Require an attribute to mark the trait as implementable

We could require an attribute on implementable aliases; e.g. `#[implementable]
trait Foo = ...`. However, there is not much reason to opt out of
implementability.

## No trait alias bodies

Not including this part of the proposal would significantly decrease the overall
complexity of the feature. However, it would also reduce its power: trait
aliases could no longer be used to rename trait items, and naming conflicts in
multi-primary-trait aliases would be impossible to resolve.

It's this last issue especially that leads me to not relegate this to a future
possibility. Adding a defaulted item to a trait should at most require minor
changes to dependents, and restructuring a large `impl` block is not “minor”.

## No non-implementable items in trait alias bodies

Such items don't have much utility from a backward-compatibility perspective,
and overlap with extension traits. However, the cost of allowing them is very
low.

## Unconstrained generic parameters

A previous version of this RFC required generic parameters of implementable
trait aliases to be used as generic parameters of a primary trait of the alias.
This restriction was meant to avoid surprising errors:

```rust
trait Foo<T> = Copy;

#[derive(Clone)]
struct MyType;

impl<T> Foo<T> for MyType {} // ERROR: `T`` is unconstrained
```

```rust
trait Foo<T> = Iterator<Item = T>;

struct MyType;

impl Foo<u32> for MyType {
    fn next(&mut Self) -> Option<u32> {
        todo!()
    }
}

impl Foo<i32> for MyType { // ERROR: overlapping impls
    fn next(&mut Self) -> Option<i32> {
        todo!()
    }
}
```

However, upon further discussion, I now lean toward allowing more flexibility,
even at the risk of potential confusion.

## Allow `impl Foo + Bar for Type { ... }` directly, without an alias

It's a forward-compatibility hazard (if the traits gain items with conflicting
names), with no use-case that I can see.

## Implementing aliases with 0 primary traits

We could allow implementing aliases with no primary traits, as a no-op. However,
I don't see the point in it.

# Prior art

- [`trait_transformer` macro](https://github.com/google/impl_trait_utils)

# Unresolved questions

- How does `rustdoc` render these?

# Future possibilities

- New kinds of bounds: anything that makes `where` clauses more powerful would
  make this feature more powerful as well.
  - Variance bounds could allow this feature to support backward-compatible
    GATification.
- We could allow trait aliases to define their own defaults for `impl`s. One
  possibility is [the `default partial impl` syntax I suggested on
  IRLO](https://internals.rust-lang.org/t/idea-partial-impls/22706/).
- We could allow implementable `fn` aliases in non-alias `trait` definitions.
- We could allow any `impl` block to implement items from supertraits of its
  primary trait(s).
  - This would allow splitting a trait into a supertrait and a subtrait without
    having to give the subtrait a new name.
  - However, it would make it more difficult to deduce what traits an `impl`
    block is implementing.
  - In addition, it poses a danger if an `unsafe` subtrait depends on an
    `unsafe` marker supertrait: you could implement the subtrait, carefully
    checking that you meet its preconditions, while not realizing that you are
    also implementing the supertrait and need to check its conditions as well.
  - And even if the traits are not `unsafe`, they could still have preconditions
    that are important for correctness. Users should never be committing to such
    things unknowingly.
- We could add an attribute for trait aliases to opt in to generating their own
  `dyn` type.
  - This could be prototyped as a proc macro.
