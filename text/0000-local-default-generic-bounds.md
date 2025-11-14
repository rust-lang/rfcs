- Feature Name: `local_default_bounds`
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

<!-- todo: Replace with RFC PR later -->
[`forget_marker_trait`]: https://github.com/rust-lang/rfcs/pull/3782

# Summary
[summary]: #summary

This RFC proposes a mechanism for crates to define default bounds on generics. By specifying these defaults at the crate level we can reduce the need for verbose and repetitive `?Trait` annotations while maintaining backward compatibility and enabling future language evolution.

# Motivation
[motivation]: #motivation

## What are `?Trait` bounds

Generic parameters on functions (`fn foo<T>()`), associated types in traits `trait Foo { type Assoc; }` and `Self` in traits `trait Foo where Self ...` can have `where` bounds.

This function expects any `T` that can be compared via `==` operator:

```rust
fn foo<T: PartialEq>(t: &T) {}
```

But Rust introduces some bounds by default. In the code above, `T` must be both `PartialEq` and `Sized`. To opt out of this, users need to write `+ ?Sized` manually:

```rust
fn foo<T: PartialEq + ?Sized>(t: &T) {}
```

## Use of `?Trait` bounds for new features
[applicability-of-default-bounds]: #applicability-of-default-bounds

A lot of new features (see [#use-cases](#use-cases)) require breaking old code by removing long-established assumptions like `size = stride` or the ability to skip the destructor of a type. To avoid breaking the code, they create a new trait representing an assumption and then define their feature as types that do not implement this trait. Here `?Trait` bounds come in - old code has old assumptions, but new code can add `?Trait` to opt out of them and support more types.

It is also important to note that in most cases those assumptions are not actually exercised by generic code, they are just already present in signatures - rarely code needs `size = stride`, or to skip the destructor (especially for a foreign type).

## The problem
[problem-of-default-bounds]: #problem-of-default-bounds

Quotes from "Size != Stride" [Pre-RFC thread](https://internals.rust-lang.org/t/pre-rfc-allow-array-stride-size/17933):

> In order to be backwards compatible, this change requires a new implicit trait bound, applied everywhere. However, that makes this change substantially less useful. If that became the way things worked forever, then `#[repr(compact)]` types would be very difficult to use, as almost no generic functions would accept them. Very few functions actually need `AlignSized`, but every generic function would get it implicitly.


@scottmcm

> Note that every time this has come up -- `?Move`, `?Pinned`, etc -- the answer has been **"we're not adding more of these"**.
> 
> What would an alternative look like that doesn't have the implicit trait bound?

In general, many abstractions can work with both `Trait` and `!Trait` types, and only a few actually require `Trait`. For example, `Forget` bound is necessary for only a few functions in std, such as `forget` and `Box::leak`, while `Option` can work with `!Forget` types too.
However, if Rust were to introduce `?Forget`, every generic parameter in `std` would need an explicit `?Forget` bound. This would create excessive verbosity and does not scale well.

There is a more fundamental problem noted by @bjorn3: `std` would still need to have `Forget` bounds on all associated items of traits to maintain backward compatibility, as some code may depend on them. This makes `!Forget` types significantly harder to use and reduces their practicality. Fortunately, @Nadrieril proposed a solution to that problem, which resulted in that RFC.

See [#guide-level-explanation](#guide-level-explanation) for details.

## Use cases
[use-cases]: #use-cases

- `!Forget` types - types with a guarantee that destructors will run at the end of their lifetime. Those types are crucial for async and other language features, which are described in [`forget_marker_trait`] Pre-RFC. <!--  Change to RCF and update link -->
- `Size != Stride` is a [frequently requested feature][freaquently-requested-features-size-neq-stride], but it is [fundamentally backward-incompatible change that requires `?AlignSized` bound][size-neq-stride-backward-incompatibe].
- [`Must move`] types will benefit from this too, further improving async ergonomics.

[freaquently-requested-features-size-neq-stride]: https://github.com/rust-lang/lang-team/blob/master/src/frequently-requested-changes.md#size--stride
[size-neq-stride-backward-incompatibe]: https://internals.rust-lang.org/t/pre-rfc-allow-array-stride-size/17933#the-alignsized-trait-and-stdarrayfrom_ref-8
[`Must move`]: https://smallcultfollowing.com/babysteps/blog/2023/03/16/must-move-types/#so-how-would-must-move-work

The expected outcome is an open road for new language features to enter the language in a backward-compatible way and allow users and libraries to adapt gradually.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This RFC is targeted at supporting migrations from default being `Trait` to `?Trait`, where `Trait` represents some assumption that is present everywhere but is not really exercised a lot, such as `Forget`, `size = stride` etc. Features like [`DynSized`], as well as [`extern types`], are out of the scope of this RFC, because it does not fit into this category. `DynSized` is not retracting mostly unexercised assumptions in order to make it `?DynSized` the default.

[`DynSized`]: https://github.com/rust-lang/rfcs/pull/2984
[`extern types`]: https://github.com/rust-lang/rfcs/pull/1861

The syntax is to be bikeshedded, initially, it might be with a crate-level attributes.

```rust
#![default_generic_bounds(?Forget, PartialEq)]
```

The following example demonstrates how the compiler will understand the code. `PartialEq` is used just for illustration purposes. In reality, only a special set of traits would be allowed and would grow with new "breaking" traits, like `Forget`. `PartialEq` would not be one of them.

```rust
#![default_generic_bounds(?Forget, PartialEq)]

use std::ops::Deref;

trait Trait: Deref + ?PartialEq {
    type Assoc: Forget;
}

struct Qux;
struct Foo<T>(T);
struct Bar<T: ?PartialEq>(T);
struct Baz<T: Trait>(T, T::Target, T::Assoc);

impl Trait for &i32 {
    type Assoc = &'static str;
}

fn use_qux(qux: Qux) { /* ... */ }
fn use_foo<T>(foo: Foo<T>) { /* ... */ }
fn use_bar<T: ?PartialEq>(bar: Bar<T>) { /* ... */ }
fn use_baz<T: Trait>(baz: Baz<T>) { /* ... */ }

fn main() {
    let foo = Foo(Qux); //~ error[E0277]: the trait bound `Qux: PartialEq` is not satisfied
    let bar = Bar(Qux); // compiles as expected
    let baz = Baz(&3, 3, "assoc"); // compiles as expected
}
```

Code above will be observable as (code in today's Rust without any defaults):

```rust
use std::ops::Deref;

trait Trait: Deref<Target: ?Forget> + ?PartialEq {
    type Assoc;
}

struct Qux;
struct Foo<T: ?Forget>(T);
struct Bar<T: ?Forget>(T);
struct Baz<'a, T: Trait>(T, &'a T::Target, T::Assoc);

impl Trait for &i32 {
    type Assoc = &'static str;
}

fn use_qux(qux: Qux) { /* ... */ }
fn use_foo<T: ?Forget + PartialEq>(foo: Foo<T>) { /* ... */ }
fn use_bar<T: ?Forget>(bar: Bar<T>) { /* ... */ }
fn use_baz<T>(baz: Baz<T>)
where
    T: ?Forget + PartialEq, // `Trait` has `?PartialEq` for `Self`, but there is no `T: ?PartialEq`
    T: Trait<Target: ?Forget, Assoc: Forget>
{ 
    /* ... */
}

fn main() {
    let foo = Foo(Qux);
    let bar = Bar(Qux);
    let baz = Baz(&3, 3, "assoc");
}
```

Introducing this feature is backward compatible and does not require an edition.

RFC tries to be consistent with already existing handling of `Sized`.

## Example: Migrating to `Forget`

With this RFC, transitioning to `Forget` is straightforward for any `#![forbid(unsafe)]` crate:

1. Set the appropriate bounds:

```rust
#![default_generic_bounds(?Forget)]
```

2. Resolve any compilation errors by explicitly adding `+ Forget` where needed.

3. Optionally: Recurse into your dependencies, applying the same changes as needed.

Crates using `unsafe` code should beware of `ptr::write` and other unsafe ways of skipping destructors.

## Implications on the libraries

### Relax generic bound on public API

For migrated users it is equivalent to semver's `minor` change, while not migrated uses will observe it as `patch` change.

### Weakening associated type bound and `Self` bound in traits

Traits and associated types will ignore default_generic_bounds and always default to the most permissive possible bounds:

```rust
trait Foo: ?Trait {
    type Assoc: ?Trait;
}
```

This change would not be observable for not migrated crates, because `default_generic_bounds` would default to `Trait`, which takes effect on function generic arguments. But if users start migrate before libraries, they will not lock them into old bounds.

```rust
#![default_generic_bounds(?Forget)]

async fn foo<T: other_crate::Trait>(bar: T) {
    let fut = bar.baz();
    // Compiler will emit an error that `fut` maybe `!Forget` because we set `default_generic_bounds`
    // to `?Forget` and `default_assoc_bounds` in `other_crate` is already `?Forget`. Otherwise it
    // would have been a breaking change for `other_crate` to make future returned by `baz` `!Forget`,
    // as this code would've compiled now but not in the future.
    core::mem::forget(fut);
}

// Libary that has not migrated yet.
mod other_crate {
    trait Trait {
        async fn baz();
    }
}
```

### Macros

If macro-library generates code, some problems during the migration are possible:

```rust
mod user {
    #![default_generic_bounds(?Forget)]

    ::library::make!(); // Will not compile because `T` is `?Forget`.
}

mod macro_library {
    #[macro_export]
    macro_rules! make {
        () => {
            pub fn foo<T>(t: T) {
                ::core::mem::forget(t);
            }
        }
    }
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Introduce new crate level attibute: `default_generic_bounds` used to (non-exhaustively) enumerate overwrides of defaults for different types of bounds. Only a special set of traits would be allowed and would grow with new "breaking" traits, like `Forget`.

Every trait would initally have its unique default. In practice, bounds for all traits that are stable at the date of RFC except `Sized` would default to `?Trait`. For new "breaking" traits, default would be `Trait`, except bounds for `Self` in traits and associated types in traits.

[^trait-not-sized-by-default]: https://rust-lang.github.io/rfcs/0546-Self-not-sized-by-default.html

`default_generic_bounds` is applied for generic parameters. Effectively, it would be observable like that:

```rust
// crate `b` that has not migrated to `#![default_generic_bounds(?Forget)]`
mod b {
    fn foo<T>() {} // Observed as `T: Forget` by `b` and other crates that have not migrated.
    struct Bar<T>(T); // Observed as `T: Forget`
    // `Self` and `Qux` will be ovservable or other crates, that migrated, without `Forget` bounds
    trait Baz<T> { // Observed as `T: Forget`
        type Qux<U>; // `U` is observed as `U: Forget` by `b` and other crates that have not migrated. 
    }
    trait Foo {}

    fn foo_static() -> impl Foo; // Observed as `impl Foo + Forget`
    fn foo_dyn() -> Box<dyn Foo>; // Observed as `Box<dyn Foo + Forget>`

    // Observed as `T: Forget`, `U: Forget`, `for<V: Forget> Baz<V>: Forget`.
    fn baz<T: Baz<U>, U>() {}

    trait Async {
        async fn method();
    }
    // Applies to RPITIT too where, so observed as `T::method(..): Forget`
    fn async_observer<T: Async>() {}

    trait DerefTrait: Deref { }

    // Associated types in generics are masked with `Forget` too.
    // So `<T as Deref<Target>>` observed as `Deref<Target: Forget>`
    fn deref_observer<T: DerefTrait>() {}

    trait RecursiveTrait {
        type Assoc: RecursiveTrait;
    }

    // All `<T as Trait>::Assoc`, `<<T as Trait>::Assoc as Trait>::Assoc`,
    // `<<<T as Trait>::Assoc as Trait>::Assoc as Trait>::Assoc` etc would be
    // observable as `: Forget`.
    // `T` is observed as `T: RecursiveTrait + Forget` too.
    fn recursive_observer<T: RecursiveTrait>() { }
}
```

# Drawbacks
[drawbacks]: #drawbacks

- It may increase compilation time due to the additional complexity of trait solving.
- It may make reading source files of crates harder, as the reader should first look at the top of the crate to see the defaults, and then remember them. It may increase cognitive load.
- It may take some time for the ecosystem around the language to fully adapt `!Trait`, but it will not include semver breaking changes for libraries or Rust code in general. Similar to `const fn` now.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This design is simple yet powerful because it offers a backward-compatible way to evolve the language.

The impact of not accepting this RFC is that language features requiring types like `!Forget`, `MustMove`,
[`!AlignSized`] and many others will not be accepted.

[`!AlignSized`]: https://internals.rust-lang.org/t/pre-rfc-allow-array-stride-size/17933

## [Default Auto Traits]

This is a very similar proposal which is partially implemted already, could totally be an alternative path. It makes same trick over an edition for traits that we want to remove from defaults. In the case of `Forget`, we may set default bound for crates of edition 2024 and earlier, and lift it for editions after 2024. In terms of this RFC, it would mean that editions would have different presets of default bounds, while users would not be able to manipulate them manually.

Pros of this is that we do not need a new syntax and implementation should be simpler.

Cons are that migration is more invasive and enormous, and feels more "forced" - to migrate to the new edition, you must migate to the new bound (or several bounds). The other thing is that [Default Auto Traits] makes no mention of what would happen (but it probably can be added) if library did not migrate to the next edition but users did - would library be locked into `Trait` bounds in associated types (like `async` functions) and need a breaking semver change to remove it? `local_default_bounds` address that issue directly and allows for non-breaking changes.

[Default auto traits]: https://github.com/rust-lang/rust/pull/120706

## Add fine-grained attributes
[split]: #split

We may have four attributes: `default_generic_bounds`, `default_foreign_assoc_bounds`, `default_trait_bounds` and `default_assoc_bounds` for more fine-grained control over defaults. For example, `Sized` has following defaults:

```rust
#![default_generic_bounds(Sized)]
#![default_trait_bounds(?Sized)]
#![default_assoc_bounds(Sized)]
#![default_foreign_assoc_bounds(?Sized)]
```

Previous version of this RFC was exactly this, you can read it [here](https://github.com/Ddystopia/rfcs/blob/49f52526b9f455ddbc333a7b453f8d61f1918534/text/0000-local-default-generic-bounds.md).

## Alternative syntax
[alternative-syntax]: #alternative-syntax

We may have a single macro to declare all bounds:

```rust
declare_default_bounds! { Sized, ?Forget, PartialEq };
```

## Do not default `Self` in traits and associated types to `?Trait` from the beginning

This will drastically reduce implementation complexity as it would be possible to do with a simple desugaring, because recursive bounds would not need to be infinitely bounded. But it will open a possibility for libraries to be locked into `Forget` bounds in some cases.

# Prior art
[prior-art]: #prior-art

## Links

- Default auto traits: https://github.com/rust-lang/rust/pull/120706
- `Self` not `Sized` by default: https://rust-lang.github.io/rfcs/0546-Self-not-sized-by-default.html

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- [ ] How to handle GATs? Rustc currently does not support proving `for<U> <T as Trait>::Assoc<U>: Forget`.
- [ ] How to solve recursive associated type bounds? `trait Trait { type Assoc: Trait }`
- [ ] We probably don't want to alter macro output, it would probably be too hard to implement and design. How should we handle this?

```rust
// macro crate
#![default_generic_bounds(PartialEq)]
#[macro_export]
macro_rules! make_functions {
    (
        { $a:item }
        { $($b:tt)* }
        { $($d:tt)* }
    ) => {
        $a
        $($b)*
        pub fn c<C>(c: &C) -> bool { c == c }
        pub fn d<D: $($d:tt)*>(_: &D) -> bool { true }
    }
}

// user crate
#![default_generic_bounds(?Forget)]
make_functions! { 
    { pub fn a<A>(_: &A) -> bool { true } }
    { pub fn b<B>(_: &B) -> bool { true } }
    {}
}
```

- [ ] Syntax
- [ ] How to display it in Rustdoc
- [ ] Should we allow default `!` bounds? What would it mean?
- [ ] Maybe use the term "implicit" instead of "default".
- [ ] Should we allow `Sized`.
- [ ] Maybe have 4 different attributes for more fine-grained control?
- [ ] Maybe go with [Default auto traits].

# Shiny future we are working towards

Less backward compatibility burden and more freedom to fix old mistakes, to propose new features.
