- Feature Name: undo-universal-impl-trait
- Start Date: 2018-05-21
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

This RFC undoes [RFC 1951], which gave Rust the `impl Trait` syntax in function argument position.

# Motivation
[motivation]: #motivation

## Brief history

`impl Trait` was initially introduced in [RFC 1522]. That feature was completely new to Rust as it
added a very interesting and powerful feature: being able to provide the caller of a function with a
type contract by providing *an anonymized type* in return location of the function and by letting
the callee pick the type.

    // the caller only knows _properties_ on the returned value, it doesn’t know its real type
    fn zero_to_ten() -> impl Iterator<Item = u32>

That feature was a complete success and was quickly adopted. The syntax, `impl Trait`, was then
quickly refered as “the syntax to pick the type in the callee” and was very simple to get our
fingers around. This is a use of *existential quantification* and the single one currently available
in Rust. With the right education – and the Rust team made a wonderful job with the book – it’s easy
to teach newcomers about the feature.

Then came [RFC 1951] and [RFC 2071]. The latter generalizes the use of `impl Trait` to `let`
bindings, `const` and `static` declarations and is not of our matter here – because they also act as
*existentials*, which is great. However, the former, [RFC 1951], introduced `impl Trait` in argument
position. There were a lot of debates about the semantics that the syntax should have but
eventually, the consensus came to a decision. The following syntax:

    fn foo(x: impl Debug);

Is akin to this:

    fn foo<T: Debug>(x: T)
    fn foo<T>(x: T) where T: Debug

And so the feature was stabilized.

This RFC wants to undo this.

## Why is `impl Trait` in argument position wrong?

People out there didn’t really realize what was going on. Complains and confusions on IRC, reddit
and the Rust forum started to appear, after the stabilization, by different kind of people –
confusion among newcomers, advanced proficient Rustaceans, etc.. Here are some – non-exhaustive –
links:

  - https://internals.rust-lang.org/t/how-is-impl-display-different-from-foo-t-display-x-t-t/7566/3
  - https://www.reddit.com/r/rust/comments/8jfn7z/what_is_the_advantage_of_impl_trait_in_argument/
  - https://www.reddit.com/r/rust/comments/8io4q6/impl_trait_is_here_now_what/dyu5dlm/
  - https://github.com/rust-lang/rust/issues/44721#issuecomment-330935175

[RFC 1951] is organized with *arguments* that we’re basing the current document against.

### Argument from learnability

