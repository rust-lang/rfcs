- Feature Name: `edition_based_method_disambiguation`
- Start Date: 2022-03-04
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

*Note*: **The changes proposed in this RFC do not tie methods to editions**. It
only turns currently allowed breakages between editions into a breakage at an
edition boundary, if there is no ambiguity the method is still callable
immediately upon stabilization as things work today. This RFC only changes
behavior when there is an ambiguity to make things that are currently errors
into warnings until the next edition boundary.

This RFC proposes a way to introduce new trait methods that conflict with
pre-edition[^1] trait methods in downstream crates in a backwards compatible fashion.
We do so by annotating new methods with the edition they're introduced in. Then
when ambigutity is detected between a new method in the standard library and an
pre-edition downstream method the compiler will check if the crate edition matches
the edition that the method was introduced in. If it does we pick the
pre-edition method and output a warning that there was
an ambigutity with a newly introduced std method and that this warning will be
promoted to a hard error in the next edition.

# Motivation
[motivation]: #motivation

Rust has had a long standing issue with breaking changes caused by introducing
new methods that conflict with pre-edition downstream methods. This issue is best
exemplified with the recent attempt to move `Itertools::intersperse` into the
`Iterator` trait which [broke a large number of
crates](https://github.com/rust-lang/rust/issues/88967). Continuing as we have
been and managing these breakages on a case by case level is not aligned with
our strict stability guarantees. The libs-api team needs a robust solution to
introduce methods like these without causing any breakage.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

*written as though this feature is already implemented and stable*

The rust standard library recently added support for adding new methods to
traits that conflict with pre-edition[^1] methods with the same name on other
traits without causing breakage due to ambiguity. Since adding this feature you
may start running into errors that look like this

```
warning[E0034]: multiple applicable items in scope
  --> src/lib.rs:23:10
   |
23 |     it.intersperse(sep)
   |        ^^^^^^^^^^^ multiple `intersperse` found
   |
note: candidate #1 is defined in an impl of the trait `Iterator` for the type `MyIter`
  --> src/lib.rs:7:1
   |
7  | impl Iterator for MyIter {
   | ^^^^^^^^^^^^^^^^^^^^^^^^
   = note: this was introduced in the current edition and has been deprioritized to prevent breakage
   = warning: in the next edition this warning will become an error
note: candidate #2 is defined in an impl of the trait `Itertools` for the type `MyIter`
  --> src/lib.rs:16:5
   |
16 |     fn intersperse(self, separator: Self::Item) -> Intersperse<Self> {
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   = note: to avoid the ambiguity, this candidate was selected to prevent breakage in edition < 20XX
help: disambiguate the associated function for candidate #1
   |
23 |     Iterator::intersperse(it, sep)
   |
help: disambiguate the associated function for candidate #2
   |
23 |     Itertools::intersperse(it, sep)
   |
```

These errors are an expected stage of the standard library development
lifecycle, where methods are experimented within 3rd party crates and then
moved into the standard library once they've been thorougly tested. A classic
example of this is the `Itertools` crate for experimenting with extentions to
the `Iterator` trait. However this problem isn't restricted to extension traits
of pre-edition standard library traits, and can indeed become a problem whenever
any two methods have the same name.

You can fix issues like this by manually editing the code to select the
specific version of the method you wish to use or, in certain common cases, you
can use cargo fix. cargo fix will make assumptions about how the methods relate
depending on if you're using cargo fix for an edition upgrade or not.

* **Within same edition** cargo fix will assume that the new method is a drop
  in replacement of the pre-edition downstream one and will disambiguate by
  selecting the upstream method defined in `std`.
* **As part of an edition upgrade** cargo fix will prioritize maintaining the
  same behavior, and will disambiguate by selecting the pre-edition method that
  was being used previously.

To run cargo fix within the same edition run:

```
cargo fix
```

In the example above this would replace the ambiguous code with
`Iterator::intersperse(it, sep)`, selecting the new implementation.

To run cargo fix as part of an edition upgrade run:

```
cargo fix --edition
```

In the example above this would replace the ambiguous code with
`Itertools::intersperse(it, sep)`, maintaining the pre-edition behavior.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This feature will be implemented by modifying the `rustc_stable` attribute to
support an additional optional `edition` field.

During method resolution, when we detect an ambiguity we should then check if
one of the methods in question is a standard library method with an `edition`
field. When the edition field exists in the stability attribute and the edition
field of that method matches the current crate's edition we ignore that method
and select the pre-edition method that conflicted with it and generate a
warning. If the edition field in the stability attribute is an earlier edition
than the crate's edition we continue as normal and emit an error for the
ambiguity.

This flag should be usable to resolve the following forms of breakage:

* A new method on an pre-edition trait ([e.g.
  itertools::intersperse](https://github.com/rust-lang/rust/issues/88967))
* A new trait implementation of an pre-edition trait on an pre-edition type ([e.g.
  ErrorKind Display](https://github.com/rust-lang/rust/issues/94507))
* A new inherent method on an pre-edition type (no recent examples)

# Drawbacks
[drawbacks]: #drawbacks

## Disambiguation can require invasive changes

In simple cases where you only call a single method in an expression switching
from the ambiguous method call syntax, `self.method(args...)` to the
unambiguous function call syntax `Trait::method(self, args...)` is an easy
change. In longer method call chains however there isn't a way to disambiguate
the trait a method is associated with when calling it without splitting up that
expression into multiple expressions, which can change drop behavior and
prevent temporaries for living as long as they need to.

This RFC intentionally avoids solving this problem or even proposing strawmen
versions of the syntax to avoid distracting from the core issue, but at the
same time it increases the need for a language syntax extension for quickly
disambiguating the trait a method call should come from.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

[This
comment](https://github.com/rust-lang/rust/issues/88967#issuecomment-938024847)
on the `Iterator::intersperse` issue details a few alternatives that were
discussed by the libs team when we encountered that specific issue. These
include both short term and long term suggestions.

**Short Term**

* use build scripts in crates where this breakage is expected to detect when
  the breakage is introduced by the compiler and disable the conflicting APIs
  internally using `#[cfg]` directives.
* use `#[cfg(accessible)]` in the crate where the breakage is expected to
  automatically remove the conflicting API when the upstream API is introduced.

These solutions don't solve the problem generally, and instead address the
specific breakage that are known or expected. We can and do catch many such
issues via crater but we cannot test all crates via crater and we still end up
breaking people's crates on nightly before the crater runs have had a chance to
catch any breakage.

**Longer Term**

* `rust-version` based visibility filtering - make it a hard error to use APIs
  that were introduced in later versions of Rust than your current Minimum
  Supported Rust Version (MSRV) as specified in the `rust-version` field.
* [Supertrait item shadowing](https://github.com/rust-lang/rfcs/pull/2845)

These proposals are not actually alternatives, but rather complementary
features that help reduce breakages and which should be persued alongside this
RFC.

The `rust-version` approach would prevent many breakages by not ever resolving
ambiguous method calls to new methods when those new methods are introduced in
later versions than your MSRV, but it would not be a complete solution by
itself since otherwise it would turn bumping MSRV into a breaking change. This
is counter to our stability policy which promises to only introduce breaking
changes at edition boundaries.

The Supertrait item shadowing RFC would prevent breakages where traits have a
supertrait/subtrait relationship such as in the `Iterator`/`Itertools` case and
would give us a better fallback, where we can immediately resolve methods to
the supertrait instance within the same edition rather than producing the
warning, but it does not help with situations like the `Display`/`Debug`
breakage or with new inherent methods where a supertrait/subtrait relationship
does not exist.

# Prior art
[prior-art]: #prior-art

- [previous discussion on irlo](https://internals.rust-lang.org/t/idea-paths-in-method-names/6834/14?u=scottmcm)

# Unresolved questions
[unresolved-questions]: #unresolved-questions

## Interaction with autoderef

We already have logic for preferring stable methods to unstable methods but it
breaks[^2] if the auto deref for one requires one more level of indirection
than the other. We should be careful to consider how autoderef behavior can
affect edition based method disambiguation.

As a prior example, the addition of `into_iter` for arrays was done via special
case treatment in the compiler because of this exact sort of breakage in
autoderef precidence. If we can make this edition based disambiguation properly
handle autoderef precidence we maybe able to remove that special case handling
for array's `into_iter` impl and replace it with an `edition = "2018"` field in
its stability attribute.

# Future possibilities
[future-possibilities]: #future-possibilities

## Unambiguous method call syntax

As this RFC previously pointed out in the drawbacks section, introducing a new
syntax for unambiguous method calls for a specific trait would significantly
improve the experience of resolving these warnings.

## Extension to 3rd party crates ecosystem

The lang teams is already persuing the possibility of [stabilizing stability
attributes](https://github.com/rust-lang/lang-team/blob/master/design-meeting-minutes/2022-02-16-libs-most-wanted.md#make-stable-and-unstable-available-for-third-party-crates)
to allow 3rd party crates to mark APIs as `#[stable]` or `#[unstable]`. We
would likely need to consider how this disambiguation functionality would be
extended along with the stability attributes. How it would interact with semver
and editions, and whether we could better support crates that take a similar
perma-1.0 stability policy to that of `std`.

[^1]: Definition: Pre-edition methods are methods that could legally have been
  introduced during the current crate's edition which do not conflict with any
  methods that existed during the initial release of that edition.
[^2]: https://github.com/rust-lang/rust/issues/86682
