- Feature Name: associated_size
- Start Date: 2015-06-22
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Provide the size and alignment of a type as associated constants.

# Motivation

Constants and array bounds can only contain constants, which currently does not
include sizes of types. By making them associated constants they become more
versatile and can e. g. be used in `const fn`s.

# Detailed design

Extend the `Sized` trait by two associated constants `SIZE` and `ALIGNMENT`,
which contain the size of the type in bytes, and the preferred alignment,
respectively.

# Drawbacks

`SIZE` and `ALIGNMENT` (names still up to debate) are going to be members of
every type, as such they are a slight backward incompatiblity.

# Alternatives

An alternative name for `ALIGNMENT` would be `ALIGN`, also, one could consider
to use snake case for associated constants, which would result in `size` and
`align`.

Additionally, one could keep the current design and make these values available
via freestanding `const fn`s. However in the past, Rust tried to avoid using
free functions instead of member functions, or in this case, associated
constants.

# Unresolved questions

None.
