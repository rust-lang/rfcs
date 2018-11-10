- Feature Name: `enum_variant_types`
- Start Date: 10-11-2018
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

Consider enum variants types in their own rights. This allows them to be irrefutably matched
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
they are handling a particular variant of the enum ([1], [2], [3], [4], [5], etc.). This is especially the case when abstracting
behaviour for a certain enum variant. However, currently, this information is entirely hidden to the
compiler and so the enum types must be matched upon even when the variant is certainly known.

[1]: https://github.com/rust-lang/rust/blob/69a04a19d1274ce73354ba775687e126d1d59fdd/src/liballoc/borrow.rs#L245-L248
[2]: https://github.com/rust-lang/rust/blob/69a04a19d1274ce73354ba775687e126d1d59fdd/src/liballoc/raw_vec.rs#L424
[3]: https://github.com/rust-lang/rust/blob/69a04a19d1274ce73354ba775687e126d1d59fdd/src/librustc_mir/transform/simplify.rs#L162-L166
[4]: https://github.com/rust-lang/rust/blob/69a04a19d1274ce73354ba775687e126d1d59fdd/src/librustc_resolve/build_reduced_graph.rs#L301
[5]: https://github.com/rust-lang/rust/blob/69a04a19d1274ce73354ba775687e126d1d59fdd/src/librustc_resolve/macros.rs#L172-L175

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
more restricted than most user-defined types. This means that when you define an enum, you are more
precisely defining a collection of types: the enumeration itself, as well as each of its
variants. However, the variant types act identically to the enum type in the majority of cases.

Specifically, variant types act differently to enum types in the following case:
- When pattern-matching on a variant type, only the constructor corresponding to the variant is
considered possible. Therefore you may irrefutably pattern-match on a variant:

```rust
enum Sum { A(u32), B, C }

fn print_A(a: Sum::A) {
    let A(x) = a;
    println!("a is {}", x);
}
```
However, in order to be backwards-compatible with existing handling of variants as enums, matches on
variant types will permit (and simply ignore) arms that correspond to other variants:

```rust
let a = Sum::A(20);

match a {
    A(x) => println!("a is {}", x),
    B => println!("a is B"), // ok, but unreachable
    C => println!("a is C"), // ok, but unreachable
}
```

- You may project the fields of a variant type, similarly to tuples or structs:

```rust
fn print_A(a: Sum::A) {
    println!("a is {}", a.0);
}
```

Variant types, unlike most user-defined types are subject to the following restriction:
- Variant types may not have inherent impls, or implemented traits. That means `impl Enum::Variant`
and `impl Trait for Enum::Variant` are forbidden. This dissuades inclinations to implement
abstraction using behaviour-switching on enums (for example, by simulating inheritance-based
subtyping, with the enum type as the parent and each variant as children), rather than using traits
as is natural in Rust.

```rust
enum Sum { A(u32), B, C }

impl Sum::A { // ERROR: variant types may not have specific implementations
    // ...
}
```

```
error[E0XXX]: variant types may not have specific implementations
 --> src/lib.rs:3:6
  |
3 | impl Sum::A {
  |      ^^^^^^
  |      |
  |      `Sum::A` is a variant type
  |      help: you can try using the variant's enum: `Sum`
```

Variant types may be aliased with type aliases:

```rust
enum Sum { A(u32), B, C }

type SumA = Sum::A;
// `SumA` may now be used identically to `Sum::A`.
```

If a value of a variant type is explicitly coerced or cast to the type of its enum using a type
annotation, `as`, or by passing it as an argument or return-value to or from a function, the variant
information is lost (that is, a variant type *is* different to an enum type, even though they behave
similarly).

Note that enum types may not be coerced or cast to variant types. Instead, matching must be
performed to guarantee that the enum type truly is of the expected variant type.

```rust
enum Sum { A(u32), B, C }

let s: Sum = Sum::A;

let a = s as Sum::A; // error
let a: Sum::A = s; // error

if let a @ Sum::A(_) = s {
    // ok, `a` has type `Sum::A`
    println!("a is {}", a.0);
}
```

Variant types interact as expected with the proposed
[generalised type ascription](https://github.com/rust-lang/rfcs/pull/2522) (i.e. the same as type
coercion in `let` or similar).

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
every generic parameter in the enum. Since variant types are generally considered simply as enum
types, this means that the variants need all the type information of their enums, including all
their generic parameters. This explictness has the advantage of preserving variance for variant
types relative to their enum types, as well as permitting zero-cost coercions from variant types to
enum types.

