- Feature Name: transitional_into
- Start Date: 2018-07-28
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

If there are conversions to an intermediate type, and to some other type, derive a direct conversion to the other type.

# Motivation
[motivation]: #motivation

If we are, e. g. converting units, and we can convert centimeter to meter,
and meter converts to feet, a conversion from centimeter to feet should be derived.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

If a type `A` implements `Into<B>` and `B` implements `Into<C>`,
`Into<C>` is automatically implemented for type `A`.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

If `A` already has a custom `Into<C>` implementation, it will override
the auto implementation.

Note that this should be reflexive with `From`:
if `B` implements `Into<C>` and `From<A>`, `Into<C>` and `From<A>` should
be derived for `A` and `C`, respectively.

# Drawbacks
[drawbacks]: #drawbacks

None

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Have the user do this.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None
