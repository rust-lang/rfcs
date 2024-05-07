- Feature Name: `ref_pat_eat_one_layer_2024`
- Start Date: 2024-05-06
- RFC PR: [rust-lang/rfcs#3627](https://github.com/rust-lang/rfcs/pull/3627)
- Rust Issue: [rust-lang/rust#123076](https://github.com/rust-lang/rust/issues/123076)

# Summary
[summary]: #summary

Various changes to the match ergonomics rules:

- On edition ≥ 2024, `&` and `&mut` patterns only remove a single layer of
  references.
- On edition ≥ 2024, `mut` on an identifier pattern does not force its binding
  mode to by-value.
- On edition ≥ 2024, `&` patterns can match against `&mut` references.
- On all editions, the binding mode can no longer ever be implicitly set to
  `ref mut` behind an `&` pattern.

# Motivation
[motivation]: #motivation

Match ergonomics have been a great success overall, but there are some surprising
interactions that regularly confuse users.

## `mut` resets the binding mode

`mut` resets the binding mode to by-value, which users do not expect; the
mutability of the binding would seem to be separate concern from its type
(<https://github.com/rust-lang/rust/issues/105647>,
<https://github.com/rust-lang/rust/issues/112545>).

```rust
let (x, mut y) = &(true, false);
let _: (&bool, bool) = (x, y);
```

## Can't cancel out an inherited reference

`&` and `&mut` patterns must correspond with a reference in the same position in
the scrutinee, even if there is an inherited reference present. Therefore, users
have no general mechanism to "cancel out" an inherited reference
(<https://users.rust-lang.org/t/reference-of-tuple-and-tuple-of-reference/91713/6>,
<https://users.rust-lang.org/t/cannot-deconstruct-reference-inside-match-on-reference-why/92147>,
<https://github.com/rust-lang/rust/issues/50008>,
<https://github.com/rust-lang/rust/issues/64586>).


```rust
fn foo(arg: &(String, Vec<i32>, u8)) {
    // We want to extract `&String`, `&Vec`, and `u8` from the tuple.
    let (s, v, u) = arg; // u is &u8, not what we wanted
    let &(ref s, ref v, u) = arg; // we have to abandon match ergonomics entirely
}
```

## A single `&` can strip two references

When an `&` or `&mut` pattern is used in a location where there is also an
inherited reference present, both are stripped; adding a single `&` to the
pattern can remove two `&`s from the type of the binding.

```rust
let [a] = &[&42]; // a = &&42
let [&a] = &[&42]; // a = 42
```

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Match ergonomics works a little differently in edition 2024 and above.

## `mut` no longer strips the inherited reference

`mut` on a binding does not reset the binding mode on edition ≥ 2024.

```rust
//! Edition ≥ 2024
let (x, mut y) = &(true, false);
let _: (&bool, &bool) = (x, y); // instead of `(&bool, bool)`
```

## Matching against inherited references

In all editions, when you match against an `&` or `&mut` reference with the type
of its referent, you get an "inherited reference": the binding mode of
"downstream" bindings is set to `ref` or `ref mut`.

```rust
//! All editions
// `x` "inherits" the `&` from the scrutinee type.
let [x] = &[42];
let _: &u8 = x;
```

In edition 2024 and above, an `&` or `&mut` pattern can match against this
inherited reference, consuming it. A pattern that does this has no other effect.

```rust
//! Edition ≥ 2024

// `&` pattern consumes inherited `&` reference.
let [&x] = &[42];
let _: u8 = x;

// Examples from motivation section

fn foo(arg: &(String, Vec<i32>, u8)) {
    let (s, v, &u) = arg;
    let _: (&String, &Vec<i32>, u8) = (s, v, u);
}

let [&x] = &[&42];
let _: &u8 = x;
```

## `&` matches against `&mut`

In edition 2024 and above, `&` patterns can match against `&mut` references
(including "inherited" references).

```rust
//! Edition ≥ 2024
let &foo = &mut 42;
let _: u8 = foo;
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This explanation assumes familiarity with the current match ergonomics rules,
including the "default binding mode" terminology. Refer to [RFC 2005](./2005-match-ergonomics.md#detailed-design).

## Edition 2024: `mut` does not reset binding mode to by-value

In the new edition, `mut` no longer resets the binding mode to by-value.
Therefore, it is possible to have a mutable by-reference binding. (An explicit
syntax for this is left to a future RFC.)

```rust
//! Edition ≥ 2024
let &[mut a] = &[42];
a = &47;
```

## Edition 2024: `&` patterns can match against `&mut` references

`&` patterns can match against `&mut` references.

```rust
//! Edition ≥ 2024
let &foo = &mut 42;
let _: u8 = foo;
```

However, the `ref mut` binding mode cannot be used behind such patterns.

```rust
//! All editions
let &ref mut foo = &mut 42;
//  ^~ERROR: replace `&` with `&mut `
let _: &mut u8 = foo;
```

## Edition 2024: `&` and `&mut` can match against inherited references

When the default binding mode is `ref` or `ref mut`, `&` and `&mut` patterns can
reset it. `&` patterns will reset either `ref` or `ref mut` binding modes to
by-value, while `&mut` can only reset `ref mut`. An `&` or `&mut` pattern that
resets the binding mode in this way has no other effect.

```rust
//! Edition ≥ 2024

let [&x] = &[3u8];
let _: u8 = x;

let [&mut x] = &mut [3u8];
let _: u8 = x;

let [&x] = &mut [3u8];
let _: u8 = x;
```

```rust
//! All editions
//let [&mut x] = &[3u8]; // ERROR
```

`&` patterns are otherwise unchanged from older editions.

```rust
//! All editions

let &a = &3;
let _: u8 = a;

//let &b = 17; // ERROR
```

If the default binding mode is `ref`, then `&mut` patterns are forbidden. If it
is by-value, then they have the same effect as on older editions.

```rust
//! Edition ≥ 2024
//let [&mut x] = &[&mut 42]; // ERROR
```

```rust
//! All editions

let &mut x = &mut 3;
let _: u8 = x;

let &mut x = &mut &mut 3;
let _: &mut u8 = x;

let &mut x = &mut &&mut 3;
let _: &&mut u8 = x;

//let &mut x = &&mut 3; // ERROR
```

## All editions: the default binding mode is never set to `ref mut` behind an `&` pattern or reference

The binding mode is set to `ref` instead in such cases. (On older editions, this
allows strictly more code to compile.)

```rust
//! All editions (new)

let &[[a]] = &[&mut [42]];
let _: &u8 = a; // previously `a` would be `&mut u8`, resulting in a move check error

let &[[a]] = &mut [&mut [42]];
let _: &u8 = a;
```

```rust
//! Edition ≥ 2024

let &[[&a]] = &[&mut [42]];
let _: u8 = a;

//let &[[&mut a]] = &[&mut [42]]; // ERROR
```

# Migration
[migration]: #migration

This proposal, if adopted, would allow the same pattern to have different
meanings on different editions:

```rust
let [&a] = &[&0u8]; // `a` is `u8` on edition ≤ 2021, but `&u8` on edition ≥ 2024
let [mut a] = &[0u8]; // `a` is `u8` on edition ≤ 2021, but `&u8` on edition ≥ 2024
```

Instances of such incompatibilities appear to be uncommon, but far from unknown
(20 cases in `rustc`, for example). The migration lint for the feature entirely
desugars the match ergonomics of the affected pattern. This is necessary to
produce code that works on all editions, but it means that adopting the new
rules could require editing the affected patterns twice: once to desugar the
match ergonomics before adopting the new edition, and a second time to restore
match ergonomics after adoption of the new edition.

# Drawbacks
[drawbacks]: #drawbacks

This is a silent change in behavior, which is considered undesirable even
over an edition.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Desirable property
[desirable-property]: #desirable-property

The proposed rules for new editions uphold the following property:

> For any two nested patterns `$pat0` and `$pat1`, such that `$pat1` uses match
> ergonomics only (no explicit `ref`/`ref mut`), and valid pattern match
> `let $pat0($pat1(binding)) = scrut`, either:
>
> - `let $pat0(temp) = scrut; let $pat1(binding) = temp;` compiles, with the
> same meaning as the original composed pattern match; or
> - `let $pat0(temp) = scrut; let $pat1(binding) = temp;` does not compile, but
> `let $pat0(ref temp) = scrut; let &$pat1(binding) = temp;` compiles, with the
> same meaning as the original composed pattern match.

In other words, the new match ergonomics rules are compositional.

## `mut` not resetting the binding mode

Admittedly, there is not much use for mutable by-reference bindings. This is
true even outside of pattern matching; `let mut ident: &T = ...` is not commonly
seen (though not entirely unknown either). The motivation for making this change
anyway is that the current behavior is unintuitive and surprising for users.

## Never setting default binding mode to `ref mut` behind `&`

On all editions, when a structure pattern peels off a shared reference and the
default binding mode is already `ref mut`, the binding mode gets set to `ref`.
But when the binding mode is set to `ref`, and a mutable reference is peeled
off, the binding mode remains `ref`. In other words, immutability usually takes
precedence over mutability. This change, in addition to being generally useful,
makes the match ergonomics rules more consistent by ensuring that immutability
*always* takes precedence over mutability.

Note that while this is not a breaking change for edition 2021 and below, it
*would be breaking* to adopt the rest of this RFC without this change, and then
later adopt this change alone. Specifically, pattern matches like the following
would break:

```rust
let &[[&mut a]] = &[&mut [42]];
```

Therefore, we *cannot* delay a decision on this matter.

## `&` patterns matching against `&mut`

There are several motivations for allowing this:

- It makes refactoring less painful. Sometimes, one is not certain whether an
  unfinished API will end up returning a shared or a mutable reference. But as
  long as the reference returned by said API is not actually used to perform
  mutation, it often doesn't matter either way, as `&mut` implicitly reborrows
  as `&` in many situations. Pattern matching is currently one of the most
  prominent exceptions to this, and match ergonomics magnifies the pain because
  a reference in one part of the pattern can affect the binding mode in a
  different, faraway location[^nrmba]. If patterns can be written to always use
  `&` unless mutation is required, then the amount of editing necessary to
  perform various refactors is lessened.
- It's intuitive. `&mut` is strictly more powerful than `&`. It's conceptually a
  subtype, and even if not implemented that way[^sub], coercions mean it often
  feels like one in practice.

```rust
let a: &u8 = &mut 42;
```

[^nrmba]: This is especially true in light of the [new rule](#all-editions-the-default-binding-mode-is-never-set-to-ref-mut-behind-an--pattern-or-reference)
that prevents the default binding mode from being set to `ref mut` behind `&`.

[^sub]: Making `&mut` a subtype of `&` in actual implementation would require
adding significant complexity to the variance rules, but I do believe it to be
possible.

## Versus "eat-two-layers"

An alternative proposal would be to allow `&` and `&mut` patterns to reset the
binding mode when not matching against a reference in the same position in the
scrutinee, but to not otherwise change their behavior. This would have the
advantage of not requiring an edition change. However, it would remain confusing
for users. Notably, the [property from earlier](#desirable-property) would
continue to not be satisfied.

In addition, this approach would lead to tricky questions around when
mutabilities should be considered compatible.

(This alternative is currently implemented under a separate feature gate.)

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- How much churn will be necessary to adapt code for the new edition? There are
  0 instances of affected patterns in the standard library, and 20 in the
  compiler, but that is all the data we have at the moment.

# Future possibilities
[future-possibilities]: #future-possibilities

- An explicit syntax for mutable by-reference bindings should be chosen at some
  point.
- Deref patterns may interact with `&` and `&mut` patterns.
- Future changes to reference types (partial borrows, language sugar for `Pin`,
  etc) may interact with match ergonomics.

## Matching `&mut` behind `&`

There is one notable situation where match ergonomics cannot be used, and
explicit `ref` is required. Notably, this can occur where `&mut` is nested
behind `&`:

```rust
// No way to avoid the `ref`, even with this RFC
let &[&mut ref x] = &[&mut 42];
```

There are two strategies we could take to support this:

- `&mut` patterns could match "behind" `&`. For example, in `let [&mut x] = &[&mut 42];`,
  the `&mut` pattern would match the `&mut` reference in the scrutinee, leaving
  `&` to be inherited and resulting in `x: &i32`.
  - This may not extend gracefully to future language features (partial borrows,
    for example) as it relies on reference types forming a total order.
- The compiler could insert `&mut ref` in front of identifier patterns of type
  `&mut` that are behind an `&` pattern. For example, `let &[x] = &[&mut 42];`
  would be transformed into `let &[&mut ref x] = &[&mut 42];`.
  - The full desugaring would be more complicated, as it would need to handle
    `@` patterns.
