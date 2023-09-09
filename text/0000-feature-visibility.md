- Feature Name: feature-metadata
- Start Date: 2023-09-08
- RFC PR: [rust-lang/rfcs#3416](https://github.com/rust-lang/rfcs/pull/3416)
- Rust Issue:
  [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary

[summary]: #summary

This RFC describes a new key under `features` in `Cargo.toml` to indicate that a
feature is private.

Please see the parent meta RFC for background information: [`feature-metadata`].

# Motivation

[motivation]: #motivation

WIP

No way to hide unstable API such as testing-only features or internally enabled
features.

# Guide-level explanation

[guide-level-explanation]: #guide-level-explanation

There will be a new flag allowed within `[features]`: `public`. This is boolean
flag defaulting to `true` that indicates whether or not downstream crates should
be allowed to use this feature.

```toml
[features]
foo = { enables = [], public = false}
```

Attempting to use a private feature on a downstream crate will result in
messages like the following:

```
error: feature `baz` on crate `mycrate` is private and cannot be used by
  downstream crates
```

# Reference-level explanation

[reference-level-explanation]: #reference-level-explanation

`public` is a boolean value that defaults to `true`. It can be thought of as
`pub` in Rust source files, with the exception of being true by default. If set
to `false`, Cargo should forbid its use with an error message on any downstream
crates.

The default `true` is not consistent with [`public_private_dependencies`] or
Rust's `pub`, but is a reasonable default to be consistent with the current
behavior. This means that either `feature = []` or
`feature = { "enables" = [] }` will result in the same configuration.

The name `public` was chosen in favor of `pub` to be consistent with the
[`public_private_dependencies`] RFC, and to match the existing style of using
non-truncated words as keys.

In general, marking a feature `public = false` should make tooling treat the
feature as non-public API. This is described as the following:

- The feature is always usable within the same crate:
  - Enablement by other features, e.g.
    `foo = { enables = [some-private-feature] }`, is allowed
  - Using the feature in integration tests is allowed
  - Using the feature in benchmarks is allowed
- The feature should not be accepted by `cargo add --features`
- The feature should not be reported from `cargo add`'s feature output report
- Once `rustdoc` is able to consume feature metadata, `rustdoc` should not
  document these features unless `--document-private-items` is specified
- A future tool like `cargo info` shouldn't display information about these
  features
- Explicitly specifying the feature via `--features somecrate/private-feature`
  will allow enabling a private feature that would otherwise be forbidden

Attempting to use a private feature in any of the forbidden cases should result
in an error. Exact details of how features work will likely be refined during
implementation and experimentation.

Two sample use cases for `public = false` include:

- `docs.rs` having a way to know which features should be hidden
- Features that are included in feature chains (feature `a` enables feature `b`)
  but not meant for public consumption could be marked not public

This feature requires adjustments to the index for full support. This RFC
proposes that it would be acceptable for the first implementation to simply
strip private features from the manifest; this meanss that there will be no way
to `cfg` based on these features.

Full support does not need to happen immediately, since it will require this
information be present in the index. [Index changes] describes how this can take
place.

# Drawbacks

[drawbacks]: #drawbacks

- Added complexity to Cargo. Parsing is trivial, but exact implementation
  details do add test surface area
- Added Cargo arguments if escape hatches for `public` are created
- `public` uses may not be common enough to be worth including

# Rationale and alternatives

[rationale-and-alternatives]: #rationale-and-alternatives

TODO

# Prior art

[prior-art]: #prior-art

- `docs.rs` displays a limited feature table. Features that start with `_` are
  hidden from this table.
- Ivy has a [visibility attribute] for its configuration (mentioned in
  [cargo #10882])
- Discussion on stable/unstable/nightly-only features
  <https://github.com/rust-lang/cargo/issues/10881>

# Unresolved questions

[unresolved-questions]: #unresolved-questions

- Are the semantics of `public` proposed in this RFC suitable? Should private
  features be usable in examples or integration tests without a `--features`
  argument?
- Should there be a simple `--allow-private-features` flag that allows using all
  features, such as for crater runs? This can be decided during implementation.
- Does `public` need to be in the index?

# Future possibilities

[future-possibilities]: #future-possibilities

- A `stable` field can be set false to indicate API-unstable or nightly-only
  features (somethign such as `stable = 3.2` could be used to indicate when a
  feature was stabilized). See also:
  <https://github.com/rust-lang/cargo/issues/10882>
- A `rust-version` field that could indicate e.g. `rust-version = "nightly"` or
  `rust-version = "1.65"` to specify a MSRV for that feature. See:
  <https://github.com/rust-lang/rfcs/pull/3416#discussion_r1174478461>
- The `public` option could be used to allow optional dev dependencies. See:
  <https://github.com/rust-lang/cargo/issues/1596>

[cargo #12335]: https://github.com/rust-lang/cargo/issues/12235
[cargo #10882]: https://github.com/rust-lang/cargo/issues/10882
[`cargo-info`]: https://github.com/rust-lang/cargo/issues/948
[`deprecated`]: https://doc.rust-lang.org/reference/attributes/diagnostics.html#the-deprecated-attribute
[`deprecated-suggestions`]: https://github.com/rust-lang/rust/issues/94785
[discussion on since]: https://github.com/rust-lang/rfcs/pull/3416#discussion_r1172895497
[`public_private_dependencies`]: https://rust-lang.github.io/rfcs/1977-public-private-dependencies.html
[`rustdoc-cargo-configuration`]: https://github.com/rust-lang/rfcs/pull/3421
[`tokio`]: https://docs.rs/crate/tokio/latest/features
[visibility attribute]: https://ant.apache.org/ivy/history/latest-milestone/ivyfile/conf.html
