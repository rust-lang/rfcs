- Feature Name: `mergable_rustdoc_cross_crate_info`
- Start Date: 2024-06-18
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary

Mergeable cross-crate information in rustdoc. Facilitates the generation of documentation indexes in workspaces with many crates by allowing each crate to write to an independent output directory. Final documentation is rendered with a lightweight merge step. Configurable with command-line flags, this proposal writes a `doc.parts` directory to hold pre-merge cross-crate information. Currently, rustdoc requires global mutable access to a single output directory to generate cross-crate information, which is an obstacle to integrating rustdoc in build systems that enforce the independence of build actions.

# Motivation

The main goal of this proposal is to facilitate users producing a documentation bundle of every crate in a large environment. When a crate needs to be re-documented, only a relatively lightweight merge step will be needed to produce an updated documentation bundle. This proposal is to facilitate the creation and updating of these bundles.

This proposal also targets documenting individual crates and their dependencies in non-cargo build systems. As will be explained, doc targets in non-cargo build systems often do not support cross-crate information.

There are some files in the rustdoc output directory that are read and overwritten during every invocation of rustdoc. This proposal refers to these files as **cross-crate information**, or **CCI**, as in <https://rustc-dev-guide.rust-lang.org/rustdoc.html#multiple-runs-same-output-directory>.

Build systems may run build actions in a distributed environment across separate logical filesystems. It might also be desirable to run rustdoc in a lock-free parallel mode, where every rustdoc process writes to a disjoint set of files.

Cargo fully supports cross-crate information, at the cost of requiring global read-write access to the doc root (`target/doc`). There are significant scalability issues with this approach.

Rustdoc needing global mutable access to the files that encode this cross-crate information has implications for caching, reproducible builds, and content hashing. By adding an option to avoid this mutation, rustdoc will serve as a first-class citizen in non-cargo build systems.

These considerations motivate adding an option for outputting partial CCI (parts), which are merged (linked) with a later step.

# Guide-level explanation

In this example, there is a crate `t` which defines a trait `T`, and a crate `s` which defines a struct `S` that implements `T`. Our goal in this demo is for `S` to appear as an implementer in `T`'s docs, even if `s` and `t` are documented independently. This guide will be assuming that we want a crate `i` that serves as our documentation index. See the Unresolved questions section for ideas that do not require an index crate.

```shell
mkdir -p t/src s/src i/src merged/doc
echo "pub trait T {}" > t/src/lib.rs
echo "pub struct S; impl t::T for S {}" > s/src/lib.rs
MERGED=file://$(realpath merged/doc)
```

