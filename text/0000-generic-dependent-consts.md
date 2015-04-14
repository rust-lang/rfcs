- Feature Name: generic_dependent_consts
- Start Date: 2015-03-28
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Allow consts declared in functions to depend on function type parameters. Since
this raises some issues in match and type checks, use an extremely conservative
approach that allows only minimal use of such consts when a constant expression
is required.

# Motivation

Consider the following line of code in a non-generic context:

```rust
let a: [u8; T::N + T::N] = [0u8; 2*T::N];
```

In the current associated consts implementation (modulo some temporary wrinkles
involving constants associated to traits), this will readily type-check. Since
the type `T` must be known in non-generic code, `T::N` will evaluate to some
value, say `16`, in which case it is clear that both sides of the assignment
involve arrays of size `32`.

However, in generic code, if `T` is a type parameter, we are asking the type
checker to apply the general proposition that `N + N = 2 * N` for all `usize`
values (that do not overflow when doubled). While this case is simple, requiring
the compiler to prove arbitrary algebraic identities is untenable.

A similar story applies to match patterns. In order to perform exhaustiveness
and reachability checks, the compiler must be able to perform certain
comparisons on match arms. However, this analysis is defeated if the compiler
does not know what the value in a match pattern actually is.

To date we have dodged the issue by simply not allowing constant expressions to
depend upon type parameters at all.

However, in the process we have foreclosed a fairly wide range of possibilities.
Here are several examples of functions that are forbidden, even with basic
support for associated consts:

```rust
fn do_something_optional<T>(x: Option<T>) {
    const NOTHING: Option<T> = None;
    match x {
        Some(y) => { /* ... */ }
        NOTHING => { /* ... */ }
    }
}
unsafe fn do_something_with_bitmask<T>(x: T) {
    // Assume for the sake of argument that Sized::SIZE is an associated const
    // of type `usize`.
    const SIZE_IN_BITS: usize = 8 * <T as Sized>::SIZE;
    /* ... */
}
fn circle_area<T: Float>(radius: T) {
    // This redefinition is not very useful, but has a straightforward meaning.
    const PI: T = T::PI;
    PI*radius*radius
}
fn circumference<T: Float>(radius: T) {
    const TWO_PI: T = 2.0 * T::PI;
    TWO_PI*radius
}
```

The first three examples would be allowed under this RFC. The fourth function,
`circumference`, would still be disallowed because the compiler cannot verify
during type checking that `2.0 * T::PI` is a constant expression of type
`T`. However, this RFC removes /one/ of the barriers that prevents
implementation of this last example.

Note that this issue becomes even more pressing if user-defined types are
allowed to depend on constants in the future, since in such a case these issues
are not limited to the interaction between associated consts and a handful of
language features.

# Detailed design

Because they are inlined wherever possible, non-associated constants are similar
to macros in many circumstances. Consider this definition:

```rust
const C: T = EXPR;
```

Most uses of `C` could be replaced with `EXPR: T` (using type ascription) with
no change in the meaning of the code. The main difference is that when a named
constant `C` is defined, `EXPR` is required to be a constant expression, and can
be used in certain situations (especially patterns) where inlining the
expression, either manually or via macro, would produce invalid syntax. (Of
course, named constants also improve readability and allow for clearer error
messages in some cases.)

Keeping this in mind, it is straightforward to understand most uses of constant
expressions. The design below will mostly focus on special cases and
interactions with other language features.

## Other items in functions

The scope of this RFC is limited to consts. Nested functions will still be
forbidden from referencing "outer" type parameters. The same is true for
statics. There are two justifications for this.

Firstly, nested static items, whether they are static variables or functions,
must have static locations in memory. If these items could use the type
parameters from the surrounding scope, they would have to be newly instantiated
for each instantiation of the enclosing function.

Secondly, functions that make use of outer type parameters are in effect higher-
kinded types. These seem to require more careful consideration to implement.

Neither of these considerations apply to constants, which the compiler is always
allowed (and often required) to inline. They do not need a static location in
memory, and unlike functions, they cannot add new type or lifetime parameters to
those already present in the surrounding scope.

## 'static references

Consider the following definitions of similar functions:

```rust
// Currently not valid (borrowed value does not live long enough).
fn ref_one_literal() -> &'static u32 {
    &1
}
// Currently valid (a static is implicitly created).
fn ref_one_const() -> &'static u32 {
    const REF_ONE: &'static u32 = &1;
    REF_ONE
}
// Not valid (borrowed value does not live long enough).
fn ref_one_ref_const() -> &'static u32 {
    const ONE: u32 = 1;
    &ONE
}
// Should either of the following become valid?
fn ref_one_generic_const<T: Int>() -> &'static T {
    const REF_ONE: &'static u32 = &T::ONE;
    REF_ONE
}
fn ref_one_associated_const<T: Int>() -> &'static T {
    &T::ONE
}
```