So, in this case, we have the types: `Either<A, B>`, `Either<A, B>::L` and `Either::<A, B>::R`.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

A new variant, `Variant(DefId, VariantDiscr)`, will be added to `TyKind`, whose `DefId` points to
the enclosing enum for the variant and `VariantDiscr` is the discriminant for the variant in
question. In most cases, the handling of `Variant` will simply delegate any behaviour to its `enum`.
However, pattern-matching on the variant allows irrefutable matches on the particular variant. In
effect, `Variant` is only relevant to type checking/inference and the matching logic.

The discriminant of a `Variant` (as observed by [`discriminant_value`](https://doc.rust-lang.org/nightly/std/intrinsics/fn.discriminant_value.html)) is the discriminant
of the variant (i.e. identical to the value observed if the variant is first coerced to the enum
type).

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

Explicitly coercing or casting to the `enum` type forgets the variant information.

```rust
let x: Sum = Sum::A(5); // x: Sum
let Sum::A(y) = x; // error: refutable match

let x = Sum::A(5) as Sum; // x: Sum
let Sum::A(y) = x; // error: refutable match
```

In all cases, the most specific type (i.e. the variant type if possible) is chosen by the type
inference. However, this is entirely backwards-compatible, because `Variant` acts as `Adt` except in
cases that were previously invalid (i.e. pattern-matching, where the extra typing information was
previously unknown).

Note that because a variant type, e.g. `Sum::A`, is not a subtype of the enum type (rather, it can
simply be coerced to the enum type), a type like `Vec<Sum::A>` is not a subtype of `Vec<Sum>`.
(However, this should not pose a problem as it should generally be convenient to coerce `Sum::A` to
`Sum` upon either formation or use.)

Note that we do not make any guarantees of the variant data representation at present, to allow us
flexibility to explore the design space in terms of trade-offs between memory and performance.

# Drawbacks
[drawbacks]: #drawbacks

- The loose distinction between the `enum` type and its variant types could be confusing to those
unfamiliar with variant types. Error messages might specifically mention a variant type, which could
be at odds with expectations. However, since they generally behave identically, this should not
prove to be a significant problem.
- As variant types need to include generic parameter information that is not necessarily included in
their definitions, it will be necessary to include explicit type annotations more often than is
typical. Although this is unfortunate, it is necessary to preserve all the desirable behaviour of
variant types described here: namely complete backwards-compatibility precise type inference and
variance (e.g. allowing `x` in `let x = Sum::A;` to have type `Sum::A` without explicit type
annotations).

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The advantages of this approach are:
- It naturally allows variants to be treated as types, intuitively.
- It doesn't require explicit type annotations to reap the benefits of variant types.
- As variant types and enum types are represented identically, there are no coercion costs.
- It doesn't require type fallback (as was an issue with a
[similar previous proposal](https://github.com/rust-lang/rfcs/pull/1450)).
- It doesn't require value-tracking or complex type system additions such as
[refinement types](https://en.wikipedia.org/wiki/Refinement_type).
- Since complete (enum) type information is necessary for variant types, this should be forwards
compatible with any extensions to enum types (e.g.
[GADTs](https://en.wikipedia.org/wiki/Generalized_algebraic_data_type)).

One obvious alternative is to represent variant types differently to enum types and then coerce them
when used as an enum. This could potentially reduce memory overhead for smaller variants
(additionally no longer requiring the discriminant to be stored) and reduce the issue with providing
irrelevant type parameters. However, it makes coercion more expensive and complex (as a variant
could coerce to various enum types depending on the unspecified generic parameters). It is proposed
here that zero-cost coercions are more important. (In addition, simulating smaller variants is
possible by creating separate mirroring structs for each variant for which this is desired and
converting manually (though this is obviously not ideal), whereas simulating the proposed behaviour
with the alternative is much more difficult, if possible at all.)

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
allow the variants to be treated as types in their own right. There are some languages that have
analogues however: Scala's [`Either` type](https://www.scala-lang.org/api/2.9.3/scala/Either.html)
has `Left` and `Right` subclasses that may be treated as standalone types, for instance. Regardless
of the scarcity of variant types however, we propose that the patterns in Rust make variant types
more appealing than they might be in other programming languages with variant types.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None.

# Future possibilities
[future-possibilities]: #future-possibilities

It would be possible to remove some of the restrictions on enum variant types in the future, such as
permitting `impl`s, supporting variant types that don't contain all (irrelevant) generic parameters
or permitting variant types to be subtypes of enum types. This RFC has been written intentionally
conservatively in this regard.
