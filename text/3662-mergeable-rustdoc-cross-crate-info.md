- Start Date: 2024-06-18
- RFC PR: [rust-lang/rfcs#3662](https://github.com/rust-lang/rfcs/pull/3662)
- Rust Issue: [rust-lang/rust#130676](https://github.com/rust-lang/rust/issues/130676)

# Summary

This RFC discusses mergeable cross-crate information in rustdoc. It facilitates the generation of documentation indexes in workspaces with many crates by allowing each crate to write to an independent output directory. The final documentation is rendered by combining these independent directories with a lightweight merge step. When provided with `--parts-out-dir`, this proposal writes a `doc.parts` directory to hold pre-merge cross-crate information. Currently, rustdoc requires global mutable access to a single output directory to generate cross-crate information, which is an obstacle to integrating rustdoc in build systems that enforce the independence of build actions.

# Motivation

The main goal of this proposal is to facilitate users producing a documentation bundle of every crate in a large environment. When a crate needs to be re-documented, only a relatively lightweight merge step will be needed to produce an updated documentation bundle. This proposal is to facilitate the creation and updating of these bundles.

This proposal also targets documenting individual crates and their dependencies in non-cargo build systems. As will be explained, doc targets in non-cargo build systems often do not support cross-crate information.

There are some files in the rustdoc output directory that are read and overwritten during every invocation of rustdoc. This proposal refers to these files as **cross-crate information**, or **CCI**, as in <https://rustc-dev-guide.rust-lang.org/rustdoc.html#multiple-runs-same-output-directory>.

Build systems may run build actions in a distributed environment across separate logical filesystems. It might also be desirable to run rustdoc in a lock-free parallel mode, where every rustdoc process writes to a disjoint set of files.

Cross-crate information is supported in Cargo. It calls rustdoc with a single `--out-dir`, which requires global read-write access to the doc root (e.g. `target/doc`). There are significant scalability issues with this approach. Global mutable access to the files that encode this cross-crate information has implications for caching, reproducible builds, and content hashing. By adding an option to avoid this mutation, rustdoc will serve as a first-class citizen in non-cargo build systems.

These considerations motivate adding an option for outputting partial CCI (parts), which are merged (linked) with a later step.

This RFC has the goal of enabling the future deprecation of the default (called `--merge=shared` here) practice of appending to cross-crate information files in the doc root.

# Guide-level explanation

## New flag summary

More details are in the Reference-level explanation.

* `--merge=none`: Do not write cross-crate information to the `--out-dir`. The flag `--parts-out-dir` may instead be provided with the destination of the current crate's cross-crate information parts.
* `--parts-out-dir=path/to/doc.parts/<crate-name>`: Write cross-crate linking information to the given directory (only usable with the `--merge=none` mode). This information allows linking the current crate's documentation with other documentation at a later rustdoc invocation.
* `--include-parts-dir=path/to/doc.parts/<crate-name>`: Include cross-crate information from this previously written `doc.parts` directories into a collection that will be written by the current invocation of rustdoc. May only be provided with `--merge=finalize`. May be provided any number of times.
* `--merge=shared` (default): Append information from the current crate to any info files found in the `--out-dir`.
* `--merge=finalize`: Write cross-crate information from the current crate and any crates included via `--include-parts-dir` to the `--out-dir`, overwriting conflicting files. This flag may be used with or without an input crate root, in which case it only links crates included via `--include-parts-dir`.

## Example

In this example, there is a crate `trait-crate` which defines a trait `Trait`, and a crate `struct-crate` which defines a struct `Struct` that implements `Trait`. Our goal in this demo is for `Struct` to appear as an implementer in `Trait`'s docs, even if `struct-crate` and `trait-crate` are documented independently.

```shell
mkdir -p trait-crate/src struct-crate/src merged/doc
echo "pub trait Trait {}" > trait-crate/src/lib.rs
echo "pub struct Struct; impl trait-crate::Trait for Struct {}" > struct-crate/src/lib.rs
MERGED=file://$(realpath merged/doc)
```

Compile `trait-crate`, so that `struct-crate` can depend on its `.rmeta` file.

```shell
rustc \
    --crate-name=trait-crate \
    --crate-type=lib \
    --edition=2021 \
    --emit=metadata \
    --out-dir=trait-crate/target \
    trait-crate/src/lib.rs
```

Document `struct-crate` and `trait-crate` independently, providing `--merge=none`, and `--parts-out-dir`.

```shell
rustdoc \
    --crate-name=trait-crate \
    --crate-type=lib \
    --edition=2021 \
    --out-dir=trait-crate/target/doc \
    --extern-html-root-url trait-crate=$MERGED \
    --merge=none \
    --parts-out-dir=trait-crate/target/doc.parts/trait-crate \
    trait-crate/src/lib.rs
rustdoc \
    --crate-name=struct-crate \
    --crate-type=lib \
    --edition=2021 \
    --out-dir=struct-crate/target/doc \
    --extern-html-root-url struct-crate=$MERGED \
    --extern-html-root-url trait-crate=$MERGED \
    --merge=none \
    --parts-out-dir=struct-crate/target/doc.parts/struct-crate \
    --extern trait-crate=trait-crate/target/libt.rmeta \
    struct-crate/src/lib.rs
```

Link everything with a final invocation of rustdoc. We will provide `--merge=finalize`, and `--include-parts-dir`. See the Reference-level explanation about these flags. Notice that this invocation is given no source input file.

```shell
rustdoc \
    --enable-index-page \
    --include-parts-dir=trait-crate/target/doc.parts/trait-crate \
    --include-parts-dir=struct-crate/target/doc.parts/struct-crate \
    --out-dir=merged/doc \
    --merge=finalize
```

Copy the docs from the given `--out-dir`s to a central location.

```shell
cp -r struct-crate/target/doc/* trait-crate/target/doc/* merged/doc
```

Browse `merged/doc/index.html` with cross-crate information.

In general, instead of two crates in the environment (`struct-crate` and `trait-crate`) a user could have thousands. Upon any changes, only the crates that change have to be re-documented.

<details>
<summary>Click here for a directory listing after running the example above.</summary>

<pre>
$ tree . -a
.
├── merged
│   └── doc
│       ├── crates.js
│       ├── help.html
│       ├── index.html
│       ├── struct-crate
│       │   ├── all.html
│       │   ├── index.html
│       │   ├── sidebar-items.js
│       │   └── struct.Struct.html
│       ├── search.desc
│       │   ├── struct-crate
│       │   │   └── struct-crate-desc-0-.js
│       │   └── trait-crate
│       │       └── trait-crate-desc-0-.js
│       ├── search-index.js
│       ├── settings.html
│       ├── src
│       │   ├── struct-crate
│       │   │   └── lib.rs.html
│       │   └── trait-crate
│       │       └── lib.rs.html
│       ├── src-files.js
│       ├── static.files
│           │   ├── COPYRIGHT-23e9bde6c69aea69.txt
│           │   ├── favicon-2c020d218678b618.svg
│           │   └── &lt;rest of the contents excluded&gt;
│       ├── trait-crate
│       │   ├── all.html
│       │   ├── index.html
│       │   ├── sidebar-items.js
│       │   └── trait.Trait.html
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
│           └── trait-crate
│               └── trait.Trait.js
├── struct-crate
│   ├── src
│   │   └── lib.rs
│   └── target
│       ├── doc
│       │   ├── help.html
│       │   ├── .lock
│       │   ├── struct-crate
│       │   │   ├── all.html
│       │   │   ├── index.html
│       │   │   ├── sidebar-items.js
│       │   │   └── struct.Struct.html
│       │   ├── search.desc
│       │   │   └── struct-crate
│       │   │       └── struct-crate-desc-0-.js
│       │   ├── settings.html
│       │   └── src
│       │       └── struct-crate
│       │           └── lib.rs.html
│       ├── doc.parts
│       │   └── struct-crate
│       │       └── crate-info
│       └── libs.rmeta
└── trait-crate
    ├── src
    │   └── lib.rs
    └── target
        ├── doc
        │   ├── help.html
        │   ├── .lock
        │   ├── search.desc
        │   │   └── trait-crate
        │   │       └── trait-crate-desc-0-.js
        │   ├── settings.html
        │   ├── src
        │   │   └── trait-crate
        │   │       └── lib.rs.html
        │   └── trait-crate
        │       ├── all.html
        │       ├── index.html
        │       ├── sidebar-items.js
        │       └── trait.Trait.html
        ├── doc.parts
        │   └── trait-crate
        │       └── crate-info
        └── libt.rmeta
</pre>

</details>

## Suggested workflows

With this proposal, there are three modes of invoking rustdoc: `--merge=shared`, `--merge=none`, and `--merge=finalize`.

### Default workflow: mutate shared directory: `--merge=shared`

In this workflow, we document a single crate, or a collection of crates into a shared output directory that is continuously updated.
Files in this output directory are modified by multiple rustdoc invocations. Use `--merge=shared`, and specify the same `--out-dir` to every invocation of rustdoc. `--merge=shared` will be the default value if `--merge` is not provided. This is the workflow that Cargo uses, and only mode of invoking rustdoc before this RFC. This RFC is intended to enable the future deprecation of this mode.

### Document crates, delaying generation of cross-crate information: `--merge=none`

Document crates using a dedicated HTML output directory and a dedicated "parts" output directory. No cross-crate data nor rendered HTML output is included from other crates.

This mode only renders the HTML item documentation for the current crate. It does not produce a search index, cross-crate trait implementations, or an index page. It is expected that users follow this mode with 'Link documentation' if these cross-crate features are desired.

In this mode, a user will provide `--parts-out-dir=<path to crate-specific directory>` and `--merge=none` to each crate's rustdoc invocation. The user should provide `--extern-html-root-url`, and specify a absolute final destination for the docs, as a URL. The `--extern-html-root-url` flag should be provided for each crate's rustdoc invocation, for every dependency.

A user may select a different `--out-dir` for each crate's rustdoc invocation.

The same `--out-dir` may also be used for multiple parallel rustdoc invocations, as rustdoc will continue to acquire an flock on the `--out-dir` to address conflicts. This is in anticipation of the possibility of deprecating `--merge=shared`, and Cargo adopting a `--merge=none` + `--merge=finalize` workflow. Cargo is expected continue using the same `--out-dir` for all crates in a workspace, as this eliminates the operations needed to merge multiple `--out-dirs`.

### Link documentation: `--merge=finalize`

In this mode, rendered HTML and *finalized* cross-crate information are generated into a `doc` folder. No *incremental* parts are generated (i.e., no `target/doc.parts/my-final-crate`).

This flag can be used with or without an target crate root. When used with a target crate, the parts for the target crate are included in the final docs. Otherwise, this mode functions merely to merge the input docs.

When a user documents the final crate, they will provide  `--include-parts-dir=<crate-specific path selected previously>` for each crate whose documentation is being combined, and `--merge=finalize`.

The user must merge every distinct `--out-dir` selected during the `--merge=none`, (e.g. `cp -r crate1/doc crate2/doc crate3/doc destination`). Most workspaces are expected to use a single `--out-dir`, so no manual merging is needed.

# Reference-level explanation

The existing cross-crate information files, like `search-index.js`, all are lists of elements, rendered in an specified way (e.g. as a JavaScript file with a JSON array or an HTML index page containing an unordered list). The current rustdoc (in `write_shared`) pushes the current crate's version of the CCI into the one that is already found in `doc`, and renders a new version. The rest of the proposal uses the term **part** to refer to the pre-merged, pre-rendered element of the CCI. This proposal does not add any new CCI or change their contents (modulo sorting order, whitespace).

## New flag: `--merge=none|shared|finalize`

This flag corresponds to the three modes of invoking rustdoc described in 'Suggested workflows'. It controls two internal paramaters: `read_rendered_cci`, and `write_rendered_cci`. It also gates whether the user is allowed to provide the `--parts-out-dir` and `--include-parts-dir` flags. It can be provided at most once.

When `write_rendered_cci` is active, rustdoc outputs the rendered parts to the doc root (`--out-dir`). Rustdoc will generate files like `doc/search-index.js`, `doc/search.desc`, `doc/index.html`, etc if and only if this parameter is true.

When `read_rendered_cci` is active, rustdoc will look in the `--out-dir` for rendered cross-crate info files. These files will be used as the base. Any new parts that rustdoc generates with its current invocation and any parts fetched with `include-parts-dir` will be appended to these base files. When it is disabled, the cross-crate info files start empty and are populated with the current crate's info and any crates fetched with `--include-parts-dir`.

* `--merge=shared` (`read_rendered_cci && write_rendered_cci`) is the default, and reflects the current behavior of rustdoc. Rustdoc will look in its `--out-dir` for pre-existing cross-crate information files, and append information to these files from the current crate. The user is not allowed to provide `--parts-out-dir` or `--include-parts-dir` in this mode.
* `--merge=none` (`!read_rendered_cci && !write_rendered_cci`) means that rustdoc will ignore the cross-crate files in the doc root. It only generates item docs. The user is optionally allowed to include `--parts-out-dir`, but not `--include-parts-dir`.
* `--merge=finalize` (`!read_rendered_cci && write_rendered_cci`) outputs crate info based only on the current crate and `--include-parts-dir`'ed crates. The user is optionally allowed to include `--include-parts-dir`, but not `--parts-out-dir`.
* A (`read_rendered_cci && !write_rendered_cci`) mode would be useless, since the data that is read would be ignored and not written.

The use of `--include-parts-dir` and `--parts-out-dir` is gated by `--merge` in order to prevent meaningless invocations, detect user error, and to provide for future changes to the interface.

## New directory: `doc.parts/`

`doc.parts` is the suggested name for the parent of the subdirectory that the user provides to `--parts-out-dir` and `--include-parts-dir`. A unique subdirectory for each crate must be provided to `--parts-out-dir` and `--include-parts-dir`. The user is encouraged to chose a directory outside of the `--out-dir`, as `--parts-out-dir` writes intermediate information that is not intended to be served on a static doc server. 

Rustdoc only guarantees that it will accept `doc.parts` files written by the same version of rustdoc. Rustdoc is the only explicitly supported consumer of `doc.parts`. In the initial implementation, rustdoc will write a file called `crate-info` as a child of the directory provided to `--parts-out-dir`, and an reasonable effort will be made for this to continue to be the structure of the subdirectory. However, the contents of `--parts-out-dir` are considered formally unstable, leaving open the possible future addition of other related files. Non-normatively, there are several pieces of information that `doc.parts` may contain:

* Partial source file index for generating `doc/src-files.js`.
* Partial search index for generating `doc/search-index.js`.
* Crate name for generating `doc/crates.js`.
* Crate name and information for generating `doc/index.html`.
* Trait implementation list for generating `doc/trait.impl/**/*.js`.
* Type implementation list for generating `doc/type.impl/**/*.js`.
* The file may include versioning information intended to assist in generating error messages if an incompatible `doc.parts` is provided through `--include-parts-dir`.
* The file may contain other information related to cross-crate information that is added in the future.

## New flag: `--parts-out-dir=path/to/doc.parts/<crate-name>`

When this flag is provided, the unmerged parts for the current crate will be written to `path/to/doc.parts/<crate-name>`. A typical argument is `./target/doc.parts/rand`.

This flag may only be used in the `--merge=none` mode. It is optional, and may be provided at most one time.

Crates `--include-parts-dir`ed will not appear in `doc.parts`, as `doc.parts` only includes the CCI parts for the current crate.

If this flag is not provided, no `doc.parts` will be written.

The output generated by this flag may be consumed by a future invocation to rustdoc that provides `--include-parts-dir=path/to/doc.parts/<crate-name>`.

## New flag: `--include-parts-dir=path/to/doc.parts/<crate-name>`

If this flag is provided, rustdoc will expect that a previous invocation of rustdoc was made with `--parts-out-dir=path/to/doc.parts/<crate-name>`. It will append the parts from the previous invocation to the ones it will render in the doc root (`--out-dir`). The info that's included is not written to its own `doc.parts`, as `doc.parts` only holds the CCI parts for the current crate.

This flag may only be used in the `--merge=finalize` mode. It is optional, and can be provided any number of times (once per crate whose documentation is merged).

In the Guide-level explanation, for example, the final invocation of rustdoc needs to identify the location of the `struct-crate`'s parts. Since they could be located in an arbitrary directory, the final invocation must be instructed on where to fetch them. In this example, the `struct-crate`'s parts happen to be in `./struct-crate/target/doc.parts/struct-crate`, so rustdoc is called with `--include-parts-dir=struct-crate/target/doc.parts/struct-crate`.

This flag is similar to `--extern-html-root-url` in that it only needs to be provided for externally documented crates. The flag `--extern-html-root-url` controls hyperlink generation. The hyperlink provided in `--extern-html-root-url` never accessed by rustdoc, and represents the final destination of the documentation. The new flag `--include-parts-dir` tells rustdoc where to search for the `doc.parts` directory at documentation-time. It must not be a URL.

## Merge step

This proposal is capable of addressing two primary use cases. It allows developers to enable CCI in these scenarios:
* Documenting a crate and its transitive dependencies in parallel in build systems that require build actions to be independent
* Producing a documentation index of every crate in a workspace, in such a way that if one crate is updated, only the updated crates and an index have to be redocumented. This scenario is demonstrated in the Guide-level explanation.

CCI is not automatically enabled in either situation. A combination of the `--include-parts-dir`, `--merge`, and `--parts-out-dir` flags are needed to produce this behavior. This RFC provides a minimal set of tools that allow developers of build systems, like Bazel and Buck2, to create rules for these scenarios. 

With separate `--out-dir`s, copying item docs to an output destination is needed. Rustdoc will never support the entire breadth of workflows needed to merge arbitrary directories, and will rely on users to run external commands like `mv`, `cp`, `rsync`, `scp`, etc. for these purposes. Most users are expected to continue to use a single `--out-dir` for all crates, in which case these external tools are not needed. It is expected that build systems with the need to be hermetic will use separate `--out-dir`s for `--merge=none`, while Cargo will continue to use the same `--out-dir` for every rustdoc invocation.

## Compatibility

This RFC does not alter previous compatibility guarantees made about the output of rustdoc. In particular it does not stabilize the presence of the rendered cross-crate information files, their content, or the HTML generated by rustdoc.

The content of `doc.parts` will be considered unstable. Between versions of rustdoc, breaking changes to the content of `doc.parts` should be expected. Only the presence of a `doc.parts` directory is promised, under `--parts-out-dir`. Merging cross-crate information generated by disparate versions of rustdoc is not supported. To detect whether `doc.parts` is compatible, rustdoc includes a version number in these files (see New directory: `doc.parts`).

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

The implementation may change the sorting order of the elements in the CCI. It does not change the content of the documentation, and is intended to work without modifying Cargo and docs.rs.

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

Currently, the Fuchsia project runs rustdoc on all of their crates to generate a [documentation index](https://fuchsia-docs.firebaseapp.com/rust/rustdoc_index/). This index is effectively generated as an [atomic step](https://cs.opensource.google/fuchsia/fuchsia/+/4eefc272d36835959f2e44be6e06a6fbb504e418:tools/devshell/contrib/lib/rust/rustdoc.py) in the build system. It takes [3 hours](https://ci.chromium.org/ui/p/fuchsia/builders/global.ci/firebase-docs/b8744777376580022225/overview) to document the ~2700 crates in the environment. With this proposal, building each crate's documentation could be done as separate build actions, which would have a number of benefits. These include parallelism, caching (avoid rebuilding docs unnecessarily), and robustness (automatically reject pull requests that break documentation).

# Unresolved questions

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

This RFC is primarily intended be followed by the deprecation of the now-default `--merge=shared` mode. This will reduce complexity in the long term. Changes to Cargo, docs.rs and other tools that directly invoke rustdoc will be required. To verify that the `--merge=none` -> `--merge=finalize` workflow is sufficient for real use cases, the deprecation of `--merge=shared` will be delayed to a future RFC.

This change could begin to facilitate trait implementations being
statically compiled as part of the .html documentation, instead of being loaded
as separate JavaScript files. Each trait implementation could be stored as an
HTML part, which are then merged into the regular documentation. Implementations of traits on type aliases should remain separate, as they serve as a [size hack](https://github.com/rust-lang/rust/pull/116471).

Another possibility is for `doc.parts` to be distributed on `docs.rs` along with the regular documentation. This would facilitate a mode where documentation of the dependencies could be downloaded externally, instead of being rebuilt locally.

The changes in this proposal are intended to work with no changes to Cargo and docs.rs. However, there may be benefits to using `--merge=finalize` with Cargo, as it would remove the need for locking the output directory. More of the documentation process could happen in parallel, which may speed up execution time.
