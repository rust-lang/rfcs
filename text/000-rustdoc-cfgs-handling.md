Rustdoc: stabilization of the `doc(cfg*)` attributes

- Features Name: `doc_cfg`
- Start Date: 2022-12-07
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#43781](https://github.com/rust-lang/rust/issues/43781)


# Summary
[summary]: #summary

This RFC aims at providing rustdoc users the possibility to add visual markers to the rendered documentation to know under which conditions an item is available (currently possible through the following unstable features: `doc_cfg`, `doc_auto_cfg` and `doc_cfg_hide`).

It does not aim to allow having a same item with different `cfg`s to appear more than once in the generated documentation.

It does not aim to document items which are *inactive* under the current configuration (i.e., “`cfg`'ed out”).

# Motivation
[motivation]: #motivation

The goal of this RFC is to stabilize the possibility to add visual markers to the rendered documentation to know under which conditions an item is available.

Providing this information to users will solve a common issue: “Why can I see this item in the documentation and yet can't use it in my code?”.
The end goal being to provide this information automatically so that the documentation maintenance cost won't increase.


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This RFC proposes to add the following attributes:

  * `#[doc(cfg(...))]`

    This attribute is used to document the operating systems, feature flags, and build profiles where an item is available. For example, `#[doc(cfg(unix))` will add a tag that says "this is supported on **unix** only" to the item.

    The syntax of this attribute is the same as the syntax of the [`#[cfg(unix)]` attribute][cfg attribute] used for conditional compilation.

  * `#[doc(auto_cfg)]`/`#[doc(no_auto_cfg)]`

    When this is turned on, `#[cfg]` attributes are shown in documentation just like `#[doc(cfg)]` attributes are.

  * `#[doc(cfg_hide(...))]` / `#[doc(cfg_show(...))]`

    These attributes suppress or un-suppress the `auto_cfg` behavior for a particular configuration predicate.

    For example, `#[doc(cfg_hide(windows))]` shall be used in newer versions of the [`windows` crate] to prevent the "this is supported on **windows** only" tag from being shown on every single item.

[cfg attribute]: https://doc.rust-lang.org/reference/conditional-compilation.html
[`windows` crate]: https://docs.rs/windows/latest/windows/

All of these attributes can be added to a module or to the crate root, and they will be inherited by the child items unless another attribute overrides it (except that `doc(cfg)` cannot be added to the crate root). This is why "opposite" attributes like `cfg_hide` and `cfg_show` are provided: they allow a child item to override its parent.


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## The attributes

### `#[doc(cfg(...))]`

This attribute provides a standardized format to document conditionally available items. Example:

```rust
// the "real" cfg condition
#[cfg(feature = "futures-io")]
// the `doc(cfg())` so it's displayed to the readers
#[doc(cfg(feature = "futures-io"))]
pub mod futures {}
```

It will display in the documentation for this module:

![This is supported on feature="futures-io" only.](https://user-images.githubusercontent.com/81079/89731116-d7b7ce00-da44-11ea-87c6-022d192d6eca.png)

This attribute has the same syntax as conditional compilation, but it only causes documentation to be added. This means `#[doc(cfg(false))` will not cause your docs to be hidden, even though `#[cfg(false)]` does do that.

This attribute works on modules and on items but cannot be used at the crate root level.

### `#[doc(auto_cfg)]`/`#[doc(no_auto_cfg)]`

By default, `#[doc(auto_cfg)]` is enabled at the crate-level. When it's enabled, Rustdoc will automatically display `cfg(...)` compatibility information as-if the same `#[doc(cfg(...))]` had been specified.

So if we take back the previous example:

```rust
#[cfg(feature = "futures-io")]
pub mod futures {}
```

There's no need to "duplicate" the `cfg` into a `doc(cfg())` to make Rustdoc display it.

In some situations, the detailed conditional compilation rules used to implement the feature might not serve as good documentation (for example, the list of supported platforms might be very long, and it might be better to document them in one place). To turn it off, add the `#[doc(no_auto_cfg)]` attribute.

Both `#[doc(auto_cfg)]` and `#[doc(no_auto_cfg)]` attributes impact all there descendants. You can then enable/disable them by using the opposite attribute on a given item. They can be used as follows:

```rust
// As an inner attribute, all this module's descendants will have this feature
// enabled.
#![doc(auto_cfg)]

// As an outer attribute. So in this case, `foo` and all its
// descendants won't have the `auto_cfg` feature enabled.
#[doc(no_auto_cfg)] 
pub mod foo {
    // We re-enable the feature on `Bar` and on all its descendants.
    #[doc(auto_cfg)]
    pub struct Bar {
        pub f: u32,
    }
}
```

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

Or directly on a given item/module as it covers any of the item's descendants:

```rust
#[doc(cfg_hide(doc))]
#[cfg(any(doc, feature = "futures-io"))]
pub mod futures {
    // `futures` and all its descendants won't display "doc" in their cfgs.
}
```

Then, the `doc` cfg will never be displayed into the documentation.

Rustdoc currently hides `doc` and `doctest` attributes by default and reserves the right to change the list of "hidden by default" attributes.

The attribute accepts only a list of identifiers or key/value items. So you can write:

```rust
#[doc(cfg_hide(doc, doctest, feature = "something"))]
#[doc(cfg_hide())]
```

But you cannot write:

```rust
#[doc(cfg_hide(not(doc)))]
```

### `#[doc(cfg_show(...))]`

This attribute does the opposite of `#[doc(cfg_hide(...))]`: if you used `#[doc(cfg_hide(...))]` and want to revert its effect on an item and its descendants, you can use `#[doc(cfg_show(...))]`:

```rust
#[doc(cfg_hide(doc))]
#[cfg(any(doc, feature = "futures-io"))]
pub mod futures {
    // `futures` and all its descendants won't display "doc" in their cfgs.
    #[doc(cfg_show(doc))]
    pub mod child {
        // `child` and all its descendants will display "doc" in their cfgs.
    }
}
```

The attribute accepts only a list of identifiers or key/value items. So you can write:

```rust
#[doc(cfg_show(doc, doctest, feature = "something"))]
#[doc(cfg_show())]
```

But you cannot write:

```rust
#[doc(cfg_show(not(doc)))]
```

## Inheritance

Rustdoc merges `cfg` attributes from parent modules to its children. For example, in this case, the module `non_unix` will describe the entire compatibility matrix for the module, and not just its directly attached information:

```rust
#[doc(cfg(any(windows, unix)))]
pub mod desktop {
    #[doc(cfg(not(unix)))]
    pub mod non_unix {
        //
    }
}
```

> ![Available on (Windows or Unix) and non-Unix only.](https://hackmd.io/_uploads/SJrmwYeF2.png)

[Future versions of rustdoc][boolean simplification] may simplify this display down to "available on **Windows** only."

### Re-exports and inlining

`cfg` attributes of a re-export are never merged the re-exported item(s). If `#[doc(inline)]` attribute is used on a re-export, the `cfg` of the re-exported item will be merged with the re-export's.

```rust
#[doc(cfg(any(windows, unix)))]
pub mod desktop {
    #[doc(cfg(not(unix)))]
    pub mod non_unix {
        //
    }
}

#[doc(cfg(target_os = "freebsd"))]
pub use desktop::non_unix as non_unix_desktop;
#[doc(cfg(target_os = "macos"))]
#[doc(inline)]
pub use desktop::non_unix as inlined_non_unix_desktop;
```

In this example, `non_unix_desktop` will only display `cfg(target_os = "freeebsd")` and not display any `cfg` from `desktop::non_unix`.

On the contrary, `inlined_non_unix_desktop` will have cfgs from both the re-export and the re-exported item.

# Drawbacks
[drawbacks]: #drawbacks

A potential drawback is that it adds more attributes, making documentation more complex.


# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Why not merging cfg and doc(cfg) attributes by default?

It was debated and implemented in [rust-lang/rust#113091](https://github.com/rust-lang/rust/pull/113091).

When re-exporting items with different cfgs there are two things that can happen:

 1. The re-export uses a subset of cfgs, this subset is sufficient so that the item will appear exactly with the subset
 2. The re-export uses a non-subset of cfgs like in this code:
   ```rust
    #![feature(doc_auto_cfg)]

    #[cfg(target_os = "linux")]
    mod impl_ {
        pub fn foo() { /* impl for linux */ }
    }

    #[cfg(target_os = "macos")]
    mod impl_ {
        pub fn foo() { /* impl for darwin */ }
    }

    pub use impl_::foo;
   ```
   If the non-subset cfgs are active (e.g. compiling this example on windows), then this will be a compile error as the item doesn't exist to re-export. If the subset cfgs are active it behaves like described in 1.


# Unresolved questions
[unresolved-questions]: #unresolved-questions


# Future possibilities
[future possibilities]: #future-possibilities

## Boolean simplification
[boolean simplification]: #boolean-simplification

> ![Available on (Windows or Unix) and non-Unix only.](https://hackmd.io/_uploads/SJrmwYeF2.png)

Of course, the above example is equivalent to "available on **Windows** only."

Making this actually work all the time is equivalent to a [boolean satisfiability] check, coliquially called a "SAT problem," and can take exponential time.

[boolean satisfiability]: https://en.wikipedia.org/wiki/Boolean_satisfiability_problem

We probably don't want to make promises one way or the other about whether rustdoc does this, but for compatibility's sake, Rustdoc does promise that `#[doc(cfg(false))` will not hide the documentation. This means simplification can be added, and it won't cause docs to mysteriously vanish.

This is tracked in issue [rust-lang/rust#104991](https://github.com/rust-lang/rust/issues/104991).
