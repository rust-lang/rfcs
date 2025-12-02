- Feature Name: `supertrait_auto_impl`
- Start Date: 2025-08-26
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/3851)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

We would like to allow nested trait `impl` blocks in `trait` defintion blocks and `impl Trait for` blocks, so that users can supply supertrait items in subtrait contexts.

```rust
trait Supertrait1 {
    type Type;
}
trait Supertrait2 {
    type Type1;
    type Type2;
}
trait Subtrait1: Supertrait1 {
    auto impl Supertrait1;
}
impl Subtrait1 for MyType {
    type Type = u8; // implicitly implements `Supertrait1::Type := u8`
}

trait Subtrait2: Supertrait2 {
    auto impl Supertrait2 {
        type Type1 = Self;
        type Type2 = ();
    }
}
impl Subtrait2 for MyType {
    // An implicit `impl Supertrait2 for MyType` is generated
    // with `Type1 := MyType` and `Type2 := ()` as backfill
}
```

# Motivation
[motivation]: #motivation

## Trait evolution

Trait evolution is a treatment to existing trait hierarchy in a library. Difficulty has arised in the past that hoisting items from the current trait into a new supertrait, or introduction of a second trait.

This RFC promises to improve the situation around trait evolution. It captures the common cases under this theme and aims to reduce rewrites in downstream crates, should the need to re-organise trait hierarchy arises.

### Example: trait refinement by item hoisting into supertraits

As library code grows, there is frequently a need to breakdown a big trait into several smaller trait. This would have been a breaking change in view of SemVer and a user code rewrite is mandatory. However, the aim of this RFC is to ease the transition of downstream trait implementors to the new trait hierarchy by reducing the rewrites.  

Suppose that we start with a big trait `Subtrait` and it becomes desirable that `candidate_for_hoisting` method is hoisted into another trait.
```rust
trait Subtrait {
    fn candidate_for_hoisting(&self);
    fn subtrait_method(&self);
}
// downstream crate
impl Subtrait for MyType {
    fn candidate_for_hoisting(&self) { .. }
    fn subtrait_method(&self) { .. }
}
fn assert(x: impl Subtrait)
where
    MyType: Subtrait
{
    x.candidate_for_hoisting()
}
```
With this RFC, it is possible for the library author to perform the following refactor.
```rust
trait Supertrait {
    fn candidate_for_hoisting(&self); // <~ hoisted
}
trait Subtrait: Supertrait {
    auto impl Supertrait;
    fn subtrait_method(&self);
}

// downstream crate: no rewrites are required
impl Subtrait for MyType {
    fn candidate_for_hoisting(&self) { .. }
    // ^ this is resolved as implementor of `Supertrait::candidate_for_hoisting`

    fn subtrait_method(&self) { .. }
}
fn assert(x: impl Subtrait)
where
    MyType: Subtrait
{
    x.candidate_for_hoisting() // <~ method resolves to `Supertrait::candidate_for_hoisting`
}
```

