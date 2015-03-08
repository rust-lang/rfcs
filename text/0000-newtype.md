- Feature Name: newtype
- Start Date: Fri Mar  6 17:56:48 CET 2015
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Add a `newtype` keyword that creates a non-coercible, castable alias.

# Motivation

Rust currently has two distinct mechanisms for creating type aliases. We
describe them below and then explain why there are situations where neither of
them is suitable.

## `type`

The `type` keyword creates an indistinguishable type alias. For example:

```rust
type c_char = i8;
type c_long = i64;
```

These aliases can then be used in situations where the aliased name properly
documents the behavior or where types depend on the compilation environment. For
example:

```rust
fn std::ffi::CStr::from_ptr(ptr: *const c_char) -> &'a CStr
```

Taking a `c_char` instead of an `i8` makes the intention of the API clear.  Also
note that `c_char` is `u8` on some platforms.

Aliases generated via `type` can also take lifetime and type parameters:

```rust
type IoResult<T> = std::result::Result<T, IoError>;
```

This is useful to elide repetitive type and lifetime names. If `type A = B` then
`A` and `B` are indistinguishable in the type system.

## Newtype structs

So called "newtype structs" are a special case of the tuple struct syntax:

```rust
struct New(T);
```

The type `New` is a type whose purpose is to wrap a single value of type `T`.
Unlike a type alias generated with the `type` keyword, newtype structs generate
completely new types that are quite unrelated to the wrapped type. That is, none
of the following constructs work:

```rust
struct New(u32);

fn f(v: New) { }
f(1);

fn g(v: u32) { }
g(New(1));

let x: New = 1;
let y: u32 = New(1);

let a = 1 as New;
let b = New(1) as u32;
```

The reason for this is that, as completely new types, the semantic of the
newtype structs can freely deviate from the semantics of the wrapped type:

```rust
struct New(u32);

impl Drop for New {
    fn drop(&mut self) { }
}
```

Other examples include:

- The wrapped type is `Send` but the newtype struct is not `Send`.
- The wrapped type is not `Send` but the newtype struct is `Send`.

## The problem

Above we've already seen an example where neither of these alias types are
appropriate:

```rust
#[cfg(unix)]
type c_long = i64;
#[cfg(windows)]
type c_long = i32;
```

On 64 bit systems, these are the definitions of `c_long` in liblibc. Consider
the following fictional C function which we are trying to wrap:

```c
int Gen(long input);
```

In Rust the signature looks like this:

```rust
extern {
    fn Gen(input: c_long) -> c_int;
}
```

We create a rustic wrapper:

```rust
fn gen(input: i64) -> i32 {
    unsafe { Gen(input) }
}
```

This will work fine on our Linux development box but on windows the compilation
will fail because `c_long` is `i32`.

Correct, platform independent code will have to add explicit casts:

```rust
fn gen(input: i64) -> i32 {
    unsafe { Gen(input as c_long) as i32 }
}
```

There are currently some proposals regarding `as` that would worsen this
situation:

1. Make useless casts an error or warning: `input as c_long` would cause an
   error or warning because `input` and `c_long` are both `i64`.
2. Add implicit widening: This would add more cases where `as` causes a warning
   or error.

As an alternative, one might think about using newtype structs for `c_long`:

```rust
#[cfg(unix)]
struct c_long(i64);
#[cfg(windows)]
struct c_long(i32);

impl FromPrimitive for c_long { ... }
```

This solution will enforce explicit conversions, however, they can't be used
with `as` and create a lot of friction in the code:

```rust
fn gen(input: i64) -> i32 {
    unsafe { Gen(FromPrimitive::from_i64(input).unwrap()).0 as i32 }
}
```

(Note that the error checking performed by `FromPrimitive` above is not always
desired.)

Below we propose an alternative that sits between `type` and newtype structs.

# Detailed design

Add a `newtype` keyword.

## Syntax

```rust
newtype T = U;
```

