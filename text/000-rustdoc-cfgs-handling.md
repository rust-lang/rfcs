Rustdoc: stabilization of the `doc_cfg` features

- Features Name: `doc_cfg`, `doc_auto_cfg`, `doc_cfg_hide`
- Start Date: 2022-12-07
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#43781](https://github.com/rust-lang/rust/issues/43781)

# Summary
[summary]: #summary

This RFC aims at providing to the rustdoc users the possibility to add visual markers in the rendered documentation to know under which conditions an item is available (currently possible through the following unstable features: `doc_cfg`, `doc_auto_cfg` and `doc_cfg_hide`).

# Motivation
[motivation]: #motivation

The goal of this RFC is to stabilize the possibility to add visual markers in the rendered documentation to know under which conditions an item is available.

Providing this information to users will prevent a common issue: "why can I see this item in the documentation and yet can't use it in my code?".
The end goal being to retrieve this information automatically so that the documentation maintenance cost won't increase.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This RFC proposes to add the following attributes:

 * `#[doc(cfg(...))]`
 * `#[doc(auto_cfg)]`/`#[doc(no_auto_cfg)]`
 * `#[doc(cfg_hide(...))]`

### `#[doc(cfg(...))]`

This attribute allows to manually add under which conditions an item is available to the documentation readers. Example:

```rust
// the "real" cfg condition
#[cfg(feature = "futures-io")]
// the `doc(cfg())` so it's displayed to the readers
#[doc(cfg(feature = "futures-io"))]
pub mod futures {}
```

It will display in the documentation for this module:

![display of doc(cfg) feature](https://user-images.githubusercontent.com/81079/89731116-d7b7ce00-da44-11ea-87c6-022d192d6eca.png)

This attribute works on modules and on items but cannot be used at the crate root level.

### `#[doc(auto_cfg)]`/`#[doc(no_auto_cfg)]`

By default, rustdoc will automatically retrieve the `cfg` information on items, no need for users to use `#[doc(cfg(...))]`. So if we take back the previous example:

```rust
#[cfg(feature = "futures-io")]
pub mod futures {}
```

No need to "duplicate" the `cfg` into a `doc(cfg())` to make it displayed to the documentation readers.

So by default, `#[doc(auto_cfg)]` is enabled. However, for some reasons, you might want to disable this behaviour on a given item/module or on the whole crate. To do so, you can use `#[doc(no_auto_cfg)]`.

Both `#[doc(auto_cfg)]` and `#[doc(no_auto_cfg)]` attributes impact all there descendants. You can then enable/disable them by using the opposite attribute on a given item.

As mentioned, both attributes can be used on modules, items and crate root level.

### `#[doc(cfg_hide(...))]`

This attribute is used to prevent some `cfg` to be generated in the visual markers. So in the previous example:

```rust
#[cfg(any(doc, feature = "futures-io"))]
pub mod futures {}
```

It currently displays both `doc` and `feature = "futures-io"` into the documentation, which is not great. To prevent the `doc` cfg to ever be displayed, you can use this attribute at the crate root level:

```rust
#![doc(cfg_hide(doc))]
```

Or directly on the item as it covers any of the item's children:

```rust
#[doc(cfg_hide(doc))]
#[cfg(any(doc, feature = "futures-io"))]
pub mod futures {}
```

Then, the `doc` cfg will never be displayed into the documentation.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

TO BE DONE.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

TO BE DONE.

# Prior Art
[prior-art]: #prior-art

TO BE DONE.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

### Should we add an attribute to revert `#[doc(cfg_hide(...))`?

It would make sense to think that in some locations, someone might want to not hide a given `cfg`.
