- Start Date: 2014-04-07
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Add a way to let imports "trickle down" into child modules.

# Motivation

When using non-std crates, and ubiquitous data structures, it can becomes
annoying to have to repeat the same imports everywhere.

# Detailed design

Add a keyword, `inherit`. When `inherit use path;` is encountered, `path` is
considered in-scope for not only the current module, but also all child
modules. For example, an `inherit use syntax::ast;` in the crate root of
`rustc` would make the `ast` module available everywhere.

# Alternatives

[RFC #37](https://github.com/rust-lang/rfcs/pull/37) makes this somewhat less
pressing, as it makes many imports of traits unnecessary.