where `T` and `U` are as in the definition of `type`.

(You might want to first skip the details and continue with the "Rationale"
section below.)

## Semantics

`T` behaves as if it had been declared by `struct T(());` except as described
below.

### Representation

`T` has the same representation in memory as `U` if any.

### Casting

#### Scalar casts

Let `Types` be the set of types.

Let `R0` be the set of tuples `(A, B) \in Types^2` such that `type A = B`,
`newtype A = B`, or `A` and `B` are numeric types.

Let `R1` be the smallest equivalence relation in `Types^2` which contains `R0`.

If `(A, B) \in R1` then `A as B` is a valid cast.

#### Tuple casts

Let `R2` be the set of tuples `(A, B) \in Types^2` such that `type A = B` or
`newtype A = B`.

Let `R3` be the smallest equivalence relation in `Types^2` which contains `R2`
and which is closed under the following operation: If `N > 0` is a natural
number, `(A1, ..., AN), (B1, ..., BN) \in Types^N`, and for all
`n \in {1, ..., N}` `(AN, BN) \in R3`, then
`((A1, ..., AN), (B1, ..., BN)) \in R3`.

If `(A, B) \in R3` then `A as B` is a valid cast.

#### Derived types

For every trait `T` we define the equivalence relation `equivalent wrt T` which
is a subset of `R3`. For every type `A: T`, `A` and `A` are `equivalent wrt T`.

Let `X` be a type with free type parameters `(P1, ..., PN)` and let
`((A1, ..., AN), (B1, ..., BN)) \in R3`.

If for all `n \in {1, ..., N}` and for all trait bounds `T` on `Pn` `An` and
`Bn` are `equivalent wrt T`, then `X<A1, ..., AN> as X<B1, ..., BN>` is a valid
cast.

### Inference

If `U` is an integer type and `(T, U) \in R1`, then untyped integer literals
will be inferred as `T` in `T` position.

If `U` is a floating point type and `(T, U) \in R1`, then untyped floating point
literals will be inferred as `T` in `T` position.

### Traits

We say a trait is "well-known" if the compiler is aware of its structure and
function.

No well-known traits are or can be implemented for a newtype `T` except as
described below.

For all types `W`, `PartialOrd<W>` can be implemented for `T`. If
`PartialOrd<T>` is not explicitly implemented for `T` and `PartialOrd<U>` is
implemented for `U`, then `PartialOrd<T>` shall automatically be implemented for
`T` and `T` and `U` are `equivalent wrt PartialOrd<Self>`.

For all types `W`, `Add<W>` can be implemented for `T`. If `Add<T, Output=T>` is
not explicitly implemented for `T` and `Add<U, Output=U>` is implemented for
`U`, then `Add<T, Output=T>` shall automatically be implemented for `T` and `T`
and `U` are `equivalent wrt Add<Self>`.

`Neg` can be implemented for `T`. If `Neg<Output=T>` is not explicitly
implemented for `T` and `Neg<Output=U>` is implemented for `U`, then
`Neg<Output=T>` shall automatically be implemented for `T` and `T` and `U` are
`equivalent wrt Neg`.

For all types `W`, `Shl<W>` can be implemented for `T`. If for any integer type
`I` `Shl<I, Output=T>` is not is not explicitly implemented for `T` and
`Shl<I, Output=U>` is implemented for `U`, then `Shl<I, Output=T>` shall
automatically be implemented for `T` and `T` and `U` are
`equivalent wrt Shl<I>`.

`T` implements `Drop` if and only if `U` implements `Drop` and, if so, `T` and
`U` are `equivalent wrt Drop`.

`T` implements `Send` if and only if `U` implements `Send` and, if so, `T` and
`U` are `equivalent wrt Send`.

`T` implements `Sized` if and only if `U` implements `Sized` and, if so, `T` and
`U` are `equivalent wrt Sized`.