### Example: relaxed bounds via new supertraits
A common use case of supertraits is weaken bounds involved in associated items. There are occassions that a weakend supertrait could be useful. Suppose that we have a factory trait in the following example. In this example, the `async fn make` factory method could be weakened so that the future returned could be used in the context where the future is not required to be of `Send`. This has been enabled through the use of [the `trait_variant` crate](https://docs.rs/trait-variant/latest/trait_variant/). The [`tower::Service`](https://docs.rs/tower/latest/tower/trait.Service.html) trait would benefit greatly from this proposal by having also the `!Send` bound for local service without major refactoring.

```rust
#[trait_variant::make(IntFactory: Send)]
trait LocalIntFactory {
    async fn make(&self) -> i32;
    fn stream(&self) -> impl Iterator<Item = i32>;
    fn call(&self) -> u32;
}

// `trait_variant` will generate a conceptual subtrait:

trait IntFactory: Send {
    fn make(&self) -> impl Future<Output = i32> + Send;
    fn stream(&self) -> impl Iterator<Item = i32> + Send;
    fn call(&self) -> u32;
}
```

This RFC enables one to construct the trait in the following fashion.
```rust
trait LocalIntFactory {
    async fn make(&self) -> i32;
    fn stream(&self) -> impl Iterator<Item = u32>;
    fn call(&self) -> u32;
}
trait IntFactory: Send {
    auto impl LocalIntFactory {
        async fn make(&self) -> i32 {
            IntFactory::make(self).await
        }
        fn stream(&self) -> impl Iterator<Item = u32> {
            IntFactory::stream(self)
        }
        fn call(&self) -> u32 {
            IntFactory::call(self)
        }
    }
    fn make(&self) -> impl Future<Output = i32> + Send;
    fn stream(&self) -> impl Iterator<Item = i32> + Send;
    fn call(&self) -> u32;
}
```

### <a id="po-o"></a>Example: automatic supertrait implementation

A second prominent example is the `PartialOrd` and `Ord` traits.
```rust
trait PartialOrd<Rhs = Self> {
    fn partial_cmp(&self, other: &Rhs) -> Option<Ordering>;
    // ...
}
trait Ord<Rhs = Self>: PartialOrd<Rhs> {
    fn cmp(&self, other: &Rhs) -> Ordering;
    // ...
}
// This is one of the more probably implementation:
impl PartialOrd for X {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for X {
    fn cmp(&self, other: &Self) -> Ordering {
        // here it defines a total ordering
        ..
    }
}
```

The `PartialOrd` trait could be reworked in this proposal as follows.
```rust
trait PartialOrd<Rhs = Self> {
    fn partial_cmp(&self, other: &Rhs) -> Option<Ordering>;
}
trait Ord<Rhs = Self>: PartialOrd<Rhs> {
    fn cmp(&self, other: &Rhs) -> Ordering;
    auto impl PartialOrd<Rhs> {
        fn partial_cmp(&self, other: &Rhs) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }
}
```
There are now two choices for type `X` in the downstream crate.
- Delete the `impl PartialOrd for X`. Without the overlapping `impl`, the `auto impl` can stand in and take effect.
```rust
// delete: impl PartialOrd for X { .. }
impl Ord for X {
    fn cmp(&self, other: &Rhs) -> Ordering {
        // here it defines the same total ordering
    }
}
```
- Declare use of the existing applicable `impl PartialOrd for X`.
```rust
impl PartialOrd for X {
    // this is the same implementation
}
impl Ord for X {
    extern impl PartialOrd;
    fn cmp(&self, other: &Rhs) -> Ordering {
        // here it defines the same total ordering
    }
}
```

### Example: A possible [`ToOwned`](https://doc.rust-lang.org/stable/std/borrow/trait.ToOwned.html) refactor

As of writing, `ToOwned` is defined as follows.
```rust
pub trait ToOwned {
    type Owned: Borrow<Self>;

    fn to_owned(&self) -> Self::Owned;

    fn clone_into(&self, target: &mut Self::Owned) { ... }
}
```

`ToOwned` is a trait that could be further refined into the `AsOwned` concept and itself.

```rust
pub trait AsOwned {
    type Owned: Borrow<Self>;
}

pub trait ToOwned: AsOwned {
    auto impl AsOwned;

    fn to_owned(&self) -> Self::Owned;

    fn clone_into(&self, target: &mut Self::Owned) {
        // same implementation
    }
}
```

The appeal of this refinement is that there is a proper separation between the capability to *borrow out data* and that to *take and own data*. This enables `Cow<'_, _>` to be implemented in the following snippet.

```rust
// Note that we do not out right require that the data can be taken and owned ...
pub enum Cow<'a, B: AsOwned + ?Sized> {
    Borrowed(&'a B),
    Owned(B::Owned),
}

impl<B: AsOwned + ?Sized> Deref for Cow<'_, B> {
    type Target = B;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Borrowed(v) => v,
            Self::Owned(v) => v.borrow(),
        }
    }
}

impl<B: ToOwned + ?Sized> Cow<'_, B> {
    // ... until the need arises here.
    pub fn into_owned(self) -> B::Owned {
        match self {
            Self::Borrowed(v) => v.to_owned(),
            Self::Owned(v) => v,
        }
    }
}
```

## Helpers for implementing traits

This is not the main motivation for this RFC, but it is a secondary additional feature that this RFC enables.

For some traits, it's difficult to implement the trait directly because the "raw" interface that the trait exposes is complex. If the trait is unsafe, this may be even worse as the end-user may not wish to write any unsafe code. This feature makes it easy to provide utilities for more easily implementing the trait.

### Example: Serde

The serde traits are notoriously difficult to implement directly. It's almost always done by macro. Imagine if you could write this:
```rs
struct MyStruct {
    name: String,
    int: i32,
}

#[derive(Serialize)]
struct MyStructProxy<'a> {
    name: &'a str,
    tens: i32,
    digit: i32,
}

trait SerializeByProxy: Serialize {
    // See https://docs.rs/serde/latest/serde/trait.Serialize.html
    auto impl Serialize {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            // First construct the proxy
            let proxy = SerializeByProxy::serialize(self);
            // Then delegate the serialization to the proxy
            proxy.serialize(serializer)
        }
    }

    type Proxy<'a>: Serialize
    where Self: 'a;

    fn serialize(&self) -> Proxy<'_>;
}

impl SerializeByProxy for MyStruct {
    type Proxy<'a> = MyStructProxy<'a>
    where Self: 'a;

    fn serialize(&self) -> MyStructProxy<'_> {
        MyStructProxy {
            name: &self.name,
            digit: self.int % 10,
            tens: self.int / 10,
        }
    }
    // Now `MyStruct` is also automatically `Serialize`
    // via a proxy.
}
```
And then `MyStruct` automatically implements `Serialize` by creating a `MyStructProxy` instance and serializing the proxy. So for example `MyStruct { name: "a", int: 42 }` is serialized into json as `{"name":"a","tens":4,"digit":2}`.

Right now, the only way to provide a helper like the one above is to either:

* Implement a proxy that emits the `Serialize` impl block, or
* Provide helper methods and instruct the user how to manually implement `Serialize` using the helpers you provided.



<details>
<summary>Hypothetical example: Evolving the `Deref` trait</summary>
    
    This section is included because whether Deref is to be merged with Receiver is up for deliberation at the moment.

[deref-receiver]: #Deref-Receiver-evolution

As part of the `arbitrary_self_types` feature, we need to split `Deref` into two traits. Right now, the `Deref` trait looks like this:
```rs
pub trait Deref {
    type Target: ?Sized;

    fn deref(&self) -> &Self::Target;
}
```
But we need it to look like this:
```rs
pub trait Receiver {
    type Target: ?Sized;
}

pub trait Deref: Receiver {
    fn deref(&self) -> &Self::Target;
}
```
However, making this change is difficult due to backwards compatibility. There are many crates in the ecosystem with code that looks like this:
```rs
struct MyStruct(u8);

impl Deref for MyStruct {
    type Target = u8;
    fn deref(&self) -> &u8 {
        &self.0
    }
}
```
or like this:
```rs
fn assert_deref()
where
    MyStruct: Deref<Target = u8>
{}
```
The feature in this RFC provides a mechanism for _trait evolution_ of `Deref` and `Receiver` where it becomes possible to split a trait into a super-trait and sub-trait without breaking backwards compatibility in downstream crates.

```rust
pub trait Receiver {
    type Target: ?Sized;
}

pub trait Deref: Receiver {
    auto impl Receiver;

    fn deref(&self) -> &Self::Target;
}

// Note that the `impl Deref` block is still compiled and completely equivalent
// to the definition prior to applying this RFC.
impl Deref for MyStruct {
    type Target = u8;
    fn deref(&self) -> &u8 {
        &self.0
    }
}

fn assert_deref()
where
    MyStruct: Deref<Target = u8>
{
    // The `Deref<Target = u8>` predicate resolves the name `Target` to
    // `Receiver::Target` under this RFC
}
```

### Why not a blanket impl?

Unfortunately, this does not work:
```rs
pub trait Receiver {
    type Target: ?Sized;
}

pub trait Deref: Receiver {
    type Target: ?Sized;
    fn deref(&self) -> &Self::Target;
}

impl<T: Deref> Receiver for T {
    type Target = <T as Deref>::Target;
}
```
The problem is that crates need to be able to write this code:
```rs
struct SmartPtr(*mut T);

impl<T> Receiver for SmartPtr<T> {
    type Target = T;
}

impl<T: SafeToDeref> Deref for SmartPtr<T> {
    fn deref(&self) -> &T {
        unsafe { &*self.0 }
    }
}
```
This kind of code where `SmartPtr` *sometimes* implements `Deref` but *always* implements `Receiver` is not possible with a blanket implementation. The feature proposed by this RFC would make the above construction a possibility.
</details>
    
## Tenets

- Backward-compatibility
    - We need to maintain that all the current `impl` blocks to compile, specifically the `impl Deref` litmus test mentioned in the [deref-receiver] motivating example.
    - This demand also extends to other possible library traits that may see relocation of items into a future supertrait, while ensuring that the existing `impl` blocks continue to compile.
- Intuitional readability and clear syntatical signal
    - We strive for a syntax that is intuitional and easily connected to the existing constructs.
    - A successful design is one that enables a user to easily build a correct mental picture of the trait `impl`s with assistance from the syntatical features.
    - By extension, supertrait `impl`s should appear clearly in connection with subtrait `impl`s.
- Flexibility
    - We strive for providing users the means to refactor their traits without compromising the expressiveness of the trait relationships.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This section gives an overview of the `auto impl` feature and some example use-cases.

## Overview

It is possible to declare that implementations of a sub-trait should automatically implement a given supertrait.
```rs
trait MyTrait {
    fn my_func(&self);
}

trait MySubTrait: MyTrait {
    auto impl MyTrait;
    
    fn my_second_func(&self);
}
```
Given the above traits, when you implement `MySubTrait`, you must also specify items from `MyTrait`.
```rs
impl MySubTrait for String {
    fn my_func(&self) {
        println!("my_func on String");
    }
    
    fn my_second_func(&self) {
        println!("my_second_func on String");
    }
}
```
In the above case, it is an error to not specify all items from `MyTrait`. However, it is possible to opt-out of implementing `MyTrait` in the `impl MySubTrait` block:
```rs
impl MyTrait for String {
    fn my_func(&self) {
        println!("my_func on String");
    }
}
impl MySubTrait for String {
    extern impl MyTrait;
    fn my_second_func(&self) {
        println!("my_second_func on String");
    }
}
```
The `extern impl MyTrait` declaration specifies the impl block does not automatically implement `MyTrait`, and that another impl block is used instead.

### Example: Trait evolution

Over time, needs have arised to establish a hierarchy of traits, so that the parts and pieces of existing "big" library traits, be it from `std` or ecosystem crates, can be extraced and pulled back into supertraits without requiring a breaking change in the downstream crates. In most cases, the assoication of methods to be "refactored" and the destination supertraits can be determined without ambiguity. This falls under a bigger theme of trait evolution, which concerns how a historically big trait can be broken down and refined into smaller traits and trait hierarchy. [RFC 1210](https://rust-lang.github.io/rfcs/1210-impl-specialization.html#the-default-keyword) provided an example of a trait evolution and how specialisation could have eased the refactoring.

With this proposal, specialisation is not required and, instead, the pulled-back supertrait implementation applies directly within the context of the subtrait implementation.

```rust
// A refactored Subtrait that encodes parts of its protocol
// to be implementable on other types,
// to which Subtrait is not applicable.
trait Supertrait {
    // This method is pulled out of Subtrait
    fn supertrait_fn();
}

// Subtrait is the refinement of the Supertrait protocol
trait Subtrait: Supertrait {
    // Refactored into the Supertrait
    //*** fn supertrait_fn();
    fn subtrait_fn();
}

// The proposal called for enabling specialisation of
// the following blanket implementation as *stop-gap*.
// We propose that for trait evolution, we do not need to rely on specialisation...
/**
impl<T: Subtrait> Supertrait for T {
    default fn supertrait_fn() {
        <T as Subtrait>::supertrait_fn();
    }
}
**/

struct Middeware<T>(T);

// ... but rather keep the current and future `impl` block
// the same,
impl Subtrait for Middleware<T> {
    // because the trait has been well designed so that
    // it allows seamless migration.
    fn supertrait_fn() { .. }
    fn subtrait_fn() { .. }
}
```



## Default auto implementations

It is possible to provide default implementations of functions from the super trait.
```rs
trait MyTrait {
    fn my_func(&self);
}

trait MySubTrait: MyTrait {
    auto impl MyTrait {
        fn my_func(&self) {
            self.my_second_func();
            self.my_second_func();
        }
    }
    
    fn my_second_func(&self);
}
```
In this case, an impl block for `MySubTrait` will still automatically implement `MyTrait`, but you are not required to provide an implementation of `my_func` since a default implementation exists.

### Example: Helpers for implementing a trait

With default auto implementations, it becomes possible to provide a sub-trait whose purpose is to help you implement the super trait in a specific way.

For example, given this super trait:
```rs
trait EventHandler {
    type Event;
    fn handle_event(&mut self, event: Self::Event);
}
```
Then you might have a helper for implementing `EventHandler` for a specific event type:
```rs
enum MouseEvent {
    ClickEvent(ClickEvent),
    MoveEvent(MoveEvent),
}

trait MouseEventHandler: EventHandler {
    auto impl EventHandler {
        type Event = MouseEvent;
        fn handle_event(&mut self, event: MouseEvent) {
            use MouseEvent::*;
            match event {
                ClickEvent(evt) => self.click_event(evt),
                MoveEvent(evt) => self.move_event(evt),
            }
        }
    }
    
    fn click_event(&mut self, event: ClickEvent);
    fn move_event(&mut self, event: MoveEvent);
}
```
This allows crates to implement `EventHandler` by writing this code:
```rs
struct PrintHandler;

impl MouseEventHandler for PrintHandler {
    fn click_event(&mut self, event: ClickEvent) {
        println!("Click: {:?}", event);
    }
    fn move_event(&mut self, event: MoveEvent) {
        println!("Move: {:?}", event);
    }
}
```
The `MouseEventHandler` trait could even come from a different crate than `EventHandler`.

## Unsafe auto impl

It's possible to declare that an auto implementation is unsafe.
```rs
trait MySubTrait: MyTrait {
    unsafe auto impl MyTrait;
    
    fn my_second_func(&self);
}
```
This means that it is unsafe to override the auto implementation.
```rs
impl MySubTrait for String {
    // unsafe is required here because the `auto impl`
    // is marked unsafe.
    unsafe extern impl MyTrait;

    fn my_second_func(&self) {
        println!("my_second_func on String");
    }
}
```
If the super trait is unsafe, then the `auto impl` must also be unsafe.

### Example: Safely implement unsafe trait

This can be used to provide a safe way to implement an unsafe trait.
```rs
/// Implementers must ensure that even() returns an
/// even number.
unsafe trait Even {
    /// Guaranteed to return an even number.
    fn even(&self) -> usize;
}

trait Double: Even {
    // SAFETY: 2*x is always even
    unsafe auto impl Even {
        fn even(&self) -> usize {
            2 * self.value_to_double()
        }
    }
    fn value_to_double(&self) -> usize;
}

// provides an impl for Even safely
impl Double for String {
    fn value_to_double(&self) -> usize {
        self.len()
    }
}
```

---


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

In a trait declaration, you may declare one or more `auto impl` items with a block that provides implementations for one or more items from the super trait.
```rs
trait SubTrait: SuperTrait {
    auto impl SuperTrait {
        const MY_CONST: u32 = 10;
        type MyType = String;
        fn my_func(&self) -> u32 {
            42
        }
    }
}
```
All items inside the `auto impl` block must match an item from the super trait of the same name and signature. When the block is empty, it is legal to use a semicolon instead. That is, these are equivalent:

* `auto impl SuperTrait {}`
* `auto impl SuperTrait;`

The type of the super trait can be anything that matches the [TypePath](https://doc.rust-lang.org/reference/paths.html#grammar-TypePath) grammar and evaluates to a trait. This means that, for example, this is legal:
```rs
trait SubTrait<T> {
    auto impl MyGenericTrait<T, u32>;
}
```
The trait must be a super-trait. That is, the trait solver must be able to prove that `where T: SubTrait` implies `where T: SuperTrait`.

## Impl blocks

Impl blocks using auto implementations are simply a short-hand for multiple impl blocks with all of the consequences that implies.

When a trait has an `auto impl` entry, all impl blocks for the trait that do not use `extern impl` to opt-out of the auto implementation become equivalent to two impl blocks, one for the sub-trait and one for the super-trait. They are generated according to these rules:

* Both impl blocks have the exact same set of generic items and where clauses, except that in the super trait any generic parameters that are unused by the super trait's `auto impl` are omitted.
* The equivalent impl block for the super trait contains the items in the `impl` block that come from the super trait, plus any items specified in the block of the `auto impl` if any. In case of duplicates, the item from the `impl` block is preferred.
* The equivalent impl block for the sub-trait contains any remaining items in the original `impl` block, plus an `extern impl` declaration.

So for example, given these traits:

```rs
trait SuperTrait<T, U> {
    fn my_first_item(&self, arg: U);
    fn my_default_item(&self, arg: T);
}

trait SubTrait<T>: SuperTrait<T, u32> {
    auto impl SuperTrait<T, u32> {
        fn my_default_item(&self, arg: T) {}
    }
    fn my_second_item(&self, arg: T);
}
```
then this impl block:
```rs
impl<T, U> SubTrait<T> for MyStruct<U>
where
    T: MyTraitBound<U>,
{
    fn my_first_item(&self, arg: u32) {}
    fn my_second_item(&self, arg: T) {}
}
```
is short-hand for this:
```rs
// The original impl block MINUS the super-trait's methods
// PLUS an extern impl statement:
impl<T, U> SubTrait<T> for MyStruct<U>
where
    T: MyTraitBound<U>,
{
    extern impl SuperTrait<T, u32>;

    fn my_second_item(&self, arg: T) {}
}

// PLUS an additional impl block for the super trait,
// constructed in the following manner:
impl<T, U> SuperTrait<T, u32> for MyStruct<U>
//         ^^^^^^^^^^^^^^^^^^ taken verbatim from `auto impl` statement.
where
    T: MyTraitBound<U>,
{
    // With SuperTrait's methods from the impl block
    fn my_first_item(&self, arg: u32) {}
    
    // AND SuperTrait's methods from the auto impl block
    fn my_default_item(&self, arg: T) {}
}
```
Note that this implies that you *must* use `extern impl` to provide your own implementation of the super trait. If you don't, then by the above rule, the generated impl for the super trait would overlap with your custom implementation, which is illegal by the standard trait rules.

### Extension: item aliasing in `auto impl`
To reduce possible verbosity, we can propose a future extension to `auto impl` default implementation block, in case supertrait items can be implemented as an alias to subtrait items.

When a subtrait associated method has the same signature as a supertrait associated method in terms of generics, has a set of `where` bounds that satisfies the supertrait item `where` bounds and compatible function signature, the `auto impl` default implementation can be simplified into a assignment statement, instead of a complete function body with a delegation call.

```rust
trait Supertrait {
    fn method(&self) -> impl Trait1;
}

trait Subtrait: Supertrait {
    auto impl Supertrait {
        fn method(&self) -> impl Trait1 = <Self as Subtrait>::method;
    }

    fn method(&self) -> impl Trait1 + Send
    where Self: 'static + Send;
}
```

### Extension: `auto impl` support for higher-kinded superbound

Today a higher-kinded superbound is allowed as a superbound as long as only lifetime parameters are
used.

```rust
trait Supertrait<'a> {}
trait Subtrait: for<'a> Supertrait<'a> {}
```

We propose to extend `auto impl` support to superbounds like this. In order to achieve this,
the `auto impl` item in traits and `impl` blocks are equipped with exclusively lifetime generic
parameters.

```rust
trait Supertrait<'a> {}
trait Subtrait: for<'a> Supertrait<'a> {
    auto impl<'a> Supertrait<'a>;
}
```

The usual no-shadowing rule applies when it comes to lifetime parameters.

```rust
trait Subtrait<'a>: for<'a> Supertrait<'a> {
    //         ~~ first declared here
    auto impl<'a> Supertrait<'a> {
        //~^ ERROR lifetime name `'a` shadows a lifetime name that is already in scope
        //~|  lifetime `'a` already in scope
    }
}
```

## Unsafe auto implementations

The `auto impl` items can be marked `unsafe`, which declares that implementing the sub-trait without using the auto implementation is unsafe.

When an `auto impl` is declared unsafe, then:

* To opt-out, you must write `unsafe extern impl`.
* If any methods from the `unsafe auto impl` block are overridden, then the `impl` block must be `unsafe`.

If the super trait is `unsafe`, then the `auto impl` declaration must also be `unsafe`.

## Naming ambiguity

If the sub-trait defines an item of the same name as an item in the super-trait, then the `auto impl` block must provide an implementation of that item from the super trait.

In this scenario, any item in an impl block of the sub-trait using the ambiguous name will always be resolved to the item from the sub-trait. This means that the only way to override the item from the super trait is to use `extern impl` or an overriding `auto impl` block inside the sub-trait `impl` block.

If the sub-trait definition contains two `auto impl` directives and a sub-trait implementation has an item with a name that can be resolved to an associated item in both of the `auto impl` supertraits, irrespective of the associated item kind, then it **must** also be rejected as ambiguity. Either an `extern impl` statement or an overriding `auto impl` block is required for supplying an alternative definition of this item for each relevant supertrait.

## Nesting `auto impl` in sub-trait defintion

Nesting `auto impl` is allowed in a sub-trait definition or implementor.

In a sub-trait definition site, only `auto impl`s is ever allowed in any level of nesting. If a target supertrait has at least one associated item or `auto impl` directive, **either** the full list of associated items and full list of `auto impl`s with concrete implementation are supplied as a default implementation at one nesting level, **or** the `auto impl` implementation is elided and the nesting terminates at this supertrait.

```rust
trait Supersupertrait {
    type Type;
}
trait Supertrait: Supersupertrait {
    auto impl Supersupertrait;
}
trait Subtrait: Supertrait {
    // A full implementation as default is required at each nesting level, or ...
    auto impl Supertrait {
        auto impl Supersupertrait {
            type Type = ();
        }
    }
}
trait Subtrait2: Supertrait {
    // No default implementation is supplied and the nesting terminates at this supertrait
    auto impl Supertrait;
}
```

In a sub-trait implementor site, both `auto impl`s and `extern impl`s are allowed.

## Mandatory `extern impl` declaration

For the following definition, a **non-marker** trait is a trait with an item, which can be `default` or not. A marker trait has zero associated items.

| Supertrait kind | `auto impl` block in sub-`trait` block | `auto impl` statement in sub-`trait` block |
|-|-|-|
| non-marker | Mandatory[<sup>a</sup>](#e-i-a) | Optional[<sup>b</sup>](#e-i-b) |
| marker | Mandatory[<sup>c</sup>](#e-i-c) | Mandatory[<sup>c</sup>](#e-i-c) |

### <a id="e-i-a"></a>Case a: `auto impl` block of a non-marker supertrait in sub-`trait` block
For illustration, here is an example.
```rust
trait Supertrait {
    type Item;
}
trait Subtrait: Supertrait {
    auto impl Supertrait {
        type Item = u32;
    }
}

impl Supertrait for MyStruct {
    type Item = u8;
}
impl Subtrait for MyStruct {
    // Without the following ...
    extern impl Supertrait;
    // ... the code will be rejected for overlapping `impl Supertrait`s
}
```
The reason for this is that given that `trait Subtrait` has already provided its implementation, an implementation of `Subtrait` must choose between the default implementation and a user-defined implementation. We prefer explicit confirmation through `extern impl` declaration from the implementor, rather than making the compiler to reason about whether `auto impl` should be backfilled for ease of language feature implementation.

#### Extension: Possible relaxation through an attribute and a future-compatibility lint
For important ecosystem traits like `PartialOrd` and `Ord`, this rule is still unsatisfactory due to [the potential rewrites required](#po-o) on downstream crates, even though it could be as small as an additional `extern impl PartialOrd`. As an extension, the rule could be relaxed with an attribute `#[probe_extern_impl]` and apply further trait selection to decide whether the default implementation given by the `auto impl` block should be used.

```rust
trait Ord: PartialOrd {
    #[probe_extern_impl]
    auto impl PartialOrd {
        // ...
    }
}

// The following code in the ecosystem will continue to compile.
impl PartialOrd for MyType { .. }
impl Ord for MyType {
    // Given the current facts about `MyType`,
    // the compiler can deduce that `MyType: PartialOrd` is satisfiable,
    // so the `auto impl PartialOrd` is not used
}
```

However, this practice will not be encouraged eventually under provision of this RFC. For this reason, we also propose a future-compatibility lint, which will be escalated on a future Edition boundary to denial. The lint shall highlight the existing `auto impl` block in the subtrait definition and suggest an explicit `extern impl` statement in the subtrait implementation.

### <a id="e-i-b"></a>Case b: `auto impl` block of a non-marker supertrait in sub-`trait` statement
For illustration, here is an example.
```rust
trait Supertrait {
    type Item;
}
trait Subtrait: Supertrait {
    auto impl Supertrait;
}

impl Supertrait for MyStruct {
    type Item = u8;
}
impl Subtrait for MyStruct {
    // The following `extern impl` is optional
    extern impl Supertrait;
}
```
The reason for this is that `trait Subtrait` has not already provided its implementation, an implementation of `Subtrait` must supply an implementation of `Supertrait`, which could have existed before introducing the `auto impl Supertrait`.

If `auto impl` statement is declared on a non-marker supertrait without a default implementation, the `extern impl` is optional so that we do not penalise the existing trait implementors.

### <a id="e-i-c"></a>Case c: `auto impl` block of a marker supertrait
For illustration, here is an example.
```rust
trait Supertrait {}
trait Subtrait: Supertrait {
    auto impl Supertrait;
}

impl Subtrait for MyStruct {
    // The implementor must choose between
    // - `auto impl Super` and
    // - `extern impl Supertrait`.
    // Without either, it would be rejected with unsatisfied super-bound
    extern impl Supertrait;
}
```
The reason for this is that `Supertrait` as a marker trait has no associated items. As we could not decide if the `Supertrait` would be implemented within the bounds attached to the `impl Subtrait` block, due to lack of syntatical signals, it is better to require explicit confirmation from the implementor on the condition of the marker trait `Supertrait` when this marker is applicable to `MyStruct`.

## SemVer consideration

In this section we consider the impact on semantic versioning when a change to trait definition and implementors affects a `auto impl` syntax structure.

### Addition of `auto impl` in sub-trait definition

This is a SemVer hazard and can constitute a major change to public API, provided that the supertrait relation has not been changed. Implementers now have the obligation to ensure that their external implementation does not conflict with a potential default implementation at the sub-trait definition site.

### Removal of `auto impl` in sub-trait definition

This is a SemVer hazard and can constitute a major change. This requires the downstream implementors of the sub-trait to move the `auto impl` out of the `impl` block.

### Addition and removal of `unsafe` qualifier on the `auto impl` directives

This is a SemVer hazard and mandates a major change. The implementors should inspect their implementation against the trait safety specification and add or remove safety comments accordingly. It is possible that the semantics of the API would change as the safety obligation can propagate through the API across multiple crate boundaries.

### Switching between `extern impl Supertrait` and `auto impl Supertrait`

This is a SemVer hazard and mandates a minor change. Provided that both the sub-trait and the super-trait remains SemVer stable, this constitutes only a change in implementation detail.

### Change in the proper defintion of super- and sub-traits

This is a SemVer hazard and mandates a major change. The justification follows API change in trait irregardless of super- or sub-trait relationship. This scenario encompasses any changes in types, function signature, bounds, names.


---

# Drawbacks
[drawbacks]: #drawbacks

- This is yet another new language syntax to teach.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## A `auto impl` in a `trait` definition block should not be interpreted as a blanket `impl`

Our rationale is that this blanket `impl` would unnecessarily reject genuine user-`impl`s of supertraits where a relaxed `where` bounds are desirable through overlapping `impl`s. In our opinion, a supertrait `impl` on a type that has more relaxed bounds than that on a subtrait `impl` of the same type is perfectly valid.

As illustration, the following is an example

```rust
trait BaseFunction {
    fn base_capability(&self);
}
trait ManagementExtension: BaseFunction {
    auto impl BaseFunction;
    fn management_interface(&self);
}

struct ManagementInterface<T> { .. }

impl<T> BaseFunction for ManagementInterface<T> {
    // NOTE: we do not need `T: ManagementHandle`
    // for ManagementInterface<T> to provide
    // `base_capability`
    fn base_capability(&self) { .. }
}

// Suppose that `auto impl BaseFunction` is "lowered" into a blanket `impl`,
// then it is impossible for the `impl BaseFunction` to compile.

impl<T> ManagementExtension for MangementInterface<T>
where T: ManagementHandle
{
    extern impl BaseFunction;
    fn management_interface(&self) { .. }
}
```

## Why explicit opt-in/out for marker traits and traits with only default items?

Marker supertraits and supertraits with `default` items can be easily overlooked when users write subtrait implementations. They would register too little signal for the reader to recognise the significance of traits of these kinds. For this reason, we bias towards asking users to provide clear syntatical signals through `auto impl MarkerTrait` or `extern impl MarkerTrait`, so that the automatic derivation of such traits is easily recognisable and provides obvious site for documentation in case justification is waranted.

```rust
// It is almost always a good idea to explain
// why a trait like below is implemented on a type.
trait MarkerTrait {}

trait Supertrait: MarkerTrait {
    auto impl MarkerTrait;
}

impl Supertrait for MyType {
    // Here it is a good place to explain
    // why MyType: MarkerTrait
    auto impl MarkerTrait;
    // or otherwise `extern impl MarkerTrait;` is required
}
```

### What about checking whether another impl exists to decide automatic `impl`-filling?

We could potentially determine whether the opt-out is used based on whether an `impl` of the supertrait exists, but we prefer not to. We have existing mechanism to determine the specialisation and implementation overlaps. Whether a supertrait `impl` overlaps or not, is not the concern of this proposal.

If we would deduce whether an `auto impl` should be effected, there could present a hazard that silently changes the program behaviour.

```rs
trait MyTrait { default fn .. }

trait AutoMyTrait: MyTrait {
    auto impl MyTrait;
}

trait MyOtherTrait { .. }

struct Foo;

// Suppose we allow users to elide the `auto impl`/`extern impl` directive
// and we deduce based on some applicability criterion ...
impl AutoMyTrait for Foo {} // <-- generates MyTrait

// Suppose this item is added at one point of time
impl<T: MyOtherTrait> MyTrait for T { .. }

impl MyOtherTrait for Foo {}

// QUESTION: which `impl MyTrait for Foo` fulfills the bound `Foo: MyTrait`?
// Should it be the `auto impl MyTrait;` with default items?
// Should it be the `impl<T: MyOtherTrait> MyTrait for T` with `T := Foo`?
```

## Why is a hard error on ambiguity acceptable to us?

We hold the basic assumption that most associated items of a trait have sensible names. We would rather advise that one shall avoid name clashes and ambiguity through better, future-oriented trait designs.

In any case, `auto impl Trait { .. }` blocks still remains available for cases where ambiguity is unavoidable or favorable in niche scenario.

## Why this naming?

It is still up for discussion.

# Prior art
[prior-art]: #prior-art


## Implementable trait-alias

It was suggested in the [RFC 3437](https://github.com/rust-lang/rfcs/pull/3437) that trait aliases can be made to also carry associated items, which in turn can be instantiated in `impl` trait alias blocks.

## Plain supertrait items in subtrait `impl`

In fact, this proposal is an improved version over this scheme. Previously, to disambiguate names from different supertrait namespaces, one appends the associated item identifies with a qualification and generic arguments when necessary. However, the old proposal would apply more deduction, by the compiler, on whether supertrait `impl`s are demanded and it is a weaker response to the tenet that prefers more syntatical signals to compiler deduction.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None so far.

# Future possibilities
[future-possibilities]: #future-possibilities

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
