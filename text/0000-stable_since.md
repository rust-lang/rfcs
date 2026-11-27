- Feature Name: stable_since
- Start Date: 2025-09-12
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#74182](https://github.com/rust-lang/rust/issues/74182)

# Summary
[summary]: #summary

Allow crates to specify `#[stable(since = "version")]` on items for Rustdoc to render the information.

# Motivation
[motivation]: #motivation

This functionality is already implemented for the standard library. Other crates also expand their public API over time, and it would be helpful to inform users about the minimum crate version required for each item.

It's possible to automatically infer in which version each item has been added/changed by comparing public APIs of crates' public releases, but this is too expensive and complicated to perform each time when generating documentation. The `#[stable(since)]` attribute can provide this information in a way that is readily available for `rustdoc` and other tools.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When added to an item, it specifies the oldest version of the crate that has this item available, and implemented in a way that is compatible with the crate's current version:

```rust
#[stable(since = "2.25.0")]
pub fn add_trombone_emoji() {}
```

Rustdoc will include the version in the documentation of the item, with a description such as "stable since $crate_name version 2.25.0".

To ease development of unreleased features, there is no restriction on the version range, and it may refer to a future not-yet-released version.

The version in `since` must be updated when the interface changes in a semver-incompatible way.

The version in `since` should be updated when the behavior of the item changes significantly. If in doubt, it should be the later version. Dependency management tools may use this version to suggest bumping miniumum required version in `Cargo.toml`.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The version in `since` refers to a version of the crate that defines the item this attribute belongs to. The version should parse as Cargo-compatible SemVer, but this won't be enforced by `rustc`, only `clippy` (same as `#[deprecated]`).

`rustdoc` should not display the attribute on items defined in other crates.

This attribute on `pub use` refers to availability of the path.

This attribute may be set on public items that are not accessible outside of the crate. This attribute set on private items may trigger a warning.

# Drawbacks
[drawbacks]: #drawbacks

It's not obvious how this attribute should be used with APIs that change over time. It's simpler to only specify when an item has been added, but this can be misleading if the latest version differs significantly from the earliest version. Specifying the lowest *compatible* version is more useful for choosing the minimum required crate version, but this type of compatibility is hard to define (semver is well understood when upgrading, but downgrading loses new features and reintroduces old bugs, so even a semver-patch downgrade may be breaking).

This attribute may be incorrect if added manually, or become stale if not updated after a significant change.

It specifies only a single version per item, which may not be enough to fully explain availability of items that are available conditionally or under different paths.

Versions on re-exported items are not relevant for the crate re-exporting them, because it matters when the re-export has been added.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The version used in third party crates is unlikely to be confused with MSRV, because very few crates have only versions in the 1.xx range relevant for Rust versions.

`since` is used for consistency with `#[deprecated(since)]`.

Alternatives:

- The entire `#[stable(feature)]`/`#[unstable(feature)]` functionality could be stabilized for 3rd party crates.
- API stability could be stored outside of the source code, e.g. in a file similar to `rustdoc`'s JSON.
- It could be generalized to `#[changed(version = "semver", note = "how")]` that provides a changelog for each item. This could also be implemented by allowing multiple instances of `#[stable(since = "version", note = "changes")]`.
- docs.rs could generate `rustdoc` JSON for all crates.io crates, making it easier to diff releases and annotate API changes automatically.

# Prior art
[prior-art]: #prior-art

The `#[stable(since = "version")]` attribute is a subset of standard library's `#[stable(feature, since)]`. This RFC does not include support for feature flags nor unstable APIs outside of the standard library. The standard library has unusual backwards-compatibility constraints, so it doesn't serve as a guide for hanlding APIs with breaking changes.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

Should it specify the oldest version regardless of compatibility? Should it be bumped only on semver-breaking API changes, or also when the behavior changes?

Should it be allowed on private items? (there's `--document-private-items`, but those items won't be accessible from outside of the crate).

Should it support specifying different kinds of stability, like `const_stable`?

# Future possibilities
[future-possibilities]: #future-possibilities

The entire `#[stable(feature)]`/`#[unstable(feature)]` functionality could be stabilized for 3rd party crates.

A placeholder like `NEXT` or `UNRELEASED` could be supported, and either automatically updated or rejected by `cargo publish` (clippy allows "TBD" in `deprecated(since)`).

A new `#[changed]` attribute could be added for tracking history of incompatible changes or extensions. This RFC proposes `#[stable]` to track the latest incompatible change, so `#[changed]` would provide information about partial compatibility with older versions, or a changelog.

Tools like rust-analyzer or clippy could help users bump versions in `Cargo.toml` when their crate uses items from a newer version of a dependency than the minimum version specified in `Cargo.toml` (like `clippy::incompatible_msrv` for crates).

These attributes could be automatically generated by tools like `cargo-public-api` or `cargo-semver-checks`.

These attributes could be used by tools that auto-generate changelogs.
