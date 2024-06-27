- Feature Name: feature-metadata
- Start Date: 2023-04-14
- RFC PR: [rust-lang/rfcs#3416](https://github.com/rust-lang/rfcs/pull/3416)
- Rust Issue:
  [rust-lang/cargo#14157](https://github.com/rust-lang/cargo/issues/14157)

# Summary

[summary]: #summary

This RFC adds a "detailed" feature definition:
```toml
[features]
# same as `foo = []`
foo = { enables = [] }
```

This is to unblock the following RFCs:

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

## General Implementation & Usage

Use cases for these new keys will likely develop with time,
but one of the simplest applications is for information output with `cargo
add`:

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

# Reference-level explanation

[reference-level-explanation]: #reference-level-explanation

`enables` will take the place of the feature dependency array that currently
exists. Semantics will remain unchanged.

This is a required key. If there are no requirements, an empty list should be
provided (`enables = []`). This content is already in the index.

The availability of this new syntax should not require an MSRV bump.
This means we need to make sure that if you use `feature_name = []` in your `Cargo.toml`,
then the published `Cargo.toml` should as well.
However, we leave it as an implementation detail whether using `feature_name = { enables =[] }`
requires an MSRV bump for users of your published package as we have not been
actively streamlining the workflow for maintaining separate development and
published MSRVs.

# Drawbacks

[drawbacks]: #drawbacks

- Added complexity to Cargo. Parsing is trivial, but exact implementation
  details do add test surface area
- Extending the `Cargo.toml` schema, particularly having a field support
  additional types, is disruptive to third-party parsers

# Rationale and alternatives

[rationale-and-alternatives]: #rationale-and-alternatives

This RFC has no impact on the Index Summaries.
Future RFCs will need to work with that.

## Naming

- `enables` reads better on the line than `enable`
- `enables` is likely an easier word for non-native speakers than `activates`
- `required` is used elsewhere to say "this should automatically be available if requirements are met"

## Schema

We could split the special feature syntax (`dep:`, etc) as distinct fields
but we'd prefer trivial conversion from the "simple" schema to the "detailed" schema,
like `dependencies`.
However, we likely would want to prefer using new fields over adding more syntax,
like with [disabling default features](https://github.com/rust-lang/cargo/issues/3126).

# Prior art

[prior-art]: #prior-art

# Unresolved questions

[unresolved-questions]: #unresolved-questions

# Future possibilities

[future-possibilities]: #future-possibilities

- [Cargo feature descriptions](https://github.com/rust-lang/rfcs/pull/3485)
- [Cargo feature deprecation](https://github.com/rust-lang/rfcs/pull/3486)
- [Cargo feature visibility](https://github.com/rust-lang/rfcs/pull/3487)
- [Cargo feature stability](https://github.com/rust-lang/cargo/issues/10881)

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
