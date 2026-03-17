- Feature Name: cargo_min_publish_age
- Start Date: 2026-02-23
- RFC PR: [rust-lang/rfcs#3923](https://github.com/rust-lang/rfcs/pull/3923)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary
[summary]: #summary

This proposal adds a new configuration option to cargo that specifies a minimum age for package
updates. When adding or updating a dependency, cargo won't use a version of that crate that
is newer than the minimum age when specified, with a way to override to get urgent security fixes.

An example configuration would be:

```toml
[registry]
global-min-publish-age = "14 days"
```

Or it could be specified on the command line with `--config registry.global-min-publish-age '14 days'`.

## Motivation
[motivation]: #motivation

There are a couple of reasons why one may wish not to use the most recent version of a package.

One such reason is to mitigate the risk of [supply chain attacks](https://en.wikipedia.org/wiki/Supply_chain_attack). Often, supply chain
attacks are found fairly quickly after they are deployed. Thus, if the dependency isn't updated
immediately after a release, you can have some protection against a new release of a dependency
containing malicious code. In light of recent supply chain attacks on the NPM ecosystem,
there has been an increased interest in using automated tools to ensure that packages used
are older than some age. This creates a window of time between when a dependency is compromised
and when that release is used by your project. See for example the blog post
"[We should all be using dependency cooldowns](https://blog.yossarian.net/2025/11/21/We-should-all-be-using-dependency-cooldowns)".

Another reason to wish to delay using a new release, is because new versions can introduce new bugs. By only
using versions that have had some time to "mature", you can mitigate the risk of encountering those bugs a little.
Different people (or groups of people) have different tolerance for risk, and this provides
a mechanism whereby new versions can roll out gradually to users depending on the tolerance
for risk of those users.

As such, it would be useful to have an option to put a limit on commands like `cargo add` and `cargo update`
so that they can only use package releases that are older than some threshold.

Note that this is **not** a full solution to compromised dependencies. It can increase the protection against certain types of
"supply chain" attacks, but not all of them. As such, using this feature should not be relied upon for security by itself.


## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The `registry.global-min-publish-age` [configuration option][1][^1] for Cargo can be used to specify a minimum age for published versions to use.

When set, it contains a duration specified as an integer followed by a unit of "seconds", "minutes", "days", or "weeks".
If a new crate would be added with a command such as `cargo add` or `cargo update`, it will use a version with a publish
time ("pubtime") before that is older than that duration, if possible. `cargo` may print a message in such a case.

For example with

```toml
[registry]
global-min-publish-age = "7 days"
```

running a command like `cargo update`, `cargo add`, `cargo build`, etc. will prefer to use versions of required crates that were published
at least 7 days ago.

The time can be indicated as an integer followed by a time unit such as minutes, hours, days, etc.

Crates that use path or git, rather than a registry will never trigger this check, as there isn't a relevant publish time to use. Also,
this check won't be preformed for crates published on registries that don't publish the `pubtime` information (note that crates.io does
include `pubtime`).

The `resolver.incompatible-publish-age` configuration can also be used to control how `cargo` handles versions whose
publish time is newer than the min-publish-age. By default, it will try to use an older version, unless none is available
that also complies with the specified version constraint, or the `rust-version`. However by setting this to "allow"
it is possible to disable the min-publish-age checking.

If it isn't possible to satisfy a dependency with a version that meets the minimum release age requirement and
`resolver.incompatible-publish-age` is set to "fallback", then Cargo will
fall back to using the best version that matches. In this cases, a warning will be printed next to the message for adding the
crate, similar to the warning for an incompatible rust version. It looks like:

```
Adding example v1.2.3 (published less than 2 days ago on 2026-03-07)
```

Most likely, `resolver.incompatible-publish-age` will usually be left at its default of `fallback`, however it may occasionally
be desirable to use it to temporarily turn off the minimum age check, especially if there are configurations for multiple
registries. This would typically be done with a command line argument like `--config 'resolver.incompatible-publish-age="allow"'` or an
environment variable like `CARGO_RESOLVER_INCOMPATIBLE_PUBLISH_AGE=allow`.

It is also possible to configure the `min-publish-age` per cargo registry. `registries.<name>.min-publish-age` sets
the minimum publish age for the `<name>` registry. And `registry.min-publish-age` sets it for the default registry
crates.io registry.

For example:
```toml
[registries.example]
index = "https://crates.example.com"
min-publish-age = "2 hours"

[registry.local]
index = "https://registry.local"
min-publish-age = 0 # this registry is fully trusted

[registry]
# Default for any registry without a specific value
global-min-publish-age = "2 days"
# Value to use for crates.io
min-publish-age = "5 days"
```

This will use a minimum publish age of 5 days for crates.io, 2 hours for crates.exalmple.com, no minimum for registry.local, and 2 days for any other registry.

### Using newer version

In some cases, it may be desirable to use a version that is newer than the minimum publish age. For example, because a new
version has a critical security fix, or because it is part of the same family of crates as the dependent crate, and they should
be released together.

If `resolver.incompatible-publish-age` is "fallback" (the default), it is possible to bypass the check by updating the version range to require
the newer version in `Cargo.toml`, or with `cargo add`, or specify the exact version to use with `cargo update --precise`.

In the future, additional controls may be provided (see [Future Possibilities](#future-possibilities)).

[1]: https://doc.rust-lang.org/cargo/reference/config.html
[^1]: As specified in `.cargo/config.toml` files

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This RFC adds a few new configuration options to [cargo configuration](https://doc.rust-lang.org/cargo/reference/config.html).

### Added to [Configuration Format](https://doc.rust-lang.org/cargo/reference/config.html#configuration-format)

```toml
 [resolver]
 incompatible-publish-age = "fallback" # Specifies how resolver reacts to these

 [registries.<name>]
 min-publish-age = "..."  # Override `registry.global-min-publish-age` for this registry

 [registry]
 min-publish-age = "0"  # Override `registry.global-min-publish-age` for crates.io
 global-min-publish-age = "0"  # Minimum time span allowed for packages from this registry
 ```

### Added to [`[resolver]`](https://doc.rust-lang.org/cargo/reference/config.html#resolver)
#### `resolver.incompatible-publish-age`

* Type: String
* Default: `"fallback"`
* Environment: `CARGO_RESOLVER_INCOMPATIBLE_PUBLISH_AGE`


When resolving the version of a dependency to use, specify the behavior for versions with a `pubtime` (if present) that is incompatible with `registry.min-publish-age`. Values include:

* `allow`: treat pubtime-incompatible versions like any other version
* `fallback`: only consider pubtime-incompatible versions if no other version matched


If the value is `fallback`, then cargo will print a warning if no suitable version can be found and the resolver is forced to select a version that is newer
than allowed by the appropriate `min-publish-age` setting.


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

* `min-publish-age` only apply to dependencies fetched from a registry that publishes `pubtime`, such as crates.io. They do not apply to git or path dependencies, in
  part because there is not always an obvious publish time, or a way to find alternative versions.
  They do not apply to registries that don't set `pubtime`, as there is no reliable way to know when the version
  was published.
* At this time, if a specific version is explicitly specified in Cargo.toml, or on the command line, that has higher precedence than the publish time check,
  and will be assumed to be valid. In the future it may be possible to change this behavior.
* `cargo add`
    * If a version is not explicitly specified by the user and the package is fetched from a registry (not a path or git), `min-publish-age` options
      will be respected.
* `cargo install`
    * If a specific version is not specified by the user, respect `registries.min-publish-age` for the version of the crate itself,
      as well as transitive dependencies when possible.
* `cargo update`
    * Unless `--precise` is used to specify a specific version, any crates updated from the registry will only consider versions published
      before the time specified by the appropriate `min-publish-age` option. If `--precise` is used, that version will be used, even if it
      newer than the policy would otherwise allow (although in the future, there may be an option to deny that).
    * If the version of a crate in the lockfile is already newer than `min-publish-age`, then `cargo update` will not update that crate, nor will
      it downgrade to an older version. It will leave the version as it is.
* When a lockfile is generated, as with `cargo generate-lockfile` or other commands such as `cargo build` that can do so, then versions will be
  selected that comply with the `min-publish-age` policy, if possible.
* If the only version of a crate that satisfies the `min-publish-age` constraint is a yanked version, it will behave as if no versions satisfied the
  `min-publish-age` constraint. In other words, yanked versions has higher priority than the `min-publish-age` configuration.

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

### Why not leave this to third party tools?

There are already some third party tools that fulfill this functionality to some degree. For example, dependabot and renovate can
be used for updating Cargo.toml and Cargo.lock, and both support some form of minimum publish age. And the cargo-cooldown project provides
an alternative to `cargo update` that respects a minimum publish age.

However, these tools only work for updating and adding dependencies outside of cargo itself, they do not
have any impact on explicitly run built-in cargo commands such as `cargo update` and `cargo add`.
Having built-in support makes it easier to enforce a minimum publish age policy.

Furthermore, these tools depend on the existence of a `Cargo.lock` file to lock the versions. Or having
strict version constraints in `Cargo.toml`. If a `Cargo.lock` file does not yet exist, commands such as `cargo build` won't
be protected.

### Using Cargo.toml and Cargo.lock

You can pin versions in your `Cargo.toml` but that is a manual process and doesn't cover transitive
dependencies.

`Cargo.lock` records versions but those are at the time of last change.
Adding a new dependency can cause you to pull in transitive dependencies that are outside
your desired minimum age. There isn't a manageable way to run `cargo update` and intentionally
get versions that are inside of your desired minimum age.

### Configuration Locations and Names

The locations and names of the configuration options in this proposal were chosen to be
consistent with existing Cargo options, as described in [Related Options in Cargo](#related-options).

The term "publish" was used rather than "package", "version", or "release" to make it
clear that this only applies to crates that are published in a registry.

### fallback and deny

We default `resolver.incompatible-publish-age` to "fallback" rather than deny
and defer support for "deny" to future possibilities, because that allows user to allow
users to easily override the minimum age for specific crates when necessary.

Specifically, with "fallback" it is possible to override the minimum age behavior for
specific crates by specifying a more specific version in Cargo.toml, or using `cargo update --precise`.

Furthermore, with "fallback", and the ability to override versions as mentioned above,
it isn't as critical to have a way to list crates to exclude from the rule in configuration.

We anticipate that "fallback" will be sufficient for most use cases, but the possibility of a  "deny" option
can be revisited if necessary.

### Per-registry configuration

Allowing the minimum age to be configurable per registry provides a simple mechanism
to use different minimum ages for different sets of packages, including possibly no
minimum in common situations such as using an internal registry where the crates
are completely trusted.

This makes it less necessary to have more complicated configuration for rules for including
and excluding sets of packages from the age policy, or setting different age policies
for different packages.


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

I think it is possible to create separate rules with different `minimumReleaseAge` configurations.

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

     * "never" — Never deletes old files.

     * "always" — Checks to delete old files every time Cargo runs.

     * An integer followed by “seconds”, “minutes”, “hours”, “days”, “weeks”, or “months”


 [`resolver.incompatible-rust-versions`](https://doc.rust-lang.org/cargo/reference/config.html#resolverincompatible-rust-versions)

     * Controls behavior in relation to your [`package.rust-version`](https://doc.rust-lang.org/cargo/reference/rust-version.html) and those set by potential dependendencies

     * Values:

       * allow: treat rust-version-incompatible versions like any other version
       * fallback: only consider rust-version-incompatible versions if no other version matched


 [`package.resolver`](https://doc.rust-lang.org/cargo/reference/resolver.html#resolver-versions) is only a version number. When adding `incompatible-rust-version`, we intentionally deferred anything being done in manifests.

 [`[registry]`](https://doc.rust-lang.org/cargo/reference/config.html#registry)

     * Set default registry

     * Sets credential providers for all registries

     * Sets crates.io values


 [`[registries]`](https://doc.rust-lang.org/cargo/reference/config.html#registries)

     * Sets registry specific values


 `yanked`: can't do new resolves to it but left in if already there. Unstable support to force it with `--precise` but that doesn't apply recursively.

 pre-release: requires opt-in through version requirement. Unstable support to force it with `--precise` but that doesn't apply recursively.

 We use the term `publish` and not `release`


## Unresolved Questions
[unresolved-questions]: #unresolved-questions

* Would it be better to have `registry.min-publish-age` be the global setting, and `registries.crates-io.min-publish-age` be the setting for the crates.io registry?
  The current proposal is based on precedent of "credential-provider" and "global-credential-provider", but perhaps we shouldn't follow that precedent?
* How do we make it clear when things are held back?
    * The "locking" message for [Cargo time machine (generate lock files based on old registry state) #5221](https://github.com/rust-lang/cargo/issues/5221) lists one time but the time here is dependent on where any given package is from
    * We list MSRVs for unselected packages, should we also list publish times? I'm assuming that should be in local time
    * Locking message for [Cargo time machine (generate lock files based on old registry state) #5221](https://github.com/rust-lang/cargo/issues/5221) is in UTC time, see [Tracking Issue for _lockfile-publish-time_ #16271](https://github.com/rust-lang/cargo/issues/16271), when relative time differences likely make local time more relevant
* Implementation wise, will there be much complexity in getting per registry information into `VersionPreferences` and using it?
* `fallback` precedence between this and `incompatible-rust-version`?
    * Most likely, `incompatible-rust-version` should have higher precedence to increase the chance of builds succeeding.
* Can we, and should we make any guarantees about security when using this feature, such as "a release of a malicious version of a crate will not compromise the build

## Future Possibilities
[future-possibilities]: #future-possibilities

- Support "deny" for `resolver.incompatible-publish-age`.
    - This is initially excluded, because it isn't clear how this should behave with respect to versions already in Cargo.lock, or use with the `--precise` flag.
    - What would an error look like?
    - How would you be able to override this for specific crates for important security updates, or for related crates that should be released at the same time?
- Add a way to specify that the minimum age doesn't apply to certain packages. For example, by having an array of crates that should always use the newest version.
    - I excluded this from the initial RFC, because implementing it adds significant complexity to the proposal, and it is relatively easy to work around by explicitly updating
      those packages to newer versions in Cargo.toml and/or Cargo.lock.
    - This may be more important if support for "deny" is added to `resolver.incompatible-publish-age`.
- Potentially support other source of publish time besides the `pubtime` field from a cargo registry.
- Provide a mechanism to compare the publish time against a time other than the current system time. For example, comparing to the time of some snapshot, or the timestamp
  of a local cache.
- Allow specifying a timestamp for the `min-publish-age`.
