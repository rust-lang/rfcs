- Feature Name: feature-deprecation
- Start Date: 2023-09-09
- RFC PR: [rust-lang/rfcs#3486](https://github.com/rust-lang/rfcs/pull/3486)
- Rust Issue:
  [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary

[summary]: #summary

This RFC describes a new key under `features` in `Cargo.toml` to indicate that a
feature is deprecated.

Please see the parent meta RFC for background information: [`feature-metadata`].

# Motivation

[motivation]: #motivation

Cargo features are widely used and typically have lifecycles the same as other
API components. There is not currently a way to indicate that a feature is
intended for removal and warn about it: This RFC proposes a `deprecated` key
that shows this information.

# Guide-level explanation

[guide-level-explanation]: #guide-level-explanation

A new `deprecated` key will be allowed for features, defaulting to `false` if
not specified. If specified, the value can be either a boolean, a string, or an
object with `since` and/or `note` keys. Cargo will warn downstream crates using
this feature.

```toml
[features]
foo = { enables = [], deprecated = true }
foo = { enables = [], deprecated = "this works as a note" }
bar = { enables = [], deprecated = { since = "1.2.3", note = "don't use this!" } }
```

See [`feature-metadata`] for information about `enables`.

# Reference-level explanation

[reference-level-explanation]: #reference-level-explanation

`deprecated` should be thought of as the equivalent of the [`deprecated`]
attribute in Rust source. The value can be a boolean, string, or an object with
`since` or `note` keys. Schema rules are as follows:

- If a boolean value, `false` indicates not deprecated and `true` indicates
  deprecated
- If an object, the keys `since` and/or `note` can be specified
  - An empty object is not allowed to avoid ambiguity `foo = { deprecated = {} }`
- If a string (e.g. `foo = { deprecated = "my msg" }`), it will be equivalent to if that
  string was specified in the `note` field (e.g. `foo = { deprecated = { note = "my msg" } }`)
- If not specified, the default is `false`

If a downstream crate attempts to use a feature marked `deprecated`, Cargo
should produce a warning that contains the `note`. This warning should not be
emitted for crates that reexport the feature under a feature also marked
deprecated. For example: crate `foo` exports feature `phooey`, and crate `bar`
exports feature `barred = ["foo/phooey"]`. If `foo` markes `phooey` as deprecated,
running any cargo action on `bar` will emit a warning unless `barred` is also
marked `deprecated`.

Accessing this information will require access to the manifest as it will not be
in the index.

## A note on `since`

The exact behavior of the `since` key is not provided in this RFC as there are
decisions related to resolution that need to be made. The generally accepted
concept is that there should be a warning if a deprecated feature is used _and_
there is something actionable to resolve this issue for all downstream crates -
but the details of how best to do this are not yet clear. Please see
[discussion on since].

If the exact behavior of `since` does not reach consensus before `deprecated` is
nearing stabilization, this key can stabilized separately or dropped entirely.

## Index changes

[index changes]: #index-changes

The infromation provided by `deprecated` needs to be stored in the index, and
will be stored under a `features3` key. Older versions of Cargo will ignore this
key, newer Cargo would be able to merge `features`, `features2`, and
`features3`. `features3` should mirror the most complete syntax of the relevant
keys from the `[features]` table, i.e.:

```json5
"features3": {
    "bar": {
        deprecated = { since = "1.2.3", note = "don't use this" }
    }
}
```

In order to conserve index space, default keys should be omitted. `Cargo` should
ignore unrecognized keys within a feature, to allow for future additions without
needing a new `features` section.

# Drawbacks

[drawbacks]: #drawbacks

- Added complexity to Cargo. Parsing is trivial, but exact implementation
  details do add test surface area

# Rationale and alternatives

[rationale-and-alternatives]: #rationale-and-alternatives

WIP

# Prior art

[prior-art]: #prior-art

WIP

# Unresolved questions

[unresolved-questions]: #unresolved-questions

- How should `since` work with the `deprecated` key? See
  [a note on `since`](#a-note-on-since) for further information.

# Future possibilities

[future-possibilities]: #future-possibilities

- Somehow inform users if they are using to-be-deprecated features, i.e.,
  deprecated `since` is set but is later than the current dependancy version.
- Via the `manifest-lint` RFC, a user could specify that deprecated crates
  should be denied. This would, however, be blocked by [cargo #12335].
- A `stable` field can be set false to indicate API-unstable or nightly-only
  features (something such as `stable = 3.2` could be used to indicate when a
  feature was stabilized). See also:
  <https://github.com/rust-lang/cargo/issues/10882>
- A `rust-version` field that could indicate e.g. `rust-version = "nightly"` or
  `rust-version = "1.65"` to specify a MSRV for that feature. See:
  <https://github.com/rust-lang/rfcs/pull/3416#discussion_r1174478461>
- `cargo add` can show the `deprecated` summary with the listed features.
- `deprecated` could take a `suggestion` key that indicates features have moved
  to a different name (as with the [`deprecated-suggestions`] feature)

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
[`feature-metadata`]: https://github.com/rust-lang/rfcs/pull/3416
