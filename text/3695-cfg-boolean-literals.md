- Feature Name: `cfg_boolean_literals`
- Start Date: 2024-09-16
- RFC PR: [rust-lang/rfcs#3695](https://github.com/rust-lang/rfcs/pull/3695)
- Tracking Issue: [rust-lang/rust#131204](https://github.com/rust-lang/rust/issues/131204)

# Summary
[summary]: #summary

Allow `true` and `false` boolean literals as `cfg` predicates, i.e. `cfg(true)`/`cfg(false)`.

# Motivation
[motivation]: #motivation

Often, we may want to temporarily disable a block of code while working on a project; this can be useful, for example, to disable functions which have errors while refactoring a codebase.

Currently, the easiest ways for programmers to do this are to comment out the code block (which means syntax highlighting no longer works), or to use `cfg(any())` (which is not explicit in meaning).

By allowing `#[cfg(false)]`, we can provide programmers with an explicit and more intuitive way to disable code, while retaining IDE functionality.

Allowing `cfg(true)` would also make temporarily enabling `cfg`'ed out code easier; a `true` may be added to a `cfg(any(..))` list. Adding a `cfg(all())` is the current equivalent of this.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Boolean literals (i.e. `true` and `false`) may be used as `cfg` predicates, to evaluate as always true/false respectively.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The syntax for configuration predicates should be extended to include boolean literals:

> **<sup>Syntax</sup>**\
> _ConfigurationPredicate_ :\
> &nbsp;&nbsp; &nbsp;&nbsp; _ConfigurationOption_\
> &nbsp;&nbsp; | _ConfigurationAll_\
> &nbsp;&nbsp; | _ConfigurationAny_\
> &nbsp;&nbsp; | _ConfigurationNot_ \
> &nbsp;&nbsp; | `true` | `false`

And the line
> - `true` or `false` literals, which are always `true`/`false` respectively

should be added to the explanation of the predicates.

`cfg(r#true)` and `cfg(r#false)` should continue to work as they did previously (i.e. enabled when `--cfg true`/`--cfg false` are passed).

`true` and `false` should be expected everywhere Configuration Predicates are used, i.e.
- the `#[cfg(..)]` attribute
- the `cfg!(..)` macro
- the `#[cfg_attr(.., ..)]` attribute

# Drawbacks
[drawbacks]: #drawbacks

By making it more convenient, this may encourage unconditionally disabled blocks of code being committed, which is undesirable.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- This could instead be spelled as `cfg(disabled|enabled)`, or `cfg(none)` for disabling code only. However, giving special meaning to a valid identifier will change the meaning of existing code, requiring a new edition
- As the existing predicates evaluate to booleans, using boolean literals is the most intuitive way to spell this

# Prior art
[prior-art]: #prior-art

Many languages with conditional compilation constructs have a way to disable a block entirely.

- C: `#if 0`
- C#: `#if false`
- Dlang: `version(none)`
- Haskell: `#if 0`

Searching for `cfg(false)` on [GitHub](https://github.com/search?q=%23%5Bcfg%28false%29%5D+language%3ARust&type=code) reveals many examples of projects (including Rust itself) using `cfg(FALSE)` as a way to get this behavior - although this raises a `check-cfg` warning.

# Future possibilities
[future-possibilities]: #future-possibilities

A future lint could suggest replacing constructs such as `cfg(any())` with `cfg(false)`, and `cfg(all())` with `cfg(true)`.

The `check-cfg` lint could be with a special case for identifiers such as `FALSE` and suggest `cfg(false)` instead.
