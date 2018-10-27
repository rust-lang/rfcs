- Feature Name: binary_ops_specialization
- Start Date: 2018-10-20
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

Provide new traits for overloading binary operators, such as
`PartialEq2`, `PartialOrd2`, `Add2`, `AddAssign2`, etc.,
with default blanket implementations abstracted over `Borrow`
for built-in and standard library types. This will drastically reduce
the number of explicit impl items necessary to cover compatible type pairs.
When resolving the implementation for an operator, the compiler will
consider the new traits along with the old school non-specializable traits.

# Motivation
[motivation]: #motivation

Operator overloading brings a lot of convenience into usage of data types.
When a Rust type is one of multiple representations of the same underlying
data type (usually indicated by implementing the same `Borrow<T>`), it makes
sense to define binary operator trait impls that work between each pair of
these types. However, with proliferation of special-purpose representations
of widely used data types, such as byte arrays and strings, the number of
possible such pairs undergoes a combinatorial explosion. Each of
the crates defining such special-purpose types has to be in a dependency
relationship with any other of these crates, otherwise there cannot be
an operator impl to make them work together.

[Specialization][rfc1210] of blanket trait implementations could be used to
deal with this problem. These two impls of `PartialEq` could automatically
enable equality comparison for `String` on the left hand side and any type
on the right hand side that implements `Borrow<str>`:

```rust
impl PartialEq<str> for String {
    fn eq(&self, other: &str) -> bool {
        &self[..] == other
    }
}

impl<Rhs> PartialEq<Rhs> for String
where
    Rhs: ?Sized + Borrow<str>,
{
    default fn eq(&self, other: &Rhs) -> bool {
        &self[..] == other.borrow()
    }
}
```

However, introducing default impls for already defined operator traits
is a breaking change: there are crates that don't restrict their
binary operator type pairs to ones sharing the same `Borrow` target.
One example is `bytes` defining `PartialEq` impls that allow comparing
`Bytes` and the standard string types. While such data domain crossing is
problematic for other reasons (e.g. differences in `Hash` for values that
compare as equal), the change should not break crates doing what has not been
forbidden. New operator traits with blanket default impls abstracted over
`Borrow` can provide a migration path and lay down discipline.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## New operator traits
[new-operator-traits]: #new-operator-traits

This proposal adds second-generation traits for all binary operators in
the standard library where the right-hand operand type is defined generically.
The traits are named `PartialEq2`, `PartialOrd2`, `Add2`, etc. and defined
with the same method signatures as their Rust 1.0 counterparts. Example
for `PartialEq2`:

```rust
pub trait PartialEq2<Rhs: ?Sized = Self> {
    fn eq2(&self, other: &Rhs) -> bool;
    fn ne2(&self, other: &Rhs) -> bool { !self.eq(other) }
}
```

## Default blanket implementation rule
[default-blanket-implementation-rule]: #default-blanket-implementation-rule

Types that need to work as an operand type in binary operators broadly
fall into two categories. One category is use case specific representations
of an underlying data type that usually provides binary operators on itself.
For example, `String` is the owned version of `str`, and `PathBuf` is that
for `Path`. The underlying types themselves are counted in for the purposes
of the following rule.
These types usually implement `Borrow` to the basic type, and their Rust 1.0
binary operator trait impls tend to cover any pairs with other types that
satisfy the same `Borrow`. The rule for the crate defining such a type is to
also define default impls of the new-style operator traits described in
this RFC, where this type is the `&self` or `self` operand type, and a
generic type parameter bound by `Borrow` defines the other operand's type:

```rust
impl<Rhs> PartialEq2<Rhs> for String
where
    Rhs: ?Sized + Borrow<str>,
{
    default fn eq2(&self, other: &Rhs) -> bool {
        &self[..] == other.borrow()
    }
}
```

The type parameter of the `Borrow` bound is the basic data type that `Self`
can also be borrowed as (which can be just `Self`). The role of `Borrow`
therefore extends to restricting operand types of binary operators
available for the implementing type.
Notably, the standard library largely maintains this stratification in the
provided implementations of `Borrow` and Rust 1.0 operator traits;
`PathBuf`/`Path` is a [problematic][issue55319] exception.

