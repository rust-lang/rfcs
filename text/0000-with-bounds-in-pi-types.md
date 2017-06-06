- Feature Name: with-bounds-in-pi-types
- Start Date: 2017-02-26
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary
This RFC is a part of the [pi type
trilogy](https://github.com/rust-lang/rfcs/issues/1930).

This RFC introduces `with` bounds on type level values, making it possible to
set constraints on the constant parameters introduced in implementations,
functions, structs, and other items.

# Motivation
[motivation]: #motivation

There are many usecases. These allows certain often requested features to live
in standalone libraries (e.g.,
[bounded-integers](#bounded-integersinterval-arithmetics), [type level
numerals](#array-generics), [statically checked
indexing](#statically-checked-indexing), lattice types).

I've listed examples of such usecases and their implementations below.

## Examples

### Bounded integers/interval arithmetics

One can define the so called "bounded integers" (integers which carry an
upper and lower bound, checked at compile time):

```rust
use std::ops;

/// A bounded integer.
///
/// This has two value parameter, respectively representing an upper and a lower bound.
pub struct BoundedInt<const lower: usize, const upper: usize> {
    /// The inner runtime value.
    ///
    /// Note how this is not public. This is because we cannot, in the updated
    /// version of the RFC, set constraints on compile time, so when created it
    /// must either be dynamically ensured or an unsafe contract must be made
    in order to satisfy the invariants.
    n: usize,
}

// To see how this holds the `with` clause above, see the section on `identities`.
impl<const n: usize> BoundedInt<n, n> {
    fn new() -> Self {
        BoundedInt {
            n: n,
        }
    }
}

/// Addition of two `BoundedInt` will simply add their bounds.
///
/// We check for overflow making it statically overflow-free calculations.
impl<const upper_a: usize,
     const lower_a: usize,
     const upper_b: usize,
     const lower_b: usize> ops::Add<BoundedInt<lower_b, upper_b>> for BoundedInt<lower_a, upper_a>
     // We have to satisfy the constraints set out in the struct definition.
     with lower_a <= upper_a,
           lower_b <= upper_b,
           // Check for overflow by some `const fn`.
           is_overflow_safe(upper_a, upper_b) {
    // These parameters are constant expression.
    type Output = BoundedInt<lower_a + lower_b, upper_a + upper_b>;

    fn add(self, rhs: BoundedInt<lower_b, upper_b>) -> Self::Output {
        BoundedInt {
            n: self.n + rhs.n,
        }
    }
}

impl<const upper_a: usize,
     const lower_a: usize,
     const upper_b: usize,
     const lower_b: usize> From<BoundedInt<lower_b, upper_b>> for BoundedInt<lower_a, upper_a>
     with lower_a <= upper_a,
           lower_b <= upper_b,
           // We will only extend the bound, never shrink it without runtime
           // checks, thus we add this clause:
           lower_b <= lower_a && upper_b >= upper_a {
    fn from(from: BoundedInt<lower_b, upper_b>) -> Self {
        BoundedInt {
            n: from.n,
        }
    }
}
```

### Homogeneous varargs

We can use arbitrarily length arrays to simulate homogeneous varargs:

```rust
fn my_func<const n: usize>(args: [u32; n]) { /* whatever */ }

my_func([1, 2, 3]);
my_func([1, 2, 3, 4]);
```

### Array generics

Currently libcore only implements various traits up to arrays of length 32.
This allows for implementing them for arrays of arbitrary length:

```rust
impl<const n: usize, T: Clone> Clone for [T; n] {
    fn clone(&self) -> [T; n] {
        // Clone it...
    }
}
```

### Statically checked indexing

One can perform simple, interval based, statically checked indexing:

```rust
use std::ops;

impl<const n: usize, T: Clone> ops::Index<BoundedInt<0, n - 1>> for [T; n] {
    type Output = T;

    fn index(&self, ind: BoundedInt<0, n - 1>) -> &T {
        unsafe {
            // This is safe due to the bound on `ind`.
            self.unchecked_index(*ind)
        }
    }
}
```

### Fancy number stuff

```rust
struct Num<const n: usize>;

trait Divides<const m: usize> {}

impl<const a: usize, const b: usize> Divides<b> for Num<a> with b % a == 0 {}
```

### Playground

[This repo](https://github.com/ticki/rfc-1657-playground) is a playground for
usecases of such a feature. Refer to that for more examples.

## `with` clauses

Often, it is wanted to have some statically checked clause satisfied by the
constant parameters (e.g., for the sake of compile-time bound checking). To
archive this, in a reasonable manner, we use constexprs, returning a boolean.

We allow such constexprs in `with` clauses of functions. Whenever the
function is invoked given constant parameters `<a, b...>`, the compiler
evaluates this expression, and if it returns `false`, an aborting error is
invoked.

To sum up, the check happens when typechecking the function calls (that is,
checking if the parameters satisfy the trait bounds). The caller's bounds must
imply the invoked functions' bounds:

### Transitivity of bounds

We require a bound of a function to imply the bounds of the functions it calls,
through a simple reductive, unification algorithm. In particular, this means
that a statement is reduced by some specified rules (see below), that ensures
termination. A statement implies another statement, if the set of statements it
reduces to is a superset of the other statement's reduction set.

The compiler would enforce that if `f` calls `g`, `unify(bound(g)) ⊆
unify(bound(f))` (by structural equality):

    ExpandBooleanAnd:
      P ∧ Q
      ─────
      P
      Q

This simply means that `a ∧ b` means `a` and `b`.

    SubstituteEquality:
      P(a)
      a = b
      ─────
      P(b)

This is an important inference rule, when doing unification. This means that
you can substitute all `a` for all free `b`s, if `a = b`.

    DoubleNegation:
      ¬¬x
      ───
      x

This rule is simply stating that double negation is identity, that is, `!!a`
means that `a` is true.

These rules are "eliminatory" (recursing downwards the tree and decreasing the
structure), and thus it is possible to check, in this language, that `a ⇒ b`
relatively quickly (`O(n)`). For a proof of see the section below.

More rules can be added in the future. It is however important to preserve the
"sequential property" (that is, each step is a reduction, not an expansion),
allowing one to check the implication in linear time.

This is done under type unification. Thus, we only need to check the bounds at
the top level.

#### Decidability of this rule set

One can show this by considering each case:

1. `ExpandBooleanAnd` eliminates `{P ∧ Q} ⊢ {P, Q}`. The right hand side's
   depth is `max(dep(P), dep(Q))`, which is smaller than the original,
   `max(dep(P), dep(Q)) + 1`
2. `SubstituteEquality` eliminates `{a = b, P} ⊢ {P[b ← a]}`, which is an
   elimination, since `dep(P) + 1 > dep(P[b ← a]) = dep(P)`.
3. `DoubleNegation` eliminates `{¬¬x} ⊢ {x}`, which is an elimination, since
   `dep(x) + 2 > dep(x)`.

In fact, this set of rule is strictly reductive (like equality-based unification).

#### An example

We will quickly give an example of a possible proof. Say we want to show that
`(x = b) ∧ ¬¬(x < a) ⇒ b < a`. Starting with the left hand side, we can sequentially
prove this, by simple unification (which already exists in the Rust type
checker):

    (x = b) ∧ ¬¬(x < a)
    ∴ x = b      (ExpandBooleanAnd)
      ¬¬(x < a)
    ∴ ¬¬(b < a)  (SubstituteEquality)
    ∴ b < a      (DoubleNegation)
      ¯¯¯¯¯

### Contradictive or unsatisfiable bounds

Contradictive or unsatisfiable bounds (like `a < b, b < a`) cannot be detected,
since such a thing would be undecidable.

These bounds don't break anything, they are simply malformed and unreachable.

Take `a < b, b < a` as an example. We know the values of `a` and `b`, we can
thus calculate the two bounds, which will clearly fail. We cannot, however,
stop such malformed bounds in _declarations_ and _function definitions_, due to
mathematical limitations.

## The grammar

These extensions expand the type grammar to:

         T = scalar (...)                  // Scalars (basic types s.a. primitive types)
           | X                             // Type variable
           | Id<P0..Pn>                    // Nominal type (struct, enum)
           | &r T                          // Reference (mut doesn't matter here)
           | O0..On+r                      // Object type
           | [T]                           // Slice type
           | for<r..> fn(T1..Tn) -> T0     // Function pointer
           | <P0 as Trait<P1..Pn>>::Id     // Projection
    +      | C                             // const types
    +    F = c                             // const fn name
    +    C = E                             // Pi constructed const type
         P = r                             // Region name
           | T                             // Type
         O = for<r..> TraitId<P1..Pn>      // Object type fragment
         r = 'x                            // Region name
    +    E = F(E)                          // Constant function application.
    +      | p                             // const type parameter
    +      | [...]                         // etc.

Note that the `const` syntax is only used when declaring the parameter.

## `impl` unification

Only one `with` bound can be specified on each disjoint implementations (for
possible extensions, see below). In other words, no overlap is allowed, even if
the `with` bounds are mutually exclusive.

To find the right implementation, we use the data from the type inference (see
the inference rules above). Since the parameters are, in fact, not much
semantically different from normal generic parameters, we can resolve it in a
normal manner (that is, by treating the value parameters as if they were actual
type parameters).

Likewise are disjointness checks based on structural equality. That is, we only
care about structural equality, not `Eq` or something else. This allows us to
reason more rigorously about the behavior.

Any non-identity-related term is threated as an unknown parameter, since reasoning about uniqueness of those is undecidable. For example,

```rust
impl<const x: usize> Trait<x * x> for Struct<x> with some_fn(x)
```

is, when checking for implementation uniqueness, semantically behaving like

```rust
impl<const x: usize, const y: usize> Trait<y> for Struct<x>
```

since we cannot prove injectivity. Note that this is only about behavior under
_uniqueness checking_.

Since not all parameters' edges are necessarily the identity function,
dispatching these would be undecidable. A way to solve this problem is to
introduce some syntax allowing to specify the `impl` parameters. This is not
something we consider in this proposal, but a secondary RFC can introduce these.

## Division by zero

If some function contain a constexpr divisor, dependent on some value parameter
of the function, that is (`a / f(x)`), the compiler must ensure that the bound
implies that `f(x) != 0`.

## Parsing

Originally, it was proposed to use `where` bounds. To avoid ambiguities, we
changed this to `with` instead.

I`with` bounds must be before, the `where` bounds (if any).

## An example

This is the proposed syntax:

```rust
use std::{mem, ptr};

// We start by declaring a struct which is value dependent.
struct Array<const n: usize, T> {
    // `n` is a constexpr, sharing similar behavior with `const`s, thus this
    // is possible.
    content: [T; n],
}

// We are interested in exploring the `with` clauses and Π-constructors:
impl<const n: usize, T> Array<n, T> {
    // This is simple statically checked indexing.
    fn checked_index<const i: usize>(&self) -> &T with i < n {
        //                 note that this is constexpr  ^^^^^
        unsafe { self.content.unchecked_index(i) }
    }

    // "Push" a new element, incrementing its length **statically**.
    fn push(self, elem: T) -> Array<n + 1, T> {
        let mut new: [T; n + 1] = mem::uninitialized();
        //               ^^^^^ constexpr
        unsafe {
            ptr::copy(self.content.as_ptr(), new.as_mut_ptr(), n);
            ptr::write(new.as_mut_ptr().offset(n), elem);
        }

        // Don't call destructors.
        mem::forget(self.content);

        // So, the compiler knows the type of `new`. Thus, it can easily check
        // if the return type is matching. By siply evaluation `n + 1`, then
        // comparing against the given return type.
        Array { content: new }
    }
}

fn main() {
    let array: Array<2, u32> = Array { content: [1, 2] };

    assert_eq!(array.checked_index::<0>(), 1);
    assert_eq!(array.checked_index::<1>(), 2);
    assert_eq!(array.push(3).checked_index::<2>(), 3);
}
```

# Experimental extensions open to discussion

## Remark!

These are _possible_ extensions, and not something that would be a part of the
initial implementation. These a brought up for the sake of discussion.

## SMT-solvers?

This RFC doesn't propose such thing, but future possibilities are worth discussing:

### What a Rusty SMT-solver would look like

The simplest and least obstructive SMT-solver is the SAT-based one. SAT is a
class of decision problem, where a boolean formula, with some arbitrary number
of free variables, is determined to be satisfiable or not. Obviously, this is
decidable (bruteforcing is the simplest algorithm, since the search space is
finite, bruteforcing is guaranteed to terminate).

SAT is NP-complete, and even simple statements such as `x + y = y + x` can take
a long time to prove. A non-SAT (symbolic) SMT-solver is strictly more
expressive, due to not being limited to finite integers, however first-order
logic is not generally decidable, and thus such solvers are often returning
"Satisfiable", "Not satisfiable", "Not known".

In general, such algorithms are either slow or relatively limited. An example
of such a limitation is in the [Dafny
language](https://github.com/Microsoft/dafny), where programs exist that
compile when having the bound `a \/ b`, but fails when having the bound `b \/
a`. This can be relatively confusing the user.

It is worth noting that the technology on this area is still improving, and
these problems will likely be marginalized in a few years.

Another issue which is present in Rust, is that you don't have any logical
(invariant) information about the return values. Thus, a SMT-solver would work
relatively poorly (if at all) non-locally (e.g. user defined functions). This
is often solved by having an expression of "unknown function", which can have
any arbitrary body.

That issue is not something that prevents us from adopting a SMT-solver, but it
limits the experience with having one.

### Backwards compatibility

While I am against adding SMT-solvers to `rustc`, it is worth noting that this
change is, in fact, compatible with future extensions for more advanced theorem
provers.

The only catch with adding a SMT-solver is that errors on unsatisfiability or
contradictions would be a breaking change. By throwing a warning instead, you
essentially get the same functionality.

### Implementation complications

It will likely not be hard to implement itself, by using an external SMT-solver
(e.g., Z3). The real problem lies in the issues with performance and
"obviousness" of the language.

## Candidates for additional rules

### Propositional logic

Currently, the set of rules is rather conservative for rewriting. To make it
easier to work with, one can add multiple new reductive rules, at the expense
of implementation complexity:

    RewriteOr:
      P ∨ Q
      ──────────
      ¬(¬P ∧ ¬Q)

This rule states that if `a` nor `b`, none of them can be true. It allows us to
rewrite OR in terms of NOT and AND.

`RewriteOr` does not reduce depth. In fact, it does increase depth, but that
rule is only triggered by `∨`, which no other rules infer. Thus, there is no
way, we can enter a cycle, since `RewriteOr(P)` is a reduction of `P` with
respect to `∨`.

    DisjunctiveSyllogism:
      ¬(P ∧ Q)
      P
      ────────
      ¬Q

Basically, this states that if two propositions are mutually exclusive (that
is, not both of them can be true), and one of them is true, the other must be
false, due to being disjunctive.

This is strictly reductive.

Now let's go funky:

### Cancelation

    AdditiveCancelationRR:
      a + c = b + c
      ─────────────
      a = b
    AdditiveCancelationLL:
      c + a = c + b
      ─────────────
      a = b
    AdditiveCancelationRL:
      a + c = c + b
      ─────────────
      a = b
    AdditiveCancelationLR:
      c + a = b + c
      ─────────────
      a = b
    MultiplicativeCancelationRR:
      ac = bc
      ─────────────
      a = b
    MultiplicativeCancelationLL:
      ca = cb
      ─────────────
      a = b
    MultiplicativeCancelationRL:
      ac = cb
      ─────────────
      a = b
    MultiplicativeCancelationLR:
      ca = bc
      ─────────────
      a = b

These are all reductive.

### Inequalities

Inequalities are something, we are interested in simplifying. These carry many
interesting properties, which shall be covered:

    RewriteGeq:
      a ≥ b
      ─────
      b ≤ a

Here, we rewrite greater than or equal to a form of less than or equal.

    RewriteLessThan:
      a < b
      ────────
      a ≤ b
      ¬(a = b)
    RewriteGreaterThan:
      a > b
      ────────
      a ≥ b
      ¬(a = b)

This allows us to rewrite less than (`<`) and greater than (`>`), in terms of
their equality accepting versions.

    LeqAdditiveCancelationRR:
      a + c ≤ b + c
      ─────────────
      a ≤ b
    LeqAdditiveCancelationLL:
      c + a ≤ c + b
      ─────────────
      a ≤ b
    LeqAdditiveCancelationRL:
      a + c ≤ c + b
      ─────────────
      a ≤ b
    LeqAdditiveCancelationLR:
      c + a ≤ b + c
      ─────────────
      a ≤ b
    LeqMultiplicativeCancelationRR:
      ac ≤ bc
      ─────────────
      a ≤ b
    LeqMultiplicativeCancelationLL:
      ca ≤ cb
      ─────────────
      a ≤ b
    LeqMultiplicativeCancelationRL:
      ac ≤ cb
      ─────────────
      a ≤ b
    LeqMultiplicativeCancelationLR:
      ca ≤ bc
      ─────────────
      a ≤ b

These are known as the cancelation laws, and are essentially the inequality
version, of those stated for equal.

    LeqNegation:
      ¬(a ≤ b)
      ────────
      a > b

This allows us to define the _negation_ of less than or equals to.

---

Unfortunately, transitivity is not reductive.

### Other relational definitions

Non-equality is defined by:

    NeqDefinition:
      a ≠ b
      ────────
      ¬(a = b)

## Expression reduction rules

We might want to have expression reduction rules beyond the basic const
folding. This would allow certain symbolic comparation to improve.

    DistributiveMultiplicationLhs:
      c(a + b) ↦ ca + cb
    DistributiveMultiplicationRhs:
      (a + b)c ↦ ca + cb

This is simply the distributive property of multiplication.

    AdditionLeftAssociate:
      a + (b + c) ↦ (a + b) + c
    MultiplicationLeftAssociate:
      a(bc) ↦ (ab)c

This rules allows us to observe that `a(bc)` is no different from `(ab)c`

Lastly, we are interested in rewriting subtraction in terms of addition:

    SubtractionToAddition:
      a - b ↦ a + (-b)

All these rules are reductive.

## "Exit-point" identities

These are simply identities which always holds. Whenever the compiler reaches one
of these when unfolding the `with` clause, it returns "True":

    LeqReflexive:
        f(x) ≤ f(x) for x primitive integer
    GeqReflexive:
        f(x) ≥ f(x) for x primitive integer
    EqReflexive:
        f(x) = f(x)
    NegFalseIsTrue:
        ¬false
    TrueAndTrue:
        true ∧ true
    OrTrue1:
        P ∨ true
    OrTrue2:
        true ∨ P

# Alternatives
[alternatives]: #alternatives

## Use purely symbolic `with` clause checking

We can simplify things somewhat, by using a purely symbolic model of
implication. Say that a set of clause, `A`, implies a set of clause `B`, iff.
`B ⊆ A`.

## Allow multiple implementation bounds

Allow overlapping implementations carrying bounds, such that only one of the
conditions may be true under monomorphization.

## Type/`with` clause checking

### Lazily type check without transitivity rule

Simply evaluate the bounds when calling. Remove the requirement of implication.
This introduces errors at monomorphization time.

### Inheriting `with` clauses

An interesting idea to investigate is to let functions inherit called
function's `with` clauses. This allows for non-monomorphization, yet
ergonomic, `with` clauses.

# Unresolved questions
[unresolved]: #unresolved-questions

Are there other rules to consider?

How can one change bounds without breaking downstream? Shall some form of
judgemental OR be added?

# How we teach this

**What are the edge cases, and how can one work around those (e.g. failed
  unification)?**

If you use this a lot, you will likely encounter edge cases, where the
compiler isn't able to figure out implication, since the reductive rules are
dumb. However, there is hope! Say your function calls some function, where
the compiler cannot prove the bound. You can work around this by simply
adding the called function's `with` bound to the caller's `with` bound.
While, this is a minor annoyance, working around it is relatively easy.
