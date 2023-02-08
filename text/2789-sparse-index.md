- Feature Name: sparse_index
- Start Date: 2019-10-18
- RFC PR: [rust-lang/rfcs#2789](https://github.com/rust-lang/rfcs/pull/2789)
- Tracking Issue: [rust-lang/cargo#9069](https://github.com/rust-lang/cargo/issues/9069)

# Summary
[summary]: #summary

Selective download of the crates-io index over HTTP, similar to a solution used by Ruby's Bundler. Changes transport from an ahead-of-time Git clone to HTTP fetch as-needed. The existing structure and content of the index can remain unchanged. Most importantly, the proposed solution works with static files and doesn't require custom server-side APIs.

# Motivation
[motivation]: #motivation

The full crate index is relatively big and slow to download. It will keep growing as crates.io grows, making the problem worse. The requirement to download the full index slows down the first use of Cargo. It's especially slow and wasteful in stateless CI environments, which download the full index, use only a tiny fraction of it, and throw it away. Caching of the index in hosted CI environments is difficult (`.cargo` dir is large) and often not effective (e.g. upload and download of large caches in Travis CI is almost as slow as a fresh index download).

The kind of data stored in the index is not a good fit for the git protocol. The index content (as of eb037b4863) takes 176MiB as an uncompressed tarball, 16MiB with `gzip -1`, and 10MiB compressed with `xz -6`. Git clone reports downloading 215MiB. That's more than just the uncompressed latest index content, and over **20 times more** than a compressed tarball.

Shallow clones or squashing of git history are only temporary solutions. Besides the fact that GitHub indicated they [don't want to support shallow clones of large repositories](http://blog.cocoapods.org/Master-Spec-Repo-Rate-Limiting-Post-Mortem/), and libgit2 doesn't support shallow clones yet, it still doesn't solve the problem that clients have to download index data for *all* crates.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Expose the index over HTTP as plain files. It would be enough to expose the existing index layout (like the raw.githubusercontent.com view), but the URL scheme may also be simplified for the HTTP case.

To learn about crates and resolve dependencies, Cargo (or any other client) would make requests to known URLs for each dependency it needs to learn about, e.g. `https://index.example.com/se/rd/serde`. For each dependency the client would also have to request information about its dependencies, recursively, until all dependencies are fetched (and cached) locally.

It's possible to request dependency files in parallel, so the worst-case latency of such dependency resolution is limited to the maximum depth of the dependency tree. In practice it's less, because dependencies occur in multiple places in the tree, allowing earlier discovery and increasing parallelization. Additionally, if there's a lock file, all dependencies listed in it can be speculatively checked in parallel.

## Offline support

The proposed solution fully preserves Cargo's ability to work offline. Fetching of crates (while online) by necessity downloads enough of the index to use them, and all this data remains cached for use offline.

## Bandwidth reduction

Cargo supports HTTP/2, which handles many similar requests efficiently.

All fetched dependency files can be cached, and refreshed using conditional HTTP requests (with `Etag` or `If-Modified-Since` headers), to avoid redownloading of files that haven't changed.

Dependency files compress well. Currently the largest file of `rustc-ap-rustc_data_structures` compresses from 1MiB to 26KiB with Brotli. Many servers support transparently serving pre-compressed files (i.e. request for `/rustc-ap-rustc_data_structures` can be served from `rustc-ap-rustc_data_structures.gz` with an appropriate content encoding header), so the index can use high compression levels without increasing CPU cost of serving the files.

Even in the worst case of downloading the entire index file by file, it should still use significantly less bandwidth than git clone (individually compressed files currently add up to about 39MiB).

An "incremental changelog" file (described in "Future possibilities") could be used to avoid many conditional requests.

## Handling deleted crates

The proposed scheme may support deletion of crates, if necessary. When a client checks freshness of a crate that has been deleted, it will make a request to the server and notice a 404/410/451 HTTP status. The client can then act accordingly, and clean up local data (even tarball and source checkout).

If the client is not interested in the deleted crate, it won't check it, but chances are it never did, and didn't download it. If ability to proactively erase caches of deleted crates is important, then the "incremental changelog" feature could be extended to notify about deletions.

# Drawbacks
[drawbacks]: #drawbacks

* crates-io plans to add a cryptographic signatures to the index as an extra layer of protection on top of HTTPS. Cryptographic verification of a git index is straightforward, but signing of a sparse HTTP index may be challenging.
* A basic solution, without the incremental changelog, needs many requests update the index. This could have higher latency than a git fetch. However, in preliminary benchmarks it appears to be faster than a git fetch if the CDN supports enough (>60) requests in parallel. For GitHub-hosted indexes Cargo has a fast path that checks in GitHub API whether the master branch has changed. With the incremental changelog file, the same fast path can be implemented by making a conditional HTTP request for the changelog file (i.e. checking `ETag` or `Last-Modified`).
* Performant implementation of this solution depends on making many small requests in parallel. HTTP/2 support on the server makes checking twice as fast compared to HTTP/1.1, but speed over HTTP/1.1 is still reasonable.
* `raw.githubusercontent.com` is not suitable as a CDN. The sparse index will have to be cached/hosted elsewhere.
* Since the alternative registries feature is stable, the git-based index protocol is stable, and can't be removed.
* Tools that perform fuzzy search of the index (e.g. `cargo add`) may need to make multiple requests or use some other method. URLs are already normalized to lowercase, so case-insensitivity doesn't require extra requests.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Query API

An obvious alternative would be to create a web API that can be asked to perform dependency resolution server-side (i.e. take a list dependencies and return a lockfile or similar). However, this would require running dependency resolution algorithm server-side. Maintenance of a dynamic API, critical for daily use for nearly all Rust users, is much harder and more expensive than serving of static files.

The proposed solution doesn't require any custom server-side logic. The index can be hosted on a static-file CDN, and can be easily cached and mirrored by users. It's not necessary to change how the index is populated. The canonical version of the index can be kept as a git repository with the full history. This makes it easy to keep backwards compatibility with older versions of Cargo, as well as 3rd party tools that use the index in its current format.

## Initial index from rustup

Rust/Cargo installation could come bundled with an initial version of the index. This way when Cargo is run, it wouldn't have to download the full index over git, only a delta update from the seed version. The index would need to be packaged separately and intelligently handled by rustup to avoid downloading the index multiple times when upgrading or installing multiple versions of Cargo. This would make download and compression of the index much better, making current implementation usable for longer, but it wouldn't prevent the index from growing indefinitely.

The proposed solution scales much better, because Cargo needs to download and cache only a "working set" of the index, and unused/abandoned/spam crates won't cost anything.

## Rsync

The rsync protocol requires scanning and checksumming of source and destination files, which creates a lot of unnecessary I/O, and it requires SSH or a custom daemon running on the server, which limits hosting options for the index.

# Prior art
[prior-art]: #prior-art

<https://andre.arko.net/2014/03/28/the-new-rubygems-index-format/>

Bundler used to have a full index fetched ahead of time, similar to Cargo's, until it grew too large. Then it used a centralized query API, until that became too problematic to support. Then it switched to an incrementally downloaded flat file index format similar to the solution proposed here.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

* How to configure whether an index (including alternative registries) should be fetched over git or the new HTTP? The current syntax uses `https://` URLs for git-over-HTTP.
* How do we ensure that the switch to an HTTP registry does not cause a huge diff to all lock files?
* How can the current resolver be adapted to enable parallel fetching of index files? It currently requires that each index file is available synchronously, which precludes parallelism.

# Implementation feasibility

An implementation of this RFC that uses a simple "greedy" algorithm for fetching index files has been tested in https://github.com/rust-lang/cargo/pull/8890, and demonstrates good performance, especially for fresh builds. The PR for that experimental implementation also suggests a strategy for modifying the resolver to obviate the need for the greedy fetching phase.

# Future possibilities
[future-possibilities]: #future-possibilities

## Incremental crate files

Bundler uses an append-only format for individual dependency files to incrementally download only new versions' information where possible. Cargo's format is almost append-only (except yanking), so if growth of individual dependency files becomes a problem, it should be possible to fix that. However, currently the largest crate `rustc-ap-rustc_data_structures` that publishes versions daily grows by about 44 bytes per version (compressed), so even after 10 years it'll take only 190KB (compressed), which doesn't seem to be terrible enough to require a solution yet.

## Incremental changelog

The scheme as described so far must revalidate freshness of every index file with the server to update the index, even if many of the files have not changed. And index update happens on a `cargo update`, but can also happen for other reasons, such as when a project has no lockfile yet, or when a new dependency is added. While HTTP/2 pipelining and conditional GET requests make requesting many unchanged files [fairly efficient](https://github.com/rust-lang/cargo/pull/8890#issuecomment-737472043), it would still be better if we could avoid those extraneous requests, and instead only request index files that have truly changed.

One way to achieve this is for the index to provide a summary that lets the client quickly determine whether a given local index file is out of date. To spare clients from fetching a snapshot of the entire index tree, the index could maintain an append-only log of changes. For each change (crate version published or yanked), the log would append a record (a line) with: epoch number (explained below), last-modified timestamp, the name of the changed crate, and possibly other metadata if needed in the future.

Because the log is append-only, the client can incrementally update it using a `Range` HTTP request. The client doesn't have to download the full log in order to start using it; it can download only an arbitrary fraction of it, up to the end of the file, which is straightforward with a `Range` request. When a crate is found in the log (searching from the end), and modification date is the same as modification date of crate's cached locally, the client won't have to make an HTTP request for the file.

When the log grows too big, the epoch number can be incremented, and the log reset back to empty. The epoch number allows clients to detect that the log has been reset, even if the `Range` they requested happened to be valid for the new log file.

Ultimately, this RFC does not recommend such a scheme, as the changelog itself introduces [significant complexity](https://github.com/rust-lang/cargo/commit/bda120ad837e6e71edb334a44e64533119402dee) for relatively [rare gains](https://github.com/rust-lang/rfcs/pull/2789#issuecomment-738194824) that are also [fairly small in absolute value relative to a "naive" fetch](https://github.com/rust-lang/cargo/pull/8890#issuecomment-738316828). If support for index snapshots landed later for something like registry signing, the implementation of this RFC could take advantage of such a snapshot just as it could take advantage of a changelog.

## Dealing with inconsistent HTTP caches

The index does not require all files to form one cohesive snapshot. The index is updated one file at a time, and only needs to preserve a partial order of updates. From Cargo's perspective dependencies are always allowed to update independently.

The only case where stale caches can cause a problem is when a new version of a crate depends on the latest version of a newly-published dependency, and caches expired for the parent crate before expiring for the dependency. Cargo requires dependencies with sufficient versions to be already visible in the index, and won't publish a "broken" crate.

However, there's always a possibility that CDN caches will be stale or expire in a "wrong" order. If Cargo detects that its cached copy of the index is stale (i.e. it finds that a crate that depends on a dependency that doesn't appear to be in the index yet) it may recover from such situation by re-requesting files from the index with a "cache buster" (e.g. current timestamp) appended to their URL. This has an effect of reliably bypassing stale caches, even when CDNs don't honor `cache-control: no-cache` in requests.
