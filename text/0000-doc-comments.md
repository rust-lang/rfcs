- Feature Name: N/A
- Start Date: 2015-11-17
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

Remove some kinds of doc comments.

See also #287, #1371, and rust-lang/rust#6782.

# Motivation
[motivation]: #motivation

There are too many kinds of doc comments, they are confusing for users and tools.

# Detailed design
[design]: #detailed-design

Remove doc comments as attributes and block doc comments, leaving line doc comments. I.e., remove `#[doc=""]`, `#![doc=""]`, `/** */`, and `/*! */`. Keep `///` and `//!`.

Note that doc block comments would still be valid block comments, so code will continue to compile in this case (although obviously it won't play well with rustdoc). The attributes would give unknown attribute errors.

Since this would be a breaking change, I propose that the removed comments become deprecated with intention to remove in Rust 2.0.

We can provide a tool (based on Rustfmt) to automatically convert all invalid doc comments to valid ones.


# Drawbacks
[drawbacks]: #drawbacks

Less flexibility. Some people like block comments.

# Alternatives
[alternatives]: #alternatives

I would like to also remove inner line comments, but I think this is not possible because we want to allow documenting crates and it makes less sense for modules. An alternative syntax would be to only support `///`, but for the first doc comment in a module to have special meaning as an internal comment for that module. That has problems of its own, of course (e.g., if you don't want a doc comment for the module, but you do for the first item in the module).

# Unresolved questions
[unresolved]: #unresolved-questions

Should deprecation warnings be issued by the compiler or rustdoc or both? For attributes, the compiler needs to be aware, but since block doc comments remain valid comments, an argument could be made that the compiler shouldn't care. On the other hand, users are unlikely to pay as much attention to warnings issued by rustdoc as to warnings issued by the compiler. Internally, the compiler also treats doc comments differently from normal comments, so it is affected.
