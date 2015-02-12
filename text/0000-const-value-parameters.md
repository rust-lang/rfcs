- Feature Name: const_value_parameters
- Start Date: 2014-02-07
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Allow items to be parameterized by constant values. This RFC only introduces
parameterization over integer and boolean constants, but much of the machinery
being introduced apply to any other types of constant values allowed in the
future.

A significant part of the design and text of this RFC was borrowed from a draft
RFC that @aepsil0N introduced for discussion.

# Motivation

Rust already contains one type which is parameterized by an integer, namely the
array (`[T; N]`). There are several benefits to having statically sized arrays:

 - Arrays are sized types, and therefore can readily be placed on the stack.
 - Array size incompatibilities usually cause compile-time type errors, rather
   than run-time errors or silent misbehavior.
 - The compiler has the opportunity to optimize operations based on
   the array size and alignment.
 - It is easier to port and/or interoperate with C or C++ code that uses fixed
   array sizes.

However, this is the only case where Rust allows parameterization by a value.
This has already caused several issues, for instance:

 - It is impossible to generically implement a trait for arrays of any size.
 - It is impossible to define a parameterized struct to contain an array of any
   size.
 - It is impossible to generically define conversion functions between array
   types. (E.g. converting `[u8; 4]` to `[u32; 1]`, `[u8; 8]` to `[u32; 2]`, and
   so on.)

Since there is no generic way to define these items, developers typically have
to resort to macros or code generation instead. Even then, the generated code
may not cover all cases. For instance, standard traits like `Clone` and `Debug`
are currently only implemented for arrays up to an arbitrary size (currently
`32`).

