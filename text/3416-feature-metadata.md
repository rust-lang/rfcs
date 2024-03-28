- Feature Name: feature-metadata
- Start Date: 2023-04-14
- RFC PR: [rust-lang/rfcs#3416](https://github.com/rust-lang/rfcs/pull/3416)
- Rust Issue:
  [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary

[summary]: #summary

This RFC is just meta tracking information for the three following RFCs:

- [Cargo feature descriptions](https://github.com/rust-lang/rfcs/pull/3485)
- [Cargo feature deprecation](https://github.com/rust-lang/rfcs/pull/3486)
- [Cargo feature visibility](https://github.com/rust-lang/rfcs/pull/3487)

# Motivation

[motivation]: #motivation

Features are widely used as a way to do things like reduce dependency count,
gate `std` or `alloc`-dependent parts of code, or hide unstable API. Use is so
common that many larger crates wind up with tens of feature gates, such as
[`tokio`] with 24. Despite being a first class component of crate structure,
there are some limitations that don't have elegant solutions:

- Documentation is difficult, often requiring library authors to manually manage
  a table of descriptions
- There is no way to deprecate old features, as a way to help crates maintain
  semvar compliance
- Features cannot be hidden from use in any way

This RFC proposes a plan that add that information to `Cargo.toml`, solving
these problems.

# Guide-level explanation

[guide-level-explanation]: #guide-level-explanation

Usage is simple: features will be able to be specified as a table, instead of
just a dependency array. This sample section of `Cargo.toml` shows new
possibilities:

```toml
[features]
# Current configuration will continue to work
foo = []
# New configurations
bar = { enables = ["foo"], doc = "simple docstring here"}
baz = { enables = ["foo"], public = false}
qux = { enables = [], deprecated = true }
quux = { enables = [], deprecated = { since = "1.2.3", note = "don't use this!" } }

# Features can also be full tables if descriptions are longer
[features.corge]
enables = ["bar", "baz"]
doc = """
# corge

This could be a longer description of this feature
"""
```

The `enables` key is synonymous with the existing array, describing what other
features are enabled by a given feature. For example,
`foo = ["dep:serde", "otherfeat"]` will be identical to
`foo = { enables = ["dep:serde", "otherfeat"] }`

All other keys are described in their individual RFCs.

# Reference-level explanation

[reference-level-explanation]: #reference-level-explanation

`enables` will take the place of the feature dependency array that currently
exists. Semantics will remain unchanged.

This is a required key. If there are no requirements, an empty list should be
provided (`enables = []`). This content is already in the index.

# General Implementation & Usage

Use cases for this information will likely develop with time, but one of the
simplest applications is for information output with `cargo add`:

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
             - deprecated (D)   Not a real feature, but it could be

    Updating crates.io index
```

Features like `aho-corasick`, `memchr`, or `use_std` would likely be
`public = false` since they aren't listed on the crate landing page.

## Implementation note: sort order

In general, any tool that renders features (`rustdoc`, `cargo add`) should
attempt to present them in the following way:

- Display default features first
- Display non-default but stable features next (can be in a separate section)
- Display deprecated features last (can be in a separate section)
- Do not display private features unless receiving a flag saying to do so (e.g.
  `--document-private-items` with `rustdoc`)
- If ordering is not preserved, present the features alphabetically

# Drawbacks

[drawbacks]: #drawbacks

- Added complexity to Cargo. Parsing is trivial, but exact implementation
  details do add test surface area

# Rationale and alternatives

[rationale-and-alternatives]: #rationale-and-alternatives

# Prior art

[prior-art]: #prior-art

# Unresolved questions

[unresolved-questions]: #unresolved-questions

# Future possibilities

[future-possibilities]: #future-possibilities

- A `rust-version` field that could indicate e.g. `rust-version = "nightly"` or
  `rust-version = "1.65"` to specify a MSRV for that feature. See:
  <https://github.com/rust-lang/rfcs/pull/3416#discussion_r1174478461>
- `cargo add` can show the `doc` and `deprecated` summary with the listed
  features.
- [`cargo-info`] can use this information to provide feature descriptions.

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
