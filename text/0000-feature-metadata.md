-   Feature Name: feature-metadata
-   Start Date: 2023-04-14
-   RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/3416)
-   Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary

[summary]: #summary

The main purpose of this RFC is to define a structured way to add attributes to
features, including documentation (mostly for `rustdoc`), visibility,
deprecation status, and stability.

This RFC only describes a new `Cargo.toml` schema; `rustdoc`'s handling of this
information is in a separate RFC.

# Motivation

[motivation]: #motivation

Features are widely used as a way to do things like reduce dependency count,
gate std or alloc-dependent parts of code, or hide unstable API. Use is so
common that many larger crates wind up with tens of feature gates, such as
[`tokio`] with 24. Despite being a first class component of crate structure,
there are some limitations that don't have elegant solutions:

-   Documentation is difficult, often requiring library authors to manually manage
    a table of descriptions
-   There is no way to deprecate old features, as a way to help crates maintain
    semvar compliance
-   Features cannot be hidden from use in any way

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
bar = { requires = ["foo"], doc = "simple docstring here"}
baz = { requires = ["foo"], public = false}
qux = { requires = [], deprecated = true, unstable = true }
quux = { requires = [], deprecated = { since = "1.2.3", note = "don't use this!" } }

# Features can also be full tables if descriptions are longer
[features.corge]
requires = ["bar", "baz"]
doc = """
# corge

This could be a longer description of this feature
"""
```

The following keys would be allowed in a feature table:

-   `requires`: This is synonymous with the existing array describing required
    features. For example, `foo = ["dep:serde", "otherfeat"]` will be identical to
    `foo = { requires = ["dep:serde", "otherfeat"] }`
-   `doc`: A markdown docstring describing the feature. Like with `#[doc(...)]`,
    the first line will be treated as a summary.
-   `deprecated`: This can be either a simple boolean, a string, or an object
    with `since` and/or `note` keys. Cargo will warn downstream crates using this
-   `public`: A boolean flag defaulting to `true` that indicates whether or not
    downstream crates should be allowed to use this feature
-   `unstable`: A boolean flag indicating that the feature is only usable with
    nightly Rust, or is otherwise not API-stable

If a downstream crate attempts to use the features `baz` and `qux`, they will
see messages like the following:

```
warning: feature `quux` on crate `mycrate` is deprecated since version
  1.2.3: "don't use this!"

error: feature `baz` on crate `mycrate` is private and cannot be used by
  downstream crates
```

# Reference-level explanation

[reference-level-explanation]: #reference-level-explanation

Validation and parsing of the new schema, described above, shoud be relatively
straightforward. Each of the added keys is discussed individually in the
following sections:

## `requires`

`requires` will take the place of the feature dependency array that currently
exists. Semantics will remain unchanged.

This is a required key. If there are no requirements, an empty list should be
provided (`requires = []`).

## `doc`

`doc` is the most straightforward: it accepts markdown-flavored text, and should
be thought of as the equivalent to a `#[doc(...)]` attribute. Like doc comments,
the first line should be treated as a summary.

There is nothing in this RFC that cargo `must` do with this action, since it is
mainly intended for the consumption of `rustdoc` or `docs.rs`. However, it can
be used for general diagnostic information such as during `cargo add` or
a possible `cargo info` command.

## `deprecated`

`deprecated` should be thought of as the equivalent of the [`deprecated`]
attribute in Rust source. The value can be a boolean, string, or an object with
`since` or `note` keys. Schema rules are as follows:

-   If a boolean value, `false` indicates not deprecated and `true` indicates
    deprecated
-   If an object, the keys `since` and/or `note` can be specified
-   An empty object is not allowed to avoid the ambiguous
    `foo = { deprecated = {} }`
-   A string `foo = { deprecated = "my msg" }` will be equivalent to if that string
    was specified in the `note` field:
    `foo = { deprecated = { note = "my msg" } }`
-   If not specified, the default is `false`

If a downstream crate attempts to use a feature marked `deprecated`, Cargo
should produce a warning. There are two exceptions to this:

-   If a `since` is specified that is later than the crate's current version,
    this warning should not be emitted.
-   This warning should not be emitted for crates that reexport the feature
    under a feature also marked deprecated. For example: crate `foo` exports
    feature `phooey`, and crate `bar` exports feature `barred = ["foo/phooey"]`.
    If `foo` markes `bar` as deprecated, checking `bar` will emit a warning
    unless `barred` is also marked `deprecated.

## `public`

`public` is a boolean value that defaults to `true`. It can be thought of as
`pub` in Rust source files, with the exception of being true by default. If set
to `false`, Cargo should forbid its use with an error message on any downstream
crates.

In general, marking a feature `public = false` should make tooling treat the
feature as non-public API. That includes:

-   The feature should not be accepted by `cargo add --features`
-   The feature should not be reported from the feature output report of `cargo
add`
-   In the future, `rustdoc` should not document these features unless
    `--document-private-items` is specified
-   A future tool like `cargo info` shouldn't display information about these
    features

There needs to be an escape hatch for this for things like benchmarks - RFC TBD
on how this works.

## `unstable`

`unstable` is a boolean value that defaults to `false`. It should indicate that
a feature gates something that is not API-stable, or can only be built using
`nightly` Rust.

---

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
             - unstable (U)     Some unstable features
             - deprecated (D)   Not a real feature, but it could be

    Updating crates.io index
```

