- Start Date: 2014-11-26
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)


# Summary

Make brackets in attributes optional for single attributes using the `#attr` or `#attr(arg)` syntax. They remain mandatory for attributes using the assignment `#[attr = arg]` syntax, and may also be used to specify multiple attributes at once: `#[attr1, attr2, attr3]`, as today, although this is equivalent to `#attr1 #attr2 #attr3`, which may be preferred in practice.

This is fully backwards compatible: all existing syntax remains legal, with the same meaning.


# Motivation

The brackets are visual clutter, syntactic noise, and serve no real purpose.

`#attribute` looks like a hashtag, and is a good mnemonic for metadata, which attributes and hashtags both are.

Note that this is purely an incremental change, and does *not* preclude subsequent further changes (such as removing the `=` form entirely, should we want to).


# Detailed design

Refer to *Summary*.


# Drawbacks

None!


# Alternatives

[RFC 483](https://github.com/rust-lang/rfcs/pull/483): Use `@` instead.


# Unresolved questions

None.
