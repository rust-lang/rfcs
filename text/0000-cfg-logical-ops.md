- Feature Name: `cfg_logical_ops`
- Start Date: 2025-03-30
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

`#[cfg]`, `#[cfg_attr]`, and `cfg!()` can use `&&`, `||`, and `!` for `all`, `any`, and `not`,
respectively.

# Motivation
[motivation]: #motivation

While there are no technical restrictions to using logical operators, this was not always the case.
In Rust 1.0, attributes could not contain arbitrary tokens. This restriction was lifted in Rust
1.34, but the `cfg` syntax was not updated to take advantage of this. By letting developers use
logical operators, we are _lessening_ the burden of having to remember the `cfg` syntax.

# Explanation
[explanation]: #explanation
[cfg-syntax]: https://doc.rust-lang.org/reference/conditional-compilation.html#r-cfg.syntax

`#[cfg(foo && bar)]` enables the annotated code if and only if both `foo` **and** `bar` are enabled.
Similarly, `#[cfg(foo || bar)]` enables the annotated code if and only if either `foo` **or** `bar`
is enabled. Finally, `#[cfg(!foo)]` enables the annotated code if and only if `foo` is **not**
enabled. `#[cfg_attr]` and `cfg!()` behave the same way. Precedence is the same as in expressions.

In terms of formal syntax, the [`[cfg.syntax]`][cfg-syntax] is changed to the following:

> **<sup>Syntax</sup>**\
> _ConfigurationPredicate_ :\
> &nbsp;&nbsp; &nbsp;&nbsp; _ConfigurationOption_\
> &nbsp;&nbsp; | _ConfigurationAll_\
> &nbsp;&nbsp; | _ConfigurationAny_\
> &nbsp;&nbsp; | _ConfigurationNot_\
> &nbsp;&nbsp; | _ConfigurationAnd_\
> &nbsp;&nbsp; | _ConfigurationOr_\
> &nbsp;&nbsp; | _ConfigurationNotOption_\
> &nbsp;&nbsp; | `(` _ConfigurationPredicate_ `)`
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
> _ConfigurationNotOption_\
> &nbsp;&nbsp; `!` _ConfigurationOption_
>
> _ConfigurationPredicateList_\
> &nbsp;&nbsp; _ConfigurationPredicate_ (`,` _ConfigurationPredicate_)<sup>\*</sup> `,`<sup>?</sup>

# Drawbacks
[drawbacks]: #drawbacks

- Two ways to express the same thing. This can be somewhat mitigated by a lint for the old syntax.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- The current syntax is verbose and a relic of the past when attributes could not contain arbitrary
  tokens.
- Using existing, widely-understood operators makes the syntax more familiar.

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
  `#[cfg(feature = "foo" || feature = "bar")]`. This would be particularly useful for
  platform-specific code (e.g. `#[cfg(target_os = "linux" | "windows")]`).
