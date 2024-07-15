- Feature Name: feature-documentation
- Start Date: 2023-09-09
- RFC PR: [rust-lang/rfcs#3485](https://github.com/rust-lang/rfcs/pull/3485)
- Rust Issue:
  [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary

[summary]: #summary

This RFC describes a new key under `features` in `Cargo.toml` for documentation.
This will allow Cargo to display this information to the user and provide a way
for `rustdoc` to eventually render this data (how this is rendered is outside
the scope of this RFC).

Example:

```toml
[features.serde]
enables = []
doc = "enable support for serialization and deserialization via serde"
```

Please see the parent meta RFC for background information: [`feature-metadata`].

# Motivation

[motivation]: #motivation

Cargo features have become extremely widely used, with many crates having at
least some level of configuration and larger crates winding up with tens of
gates. Despite being a first class component of crate structure, they suffer
from a documentation problem: users need to maintain documentation separate from
feature definition, typically a manually-created table within API docs.

This RFC proposes adding feature documentation to `Cargo.toml`, which will allow
for keeping feature definitions and documentation together.

# Guide-level explanation

[guide-level-explanation]: #guide-level-explanation

A new `doc` key will be allowed within a feature's table. This key provides a
markdown docstring describing the feature. The first paragraph will be treated
as a summary, and should be suitable to display standalone without the rest of
the description.

Don't include the name of the feature in the summary; tools will typically
already display this documentation alongside the name of the feature.

```toml
[features]
# Feature without documentation
foo = []

# Short documentation comment
bar = { enables = ["foo"], doc = "simple docstring here"}

# Tables are preferred for longer descriptions
[features.corge]
enables = ["bar", "baz"]
doc = """
The first paragraph is a short summary, which might be displayed standalone.
This could be a longer description of this feature
"""
```

See [`feature-metadata`] for information about `enables`.

# Reference-level explanation

[reference-level-explanation]: #reference-level-explanation

The new `doc` key accepts markdown-flavored text, and should be thought of as
the equivalent to a `#[doc(...)]` attribute. Like doc comments, the first line
should be treated as a summary. Intra-doc link support is not included in this
RFC, so they should not be used.

There is nothing in this RFC that cargo **must** do with this action, since it
is mainly intended for the consumption of `rustdoc` or `docs.rs`. However, it
can be used for general diagnostic information such as during `cargo add` or a
possible `cargo info` command. A sample application with `cargo add`:

```text
crab@rust foobar % cargo add regex
    Updating crates.io index
      Adding regex v1.7.3 to dependencies.
             Features:
             + perf             Enables all performance related features
             + perf-dfa         Enables the use of a lazy DFA for matching
             + perf-inline      Enables the use of aggressive inlining inside
                                match routines
             + perf-literal     Enables the use of literal optimizations for
                                speeding up matches
             + std              When enabled, this will cause regex to use the
                                standard library
             + unicode          Enables all Unicode features

    Updating crates.io index
```

_(features like `aho-corasick`, `memchr`, or `use_std` would likely be
`public = false` since they aren't listed on the crate landing page)_

Any tools that want the information in `doc` will require access to the
manifest. Adding this information to the index was decided against due to
concerns about bloat, but this is further discussed in
[future possibilities][future-possibilities].

# Drawbacks

[drawbacks]: #drawbacks

- Added complexity to Cargo.
  - Exact implementation details do add test surface area
  - A markdown parser is required to properly parse the `doc` field.
- Docstrings can be lengthy, adding noise to `Cargo.toml`. This could
  potentially be solved with the below mentioned `doc-file` key.
- When rendering features in documentation, this RFC does not specify any way
  for `rustdoc` to get the information it requires. This will require separate
  design work.
- Unlike with the
  [`document-features`](https://crates.io/crates/document-features) crate there
  is no way to group features into sections or have a user-specified layout
- Users cannot control features ordering in documentation since the TOML
  specification defines table keys as unordered.

# Rationale and alternatives

[rationale-and-alternatives]: #rationale-and-alternatives

- To avoid increasing the size of the registry index, this does not add `doc` to
  a package's index entry. This means a `.crate` file must be downloaded and
  extracted to access the features.
- Feature descriptions could be specified somewhere in Rust source files. This
  has the downside of creating multiple sources of truth on features.
- Cargo could parse doc comments in `Cargo.toml`, like the `document-features`
  crate (linked below).

  ```toml
  # RFC proposal
  foo = { enables = [], doc = "foo feature" }

  # Alternative equivalent using doc comments
  ## foo feature
  foo = []
  ```

  This was decided against as part of this RFC because it would mean that
  TOML-compliant parsers (including anything `serde`-based) would be
  insufficient to extract all information in the manifest, requiring custom
  deserialization of the fields via a format-preserving parser. This differs
  from documentation in Rust source as the doc-comment behavior is described
  specified within the grammar with parsers supporting extracting those
  elements.

# Prior art

[prior-art]: #prior-art

- There is an existing crate that uses TOML comments to create a features table:
  <https://docs.rs/document-features/latest/document_features/>
- `docs.rs` displays a feature table, but it is fairly limited. If features
  start with `_`, they are hidden from this table
  ([example](https://docs.rs/crate/regex/latest/features)).
- `lib.rs` extracts feature documentation from `Cargo.toml` and source
  ([example](https://lib.rs/crates/regex/features))

# Unresolved questions

[unresolved-questions]: #unresolved-questions

- Rather than being consistent with `rustdoc` and accepting markdown, should the
  `doc` key be consistent with `package.description` and only support plain
  text? This needs to be a point of discussion before approval of this RFC.

# Future possibilities

[future-possibilities]: #future-possibilities

- Rustdoc can build on this to show feature documentation.

  If this RFC gets stabilized before any corresponding change in rustdoc, its
  documentation should highlight that rustdoc may parse the description and
  support intra-doc links in the future, but not at the current time. Users need
  to be aware of this potential incompatibility.
- At some point, the decision to not include `doc` in the index could be
  reevaluated. Including only the first (summary) line of `doc` could be a
  possibility.
- `cargo add` can show the `doc` and `deprecated` summary with the listed
  features.
- [`cargo-info`] can use this information to provide feature descriptions.
- crates-io could be updated to render feature documentation
- Feature documentation could be allowed in a separate markdown file. For
  convenience, markdown anchors could be used to specify a section, so multiple
  features can share the same file. This could be a good option for features
  requiring long descriptions.

  ```toml
  foo = { enables = [], doc-file = "features.md#foo" }
  bar = { enables = [], doc-file = "features.md#bar" }
  ```

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
