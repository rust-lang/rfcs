- Feature Name: partial_turbofish
- Start Date: 2017-10-16
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Assume a function (free, inherent, trait, ...) `fn turbo< $($tparam: ident),* )>(..) -> R` involving generic types `$($tparam: ident),*`. If while calling
`turbo::< $($tconcrete: ty),* >(...)` a suffix of the applied types can be
replaced with a list of `_`s of equal length, then the suffix may be omitted
entirely. A shorter suffix may be chosen at will. This also applies to
turbofish:ing types (structs, enums, ..), i.e: `Type::< $($tconcrete: ty),* >::fun(..)`.

In concrete terms, this entails that if `turbo::<u32, _>()` and `Turbo::<u32, _>::new()` typechecks, then `turbo::<u32>()` and `Turbo::<u32>::new()` must as well.

# Motivation
[motivation]: #motivation

When dealing with parametric polymorphism with more than one type parameter, if
the first parameter must be specified, but others may be inferred either from
the return type, arguments or the first parameter, then the developer is forced
to make the compiler happy by acknowledging that there are more type parameters that the compiler is already aware of.

A contrived example of this is:

```rust
use std::iter::FromIterator;
use std::iter::repeat;

fn turbo<T, U>(elt: U, n: usize) -> T
where
    U: Clone,
    T: FromIterator<U>
{
    repeat(elt).take(n).collect()
}

struct Turbo<T: FromIterator<U>, U>(T, U);

impl<T: FromIterator<U>, U> Turbo<T, U> {
    fn new(elt: U, n: usize) -> Self
    where
        U: Clone
    {
        Turbo(turbo(elt.clone(), n), elt)
    }
}

fn main() {
    // This compiles in today's Rust:
    let vec = turbo::<Vec<u32>, _>(1, 1);

    // This does not, but will compile:
    let vec = turbo::<Vec<u32>>(1, 1);

    // This compiles in today's Rust:
    let vec = Turbo::<Vec<u32>, _>::new(1, 1);

    // This does not, but will compile:
    let vec = Turbo::<Vec<u32>>::new(1, 1);
}
```

By letting the user omit any suffix of `_`s, the ergonomics of working with
generic types can be improved.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Currently, the line `let vec = Turbo::<Vec<u32>>::new(1, 1);` in [motivation]
results in the following errors:

For line 29:
```
error[E0089]: too few type parameters provided:
expected 2 type parameters, found 1 type parameter
```

For line 35:
```
error[E0243]: wrong number of type arguments: expected 2, found 1
```

However, in both cases, the compiler already knows that `U = u32` since this is
a requirement induced by `T: FromIterator<U>`.

If a user attempts to type check: `let vec = turbo::<Vec<u32>, i32>(1, 1);`,
rustc will complain that "the trait bound `Vec<u32>: FromIterator<i32>` is not satisfied". Since the only way to satisfy `Vec<T>: FromIterator<U>` is with `impl<T> FromIterator<T> for Vec<T>`, unifying `T` with `U`, subsituted to `u32` and `i32` fails.

Starting from `turbo::<Vec<u32>, _>(1, 1)`, the compiler can first see
that `T = Vec<u32>`. It can then see that the only way for `Vec<u32>: FromIterator<U>` is if `U = u32`. If `turbo::<Vec<_>, _>(true, 1)` is used, then rustc can start from `U = bool` and so it may infer from the other direction.

In either case, the argument to the second type parameter is known which is why
rustc allows `_` instead of a concrete type.

This RFC proposes that suffixes to a "turbofish" which consist entirely of `_`
be omittable.

The following:

```rust
fn main() {
    let vec = turbo::<Vec<u32>, _>(1, 1);
    let vec = Turbo::<Vec<u32>, _>::new(1, 1);
}
```

thus becomes:

```rust
fn main() {
    let vec = turbo::<Vec<u32>>(1, 1);
    let vec = Turbo::<Vec<u32>>::new(1, 1);
}
```

