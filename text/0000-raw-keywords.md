- Feature Name: raw_keywords
- Start Date: 2021-03-05
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

<!--
One paragraph explanation of the feature.
-->

Reserve `k#keyword` in edition 2021 and beyond as a general syntax for adding keywords mid-edition instead of needing speculative reservations.

# Motivation
[motivation]: #motivation

<!--
Why are we doing this? What use cases does it support? What is the expected outcome?
-->

There were a few attempts to reserve keywords for the the 2018 edition.  Some of those proved controversial, and the language team eventually [decided](https://github.com/rust-lang/rfcs/pull/2441#issuecomment-395256368) not to accept any reservations for not-yet-approved features:

> [...] felt particularly strongly that up-front reservations are wrong and a mistake in the initial Edition proposal, basically for the reasons I've already outlined in the thread: they force up-front decisions about surface issues of features that are not yet fully proposed, let alone accepted or implemented. That just seems totally backwards and is going to keep leading to unworkable discussions. We both feel that the role of Editions here is that they can absorb any keyword-flags that have accumulated in the meantime.
>
> In all, there is certainly no consensus to merge this RFC as-is, and I think there are no objections to instead closing it, under the assumption that we'll add a keyword-flag mechanism (or something like it) as needed later.

This RFC is thus a proposal to add that general mechanism.

The other thing that was learned with the 2018 edition is that the period between editions is long enough that the normal "stability without stagnation" principle of "it can just wait for the next train" doesn't work.  Instead, it encouraged rushing to try to get things in on time, which had negative quality of life consequences for many contributors.  As such, it's important that an alternative mechanism be made available so that missing an edition train doesn't mean having to wait another 3 years -- even if that alternative has syntax that's slightly less nice until the next train.

As an additional bonus, this gives a space in which experimental syntax can be implemented on nightly without risking breakage.  In the past, this was sometimes done in conjunction with other keywords, for example `do catch { ... }` instead of just `catch { ... }` to avoid the grammar conflict with a struct initializer.  With this RFC, it could instead have been implemented as `k#catch { ... }` directly without worry.


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

<!--
Explain the proposal as if it was already included in the language and you were teaching it to another Rust programmer. That generally means:

- Introducing new named concepts.
- Explaining the feature largely in terms of examples.
- Explaining how Rust programmers should *think* about the feature, and how it should impact the way they use Rust. It should explain the impact as concretely as possible.
- If applicable, provide sample error messages, deprecation warnings, or migration guidance.
- If applicable, describe the differences between teaching this to existing Rust programmers and new Rust programmers.

For implementation-oriented RFCs (e.g. for compiler internals), this section should focus on how compiler contributors should think about the change, and give examples of its concrete impact. For policy RFCs, this section should provide an example-driven introduction to the policy, and explain its impact in concrete terms.
-->

*Pretend the year is 2023 and Rust has just stabilized `trust_me { ... }` blocks as a clearer syntax for `unsafe { ... }` blocks.  The blog post in which they stabilize might say something like this.*

This release stabilizes "trust me" blocks!  Newcomers to rust are often confused by the difference between `unsafe` functions and `unsafe` blocks, as they do very different things.  So these do a better job of emphasizing that these blocks are the place in which you can call unsafe code.

Because of Rust's commitment to its stability guarantees, these are available to edition 2021 code using the syntax `k#trust_me { ... do unsafe things here ... }` to avoid breaking hypothetical code using `trust_me` as a function/type/etc name.  In another year when the next edition comes out on its usual train, `trust_me` will be a reserved keyword in it and the edition migration will remove the `k#` for you.  But for now you'll need to keep it.

*(This RFC is, of course, not actually proposing "trust me" blocks.)*

## What code could I have written that this breaks?

`k#keyword` is never valid rust code on its own, so this is only relevant inside calls to macros, where it will affect tokenization.

For example, consider [this code](https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=f50aea0afcd1f65896335b6aa5cae88a) in the 2018 edition:
```rust
macro_rules! demo {
    ( $x:tt ) => { "one" };
    ( $a:tt $b:tt $c:tt ) => { "three" };
}

fn main() {
    dbg!(demo!(k#keyword));
    dbg!(demo!(r#keyword));
    dbg!(demo!(k#struct));
    dbg!(demo!(r#struct));
    dbg!(demo!(k #struct));
    dbg!(demo!(r #struct));
}
```

It produces the following output:
```text
[src/main.rs:7] demo!(k # keyword) = "three"
[src/main.rs:8] demo!(r#keyword) = "one"
[src/main.rs:9] demo!(k # struct) = "three"
[src/main.rs:10] demo!(r#struct) = "one"
[src/main.rs:11] demo!(k # struct) = "three"
[src/main.rs:12] demo!(r # struct) = "three"
```

In the 2021 edition and beyond it will instead be
```text
[src/main.rs:7] demo!(k#keyword) = "one"
[src/main.rs:8] demo!(r#keyword) = "one"
[src/main.rs:9] demo!(k#struct) = "one"
[src/main.rs:10] demo!(r#struct) = "one"
[src/main.rs:11] demo!(k # struct) = "three"
[src/main.rs:12] demo!(r # struct) = "three"
```

So it will only affect you if you're making calls with all three of those tokens *directly* adjacent.  The edition pre-migration fix will update such calls to add spaces around the `#` such that the called macro will continue to see three tokens.

## How do I implement a feature that needs a new keyword?

For a feature using a new keyword `foo`, follow these steps:

1. Implement it in nightly as `k#foo`, ensuring that all uses of `k#foo` are feature-gated in the parsing code.
2. Test and debug the feature as you would any other feature.
3. Pause here until ready to stabilize.
4. Add an edition pre-migration fix to replace all uses of `foo` with `r#foo`.
5. Make it parse as both `foo` and `k#foo` in edition vNext.
6. Add an edition post-migration fix to replace all uses of `k#foo` with `foo`.
7. Be sure to reference the test for those steps in the stabilization report for FCP.


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

<!--
This is the technical portion of the RFC. Explain the design in sufficient detail that:

- Its interaction with other features is clear.
- It is reasonably clear how the feature would be implemented.
- Corner cases are dissected by example.

The section should return to the examples given in the previous section, and explain more fully how the detailed proposal makes those examples work.
-->

A new tokenizer rule is introduced:

> RAW_KEYWORD : `k#` IDENTIFIER_OR_KEYWORD

Unlike RAW_IDENTIFIER, this doesn't need the `crate`/`self`/`super`/`Self` exclusions, as those are all keywords anyway.

Analogously to [raw identifiers](https://rust-lang.github.io/rfcs/2151-raw-identifiers.html#reference-level-explanation),
raw keywords are always interpreted as keywords and never as plain identifiers, regardless of context. They are also treated equivalent to a keyword that wasn't raw.

For contextual keywords, that mean that a raw keyword is only accepted where it's being used as a keyword, not as an identifier.  For example, `k#union Foo { x: i32, y: u32 }` is valid, but `fn k#union() {}` is not.

In a rust version where `k#pineapple` is not a known keyword, it causes a tokenization error.  (Like using [`r#$pineapple` does today](https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=04f2f8d52487b03c93e2caa00446594e), and like how [`r#pineapple` did before raw identifiers were a thing](https://rust.godbolt.org/z/eeGvzMq8r).)

## Edition migration support

The pre-migration fix will look for the tokens "`k` `#` ident" in a macro call without whitespace between either pair, and will add a single space on either side of the `#`.

## Support for past editions

A new tokenizer rule is introduced:

> RAW_KEYWORD : `r#$` IDENTIFIER_OR_KEYWORD

This is supported for use in 2015 and 2018, as well as in 2021 for edition migration purposes.  In 2024 and beyond, this will no longer be supported.

However, it's strongly recommended that everyone migrate to a current edition rather than use `r#$`.  For example, code wanting to use `async.await` should just move to the 2018 edition, not use `.r#$await`.

Semantically, it will do the same as the equivalent `k#`, just with different syntax.

There is a warn-by-default lint against using `r#$pineapple` in 2021, which will be included as a post-migration `--fix` lint, so that code using `foo.r$#await` in 2018 will be changed to using `foo.k#await` in 2021.


# Drawbacks
[drawbacks]: #drawbacks

<!--
Why should we *not* do this?
-->

- This adds more ways of writing the same thing.
- This makes macro token rules even more complicated than they already were.
- This only works for keywords that will match the existing IDENTIFIER_OR_KEYWORD category.
- This is more complicated than just telling people to wait for the next edition.
- This cannot be done in the 2015 and 2018 editions, with the proposed regex.


# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

<!--
- Why is this design the best in the space of possible designs?
- What other designs have been considered and what is the rationale for not choosing them?
- What is the impact of not doing this?
-->

There are a few fundamental differences between raw keywords and raw identifiers:

- **It was important that old editions support raw identifiers, but old editions do not need to support raw keywords.** \
  Raw identifiers in 2015 were needed so that pre-migration fixes could be applied to rename `async` -> `r#async` separately from updating the edition number.  There's no 2015 nor 2018 edition code that *needs* raw keywords, however.  [Editions are meant to be adopted](https://github.com/nikomatsakis/rfcs/blob/edition-2021-or-bust/text/0000-edition-2021.md#editions-are-meant-to-be-adopted), so it's fine to expect actively-developed code that wants to write (necessarily) *new* code using new features to move to a new edition in order to do so.

- **Raw identifiers can be forced on you by another crate, but raw keywords are up to you.** \
  If a crate you're using has a method named `r#crate`, then you're stuck using a raw identifier to call it (unless you fork the crate).  But nothing going on in an external crate can force you to use a feature that needs a raw keyword.  If you want to only use things once they're available in the new edition as full keywords, you can do that.

- **We hope that code won't need raw identifiers, but expect people will use raw keywords.** \
  Part of the decision process for a new keyword involves looking at the impact it would have.  That's not to say it's a controlling factor -- we don't need to pick [a](https://en.cppreference.com/w/cpp/keyword/co_await) suboptimal keyword just to avoid breakage -- but the goal is that is that it not create a pervasive issue.  Whereas accepting a new feature implies that it's useful enough that many people will likely wish to use it immediately, despite the extra lexical wart.

In concert, these push for a particular tradeoff:

> **It's better for raw keywords to be nice on 2021 than for them to be consistent with 2015**

Arguably they never *should* be used in 2015 (or even in 2018, since there are no features planned to use this before 2021 stabilizes), as it's always better to move to the newest-available edition before adopting new features, but they're available with a worse syntax there for completeness. <!-- Also, the author of this RFC thinks that they shouldn't actually exist in 2015, nor in 2018, but got outvoted :( -->


# Prior art
[prior-art]: #prior-art

<!--
Discuss prior art, both the good and the bad, in relation to this proposal.
A few examples of what this can include are:

- For language, library, cargo, tools, and compiler proposals: Does this feature exist in other programming languages and what experience have their community had?
- For community proposals: Is this done by some other community and what were their experiences with it?
- For other teams: What lessons can we learn from what other communities have done here?
- Papers: Are there any published papers or great posts that discuss this? If you have some relevant papers to refer to, this can serve as a more detailed theoretical background.

This section is intended to encourage you as an author to think about the lessons from other languages, provide readers of your RFC with a fuller picture.
If there is no prior art, that is fine - your ideas are interesting to us whether they are brand new or if it is an adaptation from other languages.

Note that while precedent set by other languages is some motivation, it does not on its own motivate an RFC.
Please also take into consideration that rust sometimes intentionally diverges from common language features.
-->

This is patterned on [RFC #2151, `raw_identifiers`](https://rust-lang.github.io/rfcs/2151-raw-identifiers.html).

Some scripting languages take the opposite approach and essentially reserve all unprefixed identifiers as keywords, requiring a sigil (such as `$foo`) to have it be interpreted as an identifier in an expression.  This is clearly infeasible for rust, due to the extraordinary churn it would require.

C reserves all identifiers starting with an underscore, and uses that along with `#define` to add features.  For example, it added `_Bool`, and made that available as `bool` only when `#include <stdbool.h>` is specified.  Rust doesn't need this for types (as `i32` and friends are not keywords), but could add new syntax constructs as macros.

C# releases new versions [irregularly](https://en.wikipedia.org/wiki/C_Sharp_%28programming_language%29#Versions), major versions of which may include source-breaking changes such as new keywords.  Rust could decide to just roll editions more often instead of introducing features in the middle of them.

C# also leverages contextual keywords heavily.  For example, `await` is only a keyword inside functions using the `async` contextual keyword, so they could be introduced as non-breaking.  This kind of contextual behaviour is more awkward for rust, which needs to be able to parse an `expr` to pass it to a macro.

Python uses [*future statements*](https://docs.python.org/3/reference/simple_stmts.html#future) to allow use of the new features on a per-module basis before those feature become standard.  Rust's `#![feature(foo)]` on nightly is similar here.

Haskell has the [`LANGUAGE` pragma](https://ghc.readthedocs.io/en/8.0.2/glasgow_exts.html#language-pragma), which `ghc` also supports as command line parameters.  This is again similar to Rust's `#![feature(foo)]` on nightly.


# Unresolved questions
[unresolved-questions]: #unresolved-questions

<!--
- What parts of the design do you expect to resolve through the RFC process before this gets merged?
- What parts of the design do you expect to resolve through the implementation of this feature before stabilization?
- What related issues do you consider out of scope for this RFC that could be addressed in the future independently of the solution that comes out of this RFC?
-->

None


# Future possibilities
[future-possibilities]: #future-possibilities

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

- Since an edition fix that can do it is required anyway, it may be good to have a lint on by default that suggests removing superfluous `k#`s.

