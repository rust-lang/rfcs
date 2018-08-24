- Feature Name: `hidden_impls`
- Start Date: 2018-06-24
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

Allow a visibility modifier on a `trait` implementation such that:

+ it is public wrt. coherence and overlap
+ it is not usable in contexts where it is *hidden* (not visible)

An example:

```rust
#[derive(Clone)]
struct Foo(..);

crate impl Copy for Foo {} // Can only be used in the current crate!
```

# Motivation
[motivation]: #motivation

First, let's assume that we have a trait `Property` defined somewhere.
This trait, which doesn't have to be inside your crate, is also visible
outside of it. It also happens that your crate has defined and exported
a type called `Thing`. The specifics of `Property` and `Thing` are not
interesting for our purposes, but they exist.

You now want to, define an implementation of `Property` for `Thing`.
But for some reason, you are not willing, or able, to expose this implementation
outside your crate and make it part of your semantic versioning guarantees.

An example of such a scenario is defining a new-type wrapper `TestRng` around
`XorShiftRng` of the `rand` crate. We want to define an implementation of
`SeedableRng` for `TestRng` to take advantage of the logic of `FromEntropy`
which is automatically provided for us when `Rng: SeedableRng`.
But we also want to avoid exposing `SeedableRng` as a guarantee to the
users of our library (`proptest`) right now. The allure of getting the logic
provided when defining `SeedableRng` could also have caused us to prematurely
and mistakenly provide a guarantee we were not ready to provide.

