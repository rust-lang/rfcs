Rustdoc: stabilization of the `doc(cfg*)` attributes

- Features Name: `doc_cfg`
- Start Date: 2022-12-07
- RFC PR: [rust-lang/rfcs#3631](https://github.com/rust-lang/rfcs/pull/3631)
- Rust Issue: [rust-lang/rust#43781](https://github.com/rust-lang/rust/issues/43781)


## Summary
[summary]: #summary

This RFC aims at providing rustdoc users the possibility to add visual markers to the rendered documentation to know under which conditions an item is available (currently possible through the following unstable features: `doc_cfg`, `doc_auto_cfg` and `doc_cfg_hide`).

It does not aim to allow having a same item with different `cfg`s to appear more than once in the generated documentation.

It does not aim to document items which are *inactive* under the current configuration (i.e., “`cfg`ed out”). More details in the [Unresolved questions section](#unresolved-questions).

## Motivation
[motivation]: #motivation

The goal of this RFC is to stabilize the possibility to add visual markers to the rendered documentation to know under which conditions an item is available.

Providing this information to users will solve a common issue: “Why can I see this item in the documentation and yet can't use it in my code?”.
The end goal being to provide this information automatically so that the documentation maintenance cost won't increase.


## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This RFC proposes to add the following attributes:

  * `#[doc(auto_cfg)]`/`#[doc(auto_cfg = true)]`/`#[doc(auto_cfg = false)]`

    When this is turned on (with `doc(auto_cfg)` or `doc(auto_cfg = true)`), `#[cfg]` attributes are shown in documentation just like `#[doc(cfg)]` attributes are. By default, `auto_cfg` will be enabled.

  * `#[doc(cfg(...))]`

    This attribute is used to document the operating systems, feature flags, and build profiles where an item is available. For example, `#[doc(cfg(unix))` will add a tag that says "this is supported on **unix** only" to the item.

    The syntax of this attribute is the same as the syntax of the [`#[cfg()]` attribute][cfg attribute] used for conditional compilation.

  * `#![doc(auto_cfg(hide(...)))]` / `#[doc(auto_cfg(show(...)))]`

    These attributes suppress or un-suppress the `auto_cfg` behavior for a particular configuration predicate.

    For example, `#[doc(auto_cfg(hide(windows)))]` could be used in newer versions of the [`windows` crate] to prevent the "this is supported on **windows** only" tag from being shown on every single item. Using these attributes will also re-enable `doc(auto_cfg)` if it was disabled at this location.

[cfg attribute]: https://doc.rust-lang.org/reference/conditional-compilation.html
[`windows` crate]: https://docs.rs/windows/latest/windows/

All of these attributes can be added to a module or to the crate root, and they will be inherited by the child items unless another attribute overrides it. This is why "opposite" attributes like `auto_cfg(hide(...))` and `auto_cfg(show(...))` are provided: they allow a child item to override its parent.

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

### The attributes

#### `#[doc(auto_cfg)`/`#[doc(auto_cfg = true)]`/`#[doc(auto_cfg = false)]`

By default, `#[doc(auto_cfg)]` is enabled at the crate-level. When it's enabled, Rustdoc will automatically display `cfg(...)` compatibility information as-if the same `#[doc(cfg(...))]` had been specified.

This attribute impacts the item on which it is used and its descendants.

So if we take back the previous example:

```rust
#[cfg(feature = "futures-io")]
pub mod futures {}
```

There's no need to "duplicate" the `cfg` into a `doc(cfg())` to make Rustdoc display it.

In some situations, the detailed conditional compilation rules used to implement the feature might not serve as good documentation (for example, the list of supported platforms might be very long, and it might be better to document them in one place). To turn it off, add the `#[doc(auto_cfg = false)]` attribute on the item.

If no argument is specified (ie `#[doc(auto_cfg)]`), it's the same as writing `#[doc(auto_cfg = true)]`.

#### `#[doc(cfg(...))]`

This attribute provides a standardized format to override `#[cfg()]` attributes to document conditionally available items. Example:

```rust
// the "real" cfg condition
#[cfg(feature = "futures-io")]
// the `doc(cfg())` so it's displayed to the readers
#[doc(cfg(feature = "futures-io"))]
pub mod futures {}
```

It will display in the documentation for this module:

![This is supported on feature="futures-io" only.](https://user-images.githubusercontent.com/81079/89731116-d7b7ce00-da44-11ea-87c6-022d192d6eca.png)

You can use it to display information in generated documentation, whether or not there is a `#[cfg()]` attribute:

```rust
#[doc(cfg(feature = "futures-io"))]
pub mod futures {}
```

It will be displayed exactly the same as the previous code.

This attribute has the same syntax as conditional compilation, but it only causes documentation to be added. This means `#[doc(cfg(not(windows)))]` will not cause your docs to be hidden on non-windows targets, even though `#[cfg(not(windows))]` does do that.

If `doc(auto_cfg)` is enabled on the item, `doc(cfg)` will override it anyway so in the two previous examples, even if the `doc(auto_cfg)` feature was enabled, it would still display the same thing.

This attribute works on modules and on items.

#### `#[doc(auto_cfg(hide(...)))]`

This attribute is used to prevent some `cfg` to be generated in the visual markers. It only applies to `#[doc(auto_cfg = true)]`, not to `#[doc(cfg(...))]`. So in the previous example:

```rust
#[cfg(any(unix, feature = "futures-io"))]
pub mod futures {}
```

It currently displays both `unix` and `feature = "futures-io"` into the documentation, which is not great. To prevent the `unix` cfg to ever be displayed, you can use this attribute at the crate root level:

```rust
#![doc(auto_cfg(hide(unix)))]
```

Or directly on a given item/module as it covers any of the item's descendants:

```rust
#[doc(auto_cfg(hide(unix)))]
#[cfg(any(unix, feature = "futures-io"))]
pub mod futures {
    // `futures` and all its descendants won't display "unix" in their cfgs.
}
```

Then, the `unix` cfg will never be displayed into the documentation.

Rustdoc currently hides `doc` and `doctest` attributes by default and reserves the right to change the list of "hidden by default" attributes.

The attribute accepts only a list of identifiers or key/value items. So you can write:

```rust
#[doc(auto_cfg(hide(unix, doctest, feature = "something")))]
#[doc(auto_cfg(hide()))]
```

But you cannot write:

```rust
#[doc(auto_cfg(hide(not(unix))))]
```

So if we use `doc(auto_cfg(hide(unix)))`, it means it will hide all mentions of `unix`:

```rust
#[cfg(unix)] // nothing displayed
#[cfg(any(unix))] // nothing displayed
#[cfg(any(unix, windows))] // only `windows` displayed
```

However, it only impacts the `unix` cfg, not the feature:

```rust
#[cfg(feature = "unix")] // `feature = "unix"` is displayed
```

If `cfg_auto(show(...))` and `cfg_auto(hide(...))` are used to show/hide a same `cfg` on a same item, it'll emit an error. Example:

```rust
#[doc(auto_cfg(hide(unix)))]
#[doc(auto_cfg(show(unix)))] // Error!
pub fn foo() {}
```

Using this attribute will re-enable `auto_cfg` if it was disabled at this location:

```rust
#[doc(auto_cfg = false)] // Disabling `auto_cfg`
pub fn foo() {}
```

And using `doc(auto_cfg)` will re-enable it:

```rust
#[doc(auto_cfg = false)] // Disabling `auto_cfg`
pub mod module {
    #[doc(auto_cfg(hide(unix)))] // `auto_cfg` is re-enabled.
    pub fn foo() {}
}
```

However, using `doc(auto_cfg = ...)` and `doc(auto_cfg(...))` on the same item will emit an error:

```rust
#[doc(auto_cfg = false)]
#[doc(auto_cfg(hide(unix)))] // error
pub fn foo() {}
```

The reason behind this is that `doc(auto_cfg = ...)` enables or disables the feature, whereas `doc(auto_cfg(...))` enables it unconditionally, making the first attribute to appear useless as it will be overidden by the next `doc(auto_cfg)` attribute.

#### `#[doc(auto_cfg(show(...)))]`

This attribute does the opposite of `#[doc(auto_cfg(hide(...)))]`: if you used `#[doc(auto_cfg(hide(...)))]` and want to revert its effect on an item and its descendants, you can use `#[doc(auto_cfg(show(...)))]`.
It only applies to `#[doc(auto_cfg = true)]`, not to `#[doc(cfg(...))]`.

For example:

```rust
#[doc(auto_cfg(hide(unix)))]
#[cfg(any(unix, feature = "futures-io"))]
pub mod futures {
    // `futures` and all its descendants won't display "unix" in their cfgs.
    #[doc(auto_cfg(show(unix)))]
    pub mod child {
        // `child` and all its descendants will display "unix" in their cfgs.
    }
}
```

The attribute accepts only a list of identifiers or key/value items. So you can write:

```rust
#[doc(auto_cfg(show(unix, doctest, feature = "something")))]
#[doc(auto_cfg(show()))]
```

But you cannot write:

```rust
#[doc(auto_cfg(show(not(unix))))]
```

If `auto_cfg(show(...))` and `auto_cfg(hide(...))` are used to show/hide a same `cfg` on a same item, it'll emit an error. Example:

```rust
#[doc(auto_cfg(show(unix)))]
#[doc(auto_cfg(hide(unix)))] // Error!
pub fn foo() {}
```

Using this attribute will re-enable `auto_cfg` if it was disabled at this location:

```rust
#[doc(auto_cfg = false)] // Disabling `auto_cfg`
#[doc(auto_cfg(show(unix)))] // `auto_cfg` is re-enabled.
pub fn foo() {}
```

### Inheritance

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

#### Re-exports and inlining

`cfg` attributes of a re-export are never merged with the re-exported item(s) attributes except if the re-export has the `#[doc(inline)]` attribute. In this case, the `cfg` of the re-exported item will be merged with the re-export's.

When talking about "attributes merge", we mean that if the re-export has `#[cfg(unix)]` and the re-exported item has `#[cfg(feature = "foo")]`, you will only see `cfg(unix)` on the re-export and only `cfg(feature = "foo")` on the re-exported item, unless the re-export has `#[doc(inline)]`, then you will only see the re-exported item with both `cfg(unix)` and `cfg(feature = "foo")`.

Example:

```rust
#[doc(cfg(any(windows, unix)))]
pub mod desktop {
    #[doc(cfg(not(unix)))]
    pub mod non_unix {
        // code
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

So that also means that if a crate re-exports a foreign item, unless it has `#[doc(inline)]`, the `cfg` and `doc(cfg)` attributes will not be visible:

```rust
// dep:
#[cfg(feature = "a")]
pub struct S;

// crate using dep:

// There will be no mention of `feature = "a"` in the documentation.
pub use dep::S as Y;
```

## Drawbacks
[drawbacks]: #drawbacks

A potential drawback is that it adds more attributes, making documentation more complex.


## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

### Why not merging cfg and doc(cfg) attributes by default?

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


## Unresolved questions
[unresolved-questions]: #unresolved-questions

### `cfg`ed out items

Rustdoc doesn't take into account `cfg`ed out items. The reason for this limitation is that Rustdoc has only access to rustc's information: `cfg`ed out items, although still present, don't have enough information to be useful to rustdoc when generating documentation, hence why they are not treated.

So for the following crate, `function` wouldn't show up in the generated docs unless you actually passed `--cfg special` to Rustdoc:

```rust
#[cfg(special)]
pub fn function() {}
```

Therefore, the common and offical workaround is the use of the semi-special cfg `doc`:

```rust
#[cfg(any(doc, special))]
pub fn function() {}
```

There are a few leads on how Rustdoc could solve this issue, but they all come with big drawbacks, so this problem is not addressed in this RFC but will (hopefully) be in the future.

## Future possibilities
[future possibilities]: #future-possibilities


### Boolean simplification
[boolean simplification]: #boolean-simplification

> ![Available on (Windows or Unix) and non-Unix only.](https://hackmd.io/_uploads/SJrmwYeF2.png)

Of course, the above example is equivalent to "available on **Windows** only."

We probably don't want to make promises one way or the other about whether rustdoc does this, but for compatibility's sake, Rustdoc does promise that `#[doc(cfg(false))]` will not hide the documentation. This means simplification can be added, and it won't cause docs to mysteriously vanish.

This is tracked in issue [rust-lang/rust#104991](https://github.com/rust-lang/rust/issues/104991).
