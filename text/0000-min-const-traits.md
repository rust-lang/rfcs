- Feature Name: `min_const_traits`
- Start Date: 2026-09-27
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary
[summary]: #summary

This proposal makes trait methods callable in const contexts:

- Allow marking `trait` declarations as `const` implementable
- Allow marking `impl`s as const, both inherent and trait impls.

Fully contained example:

```rust
const trait Default {
    fn default() -> Self;
}

const impl Default for () {
    fn default() {}
}

struct Thing<T>(T);

const impl<T: Default> Default for Thing<T> {
    fn default() -> Self { Self(T::default()) }
}

struct S {}

const impl S {
    fn default<T: Default>() -> T {
        T::default()
    }
}

const _: () = Default::default();

fn main() {
    let () = S::default();
    let () = Default::default();
}
```

## Motivation
[motivation]: #motivation

(copied from [oli's RFC](https://github.com/oli-obk/rfcs/blob/const-trait-impl/text/0000-const-trait-impls.md))

Const code is currently only able to use a small subset of Rust code, as many standard library APIs and builtin syntax require calling trait methods to work.
As an example, in const contexts, comparing values whose types do not have built-in support is not currently allowed:

```rust
const fn foo() {
    let a = [1, 2, 3];
    let b = [1, 2, 4];
    if a == b {} // ERROR: cannot call non-const operator in constant functions
}
```

Enabling const traits will allow these builtin language constructs to be used in const code, including but not limited to: `for` loops, `?` operator, and user-defined binary operators such as `==`. Furthermore, it will make it possible to call methods on generic types in const code, making them much more expressive.

## Background

(copied from [oli's RFC](https://github.com/oli-obk/rfcs/blob/const-trait-impl/text/0000-const-trait-impls.md))

This RFC requires familiarity with "const contexts", so you may have to read [the relevant reference section](https://doc.rust-lang.org/reference/const_eval.html#const-context) first.

Calling functions during const eval requires those functions' bodies to only use statements that const eval can handle. While it's possible to just run any code until it hits a statement const eval cannot handle, that would mean the function body is part of its semver guarantees. Something as innocent as a logging statement would make the function uncallable during const eval.

Thus we have a marker, `const`, to add in front of functions that requires the function body to only contain things const eval can handle. This in turn allows a `const` annotated function to be called from const contexts, as you now have a guarantee it will stay callable.

When calling a trait method, this simple scheme (that works great for free functions and inherent methods) does not work.

```rust
const fn default<T: Default>() -> T {
    T::default()
}
```

The above shouldn't (and don't) compile.
This is because you could pass any type `T` whose `impl` could

* mutate a global static,
* read from a file, or
* just allocate memory,

which are all not possible right now in const code, and some can't be done in Rust in const code at all.

It should be possible to write `default` in a way that allows it to be called in const contexts
for types whose `Default` impl's `default` method satisfies all rules that `const fn` must satisfy
(including some annotation that guarantees this won't break by accident).
It must always be possible to call `default` outside of const contexts with no limitations on the generic parameters that may be passed.

So, we need some annotation that differentiates a `T: Default` bound from one that gives us the guarantees we're looking for.

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

### Const inherent impls

When an inherent impl is prefixed with `const`, all of its contained methods are expected to be `const`. The bodies of such functions must satisfy all of what `const fn` requires.

```rust
struct Foo {}
const impl Foo {
    fn new() -> Foo { Foo {} }
}
const NEW_FOO: Foo = Foo::new();
// ^ OK, since `Foo::new` is in a `const impl`.
```


```rust
const impl Foo {
    fn incorrect() {
        println!("I am silly!"); // <- Error: println isn't callable in const
    }
}
```

### Const traits and their methods

To allow `const impl`s and trait bounds being used in const contexts, traits must opt-in to be a `const trait`:

```rust
const trait PartialEq {
    fn eq(&self, other: &Self) -> bool;
    fn ne(&self, other: &Self) -> bool {
        !self.eq(other)
    }
}
```

This will require all its default bodies to be callable in compile time, and will allow the trait bound to be used in `const impl`s:

```rust
const impl PartialEq for u32 {
    fn eq(&self, other: &u32) -> bool {
        *self == *other
    }
}
const impl Foo {
    fn equals_self<T: PartialEq>(a: &T) -> bool {
        a == a
    }
}

const _: () = assert!(Foo::equals_self(&1u32);
```

### Generic bounds

When inside a `const impl` or `const trait`, bounds such as `T: PartialEq` will require `T` to provide a `const impl` for `PartialEq`, if called from compile time:

```rust
struct MyBadType {}

impl PartialEq for MyBadType {
    fn eq(&self, _other: &MyBadType) -> bool {
        println!("I like to do silly things!");
        true
    }
}

const _: () = assert!(Foo::equals_self(&MyBadType {}));
// ^ errors: MyBadType doesn't have a `const impl PartialEq`,
//           which is required by `equals_self`
```

These bounds are conditionally applied, so non-const callers will not error:

```rust
fn main() {
    assert!(Foo::equals_self(&MyBadType {}));
    // ^ this is OK, because we aren't calling `equals_self` from a const context.
    //   So it doesn't require `const impl PartialEq` for `MyBadType`.
}
```

This _only_ applies to `const impl` and `const trait`, as `const fn` is not affected and retains the behavior before this RFC:

```rust
const fn welp<T: PartialEq>(_x: &T) {
    // assert!(_x == _x);
    // ^ this would error, as `const fn` does not allow usage of
    //   trait methods on generic types
}

const _: () = welp(&MyBadType {});
// ^ this is fine, because `welp` does not have a const impl requirement
```



### Destructors and rules for ensuring const destructors

When a value goes out of scopes, its destructor should run. This applies to `const` contexts as well. However, some destructors are not compatible to be run in the const context, due to having non-const operations in them:

```rust
impl Drop for MyBadType {
    fn drop(&mut self) {
        println!("I am silly again!");
    }
}
```

This gets caught by static analysis and any code that attempts to run non-const destructors will be promptly rejected. However, that also means rejecting any generic types:

```rust
const fn drop_it<T>(x: T) {
    // ^ error: cannot drop `T` at compile time.
} 
```

Because const traits enables more powerful generic const functions, they will inevitably require running destructors on generic types. To enable that, we use the following two rules:

1. No `const impl`, `const trait` methods, or `const` items/blocks that call them are allowed to create any value that has a non-const destructor.
2. `const fn`s are not allowed to call `const impl` or `const trait` methods. (see [#Allowing-const-fn-to-call-const-impls](#Allowing-const-fn-to-call-const-impls) for the rationale)


```rust
const impl Foo {
    fn new() -> Foo {
        std::mem::forget(MyBadType {}); // *not* ok! ..
        // ^ ..since the `Drop` impl for `MyBadType` isn't `const`.
        Foo {}
    }
}
const fn new_foo() -> Foo {
    Foo::new(); // *not* ok! `const fn` cannot call `const impl`.
}
```

To remain backwards compatible with existing const items, only bodies that call `const impl` methods or `const trait` methods will be subject to this rule. (This may change at a later edition)
```rust
struct NonConstDrop;

impl Drop for NonConstDrop {
    fn drop(&mut self) {
        println!("some non-const operation")
    }
}

const _: () = {
    std::mem::forget(NonConstDrop);
};
// ^ OK: doesn't call any `const impl` or `const trait` methods.

const _: () = {
    let _ = Foo::new();
    std::mem::forget(NonConstDrop);
    // ^ *not* okay!
};
```

Combining the two rules allows us to assume that all types (even generic ones) have a destructor that can be run in compile time, inside `const impl`s:

```rust
const impl Foo {
    fn drops_generic<T>(_x: T) {}
    // ^ OK: we're in a `const impl`
}
const fn drops_generic<T>(_x: T) {}
// ^ *not* okay!
```

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

### Syntax details

It is not permitted to write `const fn` within a `const impl` as it is redundant:

```rust
const impl Foo {
    const fn redundant();
    // ^ errors
}
```

This applies similarly for `const trait`s and trait impls: 

```rust
const trait MyTrait {
    const fn redundant();
    // ^ errors
}

const impl MyTrait for Foo {
    const fn redundant() {}
    // ^ errors
}
```

These, however, may be allowed after a future RFC if it assigns them a special meaning.

If the trait is not const, they cannot be implemented via `const impl`s or referred to in trait bounds in `const impl`s (this includes anywhere: super traits, associated type bounds, etc):

```rust
trait NotConstTrait {}
const impl NotConstTrait for u32 {} // *not* ok!
const impl Foo {
    fn bounds<T: NotConstTrait>() {} // *not* ok!
}
```

### Proving that we can call a type's trait methods in const contexts

The trait solver parts to this are covered in the [const conditions checking chapter](https://rustc-dev-guide.rust-lang.org/effects.html) in rustc-dev-guide.

To prove we can call a type's trait methods, we need to know whether we are calling from a `const` block or regular `const impl` fn. `const` blocks are stricter because functions need to be called in compile time even if the caller is from runtime. That could require a new bound `T: const Trait` ("always const"), but is not specified in this RFC (see [#Explicit const bounds](#Explicit-const-bounds)).

Bounds can be "conditionally const" and "not const". "conditionally const" will require `const impl`s if called from const contexts, but won't require them if not, as explained above. This section will use `T: [const] Trait` to represent "conditionally const" bounds and `T: Trait` as "not const" bounds, but do note that it gets inferred based on whether if inside a `const impl`/`const trait` and `[const]` syntax is not exposed by this RFC.

To prove `T: [const] Trait` for a const context, we need to have `const impl Trait for T` (with all its bounds satisfied too) or have that as a requirement for the function (`T: Trait` within `const trait`/`const impl`).

Built-in impls should be provided, for example `const fn` should satisfy `: [const] Fn` as appropriate.

### Keyword order

Same as `fn`, comes before `unsafe`:

```rust
const unsafe fn hi() {}
const unsafe trait Hi {}
const unsafe impl Hi for () {}
```

### Destructors

If a type has a non-const implementation of `Drop`, it has a non-const destructor:

```rust
struct S {}

impl Drop for S {
    fn drop(&mut self) {}
}
```

Any type that contains a field with a non-const destructor also has a non-const destructor:

```rust
struct ContainsS { inner: S }
```

Conditions the compiler uses to prove a type can be dropped in compile time:

1. For any type, if they have a `const impl Drop for Type` present, that is enough to prove it can be dropped.
    * compiler must check at the `const impl Drop` site that all fields of such type also can be dropped at compile time.
1. Any type `T` that has a clause guaranteeing `T: Copy` can be dropped in compile time.
1. ADTs can be dropped in compile time if all its fields can be dropped in compile time (unless it has `impl Drop`).
1. Compound types can be dropped in compile time if the types they are made out of can be dropped in compile time. e.g., `(A, B)` can be dropped in compile time if and only if both `A` and `B` can be dropped in compile time.
1. Parameter types (`T` from `fn a<T>`) and projection types (`T::A` from `fn a<T: Trait>`) are assumed to be droppable at compile time, but only when within a `const impl` or `const trait` body.

Proving such property is done via a built-in trait that is not exposed to users. 

Because `const fn` cannot assume parameter types and projection types as const droppable as in 5, `const impl` and `const trait` may use a separate built-in trait to distinguish such behavior for the trait solver.

`const impl`, `const trait` methods, as well as any `const` items or blocks that call them, must have their bodies checked: no expressions may produce a value that cannot be dropped at compile time.
* This can be done by checking the types of all locals in the MIR.

#### Opting out

There is no opt-out per trait bound (to not require `const impl` for trait bounds). Because it is [expected](https://cel.cs.brown.edu/const-traits-analysis/) that using const traits' methods is the default and most used. This allows this proposal to be free of any additional specified syntax except `const impl` and `const trait`.

Some traits nevertheless want to be opt out from const bounds wholesale. This includes `{Meta,Pointee,}Sized`, `Copy`, `Tuple`, auto traits, and other marker traits that do not make sense to be `const trait`s. These traits will never become `const trait`s and there can be an internal attribute `#[rustc_never_const_trait]` to allow them to be used in `const impl`s without being const traits, pending any further design on this.

It is expected that, without allowing `const fn` to call `const impl`s, more functions will be written as associated functions. This is a reasonable workaround. Existing `fn` in the standard library, however, cannot be worked around as that would be a breaking change. So it also requires an internal attribute `#[rustc_min_const_traits_fn]` that allows the compiler to treat it like a `const impl` method: It disallows any non-const-destruct values within the body, allows calling `const impl`s, but disallows other normal `const fn` callers.


### When we imply `T: [const] Trait`

The principle to apply here is to always break function bodies and not types in case we do a breaking change across editions.

```rust
const impl Foo {
    fn foo<T: PartialEq>() {}
}

const trait Bar: PartialEq {
    type Assoc: PartialEq;
}
```

All bounds here are assumed to mean `T: [const] PartialEq`. This means all callers of `Foo::foo` must provide a type that satisfies this, and all const implementers of `Bar` must provide a type that satisfies `PartialEq`, both for the super trait and for the associated type.

If such assumption changes to mean "non-const" bound, no callers of `Foo::foo`, or const implementers of `Bar` will need to change because they were satsifying a previously stricter requirement.

Only function bodies that rely on these assumptions will be broken, and that appears relatively easier to migrate for edition changes.

### Banned types and constructs

`fn(T) -> T` function pointers, `impl Trait`, and `dyn Trait` must not appear on any `const impl`s or `const trait`s (or exposed via an unstable feature out of scope for this RFC). Self types, parameter types, return types must be walked recursively to enforce this.


## Drawbacks
[drawbacks]: #drawbacks

### New feature

We're introducing a new feature, that has new concepts to teach and new gotchas to figure out. But enabling powerful compile time computation needs this feature, such as using `Vec` in compile time, and having `for` loops and other iterator methods.


## Rationale, alternatives, and unresolved questions
[rationale-and-alternatives]: #rationale-and-alternatives


### Specifying any syntax

It is quite true that `T: [const] Trait` is the default bound mode in these contexts. They definitely get used more often than not. Considering an opt-in scheme would have to specify a syntax that remains compatible for other opt-in schemes in the future (other effects? `T: async Trait`). This is hard.

Opting out, similarly, requires specifying a syntax to do so. That is also hard given that it has future considerations with other effects, if we want to add them.

### Allowing `const fn` to call `const impl`s

Because `const fn` doesn't get to call `const impl` methods, many libraries will switch to using inherent `const impl` even if functions can be free functions. However, migrating associated `const impl` functions to and from standalone `const fn` is a breaking change for both directions. This can cause a wave of libraries essentially being stuck using associated `const impl` functions even if we enable `const fn` to use this feature in the future.

This appears to be a reasonable trade-off: We get to specify the new semantics for `const impl` and `const trait`s and design this to add the support we need. If eventually we come up with a design that makes `const fn` usable, library authors can also deprecate the old `const impl` methods that they believe exist more naturally as `const fn`. Breaking changes for library authors is less costly than breaking changes for the Rust language.

We could invent a new opt-in for `const fn` so that they are restricted and cannot construct any non-const-drop values. We could also specify that the restriction applies to a new edition, and only then can they call `const impl`s. These are valid and would unblock many use cases, but would require a separate syntax from `const fn` (otherwise, we could use an attribute?)

Implicitly opting `const fn` in if they call any `const impl` has the bad effect that the function body now affects the property of the function, because it is transitively colored. If `const fn` calls a `const impl`, then any `const fn` that calls it also need to be restricted to not make any non-const-drop values. This is very difficult for both users and compiler developers.

At time of proposal, no `const fn` use `const impl`/`const trait`s in public libraries, as such language feature is unstable/experimentally supported. We can take the most conservative path to allow developing this feature further, while exposing a set of them to the users to make many useful things.

### Exposing `Destruct`

This refers to the currently unstable trait `Destruct` being used to require and prove a type can be dropped in compile time.

Naming such a trait `Destruct` is up to debate. Having more than one trait for destructors/dropping is likewise confusing. Not exposing them/specifying what it is in this proposal helps avoid bikeshed for the naming/how to reconcile duplication with the `Drop` trait.

In the future, if Rust provides support for `Linear` types, that might help us expose types that should always be consumed instead of dropped.

### Allowing `fn` pointers, `dyn Trait` and `impl Trait`

It would be pretty hard to know whether we also want to implicitly require `const fn` or `const impl` for them. Properly reserving them and making them unstable appears to be a good idea until we have a more concrete story to act upon.

## Prior art
[prior-art]: #prior-art

Allowing const functions to call generic trait methods was originally proposed as [RFC 2632](https://github.com/rust-lang/rfcs/pull/2632) (2019-02-05), which was provisionally approved for experimentation (tracked at [#67792](https://github.com/rust-lang/rust/issues/67792)). After compiler support matured for a while, an opt-in syntax `~const` was proposed along with other changes, and resulted in a [revamped proposal](https://internals.rust-lang.org/t/pre-rfc-revamped-const-trait-impl-aka-rfc-2632/15192) (2021-08-18) posted to IRLO.

Further experimentation and improvements to compiler support ensued. On 2024-01-07, project-const-traits was [created](https://github.com/rust-lang/team/pull/1173) as a subteam of T-compiler to organize effort around const traits implementation within the compiler. After the implementation settled, [RFC 3762](https://github.com/rust-lang/rfcs/pull/3762) opened on 2025-01-13.

On 2025-08-20, [@fee1-dead](https://github.com/fee1-dead) published a [blogpost](https://dbeef.dev/const-trait-counterexamples/) summarized common refutations for arguments around this language feature.

See also [Prior art](https://github.com/oli-obk/rfcs/blob/const-trait-impl/text/0000-const-trait-impls.md#prior-art) from RFC 3762.



## Future possibilities
[future-possibilities]: #future-possibilities


### Const closures

One should be able to define closures as callable from within const contexts, e.g. `const |x| x * 2` or similar. It is nice to have, however that requires additional syntax consideration and should be left out of this proposal.

### Const derives

Derives for traits should distinguish whether users want to generate `const impl`s. This ties into other language proposals to make `derive` better, and should be grouped together with those language proposals. This is because while it is easy to add `const impl` generation for built-in impls, existing user-defined `derive`s don't have a way to figure out whether a user requests a `const impl`, and we'd also need a mechanism for the user to ensure such a request is respected.

### Const `fn` pointers, `dyn Trait`s, and `impl Trait`

Even non-const versions of these are not currently allowed within `const impl` and `const trait` blocks, to allow maximum freedom in how we want to deal with them in a future language proposal.

### Explicit const bounds

There could also be a bound `T: const Trait` that allows calling `T`'s methods within a const block (unlike `const fn`, a `const` block would always be evaluated in compile time, so it would be a stricter bound). However that is orthogonal to the feature we're proposing, and I intend this proposal to be as minimal as possible.

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

## Credits

Huge thanks to [oli](https://github.com/oli-obk) for pushing this proposal and sticking around for 7 years, laying out the basis for much of the language design and compiler implementation.

Thanks [Michael Goulet](https://github.com/compiler-errors), [fee1-dead](https://github.com/fee1-dead), [oli](https://github.com/oli-obk), [fmease](https://github.com/fmease), and many others that helped refine the experimental compiler support for const traits, that in turn informed many language design choices in syntax and semantics.

Thanks to [nikomatsakis](https://github.com/nikomatsakis/), [TC](https://github.com/traviscross), and many others who have participated in language discussions and helped shaped this proposal.

Thanks to [Will Crichton](https://github.com/willcrichton/) and [Jack Huey](https://github.com/jackh726) for running [the survey](https://cel.cs.brown.edu/const-traits-analysis/) to help identify what is a reasonable default for const bound's semantics, that informed the direction of this RFC.