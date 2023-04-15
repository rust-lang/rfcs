- Feature Name: feature-metadata-doc-cfg
- Start Date: 2023-04-14
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/3416)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary

[summary]: #summary

This RFC has three simple goals:

1. Add a way to write feature descriptions in `Cargo.toml`, as well some other
   simple feature attributes.
2. Establish a way for Cargo or other build systems to easily pass advanced
   configuration information to `rustdoc` (e.g., favicon or playground URLs from
   `Cargo.toml`)
3. Allow `rustdoc` to accept this information either from the CLI or from a new
   `rustdoc.toml` file.

The outcome is that `rustdoc` will gain the ability to document cargo features
and get its configuration from `Cargo.toml`, without having any awareness of
Cargo project structure itself. There will also be room to grow for more
advanced configuration options.

# Motivation

[motivation]: #motivation

Currently, <http://docs.rs> provides a basic view of available feature flags on
a rather simple page: for example, [`tokio`]. It is helpful as a quick overview
of available features, but it is not managed by `rustdoc` (i.e., is not
available on local) and there is no way to specify a description or other useful
information.

The second problem is that `rustdoc` has some per-crate configuration settings,
such as relevant URLs, that are awkward to define in Rust source files using
attributes. It is expected that there may be further configuration options in
the future, for specifying things like:

