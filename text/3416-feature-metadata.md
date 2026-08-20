- Feature Name: feature-metadata
- Start Date: 2023-04-14
- RFC PR: [rust-lang/rfcs#3416](https://github.com/rust-lang/rfcs/pull/3416)
- Rust Issue:
  [rust-lang/cargo#14157](https://github.com/rust-lang/cargo/issues/14157)
- Amendment adding a metadata schema: 2026-08-19

## Summary

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

## Motivation

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

## Guide-level explanation

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

### General Implementation & Usage

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

## Reference-level explanation

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

### Metadata changes

Because the `features` key in metadata output does not allow for additional
fields, a `features_v2` key is introduced. It follows the following rules:

1. The type is an object with feature names as keys (same as `features`) and
   objects as values.
2. The value objects *may* contain a key `enables`, a list of strings of
   features to enable. With future RFCs, there will be additional keys.
3. If `enables` is not provided or is empty, the feature enables no other
   features.
4. If `features` and `features_v2` are both present, they *must* have identical
   feature names and enabled features. That is, the keys and values in
   `features` are the same as the keys and `<value>.enables`, respectively, in
   `features_v2`.

If a new `--format-version=2` is ever introduced, the content of `features`
will be replaced by `features_v2`.

As an example, the following features table:

```toml
[features]
foo = []
# Note that `doc` is not accepted as part of this RFC, but it is included here
# to demonstrate additional keys.
bar = { enabled = ["foo"], doc = "simple docstring for bar" }
baz = { doc = "simple docstring for baz" }
```

Will include the following in its metadata:

```json5
"features": {
    "foo": [],
    "bar": ["foo"],
    "baz": []
},
"features_v2": {
    "foo": {},
    "bar": { "enables": ["foo"], "doc": "simple docstring for bar" },
    "baz": { "doc": "simple docstring for baz" }
}
```

The following `jq` query can be used to select `features_v2` or, if not
available, to get `features` in the same format:

```jq
.features_v2 // (.features | map_values({enables: .}))
```

## Drawbacks

[drawbacks]: #drawbacks

- Added complexity to Cargo. Parsing is trivial, but exact implementation
  details do add test surface area
- Extending the `Cargo.toml` schema, particularly having a field support
  additional types, is disruptive to third-party parsers

## Rationale and alternatives

[rationale-and-alternatives]: #rationale-and-alternatives

This RFC has no impact on the Index Summaries.
Future RFCs will need to work with that.

### Naming

- `enables` reads better on the line than `enable`
- `enables` is likely an easier word for non-native speakers than `activates`
- `required` is used elsewhere to say "this should automatically be available if requirements are met"

### Schema

We could split the special feature syntax (`dep:`, etc) as distinct fields
but we'd prefer trivial conversion from the "simple" schema to the "detailed" schema,
like `dependencies`.
However, we likely would want to prefer using new fields over adding more syntax,
like with [disabling default features](https://github.com/rust-lang/cargo/issues/3126).

### Metadata

There are a handful of alternatives for metadata design:

* Instead of using `features_v2`, we could introduce metadata version 2 right
  away. A new metadata version may want to include other changes outside of this
  RFC's scope, so a way to work with existing metadata is preferred for now.
* Within `features_v2` JSON values, this RFC allows `enables` to be omitted
  if empty: it could be always emitted instead. Metadata output is typically
  machine-read so, given keys can easily be defaulted, requiring the key would
  provide little value. For example, this simple `jq` query adds `"enables": []`
  on keys where not present:

  ```jq
  `.features_v2 | map_values({enables: .enables // []})`
  ```

  This decision is forward-compatible with unconditionally emitting `enables`
  in the future (similar to accepting feature objects in `Cargo.toml` without
  `enables`), so this decision does not block always emitting the key if desired
  at some point. The reverse would not be true.
* `features_v2` could list only features that have additional metadata, relying
  on the combination of `features` and `features_v2` to fully express the
  metadata schema. Under this idea, the following could be considered valid:

  ```json5
  "features": {
      "foo": [],
      "bar": ["foo"],
  },
  "features_v2": {
      // No `"foo"` key, and no `enables` for `bar`
      "bar": { "doc": "simple docstring for bar" },
  }
  ```

  This is not done because it conflicts with the idea that `features_v2` will
  eventually replace `features`. By making `features_v2` contain all information
  that `features` does, this transition will be easier for metadata consumers,
  and the (small) complexity of object merging is avoided.

## Prior art

[prior-art]: #prior-art

## Unresolved questions

[unresolved-questions]: #unresolved-questions

## Future possibilities

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
