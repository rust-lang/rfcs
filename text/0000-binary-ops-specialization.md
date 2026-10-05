- Feature Name: binary_ops_specialization
- Start Date: 2018-10-20
- RFC PR: [rust-lang/rfcs#2578](https://github.com/rust-lang/rfcs/pull/2578)
- Rust Issue:

# Summary
[summary]: #summary

Provide a family of traits to augment overloading of binary operators
with more practical generic implementations. The traits are named
`DefaultPartialEq`, `DefaultPartialOrd`, `DefaultAdd`, `DefaultAddAssign`, etc.
For these traits, default generic implementations abstracted over `Borrow`
are to be provided for built-in and standard library types, including
`[u8]`, `Vec`, `str`, `String`, `OsStr`, `OsString`, `Path`, `PathBuf`.
This will drastically reduce the number of explicit impl items necessary to
cover all type pairs on which the binary operators can act.

When resolving the implementation for an operator, the compiler will
consider the impls for the new "default" overload trait corresponding to the
operator as the second choice after its Rust 1.0 overload trait.

# Motivation
[motivation]: #motivation

Operator overloading makes data types more convenient to use.
For a set of types which provide different containers and ownership patterns
for the same underlying data type (usually indicated by implementing the same
`Borrow<T>`),
it makes sense to define binary operator trait impls that act on each pair of
these types. However, with proliferation of special-purpose representations
of widely used data types like byte arrays and strings, the number of
possible such pairs undergoes a quadratic explosion. Each of the crates
defining a type in the operator-compatible set has to be in a dependency
relationship with any other of such crates, otherwise there cannot be
an operator impl to make them work together.

[Specialization][rfc1210] of trait implementations could be a convenient way to
deal with this problem. This implementation of `PartialEq` could automatically
enable equality comparison for `String` on the left hand side and any type
on the right hand side that implements `Borrow<str>`:

```rust
impl<Rhs> PartialEq<Rhs> for String
where
    Rhs: ?Sized + Borrow<str>,
{
    default fn eq(&self, other: &Rhs) -> bool {
        self.as_str() == other.borrow()
    }
}
```

However, introducing default impls for operator traits that have been
stable since Rust 1.0 is a breaking change: there are crates that don't
restrict their binary operators to types sharing the same `Borrow` target,
so their overload trait implementations will come into conflict.
One example is `bytes` defining `PartialEq` impls that allow comparing
`Bytes` and the standard string types. While such data domain crossing is
problematic for other reasons (e.g. differences in `Hash` for values that
compare as equal), the change should not break crates doing what has not been
previously forbidden. Newly introduced fallback overload traits with generic
impls abstracted over `Borrow` provide a backward compatible solution
and lay down some discipline.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## Default operator traits
[default-operator-traits]: #default-operator-traits

This proposal adds secondary overload traits for all overloadable binary
operators where the right-hand operand type is generic.
The traits are named `DefaultPartialEq`, `DefaultPartialOrd`, `DefaultAdd`, etc.
and defined with the same type parameters and method signatures as the plain
old Rust 1.0 operator overload traits.

Example for `DefaultPartialEq`:

```rust
pub trait DefaultPartialEq<Rhs: ?Sized = Self> {
    fn eq(&self, other: &Rhs) -> bool;
    fn ne(&self, other: &Rhs) -> bool {
        !self.eq(other)
    }
}
```

## Default implementation rule
[default-implementation-rule]: #default-implementation-rule

Types that need to work as an operand type in binary operators broadly
fall into two categories. One category is different purpose-specific
representations of an underlying data type that provides binary
operators acting on itself. We'll call it the **base operand type** for the
purposes of this proposal.
For example, `String` is the standard owned counterpart of `str`,
and `PathBuf` is this for `Path`. The base operand type itself is
considered together with its operand type family for the following rule.
Types in each such family usually implement `Borrow` to the base operand type,
and their binary operator trait impls, as currently provided, tend to cover
any possible pairs with the other types in the family.

The rule for a crate defining such a type is to also define generic default
implementations of the default operator overload traits described in
this RFC, where a generic type parameter bound by `Borrow` to the base
operand type defines the operand type other than `Self`:

```rust
impl<Rhs> DefaultPartialEq<Rhs> for String
where
    Rhs: ?Sized + Borrow<str>,
{
    default fn eq(&self, other: &Rhs) -> bool {
        &self[..] == other.borrow()
    }
}
```

The type parameter of the `Borrow` bound is the base operand type for `Self`
(which is `Self` in case the impl is defined for the base type itself).
The new semantic of `Borrow` therefore extends the "acts the same"
guarantee to binary operators available for the implementing type, which
is already the case for the intra-type traits `Eq`, `Ord`, and `Hash`.
Notably, the standard library largely maintains this stratification in the
provided implementations of `Borrow` and the plain old operator traits;
`PathBuf`/`Path` is a [problematic][issue55319] exception.

Operator traits that take ownership of the operands are trickier to implement
for non-`Copy` types: these should not work between two borrowed
values to avoid allocations or other side effects hidden in operator notation,
while moving both owned operands into an operator expression
may be non-ergonomic.
A precedent is set in the `Add` implementation for `String` to only let
the left hand operand value be moved into the expression, owing to the
left-associative order of evaluation; the right hand side needs to coerce
to an `str` reference.
`Deref` coercions go a long way to make pointers to various string types
fit that impl, but to extend the operator's applicability to any types
that satisfy `Borrow<str>`, the crate `std` may provide this
default implementation of the new trait `DefaultAdd`:

```rust
impl<'a, T> DefaultAdd<&'a T> for String
where T: Borrow<str>
{
    type Output = String;

    default fn add(self, other: &'a T) -> String {
        self + other.borrow()
    }
}
```

Other types do not have an underlying borrowable type indicating their data
domain, but they still need binary operators to apply across some
family of types. Examples from the standard library are `IpAddr`, `Ipv4Addr`,
and `Ipv6Addr`. These types can have their plain old operator trait impls
defined just like they do now.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

When picking the implementation for an operator overloaded by the notional
traits `Op` and `DefaultOp`, the compiler will consider the available
implementations for `Op` first, before falling back to `DefaultOp`.

The proposed system allows the existing operator trait implementations
to coexist with the newly introduced generic default overload trait
implementations in a complementary and backward compatible way.
Systematic application of the [default impl rule][default-implementation-rule]
can provide any-to-any operand type compatibility for all types sharing a
particular `Borrow` bound, without necessity for any two crates defining
these types to be in a direct dependency relationship.

Specialized implementations of the default overload traits can be defined
when practical, within the `Borrow` bound of the default implementation.
Crate authors are also free to provide new impls of the plain old overload
traits, which override the generic impls of the default overload traits
for purposes of operator overloading, or apply outside of the type families
circumscribed by the default overload trait impls.

# Drawbacks
[drawbacks]: #drawbacks

The fallback overload traits add complexity, especially to operator
overload resolution. It's likely that implementations for both default and
plain old operator traits will have to be provided side by side to support
older versions of the compiler, which increases the possibility of
implementation errors. This takes further the precedent set by the
[specialization RFC][rfc1210] that multiple different implementations may be
considered to fit one use.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The proposed [rule][default-implementation-rule] of defining generic
operator impls slashes the quadratic explosion of mostly tedious
non-generic operator trait implementations that takes place today.
The crate authors are free to define non-generic impls of plain old operator
traits as they see fit, including outside of the `Borrow` type family
of the default generic impl.

Previous revisions of this RFC envisioned the new traits as replacements
for the Rust 1.0 operator traits, which would be soft-deprecated. This
limited the space for any new custom overload impls to specializations of the
`Borrow` bound of the default generic impl, and complicated the rules
for overload resolution in order to avoid "spooky action at a distance",
when adding generic impls of new style operator traits could shadow old style
concrete impls defined in a different crate.

# Prior art
[prior-art]: #prior-art

The language evolution that led to this design seems unique to Rust.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None.

[rfc1210]: ./1210-impl-specialization.md
[issue55319]: https://github.com/rust-lang/rust/issues/55319
