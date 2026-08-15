- Feature Name: `derive-default-enum-with-fields`
- Start Date: 2024-08-25
- RFC PR: [rust-lang/rfcs#3683](https://github.com/rust-lang/rfcs/pull/3683)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Allow `#[derive(Default)]` on `enum` variants with data.

```rust
#[derive(Default)]
enum Foo {
    #[default]
    Bar {
        x: Option<i32>,
        y: Option<i32>,
    },
    Baz,
}
```

Previously, only unit enum variants were allowed to derive `Default`, by marking
them with `#[default]`. This feature extens this support to tuple and struct
enum variants with fields when they all implement `Default`. By extension this
also means that tuple and struct enum variants with no fields are also suitable
to be marked with `#[default]`.

# Motivation
[motivation]: #motivation

Currently, `#[derive(Default)]` is not usable on `enum` variants with data. To
rectify this situation, we expand the existing `#[default]` attribute
implementation to support tuple and struct variants.

This allows you to use `#[derive(Default)]` on enums wherefore you can now write:

```rust
#[derive(Default)]
enum Padding {
    #[default]
    Space {
        n: i32,
    },
    None,
}
```

This feature allows for more cases where `Default` can be derived, instead of
explicitly implemented. This reduces the verbosity of Rust codebases.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

In the same way that `struct`s can be annotated with `#[derive(Default)]`:

```rust
#[derive(Default)]
struct Bar {
    x: Option<i32>,
    y: Option<i32>,
}
```

which expands to:

```rust
impl Default for Bar {
    fn default() -> Bar {
        Bar {
            x: Default::default(),
            y: Default::default(),
        }
    }
}
```

The same annotation on an `enum` with a variant annotated with `#[default]`:

```rust
#[derive(Default)]
enum Foo {
    #[default]
    Bar {
        x: Option<i32>,
        y: Option<i32>,
    },
    Baz,
}
```

expands to:

```rust
impl Default for Foo {
    fn default() -> Foo {
        Foo::Bar {
            x: Default::default(),
            y: Default::default(),
        }
    }
}
```

Because the expanded code calls `Default::default()`, if the fields do not
implement `Default` the compiler will emit an appropriate error pointing at the
field that doesn't meet its requirement.

```
error[E0277]: the trait bound `S: Default` is not satisfied
 --> src/main.rs:4:5
  |
2 | #[derive(Default)]
  |          ------- in this derive macro expansion
3 | enum Foo {
3 |     Bar {
4 |         x: S,
  |         ^^^^ the trait `Default` is not implemented for `S`
  |
  = note: this error originates in the derive macro `Default` (in Nightly builds, run with -Z macro-backtrace for more info)
help: consider annotating `S` with `#[derive(Default)]`
  |
1 + #[derive(Default)]
2 | struct S;
  |
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

[`default_enum_substructure`]: https://github.com/rust-lang/rust/blob/6bb4656ee2ad88425917e3d4ad7ec11a033f181c/compiler/rustc_builtin_macros/src/deriving/default.rs#L96C4-L96C29

[`extract_default_variant`]: https://github.com/rust-lang/rust/blob/6bb4656ee2ad88425917e3d4ad7ec11a033f181c/compiler/rustc_builtin_macros/src/deriving/default.rs#L154C4-L154C27

[`default_struct_substructure`]: https://github.com/rust-lang/rust/blob/6bb4656ee2ad88425917e3d4ad7ec11a033f181c/compiler/rustc_builtin_macros/src/deriving/default.rs#L63C4-L63C31

In `rustc_builtin_macros/src/deriving/default.rs`, we change
[`extract_default_variant`] to not filter *only* on `VariantData::Unit`, and
[`default_enum_substructure`] to expand the `impl` in a similar way to
[`default_struct_substructure`].

[RFC-3107]: https://rust-lang.github.io/rfcs/3107-derive-default-enum.html.

This expands on [RFC-3107]. No other changes are needed.

# Drawbacks
[drawbacks]: #drawbacks

[perfect derives]: https://smallcultfollowing.com/babysteps/blog/2022/04/12/implied-bounds-and-perfect-derive/

The usual drawback of increasing the complexity of the implementation applies.
However, the degree to which complexity is increased is not substantial. If
anything, the complexity of the concepts needed to be understood is reduced, as
there are fewer special cases users need to keep in mind when using
`#[derive(Default)]`, as well as allow us to remove `impl Default`s from the
standard library.

[The same](https://github.com/rust-lang/rust/issues/26925) issue highlighted on
[RFC-3107] of current `#[derive(Default)]` on `struct`s producing `impl`s with
incorrect bounds (non-[perfect derives]) applies to this proposal as well.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

[`derivative`]: https://crates.io/crates/derivative

[`smart-default`]: https://crates.io/crates/smart-default

As shown by the existence of [`derivative`] and [`smart-default`], there is a
desire to fill this perceived gap in flexibility that the built-in
`#[derive(Default)]` support has. We can do nothing and let the ecosystem sort
this gap out.


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
    Bar {
        value: Option<i32>,
    },
    Baz,
}
```

Contrast this with the equivalent in the style of this RFC:

```rust
#[derive(Default)]
enum Foo {
    #[default]
    Bar {
        value: Option<i32>,
    },
    Baz,
}
```

Like in this RFC, `derivative` allows you to derive `Default` for `enum`s. The
syntax used in the macro is `#[derivative(Default)]` whereas the RFC provides
uses the already existing `#[default]` annotation.

### `#[derive(SmartDefault)]`

[`smart-default`]: https://crates.io/crates/smart-default

The [`smart-default`] provides `#[derive(SmartDefault)]` custom derive macro. It functions similarly
to `derivative` but is specialized for the `Default` trait. With it, you can write:

```rust
#[derive(SmartDefault)]
enum Foo {
    #[default]
    Bar {
        value: Option<i32>,
    },
    Baz,
}
```

- There is no trait `SmartDefault` even though it is being derived. This works because
  `#[proc_macro_derive(SmartDefault)]` is in fact not tied to any trait. That `#[derive(Serialize)]`
  refers to the same trait as the name of the macro is from the perspective of the language's static
  semantics entirely coincidental.

  However, for users who aren't aware of this, it may seem strange that `SmartDefault` should derive
  for the `Default` trait.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Should we wait until [perfect derives] are addressed first?
- Should `#[default]` be allowed on tuple and struct enum variants with no fields?

# Future possibilities
[future-possibilities]: #future-possibilities

## Overriding default values

[RFC-3681]: https://github.com/rust-lang/rfcs/pull/3681

[RFC-3681] already proposes supporting the definition of struct and struct enum
variant field default values, that can be used by `#[derive(Default)]` to
override the use of `Default::default()`. These two RFCs interact nicely with
each other.

```rust
#[derive(Default)]
enum Foo {
    #[default]
    Bar {
        value: i32 = 42,
    },
    Baz,
}
```
