- Feature Name: `cfg-target-abi`
- Start Date: 2020-09-27
- RFC PR: [rust-lang/rfcs#2992](https://github.com/rust-lang/rfcs/pull/2992)
- Rust Issue: [rust-lang/rust#80970](https://github.com/rust-lang/rust/issues/80970)

# Summary
[summary]: #summary

This proposes a new `cfg`: `target_abi`, which specifies certain aspects of the
target's [Application Binary Interface (ABI)][abi]. This also adds a
`CARGO_CFG_TARGET_ABI` environment variable for parity with other
`CARGO_CFG_TARGET_*` variables.

# Motivation
[motivation]: #motivation

Certain targets are only differentiated by their ABI. For example: the `ios` OS
in combination with the `macabi` ABI denotes targeting Mac Catalyst (iOS on
macOS). The non-`macabi` `x86_64-apple-ios` target is not for Mac Catalyst and
instead is for the iOS simulator, which is a very different environment.

It is not currently possible to `#[cfg]` against a certain target ABI without
a `build.rs` script to emit a custom `cfg` based on the `TARGET` environment
variable. This is not ideal because:

- Adding a build script increases compile time and makes a crate incompatible
  with certain build systems.

- Checking `TARGET` is error prone, mainly because the ABI often follows
  `target_env` without separation.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This would act like [existing `target_*` configurations][cfg-options].

For example: if one had a module with bindings to
[Apple's AppKit](https://developer.apple.com/documentation/appkit), this feature
could be used to ensure the module is available when targeting regular macOS and
Mac Catalyst.

```rust
#[cfg(any(
    target_os = "macos",
    all(
        target_os = "ios",
        target_abi = "macabi",
    ),
))]
pub mod app_kit;
```

This configuration option would also be usable as
`#[cfg_attr(target_abi = "...", attr)]`.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

`target_abi` is a key-value option set once with the target's ABI. The value is
similar to the fourth element of the platform's target triple. It often comes
after the `target_env` value. Embedded ABIs such as `gnueabihf` will define
`target_env` as `"gnu"` and `target_abi` as `"eabihf"`.

Example values:

- `""`
- `"abi64"`
- `"eabi"`
- `"eabihf"`
- `"macabi"`

# Drawbacks
[drawbacks]: #drawbacks

- Additional metadata for the compiler to keep track of.

- Like other `cfg`s, this can be manipulated at build time to be a value that
  mismatches the actual target.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

We can keep the existing work-around of checking the `TARGET` environment
variable in a `build.rs` script. However, this is not ideal because:

- Adding a build script increases compile time and makes a crate incompatible
  with certain build systems.

- Checking `TARGET` is error prone, mainly because the ABI often follows
  `target_env` without separation.

# Prior art
[prior-art]: #prior-art

- [Target component configurations][cfg-options]: `target_arch`,
  `target_vendor`, `target_os`, and `target_env`.

- `CARGO_CFG_TARGET_*`
  [environment variables for `build.rs`](https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-build-scripts).

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None.

# Future possibilities
[future-possibilities]: #future-possibilities

None.

[abi]: https://en.wikipedia.org/wiki/Application_binary_interface
[cfg-options]: https://doc.rust-lang.org/reference/conditional-compilation.html#set-configuration-options
