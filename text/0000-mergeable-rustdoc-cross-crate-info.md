- Feature Name: `mergable_Rustdoc_cross_crate_info`
- Start Date: 2024-06-18
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Rustdoc Proposal - Merge Documentation From Multiple Crates

# Work in progress

<https://github.com/EtomicBomb/rust/tree/master>

# Summary

Mergeable cross-crate information in Rustdoc. Facilitates the generation of documentation indexes in environments with many crates by allowing each crate to write to an independent output directory. Final documentation is generated with a lightweight merge step. Configurable with command-line flags, this proposal writes a `doc.parts` directory to hold pre-merge cross-crate information. Currently, Rustdoc requires global mutable access to a single output directory to generate cross-crate information, which is an obstacle to integrating Rustdoc in build systems that make build actions independent.

# Motivation

The main goal of this proposal is to facilitate users producing a documentation bundle of every crate in a large environment. When a crate needs to be re-documented, only a relatively lightweight merge step will be needed to produce a complete documentation bundle. This proposal is to facilitate the creation and updating of these bundles. 

There are some files in the Rustdoc output directory that are read and overwritten during every invocation of Rustdoc. This proposal refers to these files as **cross-crate information**, or **CCI**, as in <https://rustc-dev-guide.rust-lang.org/Rustdoc.html#multiple-runs-same-output-directory>. 

Build systems may run build actions in a distributed environment across separate logical filesystems. It might also be desirable to run Rustdoc in a lock-free parallel mode, where every Rustdoc process writes to a disjoint set of files.

Cargo fully supports cross-crate information, at the cost of requiring read-write access to the documentation root (`target/doc/`). There are significant scalability issues with this approach.

Rustdoc needing global mutable access to the files that encode this cross-crate information has implications for caching, reproducible builds, and content hashing. By adding an option to avoid this mutation, Rustdoc will serve as a first-class citizen in non-cargo build systems.

These considerations motivate adding an option for outputting partial CCI (parts), which are merged (linked) with a later step.


<!--
This proposal also has the goal of enabling cross-crate links for items whose documentation location is unknown at the time of documentation. If a crate is compiled without the documentation of another crate being included, the documentation location is marked unknown, and links are not generated. 
-->

# Guide-level explanation

In this example, there is a crate `t` which defines a trait `T`, and a crate `s` which defines a struct `S` that implements `T`. Our goal in this demo is for `S` to appear as in implementer in `T`'s docs, even if `s` and `t` are documented independently. This guide will be assuming that we want a crate `i` that serves as our documentation index. See the Unresolved questions section for ideas that do not require an index crate. 

```shell
mkdir -p t/src s/src i/src merged/doc
echo "pub trait T {}" > t/src/lib.rs
echo "pub struct S; impl t::T for S {}" > s/src/lib.rs
MERGED=$(realpath merged/doc)
```

