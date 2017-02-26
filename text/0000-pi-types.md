- Feature Name: pi-types
- Start Date: 2017-02-26
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

# Motivation
[motivation]: #motivation

An often requested feature is the "type-level numerals", which enables generic
length arrays. The current proposals are often limited to integers or even lack
of value maps, and other critical features.

_Note:_ In an earlier version of this RFC, a `where` bound was proposed, but it
proved too complex, so for now, types with invariants about the constant
parameter can be used.

It allows for creating powerful abstractions without type-level hackery.

## What we want, and what we don't want

We have to be very careful to avoid certain things, while still preserving the core features:

1. Ability to use and manipulate values at type-level.
2. The ability to use said values on expression-level (runtime).

Yet, we do not want:

1. SMT-solvers, due to not only undecidability (note, although, that SAT is
   decidable) and performance, but the complications it adds to `rustc`.
2. Monomorphisation-time errors, i.e. errors that happens during codegen of
   generic functions. We try to avoid adding _more_ of these (as noted by
   petrochenkov, these [already exists](https://github.com/rust-lang/rfcs/pull/1657#discussion_r68202733))

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

## Parsing

Introducing expr subgrammar in type position isn't possible without setting
some restrictions to the possible expressions.

In this case, we put restrictions to arithmetic and bitwise operations (`+-/!^`
etc.) and function calls (`myconstfn(a, b, c, ...)` with `a, b, c, ...` being
`const-expr` fragments).

Additionally, we allow parenthesizes, which can contain any general const-expr
fragment.

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

# Drawbacks
[drawbacks]: #drawbacks

If we want to have type-level Turing completeness, the halting problem is
inevitable. One could "fix" this by adding timeouts, like the current recursion
bounds.

Another drawback is the lack of implication proves.

# Alternatives
[alternatives]: #alternatives

## Alternative syntax

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

The pro is that it adds ability to implement e.g. constant indexing, `Index<Const<usize>>`.

The syntax is described above is, in fact, ambiguous, and multiple other better
or worse candidates exists:

### Blending the value parameters into the arguments

This one is an interesting one. It allows for defining functions with constant
_arguments_ instead of constant _parameters_. This allows for bounds on e.g.
`atomic::Ordering`.

```rust
fn do_something(const x: u32) -> u32 { x }
```

From the callers perspective, this one is especially nice to work with, however
it can lead to confusion about mixing up constargs and runtime args. One
possible solution is to segregate the constargs from the rest arguments by a
`;` (like in array types).

Another way to semantically justify such a change is by the [`Const` type constructor](#an-extension-a-constexpr-type-constructor)

### Square brackets

Use square brackets for dependent parameters:

```rust
fn do_something[x: u32]() -> u32 { x }

do_something::[2]();
```

### `const` as an value-type constructor

Create a keyword, `const`:

```rust
fn do_something<x: const u32>() -> u32 { x }

do_something::<2>();
```
# Unresolved questions
[unresolved]: #unresolved-questions

## Syntactical/conventional

What syntax is preferred?

What should be the naming conventions?

Should we segregate the value parameters and type parameters by `;`?

## Compatibility

How does this play together with HKP?

What API would need to be rewritten to take advantage of Π-types?

## Features

Should there be a way to parameterize functions dynamically?

## Semantics

Find some edge cases, which can be confusing.