[proptest#80]: https://github.com/AltSysrq/proptest/issues/80
[regex#457]: https://github.com/rust-lang/regex/issues/457

A further example from `proptest` is when the author of this RFC
[mistakenly][proptest#80] exposed `impl From<SizeRange> for SizeRange`
publicly while the API was only needed internally.
Another occasion where a similar scenario occurred was when
[regex@0.2][regex#457] accidentally had a public dependency
on `regex_syntax` due to yet another a `From` implementation.

Another example use-case which [@nikomatsakis](https://github.com/nikomatsakis)
has provided is wanting to have the convenience of `#[derive(Default)]`,
but not wanting to commit to there being a default value.

However, we have no way eat our cake and keep it. We can't segregate who
has access to what implementations by visibility.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## The basic idea

The problem set out by the [motivation] is solved by allowing you to
hide away implementations which you, for whatever reason, are not willing or
able to expose to users of your library. This RFC solves that with a simple
solution: *visibility modifiers on implementations.*

Instead of writing:

```rust
// crate A:

impl Property for Thing { .. }
```

we can now write:

```rust
// crate A:

crate impl Property for Thing { .. }
```

where `crate`, or `pub(crate)`, is the familiar visibility
modifier for the current crate.

## The effect of hiding

### Error E0277

The effect `crate` has on `Property for Thing` is that when you `use Property;`
outside of the current crate, and you are working with a `Thing`,
then the compiler will pretend that `Property for Thing` does not exist.

Assuming we have a function that creates a `Thing`:

```rust
// some crate:

fn make_a_thing() -> Thing { .. }
```

and functions that accept things that implement `Property`:

```rust
// some crate:

fn use_property_1<P: Property>(..) { .. }
fn use_property_2(arg: impl Property) { .. }
```

Concretely, this means that all of these usages will be rejected by the compiler:

```rust
// crate B:

fn main() {
    use_property_1::<Thing>(..);             // ← error[E0277]
    use_property_2(make_a_thing());          // ← error[E0277]

    let x: <Thing as Property>::Projection;  // ← error[E0277]
    let y = <Thing as Property>::MY_CONST;   // ← error[E0277]
    let z = <Thing as Property>::function(); // ← error[E0277]
    ...
}

fn return_property() -> impl Property {
    make_a_thing() // ← error[E0277]
}
```

[`E0277`]: https://doc.rust-lang.org/stable/error-index.html#E0277

in all of these cases, the error [`E0277`] will be emitted. A possible variant
on `E0277` that highlights the particulars of the problem could be:

```rust
error[E0277]: the trait bound `Thing: Property` is not satisfied
 --> src/main.rs:<line>:<column>
  |
L |     use_property_1::<Thing>();
  |     ^^^^^^^^^^^^^^^^^^^^^^^ the trait `Property` is implemented for `Thing`,
  |                             but the implementation is hidden.
  |
note: required by `use_property_1`
```

### Method calls and E0599

Let's assume that `Property` is in scope, and that we've written:

```rust
// crate B:

fn main() {
    let thing = make_a_thing();
    thing.method();
}
```

Let's also assume that there's no other `method` defined in some inherent
or trait method for `Thing`. In that case, error `E0599` will be emitted,
with a twist:

```rust
error[E0599]: no method named `method` found for type `Thing` in the current scope
  --> src/main.rs:15:7
   |
L  | struct Thing;
   | ------------- method `method` not found for this
...
L  |     thing.method();
   |           ^^^^^^
   |
   = help: items from traits can only be used if the trait is implemented and in scope
   = note: an implementation of `Property`, which defines an item `method`,
           for `Thing` exists, but is hidden
```

If instead there does exist some trait implemented for `Thing` which does
provide the item `method`, then the `hidden_function` lint, adapted with
language appropriate for methods, will be triggered.

## Lint: `hidden_fn` ⇒ Use UFCS

Consider for a moment that we've written:

```rust
// crate B:

trait Foo {
    fn function();
}

fn main() {
    use crate_a::Property; // ← assume that `Property::function` exists.

    let x = Thing::function();
}
```

Here we want to avoid the future possible breakage of `Property for Thing`
eventually becoming public. Since the compiler is aware of `Property for Thing`
and that we've brought `Property` into scope, a warning will be emitted
in addition to the `unused import` lint, suggesting that you should use
*uniform function call syntax (UFCS)* instead:

```rust
warning: the trait bound `Thing: Property` is not satisfied
 --> src/main.rs:<line>:<column>
  |
L |     let x = Thing::function();
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^^ a hidden implementation of trait `Property`,
  |                                which has a function `function`, exists for `Thing`.
  |
  | help: rewrite using UFCS:
  |
  |     let x = <Thing as Foo>::function();
  |
  = note: #[warn(hidden_fn)] on by default
```

## Hidden, but coherent

It is crucial to note that `Property for Thing` acts, with respect to coherence,
as if it were public. Consider that a public trait `Property` is defined in
crate A and that a blanket implementation exists in crate A:

```rust
// crate A:

pub trait Property<T> { ... }

crate impl<T> Property<T> for T { ... } // `From` provides a blanket impl like this.
```

That means that crate B, which is a reverse dependency of crate A, may not write:

```rust
// crate B:

struct Thing { ... } // or `union` / `enum`

impl Property<Thing> for Thing { ... }
```

This implementation is rejected, by the compiler, as overlapping with the one
specified in crate A. The following is one *possible* error message,
which is a variant of `E0119`, tailored for this specific case:

```rust
error[E0119]: conflicting implementations of trait `crate_A::Property` for type `Thing`:
 --> src/main.rs:<line>:<column>
  |
L | impl Property<Thing> for Thing {
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  |
  = note: conflicting implementation in crate `A`:
          - crate impl<T> crate_A::Property<T> for T { ... }
    note: the implementation exists, but is hidden.
```

[unsound-named]: https://internals.rust-lang.org/t/looking-for-rfc-coauthors-on-named-impls/6275/2

If the compiler did not reject this implementation, it would amount to the same
problem as with named implementations, which would be decidedly unsound in Rust
since [*"coherence is an invariant that unsafe code is allowed to assume"*][unsound-named].

## Hiding things from yourself

Thus far, we've only used `crate impl ..` to hide implementations.
While we think that will be the most likely usage, we also believe
that more fine grained control is beneficial for larger projects
with many people involved in the development of those projects.
Therefore, this RFC proposes that all visibility modifiers (such as
`pub(super)`) be permitted on `impl` to hide implementations from
different parts of the crate.

## `pub` by default

Given that you are now allowed to put any visibility modifier on an `impl`,
you might wonder if not specifying one is the same as private visibility
(module visibility). This RFC specifies that it is not. Instead, default
visibility is contextual. This means that `impl Foo for Bar { .. }` is
equivalent to `pub impl Foo for Bar { .. }`. This is right for two reasons:

+ Changing the default visibility to module private would constitute a major
  breaking change which would be too large to countenance even with an edition.

+ `pub` as a default for implementations is a *good* default, since it is what
  you want most of the time.

Having `pub` be permitted on `impl` also means that you can use the `vis`
macro fragment specifier.

To promote uniformity in the ecosystem, directly writing `pub impl ..` will
be linted against with a lint named `redundant_vis` (or a naming in that spirit).
A sample warning is:

```rust
warning: `pub` is redundant on `impl`
 --> src/main.rs:<line>:<column>
  |
L |     pub impl Foo for Bar { .. }
  |     ^^^ help: remove `pub`
  |
  = note: #[warn(redundant_vis)] on by default
```

## Private visibility, `pub(self)`

[RFC 1442]: https://github.com/nox/rust-rfcs/blob/master/text/1422-pub-restricted.md

As we previously discussed, writing just `impl Foo for Bar { .. }`
does not entail that `Foo for Bar` is only visible in the current module.
So how do we achieve that effect? It just so happens that the visibility
`pub(self)`, which makes something (only) visible in the current module,
already exists in Rust (since 1.18, introduced by [RFC 1422]).

The visibility modifier `pub(self)` is the default for all contexts but `impl`.
As with writing `pub(self)`, if you directly write out the default visibility
such as in:

```rust
struct Foo {
    pub(self) x: Bar,
}
```

then the lint `redundant_vis` will be raised.

## `#[derive(crate Trait)]`

To make it simpler for you to provide implementation of common traits
which can be derived, you are permitted to place a visibility modifier,
such as `crate`, before `Trait` in the `#[derive(..)]` attribute on a
type definition. An example:

```rust
#[derive(Default, crate Copy, crate Clone)]
struct Foo {
    ...
}
```

In the case of derivable standard library traits, the compiler will insert the
visibility modifier before `impl` in the implementations generated.
This *also* applies to custom derive macros. After a custom derive macro
has generated the `TokenStream` for the `#[derive(crate CustomTrait)]` invocation,
then the compiler will ensure that nothing generated in the `TokenStream`
is more visible than `crate`. It will do so by clamping a `pub` visibility
to `crate`. This applies generally for any visibilities.

## In relation to trait objects

One place where crate B is allowed to use a hidden implementation of crate A
is when it comes to trait objects. This is fine for a few reasons:

+ It is useful to pass around trait objects of `Property` inside crate A
  internally even such objects arise from hidden implementations
  so we don't want to ban making trait object of hidden implementations
  outright.

+ It is also useful to pass trait objects from crate A to crate B
  because crate A is under no obligation, under semver rules,
  to pass an object of type `Thing` in a new minor / patch version.
  As long as operations afforded by `Property` for the trait object,
  that crate B gets, behaves observably in an equivalent manner,
  for some semantic definition of "equivalent", no breakage has occurred.

+ If we consider passing hidden implementations to crate B to be "bad"
  and passing public implementations to crate B to be "good",
  then it is impossible to reject all bad programs (*soundness*)
  but allow all good programs (*completeness*).
  To see why, let's first assume that `Property` is object safe.
  Let's also assume that the following definitions exist in crate A:

  ```rust
  struct Thing;
  struct OtherThing;
  
  crate impl Property for Thing { ... }
  pub impl Property for OtherThing { ... }

  fn halting_problem() -> bool { ... }

  pub fn make_with_property() -> Box<dyn Property> {
      if halting_problem() {
          Box::new(Thing)
      } else {
          Box::new(OtherThing)
      }
  }
  ```

  [undecidable]: https://en.wikipedia.org/wiki/Undecidable_problem

  Since we know that [the halting problem is undecidable][undecidable],
  and whether we return `Box::new(Thing)` or `Box::new(OtherThing)` is
  dependent on the halting problem, then it is also undecidable whether
  a hidden implementation is leaked or not via a trait object.
  Thus, static analysis can't be used unless we accept that some good programs
  be rejected by the compiler.

  If we want to devise such an analysis, we could do so by starting at
  the root of each `pub` function and transitively check whether it,
  or any of its dependencies, will possibly construct a pointer to the
  vtable of any hidden implementations. This could be done as a lint
  of some form, but is currently left as possible future work.

## Recommendation: When you should use hidden implementations

Simply put, it is recommended that your implementation be hidden if you are
not yet ready to guarantee that the implementation exists to users of your
API, be they internal APIs within a project, or the public APIs of a crate.
In most cases, you implementations will not need to be hidden.

## Teaching beginners about hidden implementations

The contents of this RFC are mostly considered to be *advanced* features of Rust.
However, it should mostly be sufficient to mention to beginners in sections about
trait implementations that they can hidden, with one or two simple examples.
The still more advanced details in this RFC can be deferred to the reference.

One particular aspect of this RFC is important to teach: The coherent nature
of hidden implementations. This should be stressed, and in particular,
it should be noted that these do *not* act as the modular implementations
discussed in the [prior art][prior-art]. The naming of these implementations
as *hidden*, as opposed to *private* is intentional. The word *private*
should therefore be *avoided* in learning material.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Grammar and parsing

### Visibility modifier `pub(self)`

+ `pub(self)` is considered the default visibility for all contexts but `impl`.

### Visibility modifiers on `impl`

+ The grammar of `item_impl` is amended such that `visibility` is optionally
permitted on *trait implementations*. `visibility` is not allowed before
inherent implementations.

+ `pub` is considered the default visibility for `impl`.

#### `unsafe` and visibility

+ When specifying `unsafe` on an `impl`, if a visibility modifier is specified,
  it must come first. That is, you must write:

  ```rust
  crate unsafe impl TrustedLen for Bar { .. }
  ```

  but never:

  ```rust
  unsafe crate impl TrustedLen for Bar { .. }
  ```

#### Linting on default visibility

+ If the *default visibility* for some *context* is specified in that context
  *directly* in a source file, then a warn-by-default lint `redundant_vis`
  is raised.

## `#[derive(..)]`ing

+ The grammar for the `derive` attribute will permit `visibility` before
each trait.

+ When the compiler is generating code for each derivable standard library trait,
if a visibility modifier is specified before the trait in `#[derive(..)]`,
then it will also be specified on the generated `impl`.

+ The generated code by a custom derive macro `#[derive(vis? Trait)]`,
  including any other macro invocations in that macro as well as
  the transitive closure of macro invocations in those is here referred
  to as "the expansion".

  When the compiler executes a custom derive macro `#[derive(vis? Trait)]`,
  if then a visibility modifier was specified on `vis? Trait`,
  then the visibility of everything (which can have a visibility modifier)
  in "the expansion" will be clamped to `vis` such that there is nothing
  which is more visible than `vis` in "the expansion".

### On `#[structural_match]`

+ The `#[structural_match]` attribute will be changed to optionally permit
a visibility modifier with the syntax `#[structural_match($vis)]`.

+ Not specifying one is desugared to: `#[structural_match(pub)]`.

+ When generating code for `#[derive($vis_a PartialEq, $vis_b Eq)]` on a type,
a `#[structural_match($vis)]` will be generated where `$vis` is the least
visible of `$vis_{a, b}`.

+ When pattern matching, if `$vis` in `#[structural_match($vis)]` is visible
  in the given context, then the type on which `#[structural_match($vis)]` is
  specified on is considered structurally matchable. Otherwise, it is not.

## Type checking

1. The visibility modifier `pub(self)` entails that an item is only visible in the
   module in which the item is defined in. *(this is already the case in Rust)*

2. The default visibility modifier for a context is first desugared to a form
   where it is explicitly specified. That is, `impl<..> Trait<..> for Type<..>`
   is desugared to `pub impl<..> Trait<..> for Type<..>` and
   `struct X { field: Y }` is desugared to `struct X { pub(self) field: Y }`.

3. When type checking an implementation with respect to checking its coherence,
   all implementations are considered irrespective of their specified visibility.
   When overlap is detected, the compiler should specify whether the
   other implementation, which caused the overlap, was hidden or not.

4. When otherwise type checking if an implementation exists, the visibility
   modifier - which exists in the crate metadata, specified on the `impl` will
   be taken into account. If the specified visibility is not considered to be
   visible in the context where the existence of an implementation on the type
   of the trait is checked, the implementation will be considered to not exist.
   Such an implementation that doesn't exist solely due to visibility is
   classified as *hidden*.

   Note that when type checking the super trait obligations placed on
   an implementation of a trait such as `Copy`, the visibility modifier
   on the `impl` will be considered the context where the existence of
   the super trait implementation is checked. Thus, if a user writes:

   ```rust
   #[derive(crate Clone, pub Copy)]
   struct Foo;
   ```

   or equivalently:

   ```rust
   struct Foo;

   crate impl Clone for Foo { .. }

   pub impl Copy for Foo {}
   ```

   then the implementation of `Copy` is rejected, because it is not considered
   as visible as `pub`.

5. If in 4. the implementation did not exist due to being hidden,
   error messages will take that into account when informing users
   that no implementation exists.

6. Move checking will consider the visibility of `Copy` for a type when
   checking if `Copy` is implemented for that type or not.

7. If:
   + A trait `$A`, with some `fn` item named `$candidate`, is in scope in a
     scope `S`.
   + A trait `$B`, with some `fn` item named `$candidate`, is in scope in
     scope `S`.
   + There exists a visible implementation of `$A` for a type `$T`.
   + There exists a hidden implementation of `$B` for type `$T`.
   + In the scope `S` fn `$candidate` is called either as a free function
     (without specifying the trait with UFCS) or method call syntax.
   
   Then:
   + A warn-by-default lint `hidden_fn` will be emitted at the call site
     suggesting that the user use UFCS `<$T as $A>::$candidate(args..)`
     instead.

8. When type checking a *negative implementation* of an `auto`-trait, i.e:

   ```rust
   pub(self) impl !Send for MyUnsafeType {}
   ```

   if the visibility of the negative implementation is less than that of the
   `auto`-trait, then the implementation is rejected.
   Conversely, if the visibility of the `impl` is `>=` that of the trait,
   then that will not cause the implementation to be rejected.

   The visibility specified on a negative implementation has no impact on
   type checking if an implementation exists (4.). This is sound because
   we've ruled out a situation where a negative implementation is less
   visible than the trait it negatively implements. This is an intentionally
   conservative restriction which may or may not be lifted in the future.

9. Error [`E0449`](https://doc.rust-lang.org/error-index.html#E0449) will
   no longer be emitted when a visibility modifier is applied on a `trait`
   implementation.

## Code generation

+ Code generation will ignore visibility specified on any implementations.

+ However, the visibility of implementations will be included in the crate
  metadata.

## Documentation

+ Only `pub` (whether explicitly specified or inferred) implementations are
  visible in the documentation generated by `rustdoc`.

# Drawbacks
[drawbacks]: #drawbacks

While this RFC argues that the changes proposed in this RFC are intuitive,
and uniformly specified, and will therefore not have a severe impact on
learning and the complexity budget, it should be recognized that the additions
to the language in the RFC are non-trivial. These should be judged in comparison
to the benefits provided by the changes.

There is also the *possibility* that visibility modifiers on implementations
will be mistaken for modular instances as laid out in the [prior art][prior-art].
However, such misconceptions should be eventually cleared up by the error
messages which are generated when incoherent setups using hidden implementations
are attempted. For those who understand the coherent nature of traits and
their implementations in Rust, there will be no change, thus, there is no
misconception to clear up.

Another possible drawback is the now contextual nature of visibility
when it is implicit rather than specified in the source code.
With this proposal, the default for implementations is `pub`,
but for everywhere else it is `pub(self)`. What we could do is make `pub(self)`
the implicit default on implementations as well, but this would break every
crate in existence because all public implementations would become private.
It is allowed to do this with an edition and so we could do it in 2021.
However, just because we can make edition breaking changes does not mean that
we should. For now, this RFC specifies a lint away from `pub impl..`.
If it turns out to be a problem in practice that `impl..` is thought of as
private, we can lint in the other direction favoring `pub impl..` over `impl..`.
This step should however be taken when we have some practical experience.

# Rationale and alternatives
[alternatives]: #rationale-and-alternatives

In this section, we discuss some of the design choices made in this RFC
and what some alternatives to those are.

## Lexical order: `vis? unsafe? impl` or `unsafe? vis? impl`

Let's first deal with a softball. We have to decide whether one should write
`unsafe crate impl`, or `crate unsafe impl`, or whether both should be legal.
Since it is only legal to write:

```rust
pub unsafe trait Foo { .. }
```

but illegal to write:

```rust
unsafe pub trait Foo { .. }
```

the only consistent alternative is to only permit the user to
write `crate unsafe impl`.

## Should you be able to implement a trait parallel to a hidden `impl`?

Irrespective of one's view of whether this would be beneficial,
it is unworkable to permit this. As previously discussed, doing so would
[violate][unsound-named] the invariant that `unsafe` code may assume coherence.

## Should `crate` be the only visibility modifier on `impl`?

This RFC argues no. While it does make the extent of changes smaller and while
`crate impl` is the most likely scenario, there can be cases of large projects
where more fine tuned privacy is beneficial, as discussed in the [motivation].
Furthermore, we argue that introducing an artificial limit on the usefulness of
this feature does not aid learning of Rust. In fact, we consider a less of
uniformity to have a negative impact on learning the language.

Other than that, being able use the `vis` macro fragment on `impl` is a boon
for macro authors who can now provide a macro invoked as `mac!(pub(super))`
which will then use `pub(super)` on everything you can specify visibility on.

## On permitting visibility in `#[derive(..)]`

Sometimes, you might want to make a type `Copy`, but only for internal usage.
Other times, you could want to compare types for equality, but not expose that
ability as public API. Rust makes it possible to derive implementations for
some of these standard library traits. If we want to extend that ability to
hidden implementations, which we argue is useful, then some way to specify
the visibility desired when deriving will be necessary.

Furthermore, if `#[structural_match]` should be controllable wrt. visibility,
then there is no choice but to provide a way to control visibility in
`#[derive(..)]` since you need to `#[derive(PartialEq, Eq)]` for the compiler
to specify `#[structural_match]` for you.

As syntax goes, `#[derive($vis Trait)]` is chosen as a simple but intuitive
syntax. One could consider applying visibility to a group of crates,
but it does not seem worth it to make this much terser.

When it comes to custom deriving, this RFC specifies that nothing generated
by the custom derive macro (including by invocations of macros inside
that macro, and transitively) will be more visible than the `vis` specified
in `#[derive($vis Trait)]`. We argue that this is a good thing to do because
it removes a large implementation burden on custom derive authors as well as
allowing users a uniform experience in the ecosystem. By clamping the visibility,
we may rule out some unhygenic things that users may want to do, but the ability
to write the macro generated code always remains. We could potentially have
some mechanism to opt-out of the hygiene at the macro definition site,
but this should await users actually needing that feature and be left as
future work.

## Alternative: Newtypes

The principal alternative to this RFC is to continue using the
manual workaround mechanism of newtypes around our types and providing
implementations only for the former. An example of this is:

```rust
pub struct MyOriginalType { ... }

pub trait Property { ... }

struct MyNewType(MyOriginalType);

impl Property for MyNewType { ... }
```

[RFC 2393]: https://github.com/rust-lang/rfcs/pull/2393

We then use `MyNewType` inside of our crate but don't expose the type, and thus
the implementation, to anyone who shouldn't get access to the implementation
of `Property` for `MyNewType`.

This mechanism usually works, but incurs
significant boilerplate. This is especially true if `Property` has any
super-trait bounds or has a lot of required items in need of specification.
It doesn't take a trait more complex than `Clone` or `Iterator` for there
to be, comparatively speaking, a lot of overhead.

One way of reducing the boilerplate is with `#[derive(..)]` on the newtype or
with some sort of generalized newtype deriving or "delegation" (as proposed in
[RFC 2393]). However, delegation does not extend well to all forms of traits.
And even if delegation or deriving does work, there is still overhead.

This syntactic overhead, whether large (as with manual implementations),
or medium (as with delegation or deriving) exists both for the writer *and*
the reader. In comparison, allowing the user to specify `crate impl ...`
is both a more direct approach, which indicates the semantic intent clearly,
while also being superior for reading, writing, and maintaining.

Assuming that you already know about this newtype pattern,
which you may very well not do as a beginner,
the only comparative advantage of newtypes is that it does not
require learning any new language features at all.
However, as we've previously argued, permitting visibility modifiers
on `impl` only takes existing concepts from other parts of the language
and applies them to a new place. Thus, the increased learning burden
should not be large. For anyone who already understands visibility
modifiers on anything else in Rust, seeing them for the first time
should also be relatively intuitive.

# Prior art
[prior-art]: #prior-art

[guarantees coherence]: http://blog.ezyang.com/2014/07/type-classes-confluence-coherence-global-uniqueness/

As Rust's trait system (and thus implementations on types thereof) has its
direct descendant in Haskell, we consider how that language deals with
the relation between modules and type class instances (same as `impl` in Rust).
While Haskell - without certain extensions enabled, like Rust,
[guarantees coherence] of type class instances, it does not guarantee
global uniqueness of instances. Haskell will allow you to define overlapping
instances but won't allow you to actually use them - producing an error once
you try to do that.

[optional_doi]: https://link.springer.com/chapter/10.1007/978-3-319-45279-1_9
[optional_pdf]: http://homepages.dcc.ufmg.br/~camarao/CT/optional-type-classes.pdf

In a paper [Optional Type Classes for Haskell][optional_pdf] due to
Ribero et. al [(DOI: 10.1007/978-3-319-45279-1_9)][optional_doi]
modularization of instances is considered. This is however radically different
than what is proposed in this RFC, which does *not* propose modularization of
implementations. Fundamentally, that paper seems permit different instances
of the same type class to coexist as long as the instance to use for a given
expression is uniquely determinable. As previously discussed, such a mechanism
would not be sound in Rust. The main difference between these approaches is
that in our proposal, all implementations are considered when coherence checking
an implementation. The effect of that is to prevent different crates and modules
from defining implementations for a trait and a type when a hidden one exists.

# Future work

This section outlines some *possible* future work that we might want to consider
but which this RFC does not propose.

## `priv` visibility

As we've previously noted, the `pub(self)` visibility already exists.
That visibility modifier suffices to make trait implementations private.
However; `pub(self)` is perhaps not the most ergonomic syntax if we wish
to make it *easy* to make an implementation private.

To do that, we *could* introduce the visibility modifier `priv`.
As it happens, `priv` is a reserved keyword, but it is unused.

As we've previously specified, `pub(self)` is the default visibility
for all contexts except `impl`. If we were to introduce `priv`,
it would merely be a syntactic alias for `pub(self)` and would act,
in all respects, as `pub(self)` does.

### Is `priv` better than `pub(self)`?

As always: It depends!

+ On the one hand, `pub(self)` is more consistent with the `pub(path)` syntax.
  Since `pub(self)` already exists, it could also be argued that introducing
  another way to do the same thing is a bad choice due to the common saying that
  *"there should only be one way to do it"*.

+ On the other hand, we have diverged slightly from both the consistency angle
  and the above cited "principle" (since `pub(crate)` already exists) by
  introducing `crate` as a visibility modifier to make things more easy and
  ergonomic. It is therefore clear that Rust often does not adhere to that
  principle. If we want to *encourage* users *more* to not overpromise on facts
  about their type, then introducing `priv` helps. We also note that larger
  modules *frequently* do occur, and as such, `priv impl Foo for Bar` may
  not be too rare.

* We also argue that `priv` is fitting for the proposed usage since it matches
  well with `pub`. In our view, the keyword `priv` is also more self documenting
  and intuitive than `pub(self)` is.

### Other possible keywords

+ `mod`

  This one is interesting, ergonomic, and short,
  but ultimately we consider `priv` to be more intuitive.

### A specification for `priv`

Here we lay out a feel technical details for `priv` for future reference.

#### Grammar and parsing

+ The `PRIV` token will lex the terminal `"priv"`.

+ To the production `visibility` we add:

  ```
  visibility : PRIV | .. ;
  ```

  where `..` is the old definition of `visibility`.

+ The macro fragment specifier `vis` will accept `PRIV`.

+ `priv` is considered the default visibility for all contexts but `impl`.

#### Semantics

1. The visibility modifier `priv` will be treated as `pub(self)` in all respects
   except for error reporting, where `priv` is used instead.

# Unresolved questions
[unresolved]: #unresolved-questions

Exact wordings of error messages discussed in this RFC and their error codes,
as well as lint names and their phrasings are out of the scope of this RFC.
Designs on crate-local inherent implementations of crate-external types are,
while related, also out of scope for this RFC.

The following questions are in the scope of the RFC and should be resolved
*before* the RFC is merged:

1. Are there any strange interactions with possible plans for negative bounds?

2. Should `crate` (and `pub(crate)`) be the only visibility modifier permitted
   on `impl`?

3. Should you be able to specify a visibility modifier in `#[derive(..)]`?

   1. Should `#[structural_match]` take visibility of `impl`s into account?

   2. Should the visibility modifier be automatically added to all
      trait implementations emitted by a custom derive macro?

The following questions should be resolved at least before stabilization
(but possibly sooner):

4. What is an appropriate default lint level for `hidden_fn`?
   1. Should `hidden_fn` be promoted to `deny`-by-default?
   2. should it be `allow`-by-default?
   3. Should it perhaps be a hard error instead?

5. Should `pub impl ..` be linted against, suggesting `impl ..` instead?
   Or perhaps the reverse situation instead?

6. Should `pub(self) field: Type` and the `pub(self)` visibility modifier in
   general be linted against when it is the contextual default?

# Future work
[future work]: #future-work

## Privacy hygiene

Consider a situation with a declarative macro and method dispatch such as:

```rust
// Crate A:
// -----------------------------------------------------------------------------

pub trait Foo {
    fn method(&self) -> u8;
}

pub struct Bar {}

crate impl Foo for Bar {
    fn mehtod(&self) -> u8 { 42u8 }
}

pub macro mac() {
    println!("result: {}", Bar {}.method())
    // Or instead: <Bar as Foo>::method(&Bar {})
}

// Crate B: (depends on crate A)
// -----------------------------------------------------------------------------

fn main() {
    mac!();
//  ^^^^^^^ Error? This depends on whether we have privacy hygiene or not.
}
```

Would the call to `mac!()` inside `main` be accepted by the compiler?
The answer to this question depends on whether we have *privacy hygiene* or not.
If we do, then calling `mac!()` will use the definition site of `mac`'s
privacy context to check whether the implementation is visible or not.
In this case, `<Bar as Foo>::method` *is* visible in crate A and
so the call would be accepted. However, if we do not have privacy hygiene,
then `mac!()` would result in an error because `<Bar as Foo>::method`
is hidden in crate B.

[decl_mac_2]: https://github.com/rust-lang/rust/issues/39412

Procedural macros and `macro_rules!` macros are unhygienic,
so we would expect them to produce an error in this case.
However, for [declarative macros 2.0][decl_mac_2],
there is a design choice to be made.
Fortunately, this question remains open to us as declarative macros are unstable.
As we move towards finalizing macros 2.0, we will need to consider this question.
However, this RFC does not consider the question of whether or not we should
have privacy hygiene but leaves it instead as future work.
