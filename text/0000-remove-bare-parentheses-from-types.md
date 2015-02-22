- Feature Name: N/A
- Start Date: 2015-02-24
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

Summary
=======

Remove the `(TYPE)` syntax from the syntax for types in favour of the existing
`<TYPE>` syntax that was introduced as a by-product of UFCS.

Motivation
==========

An often-desired addition to Rust’s type system is [constant value
parameterisation][884]. This feature would allow parameterising types,
functions, and traits by constant values (allowing types like `SmallVec<i32,
4>`). Unfortunately, allowing values in type position would be tricky to parse
in complex cases, so a syntax for disambiguating between complex constants and
types is needed. A syntax like `{EXPR}` could be used, as proposed in [RFC
884][884], but that syntax could be hard to discover and seem rather ugly. The
obvious syntax to use for this is `(EXPR)`, but that clashes with the existing
`(TYPE)` syntax, and thus doesn’t help resolve the ambiguity much. To leave room
for such a feature in the future, this RFC proposes to remove the `(TYPE)`
syntax.

Additionally, the `(TYPE)` syntax is redundant, as an equivalent is also
(theoretically – it has not been implemented yet) available in the form of
`<TYPE>`, part of [UFCS, RFC 132][132_TYPE_SEGMENT]:

> When a path begins with a `TYPE_SEGMENT`, it is a type-relative path. *If this
> is the complete path (e.g., `<int>`), then the path resolves to the specified
> type.*

Furthermore, angle brackets can be considered the ‘type equivalent’ of
parentheses: the type `Foo<Bar>` is similar in concept to the expression
`foo(bar)`, and uses angle brackets instead of parentheses. Thus, it is natural
to extend this symmetry to the `(TYPE)` syntax.

Detailed design
===============

Remove the `(TYPE)` syntax from the syntax from types. Instead, request using
the syntax `<TYPE>`, which is already valid according to [RFC
132][132_TYPE_SEGMENT].

Drawbacks
=========

Removes an arguably nice-looking syntax in favour of one that could be
considered uglier and has not yet been implemented. Fortunately, this syntax is
little-used outside specifying lifetimes on trait objects behind references
(e.g., `&(Trait + 'a)`, which would become `&<Trait + 'a>`).

Alternatives
============

Choose some other syntax to replace `(TYPE)`. Even if another syntax is chosen,
`<TYPE>` will still be valid unless an amendment is also made to the UFCS RFC.

Unresolved questions
====================

None, yet.

[884]: https://github.com/rust-lang/rfcs/pull/884
[132_TYPE_SEGMENT]: https://github.com/rust-lang/rfcs/blob/master/text/0132-ufcs.md#paths-that-begin-with-a-type_segment
