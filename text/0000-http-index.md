- Feature Name: http_index
- Start Date: 2019-10-18
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Selective download of the crates-io index over HTTP, similar to a solution used by Ruby's Bundler. Changes transport from an ahead-of-time Git clone to HTTP fetch as-needed, while keeping existing content and structure of the index. Most importantly, the proposed solution works with static files and doesn't require custom server-side APIs.

# Motivation
[motivation]: #motivation

The full crate index is relatively big and slow to download. It will keep growing as crates.io grows, making the problem worse. The requirement to download the full index slows down the first use of Cargo. It's especially slow and wasteful in stateless CI environments, which download the full index, use only a tiny fraction of it, and throw it away. Caching of the index in hosted CI environments is difficult (`.cargo` dir is large) and often not effective (e.g. upload and download of large caches in Travis CI is almost as slow as a fresh index download).

The kind of data stored in the index is not a good fit for the git protocol. The index content (as of eb037b4863) takes 176MiB as an uncompressed tarball, 16MiB with `gz -1`, and 10MiB compressed with `xz -6`. Git clone reports downloading 215MiB. That's more than just the uncompressed latest index content, and over **20 times more** than a compressed tarball.

A while ago, GitHub indicated they [don't want to support shallow clones of large repositories](http://blog.cocoapods.org/Master-Spec-Repo-Rate-Limiting-Post-Mortem/). libgit2 doesn't support shallow clones yet. Squashing of the index history adds complexity to management and consumption of the index (which is also used by tools other than Cargo), and still doesn't solve problems of the git protocol inefficiency and overall growth.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Expose the index over HTTP as simple files, keeping the existing content and directory layout unchanged (the existing raw.githubusercontent.com view may even be enough for this). The current format is structured like this:

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

To learn about crates and resolve dependencies, Cargo (or any other client) would make requests to known URLs for each dependency it needs to learn about, e.g. `https://index.example.com/se/rd/serde`. For each dependency the client would also have to request information about its dependencies, recursively, until all dependencies are fetched (and cached) locally.

It's possible to request dependency files in parallel, so the worst-case latency of such dependency resolution is limited to the maximum depth of the dependency tree. In practice it's less, because dependencies occur in multiple places in the tree, allowing earlier discovery and increasing parallelization. Additionally, if there's a lock file, all dependencies listed in it can be speculatively checked in parallel. Similarly, cached dependency files can be used to speculatively check known sub-dependencies sooner.

## Greedy fetch

To simplify the implementation, and parallelize fetches effectively, Cargo will have to fetch all dependency information before performing the actual dependency resolution algorithm. This means it'll have to pessimistically fetch information about all sub dependencies of all dependency versions that *may* match known version requirements. This won't add much overhead, because requests are per create, not per crate version. It causes additional fetches only for dependencies that were used before, but were later dropped. Fetching is still narrowed by required version ranges, so even worst cases can be avoided by bumping version requirements. For example:

* foo v1.0.1 depends on old-dep v1.0.0
* foo v1.0.2 depends on maybe-dep v1.0.2
* foo v1.0.3 depends on maybe-dep v1.0.3
* foo v1.0.4 has no dependencies

If a dependency requires `foo >=1.0.2`, then Cargo would need to fetch information about `maybe-dep` (once), even if `foo v1.0.4` ends up being selected later. However, it would not need to fetch `old-dep`. If the version requirement was upgraded to `foo >=v1.0.4` then there wouldn't be any extra fetches.

## Offline support

The proposed solution fully preserves Cargo's ability to work offline. Fetching of crates while online by necessity downloads enough of the index to use them, and all this data remains cached for use offline.

## Bandwidth reduction

Cargo supports HTTP/2, which handles many similar requests efficiently.

All fetched dependency files can be cached, and refreshed using conditional HTTP requests (with `Etag` or `If-Modified-Since` headers), to avoid redownloading of files that haven't changed.

Dependency files compress well. Currently the largest file of `rustc-ap-rustc_data_structures` compresses from 1MiB to 26KiB with Brotli. Many servers support transparently serving pre-compressed files (i.e. request for `/rustc-ap-rustc_data_structures` can be served from `rustc-ap-rustc_data_structures.gz` with an appropriate content encoding header), so the index can use high compression levels without increasing CPU cost of serving the files.

Even in the worst case of downloading the entire index file by file, it should still use significantly less bandwidth than git clone (individually compressed files add up to about 39MiB).

## Optionally, a rotated incremental changelog

To further reduce number requests needed to update the index, the index may maintain an append-only log of changes. For each change (crate version published or yanked), the log would append a line with: epoch number (explained below), last-modified timestamp, and the name of the changed crate, e.g.

```
1 2019-10-18 23:51:23 oxigen
1 2019-10-18 23:51:25 linda
1 2019-10-18 23:51:29 rv
1 2019-10-18 23:52:00 anyhow
1 2019-10-18 23:53:03 build_id
1 2019-10-18 23:56:16 canonical-form
1 2019-10-18 23:59:01 cotton
1 2019-10-19 00:01:44 kg-utils
1 2019-10-19 00:08:45 serde_traitobject
```

Because the log is append-only, the client can incrementally update it using a `Range` HTTP request. The client doesn't have to download the full log in order to start using it; it can download only an arbitrary fraction of it, up to the end of the file, which is straightforward with a `Range` request. When a crate is found in the log (searching from the end), and modification date is the same as modification date of crate's cached locally, the client won't have to make an HTTP request for the file.

When the log grows too big, the epoch number can be incremented, and the log reset back to empty. The epoch number allows clients to detect that the log has been reset, even if the `Range` they requested happened to be valid for the new log file.

# Drawbacks
[drawbacks]: #drawbacks

* A basic solution, without the incremental changelog, needs more requests and has higher latency to update the index. With the help of the incremental changelog, this is largely mitigated. For GitHub-hosted indexes Cargo has a fast path that checks in GitHub API whether the master branch has changed. With the changelog file, the same fast path can be implemented by making a conditional HTTP request for the changelog file (i.e. checking `ETag` or `Last-Modified`).
* Performant implementation of this solution depends on making many small requests in parallel. HTTP/2 support on the server makes checking twice as fast compared to HTTP/1.1, but speed over HTTP/1.1 is still reasonable.
* If GitHub won't like high-traffic usage of the index via raw.githubusercontent.com, the index may need to be cached/hosted elsewhere.
* Since alternative registries are stable, the git-based protocol is stable, and can't be removed.
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

Bundler also uses an append-only format for individual dependency files to incrementally download only new versions' information where possible. Cargo's format is almost append-only (except yanking), so if growth of individual dependency files becomes a problem, it should be possible to fix that. However, currently the largest crate `rustc-ap-rustc_data_structures` that publishes versions daily grows by about 44 bytes per version (compressed), so even after 10 years it'll take only 190KB (compressed), which doesn't seem to be terrible enough to require a solution yet.
