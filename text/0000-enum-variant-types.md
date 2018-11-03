- Feature Name: `enum_variant_types`
- Start Date: 03-11-2018
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

Enum variants are considered types in their own rights. This allows them to be irrefutably matched
upon. Where possible, type inference will infer variant types, but as variant types may always be
treated as enum types this does not cause any issues with backwards-compatibility.

```rust
enum Either<A, B> { L(A), R(B) }

fn all_right<A, B>(b: B) -> Either<A, B>::R {
    Either::R(b)
}

let Either::R(b) = all_right::<(), _>(1729);
println!("b = {}", b);
```

# Motivation
[motivation]: #motivation

When working with enums, it is frequently the case that some branches of code have assurance that
they are handling a particular variant of the enum. This is especially the case when abstracting
behaviour for a certain enum variant. However, currently, this information is entirely hidden to the
compiler and so the enum types must be matched upon even when the variant is certainly known.

By treating enum variants as types in their own right, this kind of abstraction is made cleaner,
avoiding the need for code patterns such as:
- Passing a known variant to a function, matching on it, and use `unreachable!()` arms for the other
variants.
- Passing individual fields from the variant to a function.
- Duplicating a variant as a standalone `struct`.

However, though abstracting behaviour for specific variants is often convenient, it is understood
that such variants are intended to be treated as enums in general. As such, the variant types
proposed here have identical representations to their enums; the extra type information is simply
used for type checking and permitting irrefutable matches on enum variants.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The variants of an enum are considered types in their own right, though they are necessarily
more restricted than most user-defined types. This means that when one define an enum, one is more
precisely defining a collection of types: the enumeration itself, as well as each of its
variants. However, the variant types act very similarly to the enum type in the majority of cases.

Specifically, variant types act differently to enum types in the following case:
- When pattern-matching on a variant type, only the constructor corresponding to the variant is
considered possible. Therefore one may irrefutably pattern-match on a variant.

Variant types, unlike most user-defined types are subject to the following restriction:
- Variant types may not have inherent impls, or implemented traits. That means `impl Enum::Variant`
and `impl Trait for Enum::Variant` are forbidden. This dissuades inclinations to implement
abstraction using behaviour-switching on enums, rather than using traits as is natural in Rust.

Variant types may be aliased with type aliases.

If a value of a variant type is explicitly cast to the type of its enum using a type annotation or
by passing it as an argument or return-value to or from a function, the variant information is lost
(that is, a variant type *is* different to an enum type, even though they behave very similarly).

Note that enum types may not be coerced to variant types. Instead, matching must be performed to
guarantee that the enum type truly is of the expected variant type.

```rust
enum Sum { A, B, C }

let s: Sum = Sum::A;

let a = s as Sum::A; // error
let a: Sum::A = s; // error

if let a @ Sum::A = s {
    // ok, `a` has type `Sum::A`
}
```

## Type parameters
Consider the following enum:
```rust
enum Either<A, B> {
    L(A),
    R(B),
}
```
Here, we are defining three types: `Either`, `Either::L` and `Either::R`. However, we have to be
careful here with regards to the type parameters. Specifically, the variants may not make use of
every generic paramter in the enum. Since variant types are generally considered simply as enum
types, this means that the variants need all the type information of their enums, including all
their generic parameters.

So, in this case, we have the types: `Either<A, B>`, `Either<A, B>::L` and `Either::<A, B>::R`.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

A new variant, `Variant(DefId, VariantDiscr)`, will be added to `TyKind`, whose `DefId` points to
the enclosing enum for the variant and `VariantDiscr` is the discriminant for the variant in
question. In most cases, the handling of `Variant` will simply delegate any behaviour to its `enum`.
However, pattern-matching on the variant allows irrefutable matches on the particular variant. In
effect, `Variant` is only relevant to type checking/inference and the matching logic.

Constructors of variants, as well as pattern-matching on particular enum variants, are now
inferred to have variant types, rather than enum types.

```rust
enum Sum {
    A(u8),
    B,
    C,
}

let x = Sum::A(5); // x: Sum::A
let Sum::A(y) = x; // ok, y = 5

fn sum_match(s: Sum) {
    match s {
        a @ Sum::A(_) => {
            let x = a; // ok, a: Sum::A
        }
        b @ Sum::B => {
            // b: Sum::B
        }
        c @ Sum::C => {
            // c: Sum::C
        }
    }
}
```

In essence, a value of a variant is considered to be a value of the enclosing `enum` in every matter
but pattern-matching.

Explicitly casting to the `enum` type forgets the variant information.

```rust
let x: Sum = Sum::A(5); // x: Sum
let Sum::A(y) = x; // error: refutable match
```

In all cases, the most specific type (i.e. the variant type if possible) is chosen by the type
inference. However, this is entirely backwards-compatible, because `Variant` acts as `Adt` except in
cases that were previously invalid (i.e. pattern-matching, where the extra typing information was
previously unknown).

# Drawbacks
[drawbacks]: #drawbacks

- The loose distinction between the `enum` type and its variant types could be confusing to those unfamiliar with variant types. Error messages might specifically mention a variant type, which could
be at odds with expectations. However, since they generally behave identically, this should not
prove to be a significant problem.
- As variant types need to include generic parameter information that is not necessarily included in
their definitions, it will be necessary to include explicit type annotations more often than is
typical. Although this is unfortunate, it is necessary to preserve all the desirable behaviour of
variant types described here: namely complete backwards-compatibility precise type inference
(e.g. allowing `x` in `let x = Sum::A;` to have type `Sum::A` without explicit type annotations).

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The advantages of this approach are:
- It naturally allows variants to be treated as types, intuitively.
- It doesn't require explicit type annotations to reap the benefits of variant types.
- As variant types and enum types are represented identically, there are no coercion costs.
- It doesn't require type fallback.
- It doesn't require value-tracking or complex type system additions such as refinement types.

One obvious alternative is to represent variant types differently to enum types and then coerce them
when used as an enum. This could potentially reduce memory overhead for smaller variants
(additionally no longer requiring the discriminant to be stored) and reduce the issue with providing
irrelevant type parameters. However, it makes coercion more expensive and complex (as a variant
could coerce to various enum types depending on the unspecified generic parameters).

Variant types have [previously been proposed for Rust](https://github.com/rust-lang/rfcs/pull/1450).
However, it used a more complex type inference procedure based on fallback and permitted fallible
coercion to variant types. The method proposed here is implementationally simpler and more
intuitive.

# Prior art
[prior-art]: #prior-art

Type-theoretically, enums are sum types. A sum type `S := A + B` is a type, `S`, defined in relation
to two other types, `A` and `B`. Variants are specifically types, but in programming it's usually
useful to consider particular variants in relation to each other, rather than standalone (which is
why `enum` *defines* types for its variants rather than using pre-existing types for its variants).

However, it is often useful to briefly consider these variant types alone, which is what this
RFC proposes.

Although sum types are becoming increasingly common in programming languages, most do not choose to
allow the variants to be treated as types in their own right. However, we propose that the patterns
in Rust make variant types more appealing than they might be in other programming languages with
variant types.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- None.