1. Resource manifests (paths to assets, such as `KaTeX` for math rendering)
2. Non-code instructional pages (such as [`clap`'s derive information])

This RFC provides a way to solve both problems: it specifies a way to add
user-facing metadata to cargo features, and specifies how that and other
information should be passed to `rustdoc`.

[`tokio`]: https://docs.rs/crate/tokio/latest/features
[`clap`'s derive information]: https://docs.rs/clap/4.2.2/clap/_derive/index.html

# Guide-level explanation

[guide-level-explanation]: #guide-level-explanation

Usage is simple: features will be able to be specified in a table (inline or
separate) with the keys `doc`, `public`, `deprecated`, and `requires`. Sample
`Cargo.toml`:

```toml
# Cargo.toml

[features]
# current configuration will continue to work
foo = []
# Add a description to the feature. Equivalent to today's `bar = ["foo"]`
bar = { requires = ["foo"], doc = "simple docstring here"}
# `public` indicates whether or not the feature should be public in
# documentation and usable by downstream users; defaults to `true`.
baz = { requires = ["foo"], public = false, deprecated = true }

# Features can also be full tables if descriptions are longer
[features.qux]
requires = ["bar", "baz"]
doc = """
# qux

This could be a longer description of this feature
"""
```

This RFC will also enable a `[tools.rustdoc]` table where existing configuration
can be specified

```toml
# Cargo.toml

[tools.rustdoc]
html-logo-url = "https://example.com/logo.jpg"
issue-tracker-base-url = "https://github.com/rust-lang/rust/issues/"
```

For projects that do not use Cargo or want separate configuration, these options
can also be specified in a `rustdoc.toml` file using an identical schema

```toml
# rustdoc.toml containing the same information as above
# the only difference is that [tools.rustdoc] has become top level
html-logo-url = "https://example.com/logo.jpg"
issue-tracker-base-url = "https://github.com/rust-lang/rust/issues/"

[features]
foo = []
bar = { requires = ["foo"], doc = "simple docstring here" }
# ...
```

# Reference-level explanation

[reference-level-explanation]: #reference-level-explanation

At a high level, the following changes will be necessary

1. Cargo will change its parsing to accept the new format for `features`
2. `rustdoc` will gain two optional arguments: `--config-file` (for specifying
   `rustdoc.toml`-style files), and `--config-json` (for specifying the same
   information via JSON text, or a path to a JSON file). These arguments can be
   specified more than once
3. Cargo will naively serialize some information from `Cargo.toml` to pass to
   `rustdoc`, and make `rustdoc` aware of any `rustdoc.toml` files.
4. `rustdoc` will parse each of the `--config-*` arguments to create its
   internal configuration.

This is described in more detail in the following sections.

## Changes to Cargo

Cargo will need to parse the new format for `[features]`. For its internal use, it
can discard all new information.

The `cargo doc` invocation will need to do two new things:

1. Reserialize the `[features]` and `[tools.rustdoc]` tables from any
   `Cargo.toml` file to JSON. This can be naive, i.e., Cargo does not need to
   validate the contained information in any way.
2. Pass this information as a string via `--config-json`. If string length
   exceeds a limit (e.g., 2000 characters), write this configuration instead to
   a temporary build JSON file. (this also helps to avoid maximum argument
   length restrictions).
3. Find any `rustdoc.toml` files and pass their paths to `rustdoc` using
   `--config-toml`

Cargo should use the following precedence (later items take priority over
earlier items):

1. Workspace `Cargo.toml`
2. Workspace root `rustdoc.toml`
3. Crate `Cargo.toml`
4. Crate root `rustdoc.toml`

`rustdoc` will be in charge of handling configuration merging. This should
create an intuitive layering of global options and overrides while keeping
`rustdoc` and `Cargo` reasonably separate.

## Changes to `rustdoc`

`rustdoc` must be aware of two new arguments: `--config-json` and
`--config-file`. `--config-json` accepts either a JSON file path or a JSON
string, `--config-file` accepts a path to a TOML file. The JSON and TOML
share an identical schema:

```json5
{
    "html-logo-url": "https://example.com/logo.jpg",
    "issue-tracker-base-url": "https://github.com/rust-lang/rust/issues/",
    features: {
        foo: [],
        bar: { doc: "simple docstring here", requires: ["foo"] },
        baz: { public: false, requires: ["bar"] },
        qux: {
            doc: "# corge\n\nThis could be a longer description of this feature\n",
            requires: ["bar", "baz"],
        },
    },
}
```

Spans can also be specified for JSON files for better diagnostics. It is
expected that Cargo could maintain span information for data extracted from
`Cargo.toml`, but it is not required that other build systems or handwritten
configuration provide this information. This is also not required for a minimum
viable product.

```json5
{
    "_root-span-path": "/path/to/Cargo.TOML",
    "html-logo-url": {
        data: "https://example.com/logo.jpg",
        start: 100,
        end: 123,
    },
    features: {
        data: {
            foo: { data: [], start: 10, end: 15 },
            bar: {
                data: {
                    doc: { data: "simple docstring here", start: 15, end: 20 },
                    requires: { data: ["foo"], start: 20, end: 25 },
                },
                start: 15,
                end: 30,
            },
        },
        start: 10,
        end: 100,
    },
}
```

`rustdoc` should start with a default configuration and update/overwrite it with
each `--config-file` or `--config-json` argument. Configuration specified in
rust source files (e.g. `#![doc(html_favicon_url ="foo.com/favicon")]`) take the
highest priority.

# Drawbacks

[drawbacks]: #drawbacks

- This adds complexity to `rustdoc`. In this case, it does seem like it is
  justified.
- This adds noise to the Cargo manifest that is not relevant to Cargo itself.
  This RFC seeks to provide a good middle ground: overly complex configuration
  or feature descriptions can exist in a separate `rustdoc.toml`, but a separate
  file also isn't required for simple configuration.
- If a user chooses to maintain feature descriptions in `rustdoc.toml` instead
  of `Cargo.toml`, it does add multiple sources of truth for feature data.

# Rationale and alternatives

[rationale-and-alternatives]: #rationale-and-alternatives

- `rustdoc` could accept something like `--cargo-config Cargo.toml` and parse
  the `Cargo.toml` itself. This is still possible, but there are a couple
  reasons to prefer this RFC's suggestions:
  - Cargo will have the discretion for what exactly data to send. For example,
    `cargo doc --no-default-features` could strip all features that aren't
    default
  - Documentation systems other than Cargo maintain flexibility. For example,
    `doxygen` could invoke `rustdoc` and pass a favicon URL that doesn't need to
    come from `rustdoc.toml` or `Cargo.toml`.
  - Reserializing relevant sections of `Cargo.toml` is easy for Cargo to do, as
    it doesn't have to validate anything.
- JSON configuration could be skipped entirely, only using TOML. This RFC
  proposes JSON because:
  - It is easier to make CLI-safe than TOML
  - TOML->JSON serialization is trivial. `rustdoc` can also easily handle both
    schemas using the same serde structures.
  - Build systems other than Cargo can make use of it easier: plenty of tools
    are available to serialize JSON, but serializing TOML is less common (e.g.
    Python's `tomllib` can parse TOML but not recreate it)
- No information could be provided in `Cargo.toml`, only allowing
  `rustdoc.toml`. For features, keeping descriptions in `Cargo.toml` is
  preferable because it provides a single source of truth for all things feature
  related, rather than requiring that a user maintain two separate files (which
  is more or less the current status quo).
- Feature descriptions could be specified somewhere in Rust source files. Like
  the above, this has the downside of having multiple sources of truth on
  features.

# Prior art

[prior-art]: #prior-art

- There is an existing crate that uses TOML comments to create a features table:
  <https://docs.rs/document-features/latest/document_features/>
- `docs.rs` displays a feature table, but it is fairly limited
- Ivy has a [visibility attribute] for its configuration (mentioned in [cargo #10882])

[visibility attribute]: https://ant.apache.org/ivy/history/latest-milestone/ivyfile/conf.html
[cargo #10882]: https://github.com/rust-lang/cargo/issues/10882

# Unresolved questions

[unresolved-questions]: #unresolved-questions

Implementation blocking:

- Should `cargo` use the `public` attribute to disallow downstream crates from
  using features (for e.g., functions that provide unstable features or
  benchmark-only functions). Must be adopted as the same time as parsing, as
  enabling this later would break compatibility. See also:
  <https://github.com/rust-lang/cargo/issues/10882>
- If the answer to the above is "yes", does it make sense to have separate
  `hidden` (not documented) and `public` attribute (not allowed downstream)
  attribute?

Nonblocking:

- How exactly will `rustdoc` present the feature information? A new section on
  the module's top-level could be reasonable.
- Should `rustdoc` allow a header/overview for the features section? This can be
  done in the future with e.g. a `tools.rustdoc.features-doc` key in TOML.

# Future possibilities

[future-possibilities]: #future-possibilities

- Cargo could parse doc comments in `Cargo.toml`, like the above linked
  `document-features` crate. This adds some complexity to TOML parsing, but
  `rustdoc` would not need to do anything different as long as a parser could
  make the two below examples equivalent:

  ```toml
  [features]
  foo = { requires = [], doc = "foo feature" }
  ## foo feature
  foo = []
  ```
- The `deprecated` attribute would allow Cargo to warn downstream crates using
  the feature
- `unstable` or `nightly` attributes for features could provide further
  informations or restriction on use (see
  <https://github.com/rust-lang/cargo/issues/10881>)
- `[tools.rustdoc]` can grow to hold a resources manifest. For example:
  ```toml
  [tools.rustdoc]
  # cargo would likely have to canonicalize this path
  resource-root = "../static"
  
  [resources]
  # .rs files can refer to this asset as "intro-video", located at "../static/sample.mp4"
  intro-video = "sample.mp4"
  header-include = ["js/katex.js", "js/render-output-graphs.js", "mytheme.css"]
  ```
- This could set a precedent for tools receiving information from `Cargo.toml`.
  For example, the tool `cargo-foo-all-bars` could have a `[tools.foo-all-bars]`
  table in `Cargo.toml`. Upon `cargo foo-all-bars` invocation, Cargo could pass
  the contents of this tools table.
  
  The ideas in this RFC could also eventually allow `[tools.clippy]`and
  `[tools.rustfmt]` tables to simplify configuration for other builtin tools.
