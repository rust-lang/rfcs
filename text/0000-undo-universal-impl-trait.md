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

This function has three type variables to substitute to be completely monomorphized. However, it’s
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

You can see the duplication here and the overwhelming confusion that both the `impl Trait` will
resolve to the same type, even though they don’t have the same contract, which is impossible to
guess from the interface while it could. This is legal because we only talk about two contracts here
and the function will pick a type at the union (it must be `Add + Copy` and `Copy`) but you also
have that weird `Output` contract as well.

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
the reasons listed above.

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
