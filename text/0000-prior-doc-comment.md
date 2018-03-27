- Feature Name: prior_doc_comment
- Start Date: 2018-03-26
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Allow `//!` (also called "outer doc comments" below) to be used to annotate the block just prior to them.

# Motivation
[motivation]: #motivation

This usecase, where the doc comment is put after an enum variant, is likely to be most widely used:

```rust
enum E {
    /// doc comments used currently
    A,
    B, //! doc comments with this RFC
    C,
}
```

There are also people who prefer documentation after member. See [1](https://capnproto.org/language.html#comments) [2](https://internals.rust-lang.org/t/any-interest-in-same-line-doc-comments/3212).

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

`//!` can be used to document the enclosing block, and with this RFC can be used to document the prior block.

For example:

```
enum Option<T> {
    //! The `Option` type. See [the module level documentation](index.html) for more.
    None, //! No value
    Some(T), //! Some value `T`
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

`//!` Documentation comments are now allowed after the identifier, and can be separated by a combination of whitespace or newlines.

When there are both `///` and `//!` blocks, the doc comments are concatenated in order.

It will be valid to interleave the two style of comments, although this is not a thing we want to do and we may issue an warning:

```
enum E {
    //! A
    /// B
    //! C
    /// D
    X,
    //! E
}
```

The documentation for `enum E` will be:
```
A
C
```

and the documentation for `E::X` will be:
```
B
D
E
```

# Drawbacks
[drawbacks]: #drawbacks

Rust have limited doc comment syntax for the sake of simplicity. While this RFC doesn't propose any new syntax, this may complicate the learning of Rust.

# Rationale and alternatives
[alternatives]: #alternatives

TBD. This RFC is designed to be very conservative.

# Prior art
[prior-art]: #prior-art

As mentioned in [motivation], [Cap'n Proto](https://capnproto.org/language.html#comments) has been recommending this as default
and [Doxygen](http://www.stack.nl/~dimitri/doxygen/manual/docblocks.html#memberdoc) has the support of such comments.

# Unresolved questions
[unresolved]: #unresolved-questions

TBD
