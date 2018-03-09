- Feature Name: slice_patterns
- Start Date: 2018-03-08
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Use an obvious syntax for subslice patterns - `..` and `..PAT`.  
If syntactic ambiguities arise in the future, always disambiguate in favor of subslice patterns.

# Motivation
[motivation]: #motivation

## General motivation
Stabilization of slice pattern with subslices is currently blocked on finalizing syntax for
these subslices.  
This RFC proposes a syntax for stabilization.

## Motivation for the specific syntax

### The shortcut form: `..`

This form is already used in the meaning "rest of the list" in struct patterns, tuple struct
patterns and tuple patterns so it would be logical to use it for slice patterns as well.  
And indeed, in unstable Rust `..` is used in this meaning since long before 1.0.

### The full form: `..PAT` or `PAT..`

If `..` is used in the meaning "match the subslice (`>=0` elements) and ignore it", then it's
reasonable to expect that syntax for "match the subslice to a pattern" should be some variation
on `..`.  
The two simplest variations are `..PAT` and `PAT..`.

#### Ambiguity

The issue is that these syntaxes are ambiguous with half-bounded ranges `..END` and `BEGIN..`.  
To be precise, such ranges are not currently supported in patterns, but they may be supported in
the future.

We argue that this issue is not important and we can choose this syntax for subslice patterns
anyway.  
First of all, syntactic ambiguity is not inherently bad, we see it every day in expressions like
`a + b * c`. What is important is to disambiguate it reasonably by default and have a way to
group operands in the alternative way when default disambiguation turns out to be incorrect.  
In case of slice patterns the subslice interpretation seems overwhelmingly more likely, so we
can take it as a default.  
There was no visible demand for implementing half-bounded ranges in patterns so far, but if they
are implemented in the future they will be able to be used in slice patterns as well, but they
will require explicit grouping with recently implemented
[parentheses in patterns](https://github.com/rust-lang/rust/pull/48500).  
We can also make *some* disambiguation effort and, for example, interpret `..LITERAL` as a
range because `LITERAL` can never match a subslice. Time will show if such an effort is necessary
or not.

If/when half-bounded ranges are supported in patterns, for better future compatibility we'll need
to reserve `..PAT` as "rest of the list" in tuples and tuple structs as well, and avoid interpreting
it as a range pattern in those positions.

#### `..PAT` vs `PAT..`

Originally Rust used syntax `..PAT` for subslice patterns.  
In 2014 the syntax was changed to `PAT..` by [RFC 202](https://github.com/rust-lang/rfcs/pull/202).
That RFC received almost no discussion before it got merged and its motivation is no longer
relevant because arrays now use syntax `[T; N]` instead of `[T, ..N]` used in old Rust.

Thus we are proposing to switch back to `..PAT`.
Some reasons to switch:
- Symmetry with expressions.  
One of the general ideas behind patterns is that destructuring with
patterns has the same syntax as construction with expressions, if possible.  
In expressions we already have something with the meaning "rest of the list" - functional record
update in struct expressions `S { field1, field2, ..remaining_fields }`.
Right now we can use `S { field1, field1, .. }` in a pattern, but can't bind the remaining fields
as a whole (by creating a new struct type on the fly, for example). It's not inconceivable that
in Rust 2525 we have such ability and it's reasonable to expect it using syntax `..remaining_fields`
symmetric to expressions. It would be good for slice patterns to be consistent with it.  
Without speculations, even if `..remaining_fields` in struct expressions and `..subslice` in slice
patterns are not entirely the same thing, they are similar enough to keep them symmetric already.
- Simple disambiguation.  
When we are parsing a slice pattern and see `..` we immediately know it's
a subslice and can parse following tokens as a pattern (unless they are `,` or `]`, then it's just
`..`, without an attached pattern).  
With `PAT..` we need to consume the pattern first, but that pattern may be a... `RANGE_BEGIN..`
range pattern, then it means that we consumed too much and need to reinterpret the parsed tokens
somehow. It's probably possible to make this work, but it's some headache that we would like to
avoid if possible.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Subslice (aka "rest of the slice") in a slice patterns can be matched to a pattern `PAT` using
syntax `..PAT`.  
`..` with the pattern omitted is a sugar for `.._` (wildcard pattern) so it means
"ignore the rest of the slice".

Example (without `feature(match_default_bindings)`):
```rust
let v = vec![1, 2, 3];
match v[..] {
    [1, ..ref subslice, 4] => assert_eq!(subslice.len(), 1),
    [5, ..ref subslice] => assert_eq!(subslice.len(), 2),
    [..ref subslice, 6] => assert_eq!(subslice.len(), 2),
    [x, .., y] => assert!(v.len() >= 2),
    [..] => {} // Always matches
}
```
Example (with `feature(match_default_bindings)`):
```rust
let v = vec![1, 2, 3];
match &v[..] {
    [1, ..subslice, 4] => assert_eq!(subslice.len(), 1),
    [5, ..subslice] => assert_eq!(subslice.len(), 2),
    [..subslice, 6] => assert_eq!(subslice.len(), 2),
    [x, .., y] => assert!(v.len() >= 2),
    [..] => {} // Always matches
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Subslice in a slice patterns can be matched to a pattern `PAT` using syntax `..PAT`.  
`..` with the pattern omitted is a sugar for `.._`.

If ambiguity with some other syntactic construction arises in the future, disambiguation will be
performed in favor of the subslice pattern.

# Drawbacks
[drawbacks]: #drawbacks

None known.

# Rationale and alternatives
[alternatives]: #alternatives

The `PAT..` alternative was discussed in the motivational part of the RFC.

More complex syntaxes derived from `..` are possible, they use additional tokens to avoid the
ambiguity with ranges, for example
[`..PAT..`](https://github.com/rust-lang/rust/issues/23121#issuecomment-301485132), or
`.. @ PAT` or `PAT @ ..` (original comments seem to be lost by GitHub), or other similar
alternatives.  
We reject these syntaxes because they only bring benefits in incredibly contrived cases using a
feature that doesn't even exist yet, but normally they only add symbolic noise.

More radical syntax changes not keeping consistency with `..`, for example
[`[1, 2, 3, 4] ++ ref v`](https://github.com/rust-lang/rust/issues/23121#issuecomment-289220169).

# Prior art
[prior-art]: #prior-art

Some other languages like Scala or F# has list/array patterns, but their
syntactic choices are quite different from Rust's general style.

"Rest of the list" in patterns was previously discussed in
[RFC 1492](https://github.com/rust-lang/rfcs/pull/1492)

# Unresolved questions
[unresolved]: #unresolved-questions

None known.
