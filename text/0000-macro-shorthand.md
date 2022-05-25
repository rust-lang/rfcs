- Feature Name: `macro_shorthand`
- Start Date: 2022-05-18
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This is a proposal for `m!literal` macro invocation syntax,
for macros that feel like literals:

```rust
    let num = bignum!12345678901234567890123456789012345678901234567890;
    let msg = f!"n = {num}";
    let file = w!"C:\\Thing";
```

# Motivation
[motivation]: #motivation

In the Rust 2021 edition we reserved all prefixes for literals, so we can give them a meaning in the future.
However, many ideas for specific literal prefixes (e.g. for wide strings or bignums) are domain- or crate-specific,
and should arguably not be a builtin part of the language itself.

By making `m!literal` a way to invoke a macro, we get a syntax that's just as convenient and light-weight as built-in prefixes,
but through a mechanism that allows them to be user-defined, without any extra language features necessary to define them.

For example:

- Windows crates could provide wide strings using `w!"C:\\"`
- An arbitrary precision number crate could provide `bignum!12345678901234567890123456789012345678901234567890`.
- Those who want "f-strings" can then simply do `use std::format as f;` and then use `f!"{a} {b}"`.

The difference with `f!("{a} {b}")`, `w!("C:\\")` and `bignum!(123...890)` is small, but significant.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Macros can be invoked using `m!(..)`, `m![..]`, `m!{..}` or `m!..` syntax.
In the last case, the argument must be a single literal, such as `m!123`, `m!2.1`, `m!"abc"`, or `m!'x'`.
From the perspective of a macro definition, these are all identical, and a macro cannot differentiate between the different call syntaxes.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The macro invocation syntax is changed from

```
MacroInvocation :
   SimplePath ! DelimTokenTree

MacroInvocationSemi :
     SimplePath ! ( TokenTree* ) ;
   | SimplePath ! [ TokenTree* ] ;
   | SimplePath ! { TokenTree* }
```

to

```
MacroInvocation :
     SimplePath ! Literal
   | SimplePath ! DelimTokenTree

MacroInvocationSemi :
     SimplePath ! Literal ;
   | SimplePath ! ( TokenTree* ) ;
   | SimplePath ! [ TokenTree* ] ;
   | SimplePath ! { TokenTree* }
```

# Drawbacks
[drawbacks]: #drawbacks

- It allows for confusing syntax like `vec!1` for `vec![1]`.
  - Counter-argument: we already allow `vec!(1)`, `println! { "" }` and `thread_local![]`, which also don't cause any problems.
    (Rustfmt even corrects the first one to use square brackets instead.)

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- Expect those macros to be used with `m!(..)` syntax.
  - That's already possible today, but plenty of people are asking for things
    like `f""` or `w""`, which shows that `f!("")` does not suffice.
- Have a separate mechanism for defining custom prefixes or suffixes.
  - E.g. `10.4_cm`, which is possible in C++ through `operator""`.
  - This requires a seprate mechanism, which complicates the language significantly.
- Require macros to declare when they can be called using this syntax.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Should we allow `m!b"abc"` and `m!b'x'`? (I think yes.)
- Should we allow `m!r"..."`? (I think yes.)
- Should we allow `m!123i64`? (I think yes.)
- Should we allow `m!-123`? (I'm unsure. Technically the `-` is a separate token. Could be a future addition.)

# Future possibilities
[future-possibilities]: #future-possibilities

In the future, we could consider extending this syntax in a backwards compatible way by allowing
slightly more kinds of arguments to be used without brackets, such as `m!-123` or `m!identifier`, or even `m!|| { .. }` or `m!struct X {}`.
(That might be a bad idea, which is why I'm not proposing it as part of this RFC.)
