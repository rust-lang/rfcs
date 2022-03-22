# Refined trait implementations

- Feature Name: `refined_impls`
- Start Date: 2022-03-22
- RFC PR: [rust-lang/rfcs#3245](https://github.com/rust-lang/rfcs/pull/3245)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC generalizes the [`safe_unsafe_trait_methods` RFC][safe_unsafe], allowing implementations of traits to add type information about the API of their methods and constants which then become part of the API for that type. Specifically, lifetimes and where clauses are allowed to extend beyond what the trait provides.

[safe_unsafe]: https://rust-lang.github.io/rfcs/2316-safe-unsafe-trait-methods.html

# Motivation
[motivation]: #motivation

[RFC 2316][safe_unsafe] introduced the notion of _safe implementations_ of unsafe trait methods. This allows code that knows it is calling a safe implementation of an unsafe trait method to do so without using an unsafe block. In other words, this works today:

```rust
trait Foo {
    unsafe fn foo(&self);
}

struct Bar;
impl Foo for Bar {
    fn foo(&self) {
        println!("No unsafe in this impl!")
    }
}

fn main() {
    // Call Bar::foo without using an unsafe block.
    let bar = Bar;
    bar.foo();
}
```

Unsafe is not the only area where we allow impl signatures to be "more specific" than the trait they're implementing. Unfortunately, we do not handle these cases consistently today:

### Associated types

Associated types are a case where an impl is _required_ to be "more specific" by specifying a concrete type.

```rust
struct OnlyZero;

impl Iterator for OnlyZero {
    type Item = usize;
    fn next(&mut self) -> Option<Self::Item> {
        Some(0)
    }
}
```

This concrete type is fully transparent to any code that can use the impl. Calling code is allowed to rely on the fact that `<OnlyZero as Iterator>::Item = usize`.

```rust
let mut iter = OnlyZero;
assert_eq!(0usize, iter.next().unwrap());
```

### Types in method signatures
[not-usable]: #types-in-method-signatures

We also allow method signatures to differ from the trait they implement.

```rust
trait Log {
    fn log_all(iter: impl ExactSizeIterator);
}

struct OrderedLogger;

impl Log for OrderedLogger {
    // Don't need the exact size here; any iterator will do.
    fn log_all(iter: impl Iterator) { ... }
}
```

**Unlike with `unsafe` and associated types, however, calling code _cannot_ rely on the relaxed requirements on the `log_all` method implementation.**

```rust
fn main() {
    let odds = (1..50).filter(|n| *n % 2 == 1);
    OrderedLogger::log_all(odds)
    // ERROR:              ^^^^ the trait `ExactSizeIterator` is not implemented
}
```

This is a papercut: In order to make this API available to users the `OrderedLogger` type would have to bypass the `Log` trait entirely and provide an inherent method instead. Simply changing `impl Log for OrderedLogger` to `impl OrderedLogger` in the example above is enough to make this code compile, but it would no longer implement the trait.

The purpose of this RFC is to fix the inconsistency in the language and add flexibility by removing this papercut. Finally, it establishes a policy to prevent such inconsistencies in the future.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When implementing a trait, you can use function signatures that _refine_ those in the trait by being more specific. For example,

```rust
trait Error {
    fn description(&self) -> &str;
}

impl Error for MyError {
    fn description(&self) -> &'static str {
        "My Error Message"
    }
}
```

Here, the error description for `MyError` does not depend on the value of `MyError`. The `impl` includes this information by adding a `'static` lifetime to the return type.

Code that knows it is dealing with a `MyError` can then make use of this information. For example,

```rust
fn attempt_with_status() -> &'static str {
    match do_something() {
        Ok(_) => "Success!",
        Err(e @ MyError) => e.description(),
    }
}
```

This can be useful when using impl Trait in argument or return position.[^rpitit]

```rust
trait Iterable {
    fn iter(&self) -> impl Iterator;
}

impl<T> Iterable for MyVec<T> {
    fn iter(&self) -> impl Iterator + ExactSizeIterator { ... }
}
```

Note that when using impl Trait in argument position, the function signature is refined as bounds are _removed_, meaning this specific impl can accept a wider range of inputs than the general case. Where clauses work the same way: since where clauses always must be proven by the caller, it is okay to remove them in an impl and permit a wider range of use cases for your API.

```rust
trait Sink {
    fn consume(&mut self, input: impl Iterator + ExactSizeIterator);
}

impl Sink for SimpleSink {
    fn consume(&mut self, input: impl Iterator) { ... }
}
```

Finally, methods marked `unsafe` in traits can be implemented as safe APIs, allowing code to call them without using `unsafe` blocks.

[^rpitit]: At the time of writing, return position impl Trait is not allowed in traits. The guide text written here is only for the purpose of illustrating how we would document this feature if it were allowed.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Trait implementations

The following text should be added after [this paragraph](https://doc.rust-lang.org/nightly/reference/items/implementations.html#trait-implementations) from the Rust reference:

> A trait implementation must define all non-default associated items declared by the implemented trait, may redefine default associated items defined by the implemented trait, and cannot define any other items.

Each associated item defined in the implementation meet the following conditions.

**Associated consts**

* Must be a subtype of the type in the corresponding trait item.

**Associated types**

* Associated type values must satisfy all bounds on the trait item.
* Each where clause must be implied by the where clauses on the trait itself and/or the associated type in the trait definition, where "implied" is limited to supertrait and outlives relations. _This would be expanded to all [implied bounds] when that feature is enabled._

**Associated functions**

* Must return any subtype of the return type in the trait definition.
* Each argument must accept any supertype of the corresponding argument type in the trait definition.
* Each where clause must be implied by the where clauses on the trait itself and/or the associated function in the trait definition, where "implied" is limited to supertrait and outlives relations. _This would be expanded to all [implied bounds] when that feature is enabled._
* Must not be marked `unsafe` unless the trait definition is also marked `unsafe`.

When an item in an impl meets these conditions, we say it is a valid refinement of the trait item.

[implied bounds]: https://rust-lang.github.io/rfcs/2089-implied-bounds.html

### Using refined implementations

Refined APIs are available anywhere knowledge of the impl being used is available. If the compiler can deduce a particular impl is being used, its API is available for use by the caller. This includes UFCS calls like `<MyType as Trait>::foo()`.

## Transitioning away from the current behavior

Because we allow writing impls that look refined, but are [not usable][not-usable] as such, landing this feature means we are auto-stabilizing new ecosystem API surface. There are two ways of dealing with this:

### Do nothing

Assume that public types want to expose the APIs they actually wrote in their implementations, and allow using those APIs immediately.

### Soft transition

Be conservative and require library authors to opt in to refined APIs. This can be done in two parts.

#### Lint against unmarked refined impls

After this RFC is merged, we should warn when a user writes an impl that looks refined and suggest that they copy the exact API of the trait they are implementing. Once this feature stabilizes, we can should add and suggest using `#[refine]` attribute to mark that an impl is intentionally refined.

#### Automatic migration for the next edition

Because refinement will be the default behavior for the next edition, we should rewrite users' code to preserve its semantics over edition migrations. That means we will replace trait implementations that look refined with the original API of the trait items being implemented.

#### Documentation

The following text should be added to document the difference in editions.

For historical reasons, not all kinds of refinement are automatically supported in older editions.

| Item kind   | Feature                         | Edition        |
| ----------- | ------------------------------- | -------------- |
| Type        | -                               | All editions   |
| Method      | Unsafe                          | All editions   |
| Method      | Const[^future]                  | All editions|
| Method      | impl Trait in return position[^future]| All editions|
| Method      | Lifetimes                       | 2024 and newer |
| Method      | Where clauses                   | 2024 and newer |
| Method      | impl Trait in argument position | 2024 and newer |
| Const       | Lifetimes                       | 2024 and newer |
| Const       | Where clauses                   | 2024 and newer |

[^future]: This feature is not accepted at the time of writing the RFC; it is included here for demonstration purposes.

You can opt in to the new behavior in older editions with a `#[refine]` attribute on the associated item.

```rust
impl Error for MyError {
    #[refine]
    fn description(&self) -> &'static str {
        "My Error Message"
    }
}
```

This enables refining all features in the table above.

## Preventing future ambiguity

This RFC establishes a policy that anytime the signature of an associated item in a trait implementation is *allowed to differ* from the signature in the trait, the information in that signature should be usable by code that uses the implementation.

This RFC specifically does not specify that new language features involving traits *should* allow refined impls wherever possible. The language could choose not to accept refined implementation signatures for that feature. This should be decided on a case-by-case basis for each feature.

## Interaction with other features

### Implied bounds

When [implied bounds] is stabilized, the rules for valid refinements will be modified according to the italicized text above.

### Specialization

[Specialization] allows trait impls to overlap. Whenever two trait impls overlap, one must take precedence according to the rules laid out in the specialization RFC. Each item in the impl taking precedence must be a valid refinement of the corresponding item in the overlapping impl.

[specialization]: https://rust-lang.github.io/rfcs/1210-impl-specialization.html

### Generic associated types

These features mostly don't interact. However, it's worth noting that currently generic associated types [require extra bounds][87479] on the trait definition if it is likely they will be needed by implementations. This feature would allow implementations that don't need those bounds to elide them and remove that requirement on their types' interface.

[87479]: https://github.com/rust-lang/rust/issues/87479

### `const` polymorphism

We may want to allow implementations to add `const` to their methods. This raises the question of whether we want *provided* methods of the trait to also become `const`. For example:

```rust
impl Iterator for Foo {
    const fn next(&mut self) -> ...
}
```

Should the `nth` method also be considered `const fn`?

# Drawbacks
[drawbacks]: #drawbacks

> Why should we *not* do this?

## Accidental stabilization

For library authors, it is possible for this feature to create situations where a more refined API is *accidentally* stabilized. Before stabilizing, we will need to gain some experience with the feature to determine if it is a good idea to allow refined impls without annotations.

## Complexity

Overall, we argue that this RFC reduces complexity by improving the consistency and flexibility of the language. However, this RFC proposes several things that can be considered added complexity to the language:

### Adding text to the Rust reference

Part of the reason that text is being added to the reference is that the reference doesn't specify what makes an item in a trait implementation valid. The current behavior of allowing certain kinds of divergence and "ignoring" some of them is not specified anywhere, and would probably be just as verbose to describe.

### Types are allowed to have different APIs for the same trait

It is possible for a user to form an impression of a trait API by seeing its use in one type, then be surprised to find that that usage does not generalize to all implementations of the trait.

It's rarely obvious, however, that a *trait* API is being used at a call site as opposed to an inherent API (which can be completely different from one type to the next). The one place it is obvious is in generic functions, which will typically only have access to the original trait API.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This RFC attempts to be minimal in terms of its scope while accomplishing its stated goal to improve the consistency of Rust. It aims to do so in a way that makes Rust easier to learn and easier to use.

## Do nothing

Doing nothing preserves the status quo, which as shown in the [Motivation] section, is confusing and inconsistent. Allowing users to write function signatures that aren't actually visible to calling code violates the principle of least surprise. It would be better to begin a transition out of this state sooner than later to make future edition migrations less disruptive.

## Require implementations to use exactly the same API as the trait

We could reduce the potential for confusion by disallowing "dormant refinements" with a warning in the current edition, as this RFC proposes, and an error in future editions. This approach is more conservative than the one in this RFC. However, it leaves Rust in a state of allowing some kinds of refinement (like safe impls of `unsafe` methods) but not others, without a clear reason for doing so.

While we could postpone the question of whether to allow this indefinitely, we argue that allowing such refinements will make Rust easier to learn and easier to use.

## Allow `#[refine]` at levels other than impl items

We could allow `#[refine]` on individual aspects of a function signature like the return type, where clauses, or argument types. This would allow users to scope refinement more narrowly and make sure that they aren't refining other aspects of that function signature. However, it seems unlikely that API refinement would be such a footgun that such narrowly scoping is needed.

Going in the other direction, we could allow `#[refine]` on the impl itself. This would remove repetition in cases where an impl refines many items at once. It seems unlikely that this would be desired frequently enough to justify it.

# Prior art
[prior-art]: #prior-art

### Java covariant return types

If you override a method in Java, the return type can be any subtype of the original type. When invoking the method on that type, you see the subtype.

### Auto traits

One piece of related prior art here is the [leakage of auto traits][auto-leakage] for return position `impl Trait`. Today it is possible for library authors to stabilize the auto traits of their return types without realizing it. Unlike in this proposal, there is no syntax corresponding to the stabilized API surface.

[auto-leakage]: https://rust-lang.github.io/rfcs/1522-conservative-impl-trait.html#oibit-transparency

# Unresolved questions
[unresolved-questions]: #unresolved-questions

## Do we need a soft transition?

In "Transitioning away from the current behavior" we describe two possible paths: immediate stabilization of any API the compiler accepts that happens to look refined today, and doing a soft transition.

While a soft transition is the more conservative approach, it also isn't obvious that it's necessary.

It would help to do an analysis of how frequently "dormant refinements" occur on crates.io today, and of a sample of those, how many look accidental versus an extended API that a crate author might have meant to expose.

## Should `#[refine]` be required in future editions?

As discussed in [Drawbacks], this feature could lead to library authors accidentally publishing refined APIs that they did not mean to stabilize. We could prevent that by requiring the `#[refine]` attribute on any refined item inside an implementation.

If we decide to require the `#[refine]` annotation in future editions for all refinements, the only edition change would be that the lint in earlier editions becomes a hard error in future editions.

Alternatively, we may even want to require annotations for more subtle features, like lifetimes, while not requiring them for "louder" things like `impl Trait` in return position.

This question would also benefit from the analysis described in the previous section.

# Future possibilities
[future-possibilities]: #future-possibilities

## Return position `impl Trait` in traits

One motivating use case for refined impls is return position impl trait in traits, which is not yet an accepted Rust feature. You can find more details about this feature in an [earlier RFC](https://github.com/rust-lang/rfcs/pull/3193). Its use is demonstrated in an [example][guide-level-explanation] at the beginning of this RFC.

This RFC is intended to stand alone, but it also works well with that proposal.

### Equivalence to associated types

One of the appealing aspects of this feature is that it can be desugared to a function returning an associated type.

```rust
trait Foo {
    fn get_state(&self) -> impl Debug;
}

// Desugars to something like this:
trait Foo {
    type Foo = impl Debug;
    fn get_state(&self) -> Self::Foo;
}
```

If a trait used associated types, implementers would be able to specify concrete values for those types and let their users depend on it.

```rust
impl Foo for () {
    type Foo = String;
    fn get_state(&self) -> Self::Foo { "empty state".to_string() }
}

let _: String = ().foo();
```

With refinement impls, we can say that this desugaring is equivalent because return position impl trait would give the exact same flexibility as associated types.

## Adding generic parameters

This RFC allows implementers to replace return-position `impl Trait` with a concrete type. Conversely, sometimes it is desirable to *generalize* an argument from a concrete type to `impl Trait` or a new generic parameter.

```rust
fn one_a(input: String) {}
fn one_b(input: impl Display) {}
```

More generally, one way to refine an interface is to generalize it by introducing new generics. For instance, here are some more pairs of "unrefined" APIs `a` and refined versions of them `b`.

```rust
fn two_a(input: String) {}
fn two_b<T: Debug = String>(input: T) {}

fn three_a<'a>(&'a i32, &'a i32) {}
fn three_b<'a, 'b>(&'a i32, &'b i32) {}
```

It might also be desirable to turn an elided lifetime into a lifetime parameter so it can be named:

```rust
fn four_a(&self) -> &str {}
fn four_b<'a>(&'a self) -> &'a str {}
```

Adding generic parameters to a trait function is not allowed by this proposal, whether the parameters are named or created implicitly via argument-position `impl Trait`. In principle it could work for both cases, as long as named parameters are defaulted. Implementing this may introduce complexity to the compiler, however. We leave the question of whether this should be allowed out of scope for this RFC.