> Argument [here](https://github.com/rust-lang/rfcs/blob/master/text/1951-expand-impl-trait.md#argument-from-learnability)

This argument explains that people who have an understanding of generics would try to use them in
return position for anonymized types (existentials). This argument then stated it’s confusing
because the only way to do this with current Rust is to use a complete different mechanism – being
`impl Trait`. By contrast, if they have first learned `impl Trait` in argument position, trying to
use it in return position *will just work as intended*.

This argument is twisted along the lines of subjective opinion about learnability. Rust is a
language with a fabulous set of learning resources (the first and most important one being
[The Book]). It seems sound to state that **if** people are introduced first to `impl Trait` in
argument position, it’s likely that they’ll understand the use in return location. However, what if
they don’t want to express it? Existentials in return locations are already a very specific use case
of a type system. Having a complete different mechanism is actually a feature, because we are
talking about something that has no alternative elsewhere in Rust – i.e. pick the type in the
callee. The initial assumption that having a different mechanism is confusing is wrong because
`impl Trait` carries its own semantics and because anonymization in return type is not a trivial
feature people will use without actually wanting it.

Also, people are used to generics. People coming from C++, C#, Java etc. actually know about
generics. The links listed above prove how confusing it can get to show them a complete, different
way to use generics (`impl Trait` in argument position is a use of hidden generics, that is,
anonymized type variables with explicit trait bounds). For people who don’t come from such
languages or for completely newcomers, we are assuming that generics are too hard to understand at
first but people will eventually need to learn them, as `impl Trait` in argument position is
strictly less powerful than type variables and trait bounds. They will have to read the book –
perhaps several times. It then confusing to be introduced a concept that is less powerful than
generics and yet used in a complete, separate way that has no other alternative (return location).

Finally, advanced Rustaceans are also quite confused, because they’ve been using `impl Trait` in
return location for a while now (mostly on nightly) and the initial RFC was well accepted among the
community and was a success. Those developers obviously mapped `impl Trait` to *callee-picked*
types, because at that time it was its sole semantics. When a function with an argument annotated
with `impl Trait` appears to such a developer, they get confused because that would mean *an argument
which type is chosen by the callee*, which doesn’t quite make sense.

### Argument from ergonomics

> Argument [here](https://github.com/rust-lang/rfcs/blob/master/text/1951-expand-impl-trait.md#argument-from-ergonomics)

In this argument, [RFC 1951] states that having `impl Trait` in argument position eases how our
mental model about a function works. We don’t have to read the type variables and rembember their
trait bounds anymore to understand the contract of a function.

This argument seems sound but again is wrong. Consider:

```
fn foo<T>(a: T, b: T, c: impl Debug) where T: Add<Output = T> + Copy
```

This function has two type variables to substitute to be completely monomorphized. However, it’s
very hard to see it at first because one of the variables is hidden. Worse, the ergonomics argument
doesn’t stand here because you can see you **have** to read the type variables in order to get what
you can do with the variables.

Also, consider this function that takes an argument and add it to itself:

```
fn add_self<T>(x: T) -> T where T: Add<Output = T> + Copy
```

Now consider:

```
fn add_self(x: impl Add<Output = impl Copy> + Copy)
```

You can see the duplication here and the fact that both the `impl Trait` will resolve to the same
type, even though they don’t have the same contract, which is impossible to guess from the interface
while it could. This is legal because we only talk about two contracts here and the function will
pick a type at the union (it must be `Add + Copy` and `Copy`) but you also have that weird `Output`
contract as well.

Even weirder:

```
fn add_self(x: impl Add<Output = impl Sized> + Copy) -> impl Sized {
  x + x
}
```

### Argument from familiarity

> Argument [here](https://github.com/rust-lang/rfcs/blob/master/text/1951-expand-impl-trait.md#argument-from-familiarity)

This argument is quite the same as the one for ergonomics and yet has this sentence:

> In Rust, this is just a syntactic easement into a unitary polymorphism system which is
> fundamentally one idea: parametric polymorphism with trait constraints.

The example given in that statement is itself a proof that *unitary polymorphism* isn’t that common
since the `U` type variable *needs* to be explicitely written. The familiarity with Java or C# might
hold but shouldn’t drive the design decision and even more important, the consistency of the Rust
syntax.

### Summary of arguments

This document has shown that `impl Trait` in argument position is confusing for newcomers and
already advanced proficient Rustaceans. Among the issues:

  - Two syntactic schemes to express parametric polymorphism (universal):
    + `<T: Bound>` / `<T>` + `where: Bound`
    + `impl Trait`
  - `impl Trait` is strictly less powerful than type variables plus traits, so very limited.
  - Even some examples requiring only one place type variable cannot be expressed with `impl Trait`
    (`T: Add<Output = T>`) without a lot of confusion.
  - It’s very hard to tell what are the variables to monomorphize a function, especially if you mix
    both the style.
  - Confusing because people are used to `impl Trait` as existential.

This RFC was written with respectful discussions in mind. If something can be done, it can also be
undone and this RFC strongly thinks it would be better for the community to undo [RFC 1951] for all
the reasons listed above. Also, here are a few other reasons people commented on the current RFC to
strengthen it:

  - The main argument was that `impl Trait` instead of generics would make our life easier, for both
    newcomers and proficient users. The many threads created and confusion out there is a proof by
    itself that reality is way more blurry than expected.
  - `impl Trait` for argument position is non-orthogonal, which makes it harder to decide which
    syntax to use since there are now two (even three) ways. It’s very likely that even with the
    book, this question will come up over and over.
  - It’s not that easier to learn than generics, because it looks very different from the generics /
    template syntax programmers are already familiar with (C++, C#, Java, D, etc.).
  - Universal and existential quantification shouldn’t be conflate. Being aware of the difference
    is a plus for someone to learn Rust. `impl Trait` should only be used for the existentially
    quantified variable and `<T: Trait> / where T: Trait` should be used for universally quantified
    variables. The `impl Trait` should stay in the return location and `let`, `const` and `static`
    bindings.
  - Because Rust is still a hard language to learn, newcomers will still have to read the book
    several times or maybe spend some time practicing, at least. The argument stating *“having
    `impl Trait` for argument position allows postponing introducing generics until later in the
    book”* doesn’t really make much sense then.
  - For *most* newcomers, as they come from a mainstream / popular language, they are already used
    to the angle bracket notation, which is misleading if you consider the `impl Trait` syntax
    *plus* the angle bracket notation.
  - People seem confused with the three syntaxes.
  - What makes Rust hard to learn is not universal quantification. It’s more about the borrow
    checker and linear / affine type system. The raw and bare concept of universal quantification is
    actually pretty simple to wrap your fingers around.
  - The argument of symmetry (having `impl Trait` in argument position mirroring the `impl Trait` in
    return position) doesn’t seem to account for history: we’ve been using Rust without `impl Trait`
    in argument position for years now and no one has ever felt the need to mirror existential
    quantification with universal quantification in argument position.
  - People can already write a lot of code without even needing polymorphic code / universal
    quantification, especially newcomers. Education / teching about type variables and universal
    quantification can be postponed to the end of the book if the authors are afraid it’s too hard
    for newcomers.
  - It seems like proficient developers have no reason to use `impl Trait` in argument position, so
    why encourage newcomers to do so? What is the **real value** added by such a feature?
  - About the [dialectical ratchet], it may apply to references / lifetimes but **not** to
    `impl Trait` for argument position. The concepts of references and lifetimes complement each
    other, they are used together, not only by newcomers. **These are orthogonal concepts**. Having
    newcomers first learn `impl Trait` for arguments to express universally quantified generics
    means that they still will have to unlearn or add a learning exception later when they get
    introduced to type variables, which might confuse them because of two overlapping distinct
    constructs to actually express the same thing – plus the former is strictly less powerful. It’s
    a bit akin to `<T: Trait>` vs. `<T> where T: Trait`. You’re firstly introduced to the former
    then understand the second one is more powerful (because you cannot express bounds on `Self`
    with the former). So people actualy use `where` clauses instead of the former, and they’re
    right. Why would you bother learning two ways to express the same thing when one of them is the
    the other augmented with a bit more power – bounds on `Self`?
  - The Rust language is already a very, very complex language with a lot of concepts people **will
    have to learn**, should it be through the book or any other tutorials. Making Rust easy to
    **use** and easy to **remember** is as important as making it easy to learn and making so is via
    the documentation, tutorials and the book, not via the language design itself. We can improve
    the documentation about generics without affecting the current design.
  - Overlapping concepts harm the consistency and orthoganality of a language. Having several ways
    to do something forces people to know all the ways – even if they always prefer using one –
    because they will maintain and contribute to codebases they haven’t read nor written code for
    before.
  - Turbofishing is impossible with `impl Trait`, forcing you to use type ascription in other
    places.
  - The initial RFC and especially the [dialectical ratchet] seem to state that people do the same
    thing in Java in C#. They don’t. In those languages, the functions using such similar syntax
    are completely monomorphic (dynamic dispatch). The equivalent in Rust would be the trait object
    or `dyn Trait` syntax.
  - [This is a proposition]: in the future, we could do some A/B testing to determine
    which change to Rust makes it easier to learn prior to stabilization. Also, when features are
    introduced that are controversial or unorthogonal (e.g. this or the matching auto ref), the
    debate thread should be more visible and open for longer. A lot of people only noticed about
    [RFC 1951] a few days / weeks ago, when the feature was already stabilized (or about to be)
    because it was hiding in a GitHub thread. Maybe we should have official communication or maybe
    a dedicated section in the [TWiR]?

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

  - For newcomers, nothing to teach since we’re removing a syntax construct.
  - For others, the book must be updated with a deprecated section and explain why the feature was
    removed if we reach a consensus.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

If though the syntax is removed, `rustc` must still be able to recognize it so that it can lead
people to type variables and trait / where clauses.

# Drawbacks
[drawbacks]: #drawbacks

The main drawback is that people are already starting to write Rust code with this new feature. It
will then have an impact on the existing codebase. However, as shown in the current document, moving
away from `impl Trait` in argument position is straight-forward and lossless.

It also introduces a breaking change that would invalidate both internal and public code. This is a
serious issue to take into account.

# Rationale and alternatives
[alternatives]: #alternatives

# Prior art
[prior-art]: #prior-art

Haskell is a good candidate to base this RFC against. Rust trait bounds can be seen as Haskell
typeclass constraints in functions. For instance:

```
-- a function that takes a value which type is an instance of the Num typeclass (i.e. you can add,
-- multiply, etc.) and that adds this value to itself
addSelf :: (Num a) => a -> a
addSelf a = a + a
```

The following:

```
addSelf :: Num a -> Num a
```

Would violate the type checker because a function expects *types* but the implementation uses
*constraints*. The `ghc` error is:

```
…6:12: error:
    • Expected a type, but ‘Num a’ has kind ‘Constraint’
    • In the type signature: addSelf :: Num a -> Num a
```

Haskell lists the constraints an expression is limited with in the head part of its declaration (the
part in between `()` in the above example). This makes it easy to see the contract of the function,
as it’s easy to read the same in Rust with `where` clauses.

# Unresolved questions
[unresolved]: #unresolved-questions

[RFC 1522]: https://github.com/rust-lang/rfcs/blob/master/text/1522-conservative-impl-trait.md
[RFC 1951]: https://github.com/phaazon/rfcs/blob/undo-universal-impl-trait/text/1951-expand-impl-trait.md
[RFC 2071]: https://github.com/rust-lang/rfcs/blob/master/text/2071-impl-trait-type-alias.md
[The Book]: https://doc.rust-lang.org/book/second-edition/index.html
[dialectical ratchet]: https://github.com/rust-lang/rfcs/pull/2071#issuecomment-329026602
[TWiR]: https://this-week-in-rust.org
