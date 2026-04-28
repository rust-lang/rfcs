- Feature Name: cargo_min_publish_age
- Start Date: 2026-02-23
- RFC PR: [rust-lang/rfcs#3923](https://github.com/rust-lang/rfcs/pull/3923)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary
[summary]: #summary

This proposal adds a new configuration option to cargo allowing users to specify a minimum age for dependency versions.
When specified, Cargo won't use a version of a registry crate
that is newer than the minimum age,
with a way to override for exceptions like urgent security fixes.

An example configuration would be:

```toml
[registry]
global-min-publish-age = "14 days"
```

## Motivation
[motivation]: #motivation

There are a couple of reasons why one may wish not to use the most recent version of a package:

Some [supply chain attacks](https://en.wikipedia.org/wiki/Supply_chain_attack)
are found by automated scanners on newly published package versions.
Recent supply chain attacks on the NPM ecosystem have drawn attention to the value of waiting on these
automated scanners.
For more background on version maturity requirements as a risk mitigation, see
[We should all be using dependency cooldowns](https://blog.yossarian.net/2025/11/21/We-should-all-be-using-dependency-cooldowns) and
[Dependency cooldowns, redux](https://blog.yossarian.net/2025/12/13/cooldowns-redux).

There would be value in a gradual roll out scheme for the ecosystem.
New versions can introduce inadvertent breaking changes, bugs, or security vulnerabilities.
Having everyone discover these problems at once leads to a wider, costlier disruption to the ecosystem.
Some maintainers are fine being on the bleeding edge, taking on those costs, and act as a canary for the ecosystem.
Those who are more risk averse can choose how much stagnation they are willing to accept for others to discover these problems and get them worked out.
Maintainers may even want to blend these in one project: keep risks down for local development while CI has a dependency version canary job to identify future problems and track their status.
Granted, this only helps if the problems are discovered by yourself or others.  Any fixes will also be subject to the minimum-release age but at least these will be available to upgrade to so long as there is an exception mechanism.

Allowing maintainers to encourage a certain degree of maturity for dependency versions can help these use cases.

Note that this is **not** a full solution to compromised dependencies. It can increase the protection against certain types of
"supply chain" attacks, but not all of them. As such, using this feature should not be relied upon for security by itself.


## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The `registry.global-min-publish-age` [configuration option][1][^1] for Cargo can be used to specify a minimum age for published versions to use.
When set, Cargo treats versions with a publish time ("pubtime") newer than that duration like yanked versions:
Cargo will not use a too-new version unless it is already recorded in `Cargo.lock`,
and will generate an error if there are no longer any compatible versions.

For example, in your `<repo>/.cargo/config.toml`, you may have:

```toml
[registry]
global-min-publish-age = "14 days"
```

Running `cargo update` will look something like:
```console
$ cargo update
Updating index
 Locking 1 package to recent Rust 1.60 compatible version
  Adding some-package v1.2.3 (available: v1.3.0, published 2 days ago)
```

While a CI job runs:
```
env:
  CARGO_RESOLVER_INCOMPATIBLE_RUST_VERSIONS: allow
  CARGO_RESOLVER_INCOMPATIBLE_PUBLISH_AGE: allow
steps:
  - uses: actions/checkout@v4
  - run: rustup update stable && rustup default stable
  - run: cargo update --verbose
  - run: cargo build --verbose
  - run: cargo test --verbose
```

This will mean that:

- Locally, `cargo update` will only select versions older than the minimum publish age,
  e.g., `some-package@1.2.3`
- This CI job will verify the latest versions of your dependencies,
  e.g., `some-package@1.3.0`

### Per-registry configuration

It is also possible to configure the `min-publish-age` per cargo registry.
`registries.<name>.min-publish-age` sets the minimum publish age for the `<name>` registry.
And `registry.min-publish-age` sets it for crates.io.

For example:
```toml
[registries.my-org]
index = "https://my.org"
min-publish-age = "0" # this registry is fully trusted

[registry]
# Default for any registry without a specific value
global-min-publish-age = "14 days"
# Value to use for crates.io
min-publish-age = "5 days"
```

This will use a minimum publish age of
- 5 days for crates.io
- no minimum for `my-org`
- 14 days for any other registry.

### When no version matches

If no version of a dependency satisfies both the version requirement and the minimum publish age,
the resolve will error, similar to when all matching versions are yanked:

```console
$ cargo update
error: failed to select a version for the requirement `some-package = "^1.3"`
  version 1.3.0 is too new (published 2 days ago, minimum age 14 days)
```

### Using newer versions

In some cases, it may be desirable to use a version that is newer than the minimum publish age.
For example, `some-package` from [earlier](#guide-level-explanation) has a fix for a vulnerability in v1.3.0.

Since too-new versions follow yanked semantics,
the same override mechanisms apply:

```console
$ cargo update some-package --precise 1.3.0
warning: selected package `some-package@1.3.0` is too new
  = note: published 2 days ago, minimum age 14 days
    Updating some-package v1.2.3 -> v1.3.0
```

To record the intent more permanently,
bump the version requirement in `Cargo.toml`.

For a broader override, the `CARGO_RESOLVER_INCOMPATIBLE_PUBLISH_AGE=allow` environment variable
disables the check entirely.

[1]: https://doc.rust-lang.org/cargo/reference/config.html
[^1]: As specified in `.cargo/config.toml` files

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This RFC adds a few new configuration options to [cargo configuration](https://doc.rust-lang.org/cargo/reference/config.html).

### Added to [Configuration Format](https://doc.rust-lang.org/cargo/reference/config.html#configuration-format)

```toml
[resolver]
incompatible-publish-age = "deny" # Specifies how resolver reacts to these

[registries.<name>]
min-publish-age = "..."  # Override `registry.global-min-publish-age` for this registry

[registry]
min-publish-age = "..."  # Override `registry.global-min-publish-age` for crates.io
global-min-publish-age = "0"  # Minimum time span allowed for packages from this registry
 ```

### Added to [`[resolver]`](https://doc.rust-lang.org/cargo/reference/config.html#resolver)

#### `resolver.incompatible-publish-age`

* Type: String
* Default: `"deny"`
* Environment: `CARGO_RESOLVER_INCOMPATIBLE_PUBLISH_AGE`

When resolving the version of a dependency, specify the behavior for versions with a `pubtime` (if present) that is incompatible with `registry.min-publish-age`. Values include:

* `allow`: treat pubtime-incompatible versions like any other version
* `deny`: treat pubtime-incompatible versions like yanked versions

 See the [resolver](https://doc.rust-lang.org/cargo/reference/resolver.html#rust-version) chapter for more details.

### Added to [`[registries]`](https://doc.rust-lang.org/cargo/reference/config.html#registries)

#### `registries.min-publish-age`

* Type: String
* Default: none
* Environment: `CARGO_REGISTRIES_<name>_MIN_PUBLISH_AGE`

 Specifies the minimum timespan since a version's `pubtime` that it may be considered for `resolver.incompatible-publish-age` for packages from this registry. If not set, `registry.global-min-publish-age` will be used.

 Will be ignored if the registry does not support this.

 It supports the following values:

* An integer followed by “seconds”, “minutes”, “hours”, “days”, “weeks”, or “months”
* `"0"` to allow all packages

### Added to [`[registry]`](https://doc.rust-lang.org/cargo/reference/config.html#registry)

#### `registry.min-publish-age`

* Type: String
* Default: none
* Environment: `CARGO_REGISTRY_<name>_MIN_PUBLISH_AGE`

 Specifies the minimum timespan since a version's `pubtime` that it may be considered for `resolver.incompatible-publish-age` for packages from crates.io not set, `registry.global-min-publish-age` will be used.

 It supports the following values:

 * An integer followed by “seconds”, “minutes”, “hours”, “days”, “weeks”, or “months”
 * `"0"` to allow all packages

Generally, `"0"`, `"N days"`, and `"N weeks"` will be used.

#### `registry.global-min-publish-age`

* Type: String
* Default: `"0"`
* Environment: `CARGO_GLOBAL_REGISTRY_<name>_MIN_PUBLISH_AGE`

 Specifies the global minimum timespan since a version's `pubtime` that it may be considered for `resolver.incompatible-publish-age` for packages. If `min-publish-age` is not set for a specific registry using `registries.<name>.min-publish-age`, Cargo will use this minimum publish age.

 It supports the following values:

* An integer followed by “seconds”, “minutes”, “hours”, “days”, “weeks”, or “months”
* `"0"` to allow all packages

### Behavior

In addition to what is specified above

* Only applies to dependencies fetched from a registry that publishes `pubtime`, such as crates.io.
  * They do not apply to git or path dependencies, in
    part because there is not always an obvious publish time, or a way to find alternative versions.
  * They do not apply to registries that don't set `pubtime`, as there is no reliable way to know when the version was published.
  * For [source replacement],
    registry mirrors are expected to preserve `pubtime` from the index to be applicable.
    Local registries and directory (vendored) sources typically don't have `pubtime`,
    so the check does not apply.
* When resolving dependencies,
  pubtime-incompatible versions follow the same rules as [yanked versions]:
  * They are not considered for new resolves,
    but if already present in `Cargo.lock`, they are left in place.
  * If no version satisfies both the version requirement and the minimum publish age,
    the resolve will error.
  * A status message will be printed when selecting a non-latest version
    as well for incompatible versions.
  * [`cargo update --precise`] can override the policy,
    just as it can select a yanked version.
  * `cargo install` skips pubtime-incompatible versions as well

[source replacement]: https://doc.rust-lang.org/cargo/reference/source-replacement.html
[yanked versions]: https://doc.rust-lang.org/cargo/reference/resolver.html#yanked-versions
[`cargo update --precise`]: https://doc.rust-lang.org/cargo/commands/cargo-update.html#option-cargo-update---precise

## Drawbacks
[drawbacks]: #drawbacks

The biggest drawback is that if this is widely used, it could potentially lead to it taking longer for problems to be discovered after a version is published.
However, most likely, there will be a spread of values used, depending on risk tolerance, and hopefully the result is actually that there will be a more gradual rollout in most
cases.

Also, even if all users of a crate set a minimum publish age there is still value in a delay, because it provides time for automated security scanners, and human reviewers
to review the changes before the new version is pulled in by updates. And in the case of a malicious release made using compromised credentials, it give the actual developer
time to realize their credentials have been compromised and yank the version before it is widely used.

## Rationale and Alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

### Configuration Locations

The locations and names of the configuration options in this proposal were chosen to be
consistent with existing Cargo options, as described in [Related Options in Cargo](#related-options).

### Configuration Names

The term "publish" was used rather than "package", "version", or "release" to make it
clear that this only applies to crates that are published in a registry.

`publish` is redundant with this being in the `registry` table.
This helps with the above disambiguation and for clarity in discussing this as a shorthand.

`cooldown` was avoided due to term generally referring to throttling while we are looking for a certain maturity.

### Starting with `deny`

`resolver.incompatible-publish-age` starts with `allow` and `deny`.

Unlike `resolver.incompatible-rust-versions` which starts with `fallback`,
`deny` is viable here because `pubtime` data is exhaustive.
crates.io sets it for every version once backfilled,
so there are no gaps that would cause spurious errors.

A `fallback` option would deprioritize too-new versions but still allow them as a last resort.
This is deferred because it opens the yank attack vector:
a malicious actor with right permissions could publish a malicious version and yank the safe versions.
It then forces the resolver to fall back to the malicious too-new version.
`deny` prevents this by erroring instead of falling back.

`fallback` may be useful in the future for risk-tolerant workflows
that prefer a degraded resolve over an error,
particularly when combined with other tools that validate pubtime-incompatible versions.
It would also help with `cargo update --precise` for packages with transitive dependencies.
For example,
`CARGO_RESOLVER_INCOMPATIBLE_PUBLISH_AGE=fallback cargo update clap --precise 4.5.3`
would pull in only the necessary too-new transitive dependencies
rather than disabling the check entirely with `allow`.

### Timestamp vs duration

Some prior art
- exclusively use a timestamp
- allow either a timestamp or a relative time within the same field

While a timestamp has its uses
(see [`--publish-time`](https://doc.rust-lang.org/cargo/reference/unstable.html#lockfile-publish-time)),
it wouldn't be as ergonomic for this use case.

Designing the field to support both would create a trap for users trying to reproduce a problem from the past in that they are likely to set the timestamp but overlook that they need to take the existing duration into account.
Even if they do remember to take the existing duration into account,
it would be more convenient if they can be set separately.

Setting the timestamp to resolve to is left as a future possibility

### Per-registry configuration

Allowing the minimum age to be configurable per registry provides a simple mechanism
to use different minimum ages for different sets of packages, including possibly no
minimum in common situations such as using an internal registry where the crates
are completely trusted.

This makes it less necessary to have more complicated configuration for rules for including
and excluding sets of packages from the age policy, or setting different age policies
for different packages.

### Exclude list

Exclude lists tend to be used either for:
- Forcing a specific newer version: we have this covered through `cargo update --precise`
- Marking a source as always trusted: we have this covered through per-registry configuration

One problem with an exclude list is that they tend to be a static solution (all versions) for a transient problem (a subset of versions).
This can lead people open to an attack after a high-value upgrade.
An exclude list of just names is helpful for "I have a trusted package source" scenario,
but less so for "I need a security fix now".
The user must remember to remove the exclusion once it is no longer needed,
or they lose protection for future versions of that package.
We could make the exclude list use the [Package ID Spec](https://doc.rust-lang.org/cargo/reference/pkgid-spec.html) format and even require a full version to be specified.

Users likely will need to exclude transitive dependencies as well.
For instance, to use a too-new version of `clap`, you may also need to exclude `clap_builder`, `clap_derive`, and `clap_lex`.

An exclude list can always be added in the future if a strong enough use case presents itself.
By delaying, we can also take into account any future changes.
For example, if the focus is on different levels trust within the same registry,
we could design a solution around [registry namespacing](https://internals.rust-lang.org/t/survey-of-organizational-ownership-and-registry-namespace-designs-for-cargo-and-crates-io/24027/4),
assuming support is added.

### Using Cargo.toml and Cargo.lock (i.e. "do nothing")

You can pin versions in your `Cargo.toml` but that is a manual process and doesn't cover transitive
dependencies.

Users can manage all of their direct and transitive dependencies in a `Cargo.lock` file but that is tedious and it is easy to overlook new entries on implicit lockfile changes.

### Why not leave this to third party tools?

There are already some third party tools that fulfill this functionality to some degree. For example, dependabot and renovate can
be used for updating Cargo.toml and Cargo.lock, and both support some form of minimum publish age. And the cargo-cooldown project provides
an alternative to `cargo update` that respects a minimum publish age.

However, these tools only work for updating and adding dependencies outside of cargo itself, they do not
have any impact on local changes, like directly editing `Cargo.toml` causing an implicit `Cargo.lock` update, `cargo update`, or `cargo add`.

## Prior Art
[prior-art]: #prior-art

["Package Managers Need to Cool Down"](https://nesbitt.io/2026/03/04/package-managers-need-to-cool-down.html) discusses several implementations of this in various
package managers (including this RFC).

### Debian "testing"

Debian's "testing" distribution consists of packages from unstable that have been in the "unstable" distribution for a certain minimum age (2-10 days depending on an `urgency` field in the package changelog), have been built for all previously supported targets, have their dependencies in testing, and don't have any new release-critical bugs.

Users of "unstable" include early adopters who don't mind being the canary when things break (and reporting the aforementioned bugs, release-critical or otherwise). Users of "testing" get slightly older packages and a reduced chance of release-critical bugs.

### pnpm

`minimumReleaseAge` is a configuration option which takes a number of minutes as an argument. It then won't update or install releases that were released less than that many minutes ago. This also applies to transitive dependencies.

`minimumReleaseAgeExclude` is an array of package names, or package patterns for which the `minimumReleaseAge` does not apply, and the newest applicable release is always used. It also allows specifying specific versions to be allowed.

Both configuration options can be set in global config, a project-specific config file, or with environment variables (for a specific invocation).

### yarn

Has a configuration setting that can be used in `.yarnrc.yml` named `npmMinimalAgeGate` that can be used to set the minimum age for installed package releases. It looks like it allows specifying units, as the example for three days is `3d`, however I haven't found any definitive description of the syntax.

As far as I can tell, there is no way to provide exclusions to this rule, or different times for different packages or repositories.

### uv

The `--exclude-newer` option can be used to set a timestamp (using RFC 3339 format), or a duration (either "friendly" or ISO 8601 format)
and won't use releases that happened after that timestamp. There is also an `--exclude-newer-package` option, which allows overriding the `exclude-newer` time for individual packages.

Both of these settings can also be used in the `uv` configuration file (`pyproject.toml`).

### pip

Pip has an `--uploaded-prior-to` option that only uses versions that were uploaded prior to an ISO 8601 timestamp. Can also be controlled with the `PIP_UPLOADED_PRIOR_TO`
environment variable.

### dependabot

The `cooldown` option provides a number of settings, including:

- `default-days` – Default minimum age of release, in days
- `semver-major-days`, `semver-minor-days`, `smever-patch-days` -- Override the cooldown/minimum-release-age based on what kind of release it is.
- `include` / `exclude` – a list of packages to include/exclude in the "cooldown". Supports wildcards. `exclude` has higher priority than `include`.

"Security" updates bypass the `cooldown` settings.

Dependabot doesn't support cooldown for all package managers.

This is specified in the dependabot configuration file.

### renovate

The options below can be provided in global, or project-specific configuration files, as a CLI option, or as an environment variable.

`minimumReleaseAge` specifies a duration which all updates must be older than for renovate to create an update. It looks like the duration specification uses units (ex. "3 days"), however, again I can't find a precise specification for the syntax.

It is possible to create separate rules with different `minimumReleaseAge` configurations, on per-package basis, or across groups of packages/registries.

"Security" updates bypass the minimum release age checks.

### deno

Deno supports a [configuration option](https://deno.com/blog/v2.6#controlling-dependency-stability) for `minimumDependencyAge` in the configuration file, or
`--minimum-dependency-age` on the CLI. It supports an ISO-8601 duration, RFC 3339 timestamp, or an integer of minutes.

### cargo-cooldown

There is an existing experimental third-party crate that provides a plugin for enforcing a cooldown: [https://github.com/dertin/cargo-cooldown]

### Related Options in Cargo
[related-options]: #related-options-in-cargo

Some precedents in Cargo

[`cache.auto-clean-frequency`](https://doc.rust-lang.org/cargo/reference/config.html#cacheauto-clean-frequency)

> * "never" — Never deletes old files.
> * "always" — Checks to delete old files every time Cargo runs.
> * An integer followed by “seconds”, “minutes”, “hours”, “days”, “weeks”, or “months”


[`resolver.incompatible-rust-versions`](https://doc.rust-lang.org/cargo/reference/config.html#resolverincompatible-rust-versions)

> * Controls behavior in relation to your [`package.rust-version`](https://doc.rust-lang.org/cargo/reference/rust-version.html) and those set by potential dependencies
>
> * Values:
>
> * allow: treat rust-version-incompatible versions like any other version
> * fallback: only consider rust-version-incompatible versions if no other version matched


[`package.resolver`](https://doc.rust-lang.org/cargo/reference/resolver.html#resolver-versions) is only a version number. When adding `incompatible-rust-version`, we intentionally deferred anything being done in manifests.

[`[registry]`](https://doc.rust-lang.org/cargo/reference/config.html#registry)

> * Set default registry
> * Sets credential providers for all registries
> * Sets crates.io values

[`[registries]`](https://doc.rust-lang.org/cargo/reference/config.html#registries)

> * Sets registry specific values

`yanked`: can't do new resolves to it but left in if already there. `--precise` can force it but that doesn't apply recursively.

pre-release: requires opt-in through version requirement. Unstable support to force it with `--precise` but that doesn't apply recursively.

## Unresolved Questions
[unresolved-questions]: #unresolved-questions

* Would it be better to have `registry.min-publish-age` be the global setting, and `registries.crates-io.min-publish-age` be the setting for the crates.io registry?
  The current proposal is based on precedent of "credential-provider" and "global-credential-provider", but perhaps we shouldn't follow that precedent?
* How do we make it clear when things are held back?
  * The "locking" message for [Cargo time machine (generate lock files based on old registry state) #5221](https://github.com/rust-lang/cargo/issues/5221) lists one time but the time here is dependent on where any given package is from
  * We list MSRVs for unselected packages, should we also list publish times? I'm assuming that should be in local time
  * Locking message for [Cargo time machine (generate lock files based on old registry state) #5221](https://github.com/rust-lang/cargo/issues/5221) is in UTC time, see [Tracking Issue for _lockfile-publish-time_ #16271](https://github.com/rust-lang/cargo/issues/16271), when relative time differences likely make local time more relevant
* Implementation wise, will there be much complexity in getting per registry information into `VersionPreferences` and using it?
* `deny` precedence between this and `incompatible-rust-version`?
  * Both produce errors, but `incompatible-rust-version` will likely be evaluated first to increase the chance of builds succeeding.
* Can we, and should we make any guarantees about security when using this feature, such as "a release of a malicious version of a crate will not compromise the build

## Future Possibilities
[future-possibilities]: #future-possibilities

- Support `fallback` for `resolver.incompatible-publish-age`
  (see [Starting with `deny`](#starting-with-deny) for why this is deferred).
- Add an exclude list for `min-publish-age`
  (see [Exclude list](#exclude-list) for why this is deferred).
- When all compatible older-than-min-age versions are yanked
  and a newer non-yanked version exists,
  Cargo could alert the user that they may want to override with `--precise`.
- Potentially support other source of publish time besides the `pubtime` field from a cargo registry.
- A `resolver.now` field for setting the reference time that `min-publish-age` is compared against.
  This could be useful for offline workflows where wall-clock time keeps advancing
  but the registry index may be stale.
