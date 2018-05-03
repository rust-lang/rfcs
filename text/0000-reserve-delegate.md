- Feature Name: `reserve_delegate`
- Start Date: 2018-05-03
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

The keyword `delegate` is reserved in edition 2018 and beyond.

# Motivation
[motivation]: #motivation

[RFC 2393]: https://github.com/rust-lang/rfcs/pull/2393

The motivation for reserving `delegate` is so that have the option to
later use it for delegation such as proposed in [RFC 2393].
Reserving `delegate` also gives us flexibility wrt. *"omitting the `impl` block"*
in the future if we wish.

Furthermore, this RFC is motivated right now by the time constraints to get
the keyword reserved even if the details of [RFC 2393] or similar proposals
has not been fully fleshed out.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The word `delegate` is reserved as a keyword in edition 2018.
This means that code in edition 2018 can't use it directly as an identifier.
However, you can always use raw identifiers as in `r#delegate` if you need
to refer to `delegate`, used in a crate from 2015, from an edition 2018 crate.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

[list of keywords]: https://doc.rust-lang.org/book/second-edition/appendix-01-keywords.html

The word `delegate` is reserved as a keyword in edition 2018 and added to the
[list of keywords].

# Drawbacks
[drawbacks]: #drawbacks

## It might not end up being used

Simply put, a final accepted delegation proposal *might* not end up using the keyword.

## Edition breakage

[sourcegraph]: https://sourcegraph.com/search?q=repogroup:crates+case:yes++%5Cb%28%28let%7Cconst%7Ctype%7C%29%5Cs%2Bdelegate%5Cs%2B%3D%7C%28fn%7Cimpl%7Cmod%7Cstruct%7Cenum%7Cunion%7Ctrait%29%5Cs%2Bdelegate%29%5Cb+max:400

Some code will break when transitioning from edition 2015 to 2018.
Most of this will be easily fixable with `rustfix`.
However, transitioning between editions will add some churn,
therefore, we analyse the extent of the breakage and find that `delegate`:

+ is not used as an identifier in the standard library.
+ is not used as the name of a crate.
+ is found 19+ times by [sourcegraph].
  The extent of breakage is fairly minimal.

# Rationale and alternatives
[alternatives]: #alternatives

A more frugal option to reserving `delegate` would be to reuse `derive` for
these purposes. However, the keyword fits less well than `delegate` with respect
to the user's intent where `delegate` fits quite well.

Another option is to simply not use `delegate`,
however, we feel confident that we should keep this option open to us right now.
Not doing so would mean that we couldn't use the word `delegate` as a keyword
for another 3 years or so.

[keyword policy]: https://paper.dropbox.com/doc/Keyword-policy-SmIMziXBzoQOEQmRgjJPm
[permalink]: https://gist.github.com/Centril/4c82c19b3cb02cc565622a37d1591785

Furthermore, a recent [keyword policy] ([permalink]), adopted by the language team,
decided that moving forward, keywords for new features in new editions
should be real keywords instead of being contextual. The main motivation
for this was to optimize for maintenance (and reduce technical debt).

# Prior art
[prior-art]: #prior-art

C# uses the word [`delegate`](https://docs.microsoft.com/en-us/dotnet/csharp/language-reference/keywords/delegate)
for something different.
However, the concept of *"delegation"* is widely used to mean different things
in different languages.

# Unresolved questions
[unresolved]: #unresolved-questions

None.
