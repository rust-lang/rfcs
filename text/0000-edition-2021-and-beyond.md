- Feature Name: N/A
- Start Date: 2020-07-29
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary

[summary]: #summary

- Announce plans for a Rust 2021 Edition, and for a regular cadence of editions every 3 years thereafter.
  - We will roll out an edition regardless of whether there are breaking changes.
- Unlike Rust 2018, we will avoid using editions as a "deadline" to tie together high-priority projects.
  - Instead, we embrace the train model, but editions are effectively a "somewhat bigger release", giving us an opportunity to give an overview of all the work that has landed over the previous three years.
- We specify a cadence for Edition lints.
  - "Edition idiom" lints for Edition N will warn for editions before N, and become "deny by default" in Edition N.
  - Since it would be disruptive to introduce deny-by-default lints for Rust 2018 now, the Rust 2018 lints are repurposed into Rust 2021 Edition lints.
- We specify a policy on reserving keywords and other prospective changes.
  - In short, reserving keywords is allowed only as part of an active project group.

# Motivation

[motivation]: #motivation

The plan for editions was laid out in [RFC 2052] and Rust had its first edition in 2018. This effort was in many ways a success but also resulted in some difficult lessons. As part of this year's roadmap, one of the major questions we identified was that we need to decide whether we are going to do more editions and -- if so -- how we are going to manage the process.

[rfc 2052]: https://github.com/rust-lang/rfcs/blob/master/text/2052-epochs.md

This RFC proposes various clarifications to the edition process going forward:

- We will do new Rust editions on a regular, three-year cadence.
  - We will roll out an edition regardless of whether there are breaking changes.
- Unlike Rust 2018, we will avoid using editions as a "deadline" to tie together high-priority projects.
  - Instead, we embrace the train model, but editions are effectively a "somewhat bigger release", giving us an opportunity to give an overview of all the work that has landed over the previous three years.
- We specify a cadence for Edition lints.
  - "Edition idiom" lints for Edition N will warn for editions before N, and become "deny by default" in Edition N.
  - Since it would be disruptive to introduce deny-by-default lints for Rust 2018 now, the Rust 2018 lints are repurposed into Rust 2021 Edition lints.
- We specify a policy on reserving keywords and other prospective changes.
  - In short, reserving keywords is allowed only as part of an active project group.

## Expected nature of editions to come

We believe the Rust 2018 was somewhat exceptional in that it introduced changes to the module system that affected virtually every crate, even if those changes were almost completely automated. We expect that the changes introduced by most editions will be much more modest and discrete, more analogous to `async fn` (which simply introduced the `async` keyword), or the changes proposed by [RFC 2229] (which tweaks the way that closure captures work to make them more precise).

The "size" of changes to expect is important, because they help inform the best way to ship editions. Since we expect most changes to be relatively small, we would like to design a system that allows us to judge those changes individually, without having to justify an edition by having a large number of changes combined together. Moreover, we'd like to have editions happening on a predictable cadence, so that we can take that cadence into account when designing and implementing features (i.e., so that we can try to tackle changes that may require migrations earlier, to give plenty of time).

## Key ideas of edition do not change

Just as with Rust 2018, we are firmly committed to the core concepts of an edition:

- Crates using older editions continues to compile in the newer
  compiler, potentially with warnings.
- Crates using different editions can interoperate, and people can
  upgrade to newer editions on their own schedule.
- Code that compiles without a warning on Edition N should also
  compile on Edition N + 1.
- Migration between editions should generally be automated.
- Editions make "skin-deep" changes, with all editions ultimately
  compiling to a single common representation.

# Guide-level explanation

[guide-level-explanation]: #guide-level-explanation

We use this section to try and convey the story that average users will need to understand.

## What is a Rust edition?

Every three years, we introduce a new Rust Edition. These editions are named after the year in which they occur, like Rust 2015 or Rust 2018. Each crate specifies the Rust edition that it requires in its `Cargo.toml` file via a setting like `edition = "2018"`. The purpose of editions is to give us a chance to introduce "opt-in" changes like new keywords that would otherwise have the potential to break existing code.

