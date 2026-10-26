-   Feature Name: rustdoc-cargo-configuration
-   Start Date: 2023-04-19
-   RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/3421)
-   Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary

[summary]: #summary

The goal of this RFC is to define a way for `rustdoc` to (1) get configuration
information located in `Cargo.toml` without parsing the file itself, and (2)
establish a `rustdoc` configuration file format. The result is that `rustdoc`
will gain knowledge of Cargo features and feature descriptions, and be able to
present this data in crate documentation. Additionally, `rustdoc` will have room
to grow with its configuration, allowing for things like an asset manifest in
the future.

While this RFC mainly targets `rustdoc`, it is also a partial goal to produce
Cargo interfacing guidelines that may work for other default tooling (`rustfmt`,
`clippy`) and, possibly, community tooling at some point in the future.

# Motivation

[motivation]: #motivation

Currently, `docs.rs` provides a basic view of available feature flags on a
rather simple page (for example, [`tokio`]). It is helpful as a quick overview
of available features, but it is not managed by `rustdoc` (so not available on
local) and there is no way to specify a description or other useful information.
The [`feature-metadata`] RFC will provide a way to add documentation to features
within `Cargo.toml`. `rustdoc` should present this information, but it needs a
way to consume it from the manifest file.

The second problem is that `rustdoc` has some per-crate configuration settings,
such as relevant URLs, that are awkward to define in Rust source files using
attributes. It is expected that there may be further configuration options in
the future, for specifying things like:

1. Resource manifests (paths to assets, such as `KaTeX` for math rendering or
   paths to non-image files)
2. Data required to create non-code informational pages (such as [`clap`'s
   derive information])

This RFC provides a way to solve both problems: it specifies a way that
`rustdoc` can gain an extensible configuration, as well as a way for it to
receive data directly from Cargo.

# Guide-level explanation

[guide-level-explanation]: #guide-level-explanation

Through the [`feature-metadata`] RFC, it will be possible for a crate author to
specify documentation, stability, and deprecation status for each feature in
`Cargo.toml`. This will be very useful information for `rustdoc` to have, but
there would be disadvantages to `rustdoc` extracting that information directly
from the manifest file.

The proposal is to allow Cargo to handle the parsing of the manifest and simply
reserialize the necessary portions of the data to JSON, which can then be
consumed by `rustdoc`.

The second goal of this RFC is to allow specifying `rustdoc` crate-level
configuration data either within a new `Cargo.toml` `[tools.rustdoc]` table, or
in a new `rustdoc.toml` file, rather than needing to be specified within the
Rust source files itself. Initially supported keys will be kebab case versions
of the [`rustdoc` crate-level configuration]. For example, a `Cargo.toml` file
could contain the following:

```toml
[tools.rustdoc]
html-logo-url = "https://example.com/logo.jpg"
issue-tracker-base-url = "https://github.com/rust-lang/rust/issues/"
```

Perhaps most importantly, this can be specified in the workspace `Cargo.toml`
and be used by each member crate.

For projects that do not use Cargo or want separate configuration, these options
can also be specified in a `rustdoc.toml` file using an identical schema:

```toml
# rustdoc.toml containing the same information as above
# the only difference is that [tools.rustdoc] has become top level
html-logo-url = "https://example.com/logo.jpg"
issue-tracker-base-url = "https://github.com/rust-lang/rust/issues/"
```

# Reference-level explanation

[reference-level-explanation]: #reference-level-explanation

At a high level, the following changes will be necessary:

1. Cargo will accept a new `[tools.rustdoc]` table that it does not validate
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

For the `cargo doc` invocation, Cargo will need to do two new things:

1. Reserialize the `[features]` and `[tools.rustdoc]` tables from any
   `Cargo.toml` file to JSON. This can be naive, i.e., Cargo does not need to
   validate the contained information in any way.
2. Pass this information as a string via `--config-json`. If string length
   exceeds a limit (e.g., 1000 characters), write this configuration instead to
   a temporary build JSON file, pass that path via the same argument. (This
   avoids maximum argument length restrictions, and keeps verbose output
   concise).
