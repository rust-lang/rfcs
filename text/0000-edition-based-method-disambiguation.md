- Feature Name: `edition_based_method_disambiguation`
- Start Date: 2022-03-04
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

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

This feature will be implemented by modifying the `rustc_stable` and
`rustc_const_stable` attributes to support an additional optional `edition`
field.

During method resolution, when we detect an ambiguity we should then check if
one of the methods in question is a standard libary method with an `edition`
field. When it exists and the edition field of that method matches the current
edition we ignore that method and select the other method that conflicted with
it and generate a warning. If the edition is a previous edition we continue as
normal and emit an error for the ambiguity.

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

* encoding the released std version in crates published to crates.io when
  uploading and ignore methods introduced in newer releases when building that
  published dependency.
* [Supertrait item shadowing](https://github.com/rust-lang/rfcs/pull/2845)

These solutions also do not solve the problem generally. The former solution
would only solve the problem for crates uploaded to crates.io, and the latter
would only solve the problem for traits where the ambiguity is introduced by a
supertrait.

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


# Future possibilities
[future-possibilities]: #future-possibilities

As this RFC previously pointed out in the drawbacks section, introducing a new
syntax for unambiguous method calls for a specific trait would significantly
improve the experience of resolving these warnings.

[^1]: Definition: Pre-edition methods are methods that could legally have been
  introduced during the current edition which do not conflict with any methods
  that existed during the initial release of the current edition.
[^2]: https://github.com/rust-lang/rust/issues/86682