When we introduce a new edition, we don't remove support for the older ones, so all crates continue to compile just as they ever did. Moreover, editions are fully interoperable, so there is no possibility of an "ecosystem split". This means that you can upgrade your crates to the new edition on whatever schedule works best for you.

## How do I upgrade between editions?

Upgrading between editions is meant to be easy. The general rule is, if your code compiles without warnings, you should be able to opt into the new edition, and your code will compile.

Along with each edition, we also release support for it in a tool called `rustfix`, which will automatically migrate your code from the old edition to the new edition, preserving semantics along the way. You may have to do a bit of cleanup after the tool runs, but it shouldn't be much.

## "Migrations" in an edition vs "idiom lints"

When we release a new edition, it comes together with a certain set of "migrations". Migrations are the "breaking changes" introduced by the edition, except of course that since editions are opt-in, no code actually breaks. For example, if we introduce a new keyword, you will have to rename variables or functions using the old keyword, or else use Rust's `r#keyword` feature (which allows you to use a keyword as a regular variable/function/type name). As mentioned before, the edition comes with tooling that will make these changes for you, though sometimes you will want to cleanup the resulting code afterwards.

Editions can also change the default severity of lints, so that instead of defaulting to "warn", they default to "deny" for code using the new edition. This is done to help encourage deprecation of language features or patterns that have been found to be harmful. Because lints will now default to "deny", they can feel like other migrations, but there is an important difference -- you can opt to change the lint level back to "warn" or "allow" if you don't want to change the code yet.

Lints whose severity level changes with an edition are called "idiom lints". For idiom lints associated with Edition N will be warn-by-default for earlier editions, but become deny-by-default for code that opts into Edition N. (Note that the lints are typically introduced long before the edition itself, and they simply issue warnings until the Edition is released.)

As an exception, the Rust 2018 idiom lints will warn-by-default during Rust 2018 and become deny-by-default in Rust 2021 (effectively, they are being "repurposed" as 2021 idiom lints). This is because we never made them into warnings by default in 2018, and it would be disruptive to suddenly have them start erroring on existing code now.

Like migrations, idiom lints are expected to come with automatic tooling for rewriting your code. However, in the limit, that tooling can be as simple as inserting an `#![allow(lint_x)]` at the crate level, although we'd prefer to avoid that.

## The edition guide

