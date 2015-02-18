- Feature Name: const_value_parameters
- Start Date: 2014-02-07
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Allow items to be parameterized by constant values. This RFC only introduces
parameterization over integer and boolean constants, but much of the machinery
being introduced will apply to any constant values allowed in the future.

A significant part of the design and text of this RFC was borrowed
from a draft RFC that
[@aepsil0N introduced for discussion](http://internals.rust-lang.org/t/pre-rfc-genericity-over-static-values/1538).

# Motivation

Rust already contains one type which is parameterized by an integer, namely the
array (`[T; N]`). There are several benefits to having statically sized arrays:

 - Array types are `Sized`, and therefore can readily be placed on the stack.
 - Array size incompatibilities usually cause compile-time errors, rather than
   run-time errors or silent misbehavior.
 - The compiler has the opportunity to optimize operations based on the array
   size and alignment.
 - It is easier to port and/or interoperate with C or C++ code that uses fixed
   array sizes.

However, this is the only case where Rust allows parameterization by a value.
This has already caused several issues, for instance:

 - It is impossible to generically implement a trait for arrays of any size.
 - It is impossible to define a parameterized struct to contain an array of any
   size (except by using the array type as a parameter for a struct that can
   contain *any* sized type).
 - It is impossible to generically define conversion functions between array
   types. (E.g. converting `[u8; 4]` to `[u32; 1]`, `[u8; 8]` to `[u32; 2]`, and
   so on.)

Since there is no generic way to define these items, developers typically have
to resort to macros or code generation instead. Even then, the generated code
may not cover all cases. For instance, standard traits like `Clone` and `Debug`
are only implemented for arrays up to an arbitrary size (currently `32`).

This situation also makes it more difficult to define fixed-size containers that
do not have a similar layout to arrays. This has been a problem for Servo as
described in [issue 319](https://github.com/rust-lang/rfcs/issues/319).

Aside from working with fixed-size containers, there are many more cases where
it would be helpful to use constant values as parameters. Here are a few
examples:

 - *Linear Algebra*: Algebraic vectors and matrices generally have a certain
   rank. For example, it does not make sense to add a 2-vector with a 3-vector
   or multiply a 3x4 matrix by a 5-vector. But these operations can be written
   generically using the ranks of these types.
 - *Multidimensional arrays*: Even when the *size* of a multidimensional array
   is not known at compile time, the *order* (the number of array dimensions) is
   typically known. Using a constant value parameter to represent the order
   allows a variety of operations to be defined generically for multidimensional
   arrays, without requiring extensive run-time checks (e.g. indexing, slicing,
   reductions, transposes, reshaping).
 - *Range and mod types*: This would allow range and mod types (similar to
   [Ada's](http://en.wikipedia.org/wiki/Ada_%28programming_language%29#Data_types))
   in Rust. These enforce certain range constraints and implement modular
   arithmetic, respectively. Besides providing *extra* checks for correctness,
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
parameter can also be declared using the syntax `const ident: ty`. Naturally,
`ident` is the name of the parameter, and `ty` is its type. The scope of an
item's const parameters is the same as that of type parameters.

In any position in a parameter list where a const parameter is expected, a
constant expression of appropriate type must be provided. Since the list is
positional, the compiler can readily infer the integer type, allowing
parameterized types to be used as in `Array<T, 2>` (though `Array<T, 2us>` is
also allowed).

To avoid any ambiguities that might arise from mixing type and expression
syntax, constant expressions in parameter lists must be surrounded with `{}`
braces. For convenience, these braces can be omitted if the expression is:

 1. a primitive literal, or
 2. an identifier naming a constant.

These exceptions are made because they are likely to be the most common cases,
and because they seem to be the easiest exceptions for the parser to handle.

Examples:

```rust
// Correct.
struct ArrayWrap<T, const N: usize> {
    inner: [T; N],
}
impl<T, const N: usize> ArrayWrap<T, N> {
    fn len(&self) -> usize { N }
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

## Allowed types for const parameters

For now we restrict const parameters to be of a type that implements the
`Parameter` trait. `Parameter` will be a marker trait that is implemented only
for integer primitive types and `bool`, and which cannot be explicitly
implemented. It must, of course, be handled specially by the compiler, since it
has a special role with respect to type checking.

There are two reasons to provide a marker trait, rather than having the compiler
simply treat these types specially *without* such a trait.

Firstly, it allows items such as this function to be defined.

```rust
fn use_generic_const<T: Parameter, const N: T>() { /* ... */ }
```

This is something of an edge case, but there is no clear reason to forbid such a
function. Without the `Parameter` trait, it would be impossible to specify the
constraint that `T` be one of the allowed types for a const parameter.

Secondly, and perhaps more importantly, the `Parameter` trait is likely to be
useful if and when other types become usable as const parameters. If some
`struct` and `enum` types are allowed as const parameters in the future,
`Parameter` may be automatically derived for those types by the compiler (as
with `Sized`) or by default and negative impls (as with `Send`).

## Type Inference

As implied by the initialization of `foo` in the above example, it should be
possible to infer const parameters in cases where a known constant value (in
this case the array size) must be equal to const parameter (in this case the
`ArrayWrap` const parameter).

However, when an arbitrary constant expression is used, it is not reasonable
to expect the compiler to invert the expression to perform type inference:

```rust
struct Tensor<const N: usize> { /* ... */ }
fn tensor_product<const N: usize, const M: usize>(a: Tensor<N>, b: Tensor<M>)
    -> Tensor<{M*N}> { /* ... */ }
// If x and y have to be inferred, do you really expect the compiler to
// factor a number for you?
let z: Tensor<105> = tensor_product(x, y);
```

During type inference, the compiler should not attempt to invert expressions,
but it should otherwise infer the values of const parameters wherever
possible. For example:

```rust
// The return type can be inferred from the argument type, and vice versa.
fn identity<const N: usize>(x: Foo<N>) -> Foo<N>;
// The return type can be inferred from the argument type, but not vice versa.
fn reduce1<const N: usize>(x: Foo<N>) -> Foo<{N-1}>;
// The argument type can be inferred from the return type, but not vice versa.
fn reduce2<const N: usize>(x: Foo<{N+1}>) -> Foo<N>;
// No inference of N possible; the compiler can only infer either value if N is
// specified explicitly when reduce3 is called, i.e. via UFCS.
fn reduce3<const N: usize>(x: Foo<{N+2}>) -> Foo<{N+1}>;
```

## Arithmetic semantics

Arithmetic involving const parameters "at the type level" (i.e. in parameter
lists) should always be checked for underflow and overflow.

This is consistent with the integer overflow semantics specified by
[RFC 560](https://github.com/rust-lang/rfcs/blob/master/text/0560-integer-overflow.md),
which considers integer underflow and overflow to be erroneous unless the
program explicitly uses the wrapped arithmetic operations. The compiler is
allowed (and in debug builds required) to reject programs that contain
underflow/overflow. When dealing with const parameters, these checks incur no
run-time cost, so there is no reason not to always perform them.

Additionally:

 - No one so far has claimed that wrapped arithmetic at the type level is useful
   for any specific purpose.
 - Most proposed applications of const parameters only require unsigned types.
   Unsigned underflow is likely to result in absurdly large parameter values.

Where const parameters are used like `const` values in function bodies, they
should be treated according to RFC 560, i.e. it is up to the compiler to decide
whether and when to check for underflow or overflow in non-debug builds.

# Drawbacks

## Language and implementation complexity

This change obviously represents significant additional complexity in Rust's
type system. This RFC assumes that the resulting improvements in language
expressiveness will outweigh this cost in the long run.

# Alternatives

## Do nothing

Programmers can continue to get by with macros, run-time sizes, and generated
code, as they do today. This is unpalatable to many developers, but it is not
unworkable.

## Use type-level natural numbers, defined separately from value-level integers

This approach has seen significant success in libraries in other languages, such
as Haskell. The main problem with this approach in Rust is that it is at odds
with the way we handle arrays currently. However, this approach is usable today
for many applications. See also
[Darin Morrison's `shoggoth` crate](https://github.com/epsilonz/shoggoth.rs/tree/15d1b7b94057bb75d887c355d42afe5e1da6f503).

Type-level arithmetic can also be complex to implement and verbose to use. The
"macros in types" proposal
([PR #873](https://github.com/rust-lang/rfcs/pull/873)) suggests that macros can
alleviate this problem somewhat.

This proposal does not directly conflict with the macros proposal, since the two
are compatible, and since each provides functionality that the other does not.
However, it may be useful to provide a comparison.

For type-level naturals with macros:

 - In theory, only minimal language support is necessary; the prototype
   implementation is already complete.
 - However, some degree of macro *and plugin* support is required to provide
   efficient support for natural numbers with clean syntax, which does imply
   some additional burden.
    - If a library provides items parameterized by naturals, its users will find
      the library difficult or impossible to use without the same plugin(s) that
      that library used.
    - If the standard library ever did so, it would likely be simplest to simply
      integrate this capability into the compiler.
 - The proposed macros provide capability well beyond the provision of
   type-level naturals. A rather vast array of types can be defined and used
   with simple syntax (though this requires implementation work, rather than
   being automatic, as a data kinds feature would be).
 - Macros cannot easily introduce a new type or its value into scope, meaning
   that using a type-level natural `N` as a value requires a macro (e.g.
   `val!(N)`). Working with multiple incompatible type-level systems would
   require some care, if only to avoid name clashes.
 - Not all constant expressions can be used at the type level without an
   arbitrary number of plugin passes, since the plugin is needed to determine
   which types are present, which in turn can add more `const` values, which
   could in turn could affect more types...
 - Type-level natural numbers (or integers) are not themselves given a Rust
   type. They can in principle correspond to arbitrarily large numbers (like
   bignums).
 - As a consequence of several of the above points, the capabilities of the
   macro system will not be equivalent to what the compiler does for `[T; N]`
   arrays.
 - Type error messages are likely to be confusing, since they will tend to
   expose the "internal" representation of a type as, e.g., a bit pattern. This
   might be mitigated by sufficient functionality in a plugin, or by attributes
   allowing error message customization, similar to the existing
   `rustc_on_unimplemented` attribute.

For const parameters in this proposal:

 - Significant new language complexity must be added, including new syntax, new
   aspects of the type system (e.g. affecting inference), and a new trait to be
   handled specially by the compiler.
 - Only integers and `bool` are affected. Further RFCs introducing CTFE and data
   kinds would be required before this capability would cover many of the use
   cases handled by macros. (In effect, this would move toward a "true"
   dependent type system.)
 - Integers are limited in range, not bignums.
 - Const parameters can be used as values directly, with no `val!` macro or
   other special syntax.
 - All constant expressions allowed for array sizes should be usable as const
   parameters as well. In part this is because the two can be handled by the
   same or similar code in the compiler as generic items are instantiated.
 - Type signatures in error messages should ideally resemble code that the user
   actually wrote.
 - Specialization of impls based on the value of a const parameter seems more
   tractable than doing something equivalent with type-level naturals. However,
   the proposed language additions that would actually allow this are deferred
   to a follow-up RFC.

## Const parameters without arithmetic

This RFC could be partially implemented by only allowing literals or identifiers
corresponding to const values to be used (i.e. types like `Foo<N>` and `Foo<2>`
would be allowed, but not `Foo<{N-1}>` or `Foo<{2+2}>`). Then the above points
about inference, arithmetic semantics, and the use of braces in parameter lists
would not apply.

This would be less complex initially, but care would need to be taken to allow
the syntax to backwards-compatibly grow in the future. In such a partial
implementation, array sizes would be more flexible than other const parameters
(since `[T; 2+2]` is already valid syntax in Rust today).

## Variations on this design

### Allow non-integer types to be const parameters

It is entirely possible that we will want to implement a full-fledged dependent
type system for Rust. This RFC is not intended as an alternative to dependent
types, but as an interim measure that makes progress towards such a system,
while also providing solutions to more immediate problems.

Here's a brief summary of other types of values that could be used as
parameters, and the reasons that they are omitted from this RFC:

 - Adding `char` would be simple. This is omitted partly due to having less
   obvious utility, and partly because the author did not think of it until
   this proposal was nearly complete.
 - Allowing tuple parameters seems harmless but unnecessary, since one can
   simply use multiple parameters instead of a single tuple parameter. However,
   it may be a good idea to allow them for the sake of consistency, or to
   group related parameters in a list.
 - Allowing arrays or any unsized type could cause problems for both compiler
   performance and symbol name mangling. However, arrays that are not too large
   should be OK, since they can be handled similarly to tuples.
 - Floating point types can have `NaN` values, and arithmetic is inexact. It
   seems like a very bad idea to allow details of the floating point
   representation to influence whether or not a program type checks. This is
   especially concerning for cross-compilation, since floating-point arithmetic
   may yield different results on the platform that compiles the code from the
   one that runs the code.
 - References to static values (`&'static`) could be used as const parameters,
   as in C++. This was left out because it adds complexity while being less
   likely to be useful, but it may not cause any serious problems.
 - It seems necessary to omit struct or enum values pending further design
   work. E.g. there may be other features that Rust's type system needs to
   implement these, and either CTFE or specialized plugins are probably required
   for these to be useful.

### Only allow (or default to) a particular integer type

If there was only a single type of integer that could be used as a const
parameter, it would not be necessary to specify the type of a const parameter,
and it might be somewhat easier to implement this proposal.

The current design was chosen to be more flexible and more consistent with const
values, and to be more compatible with future dependent types.

If the type of a const parameter defaulted to a specific type (e.g. `usize`),
while still allowing other types, this would retain the current proposal's
flexibility. However, the ergonomic benefit might not be worth the puzzling
inconsistency with other values (i.e. `let x;` does not default to `usize`).

### Alternative parameter syntax

The proposal above allows type and const parameters to be intermixed, but this
has some ergonomic cost, requiring the `const` keyword to be written out
frequently. It is also possible that this mixing will lead to more disorganized
parameter lists, since the const and type parameters can be jumbled in any
order.

One way of separating const and type parameters by position, without interfering
with defaulted (or variadic) parameters, would be to use `;` to separate the
two. Then the above `Gnarl` examples could be written this way:

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

# Unresolved questions

## Where clauses

A followup RFC is planned to introduce an extension to `where` clauses, in order
to constrain const parameters with new bounds. This is deferred until the syntax
and interaction with coherence checks can be rigorously specified.

## Better inference

In some circumstances, it may be possible to invert arithmetic expressions, and
therefore infer values in other circumstances than specified in this RFC. This
RFC does not take a stance on whether or not this should be done.

However, it is undesirable for a program to depend on unspecified details of the
inference algorithm used by a particular compiler version. Therefore it is
recommended that any improvements to const parameter inference be described in a
future RFC, or at a minimum be well documented in the Rust reference.

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
   parameters, but there may be hidden pitfalls. Syntactically, it will
   presumably be necessary to make a distinction between lifetime, type, and
   const parameters when specifying the expected kind of a type.
 - It's not clear whether const parameters could be added to variadic parameter
   lists, or if it will be necessary to limit variadic behavior to types.
