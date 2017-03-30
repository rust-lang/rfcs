- Start Date: (2014-12-22)
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

This RFCs goal is to remove block comments, which are currently in disregard throughout the rust standard coding conventions.

# Motivation

Block comments are rarely used in Rust code, and use of these multi-line comments are discouraged throughout many common conventions.
Removing block comments will emphasize use of single line comments, and will lower the surface area of non-idiomatic Rust code.

Removing them is also low impact, because they are used so little throughout regular Rust code. 
Block comments are not even mentioned in the Rust Guide, so it can be assumed that many Rust users do not know block comments exist.
They can also be added back to the language in a backwards compatible fashion if decided that they are of use to Rust coders.

This change also removes two of the kinds of doc comments from [issue #287](https://github.com/rust-lang/rfcs/issues/287), and simplifies the language.

# Detailed design

Removal of the comments themselves from the Rust code base should be simple, as they are used so little.

# Drawbacks

Removal of a common feature of other C-like languages may be strange to newcomers. However, it may be bset to enforce idiomatic writing of Rust code over usage of code idioms that are unidiomatic to Rust.

# Alternatives

Leave block comments in, changing nothing. If we do this, we should document them throughout Rust texts such as The Guide and Rust By Example.

# Unresolved questions

None