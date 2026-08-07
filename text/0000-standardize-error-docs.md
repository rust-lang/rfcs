- Feature Name: standardize_error_docs
- Start Date: 2023-01-10
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)

# Summary
[summary]: #summary

(Re)-standardize long-form error code explanations (referred to in this RFC as "error docs") with a more flexible system that also takes into account new errors which do not fit into the format as defined by [RFC 1567]. This RFC replaces [RFC 1567] as the primary specification for error docs. This RFC ultimately wishes to make error docs more accessible and easy to read for both new and proficient Rust users.

# Motivation
[motivation]: #motivation

[RFC 1567] provided a standard that fulfilled its aim of ensuring *readable* and *helpful* error docs. However many changes have occurred to error docs since the RFC, such as [error docs being moved to their own files](https://github.com/rust-lang/rust/pull/66314), [error docs' examples use multiple crates](https://github.com/rust-lang/rust/pull/106028) and [documentation of internal error codes](https://github.com/rust-lang/rust/pull/106614). These changes, among others, now make [RFC 1567] out of date and re-creates the very problem [RFC 1567] solved: nonstandardized error docs.  

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This RFC makes three changes to error docs in order to fulfill this RFC's (and [RFC 1567]'s) goals:

- Firstly, that the style of error docs is not defined by RFC but by a "style spec" located in the `rust-lang/rust` repository. This style spec can be updated from time-to-time in order to ensure consistent formatting across error docs. Any change to the style spec must be with the consensus of the dev-tools team. It is recommended that the style spec is initially extracted from [RFC 1567].
- Secondly, that the idea of "labels" is introduced to error docs. These would specify certain attributes about an error code that are important for users to know. Error codes can be marked with labels easily and are verified at compile-time (this would probably happen in the `register_diagnostics` macro). These labels would be added as markdown headers at doc-time and would replace the current typo-prone system of copying long lines around. Labels will be displayed in visible places in order to quickly show the user important attributes of certain error codes. For example, they could be shown in the text generated for the `--explain` flag and shown as colored badges on the error index site. Labels are defined in the style spec. Three label-candidates are exampled here:
  - `removed`: An error code has been removed, and therefore the labeled docs docs exist for backwards-compatibility purposes only. The docs may also contain a note explaining *where* the error code was moved to, if at all. (The style spec can specify this more fully)
  - `internal`: An error code is internal to the compiler/standard library. This means that it can only occur when using a perma-unstable feature gate which is explicitly marked as only intended for use internally.
  - `feature_gated`: An error code can only be emitted when using an unstable feature. This error is subject to change or removal, all the normal feature gate non-guarantees apply.
- Thirdly, that error docs (including the new labels feature) are aggressively linted according to the style spec. This helps keep the error docs consistently styled.

# Drawbacks
[drawbacks]: #drawbacks

It requires a lot of effort:
- The error docs check in `tidy` will have to be kept up to date with the style spec. 
- Error docs will have to be kept up to date with the style spec.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Some changes must be made, otherwise PR authors coming up with their own formatting for error docs will continue to increase. *This is already happening* and only makes error docs harder to understand.

# Prior art
[prior-art]: #prior-art

[RFC 1567] is prior art, its significance to this RFC was mentioned in the [motivation] section.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Is the current `mdBook` format used to present error docs still sufficient? Its index page is very unappealing and makes it hard to quickly jump to an error. Furthermore, it does not interface well with the new idea of error doc labels and cannot quickly show labels applied to an error.
- Should changes to the style spec require more or less than "dev-tools team consensus"?

# Future possibilities
[future-possibilities]: #future-possibilities

Perhaps every couple years this RFC (and maybe other related things) is reviewed and a decision is made around whether changes are needed? I'm not sure about formalizing this process though.

[RFC 1567]: https://rust-lang.github.io/rfcs/1567-long-error-codes-explanation-normalization.html