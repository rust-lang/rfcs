- Feature Name: pi-types
- Start Date: 2016-06-22
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

We propose a simple, yet sufficiently expressive, addition of dependent-types
(also known as, Π-types and value-types).

Type checking remains decidable.

# Motivation
[motivation]: #motivation

An often requested feature is the "type-level numerals", which enables generic
length arrays. The current proposals are often limited to integers or even lack
of value maps, and other critical features.

There is a whole lot of other usecases as well. These allows certain often
requested features to live in standalone libraries (e.g., bounded-integers,
type level arithmetics, lattice types).

# Detailed design
[design]: #detailed-design

## The new value-type construct, `const`

The first construct, we will introduce is `ε → τ` constructor, `const`. All this
does is taking a const-expr (struct construction, arithmetic expression, and so
on) and constructs a _type-level version_ of this.

In particular, we extend the type grammar with an additional `const C`, a type
whose semantics can be described as follows,

    ValueTypeRepresentation:
      Π ⊢ x: const c
      --------------
      Π ⊢ x = c

In other words, if `x` has type `const c`, its value _is_ `c`. That is, any
constexpr, `c`, will either be of its underlying type or of the type, `const
c`.

It is important to understand that values of `const c` are constexprs, and
follows their rules.

## `const fn`s as Π-constructors

We are interested in value dependency, but at the same time, we want to avoid
complications such as SMT-solvers and so on.

Thus, we follow a purely constructible model, by using `const fn`s.

Let `f` be a `const fn` function. From the rules of `const fn`s and constexprs,
we can derive the rule,

    PiConstructorInference:
      Π ⊢ x: const c
      Π ⊢ f(c): τ
      --------------
      Π ⊢ f(x): const τ

This allows one to take some const parameter and map it by some arbitrary, pure
function.

## Type inference

Since we are able to evaluate the function on compile time, we can easily infer
const types, by adding an unification relation, from the rule above.

The relational edge between two const types is simple a const fn, which is
resolved under unification.

## `where` clauses

Often, it is wanted to have some statically checked clause satisfied by the
constant parameters. To archive this, in a reasonable manner, we use const
exprs, returning a boolean.

We allow such constexprs in `where` clauses of functions. Whenever the
function is invoked given constant parameters `<a, b...>`, the compiler
evaluates this expression, and if it returns `false`, an aborting error is
invoked.

## The type grammar

These extensions expand the type grammar to:

    T = scalar (i32, u32, ...)        // Scalars
      | X                             // Type variable
      | Id<P0..Pn>                    // Nominal type (struct, enum)
      | &r T                          // Reference (mut doesn't matter here)
      | O0..On+r                      // Object type
      | [T]                           // Slice type
      | for<r..> fn(T1..Tn) -> T0     // Function pointer
      | <P0 as Trait<P1..Pn>>::Id     // Projection
      | C                             // const types
    F = c                             // const fn name
    C = E                             // Pi constructed const type
    P = r                             // Region name
      | T                             // Type
    O = for<r..> TraitId<P1..Pn>      // Object type fragment
    r = 'x                            // Region name
    E = F(E)                          // Constant function application.
      | p                             // const type parameter
      | [...]                         // etc.

Note that the `const` prefix is only used when declaring the parameter.

## An example

This is the proposed syntax:

```rust
use std::{mem, ptr};

// We start by declaring a struct which is value dependent.
struct Array<n: const usize, T> {
    // `n` is a constexpr, sharing similar behavior with `const`s, thus this
    // is possible.
    content: [T; n],
}

// We are interested in exploring the `where` clauses and Π-constructors:
impl<n: const usize, T> Array<n, T> {
    // This is simple statically checked indexing.
    fn const_index<i: const usize>(&self) -> &T where i < n {
    //                        note that this is constexpr  ^^^^^
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

    assert_eq!(array.const_index::<0>(), 1);
    assert_eq!(array.const_index::<1>(), 2);
    assert_eq!(array.push(3).const_index::<2>(), 3);
}
```

# Drawbacks
[drawbacks]: #drawbacks

If we want to have type-level Turing completeness, the halting problem is
inevitable. One could "fix" this by adding timeouts, like the current recursion
bounds.

Another draw back is the lack of implication proves.

# Alternatives
[alternatives]: #alternatives

Use full SMT-based dependent types. These are more expressive, but severely
more complex as well.

# Unresolved questions
[unresolved]: #unresolved-questions

What syntax is preferred? How does this play together with HKP? Can we improve
the converse type inference? What should be the naming conventions? Should we
segregate the value parameters and type parameters by `;`?