[Actively use](https://doc.rust-lang.org/rustc/command-line-arguments.html#--extern-specify-where-an-external-library-is-located) `t` and `s` in `i`. The `extern crate` declarations are not needed if the crates are otherwise referenced in the index; intra-doc links are enough.

```shell
echo "extern crate t; extern crate s;" > i/src/lib.rs
```

Compile the crates.

```shell
rustc --crate-name=t --crate-type=lib --edition=2021 --emit=metadata --out-dir=t/target t/src/lib.rs
rustc --crate-name=s --crate-type=lib --edition=2021 --emit=metadata --out-dir=s/target --extern t=t/target/libt.rmeta s/src/lib.rs
```

Document `s` and `t` independently, providing `--merge=none`, `--parts-out-dir`.

```shell
rustdoc \
    -Z unstable-options \
    --crate-name=t \
    --crate-type=lib \
    --edition=2021 \
    --out-dir=t/target/doc \
    --extern-html-root-url t=$MERGED \
    --merge=none \
    --parts-out-dir=t/target/doc.parts/t \
    t/src/lib.rs
rustdoc \
    -Z unstable-options \
    --crate-name=s \
    --crate-type=lib \
    --edition=2021 \
    --out-dir=s/target/doc \
    --extern-html-root-url s=$MERGED \
    --extern-html-root-url t=$MERGED \
    --merge=none \
    --parts-out-dir=s/target/doc.parts/s \
    --extern t=t/target/libt.rmeta \
    s/src/lib.rs
```

Link everything with a final invocation of rustdoc on `i`. We will provide `--merge=finalize`, `--include-parts-dir`, and `--include-rendered-docs`. See the Reference-level explanation about these flags.

```shell
rustdoc \
    -Z unstable-options \
    --crate-name=i \
    --crate-type=lib \
    --edition=2021 \
    --enable-index-page \
    --out-dir=i/target/doc \
    --extern-html-root-url s=$MERGED \
    --extern-html-root-url t=$MERGED \
    --extern-html-root-url i=$MERGED \
    --merge=finalize \
    --include-parts-dir=t/target/doc.parts/t \
    --include-parts-dir=s/target/doc.parts/s \
    --extern t=t/target/libt.rmeta \
    --extern s=s/target/libs.rmeta \
    --include-rendered-docs=t/target/doc/t \
    --include-rendered-docs=s/target/doc/s \
    -L t/target \
    i/src/lib.rs
```

Browse `merged/doc/index.html` with cross-crate information.

In general, instead of two crates in the environment (`s` and `t`) you could have thousands. Upon any changes, only the index and the crates that are changed have to be re-documented.

<details>
<summary>Click here for a directory listing after running the example above.</summary>

<pre>
$ tree . -a
.
├── i
│   ├── src
│   │   └── lib.rs
│   └── target
│       └── doc
│           ├── crates.js
│           ├── help.html
│           ├── i
│           │   ├── all.html
│           │   ├── index.html
│           │   └── sidebar-items.js
│           ├── index.html
│           ├── .lock
│           ├── search.desc
│           │   └── i
│           │       └── i-desc-0-.js
│           ├── search-index.js
│           ├── settings.html
│           ├── src
│           │   └── i
│           │       └── lib.rs.html
│           ├── src-files.js
│           ├── static.files
│           │   ├── COPYRIGHT-23e9bde6c69aea69.txt
│           │   ├── favicon-2c020d218678b618.svg
│           │   └── <rest of the contents excluded>
│           └── trait.impl
│               ├── core
│               │   ├── marker
│               │   │   ├── trait.Freeze.js
│               │   │   ├── trait.Send.js
│               │   │   ├── trait.Sync.js
│               │   │   └── trait.Unpin.js
│               │   └── panic
│               │       └── unwind_safe
│               │           ├── trait.RefUnwindSafe.js
│               │           └── trait.UnwindSafe.js
│               └── t
│                   └── trait.T.js
├── merged
│   └── doc
│       ├── crates.js
│       ├── help.html
│       ├── i
│       │   ├── all.html
│       │   ├── index.html
│       │   └── sidebar-items.js
│       ├── index.html
│       ├── s
│       │   ├── all.html
│       │   ├── index.html
│       │   ├── sidebar-items.js
│       │   └── struct.S.html
│       ├── search.desc
│       │   ├── i
│       │   │   └── i-desc-0-.js
│       │   ├── s
│       │   │   └── s-desc-0-.js
│       │   └── t
│       │       └── t-desc-0-.js
│       ├── search-index.js
│       ├── settings.html
│       ├── src
│       │   ├── i
│       │   │   └── lib.rs.html
│       │   ├── s
│       │   │   └── lib.rs.html
│       │   └── t
│       │       └── lib.rs.html
│       ├── src-files.js
│       ├── static.files
│           │   ├── COPYRIGHT-23e9bde6c69aea69.txt
│           │   ├── favicon-2c020d218678b618.svg
│           │   └── <rest of the contents excluded>
│       ├── t
│       │   ├── all.html
│       │   ├── index.html
│       │   ├── sidebar-items.js
│       │   └── trait.T.html
│       └── trait.impl
│           ├── core
│           │   ├── marker
│           │   │   ├── trait.Freeze.js
│           │   │   ├── trait.Send.js
│           │   │   ├── trait.Sync.js
│           │   │   └── trait.Unpin.js
│           │   └── panic
│           │       └── unwind_safe
│           │           ├── trait.RefUnwindSafe.js
│           │           └── trait.UnwindSafe.js
│           └── t
│               └── trait.T.js
├── s
│   ├── src
│   │   └── lib.rs
│   └── target
│       ├── doc
│       │   ├── help.html
│       │   ├── .lock
│       │   ├── s
│       │   │   ├── all.html
│       │   │   ├── index.html
│       │   │   ├── sidebar-items.js
│       │   │   └── struct.S.html
│       │   ├── search.desc
│       │   │   └── s
│       │   │       └── s-desc-0-.js
│       │   ├── settings.html
│       │   └── src
│       │       └── s
│       │           └── lib.rs.html
│       ├── doc.parts
│       │   └── s
│       │       └── crate-info.json
│       └── libs.rmeta
└── t
    ├── src
    │   └── lib.rs
    └── target
        ├── doc
        │   ├── help.html
        │   ├── .lock
        │   ├── search.desc
        │   │   └── t
        │   │       └── t-desc-0-.js
        │   ├── settings.html
        │   ├── src
        │   │   └── t
        │   │       └── lib.rs.html
        │   └── t
        │       ├── all.html
        │       ├── index.html
        │       ├── sidebar-items.js
        │       └── trait.T.html
        ├── doc.parts
        │   └── t
        │       └── crate-info.json
        └── libt.rmeta
</pre>

</details>

## Suggested workflows

With this proposal, there are three modes of invoking rustdoc. These modes are configured through the choice of the `--merge`, `--parts-out-dir`, `--include-parts-dir`, and `--include-rendered-docs` flags.

### Default workflow: mutate shared directory

In this workflow, we document a single crate, or a collection of crates into a shared output directory that is continuously updated.
Files in this output directory are modified by multiple rustdoc invocations. Use `--merge=shared`, and specify the same `--out-dir` to every invocation of rustdoc. `--merge=shared` will be the default value if `--merge` is not provided. This is the workflow that Cargo uses, and only mode of invoking rustdoc before this RFC.

### Document intermediate crates

Document regular (non-root, non-index) crates using a dedicated HTML output directory and a dedicated "parts" output directory. No cross-crate data nor rendered HTML output is included from other crates.

This mode only renders the HTML item documentation for the current crate. It does not produce a search index, cross-crate trait implementations, or an index page. It is expected that users follow this mode with 'Document a final crate' if these cross-crate features are desired.

In this mode, a user may specify a different `--out-dir` to every invocation of rustdoc. Additionally, a user will provide `--parts-out-dir=<path to crate-specific directory>` and `--merge=none` when documenting every crate.
The user should provide `--extern-html-root-url`, and specify a absolute final location for the URL, if they document crates in separate `--out-dir`s. This flag, with the same URL, will be needed for every invocation of rustdoc, for every dependency.

### Document a final crate

In this context, a final crate is a crate that depends directly on every crate that a user intends to appear in the documentation bundle. It may be an index crate that has no meaningful functionality on its own. It may also be a library crate that depends on every crate in a workspace.

In this mode, rendered HTML and *finalized* cross-crate information are generated into a `target/doc/my-final-crate` folder. No *incremental* parts are generated (i.e., no `target/doc.parts/my-final-crate`).

When a user documents the final crate, they will provide  `--include-parts-dir=<crate-specific path selected previously>`, `--include-rendered-docs=<out dirs chosen previously>` for each one of the dependencies, and `--merge=finalize`. They will provide `--extern-html-root-url`, in the way described in 'Document an intermediate crate'.

# Reference-level explanation

The existing cross-crate information files, like `search-index.js`, all are lists of elements, rendered in an specified way (e.g. as a JavaScript file with a JSON array or an HTML index page containing an unordered list). The current rustdoc (in `write_shared`) pushes the current crate's version of the CCI into the one that is already found in `doc`, and renders a new version. The rest of the proposal uses the term **part** to refer to the pre-merged, pre-rendered element of the CCI. This proposal does not add any new CCI or change their contents (modulo sorting order, whitespace).

## New directory: `doc.parts`

`doc.parts` is a directory that holds the partial contents and destination of several cross-crate information files. It only encodes information about a single-crate. This file is written if `--parts-out-dir` is provided. The current crate's information and any `doc.parts` added through `--include-parts-dir` are merged and rendered if `--merge=shared` or `--merge=finalize` are provided.

The content of `doc.parts` is unstable. Rustdoc only guarantees that it will accept `doc.parts` files written by the same version of rustdoc, and rustdoc is the only explicitly supported consumer of `doc.parts`. Only the presence of `doc.parts` is stabilized. Non-normatively, there are several pieces of information that `doc.parts` may contain:

* Partial source file index for generating `doc/src-files.js`.
* Partial search index for generating `doc/search-index.js`.
* Crate name for generating `doc/crates.js`.
* Crate name and information for generating `doc/index.html`.
* Trait implementation list for generating `doc/trait.impl/**/*.js`.
* Type implementation list for generating `doc/type.impl/**/*.js`.
* The file may include versioning information intended to assist in generating error messages if an incompatible `doc.parts` is provided through `--include-parts-dir`.
* The file may contain other information related to cross-crate information that is added in the future.

## New flag: `--parts-out-dir=<path/to>/doc.parts/<crate-name>`

When this flag is provided, the unmerged parts for the current crate will be written to `path/to/doc.parts/<crate name>`. A typical argument is `./target/doc.parts/rand`.

Crates `--include-parts-dir`ed will not appear in `doc.parts`, as `doc.parts` only includes the CCI parts for the current crate.

If this flag is not provided, no `doc.parts` will be written.

## New flag: `--include-parts-dir=<path/to/doc.parts/crate-name>`

If this flag is provided, rustdoc will expect that a previous invocation of rustdoc was made with `--parts-out-dir=<path/to/doc.parts/crate-name>`. It will append the parts from the previous invocation to the ones it will render in the doc root (`--out-dir`). The info that's included is not written to its own `doc.parts`, as `doc.parts` only holds the CCI parts for the current crate.

This flag is similar to `--extern-html-root-url` in that it only needs to be provided for externally documented crates. The flag `--extern-html-root-url` controls hyperlink generation. The hyperlink provided in `--extern-html-root-url` never accessed by rustdoc, and represents the final destination of the documentation. The new flag `--include-parts-dir` tells rustdoc where to search for the `doc.parts` directory at documentation-time. It must not be a URL.

In the Guide-level explanation, for example, crate `i` needs to identify the location of `s`'s parts. Since they could be located in an arbitrary directory, `i` must be instructed on where to fetch them. In this example, `s`'s parts happen to be in `./s/target/doc.parts/s`, so rustdoc is called with `--include-parts-dir=s/target/doc.parts/s`.

## New flag: `--include-rendered-docs=<path/to/target/doc/extern-crate-name>`

Rustdoc will assume that `<path/to/target/doc>` was used as the `--out-dir` for `<extern-crate-name>`. This documentation will be copied into the directory specified by `--out-dir`. Rustdoc will effectively run `cp -r <path/to/target/doc/extern-crate-name> <current --out-dir>`.

## New flag: `--merge=none|shared|finalize`

This flag corresponds to the three modes of invoking rustdoc described in 'Suggested workflows'. It controls two internal paramaters: `read_rendered_cci`, and `write_rendered_cci`.

When `write_rendered_cci` is active, rustdoc will output the rendered parts to the doc root (`--out-dir`). Rustdoc will generate files like `doc/search-index.js`, `doc/search.desc`, `doc/index.html`, etc if and only if this parameter is true.

When `read_rendered_cci` is active, rustdoc will look in the `--out-dir` for rendered cross-crate info files. These files will be used as the base. Any new parts that rustdoc generates with its current invocation and any parts fetched with `include-parts-dir` will be appended to these base files. When it is disabled, the cross-crate info files start empty and are populated with the current crate's info and any crates fetched with `--include-parts-dir`.

* `--merge=shared` (`read_rendered_cci && write_rendered_cci`) is the default, and reflects the current behavior of rustdoc.
* `--merge=none` (`!read_rendered_cci && !write_rendered_cci`) means that rustdoc will ignore the cross-crate files in the doc root. Only generate item docs. 
* `--merge=finalize` (`!read_rendered_cci && write_rendered_cci`) outputs crate info based only on the current crate and `--include-parts-dir`'ed crates.
* A (`read_rendered_cci && !write_rendered_cci`) mode would be useless, since the data that is read would be ignored and not written.

## Merge step

This proposal is capable of addressing two primary use cases. It allows developers to enable CCI in these scenarios:
* Documenting a crate and its transitive dependencies in parallel in build systems that require build actions to be independent
* Producing a documentation index of a large number of crates, in such a way that if one crate is updated, only the updated crates and an index have to be redocumented. This scenario is demonstrated in the Guide-level explanation.

CCI is not automatically enabled in either situation. A combination of the `--include-parts-dir`, `--merge`, and `--parts-out-dir` flags are needed to produce this behavior. This RFC provides a minimal set of tools that allow developers of build systems, like Bazel and Buck2, to create rules for these scenarios.

Discussion of whether additional features should be included to facilitate this merge step can be found in Unresolved questions (Index crate).

## Compatibility

This RFC does not alter previous compatibility guarantees made about the output of rustdoc. In particular it does not stabilize the presence of the rendered cross-crate information files, their content, or the HTML generated by rustdoc.

In the same way that the [rustdoc HTML output is unstable](https://rust-lang.github.io/rfcs/2963-rustdoc-json.html#:~:text=The%20HTML%20output%20of%20rustdoc,into%20a%20different%20format%20impractical), the content of `doc.parts` will be considered unstable. Between versions of rustdoc, breaking changes to the content of `doc.parts` should be expected. Only the presence of a `doc.parts` directory is promised, under `--parts-out-dir`. Merging cross-crate information generated by disparate versions of rustdoc is not supported. To detect whether `doc.parts` is compatible, rustdoc includes a version number in these files (see New directory: `doc.parts`).

The implementation of the RFC itself is designed to produce only minimal changes to cross-crate info files and the HTML output of rustdoc. Exhaustively, the implementation is allowed to 
* Change the sorting order of trait implementations, type implementations, and other cross-crate info in the HTML output of rustdoc.
* Add a comment on the last line of generated HTML pages, to store metadata relevant to appending items to them.
* Refactor the JavaScript contents of cross-crate information files, in ways that do not change their overall behavior. If the JavaScript fragment declared an array called `ALL_CRATES` with certain contents, it will continue to do so.

Changes this minimal are intended to avoid breaking tools that use the output of rustdoc, like Cargo, docs.rs, and rustdoc's JavaScript frontend, in the near-term. Going forward, rustdoc will not make formal guarantees about the content of cross-crate info files.

## Note about the existing flag `--extern-html-root-url`

For the purpose of generating cross-crate links, rustdoc classifies the location of crates as external, local, or unknown (relative to the crate in the current invocation of rustdoc). Local crates are the crates that share the same `--out-dir`. External crates have documentation that could not be found in the current `--out-dir`, but otherwise have a known location. Item links are not generated to crates with an unknown location. When the `--extern-html-root-url=<crate name>=<url>` flag is provided, an otherwise unknown crate `<crate name>` becomes an externally located crate, forcing it to generate item links.

This is of relevance to this proposal, because users who document crates with separate `--out-dir`s may still expect cross-crate links to work. Currently, `--extern-html-root-url` is the exclusive command line option for specifying link destinations for crates who would otherwise have an unknown location. We will expect users to provide `--extern-html-root-url` for all direct dependencies of a crate they are documenting, if they use separate `--out-dir`s. Example usage of this flag is in the Guide-level explanation.

The limitation of `--extern-html-root-url` is that it needs to be provided with an absolute URL for the final docs destination. If your docs are hosted on `https://example.com/docs/`, this URL must be *known at documentation time*, and provided through `--extern-html-root-url=<crate name>=https://example.com/docs/`. *Absolute URLs*, instead of relative URLs, are generated for items in externally located crates. A future proposal may address this limitation by providing a command line option that generates relative URLs (like is done between items in the current crate, or other locally documented crates) for selected external crates, assuming that these crates will end up in the same bundle. The existing `--extern-html-root-url` is sufficient for the use cases envisioned by this RFC, despite the limitation.

# Drawbacks

The WIP may change the sorting order of the elements in the CCI. It does not change the content of the documentation, and is intended to work without modifying Cargo and docs.rs.

# Rationale and alternatives

Running rustdoc in parallel is essential in enabling the tool to scale to large projects. The approach implemented by Cargo is to run rustdoc in parallel by locking the CCI files. There are some workspaces where having synchronized access to the CCI is impossible. This proposal implements a reasonable approach to shared rustdoc, because it cleanly enables the addition of new kinds of CCI without changing existing documentation.

# Prior art

Prior art for linking and merging independently generated documentation was **not** identified in Javadoc, Godoc, Doxygen, Sphinx (intersphinx), nor any documentation system for other languages. Analogs of cross-crate information were not found, but a more thorough investigation or experience with other systems may be needed.

However, the issues presented here have been encountered in multiple build systems that interact with rustdoc. They limit the usefulness of rustdoc in large workspaces.

## Bazel

Bazel has `rules_rust` for building Rust targets and rustdoc documentation.

* <https://bazelbuild.github.io/rules_rust/rust_doc.html>
* <https://github.com/bazelbuild/rules_rust/blob/67b3571d7e5e341de337317d84a6bec6b9d02ed7/rust/private/rustdoc.bzl#L174>

It does not document crates' dependencies. `search-index.js`, for example, is both a dependency and an output file for rustdoc in multi-crate documentation workspaces. If it is declared as a dependency in this way, Bazel could not build docs for the members of an environment in parallel with a single output directory, as it strictly enforces hermiticity. For a recursive, parallel rustdoc to ever serve as a first-class citizen in Bazel, changes similar to the ones described in this proposal would be needed.

There is an [open issue](https://github.com/bazelbuild/rules_rust/issues/1837) raised about the fact that Bazel does not document crates dependencies. The comments in the issue discuss a pull request on Bazel that documents each crates dependencies in a separate output directory. It is noted in the discussion that this solution, being implemented under the current rustdoc, "doesn't scale well and it should be implemented in a different manner long term." In order to get CCI in a mode like this, rustdoc would need to adopt changes, like the ones in this proposal, for merging cross-crate information.

## Buck2

The Buck2 build system has rules for building and testing rust binaries and libraries. <https://buck2.build/docs/prelude/globals/#rust_library>

It has a subtarget, `[doc]`, for generating rustdoc for a crate.

You can provide `extern-html-root-url`. You can document all crates independently and manually merge them but no cross-crate information would be shared.

buck2 does not natively merge rustdoc from separate targets. The buck2 maintainers have a [proprietary search backend](https://rust-lang.zulipchat.com/#narrow/stream/266220-t-rustdoc/topic/mergable.20rustdoc.20proposal/near/445952204) that merges and parses `search-index.js` files from separately documented crates. Their proprietary tooling does not handle cross-crate trait implementations from upstream crates. By implementing this merging directly in rustdoc, we could avoid fragmentation and bring cross-crate information to more consumers.

<https://github.com/facebook/buck2/blob/d390e31632d3b4a726aaa2aaf3c9f1f63935e069/prelude/rust/build.bzl#L213>


## Ninja [(GN)](https://fuchsia.dev/fuchsia-src/development/build/build_system/intro) + Fuchsia

Currently, the Fuchsia project runs rustdoc on all of their crates to generate a [documentation index](https://fuchsia-docs.firebaseapp.com/rust/rustdoc_index/). This index is effectively generated as an [atomic step](https://cs.opensource.google/fuchsia/fuchsia/+/main:tools/devshell/contrib/lib/rust/rustdoc.py) in the build system. It takes [3 hours](https://ci.chromium.org/ui/p/fuchsia/builders/global.ci/firebase-docs/b8744777376580022225/overview) to document the ~2700 crates in the environment. With this proposal, building each crate's documentation could be done as separate build actions, which would have a number of benefits. These include parallelism, caching (avoid rebuilding docs unnecessarily), and robustness (automatically reject pull requests that break documentation).

# Unresolved questions

## Index crate?

Require users to generate documentation bundles via an index crate (current) vs. creating a new mode to allow rustdoc to run without a target crate (proposed).

If one would like to merge the documentation of several crates, we could continue to require users to provide an index crate, like [the fuchsia index](https://fuchsia-docs.firebaseapp.com/rust/rustdoc_index/). This serves as the target of the rustdoc invocation, and the landing page for the collected documentation. Supporting only this style of index would require the fewest changes. This is the mode described in the Guide-level explanation.

The proposition, to allow users of rustdoc the flexibility of not having to produce an index, is to allow rustdoc to be run in a mode where no target crate is provided. It would generate rendered cross-crate information based only on what is provided through `--include-parts-dir`. The source crates are provided to rustdoc, through a mechanism like `--extern`, rustdoc merges and writes the CCI, and copies the item and module links to the doc root. This would require more extensive changes, as rustdoc assumes that it is invoked with a target crate. This mode is somewhat analogous to the [example scraping mode](https://github.com/rust-lang/rfcs/blob/master/text/3123-rustdoc-scrape-examples.md). Having to create an index crate, that actively uses all of the crates in the environment, might prohibit the use of this feature in settings where users do not intend to produce an index, or where exhaustively listing all dependencies (to `--extern` them) is difficult.

## Unconditionally generating the `doc.parts` files?

Generate no extra files (current) vs. unconditionally creating `doc.parts` to enable more complex future CCI (should consider).

The current version of rustdoc performs merging by [collecting JSON](https://github.com/rust-lang/rust/blob/c25ac9d6cc285e57e1176dc2da6848b9d0163810/src/librustdoc/html/render/write_shared.rs#L166) blobs from the contents of the already-rendered CCI.
This proposal proposes to continue reading from the rendered cross-crate information under the default `--merge=shared`. It can also read `doc.parts` directories, under `--include-parts-dir`. However, there are several issues with reading from the rendered CCI that must be stated:
* Every rustdoc process outputs the CCI to the same doc root by default
* It is difficult to extract the items in a diverse set of rendered HTML files. This is anticipating of the CCI to include HTML files that, for example, statically include type+trait implementations directly
* Reading exclusively from `doc.parts` is simpler than the existing `serde_json` dependency for extracting the blobs, as opposed to handwritten CCI-type specific parsing (current)
* With this proposal, there will be duplicate logic to read from both `doc.parts` files and rendered CCI.

[@jsha proposes](https://github.com/rust-lang/rfcs/pull/3662#issuecomment-2184077829) unconditionally generating and reading from `doc.parts`, with no appending to the rendered crate info.

# Future possibilities

This change could begin to facilitate trait implementations being
statically compiled as part of the .html documentation, instead of being loaded
as separate JavaScript files. Each trait implementation could be stored as an
HTML part, which are then merged into the regular documentation. Implementations of traits on type aliases should remain separate, as they serve as a [size hack](https://github.com/rust-lang/rust/pull/116471).

Another possibility is for `doc.parts` to be distributed on `docs.rs` along with the regular documentation. This would facilitate a mode where documentation of the dependencies could be downloaded externally, instead of being rebuilt locally.

A future possibility related to the index crate idea is to have an option for embedding user-specified HTML into the `--enable-index-page`'s HTML.

The changes in this proposal are intended to work with no changes to Cargo and docs.rs. However, there may be benefits to using `--merge=finalize` with Cargo, as it would remove the need for locking the output directory. More of the documentation process could happen in parallel, which may speed up execution time.