[Actively use](https://doc.rust-lang.org/rustc/command-line-arguments.html#--extern-specify-where-an-external-library-is-located) `t` and `s` in `i`. The `extern crate` declarations are not needed if you reference the crates in another way in the index;  intra-doc links are enough.

```shell
echo "extern crate t; extern crate s;" > i/src/lib.rs
```

Compile the crates.

```shell
rustc --crate-name=t --crate-type=lib --edition=2021 --emit=metadata --out-dir=t/target t/src/lib.rs
rustc --crate-name=s --crate-type=lib --edition=2021 --emit=metadata --out-dir=s/target --extern t=t/target/libt.rmeta s/src/lib.rs
rustc --crate-name=i --crate-type=lib --edition=2021 --emit=metadata --out-dir=i/target --extern s=s/target/libs.rmeta --extern t=t/target/libt.rmeta -L t/target i/src/lib.rs
```

Document `s` and `t` independently, providing `--write-merged-cci=false`, `--read-merged-cci=false`, and `--parts-out-dir=<crate name>/target/doc.parts`

```shell
rustdoc -Z unstable-options --crate-name=t --crate-type=lib --edition=2021 --out-dir=t/target/doc --extern-html-root-url t=$(MERGED) --write-merged-cci=false --read-merged-cci=false --parts-out-dir=t/target/doc.parts t/src/lib.rs
rustdoc -Z unstable-options --crate-name=s --crate-type=lib --edition=2021 --out-dir=s/target/doc --extern-html-root-url s=$(MERGED) --extern-html-root-url t=$(MERGED) --write-merged-cci=false --read-merged-cci=false --parts-out-dir=t/target/doc.parts --extern t=t/target/libt.rmeta s/src/lib.rs
```

Link everything with a final invocation of Rustdoc on `i`. We will **not** provide `--write-merged-cci=false`, because we are merging the parts. We will also provide `--read-merged-cci=false` and `--fetch-parts=<path>`. See the Reference-level explanation about these flags.

```shell
rustdoc -Z unstable-options --crate-name=i --crate-type=lib --edition=2021  --enable-index-page --out-dir=i/target/doc/ --extern-html-root-url s=$(MERGED) --extern-html-root-url t=$(MERGED) --extern-html-root-url i=$(MERGED) --read-merged-cci=false --fetch-parts t=t/target/doc.parts --fetch-parts s=s/target/doc.parts --extern t=t/target/libt.rmeta --extern s=s/target/libs.rmeta -L t/target i/src/lib.rs
```

Merge the docs with `cp`. This can be avoided if `--out-dir=$(MERGED)` is used for all of the Rustdoc calls. We copy here to illustrate that documenting `s` is independent of documenting `t`, and could happen on separate machines.

```shell
cp -r s/target/doc/* t/target/doc/* i/target/doc/* merged/doc
```

Browse `merged/doc/index.html` with cross-crate information.

In general, instead of two crates in the environment (`s` and `t`) you could have thousands. Upon any changes, only the index and the crates that are changed have to be re-documented.

<details>
<summary>Click here for a directory listing after running the example above.</summary>

<pre>
TODO
</pre>
    
</details>


# Reference-level explanation

Currently, cross-crate information is written during the invocation of the `write_shared` function in [write_shared.rs](https://github.com/rust-lang/rust/blob/04ab7b2be0db3e6787f5303285c6b2ee6279868d/src/libRustdoc/html/render/write_shared.rs#L47). This proposal does not add any new CCI or change their contents (modulo sorting order, whitespace).

The existing cross-crate information files, like `search-index.js`, all are lists of elements, rendered in an specified way (e.g. as a JavaScript file with a JSON array or an HTML index page containing an unordered list). The current Rustdoc (in `write_shared`) pushes the current crate's version of the CCI into the one that is already found in `doc/`, and renders a new version. The rest of the proposal uses the term **part** to refer to the pre-merged, pre-rendered element of a the CCI.

## New subdirectory: `doc.parts/<crate name>/<cci type>`

The `doc.parts/<crate name>/<cci type>` files contain the unmerged contents of a single crates' version of their corresponding CCI. It is written if the flag `--parts-out-dir=<path>` is provided.

Every file in `doc.parts/<crate name>/*` is a JSON array. Every element of the
array is a two-element array: a destination filename (relative to `doc/`), and
the representation of the part. The representation of that part depends on the type
of CCI that it describes.

* `doc.parts/<crate name>/src-files-js`: for `doc/src-files.js`

This part is the JSON representation of the source index that is later stored in a `srcIndex` global variable.

* `doc.parts/<crate name>/search-index-js`: for `doc/search-index.js`

This part is the JSON encoded search index, before it has been installed in `search-index.js`.

* `doc.parts/<crate name>/search-desc`: for `doc/search.desc/**/*.js`

This part contains the JavaScript code to load a shard of the search descriptions.

* `doc.parts/<crate name>/all-crates`: for `doc/crates.js`, `/doc/index.html`

This part is the crate name.

* `doc.parts/<crate name>/crates-index`: for `doc/crates.js`, `doc/index.html`

This part is the also crate name. It represents a different kind of CCI because it is written to a `doc/index.html`, and rendered as an HTML document instead as JSON. In principal, we could use this part to add more information to the crates index `doc/index.html` (the first line of the top level crate documentation, for example).

* `doc.parts/<crate name>/type-impl`: for `doc/type.impl/**/*.js`

This part is a two element array with the crate name and the JSON representation of a type implementation. 

* `doc.parts/<crate name>/trait-impl`: for `doc/trait.impl/**/*.js`

This part is a two element array with the crate name and the JSON representation of a trait implementation.

## New flag: `--parts-out-dir=<path>`

When this flag is provided, the unmerged parts for the current crate will be written to `<path>/<crate name>/<cci type>`. A typical `<path>` is be `target/doc.parts`.

If this flag is not provided, no `doc.parts` will be written.

This flag is the complement to `--fetch-parts`.

## New flag: `--write-merged-cci[=true|false]`

This flag defaults to true if not specified.

With this flag is true, Rustdoc will write the rendered CCI to the output directory, on each invocation. The user-facing behavior without the flag is the same as the current behavior, with the addition of a new `doc.parts` directory.

When this flag is false, the `doc.parts` the CCI will not be written nor rendered. Another call to a merge step will be required to merge the parts and write the CCI.

## New flag: `--read-merged-cci=[=true|false]`

This flag defaults to true if not specified.

When this flag is true, Rustdoc will read the the rendered cross crate information from the doc root. Additional parts read from parts included via `--fetch-parts` will be appended to these parts.

If this flag is false, `--fetch-parts` may still be used. The parts will initialized empty.

## New flag: `--fetch-parts <crate name>=<path>`

<!--
Rustdoc considers a crate to be locally documented if its documentation appears in the current output directory. A crate is externally documented if its documentation cannot be found there. Externally documented crates may be documented online, or elsewhere in the filesystem. 

If Rustdoc [identifies](https://github.com/rust-lang/rust/blob/dd104ef16315e2387fe94e8c43eb5a66e3dbd660/src/libRustdoc/clean/types.rs#L184C7-L187C10) a crate as being documented locally, it will expect `doc.parts/<crate name>` to contain the parts. This flag is ignored in the case of locally documented crates.

-->

If this flag is provided, it will expect `<path>/<crate name>/<cci type>` to contain the parts for `<crate name>`. It will append these parts to the ones it will output.

This flag is the complement to `--parts-out-dir`

This flag is similar to `--extern-html-root-url` in that it only applies to externally documented crates. The flag `--extern-html-root-url` controls hyperlink generation. The hyperlink provided is never accessed by Rustdoc, and represents the final destination of the documentation. The new flag `--fetch-parts` tells Rustdoc where to search for the `doc.parts` directory at documentation-time. It must not be a hyperlink.

In the Guide-level explanation, for example, crate `i` needs to identify the location of `s`'s parts. Since they could be located in an arbitrary directory, `i` must be instructed on where to fetch them. In this example, `s`'s parts happen to be in `./s/target/doc.parts/s`, so Rustdoc is called with `--fetch-parts s=s/target/doc.parts`.

## Merge step

This step is provided with a list of crates. It merges their documentation. This step involves copying parts (individual item, module documentation) from each of the provided crates. It merges the parts, renders, and writes the CCI to the documentation root.

Discussion of the merge step is described in the Unresolved questions.

# Drawbacks

The WIP may change the sorting order of the elements in the CCI. It does not change the content of the documentation, and is intended to work without modifying Cargo and docs.rs.

# Rationale and alternatives

Running Rustdoc in parallel is essential in enabling the tool to scale to large projects. Cargo implements parallel Rustdoc by locking the CCI files. There are some environments where having synchronized access to the CCI is impossible. This proposal implements a reasonable approach to shared Rustdoc, because it cleanly enables the addition of new kinds of CCI without any changes to existing documentation.

# Prior art

Prior art for linking and merging independently generated documentation was **not** identified in Javadoc, Godoc, Doxygen, Sphinx (intersphinx), nor any documentation system for other languages. Analogs of cross-crate information were not found, but a more thorough investigation or experience with other systems may be needed.

However, the issues presented here have been encountered in multiple build systems that interact with Rustdoc. They limit the usefulness of Rustdoc in large environments.

## Bazel

Bazel has `rules_rust` for building Rust targets and Rustdoc documentation.

* <https://bazelbuild.github.io/rules_rust/rust_doc.html>
* <https://github.com/bazelbuild/rules_rust/blob/67b3571d7e5e341de337317d84a6bec6b9d02ed7/rust/private/Rustdoc.bzl#L174>

It does not document crates' dependencies. `search-index.js`, for example, is both a dependency and an output file for Rustdoc in multi-crate documentation environments. If it is declared as a dependency in this way, Bazel could not build docs for the members of an environment in parallel with a single output directory, as it strictly enforces hermiticity. For a recursive, parallel Rustdoc to ever serve as a first-class citizen in Bazel, changes similar to the ones described in this proposal would be needed.

There is an [open issue](https://github.com/bazelbuild/rules_rust/issues/1837) raised about the fact that Bazel does not document crates dependencies. The comments in the issue discuss a pull request on Bazel that documents each crates dependencies in a separate output directory. It is noted in the discussion that this solution, being implemented under the current Rustdoc, "doesn't scale well and it should be implemented in a different manner long term." In order to get CCI in a mode like this, Rustdoc would need to adopt changes, like the ones in this proposal, for merging cross-crate information. 

## Buck2

The Buck2 build system has rules for building and testing rust binaries and libraries. <https://buck2.build/docs/api/rules/#rust_library>


<!--

```shell
rustup install nightly-2024-03-17
cargo +nightly-2024-03-17 install --git https://github.com/facebook/buck2.git buck2
git clone https://github.com/facebook/buck2.git
cd buck2/examples/with_prelude/rust/
buck2 init --git
git commit -a
buck2 build //:library[doc] --verbose 5
```
-->

It has a subtarget, `[doc]`, for generating Rustdoc for a crate. 

You can provide a coarse-grained `extern-html-root-url` for all dependencies. You could document all crates independently, but no cross-crate information would be shared.

It does not document crates' dependencies for the same reason that Bazel does not.

<https://github.com/facebook/buck2/blob/d390e31632d3b4a726aaa2aaf3c9f1f63935e069/prelude/rust/build.bzl#L213>


## Ninja [(GN)](https://fuchsia.dev/fuchsia-src/development/build/build_system/intro) + Fuchsia

Currently, the Fuchsia project runs Rustdoc on all of their crates to generate a [documentation index](https://fuchsia-docs.firebaseapp.com/rust/Rustdoc_index/). This index is effectively generated as an [atomic step](https://cs.opensource.google/fuchsia/fuchsia/+/main:tools/devshell/contrib/lib/rust/Rustdoc.py) in the build system. It takes [3 hours](https://ci.chromium.org/ui/p/fuchsia/builders/global.ci/firebase-docs/b8744777376580022225/overview) to document the ~2700 crates in the environment. With this proposal, building each crate's documentation could be done as separate build actions, which would have a number of benefits. These include parallelism, caching (avoid rebuilding docs unnecessarily), and robustness (automatically reject pull requests that break documentation).

# Unresolved questions

## Index crate?

Require users to generate documentation bundles via an index crate (current) vs. creating a new mode to allow Rustdoc to run without a target crate (proposed)

If one would like to merge the documentation of several crates, we could continue to require users to provide an index crate, like [the fuchsia index](https://fuchsia-docs.firebaseapp.com/rust/Rustdoc_index/). This serves as the target of the Rustdoc invocation, and the landing page for the collected documentation. Supporting only this style of index would require the fewest changes. This is the mode described in the Guide-level explanation.

The proposition, to allow users of Rustdoc the flexibility of not having to produce an index, is to allow Rustdoc to be run in a mode where no target crate is provided. The source crates are provided to Rustdoc, through a mechanism like `--extern`, Rustdoc merges and writes the CCI, and copies the item and module links to `doc/`. This would require more extensive changes, as Rustdoc assumes that it is invoked with a target crate. This mode is somewhat analogous to the [example scraping mode](https://github.com/rust-lang/rfcs/blob/master/text/3123-Rustdoc-scrape-examples.md). Having to create an index crate, that actively uses all of the crates in the environment, might prohibit the use of this feature in settings where users do not intend to produce an index, or where exhaustively listing all dependencies (to `--extern` them) is difficult.

## Unconditionally generating the `doc.parts` files?

Generate no extra files (current) vs. unconditionally creating `doc.parts` to enable more complex future CCI (should consider)

The current version of Rustdoc performs merging by [collecting JSON](https://github.com/rust-lang/rust/blob/c25ac9d6cc285e57e1176dc2da6848b9d0163810/src/libRustdoc/html/render/write_shared.rs#L166) blobs from the contents of the already-rendered CCI. 
This proposal proposes to continue reading from the rendered cross-crate information under the default `--read-merged-cci=true`. It can also read `doc.parts` files, under `--fetch-parts`. However, there are several issues with reading from the rendered CCI that must be stated:
* If a user has a single `doc/` output directory, it is impossible to avoid shared mutation if every Rustdoc process is writing to the same CCI
* It is difficult to extract the items in a diverse set of rendered HTML files. This is anticipating of the CCI to include HTML files that, for example, statically include type+trait implementations directly
* Reading exclusively from `doc.parts` is simpler than the existing `serde_json` dependency for extracting the blobs, as opposed to handwritten CCI-type specific parsing (current)
* With this proposal, there will be duplicate logic to read from both `doc.parts` files and rendered CCI. 

## Item links?

Require users to pass `--extern-html-root-url` on all external dependencies (current), vs. add a new flag to facilitate missing docs links being generated across all user crates (should consider)

Cross-crate links are an important consideration when merging documentation. Rustdoc provides a flag, `--extern-html-root-url`, which can provide fine-grain control over the generated URL for documentation. We believe this is already sufficient for generating cross-crate links, and we will make no changes to the generation of item links. Example usage of this flag is in the Guide-level explanation.

A proposal is to provide more options to facilitate cross-crate links. For example, we could add a flag that implies `--extern-html-root-url` and `--fetch-parts` on all crates passed through `--extern`. It could be something like `--extern-html-root-url-all-user-crates`. User crates would be taken to mean the transitive dependencies of the current crate, excluding the standard library. This is complicated by things like <https://doc.rust-lang.org/cargo/reference/unstable.html#build-std>.
    
## Reuse existing option?

Create a new flag, `--write-merged-cci` (proposed), vs. use existing option `no_emit_shared` 
    
There is a render option, `no_emit_shared`, which is used to conditionally generate the cross-crate information. It also controls the generation of static files, user CSS, etc.

This option is not configurable from the command line, and appears to be enabled unless Rustdoc is run in its example scraping mode. 

We could make it configurable from the command line, unconditionally generate `doc.parts/`, and use it to gate the merging of CCI.

We could also make it configurable from the command line, and use it to gate the generation of `doc.parts/` and the generation of all of the shared files.

We could also leave it as-is: always false unless we're scraping examples, and gate the generation of `doc.parts/` and the generation of all of the shared files.

<!--
This proposal has the goal of enabling cross-crate links when the documentation location is unknown. This aspect of the proposal has not been investigated as deeply as the CCI merge step.

One approach is to add a flag to Rustdoc: `--assume-doc-location <crate name>=<doc root>`. Unlike `--extern-html-root-url`, Rustdoc will assume that the docs for `<crate name>` have not been generated yet. It will still generate links to those items at `<doc root>`, assuming that they will be generated later. This approach has the problem of possibly enabling Rustdoc to generate hanging links. 

Another approach that has been considered is to add a flag like `--item-location-unknown-to-404`. This flag will generate a `doc/404.html` page. When an item has an unknown location, instead of generating no link, Rustdoc will link to `doc/404.html`, and include the item name in a `data-hanging-item` attribute. For example, if the standard library docs have not been generated, and we are linking to `Entry`, Rustdoc will generate a link like `<a href="doc/404.html" data-hanging-item="std/collections/hash_map/enum.Entry.html">Entry</a>`. During the merge step, Rustdoc would recursively identify all documented items and their locations, find hanging item links, and replace them with links referencing their identified location: `<a href="std/collections/hash_map/enum.Entry.html">Entry</a>`.
-->

# Future possibilities

This change could begin to facilitate type aliases and the trait aliases being
statically compiled as part of the .html documentation, instead of being loaded
as separate JavaScript files. Each type and trait alias could be stored as an
HTML part, which are then merged into the regular documentation.

Another possibility is for `doc.parts` to be distributed on `docs.rs` along with the regular documentation. This would facilitate a mode where documentation of the dependencies could be downloaded externally, instead of being rebuilt locally. 

A future possibility related to the index crate idea is to have an option for embedding user-specified HTML into the `--enable-index-page`'s HTML.

The changes in this proposal are intended to work with no changes to Cargo and docs.rs. However, there may be benefits to using `--write-merged-cci=false` with Cargo, as it would remove the need for locking the output directory. More of the documentation process could happen in parallel, which may speed up execution time..
