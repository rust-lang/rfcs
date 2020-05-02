- Feature Name: `experimental_keywords`
- Start Date: 2020-04-30
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC introduce a new way to specify "experimental" keywords. An experimental keyword exposes a fundamental operation of the language and/or compiler _without_ giving that operation its final intended syntax. This allows for user experimentation with the capability introduced by the feature, even on Stable, while allowing the final syntax for the capability to be decided on and stabilized separately from the initial implementation.

An experimental keyword is given with `r#$keyword`, such as:
* `r#$yeet`
* `r#$assembly_block`
* `r#$raw_ref`

# Motivation
[motivation]: #motivation

It is often the case that we know we want a new ability to be available for users to try out as soon as possible, even before we know what the final syntax for that ability should look like. In the past the compiler has had slightly silly keyword combinations or special proc-macros for this purpose (`do catch` and `await!`). This RFC is a continuation of that idea, while also trying to improve the understanding to general users that the ability they're using isn't in its final form.

Previously, introducing _any_ new keyword at all had to be done only on an edition change because of the compatibility hazard. Since `r # $ token` is not currently a valid token sequence for anything at all, we can use the `r#$` prefix as a way to "namespace" the experimental keywords away from the main language.

That said, it will be possible (on a case by case basis) for an experimental keyword available in Nightly to go through the stabilization steps and then be used with the Stable branch. If this happens, the keyword's meaning and usage becomes as fixed as any other Stable part of the language.
* When the final "real" syntax for an experimental feature is stabilized (assuming that it is) the experimental keyword for that feature would continue to work, and code would continue to compile (example: `try!` became the `?` operator).
* It's also possible that the "real" usage of an experimental keyword will be decided to be a proc-macro in the standard library, rather than some bit of syntax (example: the primary way to access inline assembly is not decided on yet, but it's likely to be a proc-macro).

It is probable that usage of an experimental keyword after the final syntax has been stabilized will fire a warning that you should move to the final syntax, but this can be decided on a case by case basis for each language feature.
* Notably, experimental keywords are intended to be available in all editions, but it's possible that a particular final syntax for a feature might not be usable within all previous editions. In such a case, we would not warn against using the experimental keyword form when the crate was compiling using that older edition.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

From time to time you may see things like `r#$name` in Rust code. These are "experimental keywords".

An experimental keyword represents accessing the "raw operations" of the compiler and language. They have an intentionally funny syntax because experimental keywords are not intended to be used directly in the long term. Instead, they're a way to add features to the language before we've decided on the final syntax for that feature.

Most experimental keywords are given a final syntax intended for general, long-term use, some time after the feature itself becomes available in the Stable compiler. This way the Rust community has an opportunity to try it out and see how it feels, how they use it in practice, and so on. Some experimental keywords aren't ever given a direct syntax of their own, instead they're simply there so that proc-macros can expand to code that uses the keyword.

[And here we'd ideally we'd add at least one or two real examples to the guide once we have some examples of this in practice]

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This RFC alters the language so that `r # $ token` is an accepted token sequence.
* It names an experimental keyword named `token`.
* The experimental keyword named _must_ be known to the compiler. If you attempt to name an experimental keyword that the compiler doesn't recognize it is an error.
* The exact meaning and usage for each experimental keyword depends on the language feature it's associated with, and is not specified here.

# Drawbacks
[drawbacks]: #drawbacks

* It can slightly hurt Rust's stability story to deliberately introduce new keywords that are intended to become deprecated within a relatively short period of time.
  * Mitigation 1: Experimental keywords that do become Stable would continue to work even once the final syntax is decided upon and also stabilized. Stable code would not break.
  * Mitigation 2: The rather unusual `r#$` prefix stands out as "clearly unusual"

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

* Alternative: We could select _some other_ sequence of invalid tokens to use as a prefix for experimental keywords.
  * `r#$` is used because it is close enough to the "raw literal" and "raw string" syntaxes that people are very likely to understand what's going on even if they're not familiar with a specific experimental keyword when they first see it.

* Alternative: We could limit the experimental keyword usage to Nightly only until the final syntax is decided upon.
  * This would save people who only use the Stable channel from having to worry about using a new feature one way and then potentially being encouraged to update to a second syntax later.
  * However, in the 2019 Survey, only 30% of users responded that they use Nightly, so a very large portion of the community would end up excluded from the experimentation phase.

# Prior art
[prior-art]: #prior-art

* The Ember framework for javascript places experimental framework abilities into a special module that is clearly experimental so that users can try new things and become familiar with new features even before the feature is fully stable.
* There has been some negative experiences elsewhere with exposing things under non-final names, such as [vendor prefixes](https://developer.mozilla.org/en-US/docs/Glossary/Vendor_Prefix) in CSS or [`X-` headers](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers) in HTTP. I think it would be good for them to be mentioned here, with some details about why this won't have those problems.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None at this time.

# Future possibilities
[future-possibilities]: #future-possibilities

* It might be desirable to also make an `experiments` crate which can provide various proc-macros that use the experimental keywords.
  * We would also ensure that the `experiments` crate is available on the Rust Playground in addition to the "100 most common crates.io crates" that it normally supports. This would help users share ideas and experimental iterations without everyone having to publish their own experimental crates.
  * Alternately, the `experiments` crate could simply always be available via the sysroot (like the `proc_macro` crate).

* It is likely that `cargo-fix` and/or `rustfmt` would be able to automatically convert code using an experimental keyword into the "final syntax" form once a final syntax is decided.
  * This would greatly help the transition from experimental keyword to final syntax, but such support would be on a case by case basis.
