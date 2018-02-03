- Feature Name: default_type_parameter_fallback_revisited
- Start Date: 2018-02-03
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Rust already allows us to set defaults for type parameters in type definitions, as in:

```rust
struct Bar<T=String>(T);
// No need to write the value of T, it's set as `String`.
fn foo(_: Bar) {
  // Also here the type of `x` is fully known as `Bar<String>`.
  let x: Bar;
}
```

For more examples see [this playground](https://play.rust-lang.org/?gist=e7dd41eecf9c753a1c20ec9f2d4c818f&version=stable). This is a purely syntatical elision rule, there is no type inference involved. This RFC seeks to extend this functionality for fns, methods and impls and to integrate type inference with defaults, allowing uninferred type parameters to fallback to their defaults.

There are many motivations for this feature, the major ones are:

- Extending a type without breaking existing clients (stability without stagnation).
- Allow customization in ways that most users do not care about (abstraction without cost).
- Allow generalizing impls without breaking inference.

 These were the goals of accepted [RFC 213](https://github.com/rust-lang/rfcs/blob/master/text/0213-defaulted-type-params.md). This RFC intends to bring revisit this discussion, with an extended motivation and a better specified API evolution story, so that we may get the feature back on track. 

The key concrete proposals are:

- The concerns about conflicts among defaults in inference fallback are addressed by documenting what is and isn't a breaking change for inference.
- A syntax for eliding the default that is both convenient and guides to a choice of default that is unlikely to cause inference conflicts.
- Stabilize writing defaults in fns and impls first and later stabilize inference fallback, so that existing stable libraries have a period to adapt without being concerned of creating conflicts among defaults.

# Motivation

Over the years the [tracking issue](https://github.com/rust-lang/rust/issues/27336) has been collecting multiple real-world use cases for this feature.

## Customzing behaviour through type parameters

The behavior of an fn, method or impl may be customized by a standalone type parameter. For example, we could allow `slice::sort` to take a type parameter that customizes it's sorting algorithm:

```rust
pub fn sort<A: Algorithm>(&mut self) {...}
```

Here parameter `A` does not come from any value so there is no way it can be inferred. This change would be not only a breaking change but also unacceptably bad ergonomics to have to write `my_slice.sort::<MergeSort>()`, especially considering most users do not care what sorting algorithm is used. With inference-aware parameter defaults we may write the following:

```rust
pub fn sort<A: Algorithm = MergeSort>(&mut self) {...}
```

Now the algorithm will always be inferred to `MergeSort` when necessary, and advanced users may customize the sorting algorithm to their preference. We may have a single, customizable sorting method without additional performance or ergonomic cost. A zero-cost abstraction indeed.

## Customizing behaviour through optional arguments

The story for optional arguments in Rust is not the best, the most popular options are using `Option<T>` arguments or the builder pattern. However the interaction with generics is unfourtunate, and the lack of generic builders in the ecosystem is a symptom of this. The root of the problem is shown in the example:

```rust
use std::path::Path;

fn func<P: AsRef<Path>>(p: Option<P>) {
    match p {
        None => { println!("None"); }
        Some(path) => { println!("{:?}", path.as_ref()); }
    }
}
```

If we call `func(None)` then the type of `P` cannot be inferred. This is frustrating as in this case neither the caller nor the callee care about the type of `P`, any type would do. Default parameters allow the callee to choose a suitable default, making optional generic arguments ergonomically viable.

We may guess this is the most often occurring use case for default arguments. [It](https://github.com/PistonDevelopers/conrod/pull/626) [comes](https://github.com/gtk-rs/gir/issues/143) [up](https://github.com/jwilm/json-request/issues/1) [a lot](https://github.com/rust-lang/rust/issues/24857). We need type parameters defaults for optional arguments to be a well supported pattern, and even more so of we wish to dream of having optional arguments as a first-class language feature.

## Backwards-compatibily extending existing types

It's perfectly backwards-compatible for a type to grow new private fields with time. However if that field is generic over a new type parameter, trouble arises. The big use case in std is extending collections to be parametric over a custom allocator. Again something that must be backwards compatible and that most users don't care about. The existing feature was successful in making `HashMap` parametric over the hasher, so it has merits but it could be improved. To understand this let's try a simplified attempt at making `Arc` parametric over an allocator ([a real attempt](https://github.com/rust-lang/rust/pull/45272)).

Consider the following definition of `ArcInner` (the payload of an `Arc`) and two ways of constructing it:

```rust
struct ArcInner<T> {
  ref_count: usize,
  data: T
}

impl ArcInner<T> {
  pub fn new(data: T) -> ArcInner<T> {
      ArcInner { ref_count: 1, data }
  }
  
  pub unsafe fn from_raw(ptr: *const T) -> ArcInner<T> {
      // alignment witchcraft ommitted
      ptr as *mut ArcInner<T, A>
  }
}
```

The first step is to add the allocator type parameter, with the appropiate default, and an `alloc` field to the type definition:

```rust
struct ArcInner<T, A: Alloc = Heap> {
  alloc: A,
  ref_count: usize,
  data: T
}
```

Nice, now anywhere we have `ArcInner<T>` that will mean `ArcInner<T, Heap>`. But how do we update the constructors? The whole point is to be able to construct `Arc`s with custom values for `alloc`. With the current features the only choice is to duplicate the constructors, as was done for `HashMap`. The `impl ArcInner<T>` block is kept, meaning `impl ArcInner<T, Heap>`, and we add a new impl block:

```rust
impl ArcInner<T> {
  pub fn new(data: T) -> ArcInner<T> {
      ArcInner { ref_count: 1, data }
  }
  
  pub unsafe fn from_raw(ptr: *const T) -> ArcInner<T> {
      // alignment witchcraft ommitted
      ptr as *mut ArcInner<T, A>
  }
}

impl ArcInner<T, A> {
  pub fn with_alloc(data: T, alloc: A) -> ArcInner<T, A> {
      ArcInner { alloc, ref_count: 1, data }
  }
  
  pub unsafe fn from_raw_with_alloc(ptr: *const T) -> ArcInner<T, A> {
      ptr as *mut ArcInner<T, A>
  }
}
```

This is reasonable for the `with_alloc` constructor as there is no way to backwards-compatibly add a new argument. But with this proposal we may avoid duplicating `from_raw`:

```rust
impl ArcInner<T> {
  pub fn new(data: T) -> ArcInner<T> {
      ArcInner { ref_count: 1, data }
  }
}
// The elided default `Heap` for `A` is taken from the type definition.
// If inference fails, `A` will fallback to `Heap`.
impl ArcInner<T, A = _> {
  pub fn with_alloc(data: T, alloc: A) -> ArcInner<T, A> {
      ArcInner { alloc, ref_count: 1, data }
  }
  
  pub unsafe fn from_raw(ptr: *const T) -> ArcInner<T, A> {
      ptr as *mut ArcInner<T, A>
  }
}
```

## Allow generalizing impls without breaking inference

The big use case for defaults in impls is solving [this issue](https://github.com/rust-lang/rust/issues/20063). We currently have:

```rust
impl<T: PartialEq> PartialEq for Option<T> { ... }
```

But we would like to have:

```rust
impl<U, T: PartialEq<U>> PartialEq<Option<U>> for Option<T> { ... }
```

This would currently result in inference failures when trying to do `assert_ne!(Some("hello"), None)`. To fix this we need to set `U` as the default of `T`, as in `impl<U, T: PartialEq<U> = U>`. These are called dependent defaults, because the default depends on another type parameter. We would also like `T` to be the default of `U`, as in `impl<U = T, T: PartialEq<U> = U>` which requires forward declaration of defaults, [though there is a hack around this](https://github.com/rust-lang/rust/issues/20063#issuecomment-130248165).

## Other motivations

- Helping type inference with too many candidate types. The famous case here is `Iterator::collect`. It is a common cause of turbofishes and type annotations because so many types implement `FromIterator`. But most of those types are niche and in the common case people just want a `Vec`. It would be nice if we could default `collect` to return a `Vec<Iterator::Item>`. Unfortunately we can't because `Iterator` is defined in `core` and `Vec` is defined in `std`. Perhaps there are similar use cases in the ecosystem.

- Making an already generic parameter more generic, for example the case of [generalizing `slice::contains` over `PartialEq`](https://github.com/rust-lang/rust/pull/46934).

- The [RFC for making enum variants types](https://github.com/rust-lang/rfcs/pull/1450) depended on this feature.

# Guide-level explanation

When writing Rust code, you may find that you'd like to make a functionality more generic. But that does not always play well with inference, leading to an error like "type annotations needed" or "the type of this value must be known in this context". Say you have the following function that prints the path to a file, if it was provided:

```rust
use std::path::Path;

fn func(p: Option<Path>) {
    match p {
        None => { println!("No path provided"); }
        Some(path) => { println!("{:?}", path); }
    }
}
```

Calling `func(None)` will print `No path provided` as expected. But `func(Some("/my/file"))` does not work because a `&str` is not a `Path`. It's convenient to allow users to pass some sort of string as the path, so let's generalize our function:

```rust
use std::path::Path;

fn func<P: AsRef<Path>>(p: Option<P>) {
    match p {
        None => { println!("No path provided"); }
        Some(path) => { println!("{:?}", path.as_ref()); }
    }
}
```

Nice, now `func(Some("/my/file"))` works fine. But we broke `func(None)`, It fails with:

```rust
error[E0282]: type annotations needed
        func(None);
        ^^^^ cannot infer type for `P`
```

There is indeed no information about the type `P`. In fact we do not care about the type of `P`, anything that makes the program compile will do. To help out in cases like this you can use a _type parameter default_, like this:

```rust
use std::path::Path;

fn func<P: AsRef<Path> = String>(p: Option<P>) {
    match p {
        None => { println!("No path provided."); }
        Some(path) => { println!("{:?}", path.as_ref()); }
    }
}
```

Which tells inference to use `String` as backup choice if dosen't have enough information to work with. And indeed now `func(None)` works with `P` falling back to `String`.

## Using defaults for API evolution

A big use case for type parameter defaults is to help evolve a library while maintaing backwards compatibility. However there are cases where defaults may break inference.

The bad news is that adding a default to an existing type parameter or changing a default may break inference for your users because that may create conflicts among defaults. The good news is that you can add a new type parameter along with a default in the declaration of a type or trait without breaking inference, and the compiler will guide you on how to update your fns, methods and impls through lints and a simple syntax called _default elision_. 

For an example, let's say an UI library has the following type for text:

```rust
struct Text {
  string: String,
  pos: Position
}

impl Text { /* ... */ }
impl UiElement for Text { /* ... */ }
fn flip_text(t: Text) { /* ... */ }
```

Now the library wishes to add a locale type parameter `L` to the `Label` type, however the library is already 1.0 and stable. First you would add the parameter with an appropriate default:

```rust
struct Text<L = DefaultLocale> {
  string: String,
  pos: Position,
  locale: L,
}
```

Now as you change your API to be generic over the new type parameter, the compile will emit lints such as:

```rust
impl Text<L> { /* ... */ }
//   ^^^^^^^
//  warning: Parameter `L` that may have elided default has no default.
//  help: Try setting an elided default `L = _` to use `DefaultLocale` as the default.
```

This is what your library will look like after adding the new parameter and following the lints:

```rust
struct Text<L = DefaultLocale> {
  string: String,
  pos: Position,
  locale: L,
}

impl Text<L = _> { /* ... */ }
impl UiElement for Text<L = _> { /* ... */ }
fn flip_text(t: Text<L = _>) { /* ... */ }
```

The lints will guide you to add `_` where possible, the underscore represents an elided default that is taken from the type declaration, in the example all occurences of `_` will be replaced with `DefaultLocale`. As long as you add the elided defaults in the same release that you extend your type, you're guaranteed to not break inference for your clients. In complex cases it might not be possible to use `_` as a default, in those cases the change might cause inference failures.

# Reference-level explanation

Defaults may be set for type parameters in in traits, impls, struct and enum definitions and also methods and fns. They may not be set in `type` aliases. They also may not be set in methods and associated fns of trait impls, such defaults can only be set in the trait declaration. As per RFC 213, parameters with defaults must be trailing and may not be forward declared.

The behaviour of omited parameters in partially supplied parameter lists is as per RFC 213, they are inferred as if filled in with `_`. This is relevant to this [postponed RFC](https://github.com/rust-lang/rfcs/pull/1196) that suggests extending that behaviour to non-defaulted parameters.

## Defaults as fallbacks for inference

A key part of this proposal is that inference is now aware of defaults. When we would otherwise error due to an uninferred type we instead try using the default. This is called inference fallback which is our final attempt at inference.

### Conflicts among defaults

The possibility of conflicts among defaults is the origin of the concerns that are currently blocking the progress on this feature. Consider the example:

```rust
fn foo<T=String>(x: Option<T>);
fn bar<U>(y: Option<U>); // What if we had `fn bar<U=usize>`?

fn main() {
  let none: Option<_> = None;
  foo(none);
  bar(none);
}
```

Here, it seems clear that we should infer `_` as `String`. However, if `bar` also had a default different from `String` then we have a conflict among defaults which we don't know how to resolve. The consequence for API evolution is that adding a default to an existing type parameter may break inference.

We may still achieve the motivation of backwards compatibly extending types with defaulted type parameters if we can prevent conflicts involving those parameters. We do this by only allowing elided defaults where we know they do not cause conflicts with other existing parameters and indicating with a lint where an elided default can be used, therefore all the library author has to do when extending a type is to follow the lints and they can be assured they generalized everything that could be generalized without potentially breaking inference.

### API evolution guarantees

Breaking inference is generally considered to be a lesser kind of breaking change and even std itself considers small impact inference breakage to be ok. Conflicts among defaults is a very edge case way of causing an inference failure. Still, we must document the guarantees that are made so that libraries may be informed to make decisions.

**Adding a new type parameter with a default is, by itself, backwards compatible**. However you should be mindful of the use of that type parameter may cause inference breakage such as using it in the type in an fn signature or the type of a public field. For example going from `fn foo(x: i32) {}` to  `fn<T = i32> foo(x: T) {}` may cause inference breakage. Using it in a private field of a struct is backwards compatible.

**Upgrading APIs in the same release is backwards compatible**. If you add a defaulted type parameter to a trait or type in a way that is backwards compatible, it is also backwards compatible to generalize your APIs using an elided default as long as you do it _in the same release_. See "Rationale and alternatives" for an example of how it may break inference if you do it in a separate release.

The following things may cause inference breakage:

- Changing a default may break inference mostly beause the new defaults might not fullfill bounds that the previous one did and it might cause conflicts among defaults.
- Adding a default to an existing type parameter may break inference because it might cause conflicts among defaults, though that should be rare in practice. If an elided default is used the risk should be even smaller.
- Removing a default may of course break inference.

## Default elision

Default elision is the syntax  `T = _` which indicates that the default is being taken from the type or trait definitions in which `T` is used. When default elision may be used for a parameter `T` but no default is set a lint is emitted to suggest writing `T =_`.

### Motivation for default elision

Consider that we managed to successfully extend `Arc<T>` with a defaulted allocator parameter and now we have `Arc<T, A = Heap>`. But all the APIs in the ecosystem are still using `Arc<T>` which equals `Arc<T, Heap>`, default elision can be thought of as a tool to help upgrade APIs as the example shows.

Given the pair of fn definitions:

```rust
fn make_my_arc<T>(t: T) -> Arc<T> {}
fn use_my_arc<T>(arc: Arc<T>) {}
```

We want to upgrade them backwards compatibly. The first thing we might attempt is:

```rust
fn make_my_arc<T, A>(t: T) -> Arc<T, A> {}
fn use_my_arc<T, A>(arc: Arc<T, A>) {}
```

But that would break `use_my_arc(make_my_arc(0))` . Maybe what we mean is:

```rust
fn make_my_arc<T, A = alloc::Heap>(t: T) -> Arc<T, A> {}
fn use_my_arc<T, A = alloc::Heap>(arc: Arc<T, A>) {}
```

Which is not pretty. Do we really have a choice for the default here? If we tried:

```rust
fn make_my_arc<T, A = MyAllocator>(t: T) -> Arc<T, A> {}
fn use_my_arc<T, A = MyAllocator>(arc: Arc<T, A>) {}
```

Then `use_my_arc(make_my_arc(0))` works but now we broke `use_my_arc(Arc::from_raw(ptr))`. So the only reasonable choice is to use the default in the type definition. Therefore we use an elided default in this situation, using the the default in the type definition as the default for `A`. 

```rust
// The default of `A` in these declarations is `alloc::Heap`
fn make_my_arc<T, A = _>(t: T) -> Arc<T, A> {}
fn use_my_arc<T, A = _>(arc: Arc<T, A>) {}
```

It can be difficult to reason about whether a type parameter can use an elided default. To help usability we lint when a parameter that may have an elided default does not have a default. In rare cases this lint may be a false positive. But this doesn't seem bad as `#[allow(default_not_elided)]` will serve as an indication that a default is purposefully not set.

```rust
 fn foo<T, A>(t: T) -> Arc<T, A> {}
//		  ^^
//	warning: Parameter that may have elided default has no default.
//  help: Try setting an elided default `A = _` to use `alloc::Heap` as the default.
//  note: Lint `default_not_elided` on by default.
```

The motivations for default elision can be summarized as:

- To improve the API evolution story.
- To avoid repetitively writing the same default everywhere.
- To try to prevent conflicts among defaults at declaration sites.

### How to determine if a parameter may have an elided default

Default elision is only allowed if there is an unambiguous choice of default, here we specify how we determine that default if it exists. Given the declaration of a type parameter `T`, if `T` substitutes at least one type parameter that has a default in the type or trait definition and and all such type parameters substituted by `T` have the same default then the default of `T` may be elided. This rule applies everywhere where you can write a default.

Examples of substitution sites where we should look for defaults: Input and output types in an fn or method. In traits and impls, the trait or impl header and also child items such as methods. In type definitions, fields in which `T` appears. Predicates in which `T` appears are also included in the check.

This check should run somewhere between name resolution and typechecking, it should not use inference.

### Code examples

Default elision in an fn:

```rust
struct Foo<U=String>(U);
struct Bar<V>(V);
struct Qux<W=String>(W);
// `_` is `String`.
fn func<T = _>(foo: Foo<T>, bar: Bar<T>) { /* ... */ }
```

Default elision in an impl:

```rust
trait TraitDefault<T=String> { }
struct NoDefault<T>(T);
// `_` is `String`.
impl<T = _> TraitDefault<T> for NoDefault<T> { }
```

Situations where default elision is not allowed:

```rust
struct Foo<T=usize>(T);
struct Bar<U=String>(U);
struct Qux<T>(T);
// `T` cannot have `_` as it's default because `Foo<T>` and `Bar<T>` have different defaults for `T`.
impl<T> Qux<T> {
  fn foo_and_bar(f: Foo<T>, b: Bar<T>);
}
```

```rust
struct Baz<T=usize>(T);
struct Qux<U=String>(U);
// `T` cannot have an elided default.
fn baz_qux_it<T>(baz: Baz<T>, qux: Qux<T>) {}
```

```rust
trait MyTrait<T=String> {}
struct Foo<T=usize>(T);
// `T` cannot have an elided default.
impl<T> MyTrait<T> for Foo<T> {}
```

The behaviour of fallback for nested types is worth noting. As the example explains:

```rust
struct Foo<T = Vec<i32>>(T);
// `_` is `Vec<i32>`.
fn func<U = _>(foo: Foo<U>) {}
// Default elision is not allowed since `Vec<U>` has no for `U`.
// `Vec<U>` happening to match `Vec<i32>` doesn't matter..
fn func<U>(foo: Foo<Vec<U>>) {}
```

## Rollout plan

The order in which things would hit stable is:

1. Allow writing a default in impls, methods and fns, but they don't yet have any effect.
2. Defaults in impls and fns inform inference and may be elided in parameter lists.

Between steps 1 and 2 is an adaptation period, during which libraries may freely set defaults for impls, methods and fns without any chance of breaking inference. After step 3 (the full rollout), setting a non-elided default to an existing type parameter may possibly break inference.

# Drawbacks

- It's another feature, and not a simple one. Though part the feature and syntax is already stable, the interaction of inference adds a lot in terms of complexity.
- The lints affect code that compiles fine today. In some cases, the lints may be false positives.
- The API evolution story is complicated. We try to help with lints to guide the upgrade of APIs, but library authors may find the whole thing too complicated to be used.

# Rationale and alternatives

## Default elision

- We could completely phase out not setting a default where it may be elided. But there are cases where you don't want the elided default:

  ```rust
  struct Foo<A = MyAllocator>(Option<A>);

  // An elided default here would cause a conflict in
  // `arc_and_vec(make_arc(), Foo(None))`
  fn make_arc<T, A>() -> Arc<T, A> { /* ... */ }
  fn arc_and_vec<T, A = MyAllocator>(a: Arc<T, A>, v: Foo<A>) {}
  ```

- How overriding defaults that can be elided can be unfourtunate:

  ```rust
  struct Foo<T = String>(Option<T>);
  // `new_foo` and `take_foo` come from different crates.
  // Say `take_foo` was updated first, overriding the elided default.
  // Now `new_foo` can't update because that breaks `take_foo(new_foo())`.
  fn new_foo() -> Foo { Foo(None) }
  fn take_foo<T = usize>(x: Foo<T>) {}

  ```

- We might wish that clients could also upgrade with `_` without risking breaking inference. But here is an example where client upgrading their API can cause an inference break even if `_` is used:

```rust
struct Foo<T = String>(Option<T>);
struct Bar<U = i32>(Option<U>);

fn new_foo<T = _>() -> Foo<T> { Foo(None) }
fn foo_to_bar<V>(f: Foo<V>) -> Bar<V> { Bar(f.0) }

// Client upgrades it's `take_bar` function.
// Before:
fn take_bar(b: Bar) {}
// After:
fn take_bar<U = _>(b: Bar<U>) {}

fn main() {
   // Before this was `i32`, now inference fails.
   take_bar(foo_to_bar(new_foo()));
}
```

- Future extension: To be an interacting default for `T `, the default type must fullfill all bounds on `T`:

  ```rust
  struct Foo<U=usize>(U);
  struct Qux<W=String>(W);
  // `T` in `Qux` is no longer considered an interacting default because the default `String` does not fullfill the bound `Copy`.
  fn func<T:Copy = _>(foo: Foo<T>) -> Qux<T> { /* ... */ }
  ```

## Future proofing against conflicts

An idea that was discussed in the tracking issue for the accepted RFC is to future-proof against any conflict among defaults. That means that for a default to apply, _all_ type variables involved must have a default, and it must be the same default. The upside is that adding a default to an existing type parameter becomes backwards-compatible, as a parameter that has no default cannot have any fallback applied to it. However this restricts the usefulness of the feature, for example the following cannot have a fallback applied:

```rust
use std::path::Path;
fn func<P: AsRef<Path> = String>(p: Option<P>) { /* ... */ }

fn main() {
  let x = Ok("/my/path")
  // No fallback here, because we future-proof
  // against the return value of `ok` having a default, even though it never will.
  func(x.ok())
}
```

Soon we would be talking about things like syntax to promise a parameter has no default.

# Unresolved questions

The following unresolved questions should be resolved prior to stabilization, but hopefully shouldn't block the acceptance of the proposal:

### Interaction with numerical fallback

There are multiple alternatives of what to do about the interaction of user fallback with numerical (and diverging) fallback. This was discussed at lenght in [this internals thread](https://internals.rust-lang.org/t/interaction-of-user-defined-and-integral-fallbacks-with-inference/2496). The options are:

1. User fallback takes precedence over numerical fallback, always.
2. Numerical fallback takes precedence, always.
3. DWIM: User fallback takes preference, but if it fails we try numerical fallback.
4. Error on any ambiguity.

The two following examples show the consequences of each alternative, example 1:

```rust
fn foo<T=u64>(t: T) { ... }
// 1. `_` is `u64`
// 2. `_` is `i32`
// 3. `_` is `u64`
// 4. Error.
fn main() { foo::<_>(22) }
```

Example 2:

```rust
fn foo<T=char>(t: T) { ... }
// 1. Error.
// 2. `_` is `i32`
// 3. `_` is `u64`
// 4. Error.
fn main() { foo::<_>(22) }
```

There is a concern with forward compatiblity of this approach presented about expanding the types a literal may be inferred to. Therefore it seems best to take option 1 or 4, as they future extensible to option 3. Any option other than 2 requires a phase-in period. Options 1 and 3 may change the behaviour of existing code, while option 4 may make existing code error. The consensus reached in the thread was for option 4, to be future-extensible but avoiding changing behaviour of existing code, is that still the consensus? 

### Terminology and syntax

Is there a better name for default elision? Default propagation? Default inheritance? Is there a better syntax than `A=_`?

### Hazard to improvements to type checking

Applying fallback seems natural when it's run at the very end of type checking, where you would get the error "type annotation needed". However type checking sometimes needs eagerly resolve a type, infamously in method calls, leading to the error "type must be known in this context". Applying fallback there maybe a hazard to a future where no longer need to eagerly resolve types.

### Interaction with specialization

Consider the example that shows the behaviour of the current implemetation:

```rust
use std::fmt::Debug;

trait B { fn b(&self) -> Self; }

impl<T=String> B for Option<T> where T: Default
{
    default fn b(&self) -> Option<T> {
        Some(T::default())
    }
}
// When there specialized but generic impls, their defaults
// are ignored no matter what they are.
// This code does not compile because `x` in main fails to infer.
// However if we commented out this impl, `x` would be inferred to `String`.
impl<T=String> B for Option<T> where T: Default + Debug
{
    fn b(&self) -> Option<T> { Some(T::default()) }
}

fn main() {
    let x = None;
    let y = x.b();
}
```

We need to figure the design and implemetation of defaults in specialization chains. Probably we want to allow only one default for a parameter in a specialization chain.

