- Feature Name: `format_dynamic_fill`
- Start Date: 2023-02-23
- RFC PR: [rust-lang/rfcs#3394](https://github.com/rust-lang/rfcs/pull/3394)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Allow specifying the *fill* character of a format string, matching behavior of *width* and *precision*.

# Motivation
[motivation]: #motivation

To allow changing the *fill* character at run-time. Due to not being able to create or modify [`Formatter`](https://doc.rust-lang.org/std/fmt/struct.Formatter.html), this is the only way to allow this.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Extending on [`std::fmt`#Fill/Alignment](https://doc.rust-lang.org/std/fmt/index.html#fillalignment):

The value for the fill can also be provided as a [`char`](https://doc.rust-lang.org/std/primitive.char.html) in the list of parameters by adding a postfix `$` indicating that the second argument is a char specifying the fill.

Referring to an argument with the dollar syntax does not affect the “next argument” counter, so it’s usually a good idea to refer to arguments by position, or use named arguments.

```rs
assert_eq!(format!("Hello {:fill$>5}!", "x", fill='+'), "Hello x++++!");
assert_eq!(format!("Hello {1:0$<5}!", '-', "x"),        "Hello x----!");
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Implementation should be matching that of width and precision.

The fill and alignment is the first specified format parameter, so if the `:` is followed by:
- `"{:-^}"` a single character *c* (this includes `$`) followed by an *alignment* ⇒ the fill is *c*
- `"{:ident$>}"` a valid rust identifier *ident* followed by a `$` and an *alignment* ⇒ the fill is the named argument/variable *ident*
- `"{:1$<}"` an usize *n* followed by a `$` and an *alignment* ⇒ the fill is the *n*th argument (this does not affect the "next argument" counter matching [width](https://doc.rust-lang.org/std/fmt/index.html#width))
- `"{:width$}"` a valid rust identifier or usize followed by a `$` and not followed by an *alignment* ⇒ this is the width

(*alignment* is one of `<^>`)

# Drawbacks
[drawbacks]: #drawbacks

It complicates the format spec, and it is unclear how much it might be used.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- If enforcing a minimum of 2 characters the `$` could be omitted, but this only makes this easier to mess up and is unnecessarily restrictive.
- This can only be implemented in rust as it is impossible to drive the `Display` family traits on stable without going through the format string.

# Prior art
[prior-art]: #prior-art

The implementation of width and precision.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

# Future possibilities
[future-possibilities]: #future-possibilities

Create a stable interface to specify all possible format arguments.
