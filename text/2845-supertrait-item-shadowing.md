- Feature Name: `supertrait_item_shadowing`
- Start Date: 2020-01-06
- RFC PR: [rust-lang/rfcs#2845](https://github.com/rust-lang/rfcs/pull/2845)
- Rust Issue: [rust-lang/rust#89151](https://github.com/rust-lang/rust/issues/89151)

# Summary
[summary]: #summary

Change item resolution for generics and trait objects so that a trait bound does not bring its supertraits' items into scope if the subtrait defines an item with this name itself.

# Motivation
[motivation]: #motivation

Consider the following situation:

```rust
mod traits {
	trait Super {
		fn foo(&self);
	}

	trait Sub: Super {
		fn foo(&self);
	}
}
```

A trait `Sub` with a supertrait `Super` defines a method with the same name as one in `Super`.

If `Sub` is used as a generic bound, or as a trait object, trying to use the `foo` method raises an error:

#### Generics:

```rust
use traits::Sub;

fn generic_fn<S: Sub>(x: S) {
	x.foo();
}
```

#### Trait objects:

```rust
use traits::Sub;

fn use_trait_obj(x: Box<dyn Sub>) {
	x.foo();
}
```

Both of these currently raise the following error:

```
error[E0034]: multiple applicable items in scope
  --> src\main.rs:10:4
   |
10 |     x.foo();
   |       ^^^ multiple `foo` found
   |
note: candidate #1 is defined in the trait `traits::Super`
  --> src\main.rs:2:2
   |
2  |     fn foo(&self);
   |     ^^^^^^^^^^^^^^
   = help: to disambiguate the method call, write `traits::Super::foo(x)` instead
note: candidate #2 is defined in the trait `traits::Sub`
```

Note that the trait bound is always `Sub`, `Super` is not mentioned in the user code that errors. The items of `Super` are only in scope because the bound on `Sub` brought them into scope.

As the diagnostic mentions, Universal function call syntax (UFCS) will work to resolve the ambiguity, but this is unergonomic. More pressingly, this ambiguity can in fact create a [Fragile base class problem](https://en.wikipedia.org/wiki/Fragile_base_class) that can break library users' code. Consider the following scenario:

#### Initial situation:
[fragile-base-class]: #fragile-base-class

There are three crates in this scenario: A low-level library, a high-level library that depends on the low-level one, and user code that uses the high-level library. The high-level library uses the trait from the low-level library as a supertrait, and the user code then uses the high-level library's trait as a generic bound:

Low-level library:
```rust
mod low {
	pub trait Super {

	}
}
```

High-level library:
```rust
use low::Super;

mod high {
	pub trait Sub: Super {
		fn foo(&self);
	}
}
```

User code:
```rust
use high::Sub;

fn generic_fn<S: Sub>(x: S) {
	// ok
	x.foo();
}
```

#### Library change:

At some point in time, the low-level library's supertrait gets refactored to have a method that also happens to be called `foo`:
```rust
mod low {
	pub trait Super {
		fn foo(&self);
	}
}
```

The user code is unchanged, but breaks:
```rust
use high::Sub;

fn generic_fn<S: Sub>(x: S) {
	// error: both Super::foo and Sub::foo are in scope
	x.foo();
}
```

A change to add a supertrait to a public trait or to add a method to an existing supertrait can therefore cause downstream breakage. Notably, the user code was never aware of the supertrait, and the low-level library could never have known the signatures of the high-level library. Taking care not to introduce name conflicts is therefore not possible, since any name that is safe at present could cause a conflict in the future, both from the perspective of the low- and the high-level library:

- The low-level library can't know all the crates that depend on it, and how they use the trait. It's not possible for it to go through dependent crates and check if the name already exists.
- The high-level library was "first" in defining the name, at the time of naming the issue didn't exist. When the low-level library changes, it's now stuck. Either it can't update its dependency, or it is forced to rename its method, which is a breaking change.

This kind of change is acknowledged as breaking but minor by the [API Evolution RFC](https://github.com/rust-lang/rfcs/blob/master/text/1105-api-evolution.md#minor-change-adding-a-defaulted-item). However, it does not specifically consider the case of sub-/supertrait interaction. It turns out that in this specific situation there is a potential solution for the problem that avoids the breakage in the first place.

To resolve this issue, this RFC proposes the following: If the user does not explicitly bring the supertrait into scope themselves, the subtrait should "shadow" the supertrait, resolving the current ambiguity in favor of the subtrait.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When using a trait as a bound in generics, or using a trait object, a trait with a supertrait will only bring the supertrait's items into scope if it does not define an item with the same name itself. Items on the object in question that were previously ambiguous now resolve to the subtrait's implementation. While it is still possible to refer to the conflicting supertrait's items, it requires UFCS. Supertrait items that were not previously ambiguous continue to be in scope and are usable without UFCS.

In the context of the trait examples above, this means:

```rust
fn generic_fn<S: Sub>(x: S) {
	// This:
	x.foo();
	// is the same as:
	Sub::foo(x);
	// also still possible:
	Super::foo(x);
}
```

```rust
fn use_trait_obj(x: Box<dyn Sub>) {
	// This:
	x.foo();
	// is the same as:
	Sub::foo(x);
	// also still possible:
	Super::foo(x);
}
```

However, when both subtrait and supertrait are brought into scope, the ambiguity remains:

```rust
fn generic_fn<S: Sub+Super>(x: S) {
	// Error: both Sub::foo and Super::foo are in scope
	x.foo();
}
```

This solution makes intuitive sense: If the user requested `Sub` and not `Super`, they should get `Sub`'s items, not `Super`'s. In fact, the user might never have known about the existence of `Super`, in which case the error message would be confusing to them.

Choosing not to resolve ambiguities when both traits are explicitly requested similarly makes sense: Both traits seem to be wanted by the user, so it's not immediately clear which trait should take precedence.

The feature is backwards-compatible: Anything changed by this proposal was previously rejected by the compiler. As seen in the motivation section, it also improves forwards-compatibility for libraries.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Currently, when a trait is brought into scope through generics or trait objects, all of its supertrait's items are brought into scope as well. Under this proposal, a supertrait's items would only be brought into scope if an item with that name is not already present in the subtrait. This extends to the case of multiple supertraits without special provisions, the rule is simply applied for each supertrait.

Specifically, if two supertraits of a subtrait conflict with *each other*, but not with the subtrait, it is still an error to refer to the item without UFCS, just as it is today:

```rust
trait Super1 {
	fn foo(&self);
}

trait Super2 {
	fn foo(&self);
}

trait Sub: Super1+Super2 {}

fn generic_fn<S: Sub>(x: S) {
	// Is and will continue to be an error
	x.foo();
}
```

The resolution rule applies recursively to super-supertraits as well:

```rust
trait SuperSuper {
	fn foo(&self);
	fn bar(&self);
}

trait Super: SuperSuper {
	fn foo(&self);
	fn bar(&self);
}

trait Sub: Super {
	fn foo(&self);
}

fn generic_fn<S: Sub>(x: S) {
	// Resolves to Sub::foo
	x.foo();
	// Resolves to Super::bar
	x.bar();
}

fn generic_fn_2<S: Sub+SuperSuper>(x: S) {
	// Error: both Sub::foo and SuperSuper::foo are in scope
	x.foo();
}
```

A case previously not presented, but also technically affected by this RFC, is the definition of a trait itself. Supertraits are brought into scope here as well, through the act of defining them as supertraits in the first place. Therefore, a situation like the following might also be of interest:

```rust
trait Super {
	fn foo(&self);
}

trait Sub: Super {
	fn foo(&self);

	fn bar(&self) {
		// Is and will continue to be an error
		self.foo();
	}
}
```

Using `self.foo()` is an error today, and it is reasonable to expect it to be, since it is not clear which trait it refers to. Under the rule laid out above, this will continue to raise an error, since `Super` is explicitly mentioned and brought into scope.

# Drawbacks
[drawbacks]: #drawbacks

This change makes the implementation of bringing supertrait items into scope a bit more complex.

While it is not the intent of this RFC, the resolution strategy it introduces is somewhat similar to how inheritance in object-oriented languages works. Users coming from those languages may be confused when they realize that Rust actually works differently. Specifically, items in Rust aren't inherited, and methods with the same name will only shadow and not override. (See also [Prior art](#prior-art) for this distinction.)

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The simplest alternative is to continue with the status quo, and require the user to explicitly disambiguate items with the same name using UFCS.

The affected area of this RFC is fairly minor, and items with the same name don't seem to come up often enough that it would be urgently needed. (The issue was [reported](https://github.com/rust-lang/rust/issues/17151) in 2014, but has so far attracted little attention.)

However, the current behavior seems to go against the spirit of why supertrait items are brought into scope: to make traits more ergonomic by avoiding additional trait bounds or explicit supertrait naming in UFCS. Ironically, right now this introduces a requirement for UFCS that wouldn't exist without the automatic scoping of supertraits.

Moreover, as demonstrated in the Motivation section, it is currently possible to inadvertently introduce downstream breakage by changing a supertrait. As outlined in the example, this can cause breakage to bubble down multiple layers of dependencies and cause errors far from the origin of the change. The breakage is especially unexpected by the user because they didn't mention the supertrait or its library themselves.

## Alternatives
[alternatives]: #alternatives

#### Resolving in favor of the *super*trait instead
[resolving-in-favor-of-supertrait]: #resolving-in-favor-of-supertrait

While it is theoretically possible to resolve in favor of the *super*trait, this is very counterintuitive and there is no reason to do so. There can't be a converse "fragile derived class problem", because the subtrait knows all its supertraits. Therefore, before adding a method in the subtrait, all the supertraits can be checked for a method of the same name. This is not possible in the "fragile base class" case because the supertrait can't know all its subtraits.

#### *Always* resolving in favor of the subtrait
[always-resolving-in-favor-of-subtrait]: #always-resolving-in-favor-of-subtrait

This RFC's resolution strategy explicitly rejects resolving items when both the sub- and the supertrait have been brought into scope by the user explicitly. An alternative would be to *always* resolve in favor of the subtrait. It can be argued that this is in line with an intuitive notion of `Sub` specializing `Super`. However, there are a few drawbacks to this strategy:

- This would be inconsistent with how `use` declarations work, although they mainly work that way because they don't bring supertrait items into scope.

- This RFC already resolves in favor of the subtrait when the supertrait is not explicitly brought into scope. Explicitly specifying the supertrait is likely done for a reason, and it would appear counterintuitive when the mention of the supertrait does not influence resolution at all.

- If the user wants resolution to be in favor of the subtrait, all they have to do is remove the explicit mention of the supertrait. The non-conflicting supertrait items will continue to work anyway, since they are implied by the subtrait.

#### Order-dependent resolution
[order-dependent-resolution]: #order-dependent-resolution

Another possibility in the face of ambiguity would be to resolve in favor of the last specified trait, so that a bound on `Sub + Super` resolves in favor of `Super`, while `Super + Sub` resolves in favor of `Sub`. However this adds semantic meaning to the ordering of traits in bounds, which right now is order-agnostic. It's also not very clear and may lead to user confusion when bounds are reordered, which could change program behavior in subtle ways.

All in all, rejecting to resolve ambiguity seems like the right way to go.

# Prior art
[prior-art]: #prior-art

#### Typeclasses in Haskell
[haskell-typeclasses]: #haskell-typeclasses

Haskell's typeclasses are closely related to Rust's traits, so it's worth seeing what the situation looks like in Haskell.

In Haskell, defining a typeclass constraint on a function does not automatically bring into scope methods defined in superclasses, no matter what they're called. To use them, they have to be explicitly imported. In fact, since Haskell binds method names to the module namespace, and there is no separate namespace for typeclasses, not even methods from the explicitly specified subclass are imported. They are only available if they are imported themselves, either by importing the entire module or by importing the methods by name explicitly.

As a result, this RFC's issue doesn't arise in Haskell in the first place, similarly to how it doesn't arise when `use` notation is used in Rust.

#### Inheritance in object-oriented languages
[oop-inheritance]: #oop-inheritance

The general idea of a method call on an object resolving to the most derived class's implementation is typically the way inheritance works in object-oriented languages. A notable distinction is that in object-oriented languages, defining a method with the same name (actually, signature) typically *overrides* the base class's method, meaning that the derived implementation will be selected even in a context that only references the base class. This is not how traits work in Rust, supertraits have no way of calling subtrait implementations, and this will not be changed by this RFC. Instead, the subtrait will only *shadow* the supertrait's items, and `Super::foo` and `Sub::foo` will still be two distinct functions.

Similar distinctions exist in
- [Visual Basic](https://docs.microsoft.com/en-us/dotnet/visual-basic/programming-guide/language-features/declared-elements/shadowing#shadowing-through-inheritance),
- [C#](https://en.wikipedia.org/wiki/Method_overriding#C#), (called "hiding"),
- [Java](https://docs.oracle.com/javase/tutorial/java/IandI/override.html), (called "hiding", however the concept is more limited, with instance methods always overriding and static methods always hiding).

# Unresolved questions
[unresolved-questions]: #unresolved-questions

#### Terminology
[terminology]: #terminology

It's not immediately clear which terminology to use for this behavior. Both "shadowing" and "hiding" are used for immediately related  behaviors in object-oriented languages,  with ["variable shadowing"](https://en.wikipedia.org/wiki/Variable_shadowing) also being used more generally for variable scopes, and ["name masking"](https://en.wikipedia.org/wiki/Name_resolution_(programming_languages)#Name_masking) used for the same concept as "variable shadowing" but from a different perspective. The concept of variable shadowing also already exists in Rust today, and the similar name could be a source of confusion.

There's also some clarification needed on how the term should be used: Should shadowing only be used for the items ("`Sub::foo` shadows `Super::foo`") or also for the traits themselves ("subtraits shadow supertraits")?

Note: using "hiding" may lead to a different interpretation here: "subtraits hide supertraits" sounds like the entirety of the supertrait is hidden.

#### Further fragile base class situations
[further-fragile-base-class]: #further-fragile-base-class

The situation laid out above is actually not the only fragile base class situation in Rust. Consider the following:

```rust
trait Super1 {
	fn foo(&self);
}

trait Super2 {
	// fn foo(&self);
}

trait Sub: Super1+Super2 {}

fn generic_fn<S: Sub>(x: S) {
	x.foo();
}
```

The above will compile, however adding a method `foo` to `Super2` will result in an error due to the introduced ambiguity. This RFC's resolution strategy won't immediately help here either, since the error does not result from a sub-/supertrait interaction. In fact, this is a fundamental problem of multiple inheritance.

However, using this RFC's rule, it would be possible to at least manually change `Sub` to prevent the breakage from flowing further downstream:

```rust
trait Super1 {
	fn foo(&self);
}

trait Super2 {
	fn foo(&self);
}

trait Sub: Super1+Super2 {
	fn foo(&self) {
		Super1::foo(self);
	}
}

fn generic_fn<S: Sub>(x: S) {
	x.foo();
}
```

By manually resolving the ambiguity the error can be avoided. This RFC's part here is to make it possible to resolve the ambiguity close to where it originates, instead of having to do it at the level of `generic_fn`, where the supertraits were never explicitly mentioned. However, this RFC is not able to resolve the fundamental issue, and it is considered out of scope for this RFC, which is intended to deal only with single "inheritance".

# Future possibilities
[future-possibilities]: #future-possibilities

#### `use` declarations
[use-declarations]: #use-declarations

As mentioned before, `use` declarations work differently from trait bounds in generics and trait objects. There may be some benefit in unifying their behavior, so that `use` declarations bring supertrait items into scope as well. However this would be a breaking change, since it has the potential to introduce ambiguity. It could however still be considered for an edition.

#### Further resolution of ambiguity
[further-resolution]: #further-resolution

As demonstrated above, there are other fragile base class problems unaddressed by this RFC. There are [some possibilities of addressing this](https://en.wikipedia.org/wiki/Multiple_inheritance#Mitigation), for example by considering the order of supertraits. However, it may be reasonable to explicitly reject resolving the ambiguity, as a resolution could mean a subtle change in behavior when a supertrait changes. It may be preferable to keep this an error instead, and ask the user to explicitly specify.
