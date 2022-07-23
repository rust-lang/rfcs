- Feature Name: `dotdot_patterns`
- Start Date: 2019-06-01
- RFC PR: [rust-lang/rfcs#2707](https://github.com/rust-lang/rfcs/pull/2707)
- Rust Issue: [rust-lang/rust#62254](https://github.com/rust-lang/rust/issues/62254)

# Summary
[summary]: #summary

Make `..` a pattern rather than a syntactic fragment of some other patterns.

# Motivation
[motivation]: #motivation

The change simplifies pattern grammar and simplifies use of `..` in macros.  
In particular, the `pat` macro matcher will now accept `..` and `IDENT @ ..`.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

`..` becomes a pattern syntactically.
The notable consequences of this are listed below.

- `pat` macro matcher will now accept `..` and more complex pattern containing `..`,
for example `ref x @ ..`.

- A trailing comma is accepted after `..` in tuple struct, tuple or slice pattern.
```rust
Variant(a, b, ..,) // OK
```

- Some nonsensical code can now be accepted under `cfg(FALSE)`.
```rust
#[cfg(FALSE)]
Tuple(.., a, ..) // OK
```

`..` in "inappropriate" positions is still rejected semantically.
```rust
let .. = 10; // Semantic error, `..` is not a part of a "list" pattern
let Option(.., ..) = 11; // Semantic error, multiple `..`s in a single "list" pattern
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Pattern grammar is extended with a new production
```
PAT = ..
```
Special productions allowing `..` in tuple struct, tuple and slice patterns are subsumed by this
new production and removed.

Semantically, the `..` pattern is accepted
- Immediately inside a tuple struct/variant pattern `Tuple(PAT, .., PAT)`
- Immediately inside a tuple pattern `(PAT, .., PAT)`
- Immediately inside a slice pattern `[PAT, .., PAT]`.
- Immediately inside a binding pattern inside a slice pattern `[PAT, BINDING @ .., PAT]`.

An error is produced if this pattern is used in any other position.

An error is produced if more that one `..` or `BINDING @ ..` pattern is used inside its containing
tuple struct / tuple / slice pattern.

`(..)` is still a tuple pattern and not a parenthesized `..` pattern for backward compatibility.

Note that `..` in struct patterns
```rust
Struct { field1: PAT, field2, .. }
```
is still not a pattern, but a fragment of a struct pattern syntax.

# Drawbacks
[drawbacks]: #drawbacks

More meaningless code may be accepted under `cfg(FALSE)` where semantic checks are not performed.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

See "Motivation" for the rationale.  
Status quo is always an alternative.

# Prior art
[prior-art]: #prior-art

This RFC is a follow up to https://github.com/rust-lang/rfcs/pull/2359.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None so far.

# Future possibilities
[future-possibilities]: #future-possibilities

Accept `BINDING @ ..` in tuple patterns, `(head, tail @ ..)`.
