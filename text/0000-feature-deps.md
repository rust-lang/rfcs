- Feature Name: feature-deps
- Start Date: 2024-06-20
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary

[summary]: #summary

This RFC extends Cargo's structured table form of features to include a `deps`
key, to unambiguously specify packages the feature depends on without having to
use the `dep:foo` microformat.

```toml
[features]
myfeature = { deps = ["some-crate", "another-crate"] }
# This is equivalent to `myfeature = ["dep:some-crate", "dep:another-crate"]`
# This can also be written in shorthand form:
myfeature.deps = ["some-crate", "another-crate"]
```

This depends on
[RFC 3416 (feature metadata)](https://github.com/rust-lang/rfcs/pull/3416),
which defines the structured table form of features.

# Motivation
[motivation]: #motivation

Cargo features can depend on other features, on other packages, and on features
of other packages. [RFC 3143](https://github.com/rust-lang/rfcs/pull/3143) and
[RFC 3491](https://github.com/rust-lang/rfcs/pull/3491) have migrated Cargo to
distinguishing a feature's different kinds of dependencies with different
syntax: `dep:some-crate` for a crate, `some-crate?/feature` to enable an
optional dependency's feature without force-enabling the optional dependency
itself, and `some-crate/feature` as a shorthand combining the two.

This has created somewhat of a microformat within the dependencies list, and
this microformat adds complexity compared to other parts of the Cargo manifest,
which distinguish different kinds of things via different keys or sections. (By
way of example, we have `dev-dependencies` and `build-dependencies`, rather
than a microformat like `dependencies."build:xyz"`.)

Now that [RFC 3416](https://github.com/rust-lang/rfcs/pull/3416) has defined a
structured table form for defining features, this RFC takes a first step
towards providing structured equivalents in place of the microformat.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When defining features in a Cargo manifest, you can specify a feature's
dependencies in a separate key `deps`. You can define a feature that depends on
both other features and on optional crates, or on only one or the other.

```toml
[features]
feat1 = { enables = ["feat2", "feat3"], deps = ["crate1"] }
feat2 = { deps = ["crate2"] }
feat3.deps = ["crate3"]
"""
```

The `deps` key is equivalent to including items in `enables` (or a traditional
feature definition) with a `dep:` prefix. For example,
`feat2 = { deps = ["crate2"] }` is equivalent to
`feat2 = { enables = ["dep:crate2"] }` or
`feat2 = ["dep:crate2"]`.

However, if a crate uses the `deps` key, it should consistently use the `deps`
key for all feature dependencies on crates, rather than using a mix of `deps`
and `"dep:"`. (Cargo may choose to warn about mixing the two.)

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The index can translate this to the microformat, to preserve compatibility
until the next time the index format needs to make an incompatible change.

Cargo should detect invalid syntax in the `deps` key, such as the use of `dep:`
or `cratename/featurename` or `cratename?/featurename`, and provide errors that
suggest what to write instead.

# Drawbacks
[drawbacks]: #drawbacks

- This adds minor incremental complexity to parsing the manifest, both for
  Cargo itelf, and for third-party tools that parse the manifest without using
  Cargo's parser.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

We could translate this to the microformat when creating the `Cargo.toml` on
publish, but this would not work for features using any new fields (such as
`doc`), so this RFC proposes that using the new mechanism require new Cargo,
for simplicity.

## Naming

- `deps` is consistent with the existing `"dep:"` prefix, and is shorter so it
  more easily fits in a one-line feature definition.
- `dependencies` would be more consistent with the top-level `dependencies`
  tables.

# Future possibilities
[future-possibilities]: #future-possibilities

In the future, we could also provide an alternative to the
`crate-name?/feature-name` microformat, or alternatively we could migrate
towards making `crate-name/feature-name` mean that in a future edition. This
RFC does not propose an alternative for that syntax. This incremental approach
was inspired by the incremental, easier-to-review RFCs currently being used for
other individual pieces of the structured table format for features.