This RFC proposes that both `ref_one_generic_const` and
`ref_one_associated_const` would be invalid.

The associated const case is disallowed for the same reason as
`ref_one_ref_const`, i.e. because the expression is not `'static` and thus does
not live long enough for a static borrow.

The generic const case is disallowed because it implicitly creates a static item
that depends on a type parameter, and this is forbidden, as mentioned in the
preceding section. That is to say, the initializer expression itself is invalid.

In order to implement this restriction, use of a type parameter in a constant
expression must be considered "contagious". That is, if the initializer
expression for a const uses a type parameter, then all expressions that
reference that const implicitly depend on the same type parameter.

## Match patterns

For the moment, constant values used in match patterns are subject to the same
restrictions as statics, i.e. they cannot depend on type parameters. Without
this restriction, it would not be possible to guarantee that match expressions
satisfy the exhaustiveness and reachability criteria for all possible
instantiations of a generic function. This restriction may be loosened
backwards-compatibly in the future by adding syntax to constrain associated
consts in generic code (similar to how type parameters, and their associated
types, can already be constrained with `where` clauses).

However, note that when the /type/ of a constant depends on a generic parameter,
whereas its /value/ does not, the constant is still allowed in a match pattern.
This can occur, for instance, when the value is a nullary enum (see the example
in the `Motivation` section).

## Array sizes

In order for type checking to determine whether or not two array types are
equal, it must be able to compare the arrays' sizes. This presents a special
problem when performing arithmetic on constants that depend on type parameters,
as outlined in the `Motivation` section above.

To avoid dealing with arbitrary arithmetic expressions, all constant expressions
that affect array sizes are divided into the following three categories:

 1. Constant expressions that do not depend on type parameters at all. These
    will continue to behave as they do now; they are evaluated during type
    checking, and will be considered equal to all other expressions that can be
    evaluated during type checking to the same value.

 2. Constant expressions that consist of only a single path (or identifier),
    where that path is a constant parameter that depends on at least one type
    parameter. During type checking, such expressions will compare equal to any
    expression that consists of only a single path that resolves to the same
    item. This reduces an impossible problem (determining whether two arbitrary
    expressions are equivalent) to a simple one (determining whether two paths
    resolve to the same item).

 3. Constant expressions of any other form that depend on type parameters. These
    expressions will never be considered equivalent to any other expression.

To further explain case 2, the following will be allowed:

```rust
let a: [u8; <T>::N] = [0u8; <T>::N];
const X: usize = 2*<U as Trait>::M;
let a: [u8; X] = [0u8; X];
// This is not allowed:
// let a: [u8; X] = [0u8; 2*<U as Trait>::M];
```

Case 3 rules out many uses of arrays with sizes that depend on type parameters.
However, there are some operations where the array size is irrelevant to whether
or not the code type-checks, such as coercing a reference to an array to a
slice, or creating a raw pointer to a fixed-size buffer that can be handed to
external code via FFI. In such cases, using an array expression such as
`[0u8; 2*<U as Trait>::M]` could still be useful.

The justification for the above rules is that it seems premature to settle on a
specific strategy for dealing with constant expressions in types right now.
However, the rule in case 2, which states that two paths that resolve to the
same item will compare equal in type checking, seems to describe a bare minimum
of functionality that will almost certainly be included by any further long-term
solution.

# Drawbacks

These issues will probably be tackled eventually, since generic code is where
much of the utility of associated consts, and perhaps in the future "real"
dependent types, will be found. However, we could postpone any decisions until
further extensions of the type system force the issue. This proposal does
introduce some complexity in that generic constants must be treated for many
situations like generic types.

Since this design is in some cases fairly conservative regarding code that will
be accepted, it may also produce some confusion when code that seems "obviously"
OK is rejected by the compiler. For instance, this is rejected:

```rust
// `T` is a generic parameter.
const X: usize = <T>::N;
// We don't backtrack to see that the RHS is using the expression that defines
// X, so this line is invalid.
let a: [u8; X] = [0u8; <T>::N];
```

# Alternatives

We could keep the status quo, where type parameters cannot influence the values
in constant expressions at all. This would somewhat reduce the utility of
associated consts, and prevent us from giving this solution a "trial run", but
the language would be simpler for now.

We could implement this RFC and additionally allow the special example in the
`Drawbacks` section as valid code. This seems unnecessary if a `const`
declaration is viewed as purely creating its own item, but it seems that this
code should be accepted if a `const` declaration is viewed as instead creating
something more like an alias or macro simply expanding to some inlinable
constant expression.

# Unresolved questions

This design omits some possible extensions, such as allowing other forms of
expressions to be considered equal during type checking.

Allowing constraints on associated constant values, CTFE, and most other
constant-related features are likewise ignored. Any interaction with CTFE may be
obvious, by using the heuristic that constant functions should behave as if
their code was simply inlined everywhere that they are used.