`T` implements `Copy` if and only if `U` implements `Copy` and, if so, `T` and
`U` are `equivalent wrt Copy`.

`T` implements `Sync` if and only if `U` implements `Sync` and, if so, `T` and
`U` are `equivalent wrt Sync`.

For all types `W`, `Fn<W>` can be implemented for `T`.

For all types `W`, `Index<W>` can be implemented for `T`.

For all types `W`, `IndexMut<W>` can be implemented for `T`.

`Deref` can be implemented for `T`.

`DerefMut` can be implemented for `T`.

The same rules as for `PartialOrd` apply to `PartialEq`.

The same rules as for `Add` apply to `Sub`.

The same rules as for `Add` apply to `Mul`.

The same rules as for `Add` apply to `Div`.

The same rules as for `Add` apply to `Rem`.

The same rules as for `Neg` apply to `Not`.

The same rules as for `Add` apply to `BitAnd`.

The same rules as for `Add` apply to `BitOr`.

The same rules as for `Add` apply to `BitXor`.

The same rules as for `Shl` apply to `Shr`.

The same rules as for `Fn` apply to `FnMut`.

The same rules as for `Fn` apply to `FnOnce`.

#### Default impls

Let `Trait` be a trait with a default impl:

```rust
impl Trait for .. { }
```

If there is neither an explicit `impl Trait for T` nor an explicit negative
`impl !Trait for T`, then `T` implements `Trait` if and only if `U` implements
`Trait` and, if so, `T` and `U` are `equivalent wrt Trait`.

#### Simple traits

Let `Trait` be a trait such that no types besides `Self` appear in its
definition. If `T` does not explicitly implement `Trait` and `U` implements
`Trait`, then `Trait` shall automatically be implemented for `T` and `T` and `U`
are `equivalent wrt Trait`.

## Example

Assume

```rust
newtype c_long = i64;
```

Then `c_long` has no inherent methods and implements the following operators:
`==, <, >, <=, >=, <<, >>, |, &, ^, +, -, *, /, %`.

`c_long` does not implement `Drop` but implements `Neg`, `Send`, `Sized`,
`Copy`, `Sync`, and `Clone`.

`c_long` does not coerce to `i64` but can be cast to any integer or floating
point type. Conversely, any such type can be cast to `c_long`.

`&c_long` does not coerce to `&i64` but can be cast to `&i64`.

`Vec<c_long>` does not coerce to `Vec<i64>` but can be cast to `Vec<i64>`.

## Rationale

The motivation of this RFC is to make `c_char`, `c_ulong`, `pid_t`, etc.
distinct types that need to be explicitly cast at the "rustic" interface
boundary or between each other. At first one might consider the following
design:

>`T` behaves as if it had been declared by `struct T(U);` except that one can
>cast between `T` and `U`.

There are some problems with this:

- `U` has to be sized for the definition of `T` to make sense. 
- `T` doesn't necessarily have the same memory representation as `U`.
- `&T` cannot be cast to `&U`.
- `T` cannot be used for anything meaningful because all operators in Rust are
  used via traits.
- `T` could implement `Drop` which would make casting to `U` unsafe.

The rules above have been chosen to be a conservative solution of these
problems.

# Drawbacks

None right now.

# Alternatives

## What other designs have been considered?

None.

## What is the impact of not doing this?

While not strictly backwards incompatible (except for the part about renaming
`type`), you want to use this for many types in `liblibc`.

# Unresolved questions

A previous version of this RFC suggested renaming `type` to `alias` and
`newtype` to `type`. However, `alias` makes not much sense in the context of
associated types:

```rust
trait Iterator {
    type Item;

    fn next(&mut self) -> Option<<Self as Iterator>::Item>;
}
```

Furthermore, if associated lifetimes were added to traits then the `alias`
keyword might become ambiguous:

```rust
trait Iterator {
    alias Item;
    lifetime UnusedLifetime;

    fn next(&mut self) -> Option<<Self as Iterator>::Item>;
}
```
