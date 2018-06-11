- Feature Name: `reserve_throw_fail`
- Start Date: 2018-05-14
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

The keywords `throw` and `fail` are reserved in edition 2018 and beyond.
The keywords will still be permitted as attribute names and as macros,
i.e: `#[fail]` and `throw!(..)` calls is permissible.

# Motivation
[motivation]: #motivation

[RFC 2426]: https://github.com/rust-lang/rfcs/pull/2426

The motivation for reserving `fail` and `throw` are so that we have the option
to later use them for some `fail expr` or `throw expr`-like construct such as
proposed in [RFC 2426].

Since edition 2018 is approaching, we are under time constraints to get the
keyword reserved even if the details of [RFC 2426] or similar proposals have
not been fully fleshed out.

The reason we are reserving two keywords is so that we can delay the choice
between them since there isn't consensus for which one to pick.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The words `fail` and `throw` are reserved as keywords in edition 2018.
This means that code in edition 2018 can't use it directly as an identifier.
However, you can always use raw identifiers as in `r#fail` if you need
to refer to `fail`, used in a crate from 2015, from an edition 2018 crate.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

[list of keywords]: https://doc.rust-lang.org/book/second-edition/appendix-01-keywords.html

The words `fail` and `throw` are reserved as keywords in edition 2018 and
added to the [list of keywords].

To ensure that the `failure` crate's `#[fail]` crate is not broken nor
macros called as `throw!(..)`, those uses of `fail` and `throw` as macros
invocations and as attribute names will still work on edition 2018.

# Drawbacks
[drawbacks]: #drawbacks

## It might not end up being used

Simply put, we *might* not end up using any of the keywords.
We can also be certain that only one of the keywords will be used in the end.

## Edition breakage

Some code will break when transitioning from edition 2015 to 2018.
Most of this will be easily fixable with `rustfix`.
However, transitioning between editions will add some churn.

### For `throw`

We analyse the extent of the breakage and find that `throw`:

+ is not used as an identifier in the standard library.
+ is used as the name of a [crate](https://crates.io/crates/throw).
  This crate has zero reverse dependencies.
+ is found 3+ times by [sourcegraph](https://sourcegraph.com/search?q=repogroup:crates+case:yes++\b((let|const|type|)\s%2Bthrow\s%2B%3D|(fn|impl|mod|struct|enum|union|trait)\s%2Bthrow)\b+max:400).
  The extent of breakage is minimal.

### For `fail`

We analyse the extent of the breakage and find that `fail`:

+ is not used as an identifier in the standard library.
+ is used as the name of a [crate](https://crates.io/crates/fail).
  This one does have 5 reverse dependencies.
  However, they are all written by the same author.
  
+ is found 20+ times by [sourcegraph](https://sourcegraph.com/search?q=repogroup:crates+case:yes++%5Cb%28%28let%7Cconst%7Ctype%7C%29%5Cs%2Bfail%5Cs%2B%3D%7C%28fn%7Cimpl%7Cmod%7Cstruct%7Cenum%7Cunion%7Ctrait%29%5Cs%2Bfail%29%5Cb+max:400).
  The extent of breakage is fairly minimal.

# Rationale and alternatives
[alternatives]: #alternatives

A more frugal option to reserving two words would be to reserve one word.
However, as mentioned before, there is no consensus for which word that would be.

Another option is to simply not reserve anything, which would limit our options
for the future. However, we feel confident that we should keep this option open
to us right now. Not doing so would mean that we couldn't use the words `fail`
or `throw` as keywords for another 3 years or so.

[keyword policy]: https://paper.dropbox.com/doc/Keyword-policy-SmIMziXBzoQOEQmRgjJPm
[permalink]: https://gist.github.com/Centril/4c82c19b3cb02cc565622a37d1591785

The keywords also can't be contextual since `fail {}` would clash with a struct
named `fail`. See [RFC 2426] for a longer discussion. 
Furthermore, a recent [keyword policy] ([permalink]), adopted by the language
team, decided that moving forward, keywords for new features in new editions
should be real keywords instead of being contextual. The main motivation
for this was to optimize for maintenance (and reduce technical debt).

With respect to the choice of keyword, it is also discussed in [RFC 2426].
We pick `fail` as the non-exceptional alternative and `throw` as the most
popular exceptional alternative. Since `raise` is exceptional terminology but
not as frequently used as `throw`, we will not reserve it.

# Prior art
[prior-art]: #prior-art

For usage of a `throw` like construct, see [RFC 2426](https://github.com/Centril/rfcs/blob/rfc/throw-expr/text/0000-throw-expr.md#prior-art)'s prior art.

# Unresolved questions
[unresolved]: #unresolved-questions

None.
