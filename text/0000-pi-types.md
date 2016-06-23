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

Type checking is done by using a transitive rule, such that `where` bounds must
be implied by the caller.

# Motivation
[motivation]: #motivation

An often requested feature is the "type-level numerals", which enables generic
length arrays. The current proposals are often limited to integers or even lack
of value maps, and other critical features.

There is a whole lot of other usecases as well. These allows certain often
requested features to live in standalone libraries (e.g., bounded-integers,
type level arithmetics, lattice types).

It allows for creating powerful abstractions without type-level hackery.

# Detailed design
[design]: #detailed-design

## The new value-type construct, `const`

The first construct, we will introduce one allowing us to declare `ε → τ`
constructors (value dependent types).

In a sense, `const` "bridges" between types and values.

Declaring a parameter `const x: T` allows use in both expression context (as a
value of type `T`) and type context (as a type parameter).

Such a parameter is declared, like type parameters, in angle brackets.

The expr behavior is described as:

    ValueParameterDeclaration:
      Π ⊢ const x: T
      ──────────────
      Π ⊢ x: T

On the type level, we use the very same semantics as the ones generic
parameters currently follows.

## `const fn`s as Π-constructors

We are interested in value dependency, but at the same time, we want to avoid
complications such as SMT-solvers and so on.

Thus, we follow a purely constructible model, by using `const fn`s.

This allows one to take some const parameter and map it by some arbitrary, pure
function, following the rules described in [RFC 0911](https://github.com/rust-lang/rfcs/blob/master/text/0911-const-fn.md#detailed-design).

## Type inference

Since we are able to evaluate the function at compile time, we can easily infer
const parameters, by adding an unification relation, simply

    PiRelationInference
      Γ ⊢ y = f(x)
      Γ ⊢ T: U<y>
      ──────────────
      Γ ⊢ T: U<f(x)>

The relational edge between two const parameters is simple a const fn, which is
resolved under unification.

We add an extra rule to improve inference:

    DownLiftEquality:
      Γ ⊢ T: A → 𝓤
      Γ ⊢ c: A
      Γ ⊢ x: A
      Γ ⊢ a: T<c>
      Γ ⊢ a: T<x>
      ──────────────
      Γ ⊢ c = x

So, if two types share constructor by some Π-constructor, share a value, their
value parameter is equal.

This allows us infer:

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
constant parameters. To archive this, in a reasonable manner, we use const
exprs, returning a boolean.

We allow such constexprs in `where` clauses of functions. Whenever the
function is invoked given constant parameters `<a, b...>`, the compiler
evaluates this expression, and if it returns `false`, an aborting error is
invoked.

To sum up, the check happens when typechecking the function calls (that is,
checking if the parameters satisfy the trait bounds, and so on). The caller's
bounds must imply the invoked functions' bounds:

### Transitivity of bounds.

We require a bound of a function to imply the bounds of the functions it calls,
through a simple unification + alpha-substitution algorithm.

The compiler would then enforce that if `f` calls `g`, `unify(bound(g))	⊆
unify(bound(f))` (by structural equality):

    ExpandBooleanAnd:
      P && Q
      ──────
      P
      Q
    SubstituteEquality:
      P(a)
      a = b
      ─────
      P(b)

These rules are "exhausting" (recursing downwards the tree and decreasing the
structure), and thus it is possible to check, in this language, that `a ⇒ b`
relatively quickly (`O(n)`).

More rules can be added in the future (e.g., reflexive equality, common true
statements, etc.). It is however important to preserve the "sequential
property" (that is, each step is a reduction, not an expansion), allowing one
to check the implication in linear time.

This is done under type unification. Thus, we only need to check the bounds at
the top level.

#### An example

We will quickly give an example of a possible proof. Say we want to show that
`x = b && x < a ⇒ b < a`. Starting with the left hand side, we can sequentially
prove this, by simple unification (which already exists in the Rust type
checker):

    x = b && x < a
    ∴ x = b  (ExpandBooleanAnd)
      x < a
    ∴ b < a  (SubstituteEquality)
      ¯¯¯¯¯

### Contradictive or unsatisfiable bounds

Contradictive or unsatisfiable bounds (like `a < b, b < a`) cannot be detected,
since such a thing would be undecidable.

These bounds doesn't break anything, they are simply malformed and unreachable.

Take `a < b, b < a` as an example. We know the values of `a` and `b`, we can
thus calculate the two bounds, which will clearly fail. We cannot, however,
stop such malformed bounds in _declarations_ and _function definitions_, due to
mathematical limitations.

## The grammar

These extensions expand the type grammar to:

         T = scalar (i32, u32, ...)        // Scalars
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
possible extensions, see below).

To find the right implementation, we use the data from the type inference (see
the inference rules above). Since the parameters are, in fact, not much
semantically different from normal generic parameters, we can resolve it is a
normal manner.

Likewise are disjointness checks based on structural equality.

Since not all parameters' edges are necessarily the identity function,
dispatching these would be undecidable. A way to solve this problem is to
introduce some syntax allowing to specify the `impl` parameters.

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

# How we teach this

This RFC aims to keep a "symmetric" syntax to the current construct, giving an
intuitive behavior, however there are multiple things that are worth explaining
and/or clearing up:

1. What are Π-types? How are they different from those from other languages?

2. One will have to understand what `const fn`s are and how they are linked to
   Π-types.

3. What are the usecases?

4. What are the edge cases, and how can one work around those (e.g. failed
   unification)?

5. How can I use this to create powerful abstractions?

6. Extensive examples.

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

From the callers perspective, this one is especially nice to work with.

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

### An extension: A constexpr type constructor

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

### `with` instead of `where`

Some have raised concerns of mixing things up there. Thus one can use the
syntax `with` to denote bounds instead.

### Lazily type check without transitivity rule

Simply evaluate the bounds when calling. Remove the requirement of implication.

### Allow multiple implementation bounds

Allow overlapping implementations carrying bounds, such that only one of the
conditions may be true under monomorphization.

# Unresolved questions
[unresolved]: #unresolved-questions

What syntax is preferred?

How does this play together with HKP?

What should be the naming conventions?

Should we segregate the value parameters and type parameters by `;`?

Should disjoint implementations satisfying some bound be allowed?

Should there be a way to parameterize functions dynamically?

What API would need to be rewritten to take advantage of Π-types?
