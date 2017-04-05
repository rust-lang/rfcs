- Start Date: 2015-01-14
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Remove the `proc` keyword from the language.

# Motivation

With the removal of the proc construct, the keyword has no use currently.
Freeing it would allow using `proc` as an identifier name.

# Detailed design

Remove the keyword status of `proc`, allowing it to be treated as an
identifier.

# Drawbacks

We might want to use the `proc` keyword for something in the future.
Reintroducing it as a keyword would be a backwards-incompatible change.

# Alternatives

Keep `proc` as a reserved keyword.

# Unresolved questions

Why wasn't the `proc` keyword removed along with the proc construct?
Were there any plans with it?
