- Feature Name: `registry_crate_unavailability`
- Start Date: 2026-10-07
- RFC PR: [rust-lang/rfcs#4016](https://github.com/rust-lang/rfcs/pull/4016)

## Summary
[summary]: #summary

This RFC adds a new, optional registry index field, `availability` (`available | unavailable`).

It also specifies how Cargo handles unavailable versions during dependency resolution and reports their unavailability in errors.

This is part of the proposed [crates.io supply chain hardening Project Goal](https://goals.rust-lang.org/2026/crates-io-supply-chain-hardening.html).

## Motivation
[motivation]: #motivation

This RFC is the first step towards a goal of hardening crates.io and other registries to be able to
rapidly respond to possible attacks using non-destructive actions and, ultimately, pre-emptively hold likely
malicious bytes for pre-release reviews. The big-picture plan [is in the crates.io supply chain hardening Project Goal](https://goals.rust-lang.org/2026/crates-io-supply-chain-hardening.html).

During security investigations, crates.io operators are slow to take destructive actions like
blanket-deleting all crates of an account. They could move more rapidly, with less risk of collateral
damage, if they could "freeze" that account: something that prevents users from installing malicious
crates while still being easy to reverse if it turns out to be a false alarm. That same capability will also be needed for any sort of automated defense.
Publish-time holds will be addressed in a separate RFC.

To accomplish this goal, we add a middle ground between "available" and "erased from the index" via the
new `availability` field. We have "yanked", but yanked does not indicate a malicious release.
Yanked releases remain downloadable and usable with an existing `Cargo.lock`. We instead
want a way to say, "this release is being quarantined, and its bytes are not served via the normal download path". And 
similarly, if we ultimately delete a release, we'd like the option to leave a tombstone showing that it used to be 
there. Registries may retain and serve unavailable bytes for research purposes. This RFC does not add Cargo support
for fetching them.

A key tradeoff of this approach, unavailable crates staying in the index, is that it breaks the invariant that every
indexed version is downloadable from `dl`. We accept this because omitting the index entry can cause pinned
versions to fail with no useful explanation. Other registries (PyPI, npm) that started with omission have
since added explanatory markers or plan to.

The security boundary that we are focused on here is fresh acquisition, with a cold cache. Cargo avoids unavailable bytes 
whenever its view of the index is fresh, but the combination of a warm crate cache and a stale, cached view of the 
index that is missing a newer `availability` marker, will allow continued use of unavailable bytes. This problem also affects
deletion today.

Out of scope for this RFC:
- publish-time holds (which do not imply misuse, for instance publish-time review)
- publishing against unavailable versions (mainly a concern for publish-time holds)
- fetching unavailable bytes via Cargo, for forensic or publish-time builds, with a registry-advertised location
to access those bytes
- end-user-triggered quarantine
- withholding newly published versions by default
- evicting unavailable versions or index data from local caches
- new criteria for manual crates.io administrator action beyond the existing policies for administrator yanks and deletions
- crates.io automated detection and withholding systems
- reason index fields such as `availability_reason` and `yanked_reason`
- a general mechanism for registries to advertise pages with version information like availability context

The following will be implemented as a result of this RFC, but design specifics are deferred to a crates.io-side discussion:
- crates.io implementation of manual withholding
- crates.io frontend display of unavailable statuses (they will be displayed in the UI in some form, similar to docs.rs display)

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

`availability` is an optional index field with two values: `available` and `unavailable`.
Other values may be added in future RFCs, and clients should be forward compatible by
treating unknown values as `unavailable`.

`unavailable` means the registry does not serve the version's `.crate` file via the normal download path.
This covers both temporary quarantine and permanent withdrawal; the index does not distinguish them.

`available` is equivalent to a value of `null` and indicates that the index row has no availability
restrictions.

Cargo skips unavailable versions when choosing new dependency versions. If no other version satisfies the
requirement, or if Cargo needs the contents of an unavailable version pinned in `Cargo.lock`, it reports an error:

```
error: package `base64squatter v1.0.0` is unavailable
  |
  = note: for more information, see https://crates.io/crates/base64squatter/1.0.0
required by package `myapp v0.1.0 (/home/user/myapp)`
  |
  = note: this version is pinned by Cargo.lock; run `cargo update base64squatter` to select a different version, or change the requirement in Cargo.toml if no other version satisfies it
```

For crates.io, Cargo includes a link to the affected version's page, where crates.io explains
the version's status and any possibility of restoration.

Registries also mark unavailable versions with `yanked: true`, so older Cargo and other tools route around them.
If non-status-aware tools fetch the version anyway, for instance due to a pinned lockfile, they should get a 404,
which should include an explanation of the version's state in the body. Cargo
displays this body on fetch failure.

The reverse is not true: Direct yanking, whether by author or registry administrators,
does not imply that the crate is made unavailable.

For researchers, unavailable versions are in the index, and a registry may document a way to fetch unavailable bytes.
Fetching unavailable bytes via Cargo may be covered in a future RFC.

For maintainers, Cargo excludes unavailable versions when selecting new dependency versions for a lockfile.
Publication fails with a useful error if it requires an unavailable package’s contents.

docs.rs does not build unavailable versions and shows a status badge instead.

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

### Added to [Registry Index / JSON schema](https://doc.rust-lang.org/cargo/reference/registry-index.html#json-schema)

*Amended, new "availability" field after "yanked"*

```javascript
{
    [...]
    // Boolean of whether or not this version has been yanked.
    "yanked": true,
    // The availability status of this package version (optional).
    //
    // If set to a value besides "available" or null, Cargo skips this
    // version when choosing new dependency versions. Cargo reports an
    // error explaining the unavailability if no other version satisfies
    // the requirement, or if this version is pinned in `Cargo.lock`
    // and its contents are needed.
    //
    // If omitted or null, Cargo treats this version as "available".
    //
    // For unavailable versions, a `.crate` file is not served from a registry's `dl` endpoint
    // while the package version is unavailable.
    //
    // The current values are:
    // * "unavailable": The registry does not serve this package version's
    //   `.crate` file from its `dl` endpoint. This may be temporary or permanent.
    // * "available": Available without restrictions.
    //
    // All unknown values are treated as "unavailable".
    "availability": "unavailable",
    [...]
}
```
*Amended, availability as mutable*

The JSON objects should not be modified after they are added, except for the
`yanked` and `availability` fields, whose value may change at any time.

---

For compactness, crates.io will not rewrite its index proactively to include `availability = "available"`.

Elsewhere in this RFC:
- Drawbacks: [A second mutable index field](#a-second-mutable-index-field)
- Rationale: [Why no index protocol bump?](#why-no-index-protocol-bump)
- Rationale: [Why `availability` instead of (further) overloading `yanked`?](#why-availability-instead-of-further-overloading-yanked)
- Rationale: [Why write unavailable releases to the index?](#why-write-unavailable-releases-to-the-index)
- Rationale: [Why no reason text in the index?](#why-no-reason-text-in-the-index)

### Added to [Registry Index / Unavailable versions](https://doc.rust-lang.org/cargo/reference/registry-index.html#unavailable-versions)

*New section after "Version uniqueness":*

A registry may withhold a package version by setting the `availability` field
in its index entry to `unavailable` and also setting `"yanked": true`,
so that tools unaware of `availability` avoid the version. Yanking a package directly
does not change the value of `availability`.

An unavailable package version's `.crate` file is not served from the registry's `dl` endpoint. The endpoint should
respond 404 and serve a body explaining the status and/or linking to a webpage about
the crate's status and/or registry policies. The body should not name an
alternative download location.

When a period of unavailability ends, the registry restores the author's requested
yank state. A yank or unyank requested while the package version is unavailable
is recorded and takes effect when the version is restored.

The value of `pubtime` is not impacted by changes to the `availability` field.

---

Elsewhere in this RFC:
- Rationale: [Why `availability` instead of (further) overloading `yanked`?](#why-availability-instead-of-further-overloading-yanked)

### Added to [Dependency Resolution / Unavailable versions](https://doc.rust-lang.org/cargo/reference/resolver.html#unavailable-versions)

*New section after "Yanked versions":*

Cargo skips [unavailable releases][unavailable] when choosing new dependency versions.
If no other version satisfies the requirement, it reports an error explaining the unavailability.
If an unavailable version is pinned in `Cargo.lock`, Cargo reports an error only if its contents are requested.

[unavailable]: registry-index.md#unavailable-versions

---

For crates.io, the error includes a link to `https://crates.io/crates/{crate}/{version}`
for more information. Cargo constructs this URL without an additional registry request.
For other registries, Cargo reports the unavailability without a version-page link.

A possible error for a locked unavailable version is in the guide section. Ideally
we would like to suggest `cargo update <crate>`, though that might not make it into the
initial release.

If a version is restored, builds can use it again without changing `Cargo.lock`.

Cache lifecycles are unchanged. An unavailable version keeps building if it was last seen
not-unavailable, and has its bytes and index file cached.

Elsewhere in this RFC:
- Drawbacks: [Existing lockfiles can break without a local change](#existing-lockfiles-can-break-without-a-local-change)
- Drawbacks: [Warm caches keep unavailable versions buildable](#warm-caches-keep-unavailable-versions-buildable)
- Future possibilities: [Warm cache invalidation enhancements](#warm-cache-invalidation-enhancements)


### docs.rs

docs.rs uses availability status information and any additional context provided by crates.io
to determine build and display behavior.

On seeing an unavailable version, docs.rs skips the version's build and shows a badge stating its status.
Any retained documentation appears below the badge.

When availability is restored, docs.rs drops the badge but does not trigger
a rebuild unless manually requested.

For unavailable versions, crates.io displays the version's availability and provides any additional context
about the reason for the version's status.

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
be nice to have a better solution.

#### Warm caches keep unavailable versions buildable
- True of any design without cache invalidations.
- Already the case for current crate deletion practices
- For alternatives, see: Future possibilities: [Warm cache invalidation enhancements](#warm-cache-invalidation-enhancements)

#### Prevents reuse of withdrawn version numbers
- If an attacker publishes malicious versions, an `unavailable` tombstone could occupy "good" versions
and prevent the author from re-using them.
- This is an acceptable drawback to prevent end-user confusion, particularly if the version has a RUSTSEC
notice published against it.
- Registry administrators remain able to apply conventional deletes that free up a version for reuse,
based on registry policy.

#### Fresh resolution can fail for inactive dependencies
- Unavailable dependencies of disabled workspace features or other targets can still prevent fresh resolution.
- Avoiding this would require a new resolver policy allowing unavailable versions as a fallback and removing the yank shim.
- For now, Cargo follows yank behavior for new selections. Pinned unavailable versions cause an error
only when their contents are requested.

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

### Requirements

Essential:
1. Fresh fetches of an unavailable version through default registry paths fail with diagnostics that explain the status, 
both for Cargo, and for other tools, including ones that do not consult full index
resolution rules to decide what to fetch (Yocto, registry mirrors, etc)
2. Existing Cargo releases have a safe default behavior with no code changes
3. Tools can differentiate unavailable releases on the wire from other statuses like yanked, deleted, and never-published
4. Withholding is reversible without requiring a fresh publication, loses no information, and a restored
version is treated the same by tools as one which was never unavailable
5. Index followers such as docs.rs handle a version while unavailable and once restored
6. Mirrors and index signing keep working unchanged
7. Unavailable versions are discoverable from the index, without additional requests

Preferred:
1. Reuse the existing index, version model, synchronization, and release flow (especially, no index protocol bump)

### Major architectural alternatives

#### Why indicate status with an enum rather than a bool?

An enum leaves room for future states that require different handling, such as access through
an alternate download path, without changing the field's type.
Clients that do not recognize a new value treat it as `unavailable`.

#### Why include `availability` (with `available`) rather than `withheld`?

We don't want to have a negative connotation on crates that are withheld, since this does not
necessarily indicate malicious activity. A neutral field name like `availability` leaves us
open for alternative, softer statuses, without the negative implication.

The downside of this is slightly more complexity for downstream tooling, which now needs
to check the enum rather than assuming presence of the field indicates unavailability. This
is partially mitigated by defining everything besides `available` or `null` as inaccessible, and
equivalent to `unavailable` if unknown. This allows tools that are uninterested in more granular
status to handle this field with a straightforward equivalence check.

We prefer `availability` to `available` because `"available": "available"` is weird looking.

#### Why write unavailable releases to the index?

PyPI's quarantine and npm's publish-time scanning both remove
or never write a line. For a version that someone has already locked,
omission produces confusing resolution failures with no indication of why.
Both registries have [seen complaints](https://github.com/orgs/community/discussions/203413)
for this. PyPI has since added status markers, and npm plans to.

Writing the index entry lets Cargo explain that the version is unavailable and give
guidance to the user (such as `cargo update`). For crates.io, Cargo also links to the version's page.
Keeping entries marked `unavailable` as tombstones transparently reserves package versions, unlike omission.
It still lets older Cargo fall back to yanked behavior that avoids resolving to the unfetchable version.

This fits the values of the project around transparency and
accountability. If we are making it easier for administrators to take new
curation actions on the index, we want transparency logs and auditability.
Showing the unavailable package versions also makes it possible for researchers to
discover and access unavailable bytes (though this RFC defers this as a registry decision).

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

Whether versions held at publish time (with no misuse implied) should receive similar
treatment is deferred to a subsequent RFC, as they involve different tradeoffs and systems.

#### Why not add raw.crates.io and make crates.io a mirror of it that only serves non-unavailable versions?

This is still omission from the consumer's point of view, with all the same downsides around
legibility and clear build tool behavior. It has advantages in comparison to omission in that it preserves
an audit log, and that it makes it possible for researchers to enumerate unavailable versions.
But, writing the `availability` line to the index offers the same benefits.

In exchange, it requires building a second index, which adds a systems complexity
and operator burden. It also makes it possible for Cargo to access all unavailable bytes
with no warnings or guardrails. A future RFC will suggest a method of accessing
unavailable bytes that instead is scoped to explicitly requested bytes.

#### Why `availability` instead of (further) overloading `yanked`?

A yanked version remains downloadable and usable through existing lockfiles. Making `yanked` also mean
that the bytes might be unavailable would leave clients unable to distinguish those cases.

`availability` makes that distinction explicit, so Cargo can reject unavailable versions even when pinned
in a lockfile. Registries also set `yanked: true` for unavailable versions so older clients avoid them during
new dependency resolutions.

A registry could also adopt a policy of reserving standalone yanks for author actions and using
`availability` for administrator withholding.

### Other decisions

#### Why no reason text in the index?

The reason for withholding, including whether a version may be restored, is stored registry-side.
For crates.io, Cargo links to the affected version's page for more information. This is a
crates.io-specific diagnostic and does not add a registry metadata field or require another request.

Reason fields such as `availability_reason` and `yanked_reason`, and a general mechanism for registries
to advertise version-page URLs, are deferred. See Future possibilities:
[Better display of reasons for withholding](#better-display-of-reasons-for-withholding).

#### Why no index protocol bump?

`v` exists for changes that older Cargo would misinterpret in a way that causes incorrect behavior.
A higher `v` makes it ignore the entry entirely. Cargo already skips unknown fields.

If `availability` is ignored, Cargo will produce the same build outcome, just with worse error messages.
The `yanked: true` shim still excludes unavailable versions from new selections. If Cargo needs to download
an unavailable version pinned in `Cargo.lock`, it sees an explanatory 404 from the registry.
The main gap is that older Cargo can still use locally cached bytes for an unavailable
version pinned in `Cargo.lock`, despite `yanked: true`. This is an acceptable risk given that this RFC only
makes security claims around cold cache usage.

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
rather than per-crate level.

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
- Is the yanked handling of pubtime correct for unavailable packages? IE, totally
orthogonal, no impact on pubtime (which is set on initial publish only)
- Do we care to trigger fresh docs.rs builds for unavailable-at-publish-time releases that
never had docs? This is technically a reachable state today, if a registry freezes
an entire account and quarantines on release instead of locking, but it seems more
likely to be encountered in the later RFC on publish-time withholding.

### To resolve during implementation
- Is `availability` a good field name in the index, or something more generic like `status`?
   - `status` reads more naturally, but it might invite registries to overload it to
   indicate statuses where bytes are still available via `dl`, which breaks the "unknown
   means unavailable" invariant.
- Exact errors and prose notes, documentation notes, documentation URLs
- Whether crates.io will set mark versions as `availability: available` when restoring from `availability: unavailable`,
or drop the `availability` field from their index entry

### Related problems this RFC leaves open
- An unavailable version can leave dependent packages published but unable to build. Propagation,
linking, or docs.rs re-processing are registry- and docs.rs-side problems for a future RFC on publish-time holds.
- Warm-cache exposure (stale index + cached crate bytes) is pre-existing and un-addressed here
- Tombstones for author-deleted crates: we could use `unavailable` for this (and probably
should) but that should be a separate RFC since it has other implications related to
author privacy and version reusability.
- What to name a crates.io researcher download location (to be discussed in
subsequent crates.io-side issue/PR)

## Future possibilities
[future-possibilities]: #future-possibilities

#### `cargo info` support
- `cargo info` shows only candidate versions, meaning not yanked or unavailable ones. It could show both,
with a marker, since the index has that state.

#### Better display of reasons for withholding
- Reason fields such as `availability_reason` and `yanked_reason` could let Cargo display explanations
without requiring users to visit the registry's website. They should be designed together.
- A registry endpoint could also serve machine-readable reasons.
- A general mechanism for registries to advertise version-page URLs could extend the crates.io-specific link
to other registries.

#### Publishes against unavailable crates
- If we want to add systems that might hold releases at publish time, we
need a way to avoid breaking release trains, including via separate `cargo publish` invocations.
- A possible solution to this problem would be to reuse Cargo's workspace publish index overlay machinery
to allow a publish invocation to request that crate paths be included in an overlay. 
- This will come in a subsequent RFC to avoid scope creep.

#### Additional variants like "quarantined"
- We might want to indicate that bytes are unavailable via `dl` but are available via an alternate download
location, for instance for forensic builds.
- This deserves its own discussion since it has implications around registry policy, Cargo handling, and more.

#### Warm cache invalidation enhancements

- The [ongoing Verifiable Mirror Project Goal](https://goals.rust-lang.org/2026/mirroring.html) will improve the "stale-cache" risk
story by allowing invalidation of caches across all crates with a single network call, but it won't actively refresh its view of
the index if all needed metadata is cached in the local index
- Another option would be a knob to actively refresh the index, which might be enabled by default based on an environment variable
like `CI=true`
- This is scoped out of this design and probably belongs in the Verifiable Mirror goal, since the shift to a single network call
is what makes this viable in the first place

#### Script to probe for unavailable crates in cache

- In the recent `arrayref` supply chain attack, security responders scrambled to publish a script that could be used to check
for impact, in the form of looking for impacted versions in the local Cargo cache.
- We could offer an "evergreen" script that checks for the presence of any unavailable versions in cache.
- This might be undermined by cache eviction, depending on its implementation
(though presumably we could also add user-facing warnings on cache eviction, or instead move bytes to a
special area of the cache that the script still looks for, but Cargo doesn't use by default)
  - This partitioning of the cache will anyway be discussed in a subsequent RFC, when we need to start fetching certain
  unavailable bytes for release trains.