3. Find files named `rustdoc.toml` and pass their paths to `rustdoc` using
   `--config-toml`; no parsing of these files is necessary.

Cargo should send arguments with the following precedence (later items take
priority over earlier items):

1. Workspace `Cargo.toml`
2. Workspace root `rustdoc.toml` (located next to the workspace manifest)
3. Crate `Cargo.toml`
4. Crate root `rustdoc.toml` (located next to the crate manifest)

`rustdoc` will be in charge of handling configuration merging. This should
create an intuitive layering of global options and overrides while keeping
`rustdoc` and `Cargo` reasonably separate. The exact rules for this are
described in the "Extensible Configuration" section.

## Changes to `rustdoc`

`rustdoc` must be aware of two new arguments: `--config-json` and
`--config-file`, both of which can be repeated. `--config-json` accepts either a
JSON file path or a JSON string, `--config-file` accepts a path to a TOML file.
The JSON and TOML share an identical schema to what is shown above:

```json5
{
    "_invoked-by": "cargo", // build tool
    // Schemas for these keys are defined per rustdoc's needs
    "html-logo-url": "https://example.com/logo.jpg",
    "issue-tracker-base-url": "https://github.com/rust-lang/rust/issues/",
    // Features section not shown above, but would exactly match Cargo schema
    "features": {
        "foo": [],
        "bar": { "doc": "simple docstring here", "requires": ["foo"] },
        "baz": { "public": false, "requires": ["bar"], "deprecated": true },
    },
}
```

If using JSON, spans can be specified for better diagnostics. It is expected
that Cargo could maintain span information for data extracted from `Cargo.toml`,
but it is not required that other build systems or handwritten
configuration provide this information. This is also not required for a minimum
viable product.

```json5
{
    "_invoked-by": "cargo",
    // Indicate the source file that spans are based on
    "_root-span-path": "/path/to/Cargo.TOML",
    // Spans move value to a "data" key in a new object, and specify
    // start and end byte offsets
    "html-logo-url": {
        "data": "https://example.com/logo.jpg",
        "start": 100,
        "end": 123,
    },
    "features": {
        "data": {
            "foo": { "data": [], "start": 10, "end": 15 },
            "bar": {
                "data": {
                    "doc": {
                        "data": "simple docstring here",
                        "start": 15,
                        "end": 20,
                    },
                    "requires": { "data": ["foo"], "start": 20, "end": 25 },
                },
                "start": 15,
                "end": 30,
            },
        },
        "start": 10,
        "end": 100,
    },
}
```

`rustdoc` should start with a default configuration and update/overwrite it with
each `--config-file` or `--config-json` argument. Configuration specified in
rust source files (e.g. `#![doc(html_favicon_url ="foo.com/favicon")]`) take the
highest priority.

## Extensible Configuration

The design in this RFC is such that any tool, official or not, could specify a
simple schema that tells the build system how to invoke that tool. It is also
designed with build systems other than `Cargo` in consideration, since there is
a growing number of C/C++ projects that are adding some Rust, and likely want to
make use of `rustdoc`.

It is possible that this design is never made public so no other tools use what
is described below, but this section seeks to make the design principles clear.

That being said, the described interface is based upon being describable by a
simple schema:

```json5
{   // sample config for rustdoc
    "tool": "rustdoc",
    "manifest-tables": ["features"],
    "config-json-arg": "--config-json",
    "search-files": ["rustdoc.toml"],
    "config-file-arg": "--config-file",
    "accepts-spanned-json": true
},
{   // sample clippy config.
    "tool": "clippy",
    "cargo-tables": ["lints"],
    "search-files": ["clippy.toml", ".clippy.toml"]
    // ...
}
// ... any tools just needs to provide a similar schema
```

