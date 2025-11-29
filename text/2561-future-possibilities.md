- Feature Name: `future_possibilities`
- Start Date: 2018-10-11
- RFC PR: [rust-lang/rfcs#2561](https://github.com/rust-lang/rfcs/pull/2561)
- Rust Issue: N/A. The RFC is self-executing.

## Summary
[summary]: #summary

Adds a *"Future possibilities"* section to the `0000-template.md` RFC template
that asks authors to elaborate on what natural extensions there might to their
RFC and what future directions this may take the project into.
This section asks authors to think *holistically*.

## Motivation
[motivation]: #motivation

### The benefit for the author

Often times, when an RFC is written, the only thing an author considers
may be the feature or change proposal itself but not the larger picture
and context in which the RFC operates in. By asking the author to reflect
on future possibilities, a larger degree of introspection within the author
themselves may ensue. The hope is then that they may consider what larger
effects their proposal may have and what subsequent proposals may be.

[#2532]: https://github.com/Centril/rfcs/blob/rfc/assoc-default-groups/text/0000-assoc-default-groups.md#future-work
[#2529]: https://github.com/Centril/rfcs/blob/rfc/hidden-impls/text/0000-hidden-impls.md#future-work-1
[#2524]: https://github.com/Centril/rfcs/blob/rfc/inferred-type-aliases/text/0000-inferred-type-aliases.md#possible-future-work
[#2523]: https://github.com/Centril/rfcs/blob/rfc/cfg-path-version/text/0000-cfg-path-version.md#possible-future-work
[#2522]: https://github.com/Centril/rfcs/blob/rfc/generalized-type-ascription/text/0000-generalized-type-ascription.md#possible-future-work
[#2401]: https://github.com/Centril/rfcs/blob/rfc/mut-pattern-shorthand/text/0000-mut-pattern-shorthand.md#future-work
[#2421]: https://github.com/rust-lang/rfcs/blob/master/text/2421-unreservations-2018.md#possible-future-unreservations
[#2385]: https://github.com/Centril/rfcs/blob/rfc/implied-derive/text/0000-implied-derive.md#future-work
[#2306]: https://github.com/rust-lang/rfcs/blob/master/text/2306-convert-id.md#possible-future-work

The author of this RFC has benefitted personally from writing future-possibilities
sections ([#2532], [#2529], [#2524], [#2523], [#2522], [#2401], [#2421],
[#2385], and [#2306]). Said written sections have also caused the current
author to think more clearly about interactions in each of the written RFCs.
If for no other reason, these sections offer a permanent space to idea-dump
while writing an RFC.

### For the team

The holistic perspective that a future-possibilities section can offer may also
help the relevant sub-team to understand:

1. why something is proposed,
2. what the long term effects of said proposal is,
4. how said proposals fit with the product vision and roadmap that the team
   currently has.

### For readers in general

More generally, the benefits for the teams described above also hold for
all readers. In particular, a reader can better infer what sort of language
Rust is turning into given the information in a future-possibilities section.
Having such a section may also help generate interest in subsequent proposals
which a different author may then write.

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This Meta-RFC modifies the RFC template by adding a *"Future possibilities"*
section after the *"Unresolved questions"*. The newly introduced section is
intended to help the authors, teams and readers in general reflect holistically
on the big picture effects that a specific RFC proposal has.

Please read the [reference-level-explanation] for exact details of what an
RFC author will see in the changed template.

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The implementation of this RFC consists of inserting the following text to the
RFC template *after* the section *Unresolved questions*:

> ## Future possibilities
>
> Think about what the natural extension and evolution of your proposal would
> be and how it would affect the language and project as a whole in a holistic
> way. Try to use this section as a tool to more fully consider all possible
> interactions with the project and language in your proposal.
> Also consider how the this all fits into the roadmap for the project
> and of the relevant sub-team.
>
> This is also a good place to "dump ideas", if they are out of scope for the
> RFC you are writing but otherwise related.
>
> If you have tried and cannot think of any future possibilities,
> you may simply state that you cannot think of anything.
>
> Note that having something written down in the future-possibilities section
> is not a reason to accept the current or a future RFC; such notes should be
> in the section on motivation or rationale in this or subsequent RFCs.
> The section merely provides additional information.

## Drawbacks
[drawbacks]: #drawbacks

There are three main potential drawbacks:

### The section will be unused

There's some risk that the section will simply be left empty and unused.
However, in the recent RFCs written by the author as noted in the [motivation],
this has not been a problem. On the contrary, the very idea behind adding
this section has come as a result of the experience gained by writing
such future-possibilities sections in the aforementioned RFCs.

However, some of the RFCs written by the this RFC's author have not had such
sections. Therefore, if an RFC leaves the newly introduced section empty,
it is not the end of the world. The section is intended as encouragement and
recommendation; it is not mandatory as no section in an RFC has ever really been. 

### Higher barrier to entry

[RFC 2333]: https://github.com/rust-lang/rfcs/blob/master/text/2333-prior-art.md#drawbacks

As noted in [RFC 2333], which was the last RFC to extend the template,
the longer the template becomes, the more work there is to writing an RFC.
This can raise the barrier to entry somewhat.
However, we argue that it is worth the minor raise in the bar since
it is OK for RFCs to leave the section empty.

### Readers reacting negatively on the future possibilities

Another potential drawback is that readers of the RFC will focus too much
on what is written in the future-possibilities section and not the actual proposal
that is made in the RFC. This has not been the case in the RFCs mentioned
in the [motivation].

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

1. We could rephrase the section in various ways.
   It is possible to do such tweaking in the future.

2. We could rename it to "possible future work" or "future work" where the latter
   is more customary, but we have opted to use a section title that makes it more
   clear that the contents of the section are not what is accepted but only
   *possibilities*.

3. We could move the section up and down and around.

4. We could simply not have such a section and leave it up to each author.
   However, we argue here that it is beneficial to hint at the possibility
   of providing such a section. It might otherwise not occur to the author
   that such a section could be written.

## Prior art
[prior-art]: #prior-art

None of the languages enumerated in [RFC 2333] have such a section proposed
in this RFC. However, there are plenty of academic papers published which
do contain sections pertaining to future possibilities. It is customary for
such sections to be at the end of papers so as to not bore readers and keep
them reading.

## Unresolved questions
[unresolved-questions]: #unresolved-questions

None as of yet.

## Future possibilities
[future-possibilities]: #future-possibilities

[staged]: http://smallcultfollowing.com/babysteps/blog/2018/06/20/proposal-for-a-staged-rfc-process/

It may be the case that we would overhaul the RFC template completely if we
undertake larger changes to the RFC process itself as is proposed in the
[staged]-RFCs idea. However, we'll likely want to determine the answers and
get the information that each section in the current template provides at
some point during the lifecycle of a proposal.
