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
ATTR       = '@' [!] META
           | '@' [!] '[' META ']'
META       = ID
           | ID '(' META_SEQ ')'
META_SEQ   = META_ITEM {',' META_ITEM}
META_ITEM  = META
           | ID '=' STRING_LITERAL
```

Here are some examples of legal syntax:

* `@inline`
* `@!inline`
* `@[inline]`
* `@deprecated(reason = "foo")`
* `@deriving(Eq)`

Note that some attributes which are legal today have no equivalent:

* `#[deprecated = "foo"]` becomes `@deprecated(reason = "foo")`

`[]` delimiters are allowed but not required. They are necessary to avoid
mis-parses in certain situations:
```rust
match (foo, bar) {
    @thing
    (a, b) => { ...}
    ...
}
```
This will parse as the attribute `@thing(a, b)`, which will in turn result in
a syntax error. `[]` delimiters will resolve the ambiguity:
```rust
match (foo, bar) {
    @[thing]
    (a, b) => { ...}
    ...
}
```

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

The need to continue to allow `[]` delimiters is a bit unfortunate.

# Alternatives

We can punt on this until after 1.0. `@`-attributes and `#`-attributes will
have to coexist to avoid breaking backwards compatibility, but that won't be
all that hard to deal with.

We can leave the syntax as is, which is also not that bad.

We could use `()` as the optional delimiters instead of `[]`.

Support for `#[deprecated = "reason"]` style attributes is removed because
`@deprecated = "reason"` is a bit visually confusing since there are no
delimiters wrapping the attribute. There are a couple of alternatives here.
One is to just allow that syntax. It's not grammatically ambiguous, after all.

Another is to change `foo = "bar"` to `foo("bar")`:
```
ATTR     = '@' [!] META
META     = ID
         | ID '(' STRING_LITERAL ')
         | ID '(' META {',' META} ')'
```

For example:
* `@deprecated("foo")`
* `@cfg(all(target_os("linux"), feature("my_cargo_feature")))`

This allows for a more convenient syntax for deprecation, but does add even
more parentheses to attribute invocations like the `cfg` example above.

# Unresolved questions

Should there be a "deprecation period" where the `#` syntax is accepted with a
warning?
