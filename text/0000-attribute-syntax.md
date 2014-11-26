- Start Date: 2014-11-25
- RFC PR:
- Rust Issue:

# Summary

Change attribute syntax from `#[foo]` and `#![foo]` to `@foo` and `@!foo`.

# Motivation

Other languages like Java and Python use `@` for their equivalents of
attributes. In addition, `@foo` is easier to type than `#[foo]`, and is
arguably less noisy looking.

This change was proposed as part of [RFC
386](https://github.com/rust-lang/rfcs/pull/386/files) and was generally
well-received.

# Detailed design

Attributes and inner attributes would be written in one of the following forms
(BNF):

```
ATTR     = '@' [!] META
META     = ID
         | ID '(' STRING_LITERAL ')
         | ID '(' META {',' META} ')'
```

Here are some examples of legal syntax:

* `@inline`
* `@!inline`
* `@inline()`
* `@deprecated("foo")`
* `@deriving(Eq)`
* `@cfg(all(target_os("linux"), feature("my_cargo_feature")))`

Note that the old `foo = "bar"` syntax is now `foo("bar")`. Since the `[]`
delimiters are no longer present, `@foo = "bar"` is a bit confusing to visually
parse (though it is not grammatically ambiguous).

The removal of `[]` delimiters do pose an ambiguity problem in some situations:
```rust
match (foo, bar) {
    @thing
    (a, b) => { ...}
    ...
}
```
This will parse as the attribute `@thing(a, b)`, which will in turn result in
a syntax error. A pair of parentheses can be added to resolve the ambiguity:
```rust
match (foo, bar) {
    @thing()
    (a, b) => { ...}
    ...
}
```
This workaround is similar to the `box` syntax: `box () (foo, bar)`.

To avoid confusing syntax extensions, the internal AST representation of
`@foo` and `@foo()` should be made the same.

## Implementation

The parser will be adjusted to accept `@`-attributes in addition to current
attributes. The internal data structures will remain the same. Once a snapshot
lands, the Rust codebase can be converted, and parsing support for
`#`-attributes can be removed with an error message added explaining how to fix
code.

# Drawbacks

It's a large change that will cause a ton of churn very close to 1.0. Since the
only compiler changes required will be to the parser and pretty printer, it's
relatively low risk (compared to resolve or typeck changes for example).

The need to add parentheses to avoid ambiguities in some situations is a bit
unfortunate.

The fact that `@foo(bar, baz)` and `@foo("bar")` are legal but `@foo("bar",
"baz")` isn't is a bit weird.

# Alternatives

We can punt on this until after 1.0. `@`-attributes and `#`-attributes will
have to coexist to avoid breaking backwards compatibility, but that won't be
all that hard to deal with.

We can leave the syntax as is, which is also not that bad.

We could resolve the ambiguity problem by allowing, but not requiring, the use
of `[]` (or `()`) delimiters.

We can avoid the readability issues with `foo = "bar"` meta items by simply
forbidding them at the top level of an attribute, that is, `@foo = "bar"` would
not be allowed but `@foo(bar = "baz")` would. This would make things like
deprecation attributes a bit more verbose - `@deprecated(reason = "foo")`
instead of `@deprecated("foo")`.

# Unresolved questions

Should there be a "deprecation period" where the `#` syntax is accepted with a
warning?
