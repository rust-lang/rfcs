- Title: Update the RFC template
- Feature Name: N/A
- Start Date: 2022-10-28
- RFC PR: [#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Tracking Issue: N/A
- Team: core
- Keywords: process, meta, RFC
- Previous RFCs: [#6](https://github.com/rust-lang/rfcs/pull/6), [#32](https://github.com/rust-lang/rfcs/pull/32), [#1636](https://github.com/rust-lang/rfcs/pull/1636), [#2059](https://github.com/rust-lang/rfcs/pull/2059), [#2333](https://github.com/rust-lang/rfcs/pull/2333), [#2561](https://github.com/rust-lang/rfcs/pull/2561)
- Previous discussion: N/A

# Summary
[summary]: #summary

Improve the RFC template by adding more metadata, making the structure more flexible, and tweaking some wording.

Note that this proposal uses the proposed template as a bootstrap example.

# Motivation and background
[motivation-and-background]: #motivation-and-background

Rust's RFC process has existed since before the 1.0 release and is the primary decision-making process in the community. Over time the process has evolved and we've iterated on the template used for defining RFC proposals. This RFC proposes another iteration to address some weaknesses in the current template. These weaknesses have either revealed themselves over time, or have become worse as the volume of RFCs has grown, and as RFCs have tended to become more involved.

Concretely,

* it is hard to search for RFCs. RFCs do not have a title and some metadata is attached to the PR rather than the document itself. When an RFC is merged that metadata is no longer directly attached to the RFC.
* RFCs are used for many purposes, not just language and standard library changes. The current template is over-specialized for language and standard library changes.
* RFCs often contain unnecessary boilerplate to conform to the template. Newcomers to the process are less confident to ignore or change the template to make the RFC more readable, and conforming to the template can be a source of friction for newcomers.
* The main part of an RFC is usually the detailed explanation. This is currently split between guide-level and reference-level sections. This is intended to focus the author on describing the proposal both for users and for implementers, and to aid in documentation of the feature. However, in practice it means that there isn't a single, self-contained description of the proposal, that parts of the description are duplicated, and that authors have to role-play as technical writers, then readers have to mentally translate back to a proposal format. Furthermore, the current template requires explanation from the perspective of the complete feature, rather than describing the change from current Rust to Rust with the proposal. Often, the latter format is more appropriate.
* RFCs often extend, supersede, or deprecate previous RFCs, and/or are based on extensive previous discussion, but this is not indicated in any uniform way.

# Detailed explanation
[detailed-explanation]: #detailed-explanation

See the modified [template](../0000-template.md) for the proposed template. I'll describe the changes in this section. There are also some small, miscellaneous wording changes which hopefully don't need explanation.

### Added metadata

I've added several metadata fields to the top of the document. These are mostly things which many RFC proposals already include, but are kept in the PR rather than the RFC text, are totally implicit, or have no tool-visible representation.

The `title` field gives the RFC a human-oriented title which should make RFCs easier to browse, discuss, and cite.

The `team` field duplicates the team labels from the PR. It preserves this information in the RFC document and lets the author suggest the relevant team(s). In the future, tooling could automatically populate the PR labels from this field.

The `keywords` field lets authors specify keywords or tags. This should make RFCs easier to browse and search. Again, in the future, tooling could populate labels from this field.

The `previous RFCs` field allows linking of RFCs. This makes explicit where an RFC replaces or extends a previous RFC, in a manner which could be interpreted by tooling.

The `previous discussions` field allows linking of previous discussions. We encourage RFCs to be discussed elsewhere before making an RFC PR, this field allows (and encourages) keeping track of those discussions.

### Extend motivation section to explicitly include background

RFCs often require significant background or context to understand; there is currently nowhere in the RFC template for this. I think extending the motivation section to include background is natural and useful.

### Merge detailed explanation sections

I've merged 'guide-level explanation' and 'reference-level explanation' into 'detailed explanation'.

### Simplify drawbacks, rationale, and alternatives sections

I've changed the name of 'rationale and alternatives' to just 'rationale' and merged it with the 'drawbacks' section. Rather than having separate sections, I've suggested different kinds of rationale for this one section. This should make this part of the RFC more flexible and better adaptable for different kinds of RFC. I hope that authors will write a better rationale, focussing on what is appropriate for the RFC, rather than trying to conform to a rigid structure.

### Mark some sections as explicitly optional

I've added explicit text to the 'unresolved questions' and 'future possibilities' sections to make clear they are not always necessary. This should reduce boilerplate and make writing an RFC marginally more accessible.

# Rationale
[rationale]: #rationale

We've tried to nudge RFCs in certain directions with the structure of the RFC template. With these changes, I mostly have the same goals, but have tried to do the nudging in the text, and make the structure more flexible. I hope this leads to better RFCs and a better, more accessible experience for RFC authors. The risk here is that the nudging is less effective and RFCs get worse. If that happens we can always revert some or all of the changes, or continue to iterate (I hope we continue to iterate in any case!). I believe that the increased flexibility is worth the risk of less effective nudging.

The downsides of the metadata changes are that it adds some boilerplate to the top of the RFC. I believe that the boilerplate is a price worth paying for having the information in a concise and tool-visible format. We might consider moving the metadata to the end of the document or to after the 'summary' section to mitigate the impact on readability.

Some other alternatives:

* make the changes to the metadata, but abandon the changes to the sections,
* abandon the changes to the metadata and make the changes to the sections,
* add an 'authors' or 'owners' field to the metadata. This has some benefits and some downsides, and is a change to the culture of RFCs which merits its own discussion, therefore it should be a separate RFC.
* Have subsections in the 'rationale' and/or 'detailed design' sections. I think the added complexity outweighs the benefits.
* Add some section aimed at documentation or teaching. We have tried this in the past and it has not been successful. I think encouraging thinking about documentation and teaching in the prose is a better approach.
* Make the 'background' section separate from 'motivation'. The two topics are often linked and I think the more flexible structure is better.

I would like to see some bigger changes to the RFC process. I don't think that the proposed changes make any larger changes more difficult, they are just an incremental improvement on the current situation.

# Prior art
[prior-art]: #prior-art

## Other languages' processes

Many languages and projects have some formal process for making changes and their needs are often quite different from Rust's. I'll cover a few examples here:

### Python

Python uses the PEP. It doesn't have a template for proposals, but it does have a list of [suggested sections](https://peps.python.org/pep-0012/#suggested-sections) which is effectively the same. The sections are very much a suggestion and it is culturally acceptable for PEPs to have a different format. The first few sections are:

* abstract,
* motivation,
* rationale,
* specification,

which closely match what is proposed here (albeit with different names). There are also sections on backwards compatibility, security, etc. which Rust RFCs usually expect to be in the detailed design section(s). They do have a 'how to teach this' section, see some discussion in the rationale section and below.

### Javascript

Javascript is evolved through the TC39 committee. Proposals are a repository rather than a PR. There is a [template](https://github.com/tc39/template-for-proposals) for the repository, but the different style of process and format means it is not directly comparable to Rust's. The [How to write a good explainer](https://github.com/tc39/how-we-work/blob/HEAD/explainer.md) is closer. It is phrased as a suggestion rather than a mandatory structure. The sections are:

* status (including authors),
* motivation,
* use cases,
* description,
* comparison (analogous to 'prior art' in Rust RFCs),
* implementations,
* Q&A.

In Rust RFCs, status is part of the PR metadata rather than the RFC text. Since the PR is primary until the RFC is accepted, there is no point in adding this to the metadata. We might consider having an 'authors' field, but this would be a big change to the culture of Rust RFCs, so is out of scope here.

I believe the level of granularity here matches this proposal, with some different choices which are due to cultural and technical context.

### F#

The F# RFC process is similar to Rust's RFC process and has a similar [template](https://github.com/fsharp/fslang-design/blob/main/RFC_template.md). It's sections are:

* summary,
* motivation,
* detailed design,
* drawbacks,
* alternatives,
* compatibility,
* pragmatics (diagnostics, tooling, performance, scaling, culture-aware formatting/parsing),
* unresolved questions.

The first few sections and the last are similar to in this proposal. They have separate 'drawbacks' and 'alternatives' sections. In Rust, 'compatibility' is usually included in the 'detailed design' sections, and this seems to work well for us. The 'pragmatics' section is an interesting factoring and is worth considering for future work. I think it will take some work to choose appropriate sub-sections for Rust because our RFCs have broader use than in F#.

### Ember

Ember's RFC process was strongly influenced by Rust's and there has been much cross-pollination over the years. Their [template](https://github.com/emberjs/rfcs/blob/master/0000-template.md) is similar to earlier iterations of Rust's template.

## History of the RFC template

The RFC template has always been part of the RFC process but has changed over time. The major changes (and the RFC PR which proposed those change) are:

* #6: initial template (though actually it existed in the repo before the PR because this was the wild-west age of the RFCs repo),
* #32: adds a 'drawbacks' section,
* #1636: adds a "How do we teach this?" section; the motivation was primarily to encourage authors to think about documentation,
* #2059: changes the 'detailed design' and 'how do we teach this' sections into 'guide-level explanation' and 'reference-level explanation' sections; expands 'alternatives' to 'rationale and alternatives'. The motivation was to make the process more accessible, and because there was a feeling that the 'how do we teach this' section was not having the desired effect, in part because it was interpreted as describing the mechanism for teaching, rather than the details of describing the feature to users who have not encountered it before.
* #2333: adds a 'prior art' section,
* #2561: adds a 'future possibilities' section.

My impression is that the guide-level/reference-level split has not worked out for us. Although it has had some benefits in thinking about the way new features are documented, these are outweighed by the drawbacks (discussed above). Adding sections for drawbacks, alternatives, and rationale have all been beneficial, but with the benefit of hindsight, these would be better as one section rather than three. Although no one change is responsible, we can see that the effect of the evolution over time has led to the RFC template having many fine-grained sections.

# Future possibilities
[future-possibilities]: #future-possibilities

As mentioned earlier, part of the rationale for these changes is to make RFCs easier for tools to interpret. I hope that in the future we can have improved tooling for submitting RFCs, and for reading and searching RFCs.

I believe the RFC process as a whole needs improvement (see [blog post](https://ncameron.org/blog/the-problem-with-rfcs/)), however, I don't have concrete suggestions at this time, and such changes are beyond the scope of this proposal.
