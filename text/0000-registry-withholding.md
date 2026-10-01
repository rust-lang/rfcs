- Feature Name: `registry_withholding`
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary
[summary]: #summary

This RFC adds a new, optional registry index field, `withheld` (`quarantined | unreleased | withdrawn`).

It also defines two new optional registry `config.json` URL templates alongside `dl`: `dl-withheld`, from which 
withheld bytes can be explicitly fetched by exact coordinates, and `notice-page` which is a human-oriented
webpage display additional displaying details and policies that could either be per-crate or static and global.

Lastly, it specifies how `Cargo` should interact with the new status field and `config.json` templates, for fetches, 
builds, installs, and publishes.

This is the first of three changes that are part of the [crates.io registry response project goal](https://github.com/rust-lang/goals/pull/795).
1. Quarantined/held/withdrawn support in Cargo, registry spec, docs.rs, release-plz (this RFC)
2. crates.io support for quarantine/withdrawn admin APIs, and related admin API work (issue not opened yet)
3. crates.io support for publish-time detections with automatic holds, a manual review queue, and supporting console interface (RFC not open yet)

## Motivation
[motivation]: #motivation

This is RFC is the first step towards a goal of hardening crates.io and other registries to be able to
rapidly respond to possible attacks using non-destructive actions and, ultimately, pre-emptively hold likely
malicious bytes for pre-release reviews. The big-picture plan [is in this Project Goal](https://github.com/jlizen/goals/blob/registry-security-response/src/2026/registry-security-response.md).

The gist of the goal is, we want to move towards proactive rather than reactive defenses against supply chain attacks. Right now,
every attack is a firedrill for the crates.io team, and we fail open. The coming min-publish-age defaults will give us 
some breathing room, but it still is a firedrill that fails open if response is not quick enough. And, we expect that
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

Out of scope for this RFC, but in a subsequent one:
- Decisionmaking for the criteria registries will use for quarantining, pre-publish holding, or withdrawing 
releases. Only the wire representations and cargo handling are defined here to keep all Cargo changes consolidated to a 
single RFC and also build in first-class support for these behaviors into Cargo as far ahead of registry adoption as 
possible.
- The actual mechanism that crates.io will use to manage lifecycle transitions to/from quarantined, unreleased, and withdrawn statuses.
- crates.io mechanisms for surfacing quarantined/unreleased/withdrawn status in its web frontend or web API
- Systems for proactively "holding" releases for review at publish-time or supporting author-driven staging

And then, out of scope for both this and the immediately following RFCs, is actively revoking cached crate and index files. The security boundary that we are focused on here is fresh acquisition, with a cold cache. We make a best effort
to avoid serving withheld bytes if we have a fresh view of the index, but the combination of a warm crate cache and
a stale, cached view of the index without the `withheld` marker, will allow continued installation of withheld bytes.

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

### Index field: "withheld": "quarantined | unreleased | withdrawn"

Cargo index data adds a new `withheld` field, with the possible values of `quarantined | unreleased | withdrawn`. 
A version with this field is not selected by Cargo or other build tools, and its bytes are not served via normal 
download endpoints. A version without this field is available as usual.

- unreleased — the version has been uploaded to the registry but is not yet available, whether because the author 
staged it or because the registry is reviewing it.
- quarantined — the version was available and has been frozen by the registry, typically during a security 
investigation. It may be released again or withdrawn.
- withdrawn — the version has been permanently removed; the coordinate stays reserved so it cannot be reused.

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
to a lockfile pin, they fail with a helpful not-found message from the registry (see "Serving withheld bytes").

One gap remains for all Cargo versions: if both the index file and the crate bytes were cached before a version was 
withheld, builds keep using the cached copy until something refreshes the index (see "Stale-cache risks").

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
and when. Some registries may hold fresh uploads without writing an index entry. For these, discovery will require
a separate channel (See "Future possibilities: Delayed indexing").

Registries that set `dl-withheld` also make the withheld bytes available for triage and forensic analysis. The bytes can be fetched directly, for example `GET https://static.crates.io/withheld/base64squatter/1.0.0/download` or
through Cargo flags for forensic builds: `cargo fetch --withheld base64squatter@1.0.0`, `cargo build --fetch-withheld base64squatter@1.0.0`, `cargo install --fetch-withheld base64squatter@1.0.0`. The flags require exactly `name@version`.

When the flags are used, they only admit withheld crates for that specific invocation. The crate bytes themselves
are cached under a separate `withheld~` namespace to avoid other builds picking them up by accident. Plain builds will
attempt to fetch bytes from `dl` and fail.

### Maintainer usage

Publishing continues to work when a registry withholds something mid-release, whether in a single
`cargo publish --workspace` call or across separate invocations (such as via `release-plz`).

Cargo by default treats an upload that results in an `unreleased` status or depends on `unreleased` versions as a 
success:  Cargo will report the state, continue on, and later crates that depend on it are still published. Registries 
might have different reasons for holding uploads as unreleased, such as cooldowns, automated reviews, author staging, 
so routine holds don't break releases.

Meanwhile, an upload that is marked as `quarantined` or `withdrawn`, or depends on a quarantined/withdrawn version,
is treated as a failure. Cargo skips the upload ahead of time when a dependency is already known to be quarantined or
withdrawn. In the case of an upload returning a quarantine or withdrawal, Cargo will stop publishing further crates,
emit an error, and exit with a failure code.

Timeouts maintain are unchanged: If Cargo times out while waiting on a crate to upload that has subsequent
uploads depending on it, it errors and exits as a failure. If it times out on a crate with no dependencies, it warns
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
builds it (if has not already successfully built the version's docs) and removes the badge.

A `withdrawn` version keeps its badge permanently and its documentation is removed.

Dependents whose builds fail because they resolve solely to withheld versions are handled as if they are ordinary
build failures. See "Triggering reverse dependency re-processing based on withholding changes" under "Future 
possibilities" for what a later RFC could change.

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
a "withheld" entry is equivalent to being published with no withholding.

Presence of `withheld` means that the version is not installable. Registries MUST NOT use `withheld` for informational
annotations (deprecation, curation status, etc). Every line carrying `withheld` MUST also carry `yanked: true` (see 
"Managing withheld status transitions").

Definitions:
- `withdrawn`: a terminal state that indicates that an entry is permanently not installable, with its line kept as a 
tombstone that reserves its `name@version` coordinate. Whether the withdrawal is due to misuse is not recorded in the 
index and left for external systems to disclose as described in the "Registry handling" section.
- `quarantined` the registry has frozen a version because it suspects misuse. This can happen to a version that
previously was installable, or on arrival (for instance if the publishing account itself is frozen). The version may
be released, at which point the index will remove the `withheld` field. Or, it may transition to "withdrawn" status
and be kept permanently uninstallable.
- `unreleased`: an entry that was submitted to the registry via a publish API, but has not yet been made
available to consumers, regardless of who initiated the hold. It might be due to some author-driven staging API, or it 
might be due to registry systems probing for security risks. The origin is not disclosed on the wire. The entry may 
transition to a released status or `withdrawn`.

Build tools MUST treat unknown statuses as equivalent to "quarantined", because all versions with a `withheld` field
are uninstallable and `quarantined` is the strictest handling that is generally applicable. Build tools SHOULD include 
a warning indicating that they saw an unknown value and were treating it as quarantined.

A withheld-status-aware build tool MUST NOT resolve withheld versions regardless of yanked status, including when the
version is present in a lockfile.

Related:
- Rationale: Why no index protocol bump?
- Rationale: Why `withheld` status instead of (further) overloading `yanked`?
- Rationale: Why are withheld statuses public?
- Rationale: Why is `withheld` blocking-only?
- Future possibilities: Delayed indexing

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
`dl-withheld` path for the duration of the withheld state. Once a version exits withholding, `dl-withheld` SHOULD NOT
continue to serve it (it MAY return 404 or redirect to `dl`).

Registries MAY additionally make `withdrawn` bytes available via the `dl-withheld` path for a duration of their choosing.
If they do, they SHOULD remove the bytes after a period of time rather than hosting them indefinitely.

Bytes served from `dl-withheld` MUST match the `cksum` in the version's corresponding index line. Cargo uses the same
checksum-verification as used with `dl` bytes.

A GET to the corresponding URL returns the `.crate` bytes for the withheld crates. For example, with the config.json above, `cargo fetch --withheld base64squatter@1.0.0` requests:

```
GET https://static.crates.io/withheld/base64squatter/1.0.0/download
→ 200 OK
  Content-Type: application/octet-stream
  <.crate bytes>
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
- Rationale: Why a separate `dl-withheld` path rather than serving withheld bytes from `dl`?
- Rationale: Why errors never include a bypass command
- Rationale: Why is dl-withheld as open as dl? 
- Prior art: Researcher access in other ecosystems
- Future possibilities: Author-managed staging
- Future possibilities: Delayed indexing

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

If `notice-page` is set, Cargo renders it as  "= help: for more information see <url>" string in withheld-related
resolution errors, `--fetch-withheld` warnings, and publish-time status outputs. If unset, no `help:`
link is shown. 

Related:
- Rationale: Why no reason text in the index?

#### Authentication

The `dl-withheld` URL template does not require auth, unless the entire registry requires 
auth. In other words, if `auth-required: true` in `config.json`, Cargo attaches its registry token to `dl-withheld`
calls in the same manner as it does for `dl` calls.

This RFC provides no way to require credentials for `dl-withheld` alone. Per-template authentication configuration is 
out of scope. Registries MAY apply throttles or other CDN-layer controls to these paths, as they can for `dl` and 
`api`.

`notice-page` is never fetched by Cargo but rather via the user's browser, so no token authentication is involved.

Related:
- Rationale: Why is dl-withheld as open as dl? 

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
- Rationale: Why `withheld` instead of (further) overloading `yanked`?
- Rationale: Why not `"withheld": "quarantined_and_yanked"`?
- Rationale: Why no index protocol bump?

#### Impact on byte mirrors and other index consumers

This RFC changes an invariant relied on by downstream tooling: every index line has fetchable bytes via `dl`. Under
this RFC, a withheld line is present in the index but its bytes return 404 via the `dl` path.

This affects tools that fetch bytes for every index line rather than only for versions that a resolver selected, for instance: full byte mirrors like Panamax, caching proxies that eagerly fetch, and docs.rs. Tools that fetch only
resolver-selected versions, such as `cargo vendor` or regular `cargo build` are unaffected beyond pinned lockfiles
failures discussed under "Lockfile handling".

Impacted tools SHOULD either skip index lines with `withheld` present, or fetch withheld bytes from `dl-withheld`
and serve them under their own `dl-withheld` (see "Impact on verifiable registry mirroring"). Until they do either,
they will receive 404s for withheld lines from `dl` and SHOULD treat these 404s as expected rather than a system
failure. Registries SHOULD serve an informational body on these 404s (see "Serving withheld bytes") to make the cause
immediately apparent to a tool that is not aware of the `withheld` field.

docs.rs is the only impacted consumer whose changes are specified by this RFC (see "docs.rs impact").

Related:
- Drawbacks: An index line no longer guarantees fetchable bytes

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

When an existing Cargo.lock contains withheld entries (due to the version being withheld after it was locked, or usage of `--fetch-withheld`), Cargo handling differs from `Yanked`. A yanked pin
resolves, but a withheld pin does not. 

Under a hard lock (no change to `Cargo.toml` has unlocked the source), Cargo honors all pins rather than re-resolving, 
then rejects a withheld pinned version with a status-aware error. The error suggests `cargo update <crate>`, which
re-resolves the named crate to attempt to find a non-withheld version, while keeping every other pin.

Under a soft lock (a change to `Cargo.toml` has unlocked the source), lockfile versions become preferences. A
preferred yanked version is resolvable, but a preferred withheld version is not, and the resolver backtracks to find 
an alternative if available or else fails with a status-aware error.

`cargo install --locked` uses a crate's bundled lockfile as a hard lock, which adds new failure modes: Cargo
skips a version of a binary that is itself withheld, but it does not inspect the candidate versions' bundled
lockfiles. This means that the newest version of a binary can fail to install with `--locked` because one of its
dependencies is withheld, when an older version would have succeeded.

Failures from a withheld pin are self-healing: if the version exits withholding, the same lockfile builds 
unchanged.

Related:
- Rationale: Why `withheld` instead of (further) overloading `yanked`?
- Drawbacks: Existing lockfiles can break without a local change
- Future possibilities: Smarter `cargo install --locked` on withheld dependencies

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
- Rationale: Why no index protocol bump?

#### Yank and unyank responses

The registry web API's yank and unyank responses MAY include a `warnings` object with the same format as the publish
response (`{"ok": true, "warnings": {"other": ["..."]}}`). Cargo renders each entry in `warnings.other` as a 
`warning:` line, matching its current publish behavior. Registries that return `{"ok": true}` are unaffected.

This is used to surface to the user that an unyank was registered for a withheld version but will not take effect
until the version exits withholding (see "Managing withheld staus transitions").

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
"Resolution". Every invocation that needs withheld bytes must pass the flag.

Bytes retrieved via fetch from `dl-withheld` are cached under a distinct namespace next to regular entries:
`registry/cache/<index>/withheld~foo-1.0.0.crate`, unpacking to `registry/cache/<index>/withheld~foo-1.0.0`. This
prevents a one-time use of withheld bytes from leaking into ordinary builds that have a stale index cache. The
same directory placement matches the expectations of existing cache tooling, such as Cargo's global cache tracker.

The `--fetch-withheld` flag requires exact coordinates (`foo@1.0.0`) rather than simply `foo`. This guards against
resolving to newer versions published by external parties, when a publisher is accessing their own crate.

`--fetch-withheld` only controls resolution and byte fetching, not publish policy. Whether a publish proceeds upon
encountering a withheld dependency is governed by default Cargo behaviors and the `--continue-on-quarantined` and
`--fail-on-unavailable` flags (see "Publishing crates that might become withheld...").

Related:
- Rationale: Why exact coordinates only?
- Rationale: Why a separate dl-withheld path rather than serving withheld bytes from dl?
- Rationale: Why errors never include a bypass command

#### Stale-cache risks

This RFC does not change Cargo's crate cache or index cache lifecycles. A withheld version remains usable through
normal resolution if and only if all of the following conditions hold:
1. The local index file was cached before the version was withheld and has not been refreshed
2. The `.crate` bytes are already in the local cache, so no download is attempted
3. The resolution is satisfiable via the locally cached index without needing a refresh, for instance because its 

In this case, the cached index is refreshed on the next change to `Cargo.toml` or `Cargo.lock` that touches the
registry source, on `cargo update`, or on cache enviction. Until then, an already-withheld version keeps building.
Caching of `~/.cargo` in CI (for instance, with `Swatinem/rust-cache`) preserves this state across CI runs. A mirror
serving the same snapshot produces a similar effect.

Crate bytes admitted via `--fetch-withheld` are stored in the distinct `withheld~` namespace and are never used
by ordinary builds, so do not add to this exposure.

Related:
- Drawbacks: Warm caches keep withheld versions buildable
- Future possibilities: Probing cached crates for withholding

#### Publishing crates that might become withheld at publish-time or have withheld dependencies

`cargo publish` MUST handle a withheld result in a way that clearly captures its state for the user, in both a single
invocation (`cargo publish --workspace`) and across separate invocations (such as `release-plz`'s planner).

The resulting default behavior depends on the kind of withholding:
- A crate whose upload results in `unreleased`, or depends on an `unreleased` version, is successfully published, reported with a `note:`, and the invocation continues with an eventual 0 exit.
Routine holds MUST NOT fail a release train by default.
- A crate whose upload results in `quarantined` or `withdrawn` fails the invocation: Cargo reports an `error:`,
publishes nothing further, and exits non-zero. A crate that depends on a `quarantined`
or `withdrawn` version is not uploaded at all. Already-uploaded crates are left in place. The non-zero exit indicates
failure but not a rollback.
- Poll timeouts keep their existing behavior.

In the above cases, "depends on a <status> version" means that either a sibling publish in a workspace invocation
returned that status, or a dependency was admitted via `--fetch-withheld`.

Two additional flags change the above defaults and are described below under "Publish flags".

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

- `--continue-on-quarantined`: the invocation continues past a `quarantined` or `withdrawn` result of a the crate
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
"Fetching withheld bytes...", but does not change policies around skipping uploads or halting. This is primarily needed
in the case of separate `cargo publish` invocations (see "Separate `cargo publish` invocations").

Related:
- Drawbacks: A quarantine can break a release train
- Rationale: Why is the publish default different per type of withholding?
- Prior art: npm publish-time scanning (#203413)

---

# NOTE: below here is pretty rough, still WIP, and out of date with different handling for quarantine vs unreleased,


##### Polling for status

<modify to explain quarantine exiting>

Cargo publish polls for status after each crate that it publishes. This takes the form of looking for the index summary
line for that version. Withheld crates will show up on this line, so suits our needs if we pass their status through.

When the poll sees the release's index line with `"withheld": "unreleased""quarantined"|"withdrawn"`, `cargo publish`
prints `Found quarantined ...` (or `Found withdrawn ...`) in place of `Published`, followed by a warning
similar to the resolve-time error, and continues on to remaining crates.

In the underlying poll logic, this will require two small changes:
- `poll_one_package()` should pass through which `IndexSummary` variant it sees rather than only confirming existence
- `RegistrySource::query` should pass through `IndexSummary::Quarantined/Withdrawn` to the callback similar to `Yanked`

We do not currently cover the case where a registry opts not to publish an index entry at all until some extra
review gate is passed. All our current behaviors assume an explicit marker (eg, `quarantined`) is set in the index
entry. Prior to introducing any such behavior to crates.io in a subsequent release, we would also improve the poll
workflow to handle such cases clearly. <need to move this mostly into future possibilities>

##### Workspace publish

The workspace publishing case is straightforward and needs no special handling. For workspace publishing, 
the `cargo publish` `build_lock` and verify build runs in an ephemeral workspace whose dependencies resolve from the 
registry, but with a `TmpRegistry` local overlay of the freshly packaged crates layered on top. The overlay is used to 
resolve publication-time siblings via name + version prior to checking upstream. This means that a sibling that is 
quarantined upstream is transparently resolved from the overlay during the verify build. This applies regardless of 
whether the sibling dependency was based on `path` or regular versions, since anyway the verify build converts path 
declarations to concrete versions. 

<modify to explain quarantine exiting>

<explain that we check during the plan loop rather than relying on a build failure so we can halt on workspace
siblings that use the local overlay>


Dependency found `quarantined` in the same invocation (exit 101):
```
    Uploaded a v1.2.3 to registry `crates-io`
note: waiting for a v1.2.3 to be available at registry `crates-io`
       Found quarantined a v1.2.3 at registry `crates-io`
error: a v1.2.3 was quarantined by registry `crates-io` after upload
  |
  = note: the upload succeeded; a v1.2.3 remains on the registry in quarantined state and is not installable
  = note: not publishing remaining crates: b v1.2.3
  = help: for more information see https://crates.io/crates/a/1.2.3
```

##### Separate `cargo publish` invocations

We have a harder time if crates are published in separate invocations. For instance, `release-plz` computes its
own publish ordering and invokes `cargo publish` once per crate. This means that we do not have the shared
local overlay with sibling crates. Instead, both the packaging step (which generates the tarball's `Cargo.lock`) 
and verify build runs a fresh resolve against the real upstream index. So, withheld siblings will break this workflow
by default.

Instead, if the publisher is confident of the withheld upload's provenance, they can use the `cargo publish  --fetch-withheld`
flag to allow usage of a withheld dependency.

Example: dependency found `unreleased`, admitted via `--fetch-withheld a@1.2.3` (continue; exit 0):
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
note: b v1.2.3 was built against unreleased a v1.2.3
  |
  = note: consumers will resolve `a = "^1.2"` to another published version until a v1.2.3 is released
  = help: for more information see https://crates.io/crates/a/1.2.3
```


###### release-plz 

<needs discussion of quarantine-related stopping, passing through the publish flags to change semantics>

Note: `release-plz` is not a RustLang project and its changes do not require approval in this RFC. This discussion
is provided primarily for explanatory purposes to show that there are solutions available for `release-plz`-like 
use-cases. Specific approaches will be discussing with maintainers via issue/PR in the `release-plz` repository.
This will still be tracked as work in scope for this RFC's implementation, since we take `release-plz` support as a
pre-requisite for subsequent work on crates.io adding quarantine actions (in a subsequent RFC).

`release-plz` wants to ensure that we pass `cargo publish --fetch-withheld ...` for all local dependencies
that we are guaranteed to have directly published in a preceding step, that did not land in the quarantine status. This includes:
- A single uninterrupted invocation that publishes a series of crates over multiple `cargo publish` invocations without collisions
- A re-run of a previous workflow that was interrupted after publishing a crate successfully (that was then quarantined), IF we can prove that the uploaded release came from our own usage

Underneath the hood, `release-plz` first checks for existence of all local versions via git tags and crates.io lookups.
If non-existent, we know we need to publish them, and can include the corresponding `--fetch-withheld` commands in each
subsequent step if we didn't see a collision on publish.

If existent, we don't have a guarantee that we were the ones tha published the given release, today. We can adopt
a layered approach to help with this:
1. If the index status is not `quarantined` (via `cargo info`), then no `--fetch-withheld` is needed; skip it.
<cargo info is no longer the mechanism, need to fix this to refer to just checking the sparse index>
2. `release-plz` already checks for git tags as the first step of existence checks (`chore: Release package foo version 1.0.0`).
We can treat presence of such a tag as provenance to allow us to include `--fetch-withheld foo@1.0.0`. If a malicious 
actor is pushing arbitrary git tags up to a release-plz-managed repository, we have larger problems than worrying about
building withheld bytes during verify builds.

We could imagine further fallbacks for already-existing crates when release-plz doesn't have git tags enabled, such as 
manually regenerating and hashing tarballs and  comparing them to registry-side checksums. But, things get pretty nasty 
with reproducibility - do we unpack the withheld  bytes to get the `Cargo.lock` (if our dependencies changed) and
`.vcs_info.json` (if our git HEAD changed)? Are we resilient to cargo changing underneath us? For now, I suggest that 
we punt on this to be handled until we have evidence of broader impact.

In the meantime, `release-plz` can document steps for manually generating git tags to overcome edge cases around
out-of-band releases encountering quarantines that come from trusted sources.

Example with dependency found `unreleased`, admitted via `--fetch-withheld` (continue; exit 0):
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
note: b v1.2.3 was built against unreleased a v1.2.3
  |
  = note: consumers will resolve `a = "^1.2"` to another published version until a v1.2.3 is released
  = help: for more information see https://crates.io/crates/a/1.2.3
```


###### Other multi-publish-invocation build tools

Maintainers doing equivalent multi-crate publishes with separate `cargo publish` invocations will hit similar issues,
without the out-of-box fix we propose for `release-plz`. To help with this, `cargo publish` SHOULD, on verify build
failure, in addition to the standard quarantined-status-related errors, print helper output pointing to a section of the
[publishing docs](https://doc.rust-lang.org/cargo/reference/publishing.html) covering how to work around publishing
errors. That should include the option of passing in the `--fetch-withheld` option.
The `--fetch-withheld` option should be heavily caveated that it should only be used for trusted, publisher-controlled
inputs, such as a version that the same script successfully published beforehand.

### docs.rs impact

For this RFC, we restrict docs.rs to only refraining from building docs for quarantined/withdrawn crates, and instead
displaying a status tag. If the state transitions from quarantined to published, we can trigger a build and remove the tag.

This does leave a hole: what if resolution fails because a crate exclusively resolves to another quarantined crate? For the
purposes of this RFC, we can leave this out of scope. This would primarily be a problem for quarantined-at-the-point-of-publish
registry behaviors, since then we need a way to trigger transitive rebuilds when the dependents transition to published.
We should scope the distributed systems work to support these actual state transitions on docs.rs side into the RFC adding such behavior to crates.io.

That said, theoretical solutions do exist for this, for instance: docs.rs internally tracks build-time resolution failures due to 
quarantined dependencies and triggers rebuild if that dependency transitions to published. Alternatives exist on the crates.io
side as well, though I am nervous about adding the cost of reverse dependency walking across a possibly huge corpus of
dependents. There also are some fun bidirectional options where docs.rs identifies reverse dependency breakage, but
signals it to crates.io so that it can enrich its user-facing displays on those broken crates.

Regardless, the important piece here is that this is fundamentally not a build-tool or registry-publish-API side problem,
because state transitions happen out of band with publication. More discussion of this is in the Rationale/alternatives
section that follows.

To summarize:
1. Add support for `status` in `crates-index-diff`, `docs_rs_crates_io`, and the pending `crates.io`-event-based-sending ([#14188](https://github.com/rust-lang/crates.io/pull/14188))
2. On docs.rs, skip building when a crate is seen with `status: "quarantined"/"withdrawn"`
3. On docs.rs, when a crate is seen with `status: "quarantined"/"withdrawn"`, add a tag marking it as such and pointing to crates.io
4. On docs.rs, on seeing a status transition to `published`, trigger a build if one has not already run, and remove the status tag
5. On docs.rs, if a build fails to resolve due to quarantined dependencies, do nothing special (for now) and wait to address this use case until a subsequent RFC that implements "quarantine-at-point-of-publish" support to crates.io


## Drawbacks
[drawbacks]: #drawbacks

A second mutable index field

An index line no longer guarantees fetchable bytes

Current (and in the git index, past) withheld statuses are publicly visible. See "Rationale: Why are withheld statuses public?"

Existing lockfiles can break without a local change
- especially painflu for cargo.lock

Warm caches keep withheld versions buildable
- this is true of any design that doesn't add in cache evictions, which we scope in future possibilities

Quarantine can break release trains
- (Intended, see Why is the publish default different per type of withholding)

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Quick hits, LLM generated based on my notes, before I can get to writing them up:
| Alternative | Why not |
|---|---|
| Overload `yanked` | Means "author says don't use"; cannot distinguish redacted bytes from a maybe-broken release. See *Why `withheld` status instead of (further) overloading `yanked`?* |
| Withheld line absent from the index | Untenable for `quarantined` and `withdrawn`: pinned builds fail without saying why, no tombstone, delete-then-republish for diff-driven consumers. Defensible for
`unreleased`, where nothing is pinned yet and a feed could replace the line; chosen against because separate-invocation publishes would need a second source of index summaries, and one mechanism covers all
three states. The index-absent form of `unreleased` remains available later as delayed indexing if we added additional support in build tools. See *Why not remove the index line?* and *Future possibilities: Delayed indexing* |
| Withheld bytes at the ordinary `dl` URL | Protects fresh Cargo resolution only; deterministic URL-plus-checksum fetchers acquire the bytes without reading the index. See *Why a separate `dl-withheld` path
rather than serving withheld bytes from `dl`?* |
| Authenticated withheld bytes | Access control, token recovery, and researcher onboarding with no improvement in default acquisition; excludes Trusted Publishing. See *Why is `dl-withheld` as open as `dl`?*
and *Why nothing after upload requires the publisher to authenticate* |
| Global `--allow-withheld` | Admits arbitrary withheld transitive dependencies. See *Why `--fetch-withheld` takes an exact `name@version`* |
| Publish sessions or secret preview tokens | Public bytes bypass access tracking, so tokens cannot enforce provenance; needs a credential that outlives the one Trusted Publishing issues. See *Why no
sessions or preview tokens* |
| Owner-authenticated view of the canonical index | Conflicts with signing, mirroring, and cache isolation; does not fit short-lived Trusted Publishing identities. See *Why nothing after upload requires the
publisher to authenticate* |
| Automatic downstream propagation | False cascades; transitive gaps; the registry is the right layer, if any. See *Why not publish-time propagation of withheld states during a release train?* |
| Registry buildability checks before publish | The registry would need partial resolver semantics across features, targets, and registries. See *Why no registry buildability checks* |
| Structured yank reason | Author-controlled text, not a registry state; says nothing about bytes. Complementary, not a substitute. |
| Informational and blocking states in one field | Every consumer must classify values; PEP 792 affords it only because omission does the enforcing. See *Why is `withheld` blocking-only?* |
| Index protocol version bump | Unnecessary since Cargo 1.51 and blocked by the experimental `v: 3`. See *No index protocol version bump* |
| `min-publish-age` alone (RFC 3923) | Delays admission; cannot preserve an adverse verdict, especially for quarantines on already released crates. Even for fresh publish, the posture of needing to hit the "delete" button very quickly is not sustainable as attack volume grows. Companion, not substitute. |
| Universal release staging | Sessions, embargoes, atomic groups, upload/publish role separation — too broad; this design can evolve into it. See *Future possibilities: Author-managed staging* |
| Do nothing | Responders choose between yank (installable when pinned) and deletion (irreversible, destroys evidence); nobody gets a signal. Also does not set us up for the "not always having to  |


#### No index protocol version bump

The `withheld` field is valid under all existing `"v": 1 | 2 | 3` (where 3 is experimental).

The criteria for bumping the protocol version is that older versions of Cargo would misinterpret the line or crash.
Bumping the version is particularly awkward currently because we have an experimental `3` value that we do not want to 
release as stable, but we do want to make sure that new index features associated with withholding are immediately 
available. Ultimately protocol version is a format version with a total order, and repurposing it for 

We have wiggle room here since, as of 1.51, Cargo will ignore unknown index fields. Further, we are not changing the 
behavior of existing fields. This gives us a path to treat `withheld` as compatible with all protocol versions.

The failure mode that we are concerned about is that resolution should skip resolving entries with a withheld
status, but ignoring that withheld marker might still land on those entries. Bytes will remain unavailable server-side 
(unless we have a stale cargo cache, which is already treated as out of scope for this feature, discussed under "Stale-cache risks"). But, we do still have a backwards compatibility problem where we might fail to find a resolver solution that would exist using a non-withheld version, if Cargo knew to backtrack.

As a mitigating measure, we can nudge the resolver to avoid this by requiring that registries MUST set `yanked: true` 
when transitioning to a withheld status. This is discussed further in "Managing withheld status transitions". This
does still result in builds with `Cargo.lock` pointing to withheld crates, or build tools that don't respect yanks,
still resolving to the withheld releases, which will fail. But, even a withheld-aware build tool would fail the same
builds, albeit with better error messages. Our registry spec says that registries SHOULD offer helpful not-found
response bodies on `dl`-endpoint fetches of withheld bytes, which keeps user experience decent.

So, we don't risk miscompilations or especially confusing errors. With the `yanked: true` nudge, we are providing a 
fairly good user experience in most cases, and otherwise clear errors. This should be sufficient to avoid needing a 
protocol version bump.


### Why `withheld` status instead of (further) overloading `yanked`?

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

---


### Why not `"withheld": "quarantined_and_yanked`?
Simplifies registry stake-keeping slightly at the cost of an explosion of index states (every future state will need 
the same `_and_yanked`) that is not more comprehensible to the audience. We should optimize for simple wire
representation that reflects how build tools should interact with versions.


### Why not publish-time propagation of withheld states during a release train?
We can imagine alternative methods that attempt to propagate quarantined status to 
dependents based on poll status, with a registry publish API extension. This is useful both for avoiding broken binary versions,
and also generally offering clearer visualization of what a leaf crate is unreachable due to exclusively quarantined 
dependencies. But, this is a bit of a trap, in that we would not be probing more deeply for quarantined transitive 
dependencies and also in that dependencies can become quarantined underneath us. 

The proper place for propagation or transitive display of quarantined state would instead be at the registry level, if at all. In other words, that is a discussion for
a subsequent RFC.

### Why is withheld status publicly visible?
- We don't have great options for legibility like other registries since we have per-line release information with no 
overall crate status that captures i
- APIs to silently withdraw tombstones
or otherwise harden registries against oracle attacks are largely server-side decisions and will
be discussed in subsequent issues and RFCs. Publishers will anyway be able to tell whether their release is quarantined 
via any reasonable registry implementation, so we do not view placing this status into the index as a critical 
information leakage.
- same holds for history: the git index retains past withholdings, the sparse index does not, and a version's having been cleared is itself useful information for researchers.

###  Why is dl-withheld as open as dl? 
 — covers: unreleased bytes always served (external review in the review window is the point; registry scanners see everything regardless; existence is public); no author-privacy exception in this RFC (the carve-out an attacker would use; deferred to a staging RFC); no credential gate for researchers (PyPI's Observer-only visibility API was considered and dropped; index visibility is public anyway). Closing sentence: if public-by-default proves wrong, per-template auth-required in config.json is the natural mechanism.

### Why is the publish default different per type of withholding?
- unreleased shouldn't break release train (see: npm)
- quarantine indicates a verdict about the publisher that should break CI and cause attention, similar to
--allow-dirty to force attention


## Prior art
[prior-art]: #prior-art

<not organized, analysis is LLM generated and will be rewritten in full>
- [PyPI: Project Quarantine (2024-12)](https://blog.pypi.org/posts/2024-12-30-quarantine/) — admin-set, reversible; enforced by omission from the Simple index; yank considered and rejected ("a yanked Release is still installable"); ~140 quarantined, 1 released.
- [PyPI: project status markers (2025-08)](https://blog.pypi.org/posts/2025-08-14-project-status-markers/) / [PEP 792](https://peps.python.org/pep-0792/) — legibility marker retrofitted a year after omission-based quarantine; project-scoped; mixes informational (`archived`, `deprecated`) with enforcing (`quarantined`); works only because omission does the enforcing.
- [warehouse `Release.lifecycle_status`](https://github.com/pypi/warehouse/blob/main/warehouse/packaging/models.py) — release-level quarantine since 2026; omission in `_simple_detail`; no per-release marker in the standard API. Same shape as `withheld: quarantined`, illegible on the wire.
- [PEP 592: yanked releases](https://peps.python.org/pep-0592/) — yank = soft delete that stays installable when pinned; owner-only; reason carried in the index. The contract this RFC preserves for `yanked`.
- [PEP 694: upload API, staged releases](https://peps.python.org/pep-0694/) — staged = session state, absent from the index until published. Contrast: `unreleased` is a visible line.
- [Pre-PEP thread: status markers](https://discuss.python.org/t/pre-pep-discussion-project-status-markers-in-the-index-apis/79356) — design discussion; its description of quarantine as "yanked" does not match warehouse's implementation.
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
- Should we supported `unreleased` in the index or scope it out entirely and require that registries only handle
unreleased via delayed indexing + deeper changes to allow retrieving index summaries for multi-invocation publishes?
- The default publish behaviors for unreleased vs quarantined, and the naming of the flags. For this, we also will
want input from `release-plz`'s maintainer. 
- If we actually want `--fetch-withheld` on `cargo build` and `cargo install` (for forensic build usage) or only
`cargo fetch` and `cargo publish`
- Whether we should make `cargo_util_schemas::index::IndexPackage` `#[non_exhaustive]` while we are bumping semver anyway

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

Triggering reverse dependency re-processing based on withholding changes

Delayed indexing for unreleased
- A good middle ground to avoid index thrash might be only publishing to the sparse index but not the git index
- To avoid publishing index lines for unreleased crates, we need an alternative way to serve the full index line,
or else we break separate publish invocations
- At some point, we might want to publish a dedicated event feed of quarantined or withdrawn releases (along with yanked 
releases). That is out of scope for this RFC. For the time being, researchers will have access to the git index
for a change stream that includes status transitions. Any movement away from hosting the git registry, will need to 
answer questions around how to reliably access an event stream to use alongside the sparse index.


Author-managed staging
- https://internals.rust-lang.org/t/pre-rfc-package-staging/20459
- Privacy for author-managed staging

Restricting dl-withheld to credentialed researchers. 
- This RFC makes dl-withheld as open as dl on the same registry: public on crates.io, token-gated on an auth-required registry. A registry wanting to limit withheld bytes to vetted researchers — the model PyPI's Observer program gestured at — would need per-template authentication in config.json (for example, an auth-required map keyed by template). That is deferred; index visibility of withheld versions is public regardless, so the gate would protect bytes, not existence.

Smarter cargo install --locked on withheld dependencies

`cargo info` support
- today cargo info will never show "yanked" since it only shows candidate versions (missing from index view,
not found for specific requested version)
- we could enhance it to support both yanked and withheld with extra visibility
- This RFC exposes all the information necessary to support this via the `IndexSummary::Withheld` variant if prioritized


Probing cached crates for withholding (and yanked state)
- HEAD request to check for byte existence as a quick proxy for needing a refresh
- run it in CI probably?
- larger Cargo change that deserves its on RFC

Better display of reasons for withholding:
- today, registry tracked, available on human readable page (similar model to yanked once frontend is implemented)
- we could add a registry endpoint that surfaces this in a machine-readable format for cargo to use on different failures and warnings
- we could also write it to index lines (as part of the conversation about yanked)

---

Think about what the natural extension and evolution of your proposal would
be and how it would affect the language and project as a whole in a holistic
way. Try to use this section as a tool to more fully consider all possible
interactions with the project and language in your proposal.
Also consider how this all fits into the roadmap for the project
and of the relevant sub-team.

This is also a good place to "dump ideas", if they are out of scope for the
RFC you are writing but otherwise related.

If you have tried and cannot think of any future possibilities,
you may simply state that you cannot think of anything.

Note that having something written down in the future-possibilities section
is not a reason to accept the current or a future RFC; such notes should be
in the section on motivation or rationale in this or subsequent RFCs.
The section merely provides additional information.


