- Feature Name: `into_underlying`
- Start Date: 2020-12-27
- RFC PR: [rust-lang/rfcs#3046](https://github.com/rust-lang/rfcs/pull/3046)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Introduce a new trait, `core::convert::IntoUnderlying`, for converting types into their underlying representation. The primary target of this trait is for converting enums with primitive representations to their primitive representation (for which it should be derivable), but it could also make sense for newtypes and other types.

This RFC split off as a milestone of [RFC 3040](https://github.com/rust-lang/rfcs/pull/3040), which may provide additional useful context, but this RFC does not rely on that one.

# Motivation
[motivation]: #motivation

Some types have a natural underlying representation. For these types, it is useful to be able to convert to that underlying representation, in a type-safe way.

Currently, for `enum` types with primitive representations (the only enum type being considered in this RFC), [the `as` operator](https://doc.rust-lang.org/stable/rust-by-example/types/cast.html) is used for this, but the caller must explicitly name the type they're converting to. This is problematic, as it may silently introduce truncation, for example if the representation of the enum changes to a narrower type. It also means a reader can not tell on inspection whether a conversion is exact or truncating.

`as` casts are also problematic because some types, though implemented as a primitive enum, do not intend to be convertable to a primitive - `as` casts expose things like the order that variants were declared as part of the public API, even when this was not intended.

By introducing a trait specifically for converting to a type's natural/underlying type, we provide a type-safe way of converting to exactly the correct type. We also allow an explicit opt in for an enum to declare it _intends_ to expose its representation as part of its public API.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Trait definition (in `core::convert` and re-exported in `std::convert`):

```rust
/// Used to consume a type with a natural underlying representation into that representation.
/// This trait should only be implemented where conversion is trivial; for non-trivial conversions,
/// prefer to implement [`Into`].
pub trait IntoUnderlying {
    /// The underlying type.
    type Underlying;

    /// Performs the conversion.
    fn into_underlying(self) -> Self::Underlying;
}
```

Example usage:

```rust
#[derive(IntoUnderlying)]
#[repr(u8)]
enum Number {
    Zero,
    One,
}

fn main() {
    assert_eq!(Number::Zero.into_underlying(), 0_u8);
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This would be a new trait in `core::convert`, and a new derive macro supported only on enums with a primitive representation. There is scope to allow deriving on other types, e.g. newtypes, in the future.

There is a sample implementation in https://github.com/illicitonion/rust/tree/into-underlying.

# Drawbacks
[drawbacks]: #drawbacks

This adds a new form of conversion to consider, alongside `as`, and `Into`, adding to cognitive complexity when working out what traits to implement and what mechanisms to use to perform conversions. Currently `as` conversions are required to be trivially cheap (but may discard data), and `Into` conversions may be arbitrarily expensive (but are intended to be data-preserving). This trait aims to bridge the gap that exists between the two: providing a known-cheap way of performing conversion.

It also splits conversions such that functions generic over types which are `Into` would not apply to types which implement `IntoUnderlying`. Unfortunately, a blanket `impl <T> Into<T> for IntoUnderlying<Underlying=T>` could probably not be added without specialization, because types may already implement `Into<T>`. That said, because `IntoUnderlying` won't be automatically implemented for any types, perhaps this blanket implementation would be sufficiently coherent.

# Alternatives
[alternatives]: #alternatives

## Use `Into` instead

Instead of a new trait, we could use `Into`. This is worse, because:
1. `Into` makes no contract about being cheap, whereas `as` does. `IntoUnderlying` attempts to preserve that cheapness guarantee, even if it cannot enforce it.
2. Deriving `Into` is non-obvious - if you see it on an `enum`, it's non-obvious what type it's going to generate a conversion to.

## Make `as` more predictable

We could just deprecate or warn on using `as` where truncation may occur, so that non-truncating uses are still supported, without introducing a new trait. Then `as` would be a reliable form of conversion. This is a much larger change to the language, and one we may want to consider. I believe that should probably be done _as well as_ this RFC, and there is still place for a trait to allow this conversion.

## Rely on external crates

There are [several crates](https://github.com/rust-lang/rfcs/issues/2783#issuecomment-679147876) which implement conversions between enums and primitives, mostly using `Into`, but some simply with inherent functions on the enum. We could continue with this status quo, but it is worse because it encourages the less reliable behavior. `as` conversions are risky, but are easy to do; using external crates requires knowing about the risks in the first place, as well as taking on extra dependencies, adding to build times. We should make the obvious, easy, ergonomic thing to do, the most correct thing.

## Do nothing

If we do nothing, `as` conversions will remain the dominant form of conversion. This isn't the end of the world, but will likely lead to bugs around truncation where they could be avoided.

# Prior art
[prior-art]: #prior-art

## Other forms of conversion

There are a number of other ways of performing conversion at the moment.

For enums, there are [several crates](https://github.com/rust-lang/rfcs/issues/2783#issuecomment-679147876) which implement derive (or other) proc-macros for converting enums to their repr. This shows a need for this functionality, but right now a less type-safe conversion is supplied by the standard library (`as` conversion), which means that people are more likely to reach for that, either without realising the lack of type safety, or consciously trading off external dependencies for the lack of type safety.

For conversions, we already have the [`From`](https://doc.rust-lang.org/std/convert/trait.From.html) and [`Into`](https://doc.rust-lang.org/std/convert/trait.Into.html) traits. These are less suitable than a new trait, `IntoUnderlying` for three reasons:
1. It's non-obvious to a reader what deriving `Into` would mean for an enum, and even more so for something like a newtype.
2. They would be strange in a `derive` position because they take a generic type.
3. `Into` makes no claims about the cheapness/triviality of the conversion.

Several wrapper/smart-pointer-like types also implement similar functionality:
* `BufWriter` implements an [`into_inner`](https://doc.rust-lang.org/std/io/struct.BufWriter.html#method.into_inner) function - this should not be implement `IntoUnderlying` because it performs a write of the underlying buffer.
* `Arc` implements [`try_unwrap`](https://doc.rust-lang.org/std/sync/struct.Arc.html#method.try_unwrap) - this could implement `IntoUnderlying`, but probably should not, as it would expose a `Result` rather than simply returning the underlying value, which feels semantically distinct.
* `Option` and `Result` implement an `unwrap` function - this is distinct from `IntoUnderlying` because it may panic, whereas `IntoUnderlying` is intended to be infallible.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

Should we instead just use `Into` for this instead?

Should `IntoUnderlying` be derivable for primitive enums which don't have an explicit `repr`? These [are described](https://doc.rust-lang.org/reference/items/enumerations.html#custom-discriminant-values-for-fieldless-enumerations) as being `isize` values, though are not guaranteed to be represented as such.

Should we block on specialization, so that a blanket `impl <T> Into<T> for IntoUnderlying<Underlying=T>` implementation could be added? Alternatively, are orphan rules sufficiently strong to allow us to add this blanket implementation anyway?

Ideally `IntoUnderlying` would have some guarantees around being trivial. This could be partially obtained by making `IntoUnderlying` a [`const` trait](https://github.com/oli-obk/rfcs/blob/const_generic_const_fn_bounds/text/0000-const-generic-const-fn-bounds.md#const-traits) (from [RFC 2632](https://github.com/rust-lang/rfcs/pull/2632)). As far as I know, there are currently no plans to actaully implement `const` traits, so it would be good to avoid blocking on this, but conversely, `const`-ifying a trait would be a breaking change, so if `const` traits _are_ going to happen, we may want to block on them.

Should we try to make this more generic? For instance, this could be a more generic trait which also applied to smart pointers and other wrappers (e.g. `BufWriter`). I lean towards no, as cheap reliable conversions have a useful place.

# Future possibilities
[future-possibilities]: #future-possibilities

A follow-up may be to deprecate `as` conversions for enums (and possibly for integer types), favouring more type-safe conversions. This is detailed in [RFC 3040](https://github.com/rust-lang/rfcs/pull/3040).

The derive macro could also support `struct` types where the struct has a single field.
