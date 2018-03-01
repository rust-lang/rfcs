- Feature Name: partial_turbofish
- Start Date: 2017-10-16
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Assume a function (free, inherent, trait, ...)
`fn turbo< $($tparam: ident),* )>(..) -> R` involving generic types
`$($tparam: ident),*`. If while calling `turbo::< $($tconcrete: ty),* >(...)` a
suffix of the applied types can be replaced with a list of `_`s of equal length,
then the suffix may be omitted entirely. A shorter suffix may be chosen at will.
This also applies to turbofishing types (structs, enums, ..), i.e:
`Type::< $($tconcrete: ty),* >::fun(..)`.

In concrete terms, this entails that if `turbo::<u32, _>()` and
`Turbo::<u32, _>::new()` typechecks, then `turbo::<u32>()` and
`Turbo::<u32>::new()` must as well.

# Motivation
[motivation]: #motivation

When dealing with parametric polymorphism with more than one type parameter, if
the first parameter must be specified, but others may be inferred either from
the return type, arguments or the first parameter, then the developer is forced
to make the compiler happy by acknowledging that there are more type parameters
that the compiler is already aware of.

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

## API evolution

[RFC 1196]: https://github.com/Centril/rfcs/blob/rfc/partial-turbofish/text/0000-partial-turbofish.md
[1196-motivation]: https://github.com/huonw/rfcs/blob/prefix-ty-param/text/0000-prefix-type-params.md#motivation

This RFC is a continuation of [RFC 1196] which talks in its
[motivation][1196-motivation] about how this feature may enable adding new,
inferrable (due to eq-constraining), type parameters to functions without
breaking code. This RFC doubles down on this argument as a good motivation.

## Improving a real world scenario for `Arbitrary` in `proptest`

Let us assume the following trait from the crate `proptest` which has been
simplified a bit here:

```rust
pub trait Arbitrary: Sized {
    type Parameters: Default;

    fn arbitrary() -> Self::Strategy {
        Self::arbitrary_with(Default::default())
    }

    fn arbitrary_with(args: Self::Parameters) -> Self::Strategy;

    type Strategy: Strategy<Value = Self::ValueTree>;
    type ValueTree: ValueTree<Value = Self>;
}

pub fn any_with<A: Arbitrary>(args: A::Parameters) -> A::Strategy {
    A::arbitrary_with(args)
}

pub fn arbitrary_with<A, S, P>(args: P) -> S
where
    P: Default,
    S: Strategy,
    S::Value: ValueTree<Value = A>,
    A: Arbitrary<Strategy = S, ValueTree = S::Value, Parameters = P>,
{
    A::arbitrary_with(args)
}
```

Semantically, the two free functions `any_with` and `arbitrary_with` do the
same thing. However, the function `arbitrary_with` has other properties
with respect to type inference than `any_with`. In many cases, it works fine
to do `arbitrary_with(..)`, but with `any_with` you often have to specify a
concrete the type variable `A`. But if you need or want to (for extra clarity)
to specify the first type for `arbitrary_with`, you now have to fill in the
blanks using `_` as in: `arbitrary_with::<usize, _, _>(..)`. If we instead
were allowed to elide those `_`s, we could instead just have one function
that could be used in both for better type inference as well as more ergonomic
turbofishing.

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
rustc will complain that "the trait bound `Vec<u32>: FromIterator<i32>` is not
satisfied". Since the only way to satisfy `Vec<T>: FromIterator<U>` is with
`impl<T> FromIterator<T> for Vec<T>`, unifying `T` with `U`, subsituted to `u32`
and `i32` fails.

Starting from `turbo::<Vec<u32>, _>(1, 1)`, the compiler can first see
that `T = Vec<u32>`. It can then see that the only way for
`Vec<u32>: FromIterator<U>` is if `U = u32`. If `turbo::<Vec<_>, _>(true, 1)`
is used, then rustc can start from `U = bool` and so it may infer from the other
direction.

In either case, the argument to the second type parameter is known which is why
rustc allows `_` instead of a concrete type.

This RFC proposes that suffixes to a "turbofish" which consist entirely of
`, _`s be omittable.

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

This concept is named *partial turbofish* since it is partially applying
type arguments to type parameters and letting the compiler infer the rest.
When `, _` is present in a turbofish, it is instead referred to as *turbosword*.

It is still an error to apply too many type arguments in a turbofish
as in: `fn foo() {} foo::<i32>()`, or to partially apply types when the
compiler can not infer the omitted types. When the latter happens, rustc will
emit error indicating that there are more type parameters not applied that
could not be inferred. A sample error message is: `type annotations needed`
in addition to:

```
Can not infer concrete type(s) of omitted type parameter(s) `X, Y, ..`
in call to `fn turbo` with parameters: `<Concrete1, X, Y, ...>`.
```