Features like `aho-corasick`, `memchr`, or `use_std` would likely be `public =
false` since they aren't listed on the crate landing page.

Another thing to note is sort order. In general, any tool that renders features
(`rustdoc`, `cargo add`) should attempt to present them in the following way:

-   Display default features first
-   Display non-default but stable features next (can be in a separate section)
-   Display unstable or deprecated features last (can be in a separate section)
-   Do not display private features unless receiving a flag saying to do so (e.g.
    `--document-private-items` with `rustdoc`)
-   If ordering is not preserved, present the features alphabetically

# Drawbacks

[drawbacks]: #drawbacks

-   Added complexity to Cargo. Parsing is trivial, but exact implementation
    details do add test surface area
-   Added Cargo arguments if escape hatches for `public` or `unstable` are created
-   Docstrings can be lengthy, adding noise to `Cargo.toml`. This could
    potentially be solved with the below mentioned `doc-file` key.
-   `unstable` and `public` uses may not be common enough to be worth including
-   A markdown parser is required to properly parse the `doc` field.
-   This RFC does not provide any way for `rustdoc` to get the information it
    requires. This will require separate design work.
-   There is no way to structure features in a way that they are split into
    sections or have a user-specified layout, unlike with the
    `document-features` crate.
-   Features cannot be ordered since the TOML specification does not allow it.

# Rationale and alternatives

[rationale-and-alternatives]: #rationale-and-alternatives

-   Feature descriptions could be specified somewhere in Rust source files. This
    has the downside of creating multiple sources of truth on features.
-   Cargo could parse doc comments in `Cargo.toml`, like the `document-features`
    crate (linked below).

    ```toml
    [features]
    foo = { requires = [], doc = "foo feature" }
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

-   There is an existing crate that uses TOML comments to create a features table:
    <https://docs.rs/document-features/latest/document_features/>
-   `docs.rs` displays a feature table, but it is fairly limited. If features
    start with `_`, they are hidden from this table.
-   Ivy has a [visibility attribute] for its configuration (mentioned in [cargo #10882])

# Unresolved questions

[unresolved-questions]: #unresolved-questions

-   Do we want all the proposed keys? Specifically, `public` and `unstable` may be
    more than what is needed.

    See also:

    -   <https://github.com/rust-lang/cargo/issues/10882>
    -   <https://github.com/rust-lang/cargo/issues/10881>

-   If we use the semantics as-written, should there be a
    `--allow-private-features` or `--allow-unstable-features` flag? Or how should
    a user opt in?
-   Does it make sense to have separate `hidden` (not documented) and `public`
    (feature not allowed downstream) attribute? I think probably not
-   Should there be a way to deny deprecated features?
-   The Cargo index may need a way to be aware of deprecated features, so it can
    properly report them during resolution. What would be needed here?

It is worth noting that simpler keys (`requires`, `doc`, `deprecated`) could be
stabilized immediately and other features could be postponed.

# Future possibilities

[future-possibilities]: #future-possibilities

-   Rustdoc will gain the ability to document features. This is planned in the
    [`rustdoc-cargo-configuration`] RFC.
-   Somehow inform users if they are using to-be-deprecated features, i.e.,
    deprecated `since` is set but is later than the current dependancy version.
-   `cargo add` can show the `doc` and `deprecated` summary with the listed
    features.
-   [`cargo-info`] can use this information to provide feature descriptions.
-   `deprecated` could take a `replace-with` key that indicates features have
    moved to a different name
-   One option is to allow giving the feature documentation in a separate file,
    allowing a markdown section specifier. This could be a good option for
    features requiring long descriptions.

    ```toml
    foo = { requires = [], doc-file = "features.md#foo" }
    bar = { requires = [], doc-file = "features.md#bar" }
    ```

[`rustdoc-cargo-configuration`]: https://github.com/rust-lang/rfcs/pull/3421
[`tokio`]: https://docs.rs/crate/tokio/latest/features
[`cargo-info`]: https://github.com/rust-lang/cargo/issues/948
[visibility attribute]: https://ant.apache.org/ivy/history/latest-milestone/ivyfile/conf.html
[cargo #10882]: https://github.com/rust-lang/cargo/issues/10882
[`deprecated`]: https://doc.rust-lang.org/reference/attributes/diagnostics.html#the-deprecated-attribute
