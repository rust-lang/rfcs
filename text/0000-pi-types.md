- Feature Name: pi-types
- Start Date: 2016-06-22
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

We propose a simple, yet sufficiently expressive, addition of dependent-types
(also known as, Π-types and value-types).

Type checking will not require SMT-solvers or other forms of theorem provers.

## Generic value parameters

A `const` type parameter acts like a generic parameter, containing a constant
expression. Declaring a generic parameter `const a: usize`, declares a constant
variable `a` of type `usize`.

One can create implementations, structs, enums, and traits, abstracting over
this generic value parameter.

Such a parameter acts type-like in the context of types, generics, and
polymorphism, and value-like in the context of expressions, function bodies,
and applications.

## Compile time calculations on constant parameters

Since it is simply consisting of constexprs, one can apply constant functions
(`const fn`) to the parameter, to perform compile time, type level calculations
on the parameter. This allows for great expressiveness as `const fn` improves.

## Expression `where` bounds

The second construct added is the constant expression in `where` bounds. These
contains statements about the constant parameters, which are checked at compile
time.

## Type checking

Type checking is done by using a [transitive rule](#transitivity-of-bounds), such that `where` bounds must
be implied by the caller.

# Motivation
[motivation]: #motivation

An often requested feature is the "type-level numerals", which enables generic
length arrays. The current proposals are often limited to integers or even lack
of value maps, and other critical features.

There is a whole lot of other usecases as well. These allows certain often
requested features to live in standalone libraries (e.g., [bounded-integers](#bounded-integersinterval-arithmetics),
[type level numerals](#array-generics), [statically checked indexing](#statically-checked-indexing), lattice types).

It allows for creating powerful abstractions without type-level hackery.

## What we want, and what we don't want

We have to be very careful to avoid certain things, while still preserving the core features:

1. Ability to use, manipulate, and constraint values at type-level.
2. The ability to use said values on expression-level (runtime).

Yet, we do not want:

1. SMT-solvers, due to not only undecidability (note, although, that SAT is
   decidable) and performance, but the complications it adds to `rustc`.
2. Monomorphisation-time errors, i.e. errors that happens during codegen of
   generic functions. We try to avoid adding _more_ of these (as noted by
   petrochenkov, these [already exists](https://github.com/rust-lang/rfcs/pull/1657#discussion_r68202733))

## Examples

### Bounded integers/interval arithmetics

One can define the so called "bounded integers" (integers which carry an
upper and lower bound, checked at compile time):

```rust
use std::ops;

/// A bounded integer.
///
/// This has two value parameter, respectively representing an upper and a lower bound.
pub struct BoundedInt<const lower: usize, const upper: usize>
    // Compile time constraints.
    where lower <= upper {
    /// The inner runtime value.
    n: usize,
}

// To see how this holds the `where` clause above, see the section on `identities`.
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
     where lower_a <= upper_a,
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
     where lower_a <= upper_a,
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

impl<const a: usize, const b: usize> Divides<b> for Num<a> where b % a == 0 {}
```

# Detailed design
[design]: #detailed-design

## The new value-type construct, `const`

Declaring a parameter `const x: T` allows using `x` in both an expression context
(as a value of type `T`) and a type context (as a type parameter). In a sense,
const "bridges" the world between values and types, since it allows us to
declare value dependent types ([`ε → τ` constructors](https://en.wikipedia.org/wiki/Dependent_type)).

Such a parameter is declared, like type parameters, in angle brackets (e.g.
`struct MyStruct<const x: usize>`).

The expr behavior is described as:

    ValueParameterDeclaration:
      Π ⊢ const x: T
      ──────────────
      Π ⊢ x: T

In human language, this simply means that one can use a constant parameter,
`const x: T`, in expression context, as a value of type `T`.

On the type level, we use the very same semantics as the ones generic
parameters currently follows.

## `const fn`s as Π-constructors

We are interested in value dependency, but at the same time, we want to avoid
complications such as [SMT-solvers](https://en.wikipedia.org/wiki/Satisfiability_modulo_theories).

We achieve this by `const fn`, which allows us to take some const parameter and
map it by some arbitrary, pure function, following the rules described in [RFC
0911](https://github.com/rust-lang/rfcs/blob/master/text/0911-const-fn.md#detailed-design).

## Type inference

Since we are able to evaluate the function at compile time, we can easily infer
const parameters, by adding an unification relation, simply

    PiRelationInference
      Γ ⊢ y = f(x)
      Γ ⊢ T: U<y>
      ──────────────
      Γ ⊢ T: U<f(x)>

Informally, this means that, you can substitute equal terms (in this case, `const fn` relations).

The relational edge between two const parameters is simple a const fn, which is
resolved under unification.

We add an extra rule to improve inference:

    DownLiftEquality:
      Γ ⊢ T: A → 𝓤
      Γ ⊢ c: A
      Γ ⊢ x: A
      Γ ⊢ a: T<c>
      Γ ⊢ a: T<x>
      ────────────
      Γ ⊢ c = x

So, if two types share constructor by some Π-constructor, share a value, their
value parameter is equal. Take `a: [u8; 4]` as an example. If we have some
unknown variable `x`, such that `a: [u8; x]`, we can infer that `x = 4`.

This allows us to infer e.g. array length parameters in functions:

```rust
// [T; N] is a constructor, T → usize → 𝓤 (parameterize over T and you get A → 𝓤).
fn foo<const n: usize, const l: [u32; n]>() -> [u32; n] {
    // ^ note how l depends on n.
    l
}

// We know n from the length of the array.
let l = foo::<_, [1, 2, 3, 4, 5, 6]>();
//            ^   ^^^^^^^^^^^^^^^^
```

## `where` clauses

Often, it is wanted to have some statically checked clause satisfied by the
constant parameters (e.g., for the sake of compile-time bound checking). To
archive this, in a reasonable manner, we use const exprs, returning a boolean.

We allow such constexprs in `where` clauses of functions. Whenever the
function is invoked given constant parameters `<a, b...>`, the compiler
evaluates this expression, and if it returns `false`, an aborting error is
invoked.

To sum up, the check happens when typechecking the function calls (that is,
checking if the parameters satisfy the trait bounds). The caller's bounds must
imply the invoked functions' bounds:

## Structural equality

Structural equality plays a key role in type checking of dependent types.

Structural equality, in this case, is defined as an equivalence relation, which
allows substitution without changing semantics.

Any constant parameter must have the `structural_match` property as defined in
[RFC #1445](https://github.com/rust-lang/rfcs/pull/1445). This property, added
through the `#[structural_match]` attribute, essentially states that the `Eq`
implementation is structural.

Without this form of equality, substitution wouldn't be possible, and thus
typechecking an arbitrarily value-depending type constructor would not be
possible.

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
      ──────
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
2. `SubstituteEquality` eliminates `{a = b, P} ⊢ {P[b/a]}`, which is an
   elimination, since `dep(P) + 1 > dep(P[b/a]) = dep(P)`.
3. `DoubleNegation` eliminates `{¬¬x} ⊢ {x}`, which is an elimination, since
   `dep(x) + 2 > dep(x)`.

In fact, this set of rule is strictly reductive (like equality-based unification).

#### An example

We will quickly give an example of a possible proof. Say we want to show that
`x = b ∧ x < a ⇒ b < a`. Starting with the left hand side, we can sequentially
prove this, by simple unification (which already exists in the Rust type
checker):

    x = b ∧ ¬¬(x < a)
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

### SMT-solvers?

This RFC doesn't propose such thing, but future possibilities are worth discussing:

#### What a Rusty SMT-solver would look like

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
relatively poorly (if at all) non-locally (e.g. user defined functions).

That issue is not something that prevents us from adopting a SMT-solver, but it
limits the experience with having one.

#### Backwards compatibility

While I am against adding SMT-solvers to `rustc`, it is worth noting that this
change is, in fact, compatible with future extensions for more advanced theorem
provers.

The only catch with adding a SMT-solver is that errors on unsatisfiability or
contradictions would be a breaking change. By throwing a warning instead, you
essentially get the same functionality.

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

Only one `where` bound can be specified on each disjoint implementations (for
possible extensions, see below). In other words, no overlap is allowed, even if
the `where` bounds are mutually exclusive.

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
impl<const x: usize> Trait<x * x> for Struct<x> where some_fn(x)
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

// We are interested in exploring the `where` clauses and Π-constructors:
impl<const n: usize, T> Array<n, T> {
    // This is simple statically checked indexing.
    fn checked_index<const i: usize>(&self) -> &T where i < n {
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

## Candidates for additional rules

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
      ─────
      ¬Q

Basically, this states that if two propositions are mutually exclusive (that
is, not both of them can be true), and one of them is true, the other must be
false, due to being disjunctive.

This is strictly reductive.

Now let's go funky:

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

All these rules are reductive.

## "Exit-point" identities

These are simply identities which always holds. Whenever the compiler reaches one
of these when unfolding the `where` clause, it returns "True":

    LeqReflexive:
        f(x) <= f(x) for x primitive integer
    GeqReflexive:
        f(x) >= f(x) for x primitive integer
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

### A constexpr type constructor

Add some language item type constructor, `Const<T>`, allowing for constructing
a constexpr-only types.

`x: T` can coerce into `Const<T>` if `x` is constexpr. Likewise, can `Const<T>`
coerce into `T`.

```rust
fn do_something(x: Const<u32>) -> u32 { x }

struct Abc {
    constfield: Const<u32>,
}
```

It is unclear how it plays together with `where` bounds.

The pro is that it adds ability to implement e.g. constant indexing, `Index<Const<usize>>`.

# How we teach this

This RFC aims to keep a "symmetric" syntax to the current construct, giving an
intuitive behavior, however there are multiple things that are worth explaining
and/or clearing up:

**What are dependent types?**

Dependent types are types, which _depend_ on values, instead of types. For
example, [T; 3], is dependent since it depends on the value, `3`, for
constructing the type. Dependent types, in a sense, are similar to normal
generics, where types can depend on other types (e.g. `Vec<T>`), whereas
dependent types depend on values.

**How does this differ from other languages' implementations of dependent types?**

Various other languages have dependent type systems. Strictly speaking, all
that is required for a dependent type system is value-to-type constructors,
although some languages (coq, agda, etc.) goes a step further and remove the
boundary between value and type. Unfortunately, as cool as it sounds, it has
some severe disadvantages: most importantly, the type checking becomes
undecidable. Often you would need some form of theorem prover to type check
the program, and those have their limitations too.

**What are `const fn` and how is it linked to this RFC?**

`const fn` is a function, which can be evaluated at compile time. While it
is currently rather limited, in the future it will be extended (see
[Miri](https://github.com/solson/miri)). You can use constexprs to take one
type-level value, and non-trivially calculate a new one.

**What are the usecases?**

There are many usecases for this. The most prominent one, perhaps, is
abstracting over generically sized arrays. Dependent types allows one to lift
the length of the array up to the type-level, effectively allowing one to
parameterize over them.

**What are the edge cases, and how can one work around those (e.g. failed
  unification)?**

If you use this a lot, you will likely encounter edge cases, where the
compiler isn't able to figure out implication, since the reductive rules are
dumb. However, there is hope! Say your function calls some function, where
the compiler cannot prove the bound. You can work around this by simply
adding the called function's `where` bound to the caller's `where` bound.
While, this is a minor annoyance, working around it is relatively easy.

**How can I use this to create powerful abstractions?**

...

**Can you show some more extensive examples?**

Refer to the rest of the RFC.

- - - -

Moreover, we need to "informalize" the rules defined in this RFC.

I believe this subject is complex enough to have its own chapter in The Holy
Book, answering these questions in detail.

Lastly, the FAQ will need to be updated to answer various questions related to
this.

# Drawbacks
[drawbacks]: #drawbacks

If we want to have type-level Turing completeness, the halting problem is
inevitable. One could "fix" this by adding timeouts, like the current recursion
bounds.

Another drawback is the lack of implication proves.

# Alternatives
[alternatives]: #alternatives

## Use purely symbolic `where` clause checking

We can simplify things somewhat, by using a purely symbolic model of
implication. Say that a set of clause, `A`, implies a set of clause `B`, iff.
`B ⊆ A`.

## SMT

Use full SMT-based dependent types. These are more expressive, but severely
more complex as well.

## Alternative syntax

The syntax is described above is, in fact, ambiguous, and multiple other better
or worse candidates exists:

### Blending the value parameters into the arguments

This one is an interesting one. It allows for defining functions with constant
_arguments_ instead of constant _parameters_. This allows for bounds on e.g.
`atomic::Ordering`.

```rust
fn do_something(const x: u32) -> u32 where x < 5 { x }
```

From the callers perspective, this one is especially nice to work with, however
it can lead to confusion about mixing up constargs and runtime args. One
possible solution is to segregate the constargs from the rest arguments by a
`;` (like in array types).

Another way to semantically justify such a change is by the [`Const` type constructor](#an-extension-a-constexpr-type-constructor)

### Square brackets

Use square brackets for dependent parameters:

```rust
fn do_something[x: u32]() -> u32 where x < 5 { x }

do_something::[2]();
```

### `const` as an value-type constructor

Create a keyword, `const`:

```rust
fn do_something<x: const u32>() -> u32 where x < 5 { x }

do_something::<2>();
```

### `with` instead of `where`

Some have raised concerns of mixing things up there. Thus one can use the
syntax `with` to denote bounds instead.

## Allow multiple implementation bounds

Allow overlapping implementations carrying bounds, such that only one of the
conditions may be true under monomorphization.

## Type/`where` clause checking

### Lazily type check without transitivity rule

Simply evaluate the bounds when calling. Remove the requirement of implication.
This introduces errors at monomorphization time.

### Inheriting `where` clauses

An interesting idea to investigate is to let functions inherit called
function's `where` clauses. This allows for non-monomorphization, yet
ergonomic, `where` clauses.

An important symmetry is lost, though: trait bounds in `where` clauses. These
are not inherited, but explicit. A way to avoid this asymmetry, [`with` could
be used](#with-instead-of-where).

# Unresolved questions
[unresolved]: #unresolved-questions

## Syntactical/conventional

What syntax is preferred?

What should be the naming conventions?

Should we segregate the value parameters and type parameters by `;`?

## Compatibility

How does this play together with HKP?

How can one change bounds without breaking downstream? Shall some form of
judgemental OR be added?

What API would need to be rewritten to take advantage of Π-types?

## Features

Should there be a way to parameterize functions dynamically?

## Semantics

Find some edge cases, which can be confusing.

Are there other rules to consider?
