- Feature Name: N/A
- Start Date: June 1, 2018
- RFC PR:
- Rust Issue:
# Summary
[summary]: #summary

Allow non-ASCII identifiers, with strict checks to prevent confusion.

# Motivation
[motivation]: #motivation

One of the main use cases is Rust developers for whom English is not their native language.  This will allow them to use their native languages in identifiers.  This is very helpful, since identifiers often have significant semantic meaning.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Rust supports a subset of Unicode in identifiers.  Therefore,

```rust
use num::complex::Complex;

/// The tolerance
const ε: f64 = 0.000001;

/// Riemann zeta function
fn ζ(x: Complex) -> Complex {
   ...
}
```

There are restrictions on what can be put in identifiers, so the following is incorrect:

```rust
/// ERROR ☆ is not allowed in identifiers
const ☆: f64 = 0;
```

Additionally, Rust includes checks to make sure that no two identifiers can be confused with each other.  Specifically:

* It is a hard error if two identifiers can be confused with each other and are in scope at the same time.
* It is a hard error if identifiers are not in NFC.
* If two identifiers that can be confused with each other are present in the same source file, a warning is issued.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This RFC proposes that non-ASCII identifiers be stabilized, with the current set of allowed characters.  However, there are restrictions to avoid visual confusion:

* No two identifiers that are distinct but visually confusable may be in the same scope.

    This ensures that replacing an identifier with any identifier that it may be confused with *always* results in an error, unless the identifier was unused (in which case it cannot effect semantics).  For the purpose of this rule, `let` bindings do NOT introduce a new scope, to minimize confusion.

    I suspect that there is some mechanism to perform this check in less than O(N^2) time, though I am not aware of one.

* No identifier may be distinct from its NFC version

    This is to avoid confusion, and to ensure that editing a Rust source file does not cause problems.

* If two identifiers that can be confused with each other are present in the same source file, a warning is issued.

    This ensures that users do not get confusing error messages.  While I do not believe that it can be exploited to get past code review, it is still potentially confusing to users.

# Drawbacks
[drawbacks]: #drawbacks

This will require the compiler to be capable of performing checks for visual confusability on Unicode data.

# Rationale and alternatives
[alternatives]: #alternatives

1. We could remove the `non_ascii_idents` feature entirely, and restrict identifiers to ASCII.

    This is the approach that Pony takes.  I do not like it because it weakens the Rust language’s internationalization support.

2. We could stabilize the `non_ascii_idents` feature as-is.

    This renders Rust vulnerable to homograph attacks, which could be used to sneak code past code review.

# Prior art
[prior-art]: #prior-art

The motivation for the specific checks comes from a preprocessor for OCaml (whose name currently escapes me) that allows OCaml code to contain Unicode identifiers.

# Unresolved questions
[unresolved]: #unresolved-questions

- How can the confusability check be implemented?  We should have a fast path when there are *no* non-ASCII identifiers, but we also must be fast (at worst, O(N log N)) when there *are* non-ASCII identifiers.
- Should the warning for confusable identifiers in different scopes be made a hard error?  Should it be removed entirely?  Should it be a configurable lint?
