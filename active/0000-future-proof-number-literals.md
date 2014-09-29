- Start Date: 2014-09-28
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Tokenize number literal suffixes more eagerly to allow future
expansion. That is, tokenise a literal suffix as `[uif][0-9]*` rather
than the specific list we currently have.

# Motivation

Currently integer/float literals can have a fixed set of suffixes,
specifically:

```text
u, u8, u16, u32, u64
i, i8, i16, i32, i64
f32, f64
```

Most things not in this list are just ignored and treated as an
entirely separate token (prefixes of `128` are errors: e.g. `1u12` has
an error `"invalid int suffix"`). For example:

```rust
#![feature(macro_rules)]

macro_rules! foo( ($($a: expr)*) => { 0 $(+ $a)+ } )

fn main() {
    println!("{}", foo!(1u256));
}
```

compiles fine, but prints `257`: the compiler is eating the `1u` and
then seeing the invalid suffix `256` and so treating that as a
separate token. (This problem is only visible in macros, since that is
the only place where two literals can be placed directly adjacent.)

This behaviour means we would be unable to expand the possibilities
for numeric literals after freezing the language/macros, which would
be unfortunate, since proposals for "bit data" would like to use
types like `u1` and `u5` (e.g. [RFC PR 327][327]), and there are "fringe" types like
[`f16`][f16], [`f128`][f128] and `u128` that have uses but are not
common enough to warrant adding to the language now.

[327]: https://github.com/rust-lang/rfcs/pull/327
[f16]: http://en.wikipedia.org/wiki/Half-precision_floating-point_format
[f128]: https://en.wikipedia.org/wiki/Quadruple-precision_floating-point_format

# Detailed design

The tokenizer will eat literal suffixes according to the regular
expression `[uif][0-9]*`, and either it or the parser reject any that
it doesn't understand.

Examples of "valid" literals after this change (that is, entities that
will be consumed as a single token):

```
1 0b2 0x3 4.5 6e78 9.10e11
12u 13i 14f
15u16 17i18 19f20 21.22f23
0b11u25 0x26i27 28.29e30f31
```

Placing a space between the letter of the suffix and the number will
cause it to be parsed as two separate tokens, just like today. That is
`1u2` is one token, `1u 2` is two tokens.

The example above would then be an error, something like:

```rust
    println!("{}", foo!(1u256)); // error: literal with unsupported size
```

Similarly, `1f16` and `0i1` would be errors. (The macro example there
is definitely an error because it is using `1u256` as an `expr`. If it
was only handling it as a token, i.e. `tt`, there is the possibility
that it wouldn't have to be illegal, e.g. `stringify!(1u256)` doesn't
have to be illegal because the `1u256` never occurs at runtime/in the
type system.)

If we choose to allow tokens with nonstandard suffixes to be used in
macros, the largest suffix that would be passed through to them would
be `u64::MAX`, in particular, 2<sup>64</sup> - 1
= 18446744073709551615. Longer streams of digits would still be
consumed as a single token but would emit an error during
tokenisation. For example:

```
1u18446744073709551616 // error: suffix out of range
1u9999999999999999999999999999 // error: suffix out of range
```

# Drawbacks

None beyond outlawing the `123u456` pattern, but the current behaviour
can easily be restored with a space: `123u 456`. (If a macro is using
this for the purpose of hacky generalised literals, the unresolved
question below touches on this.)

# Alternatives

Reserve a list of specific sizes, but keep the current behaviour for
things not on that list.

# Unresolved questions

- Should it be the parser or the tokenizer rejecting invalid suffixes?
  This is effectively asking if it is legal for syntax extensions to
  be passed the raw literals? That is, can a `foo` procedural syntax
  extension accept and handle literals like `foo!(1u2)`?

- Should we just extend this to all "identifier" tokens,
  e.g. `1u123FFFäöå`? This allows for future extension to hexidecimal
  sizes or other user-defined overloads, like
  [C++'s user defined literals][cpp]. (This could be extended to
  string and char literals, but this should be a separate RFC.)

- Should we reduce the `u64::MAX` bound to to `u32::MAX` or even
  `u16::MAX`?

[cpp]: http://en.cppreference.com/w/cpp/language/user_literal