The idea is that a build system could store this configuration for `rustdoc`,
`rustfmt`, `clippy`, or some other tool, and know exactly how to invoke it. A
package on `crates.io` could potentially include this information as part of its
`Cargo.toml`, and Cargo would be able to invoke it with custom configuration.

The schema is as follows:

-   `tool`: name of the tool, which can be configured by a `[tools.toolname]`
    table. Contents of `[tools.toolname]` will be passed as top-level JSON keys.
-   `manifest-tables`: tables other than `[tool.toolname]` that should be passed
-   `config-json-arg`: the CLI flag to pass JSON from Cargo or other build systems.
    Must accept either a JSON string or a path to a JSON file.
-   `config-files`: Files that Cargo should search for. Path is relative to a
    `Cargo.toml` file; accepts relative paths and globs.
-   `config-file-arg`: Argument to pass the paths of found files
-   `accepts-spanned-json`: Whether or not Cargo should include spans when
    serializing from a TOML file.

A tool must follow the following rules:

-   `config-json-arg` must accept either a JSON string or a file path
-   `config-json-arg` and `config-file-arg` values must be able to be specified
    more than once. A tool is responsible for merging these configurations
-   When receiving multiple `config-json-arg` or `config-file-arg` arguments,
    the first has lowest precedence and the last has the highest precedence.
    That is, configuration specified by a later argument should overwrite
    configuration from an earlier argument.

Cargo, in turn, must follow the following process:

1. Serialize `[tool.toolname]` and any requested `manifest-tables` from the
   workspace `Cargo.toml`. Retain spans as specified by `accepts-spanned-json`.
   Pass this information via the `config-json-arg` argument.
2. Search at the workspace root for any paths matching `config-files`. Pass
   these paths via `config-file-arg`. Order is not important.
3. Repeat step 1 at crate root
4. Repeat step 2 at crate root

# Drawbacks

[drawbacks]: #drawbacks

-   This adds complexity to both Cargo and `rustdoc`: additional flags for
    `rustdoc`, additional behavior for both.
-   Obtaining information from `Cargo.toml`, such as feature descriptions, does
    bring `rustdoc` and Cargo closer together when there have been advantages to
    keeping the tools separate. This RFC attempts to mitigate this risk by not
    locking the configuration to Cargo in any way, but it is still a concern.

# Rationale and alternatives

[rationale-and-alternatives]: #rationale-and-alternatives

-   `rustdoc` could accept something like `--cargo-config Cargo.toml` and parse
    the `Cargo.toml` itself. This is possible, but there are a couple reasons to
    prefer this RFC's suggestions:

    -   Cargo will have the discretion for what exactly data to send. For example,
        `cargo doc --no-default-features` could strip all features that aren't
        default, without `rustdoc` needing to be aware of the argument.
    -   Documentation systems other than Cargo maintain flexibility. For example,
        `doxygen` could invoke `rustdoc` and pass a favicon URL that doesn't need to
        come from `rustdoc.toml` or `Cargo.toml`.
    -   Reserializing relevant sections of `Cargo.toml` is easy for Cargo to do, as
        it doesn't have to validate anything.

