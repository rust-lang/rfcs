- Feature Name: Limits trait for the rust types
- Start Date: 2017-12-20
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

This is an RFC to add a universal trait for the type limits. This is needed because at the moment of writing it is not
possible to abstract types which have limits. This could be done with using generic types or trait objects, but since
the `min()` and `max()` functions are not in a separate trait it is not possible. Also, providing a trait for this adds
possibility to implement it for any type and so use the limits abstractions with any type, not just ones we already have.

# Motivation
[motivation]: #motivation

The motivation is simple: By providing the methods in a trait, user code is able to require with a bound `X: Limit<Y>` that X has certain limits of type `Y`, which enables generic reasoning and simplifies code.


Another motivation is that we already have inherent methods `.max_value()` and `.min_value()` on all primitive numeric types. Generalizing those methods as a trait simplifies the code and avoids duplication.


Looking at other languages, C++ provides `std::numeric_limits`, while Haskell provides the `Bounded` typeclass. This provides precedent that such a facility belongs in the standard library.


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Explain the proposal as if it was already included in the language and you were teaching it to another Rust programmer. That generally means:

- Introducing new named concepts.
- Explaining the feature largely in terms of examples.
- Explaining how Rust programmers should *think* about the feature, and how it should impact the way they use Rust. It should explain the impact as concretely as possible.
- If applicable, provide sample error messages, deprecation warnings, or migration guidance.
- If applicable, describe the differences between teaching this to existing Rust programmers and new Rust programmers.

For implementation-oriented RFCs (e.g. for compiler internals), this section should focus on how compiler contributors should think about the change, and give examples of its concrete impact. For policy RFCs, this section should provide an example-driven introduction to the policy, and explain its impact in concrete terms.

The proposed feature adds a new trait to the standard library crate (`std`). The trait is called `Limits`.

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
[alternatives]: #alternatives

- Why is this design the best in the space of possible designs?
- What other designs have been considered and what is the rationale for not choosing them?
- What is the impact of not doing this?

# Unresolved questions
[unresolved]: #unresolved-questions

- What parts of the design do you expect to resolve through the RFC process before this gets merged?
- What parts of the design do you expect to resolve through the implementation of this feature before stabilization?
- What related issues do you consider out of scope for this RFC that could be addressed in the future independently of the solution that comes out of this RFC?
