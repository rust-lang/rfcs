- Feature Name: complementary-traits
- Start Date: 2016-06-26
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

We revisit [the specialization
RFC](https://github.com/rust-lang/rfcs/blob/master/text/1210-impl-specialization.md),
by introducing **complementary traits** (also known as, mutually exclusive
traits, negative bounds, or anti-traits).

# Motivation
[motivation]: #motivation

`impl` specialization is rather limited to one purpose. This RFC aims to A)
simplify the rules of specialization, as described in [RFC
1210](https://github.com/rust-lang/rfcs/blob/master/text/1210-impl-specialization.md),
B) generalize the concepts of specialization to expand its domain and use.

Multiple previous similar RFCs have been submitted, most notably [this
one](https://github.com/withoutboats/rfcs/blob/mutex_traits/text/0000-mutex_traits.md).
Most of them was postponed due to the lack of implementation details (in
particular, coherence rules), which this RFC covers.

In extension to the motivation described in the original RFC, this RFC allows
for several additional usecases:

## "Exceptional" implementations

This is something often encountered with the `From` trait. `From<T>` is
implemented for `T`, which makes it unusable for certain purposes:

```rust
struct Wrapper<T>(T);

impl<T: From<U>, U> From<Wrapper<U>> for Wrapper<T> {
    fn from(Wrapper(inner): Wrapper<U>) -> Wrapper<T> {
        Wrapper(T::from(inner))
    }
}
```

fails, due to the case where `T == U`. With this RFC, a fix looks like:

```rust
struct Wrapper<T>(T);

impl<T: From<U>, U> From<Wrapper<U>> for Wrapper<T> where T != U {
    fn from(Wrapper(inner): Wrapper<U>) -> Wrapper<T> {
        Wrapper(T::from(inner))
    }
}
```

## Denying built-in traits

A common problem is denying some, often built-in, traits for either safety or
simply correctness reasons. `Drop` in particular is [lacking of an
inversion](https://github.com/search?l=rust&q=%22nodrop%22+language%3ARust&type=Code&utf8=%E2%9C%93).

Especially when doing low-level programming the lack of `!Drop` is annoying:

```rust
struct OwningPtr<T: !Drop> {
    inner: Unique<T>,
}

// I don't have to care about destructors, cause there are none of them!
```

# Detailed design
[design]: #detailed-design

## Overview

We allow `!Trait` in trait bounds, indicating that the target may not implement
`Trait`. `!Trait` and `Trait` are disjoint, allowing one to have an
implementation for each case separately.

We define coherence rules such that inconsistent or unsatisfiable bounds are
impossible.

Furthermore, we formalize the current rules in place to be able to define our
proposal.

## Current rules in place

The current rules of trait bounds are not specified in detail anywhere.
Therefore, we will briefly go through them.

We define `+` on trait bounds, as AND:

    BoundConjunctionElimination:
      C: A + B
      ────────
      C: A
      C: B

We define our bound relation as a partial order:

    BoundReflexivity:
      ─────
      A: A
    BoundAntisymmetry:
      A: B
      B: A
      ─────
      A = B
    BoundTransitivity:
      A: B
      B: C
      ─────
      A: C

Lastly, we need a notion of disjointness to reason about non-overlapping `impl`s.

    BoundDisjointness:
      A, B  disjoint
      ──────────────────
      A + C, B  disjoint

None of these rules are changed.

## `!Trait`

`!Trait` means "the complementary of `Trait`", or in other words `!Trait` is
implemented for all types, which _does not_ implement `Trait`:

    NegationBound:
      A: !B
      ───────
      ¬(A: B)

### Disjointness

To be able to reason about `impl` uniqueness, we add a disjointness rule:

    ComplementaryDisjoint:
      ───────────────
      A, !A  disjoint

This rule allows one to do, e.g.

```rust
impl<T: Trait> Type {
    // ...
}

impl<T: !Trait> Type {
    // ...
}
```

To reason about the disjointness of `!!Trait` and `!Trait` as well, we can derive the rule:

    DoubleComplementaryElimination:
      A: !!B
      ──────
      A: B

This follows directly from `NegationBound`.

### Well-formedness rules

Not all bounds are well-formed by this addition. An example of a malformed bound is `!Trait + Trait`. These are disallowed due to introducing inconsistency. Thus, we add a **well-formedness rule**:

    BoundWellFormed:
      A              WF
      B              WF
      ∀C: B.¬(A: !C)
      ─────────────────
      A + B          WF

Intuitively, one can think of this as prohibiting complementaries in the same
bound, for example `Dog + !ShibaInu` is well-formed, but `Dog + !Animal` is not.

### `!=` for parameters

`!=` in `where` bounds is relatively simple. It describes that some parameter
is _not_ of a certain type. The semantics can be thought, in the model we have
described, of as:

```rust
/// Trait which is only implemented for `T`.
trait Singleton {
    type Elem;
}

impl Singleton for T {
    type Elem = T;
}
```

Then `a != b` can be thought of as `a: !Singleton<Elem = b>`.

### Avoiding breakage

If a crate adds an implementation, which another crate assume is nonexistent, a
breakage is possible. Thus, we require the absence of such implementation to be
guaranteed.

Outside the crate itself, one cannot assume `Type: !Trait`, unless it is
explicitly stated, through the syntax:

```rust
impl !Trait for Type;
```

### (Optional:) The `Destructor` trait

`Drop` is a highly unusual trait, and it is generally not suitable, unless we
want to add an exception, for `!Drop`, since `T: !Drop` can still carry a
destructor (e.g. if a field implements `Drop`).

It naturally follows to add a `Destructor` OIBIT trait, which is
auto-implemented for following cases:

1. When the type implements `Drop`.

2. When the type owns a type implementing `Destructor`.

3. When the type is parameterized over a non-`!Destructor` type.

The semantics (e.g. limits on parameterization) are equivalent to `Drop`.

# Drawbacks
[drawbacks]: #drawbacks

None known.

# Alternatives
[alternatives]: #alternatives

The main alternative is specialization.

A more recent alternative [was proposed in this Reddit thread](https://www.reddit.com/r/rust/comments/4pun1f/looking_for_feedback_on_my_first_attempt_at_an/), introducing `else` clauses for `impl`s.

# Unresolved questions
[unresolved]: #unresolved-questions

None so far.
