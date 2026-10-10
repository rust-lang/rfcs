- Feature Name: `registry_withholding`
- Start Date: 2026-10-07
- RFC PR: [rust-lang/rfcs#4016](https://github.com/rust-lang/rfcs/pull/4016)

## Summary
[summary]: #summary

This RFC adds a new, optional registry index field, `availability` (`available | quarantined | withdrawn`).

It also specifies how Cargo avoids resolving unavailable versions and instead displays status-aware errors.

This is part of the proposed [crates.io registry response project goal](https://github.com/rust-lang/goals/pull/795).

## Motivation
[motivation]: #motivation

This RFC is the first step towards a goal of hardening crates.io and other registries to be able to
rapidly respond to possible attacks using non-destructive actions and, ultimately, pre-emptively hold likely
malicious bytes for pre-release reviews. The big-picture plan [is in the registry response Project Goal](https://github.com/rust-lang/goals/pull/795).

During security investigations, crates.io operators are slow to take destructive actions like
blanket-deleting all crates of an account. They could move more rapidly, with less risk of collateral
damage, if they could "freeze" that account: something that prevents users from installing malicious
crates while still being easy to reverse if it turns out to be a false alarm. That same capability will also be needed for any sort of automated defense. The next RFC
adds an `unreleased` state to cover publish-time holds.

To accomplish this goal, we add a middle ground between "released" and "erased from the index" via the
new `availability` field. We have "yanked", but yanked does not indicate a malicious release. Yanked releases are also 
still resolvable via an existing `Cargo.lock`, and, worse, their bytes are freely downloaded by default. We instead
want a way to say, "this release is being quarantined, and its bytes are not served via the normal download path". And 
similarly, if we ultimately delete a release, we'd like the option to leave a tombstone showing that it used to be 
there. Unavailable bytes are retained for research purposes. Registries can opt to serve them, but Cargo has no way
to fetch them until the next RFC.

A key tradeoff of this approach, unavailable crates staying in the index, is that it breaks the invariant that every
indexed version is downloadable from `dl`. We accept this because omitting the line would cause pinned
versions to fail with no explanation, and it is indistinguishable from a version that was never published. Other 
ecosystems (PyPI, npm) that started with omission have since added explanatory markers or plan to.

The security boundary that we are focused on here is fresh acquisition, with a cold cache. Cargo avoids unavailable bytes 
whenever its view of the index is fresh, but the combination of a warm crate cache and a stale, cached view of the 
index that is missing a newer `availability` marker, will allow continued use of unavailable bytes. This problem also affects
deletion today. This gap will mostly be addressed by verified mirrors using the upcoming TUF signing work.

Out of scope for this RFC:
- publish-time `unreleased` state (which does not imply misuse, for instance publish-time review)
- publishing against unavailable versions (mainly a concern for `unreleased`)
- fetching unavailable bytes via Cargo, for forensic or publish-time builds, with a registry-advertised location
to access those bytes
- crates.io criteria for and implementation of withholding
- crates.io frontend display of unavailable statuses
- crates.io automated withholding systems
- crates.io API exposure of unavailable statuses, for the frontend or other consumers
- end-user-triggered quarantine
- withholding newly published versions by default

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

`availability` is an optional new index field. When it is present, and its value is not `available`,
tools should ignore the version and only consider the index row for display in error messages.
This RFC defines three possible values, `quarantined`, `withdrawn`, and `available`.
Other values may be added in future RFC's, and client should be forward compatible with this by
treating unknown values as `quarantined`.

`quarantined` is set when the registry froze a version. It is not available to select during
dependency resolution and its bytes are not served via the normal download path. `quarantined`
versions may be subsequently released or withdrawn.

`withdrawn` is a permanently unavailable version, whose index row is a tombstone
marking the removed version.

`available` is equivalent to a value of `null` and indicates that the index row has no availability
restrictions.

Cargo never selects an unavailable version. On fresh resolution, it will pick a different compatible version, so
most people never encounter it. The only time you encounter unavailable versions is when pinned in a lockfile,
or if no other version satisfies the requirement:

```
error: failed to select a version for the requirement `base64squatter = "=1.0.0"`
  version 1.0.0 is quarantined
location searched: crates.io index
required by package `myapp v0.1.0 (/home/user/myapp)`
  |
  = note: this version is pinned by Cargo.lock; run `cargo update base64squatter` to select a different version, or change the requirement in Cargo.toml if no other version satisfies it
```

Registries also mark unavailable versions with `yanked: true`, so older Cargo and other tools route around them.
If non-status-aware tools fetch the version anyway, for instance due to a pinned lockfile, they should get a 404,
which should include an explanation of the version's state in the body. Cargo
displays this body on fetch failure.

For researchers, unavailable versions are in the index, and a registry may document a way to fetch unavailable bytes.
Fetching unavailable bytes via Cargo is in the next RFC.

For maintainers, Cargo will avoid unavailable versions of crate dependencies while generating a lockfile.
If a lockfile is committed and includes an unavailable version, the publication fails with a useful errors.
`cargo publish --exclude-lockfile` skips this evaluation as before.

docs.rs does not build unavailable versions and shows a status badge instead. It instead builds them when they are 
released.


## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

### Added to [Registry Index / JSON schema](https://doc.rust-lang.org/cargo/reference/registry-index.html#json-schema)

*Amended, new "unavailable" field after "yanked"*

```javascript
{
    [...]
    // Boolean of whether or not this version has been yanked.
    "yanked": false,
    // The availability status of this package version (optional).
    //
    // If set to a value besides "available", Cargo does not select this version, even if it
    // is pinned in `Cargo.lock`, and errors if no other
    // version satisfies the requirement. If not set, Cargo treats this version as "available".
    //
    // For unavailable versions, a `.crate` file is not served from a registry's `dl` endpoint
    // while the package version is unavailable.
    //
    // The current values are:
    // * "quarantined": The registry has frozen this package version because it
    //   suspects misuse. The package version may later transition to "withdrawn"
    //   or to "available" (making it installable again).
    // * "withdrawn": The registry has permanently removed this package version.
    //   The entry remains as a tombstone and the version number cannot be reused.
    // * "available": Available with no restrictions
    //
    // All unknown values are treated as "quarantined".
    "availability": "quarantined",
    [...]
}
```
*Amended, availability as mutable*

The JSON objects should not be modified after they are added, except for the
`yanked` and `availability` fields, whose value may change at any time.

---

Elsewhere in this RFC:
- Drawbacks: [A second mutable index field](#a-second-mutable-index-field)
- Rationale: [Why no index protocol bump?](#why-no-index-protocol-bump)
- Rationale: [Why `availability` instead of (further) overloading `yanked`?](#why-unavailable-instead-of-further-overloading-yanked)
- Rationale: [Why write unavailable releases to the index?](#why-write-unavailable-releases-to-the-index)
- Rationale: [Why no reason text in the index?](#why-no-reason-text-in-the-index)

### Added to [Registry Index / Unavailable versions](https://doc.rust-lang.org/cargo/reference/registry-index.html#unavailable-versions)

*New section after "Version uniqueness":*

A registry may withhold a package version by setting the `availability` field
in its index entry to a value besides `available` and also setting `"yanked": true`,
so that tools unaware of `availability` avoid the version.

An unavailable package version's `.crate` file is not served. The endpoint should
respond 404 and serve a body explaining the status and linking to a webpage about
the crate's status and/or registry policies. The body should not not name an 
alternative  download location.

When a period of unavailability ends, the registry restores the author's requested
yank state. A yank or unyank requested while the package version is unavailable
is recorded and takes effect on release.

The value of `pubtime` is not impacted by changes to the `availability` field.

---

Elsewhere in this RFC:
- Rationale: [Why `availability` instead of (further) overloading `yanked`?](#why-unavailable-instead-of-further-overloading-yanked)

### Added to [Dependency Resolution / Unavailable versions](https://doc.rust-lang.org/cargo/reference/resolver.html#unavailable-versions)

*New section after "Yanked versions":*

[Unavailable releases][unavailable] are those that a registry has marked as
not accessible for usage. When the resolver is building the graph, it will
ignore all unavailable versions, including those that already exist
in the `Cargo.lock` file, and report an error if no other
version satisfies the requirement.

[unavailable]: registry-index.md#unavailable-versions

---

The error for a locked unavailable version suggests `cargo update <crate>`, which re-resolves
that crate and keeps every other lock entry. Once the version is released, the same
lockfile resolves again unchanged.

Cache lifecycles are unchanged. an unavailable version keeps building if it was last seen
not-unavailable, and has its bytes and index file cached. Once Cargo integrates with
verifiable mirrors, it will invalidate caches on any upstream change for verified mirrors.

Elsewhere in this RFC:
- Drawbacks: [Existing lockfiles can break without a local change](#existing-lockfiles-can-break-without-a-local-change)
- Drawbacks: [Warm caches keep unavailable versions buildable](#warm-caches-keep-unavailable-versions-buildable)
- Prior Art: [Cache invalidation for verifiable mirrors](#cache-invalidation-for-verifiable-mirrors)


### docs.rs

docs.rs does not build unavailable versions. `crates-index-diff`, `docs_rs_crates_io`, and the pending crates.io event 
feed ([crates.io#14188](https://github.com/rust-lang/crates.io/pull/14188)) expose the `availability` field along with `yanked`.

On seeing an unavailable version, docs.rs skips the version's build and shows a badge stating its status. Documentation built before withholding
stays below the badge. For `withdrawn`, documentation is deleted, leaving behind
a placeholder page indicating status.

When the `availability` field is subsequently set to `available` or removed, docs.rs drops the badge but does not trigger
a rebuild unless manually requested.

## Drawbacks
[drawbacks]: #drawbacks

#### A second mutable index field
- This stinks, but the ship has arguably sailed with yanks. See Rationale: 
[Why write unavailable releases to the index?](#why-write-unavailable-releases-to-the-index)

#### An index line no longer guarantees fetchable bytes
- By design since we want to make clear to consumers *why* unavailable versions are not reachable (see:
Rationale: [Why write unavailable releases to the index?](#why-write-unavailable-releases-to-the-index))
- Partially mitigated by helpful 404 bodies
- Index signing and verification are unaffected since unavailable statuses are written to the index and these
mechanisms do not consider byte availability

#### Existing lockfiles can break without a local change
- This is not a new problem; the same behavior affects deleted versions. This is
arguably an improvement because now we have better error messages, but it would
be nice to have a better solution at least for the install case.

#### Warm caches keep unavailable versions buildable
- True of any design without cache evictions. Will be addressed for verifiable mirrors by default, see Prior Art: [Cache invalidation for verifiable mirrors](#cache-invalidation-for-verifiable-mirrors)

- Already the case for current crate deletion practices

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

### Requirements

Essential:
1. Fresh fetches of an unavailable version through default registry paths fail with diagnostics that explain the status, 
both for Cargo, and for other tools, including ones that do not consult full index
resolution rules to decide what to fetch (Yocto, registry mirrors, etc)
2. Existing Cargo releases have a safe default behavior with no code changes
3. Tools can differentiate unavailable releases on the wire from other statuses like yanked, deleted, and never-published
4. Withholding is reversible without requiring a fresh publication, loses no information, and a released
version is treated the same by tools as one which was never unavailable
5. Index followers such as docs.rs handle a version while unavailable and once released
6. Mirrors and index signing keep working unchanged
7. Unavailable versions and their status are discoverable from the index, without additional requests

Preferred:
1. Reuse the existing index, version model, synchronization, and release flow (especially, no index protocol bump)

### Major architectural alternatives

#### Why indicate status with an enum rather than a bool?

A bool is all we need to strictly tell Cargo that bytes are unavailable and to avoid resolving.
But, an enum allows Cargo to share additional semantic meaning that is clearer to readers
(for quarantined vs withdrawn, is it a temporary hold or not?).

It also sets up better for future variants that might need additional special handling,
for instance related to publishing and release trains (see [Future Possibilities](#future-possibilities)).

Migrating from a bool to an enum later would be awkward and confusing. We arguably would
prefer for `yanked` to be part of this same status enum, rather than a bool, but are stuck
with it for backwards compatibility reasons.

#### Why include `availability` (with `available`) rather than `withheld`?

We don't want to have a negative connotation on crates that are withheld, since this does not
necessarily indicate malicious activity. A neutral field name like `availability` leaves us
open for alternative, softer statuses, without the negative implication.

The downside of this is slightly more complexity for downstream tooling, which now needs
to check the enum rather than assuming presence of the field indicates unavailbility. This
is partially mitigated by defining everything besides `available` or `null` as unavailable, and
equivalent to `quarantined` if unknown. This allows tools that are uninterested in more granular
status to handle this field with a straightforward equivalence check.

We prefer `availability` to `available` because `"available": "available"` is weird looking.

#### Why write unavailable releases to the index?

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
Showing the unavailable coordinates also makes it possible for researchers to
obtain unavailable bytes (though this RFC defers this as a registry decision).

We could address these needs with other side channels (a status feed, a query API, etc).
But they add significant complexity to our systems while producing
worse consumer-side experiences (new build-tool network calls, or else opaque
errors).

The cost of this decision is breaking the invariant that each indexed version is downloadable.
We accept it because the registry can smooth consumer experience on both sides (the index and the 404 body).
We always give an explanation where this causes a failure. We can still nudge unaware tools in a
good direction by setting `yanked: true`. The worst case (mirrors that fetch every single line
and loudly fail on missing bytes) still continue to new lines and can trivially add logic to skip
fetching `availability` lines.

Whether `unreleased` versions (held with no misuse implied) should have a similar
treatment is deferred to a subsequent RFC as they have different tradeoffs and
systems involved.

#### Why not add raw.crates.io and make crates.io a mirror of it that only serves non-unavailable versions?

This is still omission from the consumer's point of view, with all the same downsides around
legibility and clear build tool behavior. It has advantages in comparison to omission in that it preserves
an audit log, and that it makes it possible for researchers to enumerate quarantined versions.
But, writing the `availability` line to the index offers the same benefits.

In exchange, it requires building a second index, which adds a systems complexity
and operator burden. It also makes it possible for Cargo to access all unavailable bytes
with no warnings or guardrails. A future RFC will suggest a method of accessing
unavailable bytes that instead is scoped to explicitly requested bytes.

#### Why `availability` instead of (further) overloading `yanked`?

`yanked` currently means two things: an author saying, "probably don't use this" for an unspecified
reason, and an administrator's soft mitigation during an investigation. Making a yank also mean "and the bytes are gone"
makes the semantics even more confusing and makes clean handling by build tools more difficult
since they cannot distinguish a broken release from an unavailable one. `availability` also enables
`withdrawn` which signals that the state is terminal, which `yanked` currently does not imply.
We do still set `yanked: true` when `availability` as a compatibility shim for old tools.

Adding a second field with a small enum allows build tools to understand the state of crate bytes
without making additional fetches or adding further fallbacks. It also gives us a path to clean
up the semantics of `yanked` to at least be clearer that a standalone yank (with no `availability`)
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

If `availability` is ignored, Cargo will produce the same build outcome, just with worse error messages.
It still avoids resolving unavailable versions via the `yanked: true` shim. If it is forced
to resolve an unavailable version via a lockfile, it sees an explanatory 404 from the registry.

The cost of bumping `v` is effectively the same as omission for older versions of Cargo, which
seems strictly worse ("version not found" rather than an explanatory error). The unstabilized experimental
`v: 3` also makes a bump awkward since we do want `availability` to be immediately usable, before
the other v3-gated features stabilize.

#### Right, but why not use an index protocol bump itself as a way to prevent access?

It's true that we could index lines as a version that is not supported by modern Cargo,
so that the resolver avoids them. This fully mitigates even the case where old Cargo
ignores unavailable statuses and tries to resolve the yanked (from its perspective) line due to a
lockfile. And then we could manipulate the version back down again upon leaving
withholding.

But, this option seems pretty painfully hacky and confusing. We ideally shouldn't be 
mutating index line protocol versions in place. It overloads the semantics
of versioning to mean something than "this will cause Cargo to misbehave" (which is
not true, as previously explained).

It does not seem worth the added complexity of overloading `v=` to be mutable,
to avoid overloading the (already overloaded) `yanked=true`.

#### Why no end-user-triggered quarantine?

The `availability` primitive is appropriate for usage by end users via a yank-like
API. But, this brings in further UX and policy questions, registry web API,
and other discussion that is less relevant to the goals of this RFC.

It is best deferred to a later proposal.

#### Why not withhold newly published versions by default?

This RFC operates under the assumption that packages won't be held by default on publish.
Rather, withholding is done after publish or in extremely suspicious situations.

Support for withholding all crates by default belongs in a different RFC because
it has significant implications on user experience, raises concern around index and CDN
thrash, and generally deserves a full design treatment. Likely it should be discussed
alongside broader refactors to publish workflows to include user-managed upload staging as distinct
from a release action.

## Prior art
[prior-art]: #prior-art

### Cache invalidation for verifiable mirrors

The [ongoing Verifiable Mirror Project Goal](https://goals.rust-lang.org/2026/mirroring.html) will mostly
address the "stale-cache" risk that currently impacts deleted crates and will impact quarantined crates.

Because the TUF allows vending a verifiable merkle tree for all index content, Cargo can cheaply evaluate
subtrees such as individual crate files for consistency with upstream. This allows Cargo to invalidate its
cache if it sees a new change, such as a quarantine on an already-cached crate's index metadata.


### Withholding across ecosystems

Across language ecosystems, the most crates.io-like registries (PyPI, npm), have implemented
reversible quarantine, enforced via omission from the index. Both have seen user pain
due to illegibility. PyPI has added additional markers, and npm plans to.

Other registries have variants of deletions and yanks, but no reversible quarantine at
the core registry level.

<details>
  <summary>See roundup</summary>

#### Rust

[RFC 3660](https://rust-lang.github.io/rfcs/3660-crates-io-crate-deletions.html) adds destructive
deletes by removing a crate's index file entirely. It temporarily reserves the crate name upon deletion.

`availability` is a non-destructive counterpart to this, operating on the per-release
rather than per-crate level. `withdrawn` is the terminal, non-destructive equivalent.

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
- Is the yanked handling of pubtime correct for quarantined and withdrawn packages? IE, totally
orthogonal, no impact on pubtime (which is set on initial publish only)
- Do we need to also extend the registry spec to allow advertising a URL with which to
retrieve information about withdrawal reasons (or eventually, yanked)? This could
provide human-friendlier error messages, but it is a bit of scope creep. For instance:
```
{
  "web": {
    "version": "https://crates.io/crates/{crate}/{version}"
  }
}
```
- Do we care to trigger fresh docs.rs builds for unavailable-at-publish-time releases that
never had docs? This is technically a reachable state today, if a registry freezes
an entire account and quarantines on release instead of locking, but it seems more
likely to be encountered in the later RFC on publish-time withholding.

### To resolve during implementation
- Is `availability` a good field name in the index, or something more generic like `status`?
   - `status` reads more naturally, but it might invite registries to add other states
   that DO still allow installs, and then it blurs the lines of whether this field
   always connotes uninstallable

- Exact errors and prose notes, documentation notes, documentation URLs

### Related problems this RFC leaves open
- Dependents of an unavailable version are published and installable but broken until it is released. Propagation, 
linking, or docs.rs re-processing are registry- and docs.rs-side problems for a later RFC once we add `unreleased`
publish-time holds.
- Warm-cache exposure (stale index + cached crate bytes) is pre-existing and un-addressed here
- Tombstones for author-deleted crates: we could use `withdrawn` for this (and probably
should) but that should be a separate RFC since it has other implications related to
author privacy.
- What to name a crates.io researcher download location (to be discussed in
subsequent crates.io-side issue/PR)

## Future possibilities
[future-possibilities]: #future-possibilities

#### `cargo info` support
- `cargo info` shows only candidate versions, meaning not yanked or unavailable ones. It could show both,
with a marker, since the index has that state.

#### Better display of reasons for withholding
- A registry endpoint could serve machine-readable reasons that Cargo shows in errors, or reasons on the index
line itself
- This should be designed along with `yanked` reason improvements

#### Additional variants like "unreleased"
- We probably want to indicate "held at publish time but not necessarily misuse"
- This would need additional handling to support release trains (ie, opt-in way to access held bytes via Cargo)
- This will come in a subsequent RFC to avoid scope creep