Operator traits that take ownership of the operands are trickier to implement
for non-`Copy` types: these should not work between two borrowed types
to avoid allocations hidden in operator notation, while consuming the
owned operands in an operator expression may be non-ergonomic.
A precedent is set in the `Add` implementation for `String` to only let
the left-hand operand value to be moved into the expression; the right-hand
side needs to be borrowed as a `str` reference.
To extend the operator's applicability to any types that satisfy
`Borrow<str>`, the crate `std` defines this default implementation of
the new trait `Add2`:

```rust
impl<'a, T> Add2<&'a T> for String
where T: Borrow<str>
{
    type Output = String;

    default fn add2(self, other: &'a T) -> String {
        self + other.borrow()
    }
}
```

Other types do not have an underlying borrowable type to define their data
domain, but they still need cross-type operator compatibility between some
family of types. Examples from the standard library are `IpAddr`, `Ipv4Addr`,
and `Ipv6Addr`. These types can have their operator trait impls defined
in the old school non-generic way.

## Overload resolution
[overload-resolution]: #overload-resolution

When picking the implementation for an operator backed by the proverbial
traits `Op` and `Op2`, the compiler will consider the available trait
implementations in the following order:

1. Fully specialized impls of `Op2`;
2. Fully specialized impls of `Op`;
3. Default impls of `Op2`;
4. Default impls of `Op`.

## Path for future migration
[path-for-future-migration]: #path-for-future-migration

The Rust 1.0 binary operator traits can(?) be deprecated
after the new traits are introduced. In the future backward-incompatible
Rust 2.0, the new traits will lose their `2` name suffix and replace the
old school operator traits.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The proposed system allows legacy Rust 1.0 operator trait implementations
to coexist with the new blanket implementations in a backward-compatible way.
Systematic application of the [default impl rule][default-blanket-implementation-rule]
can provide any-to-any operand type compatibility for all types sharing a
particular `Borrow` bound, without necessity for any two crates, each defining
one of these types, to be in a direct dependency relationship.
Specialized implementations of new style traits can be defined when practical,
within the `Borrow` bound of the default implementation. Crates can also
choose to provide default (or even fully specialized blanket) impls of
the legacy traits, but new-style impls should be preferred in new APIs.

The interleaved, specialized-first overload resolution rule is designed to
prevent "spooky action at a distance" where e.g. adding a blanket impl of
`Add2` for type `A` defined in one crate could shadow an existing
`impl Add<B> for A` in another crate that defines `B`. The situation where
non-generic impls of `Add` and `Add2`, defined in different crates, could
apply to the same pair of types, is impossible due to acyclicity of crate
dependencies and the orphan rule.

# Drawbacks
[drawbacks]: #drawbacks

The second-generation traits add complexity, especially to operator
overload resolution. It's likely that both new and old school trait impls
will have to be provided side by side, which increases the possibility of
implementation errors. This takes further the precedent set by the
[specialization RFC][rfc1210] that multiple different implementations may be
considered to fit one use.

`Borrow<T>` may prove to be too inflexible a bound for some interoperable
type families. It's a challenge, though, to come up with an example in
existing designs that goes beyond a few closely knit types.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The proposed [rule][default-blanket-implementation-rule] of defining
operator impls slashes the combinatorial explosion of mostly tedious
operator trait implementations seen today, leaving reasonable flexibility
in which operand type pairs are allowed (with `Borrow` as the guiding force).

Addition of second-generation traits on top of the existing system
provides a backward-compatible migration path for the Rust 1.x timeframe.

If opt-in feature gates were possible in the stable channel, the new default
impls could be defined for the Rust 1.0 traits and hidden behind a feature
gate. It's unclear to the author if this could work without the need for
all crates in the dependency graph to be compatible with the feature.

# Prior art
[prior-art]: #prior-art

Labeling second-generation APIs with suffix `2` to allow coexistence
with the legacy APIs is common.

The circumstances that led to the design seem unique to Rust.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

Is it feasible to implement overload resolution in the compiler as proposed?

[rfc1210]: ./1210-impl-specialization.md
[issue55319]: https://github.com/rust-lang/rust/issues/55319
