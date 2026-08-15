- Feature Name: `enum_as_to_into`
- Start Date: 2020-12-18
- RFC PR: [rust-lang/rfcs#3040](https://github.com/rust-lang/rfcs/pull/3040)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

To improve correctness and predictibility, deprecate converting primitive-repr enums to arbitrary integer primitives using `as`, and instead require that they be converted using `From`/`Into`. Make it trivial to do so without external crates.

This RFC contains multiple incremental steps to making conversions from enums more predictable and reliable. It proposes that we implement them all, but we could also choose any reasonable subset.

# Motivation
[motivation]: #motivation

## Conversions

Currently, Rust has two main forms of infallible conversion:
* Primitives can be [cast using the `as` keyword](https://doc.rust-lang.org/stable/rust-by-example/types/cast.html). This may silently truncate when going from a larger type to a smaller one, e.g.
  ```rust
  fn main() {
      let more_bits: u16 = u16::MAX;
      let fewer_bits: u8 = more_bits as u8;
      assert_eq!(fewer_bits, 255_u8);
  }
  ```
* Both primitives and non-primitives can be converted using an implementation of the [`From`](https://doc.rust-lang.org/std/convert/trait.From.html) and [`Into`](https://doc.rust-lang.org/std/convert/trait.Into.html) traits. e.g.
  ```rust
  fn main() {
      let fewer_bits: u8 = u8::MAX;

      let more_bits_into: u16 = fewer_bits.into();
      assert_eq!(more_bits_into, 255_u16);

      let more_bits_from: u16 = u16::from(fewer_bits);
      assert_eq!(more_bits_from, 255_u16);
  }
  ```

`as` casts can be assumed to be trivially cheap, whereas `From`/`Into` casts may be arbitrarily expensive.

## Enums

There are several different styles of [enum](https://doc.rust-lang.org/reference/items/enumerations.html) in Rust. This RFC only considers enums which use a [primitive representation](https://doc.rust-lang.org/reference/type-layout.html#primitive-representations). These enums, though represented as primitive number types, are distinct from them.

Enums currently support being cast to any numeric primitive type using `as`, e.g.
```rust
#[repr(u16)]
enum Number {
    Zero,
    One,
}

fn main() {
    let number_u8 = Number::Zero as u8;
    let number_u16 = Number::Zero as u16;
    let number_u32 = Number::Zero as u32;
}
```

As with all cases of integer primtive casts, this may silently truncate.

## Correctness issue

This silent truncation causes potential correctness issues. For example, if an enum changes its repr from a smaller type to a larger one, casts which used to be non-truncating may silently become truncating, with not even a warning raised.

There should be a way to convert an enum to its underlying representation without risk of truncation. Once it is converted to a primitive, it could then be cast to another primitive using `as`. But an enum itself should not be considered as a primitive, even if it happens to be backed by one, in much the same way that a newtype wrapping a primitive is not considered a primitive.

The natural candidate for this is an `Into` implementation, as is used for other types (and newtypes). There are [several crates](https://github.com/rust-lang/rfcs/issues/2783#issuecomment-679147876) which provide these implementations, but these require being aware of the potential problem, and taking non-trivial action to side-step it (finding and taking on a new dependency, and potentially significantly increasing compile times). Instead, we should make it trivial to derive an `Into` implementation for a primitive-repr enum in `core`/`std`, and phase out support for casting enums to primitives with `as`. The obvious, and easiest, option should be the correct option.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

As this RFC proposes using existing, well-known features, little teaching is required. We will introduce the feature to users in two ways: Updates to documentation, and lints which may be upgraded to compile errors.

## Documentation updates

We will update [the enum documentation](https://doc.rust-lang.org/reference/items/enumerations.html#custom-discriminant-values-for-fieldless-enumerations). Initially, it will be updated the text along the lines of:

```diff
--- a/src/items/enumerations.md
+++ b/src/items/enumerations.md
@@ -66,8 +66,18 @@ opaque reference to this discriminant can be obtained with the
 If there is no data attached to *any* of the variants of an enumeration,
 then the discriminant can be directly chosen and accessed.

-These enumerations can be cast to integer types with the `as` operator by a
-[numeric cast]. The enumeration can optionally specify which integer each
+If you wish to convert these enumerations to integer types, it is recommended
+that you implement
+[the `Into` trait](https://doc.rust-lang.org/std/convert/trait.Into.html) for
+them. This can be done easily with `#[derive(Into)]`, supplied by the standard
+library.
+
+For legacy reasons, these enumerations can also be cast to integer types with
+the `as` operator by a [numeric cast], but this is not recommended as it can
+have unexpectedly truncating results, and may cease to be supported in a
+future version of Rust.
+
+The enumeration can optionally specify which integer each
 discriminant gets by following the variant name with `=` followed by a [constant
 expression]. If the first variant in the declaration is unspecified, then it is
 set to zero. For every other unspecified discriminant, it is set to one higher
```

In the future, if support is removed for `as` casts, the text may be further updated.

## Lints

Three new lints will be introduced to the language. They will start at `warn` level, and could independently be promoted to `error` level, or full compile errors:

### Likely incorrect casts

For casts where the value being cast is an enum, and the type being cast to is smaller than the enum's repr, a warning will be issued:

```rust
#[repr(u16)]
enum Number {
    Zero,
    One,
}

fn main() {
    let bad = Number::Zero as u8;
}
```

```
warning: truncating cast
 --> src/main.rs:8:9
  |
8 |     let bad = Number::Zero as u8;
  |         ^^^ This cast may truncate the value of `bad`, as it is represented by a `u16` which may not fit in a `u8`.
  |
  = note: `#[warn(enum_truncating_cast)]` on by default
```

If `TryInto<u8>` or `TryFrom` is implemented for the enum, the lint may also suggest calling one of those traits:
```
Consider using `u8::try_from(Number::Zero)`
```

If `Into<u16>` or `From` is implemented for the enum, the lint may also suggest calling it first:
```
Consider using `u16::from(Number::Zero) as u8` to make clear that this possible truncation is intended.
```

If none of the above traits are implemented for the enum, the lint may suggest implementing them.

### Safe casts where Into is implemented

For casts where the value being cast is an enum, the type being cast to is the enum's repr, and `Into`/`From` _is_ implemented between the types, a rustfix-able warning will be issued:

```rust
#[derive(Into)]
#[repr(u16)]
enum Number {
    Zero,
    One,
}

fn main() {
    let ok = Number::Zero as u16;
}
```

```
warning: cast used where From::from is preferred
 --> src/main.rs:9:9
  |
9 |     let ok = Number::Zero as u16;
  |         ^^ Prefer to use `From` rather than `as` when casting enums. `as` may silently change in behavior if the enum's repr changes, but `From` provides compile-time guarantees.
  |
  = note: `#[warn(enum_prefer_from_over_as)]` on by default

Instead use `let ok = u16::from(Number::Zero);`
```

In the case that the type being cast to is larger than the enum's repr, a nested call could be suggested:

```rust
let ok = u64::from(u16::from(Number::Zero));
```

### Safe casts where Into is not implemented

Like with the previous lint, but with a suggestion to implement `Into` (noting that it can be derived), and without the rustfix suggestion (because it would require updating both the definition site and call site).

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

We would implement this in several pieces:

## Derive macro for From/Into

`Into` would become a derivable macro, only for primitive-repr'd enums, in `core`.

This would be slightly unusual for a derive macro, as it would actualy derive `impl From<EnumType> for repr_type`, which in turn would provide `impl Into<repr_type> for EnumType`, but `#[derive(Into)]` reads much better on the definition site. It is also unusual for a derive macro in that it has an implicit type parameter, but this seems reasonable as primitive enums have natural types to convert to.

`Into` would not be derivable for any other type.

A sample implementation can be found [here](https://github.com/illicitonion/rust/tree/enum-into-derive-macro).

## Lints

The lints need only type information in very clear situations only around the `as` operator, and should not require noteworthy changes to the compiler. They could potentially also live in Clippy, were that preferred.

Eventually, we would remove support for `as` casts for enums, by moving support from the `as` implementation into the derive macro's generated code.

## Optional: Derive macro for TryFrom/Into

Though not directly related to this proposal, it would make some sense to allow a derive macro for a `TryFrom` implementation to go from a primitive to an enum, which would use [`std::num::TryFromIntError`](https://doc.rust-lang.org/std/num/struct.TryFromIntError.html) as its associated error type.

# Drawbacks
[drawbacks]: #drawbacks

This causes churn in the language. Existing code will need to be updated, and dependees will need to rely on `Into` impls being added to their dependencies before they can migrate.

This also expands `core` and `std`, and introduces a slightly weird corner of `Derive`s whereby they have implicit associated types.

`as` casts also offer some properties that some users may find useful, which may lean towards leaving these lints as warnings rather than promoting them to errors by default. These are:
1. `From` and `Into` are not currently usable in const contexts, whereas `as` is. [RFC 2632](https://github.com/rust-lang/rfcs/pull/2632) aims to solve this.
2. `as` is guaranteed to be a cheap operation, whereas `Into` may be arbitrarily expensive, though it is cheap in this instance.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Alternatives considered

### Leaving the derive macros in external crates

Leaving the derive macros in external crates is worse because it makes the more correct thing (type-checked non-truncation of conversion) higher friction than the convenient thing (builtin `as` casts). There is a place for external crates here for doing non-standard things, such as having default enum variants in a `From` implementation (just as the [derivative](https://crates.io/crates/derivative) crate implements custom derives of many traits), but the core infallible conversion feels like it should be within easy reach.

### Not removing `as` support for enums

We could lint/warn against `as` usage, without removing support for it. This would force less churn in the language, but leaves a foot-gun in play.

### Only warn/error or potentially truncating casts

We could only warn/error, or even only remove language support, for casts from larger to smaller types. This leaves multiple ways to do the same thing, which leaves an increased cognitive space, but could avoid a feeling of churn.

### Provide direct access to the correctly-typed value via a new derivable trait

We could add a new trait, derivable on enums, which allows access to the representation value in its correct type. This would remove the special-casing of `From`/`Into` as being derivable traits with implicit target types, and allow the caller to not need to name the underlying type, at the cost of introducing a new trait, adding cognitive load and inconsistency with other forms of conversion.

This alternative only considers whether to allow deriving `From`/`Into`, and is orthogonal to the decision of how strongly to discourage/deprecate/remove the ability to perform `as` casts.

### Do nothing

If we do nothing, we leave a footgun for our users. It's probably not the worst footgun, and it's possible to work around.

# Prior art
[prior-art]: #prior-art

- This was proposed less formally in [RFC 2596](https://github.com/rust-lang/rfcs/issues/2596).

- There used to be `FromPrimitive` and `ToPrimitive` derive macros in `std` which could be applied to enums, but [these were removed](https://github.com/rust-lang/rust/commit/eeb94886adccb3f13003f92f117115d17846ce1f) as part of a sweeping reduction in num-related functionality. I believe that this RFC represents a limited and targeted enough subset of this to be worthwhile.

- [Example issue](https://github.com/rust-lang/rust/issues/45884) reporting this behaviour as surprising, and less well warned for than the integer equivalent. [And another](https://github.com/rust-lang/rust/issues/74588).

- Enums used to be castable to floats, [and this was removed](https://github.com/rust-lang/rust/pull/14874) with the explicit suggestion to cast via an integer if needed.

- [RFC 2308](https://github.com/rust-lang/rfcs/pull/2308) has some relevant discussion around the purpose of `as`, distinction between `as` and `Into`, and some confusion caused by the handling of `as` around enums. It was closed, partially because "The `as` operator is probably not something that we want to encourage people to use.", which this RFC helps to further.

- `Drop` has unclear semantics when enums are cast with `as` (see [motivational issue](https://github.com/rust-lang/rust/issues/35941) and [tracking issue](https://github.com/rust-lang/rust/issues/73333)) - this is an example of `as` being forbidden because of problematic semantics. This RFC proposes taking this a step further.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Should `as` casts be removed for primitive enums, or simply discouraged?
- Should `Into` be derivable for primitive enums which don't have an explicit `repr`? These [are described](https://doc.rust-lang.org/reference/items/enumerations.html#custom-discriminant-values-for-fieldless-enumerations) as being `isize` values, though are not guaranteed to be represented as such.

# Future possibilities
[future-possibilities]: #future-possibilities

- We may want to also remove `as` casts for primitives where they are non-truncating (e.g. casting a `u8` to a `u16`, preferring a call to `u16::from`) to make clear that `as` is always a potentially truncating operation of which the reader should be wary.
