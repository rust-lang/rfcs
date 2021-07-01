- Feature Name: (fill me in with a unique ident, `my_awesome_feature`)
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Extend Cargo to allow some limited forms of migrations of feature sets to be expressed.
This will allow adding new features to make existing functionality optional without causing needless breakage.

# Motivation
[motivation]: #motivation

## Problem

Today, Cargo's features most easily support workflow where features added in a new versions of a crate gate new functionality.
For example, in a new version of a crate, someone crates a new function, and adds it behind a new feature; neither the function nor feature existed in the old version of the crate.
The problem is there is another quite different use case of features: making previously mandatory functionality optional.
For example, in a new version of a create, some creates a new feature for a function that *already* exists, so that it can be disabled.
The workflow isn't supported so well, the only avoidance Cargo supports for it is the "default features" set, which isn't sufficient for reasons that will be provided below.

This second use-case is really important --- in fact, perhaps more important.
The important thing to realize about features is that the ultimate *reason* they are used is not controlling a creates interface (outputs), but it's *dependencies* (inputs).
No one minds if a crate provides harmless functionality they don't need.
They do, however, mind if it depends on another crate they cannot provide, e.g. because of platform considerations (bare metal, etc.).
That means if someone adds new functionality that doesn't require new deps (common for case 1), there's little reason to bother adding features.
Conversely, if someone wants to make a dep like `std` optional, there is a lot of benefit to add `std` feature to do so.

Let's return to "default features".
What is it for, and why is it not sufficient?
Simple enough, if we create a new feature gating existing functionality, existing consumers of the functionality won't know about the new feature, so we need to provide some avoidance so they continue to receive the functionality without changing their "request".
Default features allow a new feature to be on by default, so the old functionality continues to work.
New consumers that don't want to use the now-optional feature (and moreover incur the now-optional dependencies obligations), can opt out of default features and then provide a feature list that isn't embellished.

The problem is, what happens if we later have *another* feature making existing functionality optional?
Concretely, let's say 1.0 has no features with functions `foo` and `bar`, 1.1 has a default feature `foo-feature` gating `foo`, and 1.2 has default features `foo-feature` and `bar-feature` gating `foo` and `bar`.
When we introduced 1.1, there were no default features to opt out of.
But when we introduce 1.2, there could be consumers *already* opting out of default features to skip `foo-feature` / `foo`?
How can we ensure those consumers nonetheless still get access to `bar`, while still allowing new consumers to take advantage of `bar-feature` to opt out of `bar`?
Too bad, we cannot.

## New motivation

This gap in Cargo's functionality has taken on new urgency with https://github.com/rust-lang/rfcs/pull/3140, which would propose a feature for the `alloc` crate of the latter sort, making existing functionality optional.
This would be the first example of a Cargo feature on a standard library crate slates for user consumption (as opposed to being some artifact of the current build system with no plans for stabilization).
It's very important we have a good design so that we don't end up accidentally introduction breaking changes in the standard library, because there is no way for users to opt out of the latest version!

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

I recommend first reading the [rationale-and-alternatives] section.
I don't think this design is overwrought, but it does represent a new way of thinking about these sorts of issues that might feel unfamiliar.
I fully acknowledge "migrations" is a scary word for many people due to their experiences with databases.

> Some might find the whole premise funny, because two versions of anything being "compatible" ought to be that no "migrations" are needed by definitions!
> To avoid going on a tangent, that is not my definition of "compatible".

Moreover, I am going with this design because I do think it is our best option.

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
Also consider how this all fits into the roadmap for the project
and of the relevant sub-team.

This is also a good place to "dump ideas", if they are out of scope for the
RFC you are writing but otherwise related.

If you have tried and cannot think of any future possibilities,
you may simply state that you cannot think of anything.

Note that having something written down in the future-possibilities section
is not a reason to accept the current or a future RFC; such notes should be
in the section on motivation or rationale in this or subsequent RFCs.
The section merely provides additional information.
