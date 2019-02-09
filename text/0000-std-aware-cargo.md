- Feature Name: cargo_the_std_awakens
- Start Date: 2018-02-09
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Currently, the `core` and `std` components of Rust are handled in a different way than Cargo handles other crate dependencies. This causes issues for non-mainstream targets, such as WASM, Embedded, and new not-yet-tier-1 targets. The following RFC proposes a roadmap to address these concerns in a consistent and incremental process.

# Motivation
[motivation]: #motivation

In today's Rust environment, `core` and `std` are shipped as precompiled objects. This was done for a number of reasons, including faster compile times, and a more consistent experience for users of these dependencies. This design has served fairly well for the bulk of users, however there are a number of less common, but not esoteric uses, that are not well served by this approach. Examples include:

* Supporting new/arbitrary targets, such as those defined by a ".json" file
* Making modifications to `core` or `std` through use of feature flags
* Users who would like to make different optimizations to `core` or `std`, such as `opt-level = 'z'`, with `panic = "abort"`

Previously, these needs were somewhat addressed by the external tool [xargo], which managed the recompilation of these dependencies when necessary. However, this tool has become [deprecated], and even when supported, required a nightly version of the compiler for all operation.

This approach has [gathered support] from various [rust team members], and this RFC aims to take inspiration from tools and workflows like [xargo], and integrate them into Cargo itself.

[xargo]: https://github.com/japaric/xargo
[deprecated]: https://github.com/japaric/xargo/issues/193
[gathered support]: https://github.com/japaric/xargo/issues/193#issuecomment-359180429
[rust team members]: https://www.ncameron.org/blog/cargos-next-few-years/

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Explain the proposal as if it was already included in the language and you were teaching it to another Rust programmer. That generally means:

- Introducing new named concepts.
- Explaining the feature largely in terms of examples.
- Explaining how Rust programmers should *think* about the feature, and how it should impact the way they use Rust. It should explain the impact as concretely as possible.
- If applicable, provide sample error messages, deprecation warnings, or migration guidance.
- If applicable, describe the differences between teaching this to existing Rust programmers and new Rust programmers.

For implementation-oriented RFCs (e.g. for compiler internals), this section should focus on how compiler contributors should think about the change, and give examples of its concrete impact. For policy RFCs, this section should provide an example-driven introduction to the policy, and explain its impact in concrete terms.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This is the technical portion of the RFC. Explain the design in sufficient detail that:

- Its interaction with other features is clear.
- It is reasonably clear how the feature would be implemented.
- Corner cases are dissected by example.

The section should return to the examples given in the previous section, and explain more fully how the detailed proposal makes those examples work.

# Drawbacks
[drawbacks]: #drawbacks

Why should we *not* do this?

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- Why is this design the best in the space of possible designs?
- What other designs have been considered and what is the rationale for not choosing them?
- What is the impact of not doing this?

# Prior art
[prior-art]: #prior-art

Discuss prior art, both the good and the bad, in relation to this proposal.
A few examples of what this can include are:

- For language, library, cargo, tools, and compiler proposals: Does this feature exist in other programming languages and what experience have their community had?
- For community proposals: Is this done by some other community and what were their experiences with it?
- For other teams: What lessons can we learn from what other communities have done here?
- Papers: Are there any published papers or great posts that discuss this? If you have some relevant papers to refer to, this can serve as a more detailed theoretical background.

This section is intended to encourage you as an author to think about the lessons from other languages, provide readers of your RFC with a fuller picture.
If there is no prior art, that is fine - your ideas are interesting to us whether they are brand new or if it is an adaptation from other languages.

Note that while precedent set by other languages is some motivation, it does not on its own motivate an RFC.
Please also take into consideration that rust sometimes intentionally diverges from common language features.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- What parts of the design do you expect to resolve through the RFC process before this gets merged?
- What parts of the design do you expect to resolve through the implementation of this feature before stabilization?
- What related issues do you consider out of scope for this RFC that could be addressed in the future independently of the solution that comes out of this RFC?

# Future possibilities
[future-possibilities]: #future-possibilities

Think about what the natural extension and evolution of your proposal would
be and how it would affect the language and project as a whole in a holistic
way. Try to use this section as a tool to more fully consider all possible
interactions with the project and language in your proposal.
Also consider how the this all fits into the roadmap for the project
and of the relevant sub-team.

This is also a good place to "dump ideas", if they are out of scope for the
RFC you are writing but otherwise related.

If you have tried and cannot think of any future possibilities,
you may simply state that you cannot think of anything.

Note that having something written down in the future-possibilities section
is not a reason to accept the current or a future RFC; such notes should be
in the section on motivation or rationale in this or subsequent RFCs.
The section merely provides additional information.
