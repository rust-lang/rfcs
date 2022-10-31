- Title: (fill me in with a human-focussed title for the RFC)
- Feature Name: (fill me in with a unique ident, `my_awesome_feature`)
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: [#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)
- Team: (fill me in with the team or teams responsible for this RFC)
- Keywords: (fill me in with keywords which may be useful when searching for this RFC)
- Previous RFCs: #0000 (fill me in with RFC numbers for any RFCs this RFC supersedes, deprecates, or extends)
- Previous discussion: (fill me in with links to previous discussions such as internals.rust-lang.org or Zulip threads, or issues)

# Summary
[summary]: #summary

One paragraph explanation of the feature.

# Motivation and background
[motivation]: #motivation

Why are we doing this? What use cases does it support? What is the expected outcome? Why are existing solutions not good enough? If possible, include data to support your claims. Include any background context useful for understanding the RFC.

# Detailed explanation
[detailed-explanation]: #detailed-explanation

Explain the proposal in detail. Specify the proposal as it would be experienced by a user (usually a Rust developer) and so that it can be understood by an implementer. That generally means:

- Introducing new named concepts.
- Explaining the feature with examples.
- Explaining how Rust programmers should *think* about the feature, and how it should impact the way they use Rust. It should explain the impact as concretely as possible.
- If applicable, provide sample error messages, deprecation warnings, or migration guidance.
- If applicable, describe the differences between teaching this to existing Rust programmers and new Rust programmers.
- If applicable, how the feature will be taught, documented, and tested.
- It is reasonably clear how the feature would be implemented.
- Its interaction with other features is clear.
- Corner cases are dissected by example.

For implementation-oriented RFCs (e.g. for compiler internals), this section should focus on how compiler contributors should think about the change, and give examples of its concrete impact. For policy RFCs, this section should provide an example-driven introduction to the policy, and explain its impact in concrete terms.

# Rationale
[rationale]: #rationale

Discuss the *why* of your proposal. You might want to include the following:

- What are the trade-offs, drawbacks, and risks of this design?
- Why is this design the best in the space of possible designs?
- How does this design fit into the bigger picture?
- What other designs have been considered and what is the rationale for not choosing them?
- What is the impact of not doing this?

# Prior art
[prior-art]: #prior-art

Discuss prior art, both the good and the bad, in relation to this proposal.
A few examples of what this can include are:

- For language, library, cargo, tools, and compiler proposals: Does this feature exist in other programming languages and what experience have their community had?
- For community proposals: Is this done by some other community and what were their experiences with it?
- For other proposals: What lessons can we learn from what other communities have done here?
- Papers: Are there any published papers or great posts that discuss this? If you have some relevant papers to refer to, this can serve as a more detailed theoretical background.

This section is intended to encourage you as an author to think about the lessons from other languages, provide readers of your RFC with a fuller picture.
If there is no prior art, that is fine - your ideas are interesting to us whether they are brand new or if it is an adaptation from other languages.

Note that while precedent set by other languages is some motivation, it does not on its own motivate an RFC.
Please also take into consideration that rust sometimes intentionally diverges from common language features.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

(Optional)

- What parts of the design do you expect to resolve through the RFC process before this gets merged?
- What parts of the design do you expect to resolve through the implementation of this feature before stabilization?
- What related issues do you consider out of scope for this RFC that could be addressed in the future independently of the solution that comes out of this RFC?

# Future possibilities
[future-possibilities]: #future-possibilities

(Optional)

Think about what the natural extension and evolution of your proposal would
be and how it would affect the language and project as a whole in a holistic
way. Try to use this section as a tool to more fully consider all possible
interactions with the project and language in your proposal.
Also consider how this all fits into the roadmap for the project
and of the relevant sub-team.

This is also a good place to "dump ideas", if they are out of scope for the
RFC you are writing but otherwise related.

Note that having something written down in the future-possibilities section
is not a reason to accept the current or a future RFC; such notes should be
in the section on motivation or rationale in this or subsequent RFCs.
The section merely provides additional information.
