- Feature Name: `trait_alias_impl`
- Start Date: 2023-05-24
- RFC PR: [rust-lang/rfcs#3437](https://github.com/rust-lang/rfcs/pull/3437)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary

Extend `#![feature(trait_alias)]` to permit `impl` blocks for trait aliases with a single primary trait. Also support fully-qualified method call syntax with such aliases.

# Motivation

Often, one desires to have a "weak" version of a trait, as well as a "strong" one providing additional guarantees. Subtrait relationships are commonly used for this, but they sometimes fall short—expecially when the "strong" version is expected to see more use, or was stabilized first.

## Example: AFIT `Send` bound aliases

### With subtraits

Imagine a library, `frob-lib`, that provides a trait with an async method. (Think `tower::Service`.)

```rust
//! crate frob-lib
pub trait Frobber {
    async fn frob(&self);
}
```

Most of `frob-lib`'s users will need `Frobber::frob`'s return type to be `Send`, so the library wants to make this common case as painless as possible. But non-`Send` usage should be supported as well.

`frob-lib`, following the recommended practice, decides to design its API in the following way:

```rust
//! crate frob-lib

pub trait LocalFrobber {
    async fn frob(&self);
}

// or whatever RTN syntax is decided on
pub trait Frobber: LocalFrobber<frob(..): Send> + Send {}
impl<T: ?Sized> Frobber for T where T: LocalFrobber<frob(..): Send> + Send {}
```

These two traits are, in a sense, one trait with two forms: the "weak" `LocalFrobber`, and "strong" `Frobber` that offers an additional `Send` guarantee.

Because `Frobber` (with `Send` bound) is the common case, `frob-lib`'s documentation and examples put it front and center. So naturally, Joe User tries to implement `Frobber` for his own type.

```rust
//! crate joes-crate
use frob_lib::Frobber;

struct MyType;

impl Frobber for MyType {
    async fn frob(&self) {
        println!("Sloo is 120% klutzed. Initiating brop sequence...")
    }
}
```

But one `cargo check` later, Joe is greeted with:

```
error[E0277]: the trait bound `MyType: LocalFrobber` is not satisfied
  --> src/lib.rs:6:18
   |
6  | impl Frobber for MyType {
   |                  ^^^^^^ the trait `LocalFrobber` is not implemented for `MyType`

error[E0407]: method `frob` is not a member of trait `Frobber`
  --> src/lib.rs:7:5
   |
7  | /     async fn frob(&self) {
8  | |         println!("Sloo is 120% klutzed. Initiating brop sequence...")
9  | |     }
   | |_____^ not a member of trait `Frobber`
```

Joe is confused. "What's a `LocalFrobber`? Isn't that only for non-`Send` use cases? Why do I need to care about all that?" But he eventually figures it out:

```rust
//! crate joes-crate
use frob_lib::LocalFrobber;

struct MyType;

impl LocalFrobber for MyType {
    #[refine]
    async fn frob(&self) {
        println!("Sloo is 120% klutzed. Initiating brop sequence...")
    }
}
```

This is distinctly worse. Joe now has to reference both `Frobber` and `LocalFrobber` in his code, and (assuming that the final AFIT feature ends up requiring it) also has to write `#[refine]`.

### With today's `#![feature(trait_alias)]`

What if `frob-lib` looked like this instead?

```rust
//! crate frob-lib
#![feature(trait_alias)]

pub trait LocalFrobber {
    async fn frob(&self);
}

pub trait Frobber = LocalFrobber<frob(..): Send> + Send;
```

With today's `trait_alias`, it wouldn't make much difference for Joe. He would just get a slightly different error message:

```
error[E0404]: expected trait, found trait alias `Frobber`
  --> src/lib.rs:6:6
   |
6  | impl Frobber for MyType {
   |      ^^^^^^^ not a trait
```

## Speculative example: GATification of `Iterator`

*This example relies on some language features that are currently pure speculation.Implementable trait aliases are potentially necessary to support this use-case, but not sufficent.*

Ever since the GAT MVP was stabilized, there has been discussion about how to add `LendingIterator` to the standard library, without breaking existing uses of `Iterator`. The relationship between `LendingIterator` and `Iterator` is "weak"/"strong"—an `Iterator` is a `LendingIterator` with some extra guarantees about the `Item` associated type.

Now, let's imagine that Rust had some form of "variance bounds", that allowed restricting the way in which a type's GAT can depend on said GAT's generic parameters. One could then define `Iterator` in terms of `LendingIterator`, like so:

```rust
//! core::iter
pub trait LendingIterator {
    type Item<'a>
    where
        Self: 'a;

    fn next(&'a mut self) -> Self::Item<'a>;
}

pub trait Iterator = LendingIterator
where
    // speculative syntax, just for the sake of this example
    for<'a> Self::Item<'a>: bivariant_in<'a>;
```

But, as with the previous example, we are foiled by the fact that trait aliases aren't `impl`ementable, so this change would break every `impl Iterator` block in existence.

## Speculative example: `Async` trait

There has been some discussion about a variant of the `Future` trait with an `unsafe` poll method, to support structured concurrency ([here](https://rust-lang.github.io/wg-async/vision/roadmap/scopes/capability/variant_async_trait.html) for example). *If* such a change ever happens, then the same "weak"/"strong" relationship will arise: the safe-to-poll `Future` trait would be a "strong" version of the unsafe-to-poll `Async`. As the linked design notes explain, there are major problems with expressing that relationship in today's Rust.

# Guide-level explanation

With `#![feature(trait_alias)]` (RFC #1733), one can define trait aliases, for use in bounds, trait objects, and `impl Trait`. This feature additionaly allows writing `impl` blocks for a subset of trait aliases.

Let's rewrite our AFIT example from before, in terms of this feature. Here's what it looks like now:

```rust
//! crate frob-lib
#![feature(trait_alias)]

pub trait LocalFrobber {
    async fn frob(&self);
}

pub trait Frobber = LocalFrobber<frob(..): Send> 
where
    // not `+ Send`!
    Self: Send;
```

```rust
//! crate joes-crate
#![feature(trait_alias_impl)]

use frob_lib::Frobber;

struct MyType;

impl Frobber for MyType {
    async fn frob(&self) {
        println!("Sloo is 120% klutzed. Initiating brop sequence...")
    }
}
```

Joe's original code Just Works.

The rule of thumb is: if you can copy everything between the `=` and `;` of a trait alias, paste it between the `for` and `{` of a trait `impl` block, and the result is sytactically valid—then the trait alias is most likely implementable.

# Reference-level explanation

A trait alias has the following syntax (using the Rust Reference's notation):

> [Visibility](https://doc.rust-lang.org/stable/reference/visibility-and-privacy.html)<sup>?</sup> `trait` [IDENTIFIER](https://doc.rust-lang.org/stable/reference/identifiers.html) [GenericParams](https://doc.rust-lang.org/stable/reference/items/generics.html)<sup>?</sup> `=` [TypeParamBounds](https://doc.rust-lang.org/stable/reference/trait-bounds.html)<sup>?</sup> [WhereClause](https://doc.rust-lang.org/stable/reference/items/generics.html#where-clauses)<sup>?</sup> `;`

For example, `trait Foo<T> = PartialEq<T> + Send where Self: Sync;` is a valid trait alias.

Implementable trait aliases must follow a more restrictive form:

> [Visibility](https://doc.rust-lang.org/stable/reference/visibility-and-privacy.html)<sup>?</sup> `trait` [IDENTIFIER](https://doc.rust-lang.org/stable/reference/identifiers.html) [GenericParams](https://doc.rust-lang.org/stable/reference/items/generics.html)<sup>?</sup> `=` [TypePath](https://doc.rust-lang.org/stable/reference/paths.html#paths-in-types) [WhereClause](https://doc.rust-lang.org/stable/reference/items/generics.html#where-clauses)<sup>?</sup> `;`

For example, `trait Foo<T> = PartialEq<T> where Self: Sync;` is a valid implementable alias. The `=` must be followed by a single trait (or implementable trait alias), and then some number of where clauses.

An impl block for a trait alias looks just like an impl block for the underlying trait. The alias's where clauses are treated as if they had been written out in the `impl` header.

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

// ERROR: `Bar` is not `Send`
// impl IntIterator for Bar { /* ... */ }
```

There is another restriction that trait aliases must adhere to in order to be implementable: all generic parameters of the alias itself must be used as generic parameters of the alias's primary trait.

```rust
// Implementable
trait Foo<T> = PartialEq<T>;

// Not implementable
trait Foo<T> = Copy;
trait Foo<T> = Copy where T: Send;
trait Foo<T> = Iterator<Item = T>;
trait Foo<T> = Copy where Self: PartialEq<T>;
```

Bounds on such generic parameters are enforced at the `impl` site.

```rust
trait Underlying<T> {}

trait Alias<T: Send> = Underlying<T>;

impl<T> Alias<T> for i32 {} // Error: missing `T: Send` bound
```

If the trait alias uniquely constrains a portion of the `impl` block, that part can be omitted.

```rust
pub trait IntIterator = Iterator<Item = i32> where Self: Send;

struct Baz;

impl IntIterator for Baz {
    // The alias constrains `Self::Item` to `i32`, so we don't need to specify it
    // (though we are allowed to do so if desired).
    // type Item = i32;

    fn next(&mut self) -> i32 {
        -27
    }
}
```

Alias `impl`s also allow omitting implied `#[refine]`s:

```rust
//! crate frob-lib
#![feature(trait_alias)]

pub trait LocalFrobber {
    async fn frob(&self);
}

// not `+ Send`!
pub trait Frobber = LocalFrobber<frob(..): Send> where Self: Send;
```

```rust
//! crate joes-crate
#![feature(trait_alias_impl)]

use frob_lib::Frobber;

struct MyType;

impl Frobber for MyType {
    // The return future of this method is implicitly `Send`, as implied by the alias.
    // No `#[refine]` is necessary.
    async fn frob(&self) {
        println!("Sloo is 120% klutzed. Initiating brop sequence...")
    }
}
```

Trait aliases are `unsafe` to implement iff the underlying trait is marked `unsafe`.

Implementable trait aliases can also be used with trait-qualified and fully-qualified method call syntax, as well as in paths more generally. When used this way, they are treated equivalently to the underlying primary trait, with the additional restriction that all `where` clauses and type parameter/associated type bounds must be satisfied.

```rust
trait IntIter = Iterator<Item = u32> where Self: Clone;

fn foo() {
    let iter = [1_u32].into_iter();
    let _: IntIter::Item = IntIter::next(&mut iter); // works
    let _: <std::array::IntoIter as IntIter>::Item = <std::array::IntoIter as IntIter>::next(); // works
    //IntIter::clone(&iter); // ERROR: trait `Iterator` has no method named `clone()`
    let dyn_iter: &mut dyn Iterator<Item = u32> = &mut iter;
    //IntIter::next(dyn_iter); // ERROR: `dyn Iterator<Item = u32>` does not implement `Clone`
    let signed_iter = [1_i32].into_iter();
    //IntIter::next(&mut signed_iter); // ERROR: Expected `<Self as Iterator>::Item` to be `u32`, it is `i32`
}
```

# Drawbacks

- The sytactic distance between implementable and non-implementable aliases is short, which might confuse users. In particular, the fact that `trait Foo = Bar + Send;` means something different than `trait Foo = Bar where Self: Send;` will likely be surprising to many.
  - On the other hand, the rules mirror those of `impl` blocks, which Rust programmers already understand.
  - Ideally, we would collect user feedback before stabilizing this feature.
- Adds complexity to the language, which might surprise or confuse users.
- Many of the motivating use-cases involve language features that are not yet stable, or even merely speculative. More experience with those features might unearth better alternatives.

# Rationale and alternatives

- Very lightweight, with no new syntax forms. Compare "trait transformers" proposals, for example—they are generally much heavier.
- Better ergonomics compared to purely proc-macro based solutions.
- One alternative is to allow marker traits or auto traits to appear in `+` bounds of implementable aliases.
(For example, `trait Foo = Bar + Send;` could be made implementable).
  - This may make the implementablility rules more intutive to some, as the distinction between `+ Send` and `where Self: Send` would no longer be present.
  - However, it also might make the rules less intuitive, as the symmetry with `impl` blocks would be broken.
  - Again, user feedback could help make this decision.
- Another option is to require an attribute on implementable aliases; e.g. `#[implementable] trait Foo = ...`. This would make the otherwise-subtle implementability rules more explicit, at the cost of cluttering user code and the attribute namespace.

## What about combining multiple primary traits, and their items, into one impl block?

It's possible to imagine an extension of this proposal, that allows trait aliases to be implementable even if they have multiple primary traits. For example:

```rust
trait Foo = Clone + PartialEq;

struct Stu;

impl Foo for Stu {
    fn clone(&self) -> Self {
        Stu
    }

    fn eq(&self, other: &Self) -> bool {
        true
    }
}
```

Such a feature could be useful when a trait has multiple items and you want to split it in two.

However, there are some issues. Most glaring is the risk of name collisions:

```rust
trait A {
    fn foo();
}

trait B {
    fn foo();
}

// How would you write an `impl` block for this?
trait C = A + B;
```

Such a feature could also make it harder to find the declaration of a trait item from its implementation, especially if IDE "go to definition" is not available. One would need to first find the trait alias definition, and then look through every primary trait to find the item. (However, given the current situation with postfix method call syntax, maybe this is an acceptable tradeoff.)

Perhaps a more narrowly tailored version of this extension, in which both subtrait and supertrait explicitly opt-in to support sharing an `impl` block with one another, would satisfy the backward-compatibility use-case while avoiding the above issues. I think exploring that is best left to a future RFC.

# Prior art

- [`trait_transformer` macro](https://github.com/google/impl_trait_utils)

# Unresolved questions

- How does `rustdoc` render these? Consider the `Frobber` example—ideally, `Frobber` should be emphasized compared to `LocalFrobber`, but it's not clear how that would work.

# Future possibilities

- New kinds of bounds: anything that makes `where` clauses more powerful would make this feature more powerful as well.
  - Variance bounds would allow this feature to support backward-compatible GATification.
  - Method unsafety bounds would support the `Future` → `Async` use-case.
- `trait Foo: Copy = Iterator;` could be allowed as an alternative to `trait Foo = Iterator where Self: Copy;`.
- The possible contents of `impl` bodies could be expanded, for example to support combining supertrait and subtrait implementations.