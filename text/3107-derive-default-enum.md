- Feature Name: `derive_default_enum`
- Start Date: 2021-04-07
- RFC PR: [rust-lang/rfcs#3107](https://github.com/rust-lang/rfcs/pull/3107)
- Rust Issue: [rust-lang/rust#87517](https://github.com/rust-lang/rust/issues/87517)

# Summary
[summary]: #summary

An attribute `#[default]`, usable on `enum` unit variants, is introduced thereby allowing some
enums to work with `#[derive(Default)]`.

```rust
#[derive(Default)]
enum Padding {
    Space,
    Zero,
    #[default]
    None,
}

assert_eq!(Padding::default(), Padding::None);
```

The `#[default]` and `#[non_exhaustive]` attributes may not be used on the same variant.

# Motivation
[motivation]: #motivation

## `#[derive(Default)]` in more cases

Currently, `#[derive(Default)]` is not usable on `enum`s. To partially rectify this situation, a
`#[default]` attribute is introduced that can be attached to unit variants. This allows you to use
`#[derive(Default)]` on enums wherefore you can now write:

```rust
#[derive(Default)]
enum Padding {
    Space,
    Zero,
    #[default]
    None,
}
```

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The ability to add default values to fields of `enum` variants does not mean that you can suddenly
`#[derive(Default)]` on the enum. A Rust compiler will still have no idea which variant you intended
as the default. This RFC adds the ability to mark one unit variant with `#[default]`:

```rust
#[derive(Default)]
enum Ingredient {
    Tomato,
    Onion,
    #[default]
    Lettuce,
}
```

Now the compiler knows that `Ingredient::Lettuce` should be considered the default and will
accordingly generate an appropriate implementation:

```rust
impl Default for Ingredient {
    fn default() -> Self {
        Ingredient::Lettuce
    }
}
```

Note that after any `cfg`-stripping has occurred, it is an error to have `#[default]` specified on
zero or multiple variants.

As fields may be added to `#[non_exhaustive]` variants that necessitate additional bounds, it is not
permitted to place `#[default]` and `#[non_exhaustive]` on the same variant.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## `#[default]` on `enum`s

An attribute `#[default]` is provided the compiler and may be legally placed solely on one
exhaustive `enum` unit variants. The attribute has no semantics on its own. Placing the attribute on
anything else will result in a compilation error. Furthermore, if the attribute occurs on zero or
multiple variants of the same `enum` data-type after `cfg`-stripping and macro expansion is done,
this will also result in a compilation error.

## `#[derive(Default)]`

Placing `#[derive(Default)]` on an `enum` named `$e` is permissible if and only if that enum has
some variant `$v` with `#[default]` on it. In that event, the compiler shall generate the following:
implementation of `Default` where the function `default` is defined as:

```rust
impl ::core::default::Default for $e {
    fn default() -> Self {
        $e::$v
    }
}
```

### Generated bounds

As exhaustive unit variants have no inner types, no bounds shall be generated on the derived
implementation. For example,

```rust
#[derive(Default)]
enum Option<T> {
    #[default]
    None,
    Some(T),
}
```

would generate:

```rust
impl<T> Default for Option<T> {
    fn default() -> Self {
        Option::None
    }
}
```

## Interaction with `#[non_exhaustive]`

The Rust compiler shall not permit `#[default]` and `#[non_exhaustive]` to be present on the same
variant. Non-default variants may be `#[non_exhaustive]`, as can the `enum` itself.

# Drawbacks
[drawbacks]: #drawbacks

The usual drawback of increasing the complexity of the language applies. However, the degree to
which complexity is increased is not substantial. One notable change is the addition of an attribute
for a built-in `#[derive]`, which has no precedent.

# Rationale
[rationale]: #rationale

The inability to derive `Default` on `enum`s has been noted on a number of occasions, with a common
suggestion being to add a `#[default]` attribute (or similar) as this RFC proposes.

- [IRLO] [Request: derive enum's default][rationale-1]
- [IRLO] [Deriving `Error` (comment)][rationale-2]
- [URLO] [Crate for macro for default enum variant][rationale-3]
- [URLO] [`#[derive(Default)]` for enum, [not] only struct][rationale-4]

[rationale-1]: https://internals.rust-lang.org/t/request-derive-enums-default/10576?u=jhpratt
[rationale-2]: https://internals.rust-lang.org/t/deriving-error/11894/10?u=jhpratt
[rationale-3]: https://users.rust-lang.org/t/crate-for-macro-for-default-enum-variant/44032?u=jhpratt
[rationale-4]: https://users.rust-lang.org/t/derive-default-for-enum-non-only-struct/44046?u=jhpratt

In the interest of forwards compatibility, this RFC is limited to only exhaustive unit variants.
Were this not the case, adding a field to a `#[non_exhaustive]` variant could lead to more stringent
bounds being generated, which is a breaking change. For example,

A definition of

```rust
#[derive(Default)]
enum Foo<T> {
    #[default]
    #[non_exhaustive]
    Alpha,
    Beta(T),
}
```

would not have any required bounds on the generated code. If this were changed to

```rust
#[derive(Default)]
enum Foo<T> {
    #[default]
    #[non_exhaustive]
    Alpha(T),
    Beta(T),
}
```

then any code where `T: !Default` would now fail to compile, on the assumption that the generated
code for the latter has the `T: Default` bound (nb: not part of this RFC).

# Alternatives
[alternatives]: #alternatives

One alternative is to permit the user to declare the default variant in the derive itself, such as
`#[derive(Default(VariantName))]`. This has the disadvantage that the variant name is present in
multiple locations in the declaration, increasing the likelihood of a typo (and thus an error).

Another alternative is assigning the first variant to be default when `#[derive(Default)]` is
present. This may prevent a `#[derive(PartialOrd)]` on some `enum`s where order is important (unless
the user were to explicitly assign the discriminant).

# Prior art
[prior-art]: #prior-art

## Procedural macros

There are a number of crates which to varying degrees afford macros for default field values and
associated facilities.

### `#[derive(Derivative)]`

[`derivative`]: https://crates.io/crates/derivative

The crate [`derivative`] provides the `#[derivative(Default)]` attribute. With it, you may write:

```rust
#[derive(Derivative)]
#[derivative(Default)]
enum Foo {
    #[derivative(Default)]
    Bar,
    Baz,
}
```

Contrast this with the equivalent in the style of this RFC:

```rust
#[derive(Default)]
enum Foo {
    #[default]
    Bar,
    Baz,
}
```

Like in this RFC, `derivative` allows you to derive `Default` for `enum`s. The syntax used in the
macro is `#[derivative(Default)]` whereas the RFC provides the more ergonomic and direct notation
`#[default]` in this RFC.

### `#[derive(SmartDefault)]`

[`smart-default`]: https://crates.io/crates/smart-default

The [`smart-default`] provides `#[derive(SmartDefault)]` custom derive macro. It functions similarly
to `derivative` but is specialized for the `Default` trait. With it, you can write:

```rust
#[derive(SmartDefault)]
enum Foo {
    #[default]
    Bar,
    Baz,
}
```

- The same syntax `#[default]` is used both by `smart-default` and by this RFC. While it may seem
  that this RFC was inspired by `smart-default`, this is not the case. Rather, this notation has
  been independently thought of on multiple occasions. That suggests that the notation is intuitive
  and a solid design choice.

- There is no trait `SmartDefault` even though it is being derived. This works because
  `#[proc_macro_derive(SmartDefault)]` is in fact not tied to any trait. That `#[derive(Serialize)]`
  refers to the same trait as the name of the macro is from the perspective of the language's static
  semantics entirely coincidental.

  However, for users who aren't aware of this, it may seem strange that `SmartDefault` should derive
  for the `Default` trait.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- None so far.

# Future possibilities
[future-possibilities]: #future-possibilities

## Non-unit variants

One significant future possibility is to have `#[default]` permitted on non-unit variants. This was
originally proposed as part of this RFC but has been postponed due to disagreement over what the
generated bounds should be. This is largely due to the fact that [`#[derive(Default)]` on `struct`s
may generate incorrect bounds](https://github.com/rust-lang/rust/issues/26925).

## Overriding default fields

The `#[default]` attribute could be extended to override otherwise derived default values, such as

```rust
#[derive(Default)]
struct Foo {
    alpha: u8,
    #[default = 1]
    beta: u8,
}
```

which would result in

```rust
impl Default for Foo {
    fn default() -> Self {
        Foo {
            alpha: Default::default(),
            beta: 1,
        }
    }
}
```

being generated.

Alternatively, dedicated syntax could be provided [as proposed by @Centril][centril-rfc]:

[centril-rfc]: https://github.com/Centril/rfcs/pull/19

```rust
#[derive(Default)]
struct Foo {
    alpha: u8,
    beta: u8 = 1,
}
```

If consensus can be reached on desired bounds, there should be no technical restrictions on
permitting the `#[default]` attribute on a `#[non_exhaustive]` variant.

## Clearer documentation and more local reasoning

Providing good defaults when such exist is part of any good design that makes a physical tool, UI
design, or even data-type more ergonomic and easily usable. However, that does not mean that the
defaults provided can just be ignored and that they need not be understood. This is especially the
case when you are moving away from said defaults and need to understand what they were. Furthermore,
it is not too uncommon to see authors writing in the documentation of a data-type that a certain
value is the default.

All in all, the defaults of a data-type are therefore important properties. By encoding the defaults
right where the data-type is defined gains can be made in terms of readability particularly with
regard to the ease of skimming through code. In particular, it is easier to see what the default
variant is if you can directly look at the `rustdoc` page and read the previous snippet, which would
let you see the default variant without having to open up the code of the `Default` implementation.

## `Error` trait and more

As this is the first derive macro that includes an attribute, this may open the flood gates with
regard to permitting additional macros with attributes. Crates such as `thiserror` could be, in some
form or another, upstreamed to the standard library as `#[derive(Error)]`, `#[derive(Display)]` or
more.
