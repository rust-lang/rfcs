- Feature Name: `registry_withholding`
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary
[summary]: #summary

This RFC adds a new, optional registry index field, `withheld` (`quarantined | unreleased | withdrawn`).

It also defines two new optional registry `config.json` URL templates alongside `dl`: `dl-withheld`, from which 
withheld bytes can be explicitly fetched by exact coordinates, and `notice-page`, a human-oriented
page displaying a version's status and/or the registries policies, which may be per-version, per-crate, or global.

Lastly, it specifies how Cargo should interact with the new status field and `config.json` templates, for fetches, 
builds, installs, and publishes.

This is the first of three changes that are part of the [crates.io registry response project goal](https://github.com/rust-lang/goals/pull/795).
1. Quarantined/unreleased/withdrawn support in Cargo, registry spec, docs.rs, release-plz (this RFC)
2. crates.io support for quarantine/withdrawn admin APIs, and related admin API work (issue not opened yet)
3. crates.io support for publish-time detections with automatic holds, a manual review queue, and supporting console interface (RFC not open yet)

## Motivation
[motivation]: #motivation

This RFC is the first step towards a goal of hardening crates.io and other registries to be able to
rapidly respond to possible attacks using non-destructive actions and, ultimately, pre-emptively hold likely
malicious bytes for pre-release reviews. The big-picture plan [is in the registry response Project Goal](https://github.com/rust-lang/goals/pull/795).

The gist of the goal is, we want to move towards proactive rather than reactive defenses against supply chain attacks. Right now,
every attack is a fire drill for the crates.io team, and we fail open. The prospective min-publish-age defaults will give us 
some breathing room, but incoming vulnerabilties still require urgent response since fails open if response is not quick enough. And, we expect that
the scope of crates needing rapid response will grow. Our north star, which this RFC moves us towards, is to be able to 
have high-confidence signals at publish-time that we can use to route obviously malicious crates into a human review 
queue.

The first step, which this RFC enables, is to close a gap in crates.io security responses where we are slow to 
take destructive actions like blanket-deleting all crates of an account. We need a way to "freeze" that account 
during investigation. That same capability will also be needed for any sort of automated defense.

This RFC attempts to give the registry spec that middle ground between "released" and "permanently deleted". We have
"yanked", but yanked does not indicate a malicious release. Yanked releases are also still resolvable via an existing
`Cargo.lock`, and, worse, their bytes are freely downloaded by default. We instead want a way to say, "this release is 
being quarantined, and you can go look at the bytes (at your own risk), but you need to opt into it via a 
separate, explicit path". And similarly, if we ultimately delete a release, we'd like the option to leave a tombstone 
showing that it used to be there, and continue to allow researchers to view bytes for some amount of time.

Along the way, we need answers for questions like, "well what if I want to publish a release that relies on quarantined bytes?"
and, "what if I publish a crate with a checked-in lockfile that points to quarantined bytes"?

The focus of this RFC is the build tool and registry spec end for withholding releases: Cargo (and docs.rs) should
have a good experience when interacting with withheld releases. Tools that have not added support for withheld releases 
will still by default avoid resolving withheld releases where possible, and otherwise fail with a clear 
pointer to information on the release's status rather than silently installing withheld bytes. 

The security boundary that we are focused on here is fresh acquisition, with a cold cache. Cargo avoids held bytes whenever its view of the index is fresh,
but the combination of a warm crate cache and a stale, cached view of the index that is missing a newer `withheld` marker, will allow continued use of 
withheld bytes.

Out of scope for this RFC, but in a subsequent one:
- The criteria registries will use for quarantining, pre-publish holding, or withdrawing 
releases. Only the wire representations and Cargo handling are defined here to keep all Cargo changes consolidated to a 
single RFC and also build in first-class support for these behaviors into Cargo as far ahead of registry adoption as 
possible.
- The actual mechanism that crates.io will use to manage lifecycle transitions to/from quarantined, unreleased, and withdrawn statuses.
- crates.io mechanisms for surfacing quarantined/unreleased/withdrawn status in its web frontend or web API
- Systems for proactively "holding" releases for review at publish-time or supporting author-driven staging
(More discussion in [Unresolved questions](#unresolved-questions) and "Future possibilities")

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

### Index field: "withheld": "quarantined | unreleased | withdrawn"

Cargo index data adds a new `withheld` field, with the possible values of `quarantined | unreleased | withdrawn`. 
A version with this field is not selected by Cargo or other build tools, and its bytes are not served via normal 
download endpoints. A version without this field is available as usual.

- `unreleased` — the version has been uploaded to the registry but is not yet available, whether because the author 
staged it or because the registry is reviewing it.
- `quarantined` — the registry has frozen a version due to suspected misuse, typically during a security 
investigation. It may be released again or withdrawn.
- `withdrawn` — the version has been permanently removed; the coordinate stays reserved so it cannot be reused.

When and why a version moves between these states is a registry decision and not recorded in the index. Whenever
you encounter a withheld version (resolving versions, building a crate, publishing crates, reading docs.rs, etc), Cargo
will explain that the version is not installable and share pointers to any registry-provided information about it.

### Cargo resolution

Cargo will not select withheld versions. On a non-locked resolve, Cargo skips any version with `withheld` present
and chooses the newest alternative that satisfies the version requirement. So, in most cases, you will not encounter
withheld versions.

You will see an error only if nothing else satisfies a version requirement. This happens most commonly if `Cargo.lock`
pins a version that was withheld after you locked it. Unlike a yanked version, which Cargo will quietly use when pinned,
withheld versions are rejected:

```
error: failed to select a version for the requirement `base64squatter = "=1.0.0"`
  version 1.0.0 is quarantined
location searched: crates.io index
required by package `myapp v0.1.0 (/home/user/myapp)`
  |
  = note: this version is pinned by Cargo.lock; run `cargo update base64squatter` to select a different version, or change the requirement in Cargo.toml if no other version satisfies it
  = help: for more information see https://crates.io/crates/base64squatter/1.0.0
```

The last line comes from the registry, when it advertises a notice page.

It is possible to force Cargo to resolve withheld versions, and, if a registry advertises support, fetch withheld bytes.
But, Cargo will never include copy-pasteable commands to force this behavior. It is reserved as an explicit opt-in for
security researchers and publishers (see below).

Older versions of Cargo, and other tools that do not yet understand `withheld`, still avoid withheld versions due to
registries setting these entries as yanked. If the tools attempt to request withheld bytes anyway, for instance due
to a lockfile pin, they fail with a helpful not-found message from the registry (see [Serving withheld bytes with `dl-withheld`](#serving-withheld-bytes-with-dl-withheld)).

One gap remains for all Cargo versions: if both the index file and the crate bytes were cached before a version was 
withheld, builds keep using the cached copy until something refreshes the index (see [Stale-cache risks](#stale-cache-risks)).

### Registry handling

Registries whose index might include withheld versions can add up to two optional URL templates to `config.json` 
using the same template tags as `dl`:

- `dl-withheld`: withheld bytes can be explicitly requested from this endpoint (by security researchers, publishers 
rebuilding against their own withheld crates, etc). If this is not set, withheld bytes are never available.
- `notice-page`: a human-oriented page, which may be per-version, per-crate, or a global policy page that Cargo links
to from errors.

A registry can adopt any subset of these; a mirror might only set `notice-page` and point to an upstream's page, or
it might also mirror withheld bytes using `dl-withheld`. 

Whenever a registry withholds a version, it also marks its index line as `yanked: true` to help steer older build
tools around withheld versions. Author-initiated yanks are still preserved after withholding lifts.

### Security researcher usage

Withheld versions are noted in the index, so scanners and researchers can track which releases are withheld
and when.

Registries that set `dl-withheld` also make the withheld bytes available for triage and forensic analysis. The bytes can be fetched directly, for example `GET https://static.crates.io/withheld/base64squatter/1.0.0/download` or
through Cargo flags for forensic builds: `cargo fetch --withheld base64squatter@1.0.0`, `cargo build --fetch-withheld base64squatter@1.0.0`, `cargo install --fetch-withheld base64squatter@1.0.0`. The flags require exactly `name@version`.

When the flags are used, they only admit withheld crates for that specific invocation. The crate bytes themselves
are cached under a separate `withheld~` namespace to avoid other builds picking them up by accident. Plain builds will
attempt to fetch bytes from `dl` and fail.

### Maintainer usage

Publishing continues to work when a registry withholds something mid-release, whether in a single
`cargo publish --workspace` call or across separate invocations (such as via `release-plz`).

Cargo by default treats an upload that results in an `unreleased` status or depends on `unreleased` versions as a 
success: Cargo reports the state, continues on, and later crates that depend on the unreleased version are still published. Registries 
might have different reasons for holding uploads as unreleased (such as cooldowns, automated reviews, or author staging),
so routine holds don't break releases.

Meanwhile, an upload that is marked as `quarantined` or `withdrawn`, or depends on a quarantined/withdrawn version,
is treated as a failure. Cargo skips the upload ahead of time when a dependency is already known to be quarantined or
withdrawn. When encountering a quarantined or withdrawn upload result or dependency, Cargo will stop publishing 
further crates, emit an error, and exit with a failure code.

Timeout handling is unchanged: If Cargo times out while waiting on a crate to upload that has subsequent
uploads depending on it, it errors and exits as a failure. If it times out on a crate with no dependents, it warns
and continues as a success.

These behaviors can be changed from their defaults via optional flags: `--fail-on-unavailable` will abort and exit
whenever a version is not installable, due to withholding or timeout. `--continue-on-quarantined` will continue onwards
similar to `unreleased` handling (with a warning).

| | upload comes back `unreleased` | upload comes back `quarantined` or `withdrawn` | poll times out |
|---|---|---|---|
| *default* | continues | **fails** | same as today |
| `--fail-on-unavailable` | fails | fails | fails |
| `--continue-on-quarantined` | continues | continues (with a warning) | same as today |

Example outputs:


Own upload found `unreleased` (continues):
```
    Uploaded a v1.2.3 to registry `crates-io`
note: waiting for a v1.2.3 to be available at registry `crates-io`
help: you may press ctrl-c to skip waiting; the crate should be available shortly
       Found unreleased a v1.2.3 at registry `crates-io`
note: a v1.2.3 was accepted by registry `crates-io` and is awaiting release; it is not yet installable
  |
  = help: for more information see https://crates.io/crates/a/1.2.3
```

Own upload found `quarantined` (fails):
```
    Updating crates.io index
   Packaging a v1.2.3 (/home/user/ws/a)
    Packaged 6 files, 12.3KiB (4.1KiB compressed)
   Verifying a v1.2.3 (/home/user/ws/a)
   Compiling a v1.2.3 (/home/user/ws/target/package/a-1.2.3)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.02s
   Uploading a v1.2.3 (/home/user/ws/a)
    Uploaded a v1.2.3 to registry `crates-io`
note: waiting for a v1.2.3 to be available at registry `crates-io`
help: you may press ctrl-c to skip waiting; the crate should be available shortly
       Found quarantined a v1.2.3 at registry `crates-io`
error: a v1.2.3 was quarantined by registry `crates-io` after upload
  |
  = note: the upload succeeded; a v1.2.3 remains on the registry in quarantined state and is not installable
  = help: for more information see https://crates.io/crates/a/1.2.3
```


### docs.rs impact

docs.rs does not build withheld versions. A version that is `unreleased` or `quarantined` shows a status badge
on its docs.rs page and links to the registry's information about it. If the documentation was already built before
the version was quarantined, it stays online, below the badge. When a version exits withholding, docs.rs
builds it (if it has not already successfully built the version's docs) and removes the badge.

A `withdrawn` version keeps its badge permanently and its documentation is removed.

Dependents whose builds fail because they resolve solely to withheld versions are handled as if they are ordinary
build failures. docs.rs already triggers fresh resolution on failure, so the docs will only fail to build if no other
compatible version exists. Further improvements are discussed in Future possibilities: [Re-processing failed docs.rs builds when a dependency becomes available](#re-processing-failed-docsrs-builds-when-a-dependency-becomes-available).

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The key words MUST, MUST NOT, SHOULD, SHOULD NOT, and MAY in this section are to be interpreted as described in [RFC 2119](https://www.rfc-editor.org/rfc/rfc2119).

### Index schema

We add a `withheld` field, which is orthogonal to the existing `yanked` field.

```json
{ "name": "foo", "vers": "1.2.3", "yanked": true, "withheld": "quarantined", "cksum": "..." }
{ "name": "foo", "vers": "1.2.4", "yanked": true, "withheld": "withdrawn", "cksum": "..." }
{ "name": "foo", "vers": "1.2.5", "yanked": true, "withheld": "unreleased", "cksum": "..." }

```

The initial possible values of `withheld` are: `"quarantined" | "unreleased" | "withdrawn"`. Absence of
a `withheld` entry is equivalent to being published with no withholding.

Presence of `withheld` means that the version is not installable. Registries MUST NOT use `withheld` for informational
annotations (deprecation, curation status, etc). Every line carrying `withheld` MUST also carry `yanked: true` (see 
[Managing withheld status transitions](#managing-withheld-status-transitions)).

Definitions:
- `unreleased`: an entry that was submitted to the registry via a publish API, but has not yet been made
available to consumers, regardless of who initiated the hold. This is a routine state rather than a value judgement.
It might be due to some author-driven staging API, or it  might be due to registry systems probing for security risks.
The origin of the hold is not disclosed on the wire. The entry may transition to a released status or `withdrawn`.
- `quarantined`: the registry has frozen a version because it suspects misuse. This can happen to a version that
previously was installable, or on arrival (for instance if the publishing account itself is frozen). The version may
be released, at which point the index will remove the `withheld` field. Or, it may transition to "withdrawn" status
and be kept permanently uninstallable.
- `withdrawn`: a terminal state that indicates that an entry is permanently not installable, with its line kept as a 
tombstone that reserves its `name@version` coordinate. Whether the withdrawal is due to misuse is not recorded in the 
index and left for external systems to disclose as described in the "Registry handling" section.

Build tools MUST treat unknown statuses as equivalent to `quarantined`, because all versions with a `withheld` field
are uninstallable and `quarantined` is the strictest handling that is generally applicable. Build tools SHOULD include 
a warning naming the unknown value but MUST handle it as if the release was `quarantined`.

A withheld-status-aware build tool MUST NOT resolve withheld versions regardless of yanked status, including when the
version is present in a lockfile, unless explicitly directed to (see [Fetching withheld bytes with `--fetch-withheld`](#fetching-withheld-bytes-with---fetch-withheld)).

Related:
- Drawbacks: [A second mutable index field](#a-second-mutable-index-field)
- Rationale: [Why no index protocol bump?](#why-no-index-protocol-bump)
- Rationale: [Why `withheld` instead of (further) overloading `yanked`?](#why-withheld-instead-of-further-overloading-yanked)
- Rationale: [Why write withheld releases to the index?](#why-write-withheld-releases-to-the-index)
- Future possibilities: [Delayed indexing for unreleased](#delayed-indexing-for-unreleased)

### Registry handling

This section specifies the registry-facing contract adopted by Cargo and other tools. How registries decide on
or implement state transitions is out of scope.

#### Serving withheld bytes with `dl-withheld`

Registries MAY support accessing withheld bytes. A registry advertises support for this via a `dl-withheld` field in 
`config.json`. 

The `dl-withheld` value is a URL template that follows the same interpretation as `dl`. It may contain the template
markers of `{crate}`, `{version}`, `{prefix}`, `{lowerprefix}`, and `{sha256-checksum}`. Otherwise, Cargo appends `/{crate}/{version}/download`.

```json
{
    "dl": "https://static.crates.io/crates",
    "api": "https://crates.io",
    "dl-withheld": "https://static.crates.io/withheld"
}
```
(The preceding example shows `dl-withheld` using a static crates.io endpoint, but this is up to the registry.)

If `dl-withheld` is supported, registries MUST make `unreleased` and `quarantined` bytes available via the
`dl-withheld` path for the duration of the withheld state. Once a version is released, `dl-withheld` SHOULD NOT
continue to serve it (it MAY return 404 or redirect to `dl`).

Registries MAY additionally make `withdrawn` bytes available via the `dl-withheld` path for a duration of their choosing.
If they do, they SHOULD remove the bytes after a period of time rather than hosting them indefinitely.

Bytes served from `dl-withheld` MUST match the `cksum` in the version's corresponding index line. Cargo uses the same
checksum-verification as used with `dl` bytes.

A GET to the corresponding URL returns the `.crate` bytes for the withheld version. For example, with the config.json above, `cargo fetch --withheld base64squatter@1.0.0` requests:

```
GET https://static.crates.io/withheld/base64squatter/1.0.0/download
→ 200 OK
  Content-Type: application/octet-stream
  <.crate bytes>
```

If withheld bytes are requested from a registry that does not advertise `dl-withheld`, Cargo provides
a clear error:
```
    Updating crates.io index
note: admitting quarantined dependency a v1.2.3 via `--fetch-withheld`
error: failed to download `a v1.2.3`

Caused by:
  registry `crates-io` does not advertise `dl-withheld` in its config.json, so withheld bytes cannot be fetched
  |
  = note: a v1.2.3 is quarantined and is not served from the registry's `dl` endpoint
  = help: for more information see https://crates.io/crates/a/1.2.3
```

Registries SHOULD offer useful error messages if a download is requested
via a `dl` endpoint for bytes that are withheld, for instance (as shown
by Cargo):
```
error: failed to download from `https://static.crates.io/crates/base64squatter/1.0.0/download`

Caused by:
  failed to get successful HTTP response from `https://static.crates.io/crates/base64squatter/1.0.0/download` (static.crates.io), got 404
  body:
  This version was quarantined by the crates.io security team and is not available for download.
  See https://crates.io/crates/base64squatter/1.0.0 for details.
```

The registry MUST NOT specify any alternate `fetch-withheld` commands or `dl-withheld` URLs
in these error messages to avoid misuse of the security researcher and maintainer-oriented
paths. Instead, they SHOULD point to either crate-specific information, which MAY contain
the reason for the quarantine, or else general information on the withheld status such
as the registry specification.

Related:
- Rationale: [Why a separate `dl-withheld` path rather than serving withheld bytes from `dl`?](#why-a-separate-dl-withheld-path-rather-than-serving-withheld-bytes-from-dl)
- Rationale: [Why do errors never include a bypass command?](#why-do-errors-never-include-a-bypass-command)
- Rationale: [Why is `dl-withheld` as open as `dl`?](#why-is-dl-withheld-as-open-as-dl)
- Prior art: [Researcher access in other ecosystems](#researcher-access-in-other-ecosystems)
- Future possibilities: [Author-managed staging](#author-managed-staging)
- Future possibilities: [Delayed indexing for unreleased](#delayed-indexing-for-unreleased)

#### Sharing human-facing pages with `notice-page`

Registries MAY advertise `notice-page`, an (optionally templated) URL to include in error outputs that gives human-facing information 
about registry withholding procedures and/or release-specific information.

It may contain the template markers of `{crate}`, `{version}`, `{prefix}`, and `{lowerprefix}`, if the registry
wishes to serve specific pages per specific releases or crates. If no template markers are present, the URL is used
as-is and is assumed to be a single page covering all crates and versions. Unlike `dl`, no path suffix is appended.

```json
{
    "dl": "https://static.crates.io/crates",
    "api": "https://crates.io",
    "dl-withheld": "https://static.crates.io/withheld",
    "notice-page": "https://crates.io/crates/{crate}/{version}"
}
```

The page's contents are unspecified, but it SHOULD include or link to the registry's withholding policies. A per-crate
or per-version page SHOULD additionally show the relevant release statuses.

A registry that sets `notice-page` MUST serve a page at every URL that the template can produce for a withheld version.
It MAY serve the page for other versions.

If `notice-page` is set, Cargo renders it as "= help: for more information see <url>" string in withheld-related
resolution errors, `--fetch-withheld` warnings, and publish-time status outputs. If unset, no `help:`
link is shown. 

Related:
- Rationale: [Why no reason text in the index?](#why-no-reason-text-in-the-index)

#### Authentication

The `dl-withheld` URL template does not require auth, unless the entire registry requires 
auth. In other words, if `auth-required: true` in `config.json`, Cargo attaches its registry token to `dl-withheld`
calls in the same manner as it does for `dl` calls.

This RFC provides no way to require credentials for `dl-withheld` alone. Per-template authentication configuration is 
out of scope. Registries MAY apply throttles or other CDN-layer controls to these paths, as they can for `dl` and 
`api`.

`notice-page` is never fetched by Cargo. Instead, it is accessed via the user's browser, so no token authentication is involved.

Related:
- Rationale: [Why is `dl-withheld` as open as `dl`?](#why-is-dl-withheld-as-open-as-dl)

#### Managing withheld status transitions

The criteria used by registries to withhold a version, and the APIs by which it transitions between statuses,
are unspecified by this RFC.

The only management element that is specified in the registry spec relates to backwards
compatibility: the registry MUST set `"yanked": true` when setting
`"withheld": "quarantined"|"unreleased"|"withdrawn"`. This helps older Cargo versions and other build tools avoid 
selecting withheld versions, even if they are not aware of the `withheld` field.

Because the registry sets `"yanked": true` during withholding, it 
MUST, upon lifting the withheld status, restore the author's own yank state:
- not yanked before, no yank during: `"yanked": false` restored
- yanked before, or yank requested during: `"yanked": true` preserved
- unyank requested during: `"yanked": true` held until exit, then `"yanked": false`

For the final case, an author-initiated unyank during withholding, the registry's yank/unyank web API
SHOULD provide a warning output along with a successful response, indicating that the unyank was registered but will
not be reflected during the withholding period.

Related:
- Drawbacks: [A second mutable index field](#a-second-mutable-index-field)
- Rationale: [Why `withheld` instead of (further) overloading `yanked`?](#why-withheld-instead-of-further-overloading-yanked)
- Rationale: [Why not `"withheld": "quarantined_and_yanked"`?](#why-not-withheld-quarantined_and_yanked)
- Rationale: [Why no index protocol bump?](#why-no-index-protocol-bump)

#### Relationship to `pubtime`

If a registry supports `pubtime`, used with `min-publish-age`, it MUST set its value at the point
it first becomes installable. For a version that arrives as `unreleased`, that is the time of release, not the time
of upload.

While a version is `unreleased`, the registry MAY omit `pubtime` or MAY set it to the upload time. In the second case,
the registry MUST overwrite that `pubtime` upon release.

A version that was installable before it was withheld keeps its original `pubtime` when released from withholding.

#### Impact on byte mirrors and other index consumers

This RFC changes an invariant relied on by downstream tooling: every index line has fetchable bytes via `dl`. Under
this RFC, a withheld line is present in the index but its bytes return 404 via the `dl` path.

This affects tools that fetch bytes for every index line rather than only for versions that a resolver selected, for instance: full byte mirrors like Panamax, caching proxies that eagerly fetch, and docs.rs. Tools that fetch only
resolver-selected versions, such as `cargo vendor` or regular `cargo build` are unaffected beyond pinned lockfiles
failures discussed under [Resolution](#resolution).

Impacted tools SHOULD either skip index lines with `withheld` present, or fetch withheld bytes from `dl-withheld`
and serve them under their own `dl-withheld` (see [Impact on verifiable registry mirroring](#impact-on-verifiable-registry-mirroring)). Until they do either,
they will receive 404s for withheld lines from `dl` and SHOULD treat these 404s as expected rather than a system
failure. Registries SHOULD serve an informational body on these 404s (see [Serving withheld bytes with `dl-withheld`](#serving-withheld-bytes-with-dl-withheld)) to make the cause
immediately apparent to a tool that is not aware of the `withheld` field.

docs.rs is the only impacted consumer whose changes are specified by this RFC (see [docs.rs impact](#docsrs-impact)).

Related:
- Drawbacks: [An index line no longer guarantees fetchable bytes](#an-index-line-no-longer-guarantees-fetchable-bytes)

#### Impact on verifiable registry mirroring

This change is largely transparent to [ongoing verifiable registry mirror work](https://github.com/rust-lang/goals/issues/637). Withheld status is written directly into the index, so it is covered by the same signing and 
verification that applies to the rest of the index. The same freshness caveats apply to the propagation of withheld
status as apply to propagating yanks and deletions (ie, the mirror is verified to reflect upstream for a point in time,
not necessarily the latest point in time).

Mirrors MAY mirror withheld bytes and serve them under their own `dl-withheld`. Whether withheld (or regular) bytes
are served is orthogonal to verification. Verification ensures the accuracy of the index, and the index contains 
checksums that enforce byte integrity. In other words, verification enforces byte integrity, not availability.

### Cargo handling

#### Resolution

Withheld index entries reuse the resolver's candidate-filtering mechanism. 

When a dependency is not pinned (no Cargo.lock, or no existing entry in the Cargo.lock), a new `IndexSummary` variant,
`IndexSummary::Withheld(..)` will be filtered from selection in the same way as `IndexSummary::Yanked`. This means 
that withheld versions will neither be selected from a clean slate nor written to `Cargo.lock` during typical 
resolution. The new variant is also used to surface status-aware error messages on no solution found.

When an existing Cargo.lock contains withheld entries (due to the version being withheld after it was locked, or usage of `--fetch-withheld`),
Cargo handling differs from `Yanked`. A yanked pin resolves, but a withheld pin does not. 

Under a hard lock (no change to `Cargo.toml` has unlocked the source), Cargo honors all pins rather than re-resolving, 
then rejects a withheld pinned version with a status-aware error. The error suggests `cargo update <crate>`, which
re-resolves the named crate to attempt to find a non-withheld version, while keeping every other pin.

Under a soft lock (a change to `Cargo.toml` has unlocked the source), lockfile versions become preferences. A
preferred yanked version is resolvable, but a preferred withheld version is not, and the resolver backtracks to find 
an alternative if available or else fails with a status-aware error.

`cargo install --locked` uses a crate's bundled lockfile as a hard lock, which adds a new failure mode: Cargo
skips a version of a binary that is itself withheld, but it does not inspect the candidate versions' bundled
lockfiles. This means that the newest version of a binary can fail to install with `--locked` because one of its
dependencies is withheld, when an older version would have succeeded.

Failures from a withheld pin are self-healing: if the version is released, the same lockfile builds unchanged.

Related:
- Rationale: [Why `withheld` instead of (further) overloading `yanked`?](#why-withheld-instead-of-further-overloading-yanked)
- Drawbacks: [Existing lockfiles can break without a local change](#existing-lockfiles-can-break-without-a-local-change)
- Future possibilities: [Smarter `cargo install --locked` on withheld dependencies](#smarter-cargo-install---locked-on-withheld-dependencies)

#### Modeling the new withheld field

The wire format change is additive, since existing parsers (Cargo >= 1.51, `crates-index`, `crates_io_index`,
`cargo-audit`, etc) ignore unknown fields.

The Rust API changes are not all additive:
- `cargo_util_schemas::index::IndexPackage` adds `pub withheld: Option<WithheldStatus>` field. `IndexPackage` is not 
`#[non_exhaustive]`, so this will require a minor version bump (as `cargo_util_schemas` is pre-1.0). This matches
the addition of `pubtime`, which caused a `0.11` -> `0.12` bump.
- `WithheldStatus` is a new `#[non_exhaustive]` enum in `cargo-util-schemas`. Unrecognized values deserialize to
`Unknown(String)` that carries the original name, to avoid future breakage on new values. `Unknown` is treated as 
`Quarantined` by consumers.
- `cargo::sources::registry::IndexSummary` gains a `Withheld(Summary, WithheldStatus)` variant. This is a breaking
change because `IndexSummary` is not `#[non_exhaustive]`, but the `cargo` library crate is explicitly unstable and
`0.x` versioned. This change will fit into a regular pre-release version bump.

Related:
- Rationale: [Why no index protocol bump?](#why-no-index-protocol-bump)

#### Yank and unyank responses

The registry web API's yank and unyank responses MAY include a `warnings` object with the same format as the publish
response (`{"ok": true, "warnings": {"other": ["..."]}}`). Cargo renders each entry in `warnings.other` as a 
`warning:` line, matching its current publish behavior. Registries that return `{"ok": true}` are unaffected.

This is used to surface to the user that an unyank was registered for a withheld version but will not take effect
until the version is released (see [Managing withheld status transitions](#managing-withheld-status-transitions)).

#### Fetching withheld bytes with `--fetch-withheld`

Two flags support fetching withheld bytes:
- `--fetch-withheld name@version` (available for `cargo build`, `cargo install`, `cargo publish`, and related commands)
- `--withheld name@version` (available for cargo `fetch`)
(hereafter referred to only as `--fetch-withheld`, but behavior of `cargo fetch --withheld` matches)

They can be used repeatedly in the same invocation to specify multiple `name@version` coordinates.

These flags have three effects for a named, withheld version:
- it is admitted into resolution despite a `withheld` marker
- its bytes are downloaded from `dl-withheld`, rather than `dl`, if it is currently `withheld`
- Cargo prints a notice that the withheld version is being accessed via `--fetch-withheld` flag (`note:` for 
`unreleased`, `warning:` for `quarantined` or `withdrawn`)

When used, Cargo first checks the version's status in its local index file. Withheld versions are fetched from
`dl-withheld` and non-withheld versions from `dl`. Downloaded bytes are verified against the index line's `cksum`. If 
that fetch returns a 4xx, then the local index file might be stale. Cargo refreshes its local index and retries once 
if the line's withheld marker changed. Under `--offline` or `--frozen`, no refresh is attempted and the failure is final. 

If a withheld version is accessed this way, Cargo adds no special handling to its lockfile. The version and checksum
are recorded as normal, with no marker that the version was withheld (as the status may change independent of any
lockfile state). This means that a later build without the `--fetch-withheld` flag will be rejected as described in
[Resolution](#resolution). Every invocation that needs withheld bytes must pass the flag.

Bytes retrieved via fetch from `dl-withheld` are cached under a distinct namespace next to regular entries:
`registry/cache/<index>/withheld~foo-1.0.0.crate`, unpacking to `registry/src/<index>/withheld~foo-1.0.0`. This
prevents a one-time use of withheld bytes from leaking into ordinary builds that have a stale index cache. The
same directory placement matches the expectations of existing cache tooling, such as Cargo's global cache tracker.

The `--fetch-withheld` flag requires exact coordinates (`foo@1.0.0`) rather than simply `foo`. This guards against
resolving to newer versions published by external parties, when a publisher is accessing their own crate.

`--fetch-withheld` only controls resolution and byte fetching, not publish policy. Whether a publish proceeds upon
encountering a withheld dependency is governed by default Cargo behaviors and the `--continue-on-quarantined` and
`--fail-on-unavailable` flags (see [Publishing crates that might become withheld at publish-time or have withheld dependencies](#publishing-crates-that-might-become-withheld-at-publish-time-or-have-withheld-dependencies)).

Related:
- Rationale: [Why a separate `dl-withheld` path rather than serving withheld bytes from `dl`?](#why-a-separate-dl-withheld-path-rather-than-serving-withheld-bytes-from-dl)
- Rationale: [Why do errors never include a bypass command?](#why-do-errors-never-include-a-bypass-command)

#### Stale-cache risks

This RFC does not change Cargo's crate cache or index cache lifecycles. A withheld version remains usable through
normal resolution if and only if all of the following conditions hold:
1. The local index file was cached before the version was withheld and has not been refreshed
2. The `.crate` bytes are already in the local cache, so no download is attempted
3. The resolution is satisfiable via the locally cached index without needing a refresh, for instance because the lockfile
pins it or the cached index shows no newer compatible version

In this case, the cached index is refreshed on the next change to `Cargo.toml` or `Cargo.lock` that touches the
registry source, on `cargo update`, or on cache eviction. Until then, an already-withheld version keeps building.
Caching of `~/.cargo` in CI (for instance, with `Swatinem/rust-cache`) preserves this state across CI runs. A mirror
serving the same snapshot produces a similar effect.

Crate bytes admitted via `--fetch-withheld` are stored in the distinct `withheld~` namespace and are never used
by ordinary builds, so do not add to this exposure.

Related:
- Drawbacks: [Warm caches keep withheld versions buildable](#warm-caches-keep-withheld-versions-buildable)
- Future possibilities: [Probing cached crates for withholding (and yanked state) to avoid stale-cache risks](#probing-cached-crates-for-withholding-and-yanked-state-to-avoid-stale-cache-risks)

#### Publishing crates that might become withheld at publish-time or have withheld dependencies

`cargo publish` reports and acts on withheld results, in both a single
invocation (`cargo publish --workspace`) and across separate invocations (such as `release-plz`'s planner).

After each upload, `cargo publish` keeps polling the index until the version's line
is found. The poll now reports the line's `IndexSummary` variant (to capture `IndexSummary::Withheld(..)`
status) rather than only confirming existence. The status line reads `Found unreleased a v1.2.3` (or quarantined, withdrawn) in place of `Published`.

If a `withheld` result is encountered, the default behavior depends on the kind of withholding:
- A crate whose upload results in `unreleased`, or depends on an `unreleased` version, is treated as a success: Cargo 
reports it a `note:`, and the invocation continues with an eventual 0 exit.
- A crate whose upload results in `quarantined` or `withdrawn` fails the invocation: Cargo reports an `error:`,
publishes nothing further, and exits non-zero. A crate that depends on a `quarantined`
or `withdrawn` version is skipped before upload. Already-uploaded crates are left in place. The non-zero exit indicates
failure but not a rollback.
- Poll timeouts keep their existing behavior. A version with no index line is treated as not yet available. This RFC
does not define a hold that writes no line (see Future possibilities: [Delayed indexing for unreleased](#delayed-indexing-for-unreleased)).

In the above cases, "depends on a <status> version" means that either a sibling publish in a workspace invocation
returned that status, or a dependency was admitted via `--fetch-withheld`.

Two flags change the above defaults and are described below under [Publish flags](#publish-flags).

See also:
- Drawbacks: [A quarantine can break a release train](#a-quarantine-can-break-a-release-train)
- Rationale: [Why is the publish default different per type of withholding?](#why-is-the-publish-default-different-per-type-of-withholding)

##### Publish flags

- `--fail-on-unavailable`: the invocation fails in any case that leaves a version not installable (`unreleased`,
`quarantined`, `withdrawn`, or a poll timeout). This applies to the result for a crate being published, or any 
dependency satisfied locally during packaging. A dependent of an unavailable version is not uploaded.

Example usage (`cargo publish --fail-on-unavailable`):
```
    Uploaded a v1.2.3 to registry `crates-io`
note: waiting for a v1.2.3 to be available at registry `crates-io`
help: you may press ctrl-c to skip waiting; the crate should be available shortly
       Found unreleased a v1.2.3 at registry `crates-io`
error: a v1.2.3 is not available at registry `crates-io`
  |
  = note: the upload succeeded; a v1.2.3 remains on the registry in unreleased state and is not yet installable
  = note: `--fail-on-unavailable` fails the publish unless the version is installable
  = help: for more information see https://crates.io/crates/a/1.2.3
```

- `--continue-on-quarantined`: the invocation continues past a `quarantined` or `withdrawn` result of the crate
being published, or one of its dependencies. A `warning:` is reported for each case. The invocation eventually exits 
`0`.

Example usage (`--continue-on-quarantined`, `a` quarantined, `b` depends on `a`):
```
    Uploaded a v1.2.3 to registry `crates-io`
note: waiting for a v1.2.3 to be available at registry `crates-io`
       Found quarantined a v1.2.3 at registry `crates-io`
warning: a v1.2.3 was quarantined by registry `crates-io` after upload
  |
  = note: the upload succeeded; a v1.2.3 remains on the registry in quarantined state and is not installable
  = note: continuing with remaining crates because of `--continue-on-quarantined`
  = help: for more information see https://crates.io/crates/a/1.2.3
   Packaging b v1.2.3 (/home/user/ws/b)
    Packaged 5 files, 9.8KiB (3.2KiB compressed)
   Verifying b v1.2.3 (/home/user/ws/b)
   Compiling a v1.2.3
   Compiling b v1.2.3 (/home/user/ws/target/package/b-1.2.3)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.41s
   Uploading b v1.2.3 (/home/user/ws/b)
    Uploaded b v1.2.3 to registry `crates-io`
note: waiting for b v1.2.3 to be available at registry `crates-io`
   Published b v1.2.3 at registry `crates-io`
warning: b v1.2.3 is published and installable, but depends on quarantined a v1.2.3
  |
  = note: consumers cannot resolve `a = "^1.2"` to a v1.2.3 while it is quarantined; builds of b v1.2.3 will fail unless another published version of a satisfies the requirement
  = help: for more information see https://crates.io/crates/a/1.2.3
```

- `--fetch-withheld name@version`: allows resolution and usage of withheld bytes for the build, as described in
[Fetching withheld bytes with `--fetch-withheld`](#fetching-withheld-bytes-with---fetch-withheld), but does not change policies around skipping uploads or halting. This is primarily needed
in the case of separate `cargo publish` invocations (see [Separate `cargo publish` invocations](#separate-cargo-publish-invocations)).

##### Workspace publish

In a workspace publish, the lockfile generation and verify builds run in an ephemeral workspace. Published crates'
dependencies resolve from the registry, but with freshly packaged siblings layered on top and preferred over registry
dependencies. This means that, for siblings, resolution doesn't consult the registry, so the sibling is resolvable 
without `--fetch-withheld`.

This RFC adjusts existing workspace publish behavior by passing the withheld status of any published sibling 
for user-facing notes (or aborting on `quarantined`/`withheld`). The following demonstrates for an `unreleased` 
sibling, but a `quarantined` sibling along with `--continue-on-quarantined` behaves similarly:

```
   Packaging a v1.2.3 (/home/user/ws/a)
    Packaged 6 files, 12.3KiB (4.1KiB compressed)
   Verifying a v1.2.3 (/home/user/ws/a)
   Compiling a v1.2.3 (/home/user/ws/target/package/a-1.2.3)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.02s
   Uploading a v1.2.3 (/home/user/ws/a)
    Uploaded a v1.2.3 to registry `crates-io`
note: waiting for a v1.2.3 to be available at registry `crates-io`
help: you may press ctrl-c to skip waiting; the crate should be available shortly
       Found unreleased a v1.2.3 at registry `crates-io`
note: a v1.2.3 was accepted by registry `crates-io` and is awaiting release; it is not yet installable
  |
  = help: for more information see https://crates.io/crates/a/1.2.3
   Packaging b v1.2.3 (/home/user/ws/b)
    Packaged 5 files, 9.8KiB (3.2KiB compressed)
   Verifying b v1.2.3 (/home/user/ws/b)
   Compiling a v1.2.3
   Compiling b v1.2.3 (/home/user/ws/target/package/b-1.2.3)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.41s
   Uploading b v1.2.3 (/home/user/ws/b)
    Uploaded b v1.2.3 to registry `crates-io`
note: waiting for b v1.2.3 to be available at registry `crates-io`
   Published b v1.2.3 at registry `crates-io`
note: b v1.2.3 depends on unreleased a v1.2.3
  |
  = note: consumers will resolve `a = "^1.2"` to another published version until a v1.2.3 is released
  = help: for more information see https://crates.io/crates/a/1.2.3
```

A workspace publish is not directly resumable. `cargo publish --workspace` will fail if it encounters an already published workspace sibling (regardless of withholding status). Instead, partial workspace publishes can be rerun
by specific `cargo publish -p a -p b` to specify which packages to include in the overlay. Packages not specified
are left out of the overlay and resolved from the registry (where they may be `withheld`).

This means that, if a workspace publish needs to be resumed when it has already published a crate that is withheld,
the rerun needs `--fetch-withheld a@1.2.3`. This is discussed further in [Separate `cargo publish` invocations](#separate-cargo-publish-invocations).

Related:
- Future possibilities: [Resuming a workspace publish](#resuming-a-workspace-publish)

##### Separate `cargo publish` invocations

Separate invocations have no shared overlay that resolves sibling crates locally. For instance, `release-plz` computes 
its own publish ordering and invokes `cargo publish` once per crate. This means that both the packaging step (which 
generates the tarball's `Cargo.lock`) and verify build run a fresh resolve against the real upstream index. So, 
witha held sibling fails resolution with a status-aware error.

If the publisher knows that the withheld version is their upload, `cargo publish --fetch-withheld a@1.2.3`
admits it into resolution and fetches its bytes from `dl-withheld`. The publish default then applies to the admitted
dependency: `unreleased` is a `note:` and the upload continues, `quarantined` or `withdrawn` is an error and the upload
is skipped unless `--continue-on-quarantined` is also passed.

Dependency found unreleased, admitted via `--fetch-withheld a@1.2.3` (continues, exit 0)
```
    Updating crates.io index
   Packaging b v1.2.3 (/home/user/ws/b)
note: admitting unreleased dependency a v1.2.3 via `--fetch-withheld`
    Packaged 5 files, 9.8KiB (3.2KiB compressed)
   Verifying b v1.2.3 (/home/user/ws/b)
  Downloaded a v1.2.3 (unreleased)
   Compiling a v1.2.3
   Compiling b v1.2.3 (/home/user/ws/target/package/b-1.2.3)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.41s
   Uploading b v1.2.3 (/home/user/ws/b)
    Uploaded b v1.2.3 to registry `crates-io`
note: waiting for b v1.2.3 to be available at registry `crates-io`
help: you may press ctrl-c to skip waiting; the crate should be available shortly
   Published b v1.2.3 at registry `crates-io`
note: b v1.2.3 depends on unreleased a v1.2.3
  |
  = note: consumers will resolve `a = "^1.2"` to another published version until a v1.2.3 is released
  = help: for more information see https://crates.io/crates/a/1.2.3
```

This requires the registry to advertise `dl-withheld`. Without it, the only local path available is a workspace
publish including the sibling. (See: [Fetching withheld bytes with `--fetch-withheld`](#fetching-withheld-bytes-with---fetch-withheld))

###### release-plz 

Note: `release-plz` is not a Rust Project tool and its changes are not approved by its RFC. This discussion
shows that the publish behavior above is sufficient for `release-plz`-style workflows. Specific changes will be worked
out with `release-plz` maintainers in its own respository. The work is nonetheless tracked as part of this
RFC's implementation, since `release-plz` support is a prerequisite for crates.io adopting withholding in a
subsequent RFC.

`release-plz` computes a publish plan and runs `cargo publish` once per crate. With the defaults described above,
it needs two changes:

First, it must support passing `--fail-on-unavailable` and `--continue-on-quarantined` through to publish invocations
when the user configures them.

Second, it must pass `--fetch-withheld` for siblings it published. Each subsequent `cargo publish` in an invocation
must be able to resolve siblings published in an earlier step, which may be `unreleased` (or `quarantined` in the
case of `--continue-on-quarantined`). `release-plz` already determines, during its planning phase, which local
versions exist in the registry (using git tags and registry checks). For a version it expects to publish in a given
run, it records the coordinate and passes `--fetch-withheld name@version` to subsequent `cargo publish` invocations
that depend on it. This addresses a single, uninterrupted run.

For a version that already exists, such as due to a re-run after an interruption, `release-plz`'s existing registry
check sees the `withheld` field. If the version is not `withheld`, no special handling is needed. If it is is 
`withheld`, `release-plz` needs evidence that this version is its own upload before passing `--fetch-withheld`. It uses
the release tag that `release-plz` creates for every publish (`chore: Release package foo version 1.0.0`). A
forged tag is only possible if somebody can push arbitrary tags to the repository, in which case, building
withheld bytes in a verify step is not the largest problem.

Repositories that disable release tags do not have this evidence. Alternatives are available, such as repackaging
and comparing checksums, but they are fragile due to variance in `Cargo.lock` and `.cargo_vcs_info.json` as the
dependency graph and commits change. This will not be pursued unless there is sign of need. In the interim,
`release-plz` can document creating such a tag manually to recover from failed releases.

### docs.rs impact

docs.rs does not build withheld versions. The related changes are:
1. `crates-index-diff`, `docs_rs_crates_io`, and the pending crates.io event feed (([crates.io#14188](https://github.com/rust-lang/crates.io/pull/14188)))
expose the `withheld` field along with `yanked`.
2. On seeing a version with `withheld` set, docs.rs skips the version's build and records its withheld status.
3. The version's page shows a status badge naming its state and linking to the registry's `notice-page` if one is
advertised. Documentation already built before the version was withheld stays online below the badge for `unreleased`
and `quarantined`. For `withdrawn`, it is removed, leaving only the badge.
4. On seeing the `withheld` field remove, docs.rs queues a build if the version does not already have built
documentation and removes the badge.

docs.rs's pre-existing behavior handles cases with dependents of withheld versions, providing an alternative
non-withheld version satisfies its dependency constraints. For crates without a bundled `Cargo.lock`, docs.rs
resolves fresh at build time, skipping withheld versions in favor of another compatible version. For a crate with
a bundled `Cargo.lock` that pins a withheld version, the first build attempt fails. docs.rs then deletes the lockfile,
regenerates it, and retries the build once.

This means that a dependent only fails if no compatible non-withheld version exists. That failure is recorded
as an ordinary build failure. docs.rs does not re-queue a failed dependent when the withheld version is later released,
or a newer compatible version is published. This is not specific to withholding is left for a subsequent RFC. In the
interim, publishers can manually trigger doc rebuilds via docs.rs's console.

Related:
- Future possibilities: [Re-processing failed docs.rs builds when a dependency becomes available](#re-processing-failed-docsrs-builds-when-a-dependency-becomes-available)

## Drawbacks
[drawbacks]: #drawbacks

#### A second mutable index field
- This stinks, but the ship has arguably sailed with yanks, and we lack a good non-line-oriented document to carry
index metadata without adding in entirely new Cargo fetches
- The justifications for this downside are discussed in Rationale: [Why write withheld releases to the index?](#why-write-withheld-releases-to-the-index)

#### An index line no longer guarantees fetchable bytes
- This is awkward, but we partially mitigate it through useful 404 bodies on failed fetches
- This is somewhat by design since we want to make clear to consumers *why* locked versions are not reachable (see:
Rationale: [Why write withheld releases to the index?](#why-write-withheld-releases-to-the-index))

#### Withheld statuses are publicly visible
- See Rationale: [Why write withheld releases to the index?](#why-write-withheld-releases-to-the-index)

#### Existing lockfiles can break without a local change
- This is especially painful for `cargo install --locked`
- We can consider improving that situation before adding publish-time holds to crates.io, see Future possibilities: [Smarter `cargo install --locked` on withheld dependencies](#smarter-cargo-install---locked-on-withheld-dependencies)

#### Warm caches keep withheld versions buildable
- This is true of any design that doesn't add in cache evictions, which we discuss in Future possibilities: [Probing cached crates for withholding (and yanked state) to avoid stale-cache risks](#probing-cached-crates-for-withholding-and-yanked-state-to-avoid-stale-cache-risks)

#### A quarantine can break a release train
- Intended, see Rationale: [Why is the publish default different per type of withholding?](#why-is-the-publish-default-different-per-type-of-withholding)

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

### Requirements

Essential:
1. Fresh fetches of a withheld version through default registry paths fail with diagnostics that explain the status, 
both for Cargo, and for other tools, including ones that do not consult the index to decide what to fetch
2. Existing Cargo releases have a safe default behavior with no code changes
3. Tools can differentiate withheld releases on the wire from other statuses like yanked, deleted, and never-published
4. A released hold leaves no mark that affects how tools treat the version
5. Withholding can be reversed without requiring a fresh publication. There is no loss of information due to 
withholding.
6. Nothing beyond the upload step requires publisher authentication, to avoid breaking Trusted Publishing workflows
7. A publisher can test and publish against their own withheld crate, in the same invocation or later one
8. Existing "release train" publication workflows work with no changes beyond tool version bumps except in the case
of adverse types of withholding (quarantine and withdrawal)
9. Index followers such as docs.rs can cleanly handle withheld releases without breaking on the withheld version, or
failing to handle the version when it is released
10. Mirrors and index signing keep working unchanged
11. Security researchers and scanners can discover withheld versions and obtain their bytes for review
12. Determining a version's status requires no request beyond the index
13. A released hold leaves no mark that affects how tools treat the version

Preferred:
1. Reuse the existing index, version model, synchronization, and release flow (especially, no index protocol bump)

### Major architectural alternatives

#### Why write withheld releases to the index?

First off: omitting releases from the index only seems viable for `unreleased` versions,
not `quarantined` or `withdrawn`. This would be a very confusing experience for users with `withheld`
versions in their lockfiles, since they would get resolution failures that give no indication of why.
This is a user experience that two other registries (npm and PyPI) have tripped over. PyPI quarantined through
omission and then later added project-level flags to indicate state (and is still serves plain 404s on release-level).
npm similarly omits based on publish-time scanner holds and has received [a fair bit of negative feedback](https://github.com/orgs/community/discussions/203413)
about its user experience.

Moreover, we are not particularly well positioned to add markers like PyPI has because our index files are
line-oriented. Each version is a line. PyPI, in contrast, offers a JSON document that makes it easier to reflect
status in a separate section that build tooling can still understand.

So we need need a globally accessible index line for `quarantined` and `withdrawn`. The real question
is whether we should write `unreleased` to the index.

Even for `unreleased`, two userbases need its index data and bytes:
- publishers (who resolve their own withheld crates across separate publish invocations)
- security researchers and scanners (who want to know about which crates are withheld, and access their bytes)

Serving these users without a withheld index line means additional registry-side systems: index-like metadata via
sidechannels, author- or researcher-specific authenticated views of the index, publish or session tokens, query APIs
or feeds to traverse withheld crates, and more.

The authenticated views break publishers, since Trusted Publishing doesn't preserve authentication context after crate
uploads (not to mention added local Cargo cache complexity). A per-identity index also can't be signed for mirros. And 
then, do we vend special access to researchers, and take on the responsibility of vetting them and maintaining yet
more systems for them traverse unreleased crates?

Session tokens work better for publishers but are fairly complex on the server side (durable group state, lost
token recovery, server-side access scoping, ...). The user experience also isn't great if you ever need to retry 
partially completed workflows or publish multiple entries out of band. They also still break researchers, unless we 
build dual systems like separate read capabilities with a feed. And then, once 
those systems are globally readable, they disclose the same information that an index line would. 

The alternatives to an index line are fairly complex, and have clear costs, so it's worth thinking about why
we would want to omit lines in the first place.

One reason might be to reduce the risk of oracle attacks. This is a fair concern, but keep in mind that this
only makes it easier for malicious parties other than the one publishing a release to analyze the triggers for 
registry-initiated withholding. Hardening on this front seems like a poor trade for diverging from the Rust Project's 
values around transparency and community access. And if Trusted Publishing rules out authenticated access, the data is
globally readable anyway, so it is questionable if this offers useful hardening. Regardless, the main point at which 
we need to start worrying about this style of oracle attack will be when we add pre-publish scanning, so we can 
discuss adding auth alongside that in the later RFC if desired.

Another reason for wanting this might be author privacy. If we do later have author-initiated staging, we can discuss 
shifting the index visibility of author-staged releases and/or restricting bytes in a separate RFC.

A third reason is avoiding index thrash. The remedy here is the same as for every other source of index thrash: 
continue to invest in deprecating the git index, while providing an event channel to support mirrors and other users 
needing traversal.

And, none of the mentioned options actually addresses the user experience gap of quarantined/withdrawn crates 
(unless we want to unconditionally check some other index on failure, in which case... why not just use the current 
one?). 

#### Why a separate `dl-withheld` path rather than serving withheld bytes from `dl`?
- `dl`-only would only protect fresh cargo resolution, tools that don't understand `withdrawn`
would resolve anyway. Also it's an extra layer of protection against stale cache risks. With `dl`-only,
all you need is a stale index file. But with `dl-withheld`, you need both stale index file and stale crate cache.

#### Why `withheld` instead of (further) overloading `yanked`?
First off, this would have the same issues as only `dl` re: build tools that don't understand yanked
(Yocto), tools that fetch every line in the index, and stale-cache risks. Moreover, if we had overloaded `yanked`
and then used `dl-withheld`, we would have an awkward case of needing to check both places in the case of using
`--fetch-withheld`.

Beyond that, the semantics are poor:

Yanked is an overloaded term already in that it is used by crate authors for any reason, most frequently maybe-broken 
releases. It also is used by registry administrators as an imperfect "soft" mitigation during investigation, very
similar to our intended use of `quarantine`. The lack of semantic meaning for yanked means that it simply does not
carry sufficient information for build tools to reasonably evaluate whether yanks correspond to withheld crates that
have redacted bytes, or just the "probably don't resolve this because it might break you" case.

Overloading the field further such that it may or may not point to available bytes, with no extra data indicating which 
is the case, seems like a strictly worse user experience than offering
a clear withheld field indicating state that has direct support in newer build tools. Ideally, we would prefer to have 
most security responses that currently set `"yanked": true` prefer to instead set `"withheld": "quarantined"`, and 
leave standalone `yanked` usage for authors only. The wire representation of a withheld entry regardless should 
include `"yanked": true` for backwards compatibility reasons, but the registry-side management will track it separately
from an author-initiated yank.

Note that we ARE shifting the semantics of `yanked` to mean that it may or may not point to available bytes. This
does have downstream implications. The additional `withheld` field is better in that it offers a path to cleaner 
handling by downstream tools that don't use index resolution before fetching bytes, such as mirrors. Refer to the
"Impact on byte mirrors and other index consumers" for more details.

#### Why not just `min-publish-age` with a default?
On a basic level, `min-publish-age` is insufficient because it does not provide a way to redact bytes
of already released versions. This means it is not appropriate for incident response, and we are back to
either yanking (also leak bytes) or deleting (we are slower to do it, and it doesn't provide the security
researcher access that we would like).

Beyond that, even for the basic "publish time supply chain attack handling" case, it is not ideal.
For one, it requires that build tools request `min-publish-age`, which seems unlikely to be universally
true. In general, client-side protections are not a great security boundary in comparison to registry-level
enforcement. We could discuss a registry-side `min-publish-age` for crates.io, but that seems fairly painful
and contentious, and anyway will need many of the same escape hatches as `withheld` to not break publishers
or other consumers.

Beyond this, as discussed in [Motivation](#motivation), `min-publish-age` is concerning because it fails open and thus forces
an urgent response to avoid security impact of malicious releases. Defaults that registry administration
teams might be comfortable with might be fairly long from a user perspective. This is about more than just
user experience: Extended delays also have security downsides insofar as they also delay security fixes and other 
important changes.

Also, however long the default is, the "I need to press a button with some urgency to avoid a problem" is
psychologically challenging compared to "My systems will catch things, and I just need to go check them for
false positives in a reasonable time frame". As security-minded engineers, it seems unlikely that non-automaticaly-handled
supply chain attacks will ever be treated as not-that-urgent.

It's worth mentioning a nice suggestion from @joshtriplett of letting the registry advertise a dynamic
`min-publish-age` to buy itself time if falling behind on vulnerability reports or thinks it is at heightened
risk. This is discussed in Future possibilities: [Registry-advertised dynamic `min-publish-age`](#registry-advertised-dynamic-min-publish-age). I like this
idea, but also do not think it suffices. I'm not sold that it will address the psychological pressures of reactive
responses. And then the same concerns around any byte availability, and what to do for already-released versions,
still apply.

### Other decisions

#### Why no reason text in the index?

In this RFC, we track the reason for withholding at the registry level. It can be made available on the user-facing
page that is advertised via `notice-page`. This mirrors crates.io's handling of yanked reasons. Currently they
are implemented in the backend, and if this RFC were implemented, they could also be displayed using a similar 
mechanism.

We could imagine better alternatives, but we'd prefer to consider them alongside via `yanked` in their own RFC.
See also: "Future possibilities: Better display of reasons for withholding".

#### Why no index protocol bump?

The `withheld` field is valid under all existing `"v": 1 | 2 | 3` (where 3 is experimental).

The criteria for bumping the protocol version is that older versions of Cargo would misinterpret the line or crash.
Bumping the version is particularly awkward currently because we have an experimental `3` value that we do not want to 
release as stable, but we do want to make sure that new index features associated with withholding are immediately 
available. Ultimately protocol version is a format version with a total order, and repurposing it for 

We have wiggle room here since, as of 1.51, Cargo will ignore unknown index fields. Further, we are not changing the 
behavior of existing fields. This gives us a path to treat `withheld` as compatible with all protocol versions.

The failure mode that we are concerned about is that resolution should skip resolving entries with a withheld
status, but ignoring that withheld marker might still land on those entries. Bytes will remain unavailable server-side 
(unless we have a stale cargo cache, which is already treated as out of scope for this feature, discussed under [Stale-cache risks](#stale-cache-risks)). But, we do still have a backwards compatibility problem where we might fail to find a resolver solution that would exist using a non-withheld version, if Cargo knew to backtrack.

As a mitigating measure, we can nudge the resolver to avoid this by requiring that registries MUST set `yanked: true` 
when transitioning to a withheld status. This is discussed further in [Managing withheld status transitions](#managing-withheld-status-transitions). This
does still result in builds with `Cargo.lock` pointing to withheld crates, or build tools that don't respect yanks,
still resolving to the withheld releases, which will fail. But, even a withheld-aware build tool would fail the same
builds, albeit with better error messages. Our registry spec says that registries SHOULD offer helpful not-found
response bodies on `dl`-endpoint fetches of withheld bytes, which keeps user experience decent.

So, we don't risk miscompilations or especially confusing errors. With the `yanked: true` nudge, we are providing a 
fairly good user experience in most cases, and otherwise clear errors. This should be sufficient to avoid needing a 
protocol version bump.


#### Why not `"withheld": "quarantined_and_yanked"`?
It's a bit awkward that we have some extra bookkeeping to display `"yanked: true` whenever we have a
withheld status. But, we do want this set for backwards compatibility reasons. If we don't care about backwards
compatibility, then we just leave `yanked` only for author and admin-initiated yanks and have it be totally
orthogonal to `withheld`.

But, if we do want to set the `yanked` bit, giving `withheld` and ugly-looking state explosion (since we need all
future `*_and_yanked`) doesn't buy us very much on either the readability front or the bookkeeping front. It's simple
enough for registries just to track `yanked` independent of `withdrawn` and only flush the `"yanked": true` for
`withdrawn` when writing to the wire. Better to keep the wire representation simpler.

#### Why not publish-time propagation of withheld states during a release train?
We can imagine alternative methods that attempt to propagate quarantined status to 
dependents based on poll status, with a registry publish API extension. This is useful both for avoiding broken binary versions,
and also generally offering clearer visualization of what a leaf crate is unreachable due to exclusively quarantined 
dependencies. But, this is a bit of a trap, in that we would not be probing more deeply for quarantined transitive 
dependencies and also in that dependencies can become quarantined underneath us. 

The proper place for propagation or transitive display of quarantined state would instead be at the registry level, if at all. In other words, that is a discussion for
a subsequent RFC.

#### Why not server-side propagation of withheld states?

This would be great, but today it is complex. Crates.io only checks one level of dependencies for reverse
dependencies. We don't want to start running full resolution server-side. Showing one level of withheld
dependencies is *an* option, but it seems more misleading to inconsistently reflect this state. And, even then,
it is somewhat complex, since we only show version constraints, not the actual resolved version, so we would
need to do some amount of resolution to fix that.

A better option is to make use of docs.rs's actual use of the resolver and signal back to crates.io. This could
be added later and is discussed under Future possibilities: [Re-processing failed docs.rs builds when a dependency becomes available](#re-processing-failed-docsrs-builds-when-a-dependency-becomes-available).

#### Why is `dl-withheld` as open as `dl`?
TLDR: I don't think this is worth including now, even if it might be worth it later. If we want to walk away from 
public-by-default, it is easy enough to add per-template `auth-required` in `config.json`.

One of the points of the having a `unreleased` review window, or a `quarantined` freeze is to gather information.
It seems cleaner to err on the side of general access to scanners rather than getting in the business of blessing
trusted parties. 

The counterarguments relate to author privacy during staging (handle this in a later RFC if we have staging) and
oracle attacks on the index (fair, but a tradeoff with having more eyes to catch problems - we can discuss adding
auth when we add publish-time detection/holds). Both are further discussed in Rationale: [Why write withheld releases to the index?](#why-write-withheld-releases-to-the-index).

#### Why is the publish default different per type of withholding?
We don't want unreleased versions to break release trains (see: [complaints from npm users](https://github.com/orgs/community/discussions/203413)).
So, defaulting to continuing with an informational note seems correct for this case.

But, if a version is immediatley quarantined upon release, that means that the publisher's account is under quarantine
or otherwise in a concerning state. This deserves publisher's attention and SHOULD break CI, similar to how
`--allow-dirty` forces attention. We have an easy escape hatch if desired, via `--continue-on-quarantine`, that CI
workflows are welcome to integrate with.

#### Why do errors never include a bypass command?
The person that sees the error is the least suited to evaluate it - ie, a consumer that just wants to make the build
pass. This means they (human or robot) are inclined to paste in whatever they make the error go away.

`cargo publish --allow-dirty` offers a similar precedent of never being suggested directly by Cargo.

Most intended users don't need the hint anyway. Researchers read documentation. Release tools have built-in support for
the flags. Users setting up custom CI read documentation.

Other legitimate users might be inconveninced by needing to check docs to understand what to do, but this seems
preferable to risking naive users installing quarantined bytes.

## Prior art
[prior-art]: #prior-art

### Researcher access in other ecosystems

**analysis is LLM generated and will be rewritten in full before publication**

- [PyPI: Project Quarantine (2024-12)](https://blog.pypi.org/posts/2024-12-30-quarantine/) — admin-set, reversible; enforced by omission from the Simple index; yank considered and rejected ("a yanked Release is still installable"); ~140 quarantined, 1 released.
- [PyPI: project status markers (2025-08)](https://blog.pypi.org/posts/2025-08-14-project-status-markers/) / [PEP 792](https://peps.python.org/pep-0792/) — legibility marker retrofitted a year after omission-based quarantine; project-scoped; mixes informational (`archived`, `deprecated`) with enforcing (`quarantined`); works only because omission does the enforcing.
- [warehouse `Release.lifecycle_status`](https://github.com/pypi/warehouse/blob/main/warehouse/packaging/models.py) — release-level quarantine since 2026; omission in `_simple_detail`; no per-release marker in the standard API. Same shape as `withheld: quarantined`, illegible on the wire.
- [PEP 592: yanked releases](https://peps.python.org/pep-0592/) — yank = soft delete that stays installable when pinned; owner-only; reason carried in the index. The contract this RFC preserves for `yanked`.
- [PEP 694: upload API, staged releases](https://peps.python.org/pep-0694/) — staged = session state, absent from the index until published. Contrast: `unreleased` is a visible line.
- [Pre-PEP thread: status markers](https://discuss.python.org/t/pre-pep-discussion-project-status-markers-in-the-index-apis/79356) — design discussion; warning: its description of quarantine as "yanked" does not match warehouse's implementation.
- [npm: publish-time scanning feedback (#203413)](https://github.com/orgs/community/discussions/203413) — pending-scan versions returned plain 404; "pending vs never published" ambiguity broke publish scripts; Cloudflare `wrangler`/`miniflare` release train broken by a dependency still in scan; npm "prioritizing work to display scanning status". Motivates the kind-dependent publish default, useful 404 messages, and the `notice-page` display.
- [npm: safer publishing roadmap (#208130)](https://github.com/orgs/community/discussions/208130) — staged publishing and status surfacing on the roadmap.
- [RubyGems: removing a published gem](https://guides.rubygems.org/removing-a-published-gem/) — `gem yank` removes the index entry *and* the gem file; the version cannot be re-pushed ([rubygems#2183](https://github.com/rubygems/rubygems/issues/2183)). The opposite pole: yank *is* withdrawal, no reversible state.
- [Go: `retract` directive](https://go.dev/ref/mod#go-mod-file-retract) — author-only, informational; retracted versions remain downloadable; published modules cannot be deleted. Author-informational vs registry-enforced kept separate.
- [RFC 3660: crates.io crate deletions](https://rust-lang.github.io/rfcs/3660-crates-io-crate-deletions.html) — index-line removal precedent; `deleted_crates.available_at` name reservation.
- [RFC 3923: `min-publish-age`](https://github.com/rust-lang/rfcs/blob/master/text/3923-cargo-min-publish-age.md) — client-side cooldown; the "breathing room" this RFC's response capability complements.
- [`crates-index-diff::Change::VersionDeleted`](https://docs.rs/crates-index-diff/latest/crates_index_diff/enum.Change.html#variant.VersionDeleted) — docs.rs ingestion already models line removal.
- [Project goal: yank with a reason](https://rust-lang.github.io/goals/2024h2/yank-crates-with-a-reason.html) — reason surfaced via API and frontend first, index later. Same sequencing here: no reason text on the wire.
- `cargo publish --allow-dirty` — Cargo precedent for failing a successful-looking operation to force acknowledgement; basis for `quarantined` exiting non-zero.


## Unresolved questions
[unresolved-questions]: #unresolved-questions

### To resolve before merge
- That we are happy with the proposed default publish behaviors for unreleased vs quarantined, and the naming of the 
flags. For this, we also will want input from `release-plz`'s maintainer. 
- If we actually want `--fetch-withheld` on `cargo build` and `cargo install` (for forensic build usage) or only
`cargo fetch` and `cargo publish`
- Whether we should make `cargo_util_schemas::index::IndexPackage` `#[non_exhaustive]` while we are bumping semver anyway
- Whether we should scope in implementation of an indemnipotent workspace publish (see Future possibilities: 
Resuming a workspace publish)

### To resolve during implementation
- Exact errors and prose notes, documentation notes, documentation URLs
- Interaction of `withheld~` cache namespace with `cargo clean gc` and the global cache tracker

### Related problems this RFC leaves open
- Dependents of a withheld version are published and installable but broken until it is released. Propagation, 
linking, or docs.rs re-processing are registry- and docs.rs-side problems for a later RFC. We don't expect meaningful
from this until we have actual systems that would withhold freshly-uploaded crates, which will be part of the same RFC. 
- How to support delayed index publishing lines for 'unreleased' versions
  - What are the mechanisms for avoiding breaking multi-invocation publics that need access to the registry line for unreleased, withheld crates
  - What discovery channel can we offer security researchers to offer supporting triage and analysis
- Warm-cache exposure (stale index plus cached bytes) is pre-existing and unaddressed here.
- Including withdrawn reason in index line: We follow early work on `yanked` here which only has reasons support
on crates.io backend, not frontend, and not index wire representation. We're adding crates.io backend + frontend 
support but deferring index representation to be discussed together with yanked.
    - The `notice-page` support we are adding might work well for displaying yanked reasons via the crates.io frontend
    once available

## Future possibilities
[future-possibilities]: #future-possibilities

#### Re-processing failed docs.rs builds when a dependency becomes available
- This issue affects any crate that is published with no successful resolver solution available, withheld
dependencies or otherwise
- In this case, the version's docs will fail to build, fail its retry, and then never try again unless manually triggered by the user
- Docs.rs could store a mapping of which builds broke due to resolving only to withheld dependencies, and re-trigger
those builds if that dependency is released from withholding, is unyanked, or publishes a newer compatible version
- We can explore this in a RFC that adds crates.io publish-time holds as that will introduce new impact for this
problem

#### Delayed indexing for unreleased
- A good middle ground to avoid index thrash might be only publishing to the sparse index but not the git index
- To avoid publishing index lines for unreleased crates, we need an alternative way to serve the full index line,
or else we break separate publish invocations
- Any such change will need to consider security researchers, such as exposing an event feed for withheld crates
- Alternatives are discussed at greater length in Rationale: [Why write withheld releases to the index?](#why-write-withheld-releases-to-the-index)

#### Author-managed staging
- https://internals.rust-lang.org/t/pre-rfc-package-staging/20459
- This is much larger scope that we want to pick up, but the `unreleased` status is designed to be supportive of it
- We would likely need a registry web API-side change to pass in the desire to have things staged (unless scoped
to be account-wide)
- The mechanisms for releasing from registry-side staging would be a larger design as well
- If there is a desire from authors to redact withheld bytes, or avoid writing to the index, this would take
further design that would be supplementary rather than conflicting to this one
- Future RFC territory

#### Restricting `dl-withheld` to credentialed researchers
- Discussed in Rationale: [Why is `dl-withheld` as open as `dl`?](#why-is-dl-withheld-as-open-as-dl)
- If we start to see stronger reasons to guard `dl-withheld`
- This RFC makes dl-withheld as open as dl on the same registry: public on crates.io, token-gated on an auth-required registry. A registry wanting to limit withheld bytes to vetted researchers — the model PyPI's Observer program gestured at — would need per-template authentication in config.json (for example, an auth-required map keyed by template). That is deferred; index visibility of withheld versions is public regardless, so the gate would protect bytes, not existence.

#### Smarter `cargo install --locked` on withheld dependencies
- This experience is not good today because the resolver won't try to avoid binaries that bundle
lockfiles with withheld dependencies
- Probably a better answer is something along the lines of, on encountering this failure, fetch
older versions until one does not have a broken lockfile
- At that point we could print a warning suggesting to install the candidate version (or fall back to it by default 
with a warning)
- This is a deeper set of changes to the resolver that belong in their own RFC. This experience already exists for
deleted crates, and the `quarantine`/`withdrawn` cases are largely a replacement to deletion, so it's not clear
to me how catastrophic this is

#### `cargo info` support
- Today cargo info will never show "yanked" since it only shows candidate versions (missing from index view,
not found for specific requested version)
-  We could enhance it to support both yanked and withheld with extra visibility
- This RFC exposes all the information necessary to support this via the `IndexSummary::Withheld` variant if prioritized
- I don't think it is particularly significant, given that we also don't show it for `yanked`


#### Probing cached crates for withholding (and yanked state) to avoid stale-cache risks
- HEAD request to check for byte existence as a quick proxy for needing a refresh
- There should be a way to run it in CI by default, probably? And other sensitive environments?
- This avoids issues with stale cache w/r/t withheld, but also to crate deletion
- This is a larger Cargo change that deserves its on RFC

#### Better display of reasons for withholding
- Beyond displaying it in the frontend and statically linking to the page, a good next step would be to add a registry 
endpoint that surfaces reasons in a machine-readable format for Cargo to use on different failures and warnings.
- We could alternatively write it directly to index lines, but we should do so in conversation with `yanked` reasons
as well.
- Both approaches probably merit their own RFC that unifies with `yanked` handling.

#### Registry-advertised dynamic `min-publish-age`
- This is supplementary to this proposal (see: Rationale: [Why not just `min-publish-age` with a default?](#why-not-just-min-publish-age-with-a-default)) but
a great idea by @joshtriplett! 
- It lets the registry temporarily heighten its security posture if it has reason to believe it is being
targeted in ways that raise risk (for instance: another language's registry was just compromised).
- We can consider it alongside publish-time checks in a future RFC.

#### Resuming a workspace publish
- `cargo publish --workspace` will fail if it encounters an already published workspace sibling (regardless
of withholding status), so a partial workspace publish cannot be rerun as-is
- A publisher can rerun the workspace publish with `-p` to specify remaining packages and exclude others from the
the local overlay. But, that resolves other packages from the registry, where they may be withheld.
- This is a worse experience in the case of withholding since you then need `cargo publish -p b --fetch-withheld a@1.0.0`
- The overlay currently shadows an upstream version of the same `name@version` ([#14789](https://github.com/rust-lang/cargo/issues/14789), fixed for `--dry-run` at risk of local bytes drifting from uploaded bytes)
- Skipping already-published members while keeping them in the overlay would work with withheld siblings with no 
further support
- This is requested in [#13397](https://github.com/rust-lang/cargo/issues/13397) as `--indemnipotent`. It is
indpendent of withholding but could be implemented alongside this RFC if the team desires; otherwise it remains
future work.