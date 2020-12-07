- Feature Name: sparse_index
- Start Date: 2019-10-18
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Selective download of the crates-io index over HTTP, similar to a solution used by Ruby's Bundler. Changes transport from an ahead-of-time Git clone to HTTP fetch as-needed. The existing structure and content of the index can remain unchanged. Most importantly, the proposed solution works with static files and doesn't require custom server-side APIs.

# Motivation
[motivation]: #motivation

The full crate index is relatively big and slow to download. It will keep growing as crates.io grows, making the problem worse. The requirement to download the full index slows down the first use of Cargo. It's especially slow and wasteful in stateless CI environments, which download the full index, use only a tiny fraction of it, and throw it away. Caching of the index in hosted CI environments is difficult (`.cargo` dir is large) and often not effective (e.g. upload and download of large caches in Travis CI is almost as slow as a fresh index download).

The kind of data stored in the index is not a good fit for the git protocol. The index content (as of eb037b4863) takes 176MiB as an uncompressed tarball, 16MiB with `gz -1`, and 10MiB compressed with `xz -6`. Git clone reports downloading 215MiB. That's more than just the uncompressed latest index content, and over **20 times more** than a compressed tarball.

Shallow clones or squashing of git history are only temporary solutions. Besides the fact that GitHub indicated they [don't want to support shallow clones of large repositories](http://blog.cocoapods.org/Master-Spec-Repo-Rate-Limiting-Post-Mortem/), and libgit2 doesn't support shallow clones yet, it still doesn't solve the problem that clients have to download index data for *all* crates.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Expose the index over HTTP as simple files, keeping the existing content and directory layout unchanged (similar to the existing raw.githubusercontent.com view). The current format is structured like this:

```
/config.json
/ac/ti
/ac/ti/action
/ac/ti/actiondb
/ac/ti/actions
/ac/ti/actions-toolkit-sys
/ac/ti/activation
/ac/ti/activeds-sys
…
```

To learn about crates and resolve dependencies, Cargo (or any other client) would make requests to known URLs for each dependency it needs to learn about, e.g. `https://index.example.com/se/rd/serde` (the paths are constructed and normalized the same was as for the git index). For each dependency the client would also have to request information about its dependencies, recursively, until all dependencies are fetched (and cached) locally.

It's possible to request dependency files in parallel, so the worst-case latency of such dependency resolution is limited to the maximum depth of the dependency tree. In practice it's less, because dependencies occur in multiple places in the tree, allowing earlier discovery and increasing parallelization. Additionally, if there's a lock file, all dependencies listed in it can be speculatively checked in parallel.

## Greedy fetch

To simplify the implementation, and parallelize fetches effectively, Cargo will have to fetch all dependency information before performing the actual dependency resolution algorithm. This means it'll have to pessimistically fetch information about all sub dependencies of all dependency versions that *may* match known version requirements. This won't add much overhead, because requests are per create, not per crate version. It causes additional fetches only for dependencies that were used before, but were later dropped. Fetching is still narrowed by required version ranges, so even worst cases can be avoided by bumping version requirements. For example:

* foo v1.0.1 depends on old-dep v1.0.0
* foo v1.0.2 depends on maybe-dep v1.0.2
* foo v1.0.3 depends on maybe-dep v1.0.3
* foo v1.0.4 has no dependencies

If a dependency requires `foo >=1.0.2`, then Cargo would need to fetch information about `maybe-dep` (once), even if `foo v1.0.4` ends up being selected later. However, it would not need to fetch `old-dep`. If the version requirement was upgraded to `foo >=v1.0.4` then there wouldn't be any extra fetches.

## Offline support

The proposed solution fully preserves Cargo's ability to work offline. Fetching of crates (while online) by necessity downloads enough of the index to use them, and all this data remains cached for use offline.

## Bandwidth reduction

Cargo supports HTTP/2, which handles many similar requests efficiently.

All fetched dependency files can be cached, and refreshed using conditional HTTP requests (with `Etag` or `If-Modified-Since` headers), to avoid redownloading of files that haven't changed.

Dependency files compress well. Currently the largest file of `rustc-ap-rustc_data_structures` compresses from 1MiB to 26KiB with Brotli. Many servers support transparently serving pre-compressed files (i.e. request for `/rustc-ap-rustc_data_structures` can be served from `rustc-ap-rustc_data_structures.gz` with an appropriate content encoding header), so the index can use high compression levels without increasing CPU cost of serving the files.

Even in the worst case of downloading the entire index file by file, it should still use significantly less bandwidth than git clone (individually compressed files currently add up to about 39MiB).

An "incremental changelog" file (described in "Future possibilities") could be used to avoid many conditional requests.

## Handling deleted crates

When a client checks freshness of a crate that has been deleted, it will make a request to the server and notice a 404/410/451 HTTP status. The client can then act accordingly, and clean up local data (even tarball and source checkout).

If the client is not interested in the deleted crate, it won't check it, but chances are it never did, and didn't download it. If ability to proactively erase caches of deleted crates is important, then the "incremental changelog" feature could be extended to notify about deletions.

# Drawbacks
[drawbacks]: #drawbacks

* crates-io plans to add a cryptographic signatures to the index as an extra layer of protection on top of HTTPS. Cryptographic verification of a git index is straigthforward, but signing of a sparse HTTP index may be challenging.
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

https://andre.arko.net/2014/03/28/the-new-rubygems-index-format/

Bundler used to have a full index fetched ahead of time, similar to Cargo's, until it grew too large. Then it used a centralized query API, until that became too problematic to support. Then it switched to an incrementally downloaded flat file index format similar to the solution proposed here.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

* Should the changelog use a more extensible format?
* Instead of one file that gets reset, maybe the changelog could be split into series of files (e.g. one per day or month, or a previous file ending with a filename of the next one).
* Can the changelog be compressed on the HTTP level? There are subtle differences between content encoding and transfer encoding, important for `Range` requests.
* Should freshness of files be checked with an `Etag` or `Last-Modified`? Should these be "statelessly" derived from the hash of the file or modification date in the filesystem, or explicitly stored somewhere?
* How to configure whether an index (including alternative registries) should be fetched over git or the new HTTP? The current syntax uses `https://` URLs for git-over-HTTP.

# Future possibilities
[future-possibilities]: #future-possibilities

## Incremental crate files

Bundler uses an append-only format for individual dependency files to incrementally download only new versions' information where possible. Cargo's format is almost append-only (except yanking), so if growth of individual dependency files becomes a problem, it should be possible to fix that. However, currently the largest crate `rustc-ap-rustc_data_structures` that publishes versions daily grows by about 44 bytes per version (compressed), so even after 10 years it'll take only 190KB (compressed), which doesn't seem to be terrible enough to require a solution yet.

## Incremental changelog

The scheme as described so far must double-check the contents of every index file with the server to update the index, even if many of the files have not changed. And index update happens on a `cargo update`, but can also happen for other reasons, such as when a project has no lockfile yet, or when a new dependency is added. While HTTP/2 pipelining and conditional GET requests make requesting many unchanged files [fairly efficient](https://github.com/rust-lang/cargo/pull/8890#issuecomment-737472043), it would still be better if we could avoid those extraneous requests, and instead only request index files that have truly changed.

One way to achieve this is for the index to provide a summary that lets the client quickly determine whether a given local index file is out of date. This can either come in the form of a complete "index-of-indexes" file (essentially a snapshot of the index tree), or in the form of a changelog. The former is a "large" item to fetch, since it is proportional in size to the size of the index (barring other optimizations), but may be necessary for other reasons such as whole-registry signing. Alternatively, the index could maintain an append-only log of changes. For each change (crate version published or yanked), the log would append a line with: epoch number (explained below), last-modified timestamp, and the name of the changed crate, e.g.

    1 2019-10-18T23:51:23Z oxigen
    1 2019-10-18T23:51:25Z linda
    1 2019-10-18T23:51:29Z rv
    1 2019-10-18T23:52:00Z anyhow
    1 2019-10-18T23:53:03Z build_id
    1 2019-10-18T23:56:16Z canonical-form
    1 2019-10-18T23:59:01Z cotton
    1 2019-10-19T00:01:44Z kg-utils
    1 2019-10-19T00:08:45Z serde_traitobject

Because the log is append-only, the client can incrementally update it using a `Range` HTTP request. The client doesn't have to download the full log in order to start using it; it can download only an arbitrary fraction of it, up to the end of the file, which is straightforward with a `Range` request. When a crate is found in the log (searching from the end), and modification date is the same as modification date of crate's cached locally, the client won't have to make an HTTP request for the file.

When the log grows too big, the epoch number can be incremented, and the log reset back to empty. The epoch number allows clients to detect that the log has been reset, even if the `Range` they requested happened to be valid for the new log file.

Ultimately, this RFC does not recommend such a scheme, as the changelog itself introduces [significant complexity](https://github.com/rust-lang/cargo/commit/bda120ad837e6e71edb334a44e64533119402dee) for relatively [rare gains](https://github.com/rust-lang/rfcs/pull/2789#issuecomment-738194824) that are also [fairly small in absolute value relative to a "naive" fetch](https://github.com/rust-lang/cargo/pull/8890#issuecomment-738316828). If support for index snapshots landed later for something like registry signing, the implementation of this RFC could take advantage of such a snapshot just as it could take advantage of a changelog.

## Dealing with inconsistent HTTP caches

The index does not require all files to form one cohesive snapshot. The index is updated one file at a time. Every file is updated in a separate commit, so for every file change there exists an index state that is valid with or without it. The index only needs to preserve a partial order of updates.

From Cargo's perspective dependencies are always allowed to update independently. If crate's dependencies' files are refreshed before the crate itself, it won't be different than if someone had used an older version of the crate.

The only case where stale caches can cause a problem is when a new version of a crate depends on the latest version of a newly-published dependency, and caches expired for the parent crate before expiring for the dependency. Cargo will prevent that from happening, at least for the datacenter it can see. Cargo requires dependencies with sufficient versions to be already visible in the index, and won't publish a "broken" crate.

Ideally, the server should ensure that a previous file change is visible everywhere before making the next change, i.e. make the CDN purge the changed file, and wait for the purge to be executed before updating files that may depend on it. This may be difficult to guarantee in a global CDNs, so Cargo needs a recovery mechanism:

If a crate <var>A</var> is found to depend on a crate <var>B</var> with a version that doesn't appear to exist in the index, Cargo should fetch the crate <var>B</var> again with a cache buster. The cache buster can be a query string appended to the URL with either the current timestamp, or timestamp parsed from the `last-modified` header of the crate <var>A</var>'s response: `?cachebust=12345678`.

Cache buster has an advantage over requests with `cache-control: no-cache`: it's more widely supported by CDNs, and allows the "busted" URLs to still be cached by the CDN, limiting excess traffic to the origin to 1 request per second on average.
