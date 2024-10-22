- Feature Name: `unsafe_derives_and_attrs`
- Start Date: 2024-10-22
- RFC PR: [rust-lang/rfcs#3715](https://github.com/rust-lang/rfcs/pull/3715)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Allow declaring proc macro attributes and derive macros as unsafe, and
requiring `unsafe` to invoke them.

# Motivation
[motivation]: #motivation

Some traits place requirements on implementations that the Rust compiler cannot
verify. Those traits can mark themselves as unsafe, requiring `unsafe impl`
syntax to implement. However, trait `derive` macros cannot currently require
`unsafe`. This RFC defines a syntax for declaring and using unsafe `derive`
macros.

This RFC also defines a syntax for declaring proc macro attributes as unsafe.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## Derives

When declaring a proc macro `derive`, you can add the `unsafe` parameter to the
`proc_macro_derive` attribute to indicate that the derive requires `unsafe`:

```rust
#[proc_macro_derive(DangerousTrait, unsafe)]
pub fn derive_helper_attr(_item: TokenStream) -> TokenStream {
    TokenStream::new()
}
```

Invoking this derive requires writing either
`#[unsafe(derive(DangerousTrait))]` or `#[derive(unsafe(DangerousTrait))]`.
(The latter syntax allows isolating the `unsafe` to a single derive within a
list of derives.) Invoking an unsafe derive without the unsafe derive syntax
will produce a compiler error. Using the unsafe derive syntax without an unsafe
derive will trigger an "unused unsafe" lint.

A `proc_macro_derive` attribute can include both `attributes` for helper
attributes and `unsafe` to declare the derive unsafe, in any order.

## Attributes

When declaring a proc macro attribute, you can add the `unsafe` parameter to
the `proc_macro_attribute` attribute to indicate that the attribute requires
`unsafe`:

```rust
#[proc_macro_attribute(unsafe)]
pub fn dangerous(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
```

Invoking an unsafe attribute requires the unsafe attribute syntax:
`#[unsafe(dangerous)]`.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Should we support the `#[unsafe(derive(DangerousTrait))]` syntax, or only
`#[derive(unsafe(DangerousTrait))]`? The former elevates the `unsafe` to be
more visible, and allows deriving several traits using one `unsafe`. The latter
isolates the `unsafe` to a specific trait. This RFC proposes supporting both,
but we could choose to only support the latter instead.

# Prior art
[prior-art]: #prior-art

RFC 3325 defined unsafe attributes. This RFC provides a natural extension of
that mechanism to derives.

# Future possibilities
[future-possibilities]: #future-possibilities

When we add support for `macro_rules!`-based attributes and derives, we should
provide a means for such attributes and derives to declare themselves unsafe as
well.

We could provide a syntax to declare specific helper attributes of a derive as
unsafe, without declaring the entire derive unsafe.
