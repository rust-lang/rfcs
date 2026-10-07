- Feature Name: `registry_withholding`
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary
[summary]: #summary

This RFC adds a new, optional registry index field, `withheld` (`quarantined | withdrawn`).

It also adds an optional registry `config.json` URL template alongside `dl`: `notice-page`, to point
readers to withheld crate information pages.

Lastly, it specifies how Cargo avoids resolving `withheld` versions and instead display status-aware errors.

This is part of the proposed [crates.io registry response project goal](https://github.com/rust-lang/goals/pull/795).

## Motivation
[motivation]: #motivation

This RFC is the first step towards a goal of hardening crates.io and other registries to be able to
rapidly respond to possible attacks using non-destructive actions and, ultimately, pre-emptively hold likely
malicious bytes for pre-release reviews. The big-picture plan [is in the registry response Project Goal](https://github.com/rust-lang/goals/pull/795).

Our goal in this RFC is to unblock crates.io from closing a gap in its security responses: its operators are slow to 
take destructive actions like blanket-deleting all crates of an account. It needs a way to "freeze" that account 
during investigation. That same capability will also be needed for any sort of automated defense. The next RFC
adds an `unreleased` state to cover publish-time holds.

To accomplish this goal, we add a middle ground between "released" and "erased from the index" via the
new `withheld` field. We have "yanked", but yanked does not indicate a malicious release. Yanked releases are also 
still resolvable via an existing `Cargo.lock`, and, worse, their bytes are freely downloaded by default. We instead
want a way to say, "this release is being quarantined, and its bytes are not served via the normal download path". And 
similarly, if we ultimately delete a release, we'd like the option to leave a tombstone showing that it used to be 
there. Withheld bytes are retained for research purposes. Registries can opt to serve them, but Cargo has no way
to fetch them until the next RFC.

A key tradeoff of this approach, withheld crates staying in the index, is that it breaks the invariant that every
indexed version is downloadable from `dl`. We accept this because omitting the line would cause pinned
versions to fail with no explanation, and it is indistinguishable from a version that was never published. Other 
ecosystems (PyPI, npm) that started with omission have since added explanatory markers or plan to.

The security boundary that we are focused on here is fresh acquisition, with a cold cache. Cargo avoids withheld bytes 
whenever its view of the index is fresh, but the combination of a warm crate cache and a stale, cached view of the 
index that is missing a newer `withheld` marker, will allow continued use of withheld bytes. This problem also affects
deletion today. This gap will mostly be addressed by verified mirrors using the upcoming TUF signing work.

Out of scope for this RFC:
- publish-time `unreleased` state (which does not imply misuse, for instance publish-time review)
- publishing against `withheld` versions (mainly a concern for `unreleased`)
- fetching `withheld` bytes via Cargo, for forensic or publish-time builds, with a registry-advertised location
to access those bytes (`dl-withheld`)
- crates.io criteria for and implementation of withholding
- crates.io frontend display of withheld status
- crates.io automated withholding systems

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

`withheld` is an optional new index field with two possible values: `quarantined` (the registry froze the version;
it is not installable and its bytes are not served via the normal download path; it may be released or withdrawn) and 
`withdrawn` (permanent tombstone marking the removed version).

Cargo never selects a withheld version. On fresh resolution, it will pick a different compatible version, so
most people never encounter it. The only time you encounter withheld versions is when pinned in a lockfile,
or if no other version satisfies the requirement:

```
error: failed to select a version for the requirement `base64squatter = "=1.0.0"`
  version 1.0.0 is quarantined
location searched: crates.io index
required by package `myapp v0.1.0 (/home/user/myapp)`
  |
  = note: this version is pinned by Cargo.lock; run `cargo update base64squatter` to select a different version, or change the requirement in Cargo.toml if no other version satisfies it
  = help: for more information see https://crates.io/crates/base64squatter/1.0.0
```

The last line comes from the registry's `notice-page`, if it advertises one in its `config.json`.

Registries also mark withheld versions `yanked: true`, so older Cargo and other tools route around them. If they fetch 
the version anyway, for instance due to a pinned lockfile, they get a 404 with an explanation in the body.

For researchers, withheld versions are in the index, and a registry may document a way to fetch withheld bytes.
Fetching withheld bytes via Cargo is in the next RFC.

For maintainers, publishing skips a withheld dependency like a yanked one. Pinned withheld dependencies fail
with the above error. Publishing against your own withheld versions is in the next RFC.

docs.rs does not build withheld versions and shows a status badge instead. It instead builds them when they are 
released.


## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

### Added to [Registry Index / JSON schema](https://doc.rust-lang.org/cargo/reference/registry-index.html#json-schema)

*Amended, new "withheld" field after "yanked", pubtime comment amended*

```javascript
{
    [...]
    // Boolean of whether or not this version has been yanked.
    "yanked": false,
    // The withheld status of this package version (optional).
    //
    // Omitted when the package version is not withheld.
    //
    // A withheld entry is also marked as `"yanked": true`.
    // 
    // If set, Cargo does not select this version, even if it
    // is pinned in `Cargo.lock`, and errors if no other
    // version satisfies the requirement.
    //
    // The `.crate` file is not served from a registry's `dl` endpoint
    // while the package version is withheld.
    //
    // The current values are:
    // * "quarantined": The registry has frozen this package version because it
    //   suspects misuse. The package version may later transition to "withdrawn"
    //   or have its "withheld" field removed (making it installable again).
    // * "withdrawn": The registry has permanently removed this package version.
    //   The entry remains as a tombstone and the version number cannot be reused.
    //
    // All unknown values are treated as "quarantined".
    "withheld": "quarantined",
    [...]
    // [..]
    // Example: 2025-11-12T19:30:12Z
    //
    // This should be the time the package version first became installable.
    // It is not changed on any later status changes, like `yanked` or `withheld`.
    // A package version that is withheld on arrival has no `pubtime` until
    // it is released.
    "pubtime": "2025-11-12T19:30:12Z"
}
```
*Amended, withheld as mutable, pubtime as write-once*

The JSON objects should not be modified after they are added, except for the
`yanked` and `withheld` fields, whose value may change at any time, and
`pubtime`, which may be updated exactly once from unset to a non-null value.

---

Related:
- Drawbacks: [A second mutable index field](#a-second-mutable-index-field)
- Rationale: [Why no index protocol bump?](#why-no-index-protocol-bump)
- Rationale: [Why `withheld` instead of (further) overloading `yanked`?](#why-withheld-instead-of-further-overloading-yanked)
- Rationale: [Why write withheld releases to the index?](#why-write-withheld-releases-to-the-index)
- Rationale: [Why no reason text in the index?](#why-no-reason-text-in-the-index)

### Added to [Registry Index / Index Configuration](https://doc.rust-lang.org/cargo/reference/registry-index.html#index-configuration)

*Amended, new key after `auth-required`:*
- `notice-page`: an optional URL for a human-readable page describing a package
  version's status or the registry's withholding policies. Cargo links it from errors about withheld
  versions but never fetches it. Accepts the `dl` markers except `{sha256-checksum}`.
  Without markers, the URL is used as-is, and no path is appended.


### Added to [Registry Index / Withheld versions](https://doc.rust-lang.org/cargo/reference/registry-index.html#withheld-versions)

*New section after "Version uniqueness":*

A registry may withhold a package version by setting the `withheld` field
in its index entry and also setting `"yanked": true`, so that tools unaware
of `withheld` avoid the version.

A withheld package version's `.crate` file is not served. The endpoint responds 404,
with a body explaining the status and linking to `notice-page` or the
registry's withholding policy. The body does not name an alternative download location.

When the withholding ends, the registry restores the author's requested
yank state. A yank or unyank requested while the package version is withheld
is recorded and takes effect on release.

`pubtime` is never modified for a withheld package version. This means
that it is initially unset if a package is uploaded while initially withheld,
and later set when the package exits withholding.

---

Related:
- Rationale: [Why `withheld` instead of (further) overloading `yanked`?](#why-withheld-instead-of-further-overloading-yanked)

### Added to [Registry Web API](https://doc.rust-lang.org/cargo/reference/registry-web-api.html)

*Amended, `warnings` added to the Yank response object (same for Unyank)*
```javascript
{
    // Indicates the yank succeeded, always true.
    "ok": true,
    // Optional object of warnings to display to the user, in the same form as
    // the publish response. Used, for example, when the version is withheld and
    // the change to `yanked` takes effect only when the version is released.
    "warnings": {
        "other": []
    }
}
```

---

Cargo prints each entry in `warnings.other` as a `warning:` line, matching its handling
for the `publish` response.

### Added to [Dependency Resolution / Withheld versions](https://doc.rust-lang.org/cargo/reference/resolver.html#withheld-versions)

*New section after "Yanked versions":*

[Withheld releases][withheld] are those that a registry has marked as
not installable. When the resolver is building the graph, it will
ignore all withheld versions, including those that already exist
in the `Cargo.lock` file, and report an error if no other
version satisfies the requirement.

[withheld]: registry-index.md#withheld-versions

---

The error for a locked withheld version suggests `cargo update <crate>`, which re-resolves
that crate and keeps every other lock entry. Once the version is released, the same
lockfile resolves again unchanged.

`cargo install --locked` honors the crate's bundled `Cargo.lock`, so the newest version of
a binary can fail to install because a dependency is withheld where an older version
would have succeeded; Cargo does not inspect candidates' bundled lockfiles when choosing.

Cache lifecycles are unchanged. A withheld version keeps building if it was last seen
not-withheld, and has its bytes and index file cached. Once Cargo integrates with
verifiable mirrors, it will invalidate caches on any upstream change for verified mirrors.

Related:
- Drawbacks: [Existing lockfiles can break without a local change](#existing-lockfiles-can-break-without-a-local-change)
- Drawbacks: [Warm caches keep withheld versions buildable](#warm-caches-keep-withheld-versions-buildable)
- Prior Art: [Cache invalidation for verifiable mirrors](#cache-invalidation-for-verifiable-mirrors)
- Future possibilities: [Smarter `cargo install --locked` on withheld dependencies](#smarter-cargo-install---locked-on-withheld-dependencies)


### docs.rs

docs.rs does not build withheld versions. `crates-index-diff`, `docs_rs_crates_io`, and the pending crates.io event 
feed ([crates.io#14188](https://github.com/rust-lang/crates.io/pull/14188)) expose the `withheld` field along with `yanked`.

On seeing a withheld version, docs.rs skips the version's build and shows a badge stating its status
and linking to `notice-page` if the registry advertises one. Documentation built before withholding
stays below the badge. For `withdrawn`, it is removed.

When the `withheld` field is removed, docs.rs builds the version if it has not already and drops the badge.

## Drawbacks
[drawbacks]: #drawbacks

#### A second mutable index field
- This stinks, but the ship has arguably sailed with yanks. See Rationale: 
[Why write withheld releases to the index?](#why-write-withheld-releases-to-the-index)
- We also indicate that `pubtime` is write-once, from unset to a value, but this was already the case due
to our historical backfill operations

#### An index line no longer guarantees fetchable bytes
- By design since we want to make clear to consumers *why* locked versions are not reachable (see:
Rationale: [Why write withheld releases to the index?](#why-write-withheld-releases-to-the-index))
- Partially mitigated by helpful 404 bodies
- Index signing and verification are unaffected since withheld status is written to the index and these
mechanisms do not consider byte availability

#### Existing lockfiles can break without a local change
- Especially painful for `cargo install --locked`. See Future possibilities: [Smarter `cargo install --locked` on withheld dependencies](#smarter-cargo-install---locked-on-withheld-dependencies)

#### Warm caches keep withheld versions buildable
- True of any design without cache evictions. Will be addressed for verifiable mirrors by default, see Prior Art: [Cache invalidation for verifiable mirrors](#cache-invalidation-for-verifiable-mirrors)

- Already the case for current crate deletion practices

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

### Requirements

Essential:
1. Fresh fetches of a withheld version through default registry paths fail with diagnostics that explain the status, 
both for Cargo, and for other tools, including ones that do not consult the index to decide what to fetch
2. Existing Cargo releases have a safe default behavior with no code changes
3. Tools can differentiate withheld releases on the wire from other statuses like yanked, deleted, and never-published
4. Withholding is reversible without requiring a fresh publication, loses no information, and a released
version is treated the same by tools as one which was never withheld
5. Index followers such as docs.rs handle a version while withheld and once released
6. Mirrors and index signing keep working unchanged
7. Withheld versions and their status are discoverable from the index, without additional requests

Preferred:
1. Reuse the existing index, version model, synchronization, and release flow (especially, no index protocol bump)

### Major architectural alternatives

#### Why write withheld releases to the index?

PyPI's quarantine and npm's publish-time scanning both remove
or never write a line. For a version that someone has already locked,
omission produces confusing resolution failures with no indication of why.
This is indistinguishable from a version that never existed. Both registries
have [seen complaints](https://github.com/orgs/community/discussions/203413)
for this. PyPI has since added status markers, and npm plans to.

Writing the line lets Cargo explain the status, point at `notice-page`, and give
guidance to the user (such as `cargo update`). It also lets
us set `withdrawn` tombstones that transparently reserves coordinates, unlike omission.
It still lets older Cargo fall back to yanked behavior that avoids resolving to the unfetchable version.

This fits the values of the project around transparency and
accountability. If we are making it easier for administrators to take new
curation actions on the index, we want transparency logs and auditability.
Showing the withheld coordinates also makes it possible for researchers to
obtain withheld bytes (though this RFC defers this as a registry decision).

We could address these needs with other side channels (a status feed, a query API, etc).
But they add significant complexity to our systems while producing
worse consumer-side experiences (new build-tool network calls, or else opaque
errors).

The cost of this decision is breaking the invariant that each indexed version is downloadable.
We accept it because the registry can smooth consumer experience on both sides (the index and the 404 body).
We always give an explanation where this causes a failure. We can still nudge unaware tools in a
good direction by setting `yanked: true`. The worst case (mirrors that fetch every single line
and loudly fail on missing bytes) still continue to new lines and can trivially add logic to skip
fetching `withheld` lines.

Whether `unreleased` versions (held with no misuse implied) should have a similar
treatment is deferred to a subsequent RFC as they have different tradeoffs and
systems involved.

#### Why not add raw.crates.io and make crates.io a mirror of it that only serves non-withheld versions?

This is still omission from the consumer's point of view, with all the same downsides around
legibility and clear build tool behavior. It has advantages in comparison to omission in that it preserves
an audit log, and that it makes it possible for researchers to enumerate quarantined versions.
But, writing the `withheld` line to the index offers the same benefits.

In exchange, it requires building a second index, which adds a systems complexity
and operator burden. It also makes it possible for Cargo to access all withheld bytes
with no warnings or guardrails. A future RFC will suggest a method of accessing
withheld bytes that instead is scoped to explicitly requested bytes.

#### Why `withheld` instead of (further) overloading `yanked`?

`yanked` currently means two things: an author saying, "probably don't use this" for an unspecified
reason, and an administrator's soft mitigation during an investigation. Making a yank also mean "and the bytes are gone"
makes the semantics even more confusing and makes clean handling by build tools more difficult
since they cannot distinguish a broken release from a withheld one. `withheld` also enables
`withdrawn` which signals that the state is terminal, which `yanked` currently does not imply.
We do still set `yanked: true` when `withheld` as a compatibility shim for old tools.

Adding a second field with a small enum allows build tools to understand the state of crate bytes
without making additional fetches or adding further fallbacks. It also gives us a path to clean
up the semantics of `yanked` to at least be clearer that a standalone yank (with no `withheld`)
is *not* due to administrator action.

### Other decisions

#### Why no reason text in the index?

In this RFC, the reason for withholding is stored registry-side. It can be made available on the user-facing
page that is advertised via `notice-page`. This mirrors crates.io's handling of yanked reasons; we could
also use `notice-page` to display yanked reasons once frontend support for them is built.

Putting reasons on the wire is being considered for `yanked` as well, and both should be addressed
in the same future RFC. See Future possibilities: [Better display of reasons for withholding](#better-display-of-reasons-for-withholding).

#### Why no index protocol bump?

`v` exists for changes that older Cargo would misinterpret in a way that causes incorrect behavior.
A higher `v` makes it ignore the entry entirely. Cargo already skips unknown fields.

If `withheld` is ignored, Cargo will produce the same build outcome, just with worse error messages.
It still avoids resolving withheld versions via the `yanked: true` shim. If it is forced
to resolve a withheld version via a lockfile, it sees an explanatory 404 from the registry.

The cost of bumping `v` is effectively the same as omission for older versions of Cargo, which
seems strictly worse ("version not found" rather than an explanatory error). The unstabilized experimental
`v: 3` also makes a bump awkward since we do want `withheld` to be immediately usable, before
the other v3-gated features stabilize.

## Prior art
[prior-art]: #prior-art

### Cache invalidation for verifiable mirrors

The [ongoing Verifiable Mirror Project Goal](https://goals.rust-lang.org/2026/mirroring.html) will mostly
address the "stale-cache" risk that currently impacts deleted crates and will impact quarantined crates.

Because the TUF allows vending a verifiable merkle tree for all index content, Cargo can cheaply evaluate
subtrees such as individual crate files for consistency with upstream. This allows Cargo to invalidate its
cache if it sees a new change, such as a quarantine on an already-cached crate's index metadata.


### Withholding across ecosystems

<details>
  <summary>See roundup</summary>

Across language ecosystems, the most crates.io-like registries (PyPI, npm), have implemented
reversible quarantine, enforced via omission from the index. Both have seen user pain
due to illegibility. PyPI has added additional markers, and npm plans to.

Other registries have variants of deletions and yanks, but no reversible quarantine at
the core registry level.

#### Rust

[RFC 3660](https://rust-lang.github.io/rfcs/3660-crates-io-crate-deletions.html) adds destructive
deletes via index line omission. It temporarily reserves the crate name upon deletion.

`withheld` is the non-destructive counterpart to this, and `withdrawn` is the terminal,
non-destructive equivalent.

#### PyPI

PyPI has a similar concept of yanking as Rust, where a release remains installable when
pinned but is otherwise avoided ([PEP 592](https://peps.python.org/pep-0592/)).

PyPI added admin-set, reversible [project quarantine](https://blog.pypi.org/posts/2024-12-30-quarantine/)
in 2024, enforced by omission from the index. Yanking was considered and rejected because a yanked
release is still installable. A year later, it retrofitted [project status markers](https://blog.pypi.org/posts/2025-08-14-project-status-markers/) ([PEP 792](https://peps.python.org/pep-0792/)) based on community feedback
to make the status legible to installers and mirrors.

Release-level quarantine, [added in 2026](https://github.com/pypi/warehouse/pull/20062), is also enforced via 
omission, but with no marker. There are no recorded legibility complaints for release-level
quarantine yet, but it is also a fairly recent feature.

#### npm

npm has no yank. `deprecate` is set by authors and informational only (installs with a warning). Malware
is handled by permanent deletion, similar to crates.io. When every version is gone, the name is held by
a [`security-holder`](https://github.com/npm/security-holder) tombstone. npm's 2016 [left-pad unpublish](https://blog.npmjs.org/post/141905368000/changes-to-npms-unpublish-policy)
is the classic case of a deleted version breaking downstream builds (the whole package returns a plain 404).
As a result of this, npm restricted unpublishing rather than making the state legible.

npm holds newly published versions during [publish-time scanning](https://github.blog/changelog/2026-07-28-npm-publish-time-malware-scanning-and-dual-use-metadata/)
which it enforces through omission of that version from the index. The [community thread](https://github.com/orgs/community/discussions/203413)
discusses resulting breakage (broken publish scripts and release trains). npm has said it is
prioritizing work to display scanning status (see its
[safer-publishing roadmap](https://github.com/orgs/community/discussions/208130)). The publish-time failure
issues are better prior art for the coming RFC on publish-time holds, but lessons on legibility apply here.

#### RubyGems

[`gem yank`](https://guides.rubygems.org/removing-a-published-gem/) removes both the index
entry and the gem file. The version cannot be re-published. This is not a reversible state,
and RubyGems has no quarantine.

#### Maven Central

Central artifacts are [immutable and not removed](https://central.sonatype.org/faq/can-i-change-a-component/)
except for malware and legal takedowns. Removal is destructive.

Instead, Sonatype's paid [Firewall](https://help.sonatype.com/en/firewall-quarantine.html)
offers reversible quarantines via an opt-in proxy. Firewall doesn't steer resolution. Upon requesting quarantined 
versions, it "returns an error message linking the component details to the requester". This RFC returns
a similar error but does steer resolution.

#### Go

Go's [`retract` directive](https://go.dev/ref/mod#go-mod-file-retract) is author-only and
functions similarly to `yanked` (reversible, steers resolution, still installable).

Removals are destructive and leave no marker in the index, similar to crates.io deletes. The proxy
protocol does allow a plain-text error body on 404/410, which `go` surfaces, so removed versions
can offer explanations.

</details>

## Unresolved questions
[unresolved-questions]: #unresolved-questions

### To resolve before merge
- Whether we should make `cargo_util_schemas::index::IndexPackage` `#[non_exhaustive]` while we are bumping semver anyway
- Whether this RFC names the crates.io researcher download location or leaves it to the crates.io admin-API work
(I suggest deferring)

### To resolve during implementation
- Exact errors and prose notes, documentation notes, documentation URLs

### Related problems this RFC leaves open
- Dependents of a withheld version are published and installable but broken until it is released. Propagation, 
linking, or docs.rs re-processing are registry- and docs.rs-side problems for a later RFC once we add `unreleased`
publish-time holds.
- Warm-cache exposure (stale index + cached crate bytes) is pre-existing and un-addressed here

## Future possibilities
[future-possibilities]: #future-possibilities

#### `unreleased` and publishing against withheld versions
- The next RFC adds a routine `unreleased` status, a registry-advertised `dl-withheld` location
for withheld bytes, `--fetch-withheld name@version` for forensic and publish-time builds, and related
`cargo publish` behavior for workspace publishes and release trains

#### Smarter `cargo install --locked` on withheld dependencies
- Cargo does not inspect candidates' bundled lockfiles, so a newer binary could fail where an
older one would succeed. On that failure, Cargo could check older versions and either suggest an
alternative or implicitly fall back.
- This deserves its own RFC and the same gap already exists for deleted crates

#### `cargo info` support
- `cargo info` shows only candidate versions, meaning not yanked or withheld ones. It could show both,
with a marker, since the index has that state.

#### Better display of reasons for withholding
- A registry endpoint could serve machine-readable reasons that Cargo shows in errors, or reasons on the index
line itself
- This should be designed along with `yanked` reason improvements

#### Yank reasons on `notice-page`
- `notice-page` could carry the yank reasons once crates.io's frontend shows them. This could land
alongside the crates.io admin API. Yank reasons already have backend API support, and the frontend
work for `withheld` reasons touches similar code paths.