This situation also makes it more difficult to define fixed-size containers that
do not have a similar layout to arrays. This has been a problem for Servo as
described in [issue 319](https://github.com/rust-lang/rfcs/issues/319).

There are several more cases where it would be helpful to use constant values as
parameters. Here is a list:

 - *Algebraic types*: Algebraic vectors and matrices generally have a certain
   rank. For example, it does not make sense to add a 2-vector with a 3-vector
   or multiply a 3x4 matrix by a 5-vector. But the algorithms can be written
   generically in terms of the ranks of these types.
 - *Multidimensional arrays*: Even when the *size* of a multidimensional array
   is not known at compile time, the *order* (the index dimensionality) is
   typically known. Using a constant value parameter to represent the order
   allows a variety of operations to be defined generically for multidimensional
   arrays without requiring extensive run-time checks (e.g. indexing, slicing,
   reductions, transposes, reshaping).
 - *Range and mod types*: This would allow range and mod types (similar to
   Ada's)[http://en.wikipedia.org/wiki/Ada_%28programming_language%29#Data_types]
   in Rust. These enforce certain range constraints and implement modular
   arithmetics, respectively. Besides providing *extra* checks for correctness,
   such types can also *avoid* unnecessary bounds checks, when an index can be
   proven to be in a valid range at compile time.
 - *Physical units*: In science, one often deals with numerical quantities
   equipped with units (meters, hours, kilometers per hour, etc.). To avoid
   errors dealing with these units, it makes sense to include them in the data
   type. In this context value parameters could allow conversion between units
   and/or checking formulae for consistency at compile-time. (For numerically
   intensive code, run-time checks are usually prohibitively expensive.)

Integer const parameters would enable three of the above applications, and
could make it easier to deal with physical units using the type system.

# Detailed design

## Syntax

In general, wherever a type parameter can be declared as `ident`, a const
parameter can also be declared using the syntax `const ident: ty`, where the
first identifier is the name of the parameter, and the second is its type. The
scope of an item's const parameters is the same as that of type parameters.

In any position in a parameter list where a const parameter is expected, a
constant expression of appropriate type must be provided. Since the list is
positional, the compiler can readily infer the integer type, allowing
parameterized types to be used as in `Array<T, 2>` (though `Array<T, 2us>` is
also allowed).

To avoid any ambiguities that might arise from mixing type and expression
syntax, constant expressions in parameter lists must be surrounded with `{}`
braces. For convenience, these braces can be omitted if the expression is:

1. a literal or
2. an identifier naming a constant.

These exceptions are made because they are likely to be the most common cases,
and because they seem to avoid placing any severe burden on the parser.

Examples:

```rust
// Correct.
struct ArrayWrap<T, const N: usize> {
    inner: [T; N],
}
impl<T, const N: usize> ArrayWrap<T, N> {
    fn len(&self) -> usize {
        N
    }
}
fn bar() {
    let foo = ArrayWrap{ inner: [2i32, 3] };
    assert_equal!(foo.len(), 2);
    assert_equal!(ArrayWrap::<i32, 2>::len(&foo), 2);
}
// Also correct.
trait Gnarl<T, const N: i64, U=u32, const Q: bool = true> { /* ... */ }
impl Gnarl<i32, -2> for Carl { /* ... */ }
impl Gnarl<i32, {4+4}, u64> for Darl { /* ... */ }
const x: i64 = 9;
impl Gnarl<i32, x, _, false> for Jarl { /* ... */ }
// Not correct.
impl Gnarl<i32, x>>2> for Snarl { /* ... */ }
// Correction of the above.
impl Gnarl<i32, {x>>2}> for Snarl { /* ... */ }
```

The above examples show how this feature interacts in an intuitive way with UFCS
syntax and parameter defaults.

If const and type parameters were separated by position (e.g. if all const
parameters had to follow all type parameters), it would become more difficult to
make use of defaulted type parameters. This also might interfere with variadic
parameters in the future. Therefore it seems better to allow type and const
parameters to be mixed, and use the `const` keyword to distinguish between them.

## Type Inference

As implied by the definition of `foo` in the above example, it should be
possible to infer const parameters in cases where a known value (in this case
the array size) must be equal to another parameter (in this case the `ArrayWrap`
const parameter).

However, when an arbitrary constant expression is used, it is not be reasonable
to expect the compiler to invert the expression to perform type inference:

```rust
struct Tensor<const N: usize> { /* ... */ }
fn tensor_product<const N: usize, const M: usize>(a: Tensor<N>, b: Tensor<M>)
    -> Tensor<M*N> { /* ... */ }
// If x and y have to be inferred, do you really expect the compiler to
// factor a number for you?
let z: Tensor<105> = tensor_product(x, y);
```

For the purposes of type inference, the compiler should not attempt to invert
expressions, but it should otherwise infer the values of const parameters
wherever possible. For example:

```rust
// The return type can be inferred from the argument type, and vice versa.
fn identity<const N: usize>(x: Foo<N>) -> Foo<N>;
// The return type can be inferred from the argument type, but not vice versa.
fn reduce1<const N: usize>(x: Foo<N>) -> Foo<N-1>;
// The argument type can be inferred from the return type, but not vice versa.
fn reduce2<const N: usize>(x: Foo<N+1>) -> Foo<N>;
// No inference of N possible; the compiler can only infer either value if N is
// specified explicitly when reduce3 is called.
fn reduce3<const N: usize>(x: Foo<N+2>) -> Foo<N+1>;
```

## Arithmetic semantics

Arithmetic involving const parameters "at the type level" (i.e. in parameter
lists) should always be checked for underflow and overflow. There are several
reasons for this:

 - No one so far has claimed that wrapped arithmetic at the type level is useful
   for any given application.
 - There is no run-time cost for these checks.
 - Most proposed applications of const parameters only require unsigned types.
   Unsigned underflow is likely to result in absurdly large parameter values.

Where const parameters are used like `const` values in function bodies, they
should be treated like other integers, i.e. it is up to the compiler to decide
whether and when to check for underflow or overflow.

## Where clauses

There may be cases where only certain values of a const parameter are
reasonable, and therefore one needs a way to apply bounds to an item's const
parameters. There are two new types of constraints added by this proposal:

 1. Constraints of the form `const ident: constexpr`. The right hand side must
    not contain a reference to any const parameter. The constraint is satisfied
    if the identified const parameter is equal to the right hand side.
 2. Constraints of the form `const ident: RANGE`. In this case, `N` must be of
    an integer type, and the `RANGE` is a range expression containing constant
    integer expressions (i.e. `..`, `M..`, `..N`, and `M..N`).

Some (very contrived) examples:

```rust
/// Returns the middle element.
fn center<T, const N: usize>(a: &[T, .. n]) -> &T
    where const N: 3
{ /* ... */ }
// Only can be instantiated with M < 5
struct Razm<U, const M: u8, V>
    where U: Frazm,
          const M: ..5,
          V: Crazm {
    /* ... */
}
```

However, the main motivation for this case is to allow specialization of impls
based on the value of a const parameter, as shown in the following example:

```rust
trait<T, const N: usize> ReduceArray<T, N>
      where const N: 1.. {
    type ReducedByOne;
    fn sum_dimension(self, dim: usize) -> ReducedByOne;
    /* Other functions */
}
impl ReduceArray<i32, 1> for I32Array<1> {
    type ReducedByOne = i32;
    /* Function implementations */
}
impl<const N: usize> ReduceArray<i32, N> for I32Array<N>
      where const N: 2.. {
    type ReducedByOne = I32Array<{N-1}>;
    /* Function implementations */
}
```

This use case involves a reduction of a multidimensional array, but this pattern
could apply to other cases where it is useful to connect some type with another
that's before/after it in a sequence. E.g. you can use this pattern to define a
trait method that adds/removes items of fixed-size arrays, in which case you
need to specify the type of an array that's larger/smaller than the one you were
given.

# Drawbacks

## Language and implementation complexity

This change obviously represents significant additional complexity in Rust's
type system. This RFC assumes that the resulting improvements in language
expressiveness will outweigh this cost in the long run.

# Alternatives

## Do nothing

Users can continue to get by with macros, run-time sizes, and generated code, as
most do today. This is unpalatable to many developers, but it is not unworkable.

## Use type-level natural numbers defined separately from value-level integers

This approach has seen significant success in other languages, such as
Haskell. The main problem with this approach in Rust is that it is at odds with
the way we handle arrays currently. Type-level arithmetic can also be complex to
implement and verbose to use. However, this approach is usable today.

See also
[@darinmorrison's `shoggoth` crate](https://github.com/epsilonz/shoggoth.rs).

## Const parameters without arithmetic

This RFC could be partially implemented without allowing arithmetic in constant
parameters (i.e. types like `Foo<N>` and `Foo<2>` could be used, but not
`Foo<{N-1}>` or `Foo<{2+2}>`). Then the above points about inference, arithmetic
semantics, use of braces in parameter lists, and `where` clauses would not
apply.

This would be less complex initially, but care would need to be taken to allow
the syntax to backwards-compatibly grow in the future. In such a partial
implementation, array sizes would still be more flexible than struct/enum
constant parameters (since `[T; 2+2]` is already a valid type in Rust today).

## Variations on this design

### Allow non-integer types to be const parameters

Here's a brief summary of other types of values that could be used as
parameters, and the reasons that they are omitted from this RFC:

 - It seems necessary to omit struct or enum values pending further design
   work (e.g. there may be other features that Rust's type system needs to
   implement these, and either CTFE or specialized plugins are probably required
   for these to be useful).
 - Allowing arrays or any unsized type could cause problems for both compiler
   performance and symbol name mangling.
 - Floating point values can have `NaN` values, and arithmetic is inexact. It
   seems like a very bad idea to allow details of the floating point
   representation to influence whether or not a program type checks.
 - References to static values (`&'static`) could be used as const parameters,
   as in C++. This does not appear to cause any major issues, and was left out
   only because it adds complexity while being less likely to be useful.

### Only allow (or default to) a particular integer type

If there was only a single type of integer that could be used as a constant
parameter, it would not be necessary to specify the type of a const parameter.

The current design was chosen to be more flexible and consistent with const
values, and to be more reasonable in the event that other types ever became
usable as const parameters.

If the type of a const parameter defaulted to a specific type (e.g. `usize`),
this would retain the current flexibility, though to an extent it would still
privilege integer const parameters over other types added in the future.

### Alternative parameter syntax

The proposal above allows type and const parameters to be intermixed, but this
has some ergonomic cost, requiring the `const` keyword to be written out
frequently. It is also possible that this mixing will lead to more disorganized
parameter lists, since the constant and type parameters can be jumbled in any
order.

One way of separating constant and type parameters by position, without
interfering with defaulted (or variadic) parameters, would be to use `;` to
separate the two. Then the above `Gnarl` examples could be written this way:

```rust
trait Gnarl<T, U=u32; N: i64, Q: bool = true> { /* ... */ }
impl Gnarl<i32; -2> for Carl { /* ... */ }
impl Gnarl<i32, u64; {4+4}> for Darl { /* ... */ }
const x: i64 = 9;
impl Gnarl<i32; x, false> for Jarl { /* ... */ }
impl Gnarl<i32; {x>>2}> for Snarl { /* ... */ }
```

One benefit of using the symbol `;` is that it is analogous to the existing
`[T; N]` array syntax, where the `;` already separates a type from a const
parameter.

Using `;` as a separator looks somewhat strange if only const parameters
are present, though it is not *too* irregular:

```rust
fn contrived_function<; N: u64>(x: Foo<; N>) {}
contrived_function::<; 3>(Foo::new::<; 3>());
```

A third alternative is to separate parameters positionally with commas, but to
require the keyword `const` before the first const parameter, as seen here:

```rust
trait Gnarl<T, U=u32, const N: i64, Q: bool = true> { /* ... */ }
impl Gnarl<i32, const -2> for Carl { /* ... */ }
impl Gnarl<i32, u64, const {4+4}> for Darl { /* ... */ }
const x: i64 = 9;
impl Gnarl<i32, const x, false> for Jarl { /* ... */ }
impl Gnarl<i32, const {x>>2}> for Snarl { /* ... */ }
fn contrived_function<const N: u64>(x: Foo<const N>) {}
contrived_function::<const 3>(Foo::new::<const 3>());
```

Unfortunately, this is by far the *least* ergonomic option, since now the
`const` keyword is now also required for *uses* of generic items, which are more
common than *definitions* of generic items.

### Broader `where` clauses

It has been suggested that `where` clauses for const parameter constraints
should be expanded to contain any constant boolean expression. This feature was
omitted because it adds significant complexity to the proposal, and because it
raises questions about whether and when such constraints can reasonably inform
coherence checking.

# Unresolved questions

## Better inference

In some circumstances, it may be possible to invert arithmetic expressions, and
therefore infer values in other circumstances than specified in this RFC. This
RFC does not take a stance on whether or not this should be done.

However, it is undesirable for a program to depend on unspecified details of the
inference algorithm used by a particular compiler version. Therefore it is
recommended that any improvements to const parameter inference should be
described in a future RFC, or at a minimum be well documented in the Rust
reference.

## What is a constant expression?

This RFC implicitly assumes that constant expressions, at a minimum, encompass
the operations allowed in defining a `const` value or an array size right now.
This definition may expand for various reasons, e.g. if `sizeof` can be done at
compile time, or if CTFE is implemented more generally.

## How does this interact with future language features?

It is difficult to design for compatibility with language features that are not
themselves complete.

 - It seems likely that, when higher-kinded types are implemented, it will be
   straightforward to treat const parameters similarly to type and lifetime
   parameters, but there may be hidden pitfalls.
 - It's not clear whether const parameters could be added to variadic parameter
   lists, or if it will be necessary to limit variadic behavior to types.
