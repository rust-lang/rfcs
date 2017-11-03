- Feature Name: const_default
- Start Date: 2017-10-17
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

1. Adds the trait `ConstDefault` to libcore defined as:

```rust
pub trait ConstDefault { const DEFAULT: Self; }
```

2. Adds impls for all types which are `Default` and where the returned value in
`default` can be `const`. This includes impls for tuples where all factors are
`ConstDefault`.

3. Adds the blanket impl `impl<T: ConstDefault> Default for T` and removes all
prior `Default` impls that overlap.

4. Enables deriving of `ConstDefault` for structs iff all fields are also
`ConstDefault`. 

# Motivation
[motivation]: #motivation

The motivation is two-fold.

## Primary

The `Default` trait gives a lot of expressive power to the developer. However,
`Default` makes no compile-time guarantees about the cheapness of using `default`
for a particular type. It can be useful to statically ensure that any default
value of a type does not have large unforseen runtime costs.

With a default `const` value for a type, the developer can be more certain
(large stack allocated arrays may still be costly) that constructing the default
value is cheap.

An additional minor motivation is also having a way of getting a default constant
value when dealing with `const fn` as well as generics. For such constexpr +
generics to work well, more traits may however be required in the future.

## Secondary: To enhance `#[derive_unfinished(Trait)]`

[`#[derive_unfinished(Trait)]` and `#[unfinished]` RFC]: https://github.com/Centril/rfcs/blob/rfc/derive-unfinished/text/0000-derive-unfinished.md
[documentation on associated constants]: https://doc.rust-lang.org/1.16.0/book/associated-constants.html

Traits can contain associated `const`s. An example of such a trait, given in the
[documentation on associated constants] is:

```rust
trait Foo {
    const ID: usize;

    // other stuff...
}
```

If the compiler is to be able to derive an impl for any such trait for any type
as proposed by the [`#[derive_unfinished(Trait)]` and `#[unfinished]` RFC],
it must be able to give a value for the traits associated constants, if any. As
proposed in said RFC, one way of doing so, which covers a lot of cases, is by
having a notion of a constant default for a type.

Let us assume that `Foo`, as defined above, is given, as well as:

```rust
#[derive_unfinished(Foo)]
struct S;

impl ConstDefault for usize {
    const DEFAULT: Self = 0;
}
```

The compiler, after having resolved `Foo`, knows that `Foo::ID` is of type
`usize` and can now rewrite `#[derive_unfinished(Foo)]` on `S` into:
```rust
impl Foo for S {
    const ID: usize = usize::DEFAULT;
}
```
And thus, the `impl` has been derived as promised.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## The trait

The following trait:

```rust
pub trait ConstDefault {
    const DEFAULT: Self;
}
```

is added to `core::default` and re-exported in `std::default`.

This trait should be directly used if:
+ you want to be sure that the default value does not depend on the runtime.
+ you want to use the default value in a `const fn`.
+ you want to be able to use the type `T` that is to be `ConstDefault` as the
type of an associated `const` for a trait `Foo` and then `#[derive_unfinished(Foo)]`
for a type (struct/enum/..) as discussed in [motivation].

You may also, at your leisure, continue using `Default` for type `T` which
will yield `<T as ConstDefault>::DEFAULT`. This is especially true if you are
a newcomer to the language as `const` may be considered an advanced topic.

## `impls` for standard library

Then, `impl`s are added to the types that exist in the standard library which
are `Default` and for which the default value can be `const`.

For the numeric types, with `usize` as an example, the `impl`s like the
following are added:

```rust
impl ConstDefault for usize {
    const DEFAULT: Self = 0;
}
```

Another example is for `()`, which less useful, but nonetheless informative:
```rust
impl ConstDefault for () {
    const DEFAULT: Self = ();
}
```

Another, more interesting, case is for `Option<T>`:
```rust
impl ConstDefault for Option<T> {
    const DEFAULT: Self = None;
}
```

Equally interesting is the case for tuples:

```rust
impl<T0, T1> ConstDefault for (T0, T1)
where
    T0: ConstDefault,
    T1: ConstDefault,
{
    const DEFAULT: Self = (T0::DEFAULT, T1::DEFAULT);
}

impl<T0, T1, T2> ConstDefault for (T0, T1, T2)
where
    T0: ConstDefault,
    T1: ConstDefault,
    T2: ConstDefault,
{
    const DEFAULT: Self = (T0::DEFAULT, T1::DEFAULT, T2::DEFAULT);
}

// and so on..
```

For arrays, impls like the following can be added:

```rust
impl<T: ConstDefault> ConstDefault for [T; 2] {
    const DEFAULT: Self = [T::DEFAULT, T::DEFAULT];
}
```

## Blanket impl

To reduce repetition in the standard library incurred by all the new `impl`s,
a blanket impl is added:

```rust
impl<T: ConstDefault> Default for T {
    fn default() -> Self {
        <T as ConstDefault>::DEFAULT
    }
}
```

This blanket `impl` will now conflict with those `Default` `impl`s already in
the standard library. Therefore, those `impl`s are removed. This incurrs no
breaking change.

## Deriving

