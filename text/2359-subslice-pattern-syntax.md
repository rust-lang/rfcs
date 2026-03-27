- Feature Name: `slice_patterns`
- Start Date: 2018-03-08
- RFC PR: [rust-lang/rfcs#2359](https://github.com/rust-lang/rfcs/pull/2359)
- Rust Issue: [rust-lang/rust#62254](https://github.com/rust-lang/rust/issues/62254)

## Summary
[summary]: #summary

Permit matching sub-slices and sub-arrays with the syntax `..`.  
Binding a variable to the expression matched by a subslice pattern can be done
using syntax `<IDENT> @ ..` similar to the existing `<IDENT> @ <PAT>` syntax, for example:

```rust
// Binding a sub-array:
let [x, y @ .., z] = [1, 2, 3, 4]; // `y: [i32, 2] = [2, 3]`

// Binding a sub-slice:
let [x, y @ .., z]: &[u8] = &[1, 2, 3, 4]; // `y: &[i32] = &[2, 3]`
```

## Motivation
[motivation]: #motivation

### General motivation
Stabilization of slice pattern with subslices is currently blocked on finalizing syntax for
these subslices.  
This RFC proposes a syntax for stabilization.

### Motivation for the specific syntax

#### The shortcut form: `..`

This form is already used in the meaning "rest of the list" in struct patterns, tuple struct
patterns and tuple patterns so it would be logical to use it for slice patterns as well.  
And indeed, in unstable Rust `..` is used in this meaning since long before 1.0.

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Sub-slices and sub-arrays can be matched using `..` and `<IDENT> @ ..` can be used to bind
these sub-slices and sub-arrays to an identifier.

```rust
// Matching slices using `ref` and `ref mut`patterns:
let mut v = vec![1, 2, 3];
match v[..] {
    [1, ref subslice @ .., 4] => assert_eq!(subslice.len(), 1), // typeof(subslice) == &[i32]
    [5, ref subslice @ ..] => assert_eq!(subslice.len(), 2), // typeof(subslice) == &[i32]
    [ref subslice @ .., 6] => assert_eq!(subslice.len(), 2), // typeof(subslice) == &[i32]
    [x, .., y] => assert!(v.len() >= 2),
    [..] => {} // Always matches
}
match v[..] {
    [1, ref mut subslice @ .., 4] => assert_eq!(subslice.len(), 1), // typeof(subslice) == &mut [i32]
    [5, ref mut subslice @ ..] => assert_eq!(subslice.len(), 2), // typeof(subslice) == &mut [i32]
    [ref mut subslice @ .., 6] => assert_eq!(subslice.len(), 2), // typeof(subslice) == &mut [i32]
    [x, .., y] => assert!(v.len() >= 2),
    [..] => {} // Always matches
}

// Matching slices using default-binding-modes:
let mut v = vec![1, 2, 3];
match &v[..] {
    [1, subslice @ .., 4] => assert_eq!(subslice.len(), 1), // typeof(subslice) == &[i32]
    [5, subslice @ ..] => assert_eq!(subslice.len(), 2), // typeof(subslice) == &[i32]
    [subslice @ .., 6] => assert_eq!(subslice.len(), 2), // typeof(subslice) == &[i32]
    [x, .., y] => assert!(v.len() >= 2),
    [..] => {} // Always matches
}
match &mut v[..] {
    [1, subslice @ .., 4] => assert_eq!(subslice.len(), 1), // typeof(subslice) == &mut [i32]
    [5, subslice @ ..] => assert_eq!(subslice.len(), 2), // typeof(subslice) == &mut [i32]
    [subslice @ .., 6] => assert_eq!(subslice.len(), 2), // typeof(subslice) == &mut [i32]
    [x, .., y] => assert!(v.len() >= 2),
    [..] => {} // Always matches
}

// Matching slices by value (error):
let mut v = vec![1, 2, 3];
match v[..] {
    [x @ ..] => {} // ERROR cannot move out of type `[i32]`, a non-copy slice
}

// Matching arrays by-value and by reference (explicitly or using default-binding-modes):
let mut v = [1, 2, 3];
match v {
  [1, subarray @ .., 3] => assert_eq!(subarray, [2]), // typeof(subarray) == [i32; 1]
  [5, subarray @ ..] => has_type::<[i32; 2]>(subarray), // typeof(subarray) == [i32; 2]
  [subarray @ .., 6] => has_type::<[i32, 2]>(subarray), // typeof(subarray) == [i32; 2]
  [x, .., y] => has_type::<i32>(x),
  [..] => {},
}
match v {
  [1, ref subarray @ .., 3] => assert_eq!(subarray, [2]), // typeof(subarray) == &[i32; 1]
  [5, ref subarray @ ..] => has_type::<&[i32; 2]>(subarray), // typeof(subarray) == &[i32; 2]
  [ref subarray @ .., 6] => has_type::<&[i32, 2]>(subarray), // typeof(subarray) == &[i32; 2]
  [x, .., y] => has_type::<&i32>(x),
  [..] => {},
}
match &mut v {
  [1, subarray @ .., 3] => assert_eq!(subarray, [2]), // typeof(subarray) == &mut [i32; 1]
  [5, subarray @ ..] => has_type::<&mut [i32; 2]>(subarray), // typeof(subarray) == &mut [i32; 2]
  [subarray @ .., 6] => has_type::<&mut [i32, 2]>(subarray), // typeof(subarray) == &mut [i32; 2]
  [x, .., y] => has_type::<&mut i32>(x),
  [..] => {},
}
```

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

`..` can be used as a pattern fragment for matching sub-slices and sub-arrays.

The fragment's syntax is:
```
SUBSLICE = .. | BINDING @ ..
BINDING = ref? mut? IDENT
```

The subslice fragment incorporates into the full subslice syntax in the same way as the `..`
fragment incorporates into the stable tuple pattern syntax (with regards to allowed number of
subslices, trailing commas, etc).

`@` can be used to bind the result of `..` to an identifier.

`..` is treated as a "non-reference-pattern" for the purpose of determining default-binding-modes,
and so shifts the binding mode to by-`ref` or by-`ref mut` when used to match a subsection of a
reference or mutable reference to a slice or array.

When used to match against a non-reference slice (`[u8]`), `x @ ..` would attempt to bind
by-value, which would fail due a move from a non-copy type `[u8]`.

`..` is not a full pattern syntax, but rather a part of slice, tuple and tuple
struct pattern syntaxes. In particular, `..` is not accepted by the `pat` macro matcher.
`BINDING @ ..` is also not a full pattern syntax, but rather a part of slice pattern syntax, so
it is not accepted by the `pat` macro matcher either.

## Drawbacks
[drawbacks]: #drawbacks

None known.

## Rationale and alternatives
[alternatives]: #alternatives

More complex syntaxes derived from `..` are possible, they use additional tokens to avoid the
ambiguity with ranges, for example
[`..PAT..`](https://github.com/rust-lang/rust/issues/23121#issuecomment-301485132), or
[`.. @ PAT`](https://github.com/rust-lang/rust/issues/23121#issuecomment-280920062) or
[`PAT @ ..`](https://github.com/rust-lang/rust/issues/23121#issuecomment-280906823), or other
similar alternatives.  
We reject these syntaxes because they only bring benefits in contrived cases using a
feature that doesn't even exist yet, but normally they only add symbolic noise.

More radical syntax changes do not keep consistency with `..`, for example
[`[1, 2, 3, 4] ++ ref v`](https://github.com/rust-lang/rust/issues/23121#issuecomment-289220169).

### `..PAT` or `PAT..`

If `..` is used in the meaning "match the subslice (`>=0` elements) and ignore it", then it's
reasonable to expect that syntax for "match the subslice to a pattern" should be some variation
on `..`.  
The two simplest variations are `..PAT` and `PAT..`.

#### Ambiguity

The issue is that these syntaxes are ambiguous with half-bounded ranges `..END` and `BEGIN..`,
and the full range `..`.  
To be precise, such ranges are not currently supported in patterns, but they may be supported in
the future.

Syntactic ambiguity is not inherently bad. We see it every day in expressions like
`a + b * c`. What is important is to disambiguate it reasonably by default and have a way to
group operands in the alternative way when default disambiguation turns out to be incorrect.  
In case of slice patterns the subslice interpretation seems more likely, so we
can take it as a default.  
There was very little demand for implementing half-bounded ranges in patterns so far
(see https://github.com/rust-lang/rfcs/issues/947), but if they
are implemented in the future they will be able to be used in slice patterns as well, but they
could require explicit grouping with recently implemented
[parentheses in patterns](https://github.com/rust-lang/rust/pull/48500) (`[a, (..end)]`) or an
explicitly written start boundary (`[a, 0 .. end]`).  
We can also make *some* disambiguation effort and, for example, interpret `..LITERAL` as a
range because `LITERAL` can never match a subslice. Time will show if such an effort is necessary
or not.

If/when half-bounded ranges are supported in patterns, for better future compatibility we could
decide to reserve `..PAT` as "rest of the list" in tuples and tuple structs as well, and avoid
interpreting it as a range pattern in those positions.

Note that ambiguity with unbounded ranges as they are used in expressions (`..`) already exists in
variant `Variant(..)` and tuple `(a, b, ..)` patterns, but it's unlikely that the `..` syntax
will ever be used in patterns in the range meaning because it duplicates functionality of the
wildcard pattern `_`.

### `..PAT` vs `PAT..`

Originally Rust used syntax `..PAT` for subslice patterns.  
In 2014 the syntax was changed to `PAT..` by [RFC 202](https://github.com/rust-lang/rfcs/pull/202).
That RFC received almost no discussion before it got merged and its motivation is no longer
relevant because arrays now use syntax `[T; N]` instead of `[T, ..N]` used in old Rust.

This RFC originally proposed to switch back to `..PAT`.
Some reasons to switch were:
- Symmetry with expressions.  
One of the general ideas behind patterns is that destructuring with
patterns has the same syntax as construction with expressions, if possible.  
In expressions we already have something with the meaning "rest of the list" - functional record
update in struct expressions `S { field1, field2, ..remaining_fields }`.
Right now we can use `S { field1, field1, .. }` in a pattern, but can't bind the remaining fields
as a whole (by creating a new struct type on the fly, for example). It's not inconceivable that
in Rust 2030 we have such ability and it's reasonable to expect it using syntax `..remaining_fields`
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

This RFC no longer includes the addition of `..PAT` or `PAT..`.
The currently-proposed change is a minimal addition to patterns (`..` for slices) which
already exists in other forms (e.g. tuples) and generalizes well to pattern-matching out sub-tuples,
e.g. `let (a, b @ .., c) = (1, 2, 3, 4);`.

Additionally, `@` is more consistent with the types of patterns that would be allowable for matching
slices (only identifiers), whereas `PAT..`/`..PAT` suggest the ability to write e.g. `..(1, x)` or
`..SomeStruct { x }` sub-patterns, which wouldn't be possible since the resulting bound variables
don't form a slice (since they're spread out in memory).

## Prior art
[prior-art]: #prior-art

Some other languages like Haskell (`first_elem : rest_of_the_list`),
Scala, or F# (`first_elem :: rest_of_the_list`) has list/array patterns, but their
syntactic choices are quite different from Rust's general style.

"Rest of the list" in patterns was previously discussed in
[RFC 1492](https://github.com/rust-lang/rfcs/pull/1492)

## Unresolved questions
[unresolved]: #unresolved-questions

None known.

## Future possibilities
[future-possibilities]: #future-possibilities

Turn `..` into a full pattern syntactically accepted in any pattern position,
(including `pat` matchers in macros), but rejected semantically outside of slice and tuple patterns.
