- Feature Name: `exhaustive_integer_patterns`
- Start Date: 2018-10-11
- RFC PR: [rust-lang/rfcs#2591](https://github.com/rust-lang/rfcs/pull/2591)
- Rust Issue: [rust-lang/rust#50907](https://github.com/rust-lang/rust/issues/50907)

# Summary
[summary]: #summary

Extend Rust's pattern matching exhaustiveness checks to cover the integer types: `u8`, `u16`, `u32`, `u64`, `u128`, `usize`, `i8`, `i16`, `i32`, `i64`, `i128`, `isize` and `char`.

```rust
fn matcher_full(x: u8) {
  match x { // ok
    0 ..= 31 => { /* ... */ }
    32 => { /* ... */ }
    33 ..= 255 => { /* ... */ }
  }
}

fn matcher_incomplete(x: u8) {
  match x { //~ ERROR: non-exhaustive patterns: `32u8..=255u8` not covered
    0 ..= 31 => { /* ... */ }
  }
}
```

# Motivation
[motivation]: #motivation

This is viewed essentially as a bug fix: other than the implementational challenges, there is no reason not to perform correct exhaustiveness checking on integer patterns, especially as range patterns are permitted, making it very straightforward to provide patterns covering every single integer.

This change will mean that Rust correctly performs exhaustiveness checking on all the types that currently compose its type system.

This feature has already [been implemented](https://github.com/rust-lang/rust/pull/50912) behind the feature flag `exhaustive_integer_patterns`, so this RFC is viewed as a motion to stabilise the feature.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Exhaustive pattern matching works for integer types, just like any other type. In addition, missing ranges of integers will be reported as errors.

```rust
fn matcher_full(x: u8) {
  match x { // ok
    0 ..= 31 => { /* ... */ }
    32 => { /* ... */ }
    33 ..= 255 => { /* ... */ }
  }
}

fn matcher_incomplete(x: u8) {
  match x { //~ ERROR: non-exhaustive patterns: `32u8..=255u8` not covered
    0 ..= 31 => { /* ... */ }
  }
}
```

Specifically, for non-`char` integer types, the entire range of values from `{integer}::MIN` to `{integer}::MAX` are considered valid constructors. For `char`, the Unicode Scalar Value (USV) ranges (`\u{0000}..=\u{D7FF}` and `\u{E000}..=\u{10FFFF}`) are considered valid constructors.

More examples may be found in [the file of test cases](https://github.com/rust-lang/rust/pull/50912/files#diff-8809036e5fb5a9a0fcc283431046ef51).

Note that guarded arms are ignored for the purpose of exhaustiveness checks, just like with any other type (i.e. arms with `if` conditions are always considered fallible and aren't considered to cover any possibilities).

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The implementation of this features uses interval arithmetic and an extension of the pattern matching exhaustiveness checks as described in [this paper](http://moscova.inria.fr/~maranget/papers/warn/index.html).

This feature has already [been implemented](https://github.com/rust-lang/rust/pull/50912), so the code there may be used for further reference. The source contains detailed comments about the implementation.

For `usize` and `isize`, no assumptions about the maximum value are permitted. To exhaustively match on either pointer-size integer type a wildcard pattern (`_`) must be used (or if [open-ended range patterns are added](https://github.com/rust-lang/rfcs/issues/947), ranges must be open ended [e.g. `0..`]). An unstable feature `precise_pointer_size_matching` will be added to permit matching exactly on pointer-size integer types.

# Drawbacks
[drawbacks]: #drawbacks

There is no reason not to do this: it fixes a limitation of the existing pattern exhaustiveness checks.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This is a straightforward extension of the existing exhaustiveness checks. This is the only sensible design for the feature.

# Prior art
[prior-art]: #prior-art

As far as the author is unaware, Rust is the first language to support exhaustive integer pattern matching. At the time of writing, Swift and OCaml, two languages for which this feature could also make sense, do not implement this extension. This is likely because the feature is not simple to implement and the usefulness of this feature appears in specific domains.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

This feature is already implemented and appears to meet expectations for such a feature, as there have been no issues brought up about the implementation or design.

# Future possibilities
[future-possibilities]: #future-possibilities

Having added exhaustive pattern matching for integers, all types in Rust for which exhaustive matching is sensible are matched exhaustively. We should aim to ensure this remains the case. However, at present, exhaustive pattern matching in Rust is viewed complete.