Just as you can `#[derive(Default)]`, so will you be able to
`#[derive(ConstDefault)]` iff all of the type's fields implement `ConstDefault`.
When derived, the type will use `<Field as ConstDefault>::DEFAULT` where
`Field` is each field's type.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## The trait

The following is added to `core::default` (libcore) as well as `std::default`
(stdlib, reexported):

```rust
pub trait ConstDefault {
    const DEFAULT: Self;
}
```

## `impls` for standard library

Impls are added for all types which are `Default` and where the returned
value in `<T as Default>::default()` can be `const`.

Many of these `Default`
impls are generated by macros. Such macros are changed to generate `ConstDefault` 
impls instead.

An example of how such a changed macro might look like is:

```rust
macro_rules! impl_cd_zero {
    ($($type: ty),+) => {
        $(
            impl ConstDefault for $type {
                const DEFAULT: Self = 0;
            }
        )+
    };
}

impl_cd_zero!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);
```

[const generics]: https://github.com/rust-lang/rfcs/blob/master/text/2000-const-generics.md
[const repeat expressions]: https://github.com/Centril/rfcs/blob/rfc/const-repeat-expr/text/0000-const-repeat-expr.md

Impls are also generated by macro for tuples and arrays.
If implemented [const generics] and [const repeat expressions] can be used to
implement the trait for arrays of arbitrary size, otherwise, impls can be
generated by macro for arrays up to a reasonable size.

## Blanket impl

The blanket impl referred to in [guide-level-explanation] is added and
overlapping impls are removed.

## Deriving

The mechanism and rules used for deriving `Default` are reused for `ConstDefault`.
They are however altered to produce a `const` item in the trait instead of a
function, and instead of a trait function call, the following is used for a
factor `Field` of a product type (tuples structs, normal structs - including
unit structs): `<Field as ConstDefault>::DEFAULT`.

## In relation to "Default Fields"

[RFC 1806]: https://github.com/rust-lang/rfcs/pull/1806

The currently postponed [RFC 1806], which deals with struct default field values,
allows the user to assign default values from `const` expressions to fields when 
defining a `struct` as in the following example:

```rust
struct Foo {
    a: &'static str,
    b: bool = true,
    c: i32,
}
```

The RFC argues that an alternative to the `const` requirement is to allow the
use of `Default::default()`  instead of just `const` expressions. However,
since `Default` may incur non-trival runtime costs which are un-predictable,
this is not the main recommendation of the RFC. As `<T as ConstDefault>::DEFAULT`
is const, this RFC is fully compatible with that RFC.

[RFC 1806] further mandates that when deriving `Default`, supplied field defaults
are used instead of the field type's `Default` impl. If RFC 1806 is added to this
language, for the sake of consistency the same logic should also apply to
`ConstDefault`.

# Drawbacks
[drawbacks]: #drawbacks

As always, adding this comes with the cost incurred of adding a trait and in
particular all the impls that come with it in the standard library. A mitigating
factor in this case is that a lot of impls for `Default` can simply be removed
and replaced with the blanket impl discussed earlier.

# Rationale and alternatives
[alternatives]: #alternatives

This design may in fact not be optimal. A more optimal solution may be to
add a `const` modifier on the bound of a trait which "magically" causes all
`fn`s in it to be considered `const fn` if possible. This bound may look like:
`T: const Default`. If there are any such implemented trait `fn`s for a given
type which can not also be considered `const fn`, then the bound will be
considered not fulfilled for the given type under impl.

The `T: const Default` approach may be considered heavy handed. In this case
it may be considered a bludgeon while the following approach is a metaphorical
scalpel: `<T as Trait>::method: ConstFn`. With this bound, the compiler is told
that `fn method` of `Trait` for `T` may be considered a `const fn`.

While the design offered by this RFC is not optimal compared to the two latter,
it is implementable today and is not blocked by: a) adding `const fn` to traits,
b) adding a `const` modifier to bounds. Such a proposal, while useful, is only
realizable far into the future. In the case of the last alternative,
Rust must be able to encode bounds for trait `fn`s as well as adding marker trait
which constrains the `fn`s to be const. This is most likely even more futuristic.

If `T: const Default` is the preferred alternative, this RFC still adds value
even if `ConstDefault` is not stabilized by satisfying the secondary [motivation]
regarding `#[derive_unimpl(Foo)]` (if such a proposal is implemented). The trait
may be used for `#[derive_unimpl(Foo)]` only. This will not leak the then-unstable
API of `ConstDefault`.

This RFC advocates that the more optimal alteratives are sufficiently far into
the future that the best course of action is to add `ConstDefault` now and then
deprecate it when **and if** any of the more optimal alternatives are added.

[RFC 1520]: https://github.com/rust-lang/rfcs/pull/1520

The `ConstDefault` proposed by this RFC was also independently discussed and
derived in the now closed [RFC 1520] as what could be achieved with generic consts.
The trait was not the actual suggestion of the RFC but rather discussed in passing.
However, the fact that the same identical trait was developed independently gives
greater confidence in its design.

# Unresolved questions
[unresolved]: #unresolved-questions

None, as of yet.