- Start Date: 2015-01-02
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Feature gate the `item`, `stmt`, `pat`, `expr`, `ty`, and `path` fragment
specifiers in macros.

# Motivation

For `macro_rules to be in 1.0, we need to promise backwards compatability.
There are currently
[hazards](http://discuss.rust-lang.org/t/pre-rfc-macro-input-future-proofing/1089)
surrounding certain fragment specifiers and future (otherwise
backwards-compatible) changes to the Rust syntax. Feature gate these fragment
specifiers so more thought can be put into the design and operation of
`macro_rules`.

# Detailed design

As in the summary.  This leaves the `ident`, `lit`, `block`, `meta`, and `tt`
fragment specifiers.

`ident` and `lit` are trivial to allow. Since they are always single token,
there are no potential hazards surrounding future syntax changes.

`block` is allowable because its braces present a clear boundary that prevent
any syntax changes from incompatibly "leaking out".

`meta` is allowed, even though the syntax of attributes may, in the future, be
extended to arbitrary token trees. In that case, `meta` could also be extended
compatibly (`meta` is the fragment specifier for the stuff that goes inside of
the braces of an attribute).

`tt`, though in some ways the most complicated fragment specifier, is also in
many ways the simplest, since it is merely a single token, or a delimited
sequence of tokens.

# Drawbacks

Prevents using types as the inputs to macros, for now.

# Alternatives

See [the pre-RFC on
discuss](http://discuss.rust-lang.org/t/pre-rfc-macro-input-future-proofing/1089)
for a more complex alternative.

It's likely that this feature gate will be somewhat temporary, and serves only
to "buy time" to make `macro_rules` more solid.
