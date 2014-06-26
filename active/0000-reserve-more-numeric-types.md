- Start Date: 2014-06-25
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Reserve more numeric types as keywords.

# Motivation

It is conceivable that Rust will gain support for types such as `f128`, `f16`,
or `u128`. In the interest of backwards compatability, extend the grammar to
reserve these.

# Detailed design

The `INT_SUFFIX` and `FLOAT_SUFFIX` nonterminals in [the lexical
grammar](0021-lexical-syntax-simplification.md) respectively become:

```
INT_SUFFIX : 'i' [0-9]+ ;
FLOAT_SUFFIX : 'f' [0-9]+ ;
```

Additionally, identifiers matching those patterns will be *reserved words*,
not legal for use in bindings or otherwise.

# Drawbacks

Makes the grammar larger for types which we may never use.