Some developers may end up trying to use this feature and be surprised when it
did not work. For those who this does not apply to, the documentation should
explain this feature when it explains the turbofish syntax.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This RFC allows the user to only supply the smallest prefix of
concrete types that rustc requires to infer the type of the other parameters in
the suffix. The suffix may therefore be omitted. The prefix may be overspecified
and the omitted suffix may be shorter than the longest possible.

Formally: Assume a function (free, inherent, trait, ...)
`fn turbo< $($tparam: ident),* )>(..) -> R` or a generic type (struct/enum/..)
`Turbo` with type parameters `$($tparam: ident),*`. If while calling
`turbo::< $($tconcrete: ty),* >(..)` or equivalently
`Turbo::< $($tconcrete: ty),* >::fun(..)`, for a list of concrete types
`$($tconcrete: ty),*`, a suffix of `$($tconcrete: ty),*` is `$( _ ),*` and
the type checking passes, then that suffix may be omitted.

## Type checking
[typeck]: #typeck

Currently, `typeck` may at an eariler stage (prior to full unification) check
if the number of parameters at the turbofish-site (where `::<T1, T2, ..>`
happens) and at the definition site match. With this RFC, the compiler will
instead check if more arguments were specified at the turbofish-site compared to the definition site, and if so, the the following
error (where `X < Y`) is raised:

```
error[E0087]: too many type parameters provided:
expected at most X type parameters, found Y type parameter
```

This feature does not require the introduction of type lambdas for terms,
instead, it can be achieved by having the type checker relax the above specified
rule. If fewer arguments are specified at the turbofish-site, the compiler
mechanically fills in `len(args@turbofish-site) - len(params@definition-site)`
amount of `, _`s. From this point onwards, type checking continues as normal.

If typeck can infer the concrete types of all the `_` type arguments, the typeck
stage passes (assuming that the specified types already unify with the
requirements of the definition site). If type can't infer concrete types of the
`_`s inserted by the compiler, the smallest suffix of those `_`s rustc can't
infer is used in the error message specified in [guide-level-explanation].

With respect to default type parameter fallback, the mechanics of a partial
turbofish does not directly interact with the fallback. The compiler fills in
the remaining `, _`s. If there are fallbacks defined which can be used given
the constraints placed by the call site, then the relevant subset of `_`s will
be substituted for the specified default types. This inference happens after
inserting any necessary `, _`s.

# Drawbacks
[drawbacks]: #drawbacks

+ The user will no longer be met with a hard error when new type parameters
are introduced. While this is mostly seen as an advantage, some users may
want to get such errors.

+ It *might* slow down type checking to a small degree by having the compiler
insert any additional `, _`s. The compiler still has to check the length of
type arguments passed at the call site against the number of type parameters at
the definition site in any case.

# Rationale and alternatives
[alternatives]: #alternatives

The alternative is to not do this. This means that a papercut in deailing with
generics remains.

## Acknolowedging extra inferred parameters with `..`

Another alternative is to use the syntax `turbo::<u32, ..>()` to get rid of
any extra `, _`s. This syntax may however be more useful for variadics. Also,
the `, ..` makes the call longer for the most common case which is when you have
one argument to add as in: `turbo::<u32, _>()`. Additionally, using `..` to
acknowledge that inference is going on is inconsistent with the language in
general. There is no requirement to acknowledge type inference elsewhere.
Instead, the fact that type inference is going on should itself be inferred.

## Opting into partial turbofish at definition site

In a possible amendment to this RFC, we could require that the definition site
of a function or a data constructor explicitly opt into allowing turbofish with
the proposed syntax:

```rust
fn foo<T, U = _, V = _>(..) { .. }
```

While this gives a greater degree of control to authors, it also comes with
downsides:

1. Assuming that partial turbofish is the desirable default behavior, authors
   of generic functions are penalized by having to opt in with `= _` on every
   generic parameter.

2. More choices comes with an increased mental cost of deciding whether to
   use `= _` or not, which may not be worth the additional control afforded.

3. Opting in leaves users of generic functions and data constructors with doubt
   as to whether this or that function has opted in, wherefore they must check
   the documentation. This uncertainty can contribute to disturbing writing flow.

# Prior art

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

Here, `Λ a.` is a lambda function taking a type, and not a term. In Haskell,
the language pragma `TypeApplications` enabled with lets developers partially
apply  types to such lambdas with the syntax: `fun @Int :: Int -> Int -> c`.
Rust also allows type application via the turbofish mechanism: `::<T1, T2, ...>`.
Unlike Haskell, Rust does however not allow the user to only apply some types
as a prefix, but requires the user to supply all types. Type application in
Rust is therefore an all-or-nothing proposition: "Either you give me all the
concrete types, or I will try to infer them all for you".

# Unresolved questions
[unresolved]: #unresolved-questions

+ How should the error messages when too many arguments are omitted look like?