-   JSON configuration could be skipped entirely, only using TOML. This RFC
    proposes JSON because:

    -   It is easier to make CLI-safe than TOML
    -   TOML->JSON serialization is trivial. `rustdoc` can also easily handle both
        schemas using the same serde structures.
    -   Using JSON means that spans from the initial `TOML` file can be encoded
    -   Build systems other than Cargo can make use of it easier: plenty of tools
        are available to serialize JSON, but serializing TOML is less common (e.g.
        Python's `tomllib` can parse TOML but not recreate it)

-   `[tools.rustdoc]` and `rustdoc.toml` could be skipped entirely, and this RFC
    could focus only on feature descriptions. That is still possible; this RFC
    provides guidelines here because:

    -   Some information from `Cargo.toml` is needed for feature descriptions, so
        it makes sense to design a coherent interface at the same time
    -   Having a way to specify `rustdoc` configuration in workspace root has
        immediate usability for repositories that have multiple crates

-   Explicit workspace inheritance ([`workspace.key` style]) could be used
    instead of the implicit inheritance specified here:

    -   If explicit inheritance is used, either Cargo must resolve the values, or
        the tool must be somewhat aware of workspaces. With implicit inheritance,
        the tool only needs to know how to merge configuration with a specified
        precedence
    -   It is likely that with a lot of `rustdoc` config, users would expect
        inheritance to happen by default; all crates in a repository will likely
        share the same favicon URL. Compared to `version.workspace`, where it
        would be likely that different crates have different versions.

    This is still not precluded, and is also indicated in the unresolved
    questions section.

-   No information could be provided in `Cargo.toml`, only allowing
    `rustdoc.toml`. One of the mild annoyances for users is winding up with
    `rustfmt.toml` and `clippy.toml` files that each have <5 lines each. Those
    tools are currently in the process of figuring out how to also allow
    specification via `Cargo.toml`: making the design choice now skipps annoyance
    down the line.

# Prior art

[prior-art]: #prior-art

-   There is an existing crate that uses TOML comments to create a features table:
    <https://docs.rs/document-features/latest/document_features/>
-   `docs.rs` displays a feature table, but it is fairly limited
-   There are RFCs related to Clippy obtaining information from `Cargo.toml`
    <https://github.com/rust-lang/rfcs/pull/3389>

# Unresolved questions

[unresolved-questions]: #unresolved-questions

Blocking:

-   Should inheritance be handled at all? Explicit vs. implicit? (See rationale
    section)
-   Should Cargo search for files? This was chosen because it is fairly
    straightforward for Cargo to do if there are strict rules, but it could be
    easier to just pass a `--config-search-path` for workspace and crate roots.

Nonblocking:

-   How exactly will `rustdoc` present the feature information? A new section on
    the module's top-level could be reasonable.

    This RFC does not intend to determine the exact user-facing output of
    `rustdoc` with feature information. However, it is expected that the
    rendered output will accomplish the following:

    -   Render all feature flags not marked `public = false` in a table or
        sectional format. Make sure this is on the main page or can be directly
        navigated to from the main page.
    -   Allow linking to feature flags directly (HTML anchors)
    -   Indicate deprecated and unstable feature flags in some way
    -   Treat the first line of the description as a summary and the rest as
        body text (similar to how documenting other items currently works)

-   Should `rustdoc` allow a header/overview for the features section? This can be
    done in the future with e.g. a `tools.rustdoc.features-doc` key in TOML.

# Future possibilities

[future-possibilities]: #future-possibilities

-   `[tools.rustdoc]` can grow to hold a resources manifest. For example:

    ```toml
    [tools.rustdoc]
    # cargo would likely have to canonicalize this path
    resource-root = "../static"

    [tools.rustdoc.resources]
    # .rs files can refer to this asset as "code-vid", located at "../static/demo.mp4"
    code-vid = "demo.mp4"
    header-include = ["js/katex.js", "js/render-output-graphs.js", "mytheme.css"]
    ```

-   This could set a precedent for tools receiving information from
    `Cargo.toml`. For example, the tool `cargo-foo-all-bars` could provide the
    schema from the "Extensible Configuration" section in its `Cargo.toml`, then
    receive the contents of `[tools.foo-all-bars]` when invoked with Cargo.

    The ideas in this RFC could also eventually allow `[tools.clippy]`and
    `[tools.rustfmt]` tables to simplify configuration of those tools.

[`tokio`]: https://docs.rs/crate/tokio/latest/features
[`feature-metadata`]: https://github.com/rust-lang/rfcs/pull/3416
[`clap`'s derive information]: https://docs.rs/clap/4.2.2/clap/_derive/index.html
[`rustdoc` crate-level configuration]: https://doc.rust-lang.org/rustdoc/write-documentation/the-doc-attribute.html#at-the-crate-level
[`workspace.key` style]: https://doc.rust-lang.org/cargo/reference/workspaces.html#the-dependencies-table
