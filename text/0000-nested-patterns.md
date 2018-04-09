- Feature Name: `nested_disjunction_patterns`
- Start Date: 2018-04-07
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Allow `|` to be used within a pattern such that `Some(A(x) | B(x))` becomes
a valid pattern. You can also nest `|` by an arbitrary amount.

# Motivation
[motivation]: #motivation

## DRY

## Mental model

+ French
+ German
+ Spanish
+ Esperanto
+ Swedish
+ Farsi
+ Finnish
+ Japanese
+ Portuguese

## Readability

## Ergonomics



TODO

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

```rust
match (first, next!()) {
    (0xE0         , 0xA0 ... 0xBF) |
    (0xE1 ... 0xEC, 0x80 ... 0xBF) |
    (0xED         , 0x80 ... 0x9F) |
    (0xEE ... 0xEF, 0x80 ... 0xBF) => {}
    _ => err!(Some(1))
}
```

rewrite as:

```rust
match (first, next!()) {
    (0xE0                         , 0xA0 ... 0xBF) |
    (0xE1 ... 0xEC | 0xEE ... 0xEF, 0x80 ... 0xBF) |
    (0xED                         , 0x80 ... 0x9F) => {}
    _ => err!(Some(1))
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

TODO

# Drawbacks
[drawbacks]: #drawbacks

TODO

# Rationale and alternatives
[alternatives]: #alternatives

TODO

# Prior art
[prior-art]: #prior-art

## CSS4 selectors

[CSS4]: https://drafts.csswg.org/selectors/#matches

In [CSS4] (draft proposal), it is possible to write a selector
`div > *:matches(ul, ol)` which is equivalent to `div > ul, div > ol`.
The moral equivalent of this in Rust would be: `Div(Ul | Ol)`.

## Regex

TODO

## OCaml

[This is supported](https://caml.inria.fr/pub/docs/manual-ocaml/patterns.html#sec108) in OCaml. An example from Real World OCaml is:
```ocaml
let is_ocaml_source s =
  match String.rsplit2 s ~on:'.' with
  | Some (_, ("ml" | "mli")) -> true
  | _ -> false
```

## Haskell

The equivalent proposal is currently being discussed for inclusion in Haskell.
See: <https://github.com/ghc-proposals/ghc-proposals/pull/43> for that process.

# Unresolved questions
[unresolved]: #unresolved-questions

TODO