The [edition guide](https://doc.rust-lang.org/edition-guide/introduction.html) documents each of Rust's editions and the various migrations and idiom lints that were introduced as part of it. It will be updated to use the terminology from this RFC, naturally, and be updated during each edition.

The aim of the edition guide is to help users who are migrating code from one edition to the next. Therefore, it will discuss the migrations and lints introduced as part of an edition. It will not discuss features that work across all editions, even if those features were introduced since the previous edition was released. (This marks a change from the current guide, which for example covered the `?` operator as part of Rust 2018, even though that operator can be used in Rust 2015 code.)

## Editions and semver

For semver purposes, you should think of editions as being equivalent to any other Rust feature. If Edition N is stabilized in rustc release 1.X, then upgrading your crate to edition N also means that your crate can only be compiled with rustc release 1.X or later. This is no different than if you added a use of some other new feature that was added in release 1.X but which is not tied to editions.

Rust does not have an official policy on whether it is a semver breaking change to change the version of the Rust compiler required to compile your crate. In practice, widely used crates generally adopt and document a "MSRV" (Minimum Supported Rust Version) and have rules about when it can be changed. Upgrading to an edition may then trigger a change to the MSRV and hence could be considered a breaking change, depending on the crate's policy.

# Reference-level explanation

[reference-level-explanation]: #reference-level-explanation

We use this section to answer detailed questions.

## Integrating the 3-year cadence into our roadmap planning

It is expected that the 3-year edition cadence can inform our roadmap and feature planning. In particular, larger features or features that may require migrations should be coordinated to begin work earlier in the 3-year cadence.

## Migrations

Migrations are the "breaking changes" that we make as part of an edition transition (except of course that they don't break any code). We want to ensure a smooth user experience, so each such change must either:

- Have automated, rustfix-compatible tooling that will ensure that old code continues to work in the new edition with the same semantics as before.
- Or, be expected to occur _very_ rarely, as evidenced by crater runs and experience. In these cases, it is preferable if the migration causes a compilation error, rather than silently changing semantics.

In some cases, migrations can come with a combination. For example, there may be tooling to port old code that works the vast majority of the time but occasionally fails (this can happen around macros).

## Idiom lint transitions

"Idiom lints" are issued in a lint group named after the edition year, such as `rust_2018_idioms`. They are warn-by-default in the previous edition, and are deny by default in the new edition.

Idiom lints are encouraged but not required to produce "rustfix"-compatible suggestions.

## Keyword reservation policy

One question that comes up around editions is whether to reserve keywords which we think we _might_ want but for which we don't have a particular use in mind yet. For the Rust 2018 edition, we opted not to reserve any such keywords, and in this RFC we re-affirm that policy.

The policy is that **new keywords can be introduced in an edition only as part of a design with an accepted RFC**. Note that if there is an accepted RFC for some design that introduces a new keyword, but the design is not yet fully implemented, then the edition might still make that keyword illegal. This way, the way is clear when the time comes to introduce that keyword in the future. As an example, this is what happened with async/await: the async keyword was introduced as part of the 2018 edition, but didn't do anything until later in the release cycle.

The motivation here is that any attempt to figure out a reasonable set of keywords to reserve seems inevitably to turn into "keyword fishing", where we wind up with a long list of potential keywords. This ultimately leads to user confusion and a degraded experience. Given that editions come on a regular basis, it suffices to simply allow the keyword to be reserved in the next edition. If we really want to expose the feature earlier, then a macro or other workaround can be used in the interim (and transitioned automatically as part of the move to the next edition).

## Leveraging editions for phasing in large changes

In some cases, we may leverage editions for phasing in changes which will ultimately be used for all versions of Rust. As an example, consider the introduction of [non-lexical lifetimes][rfc 2094]. Implementing that RFC required introducing an entirely new version of the borrow checker. This new borrow checker included a number of bugfixes that, while valid, had the effect of causing existing code not to compile. Therefore, we didn't want to phase it in all at once. Moreover, since this was new code, we wanted to give it some time to be used in practice to help uncover problems. We solved these issues by first deploying the new borrow checker only for Rust 2018 code. This limited its effects and gave us more time for testing. Once we were more confident in the new code, we were able to start issuing warnings for Rust 2015 code and eventually removing the old borrow checker altogether. There are other upcoming changes, such as further overhauls to the borrowing system, or changes to how we resolve traits, where we may wish to make use of an edition in a similar way.

[rfc 2094]: https://github.com/rust-lang/rfcs/blob/master/text/2094-nll.md

# Drawbacks

[drawbacks]: #drawbacks

The primary drawbacks of doing editions at all are as follows:

- Coordinating an edition release is a stressor on the organization, as we have to coordinate the transition tooling, documentation, and other changes. This was particularly true in the original formulation of the editions, which put a heavy emphasis on the "feature-driven" nature of the 2018 Edition (i.e., the goal was to release new and exciting features, not to look back on work that had already been completed).
- Transitioning to a new edition, even if optional, is an ask for our users. Some production users expressed frustration at having to spend the time upgrading their crates to the new edition. Even with tooling, the task requires time and effort to coordinate. At this stage in Rust's life, "production use" often implies "commercial use," and time and effort means "money" to them. Asking too much could harm Rust's commercial prospects, with all of the secondary impacts that has on the not-for-profit ecosystem as well.

# Rationale and alternatives

[rationale-and-alternatives]: #rationale-and-alternatives

There are several alternative designs which we could consider.

## Stop doing editions

We could simply stop doing editions altogether. However, this would mean that we are no longer able to introduce new keywords or correct language features that are widely seen as missteps, and it seems like an overreaction.

## Do editions only on demand

An alternative would be to wait and only do an edition when we have a need for one -- i.e., when we have some particular language change in mind. But by making editions less predictable, this would complicate the discussion of new features and changes, as it introduces more variables. Under the "train model" proposed here, the timing of the edition is a known quantity that can be taken into account when designing new features.

## Skipping editions

Similar to the previous, we might have an edition schedule, but simply skip an edition if, in some particular year, there aren't any migrations. This remains an option, but it remains unclear whether this will ever happen, and it also adds an additional variable that complicates RFC discussions ("But if we accept this, that'll be the only reason to have an edition, and it doesn't seem worth it.")

## Feature-driven editions released when things are ready, but not on a fixed schedule

An alternative to doing editions on a schedule would be to do a **feature-driven** edition. Under this model, editions would be tied to a particular set of features we want to introduce, and they would be released when those features complete. This is what Ember did with [its notion of editions](https://emberjs.com/editions/). As part of this, Ember's editions are given names ("Octane") rather than being tied to years, since it is not known when the edition will be released when planning begins.

This model works well for larger, sweeping changes, such as the changes to module paths in Rust 2018, but it doesn't work as well for smaller, more targeted changes, such as those that are being considered for Rust 2021. To take one example, [RFC 2229] introduced some tweaks to how closure captures work. When that implementation is ready, it will require an edition to phase in. However, it on its own is hardly worthy of a "special edition". It may be that this change, combined with a few others, merits an edition, but that then requires that we consider "sets of changes" rather than judging each change on its own individual merits.

[rfc 2229]: https://github.com/rust-lang/rfcs/blob/master/text/2229-capture-disjoint-fields.md

The fact is that, in practice, we don't expect that Rust will contain a large number of "sweeping changes" like the module reform from Rust 2018. That was rather the exception and not the norm. We expect most changes to be more analogous to the introduction of `async fn`, where we simply added a keyword, or to the closure changes from [RFC 2229].

# Prior art

[prior-art]: #prior-art

- [RFC 2052] introduced Rust's editions.
- Ember's notion of feature-driven editions were introduced in [Ember RFC 364](https://github.com/emberjs/rfcs/blob/master/text/0364-roadmap-2018.md).
- As noted in [RFC 2052], C/C++ and Java compilers both have ways of specifying which version of the standard the code is expected to conform to.
- The [XSLT programming language](https://www.w3.org/TR/xslt-30/) had explicit version information embedded in every program that was used to guide transitions. (Author's note: nikomatsakis used to work on an XSLT compiler and cannot resist citing this example. nikomatsakis also discovered that there is apparently an XSLT 3.0 now. 👀)

# Unresolved questions

[unresolved-questions]: #unresolved-questions

None.

# Future possibilities

[future-possibilities]: #future-possibilities

None. It's perfect. =)

# Appendix A. Possible migrations for a Rust 2021 edition.

At present, there are two accepted RFCs that would require migrations and which are actively being pursued. Neither represents a "large-scale" change to the compiler.

[RFC 2229] modifies closures so that a closure like `|| ... a.b.c ...` will, in some cases, capture _just_ the field `a.b.c` instead of capturing all of `a`. This can affect when values are dropped, since in some cases the older closure might have captured all of `a` and then dropped it when the closure was dropped. Most of the time this doesn't matter (and we can likely detect most of those cases). But in some cases, it might, and hence the migration would introduce a `let a = a;` statement to preserve the existing drop order.

[RFC 2795] introduces implicit named arguments in format strings, so that one can write `panic!("error: {error_code}")` in place of `panic!("error: {error_code}", error_code=error_code)`. However, in today's code, the former is accepted and simply panics with a `&str` equal to `error: {error_code}`. A migration can detect this edge case and rewrite the panic to preserve these semantics, [as discussed on the tracking issue](https://github.com/rust-lang/rust/issues/67984#issuecomment-653909850).

[rfc 2795]: https://rust-lang.github.io/rfcs/2795-format-args-implicit-identifiers.html
