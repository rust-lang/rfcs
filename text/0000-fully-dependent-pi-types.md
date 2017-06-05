- Feature Name: fully-dependent-pi-types
- Start Date: 2017-02-26
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary
This RFC is a part of the [pi type
trilogy](https://github.com/rust-lang/rfcs/issues/1930), introducing the last
brick to make Rust a fully dependent type system.

# Motivation
[motivation]: #motivation
Dependent types is a feature which is frequently requested.

# Prerequisite material
The `where` RFC (kept track of
[here](https://github.com/rust-lang/rfcs/issues/1930)) introduces a set of
axioms, defining a simple, constructive logic.

# Detailed design
[design]: #detailed-design

In the future, we might like to extend it to a fully dependent type system.
While this is, by definition, a dependent type system, one could extend it to
allow runtime defind value parameters.

Consider the `index` example. If one wants to index with a runtime defined
integer, the compiler have to be able to show that this value does, in fact,
satisfy the `where` clause.

There are multiple ways to go about this. We will investigate the Hoare logic way.

## Is this bloat?

Well, we can't know if it is. It depends on how expressive, const parameters
are in practice, or more so, if there exists edge cases, which they do not cover.

## "Sizingness"

Certain value parameters have impact on the size of some type (e.g., consider
`[T; N]`). It would make little to no sense, to allow one to determine the size
of some value, say a constant sized array, at runtime.

Thus, one must find out if a value parameter is impacting the size of some
type, before one can allow runtime parameterization.

Currently, only one of such cases exists: constant sized arrays. One could as
well have a struct containing such a primitive, thus we have to require
transitivity.

If a value parameter has no impact on the size, nor is used in a parameter of a
constructor, which is "sizing", we call this value "non-sizing".

Only non-sizing value parameters can be runtime defined.

## Hoare logic invariants and runtime calls

As it stands currently, one cannot "mix up" runtime values and value parameter
(the value parameters are entirely determined on compile time).

It turns out reasoning about invariants is not as hard as expected. [Hoare
logic](https://en.wikipedia.org/wiki/Hoare_logic) allows for this.

One need not SMT-solvers for such a feature. In fact, one can reason from the
rules, we have already provided. With the addition of MIR, this might turn out
to be more frictionless than previously thought.

Hoare logic can be summarized as a way to reason about a program, by giving
each statement a Hoare triple. In particular, in addition to the statement
itself, it carries a post- and precondition. These are simple statements that
can be incrementally inferred by the provided Hoare rules.

Multiple sets of axioms for Hoare logics exists. The most famous one is the set
Tony Hoare originally formulated.

For a successful implementation, one would likely only need a tiny subset of
these axioms:

### Assignment axiom schema

This is, by no doubt, the most important rule in Hoare logic. It allows us to
carry an assumption from one side of an assignment to another.

It states:

    ────────────────────
    {P[x ← E]} x = E {P}

That is, one can take a condition right after the assignment, and move it prior
to the assignment, by replacing the variable with the assigned value.

An example is:

```rust
// Note: This should be read from bottom to top!

// Now, we replace a with a + 3 in our postcondition, and get a + 3 = 42 in our precondition.
a = a + 3;
// Assume we know that a = 42 here.
```

This rule propagate "backwards". Floyd formulated a more complicated, but forwards rule.

### `while` rule

The `while` rule allows us to reason about loop invariants.

Formally, it reads

    {P ∧ B} S {P}
    ───────────────────────────
    {P} (while B do S) {¬B ∧ P}

`P`, in this case, is the loop invariant, a condition that much be preserved
for each iteration of the body.

`B` is the loop condition. The loop ends when `P` is false, thus, as a
postcondition to the loop, `¬B`.

### Conditional rule

The conditional rule allows one to reason about path-specific invariants in
e.g. `if` statements.

Formally, it reads

    {B ∧ P} S {Q}
    {¬B ∧ P } T {Q}
    ────────────────────────────
    {P} (if B then S else T) {Q}

This allows us to do two things:

1. Lift conditionals down to the branch, as precondition.

2. Lift conditions up as postconditions to the branching statement.

---

In addition, we propose these Rust specific axioms:

### Non-termination rule

This can be used for reasoning about assertions and panics, along with aborts
and other functions returning `!`.

Formally, the rule is:

    f: P → !
    p: P
    ───────────────────────────
    (if P then f p else E) {¬P}

This simply means that:

```rust
if a {
    // Do something `!` here, e.g. loop infinitely:
    loop {}
} else {
    // stuff
}
// We know, since we reached this, that !a.
```

### How this allows runtime calls

Runtime calls are parameterized over runtime values. These allows the compiler
to semantically reason about the value of some variable. Thus, the bound can be
enforced on compile time, by making sure the statements of the value implies
whatever bound that must be satisfied.

# Drawbacks
[drawbacks]: #drawbacks

It should be obvious that this extension is a big change, both internally and
externally. It makes Rust much more complicated, and drives it in a direction,
which might not be wanted.

One can argue that it aligns with Rust's goals: effective static checking. As
such, runtime assertions are to a less extend needed.

# Alternatives
[alternatives]: #alternatives
## MIR-based Hoare logic
I introduced MIR-based Hoare logic [last
year](https://ticki.github.io/blog/a-hoare-logic-for-rust/). It is probably
more expressive, but it requires a SMT-solver such as Z3.

# Unresolved questions
[unresolved]: #unresolved-questions
Is it needed? Are there a stronger motivation?