This concept is named "partial turbofish" since it is partially applying
type arguments to type parameters and letting the compiler infer the rest.

It is still an error to apply too many type arguments in a turbofish
as in: `fn foo() {} foo::<i32>()`, or to
partially apply types when the compiler can not infer the omitted types. When
the latter happens, rustc will emit error indicating that there are more type
parameters not applied that could not be inferred. A sample error message is:
`type annotations needed` in addition to:
```
Can not infer concrete type(s) of omitted type parameter(s) X, Y, ..
in call to fn turbo with parameters: <Concrete1, X, Y, ...>.
```

Some developers may end up trying to use this feature and be surprised when it
did not work. For those who this does not apply to, the documentation should
explain this feature when it explains the turbofish syntax.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## From a Haskell perspective

In Haskell, a function:
```haskell
fun :: a -> Int -> c
fun a i c = expr
```

is equivalent to:

```haskell
fun :: forall a c. a -> Int -> c
fun a i c = expr
```

which, in terms of [System F](https://en.wikipedia.org/wiki/System_F) is
the judgement that:
```
|- (Λ a. Λ b. λ x1^a. λ x2^Int. λ x3^c. expr) : ∀ a. ∀ c. a -> Int -> c
```

Here, `Λ a.` is a lambda function taking a type, and not a term.
In Haskell, the feature `TypeApplications` lets developers partially apply
types to such lambdas. Rust also allows this via the turbofish mechanism:
`::<T1, T2, ...>`. Unlike Haskell, Rust does however not allow the user to
only apply some types as a prefix, but requires the user to supply all types.
`TypeApplications` in Rust is therefore an all-or-nothing proposition: "Either
you give me all the concrete types, or I will try to infer them all for you".

## From a Rust perspective

This RFC allows the user to only supply only supply the smallest prefix of
concrete types that rustc requires to infer the type of the other parameters in
the suffix. The suffix may therefore be omitted. The prefix may be overspecified
and the omitted suffix may be shorter than the longest possible.

Formally:
Assume a function (free, inherent, trait, ...) `fn turbo< $($tparam: ident),* )>(..) -> R` or a type (struct/enum/..) `Turbo` involving generic types
`$($tparam: ident),*`. If while calling `turbo::< $($tconcrete: ty),* >(..)` or equivalently `Turbo::< $($tconcrete: ty),* >::fun(..)`, for a list of concrete types `$($tconcrete: ty),*`, a suffix of `$($tconcrete: ty),*` is `$( _ ),*` and
the type checking passes, then that suffix may be omitted.

## Type checking
[typeck]: #typeck

Currently, `typeck` may at an eariler stage (prior to full unification) check
if the number of parameters at the turbofish-site (where `::<T1, T2, ..>` happens) and at the definition site match. With this RFC, it may no longer do so. It can at most check if more arguments were specified at the turbofish-site compared to the definition site, if that happens, the (currently used) error:
```
error[E0087]: too many type parameters provided:
expected at most X type parameters, found Y type parameter
```
where `X < Y` is raised.

If fewer arguments are specified at the turbofish-site, then and at a later stage:
If typeck can infer all the unspecified arguments at the definition site,
the typeck stage passes (assuming that the specified types already unify with 
the requirements of the definition site).
If type can't infer those arguments, the smallest suffix it can't infer is used
in the error message specified in [guide-level-explanation].

# Drawbacks
[drawbacks]: #drawbacks

+ It *might* slow down type checking by not letting the compiler verify that the amount of type arguments applied at the call site are as many as those as the
type parameters at the definition site. Currently, such a check can be done
prior to full type checking.

# Rationale and alternatives
[alternatives]: #alternatives

The alternative is to not do this. This means that a papercut in deailing with
generics remains.

# Unresolved questions
[unresolved]: #unresolved-questions

+ How should the error messages when too many arguments are omitted look like?

+ Is the "algorithm" specified in [typeck] sound? Does it work? Is it sufficiently
detailed? If not, it must become sufficiently detailed prior to merging.