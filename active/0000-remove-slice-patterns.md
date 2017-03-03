- Start Date: 2014-06-27
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Remove variable-length slice patterns from the language.

```rust
    fn is_symmetric(list: &[uint]) -> bool {
        match list {
            [] | [_]                   => true,
            [x, ..inside, y] if x == y => is_symmetric(inside),
            _                          => false
        }
    }
```

# Motivation

1. Slice patterns do not fit in well with the rest of the supported pattern types. Slices are currently the only non-algebraic data type that can be destructured in a pattern.
2. To the best of my knowledge and understanding, they're also impossible to reason about, in all cases but the trivial (such as match expressions with only Cons-like ([first, second, ..tail]) or Snoc-like patterns ([..init, last])), when analyzing match arms for exhaustiveness and reachability, without incorporating an SMT solver.

# Detailed design

Support for variable length slice patterns will be removed from match expressions. It will still be possible to pattern match fixed-length vectors as they do not suffer from the same two issues mentioned in the Motivation section.

# Drawbacks

Slice patterns can be a convenient way of working with slices and vectors. In particular, they can be convenient when implementing tail-recursive algorithms that work on slices. Making this change will break users' code, however, there do not appear to be many uses of the feature in the wild. rustc contains 10 occurrences of slice patterns in a match expression. Servo uses a slice pattern once and it's a fixed-length slice pattern. coreutils uses them twice.

# Alternatives

To address the second limitation without removing the syntax, slice patterns can be restricted to only allow Cons-like an Snoc-like patterns, where the variable-length part of the pattern appears at the very end or the very beginning of the pattern:
```rust
match x { [first, second, ..tail] => () }
match x { [..init, first, second] => () }
```
In addition, the two could not be intermixed together in a single column of a match expression.

The impact of leaving the feature intact will be that slice patterns will have to, in some scenarios, be excluded from the exhaustiveness analysis, which will result in match expressions that can fail at runtime, introducing a new class of runtime errors.

# Unresolved questions

If the syntax was to be introduced again or preserved now to its full extent, what would be the best approach to implement an exhaustiveness/reachability analysis pass that's different than the current ML-like approach and correctly supports this flavour of patterns?
