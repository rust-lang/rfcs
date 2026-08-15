- Feature Name: `cfg_logical_ops`
- Start Date: 2025-03-30
- RFC PR: [rust-lang/rfcs#3796](https://github.com/rust-lang/rfcs/pull/3796)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

`#[cfg]`, `#[cfg_attr]`, and `cfg!()` can use `&&`, `||`, and `!` for `all`, `any`, and `not`,
respectively. Due to precedence, `feature = "foo"` must be parenthesized when adjoining any of the
new operators.

# Motivation
[motivation]: #motivation

While there are no technical restrictions to using logical operators, this was not always the case.
In Rust 1.0, attributes could not contain arbitrary tokens. This restriction was lifted in Rust
1.34, but the `cfg` syntax was not updated to take advantage of this. By letting developers use
logical operators, we are _lessening_ the burden of having to remember the `cfg` syntax.

# Explanation
[explanation]: #explanation
[cfg-syntax]: https://doc.rust-lang.org/reference/conditional-compilation.html#r-cfg.syntax
[precedence]: https://doc.rust-lang.org/reference/expressions.html#expression-precedence

`#[cfg(foo && bar)]` enables the annotated code if and only if both `foo` **and** `bar` are enabled.
Similarly, `#[cfg(foo || bar)]` enables the annotated code if and only if either `foo` **or** `bar`
is enabled. Finally, `#[cfg(!foo)]` enables the annotated code if and only if `foo` is **not**
enabled. `#[cfg_attr]` and `cfg!()` behave the same way.

Precedence is the [same as in expressions][precedence], which necessitates using parentheses in some
situations. This is shown in the table below. `=` is **not** treated the same as `==` for precedence
purposes; it has lower precedence than all logical operators.

## Examples

| Syntax                                 | Equivalent to                                      | Rationale                                                                            |
| -------------------------------------- | -------------------------------------------------- | ------------------------------------------------------------------------------------ |
| `a && b`                               | `all(a, b)`                                        | definition of `&&`                                                                   |
| `a \|\| b`                             | `any(a, b)`                                        | definition of `\|\|`                                                                 |
| `!a`                                   | `not(a)`                                           | definition of `!`                                                                    |
| `(a)`                                  | `a`                                                | definition of `()`                                                                   |
| `a && b && c && d`                     | `all(a, b, c, d)` (or `all(all(all(a, b), c), d)`) | `&&` is associative                                                                  |
| `a \|\| b \|\| c \|\| d`               | `any(a, b, c, d)` (or `any(any(any(a, b), c), d)`) | `\|\|` is associative                                                                |
| `!!!!!!a`                              | `not(not(not(not(not(not(a))))))`                  | `!` can be repeated                                                                  |
| `((((((a))))))`                        | a                                                  | `()` can be nested                                                                   |
| `a && b \|\| c && d`                   | `any(all(a, b), all(c, d))`                        | `\|\|` has lower precedence than `&&`                                                |
| `a \|\| b && c \|\| d`                 | `any(a, all(b, c), d)`                             | `\|\|` has lower precedence than `&&`                                                |
| `(a \|\| b) && (c \|\| d)`             | `all(any(a, b), any(c, d))`                        | `()` can be used for grouping                                                        |
| `!a \|\| !b && !c`                     | `any(not(a), all(not(b), not(c)))`                 | `!` has highest precedence                                                           |
| `feature="foo" \|\| feature="bar"`     | _syntax error_                                     | `\|\|` has higher precedence than `=`, which may be confusing, so we ban this syntax |
| `(feature="foo") \|\| (feature="bar")` | `any(feature="foo", feature="bar")`                | use `()` for grouping                                                                |
| `feature="foo" && feature="bar"`       | _syntax error_                                     | `&&` has higher precedence than `=`, which may be confusing, so we ban this syntax   |
| `(feature="foo") && (feature="bar")`   | `all(feature="foo", feature="bar")`                | use `()` for grouping                                                                |
| `!feature="foo"`                       | _syntax error_                                     | `!` has higher precedence than `=`, which may be confusing, so we ban this syntax    |
| `!(feature="foo")`                     | `not(feature="foo")`                               | use `()` for grouping                                                                |
| `!all(x, y)`                           | `not(all(x, y))`                                   | `!` has lower precedence than "function call"                                        |
| `any(!x \|\| !w, !(y && z))`           | `any(any(not(x), not(w)), not(all(y, z)))`         | `!`, `&&` etc. can be used inside `any`, `all` and `not`                             |
| `true && !false`                       | `all(true, not(false))`                            | `!`, `&&` etc. can be used on boolean literals (they are syntactically identifiers)  |
| `!accessible(std::mem::forget)`        | `not(accessible(std::mem::forget))`                | `!`, `&&` etc. can be used on `cfg_accessible`                                       |
| `accessible(std::a \|\| std::b)`       | _syntax error_                                     | … but not inside                                                                     |
| `!version("1.42.0")`                   | `not(version("1.42.0"))`                           | `!`, `&&` etc. can be used on `cfg_version`                                          |
| `version(!"1.42.0")`                   | _syntax error_                                     | … but not inside                                                                     |

## Formal syntax

[`[cfg.syntax]`][cfg-syntax] is changed to the following:

> **<sup>Syntax</sup>**\
> _ConfigurationPredicate_ :\
> &nbsp;&nbsp; &nbsp;&nbsp; _ConfigurationOption_\
> &nbsp;&nbsp; | _ConfigurationAll_\
> &nbsp;&nbsp; | _ConfigurationAny_\
> &nbsp;&nbsp; | _ConfigurationNot_\
> &nbsp;&nbsp; | _ConfigurationAnd_\
> &nbsp;&nbsp; | _ConfigurationOr_\
> &nbsp;&nbsp; | _ConfigurationNegation_\
> &nbsp;&nbsp; | `(` _ConfigurationPredicate_ `)`
>
> _ConfigurationNegatable_ :\
> &nbsp;&nbsp; _ConfigurationOptionIdent_\
> &nbsp;&nbsp; | _ConfigurationAll_\
> &nbsp;&nbsp; | _ConfigurationAny_\
> &nbsp;&nbsp; | _ConfigurationNot_\
> &nbsp;&nbsp; | _ConfigurationNegation_ \
> &nbsp;&nbsp; | `(` _ConfigurationPredicate_ `)`
>
> _ConfigurationOptionIdent_ :\
> &nbsp;&nbsp; [IDENTIFIER]
>
> _ConfigurationOption_ :\
> &nbsp;&nbsp; [IDENTIFIER]&nbsp;(`=` ([STRING_LITERAL] | [RAW_STRING_LITERAL]))<sup>?</sup>
>
> _ConfigurationAll_\
> &nbsp;&nbsp; `all` `(` _ConfigurationPredicateList_<sup>?</sup> `)`
>
> _ConfigurationAny_\
> &nbsp;&nbsp; `any` `(` _ConfigurationPredicateList_<sup>?</sup> `)`
>
> _ConfigurationNot_\
> &nbsp;&nbsp; (`not` | `!`) `(` _ConfigurationPredicate_ `)`
>
> _ConfigurationAnd_\
> &nbsp;&nbsp; _ConfigurationPredicate_ `&&` _ConfigurationPredicate_
>
> _ConfigurationOr_\
> &nbsp;&nbsp; _ConfigurationPredicate_ `||` _ConfigurationPredicate_
>
> _ConfigurationNegation_\
> &nbsp;&nbsp; `!` _ConfigurationNegatable_
>
> _ConfigurationPredicateList_\
> &nbsp;&nbsp; _ConfigurationPredicate_ (`,` _ConfigurationPredicate_)<sup>\*</sup> `,`<sup>?</sup>

All future function-like predicates (such as `version` and `accessible`) should be added to
_ConfigurationNegatable_.

# Drawbacks
[drawbacks]: #drawbacks

- Two ways to express the same thing. This can be somewhat mitigated by a lint for the old syntax.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- The current syntax is verbose and a relic of the past when attributes could not contain arbitrary
  tokens.
- Using existing, widely-understood operators makes the syntax more familiar.
- `&` and `|` could be used instead of `&&` and `||`. Short-circuiting behavior is unobservable in
  this context, so the behavior would be the same.
- `feature != "foo"` could be allowed as shorthand for `!(feature = "foo")`. This could plausibly be
  interpreted as "any feature except 'foo'", which is why it is not included in this proposal.

# Prior art
[prior-art]: #prior-art

The `efg` crate is nearly identical to this proposal, the sole difference being not requiring `=`
for key-value pairs.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None so far.

# Future possibilities
[future-possibilities]: #future-possibilities

- Pattern-like syntax such as `#[cfg(feature = "foo" | "bar")]` could be allowed as a shorthand for
  `#[cfg((feature = "foo") || (feature = "bar"))]`. This would be particularly useful for
  platform-specific code (e.g. `#[cfg(target_os = "linux" | "windows")]`).
- The use of parentheses could be relaxed in some situations, such as allowing `!feature = "foo"` or
  `feature = "foo" || feature = "bar"`.
- A different syntax for key-value pairs, such as `feature("foo")`, could be used for clarity (as it
  is neither assignment nor equality) and to reduce the need for manual precedence management.
