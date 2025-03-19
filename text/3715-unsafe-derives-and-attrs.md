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
#[proc_macro_derive(DangerousDeriveMacro, unsafe)]
pub fn derive_dangerous_derive_macro(_item: TokenStream) -> TokenStream {
    TokenStream::new()
}
```

Invoking this derive macro requires writing
`#[derive(unsafe(DangerousDeriveMacro))]`. Invoking an unsafe derive macro
without the unsafe derive syntax will produce a compiler error. Using the
unsafe derive syntax without an unsafe derive macro will trigger an "unused
unsafe" lint.

A `proc_macro_derive` attribute can include both `attributes` for helper
attributes and `unsafe` to declare the derive unsafe, in any order.

When writing a `SAFETY` comment for each `unsafe`, you can place the `SAFETY`
comment either prior to the derive (for a single unsafe derive) or prior to the
specific `unsafe(DangerousDeriveMacro)` in a list of derives:

```rust
// SAFETY: ...
#[derive(unsafe(DangerousDeriveMacro))]
struct SomeStruct { ... }

#[derive(
    // SAFETY: ...
    unsafe(DangerousDeriveMacro),
    // SAFETY: ...
    unsafe(AnotherDangerousDeriveMacro),
)]
struct AnotherStruct { ... }
```

(Note that current rustfmt will place every derive on a line of its own if any
have a comment. That could be changed in a future style edition, but this RFC
is not making or advocating any style proposals.)

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

When writing a `SAFETY` comment for each `unsafe`, you can place the `SAFETY`
comment immediately prior to the attribute:

```rust
// SAFETY: ...
#[unsafe(dangerous)]
```

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This RFC proposes the synax `#[derive(unsafe(DangerousDeriveMacro))]`. We
could, instead, put the `unsafe` on the outside:
`#[unsafe(derive(DangerousDeriveMacro))]`.

Some rationale for putting it on the inside:
- This encourages minimizing the scope of the `unsafe`, isolating it to a
  single derive macro.
- This allows writing all derive macros to invoke within a single
  `#[derive(...)]`, if desired. Putting the `unsafe` on the outside requires
  separate `derive`s for safe and unsafe derives, and potentially multiple in
  the (unusual) case where the derives care about ordering.
- This makes it easy to attach `SAFETY` comments to each individual derive
  macro.
- One way to think of `derive(Macro)` is that `derive(..)` enters a context in
  which one or more derive macros can be invoked, and naming `Macro` is how we
  actually invoke the derive macro. When invoking unsafe derive macros, we have
  to wrap those with `unsafe(..)` as in `derive(unsafe(DangerousDeriveMacro))`.

We could, *if* we used the `unsafe(derive(...))` syntax, additionally restrict
such derives to only contain a single trait, forcing the developer to only
invoke a single derive macro per `unsafe(derive(...))`. However, this syntax
naturally leads people to assume this would work, only to encounter an error
when they do the natural thing that seems like it should work.

We could use a different syntax for invoking unsafe derives, such as
`derive(unsafe DangerousDeriveMacro)`. However, that would be inconsistent with
unsafe attributes (which use parentheses), *and* it has the potential to look
like a modifier to `DangerousDeriveMacro` (e.g. an unsafe version of
`DangerousDeriveMacro`), particularly in the common case where
`DangerousDeriveMacro` has the same name as a trait.

We could use a similar syntax for declaring unsafe derives as invoking them:
`proc_macro_derive(unsafe(DangerousDeriveMacro))`. However, because of the
precedent of `unsafe(attribute)`, this can be interpreted as being an unsafe
operation to *define* rather than an unsafe operation to *invoke*.

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
