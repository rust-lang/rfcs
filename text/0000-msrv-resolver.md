- Feature Name: `msrv-resolver`
- Start Date: 2023-11-14
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Provide a happy path for developers needing to work with older versions of Rust by
- Preferring MSRV (minimum-supported-rust-version) compatible dependencies when Cargo resolves dependencies
- Ensuring compatible version requirements when `cargo add` auto-selects a version

Note: `cargo install` is intentionally left out for now to decouple discussions on how to handle the security ramifications.

# Motivation
[motivation]: #motivation

Let's step through a simple scenario where a developer can develop with the
latest Rust version but production uses an older version:
```console
$ cargo new msrv-resolver
     Created binary (application) `msrv-resolver` package
$ cd msrv-resolver
$ # ... add `package.rust-version = "1.64.0"` to `Cargo.toml`
$ cargo add clap
    Updating crates.io index
      Adding clap v4.4.8 to dependencies.
             Features:
...
    Updating crates.io index
$ git commit -a -m "WIP" && git push
...
```
After 30 minutes, CI fails.
The first step is to reproduce this locally
```console
$ rustup install 1.64.0
...
$ cargo +1.64.0 check
    Updating crates.io index
       Fetch [===============>         ]  67.08%, (28094/50225) resolving deltas
```
After waiting several minutes, cursing being stuck on a version from before sparse registry support was added...
```console
$ cargo +1.64.0 check
    Updating crates.io index
  Downloaded clap v4.4.8
  Downloaded clap_builder v4.4.8
  Downloaded clap_lex v0.6.0
  Downloaded anstyle-parse v0.2.2
  Downloaded anstyle v1.0.4
  Downloaded anstream v0.6.4
  Downloaded 6 crates (289.3 KB) in 0.35s
error: package `clap_builder v4.4.8` cannot be built because it requires rustc 1.70.0 or newer, while the currently ac
tive rustc version is 1.64.0
```
Thankfully, crates.io now shows [supported Rust versions](https://crates.io/crates/clap_builder/versions), so I pick v4.3.24.
```console
$ cargo update -p clap_builder --precise 4.3.24
    Updating crates.io index
error: failed to select a version for the requirement `clap_builder = "=4.4.8"`
candidate versions found which didn't match: 4.3.24
location searched: crates.io index
required by package `clap v4.4.8`
    ... which satisfies dependency `clap = "^4.4.8"` (locked to 4.4.8) of package `msrv-resolver v0.1.0 (/home/epage/src/personal/dump/msrv-resolver)`
perhaps a crate was updated and forgotten to be re-vendored?
```
After browsing on some forums, I edit my `Cargo.toml` to roll back to `clap = "4.3.24"` and try again
```console
$ cargo update -p clap --precise 4.3.24
    Updating crates.io index
 Downgrading anstream v0.6.4 -> v0.3.2
 Downgrading anstyle-wincon v3.0.1 -> v1.0.2
      Adding bitflags v2.4.1
 Downgrading clap v4.4.8 -> v4.3.24
 Downgrading clap_builder v4.4.8 -> v4.3.24
 Downgrading clap_lex v0.6.0 -> v0.5.1
      Adding errno v0.3.6
      Adding hermit-abi v0.3.3
      Adding is-terminal v0.4.9
      Adding libc v0.2.150
      Adding linux-raw-sys v0.4.11
      Adding rustix v0.38.23
$ cargo +1.64.0 check
  Downloaded clap_builder v4.3.24
  Downloaded errno v0.3.6
  Downloaded clap_lex v0.5.1
  Downloaded bitflags v2.4.1
  Downloaded clap v4.3.24
  Downloaded rustix v0.38.23
  Downloaded libc v0.2.150
  Downloaded linux-raw-sys v0.4.11
  Downloaded 8 crates (2.8 MB) in 1.15s (largest was `linux-raw-sys` at 1.4 MB)
error: package `anstyle-parse v0.2.2` cannot be built because it requires rustc 1.70.0 or newer, while the currently a
ctive rustc version is 1.64.0
```
Again, consulting [crates.io](https://crates.io/crates/anstyle-parse/versions)
```console
$ cargo update -p anstyle-parse --precise 0.2.1
    Updating crates.io index
 Downgrading anstyle-parse v0.2.2 -> v0.2.1
$ cargo +1.64.0 check
error: package `clap_lex v0.5.1` cannot be built because it requires rustc 1.70.0 or newer, while the currently active
 rustc version is 1.64.0
```
Again, consulting [crates.io](https://crates.io/crates/clap_lex/versions)
```console
$ cargo update -p clap_lex --precise 0.5.0
    Updating crates.io index
 Downgrading clap_lex v0.5.1 -> v0.5.0
$ cargo +1.64.0 check
error: package `anstyle v1.0.4` cannot be built because it requires rustc 1.70.0 or newer, while the currently active
rustc version is 1.64.0
```
Again, consulting [crates.io](https://crates.io/crates/anstyle/versions)
```console
cargo update -p anstyle --precise 1.0.2
    Updating crates.io index
 Downgrading anstyle v1.0.4 -> v1.0.2
$ cargo +1.64.0 check
  Downloaded anstyle v1.0.2
  Downloaded 1 crate (14.0 KB) in 0.60s
   Compiling rustix v0.38.23
    Checking bitflags v2.4.1
    Checking linux-raw-sys v0.4.11
    Checking utf8parse v0.2.1
    Checking anstyle v1.0.2
    Checking colorchoice v1.0.0
    Checking anstyle-query v1.0.0
    Checking clap_lex v0.5.0
    Checking strsim v0.10.0
    Checking anstyle-parse v0.2.1
    Checking is-terminal v0.4.9
    Checking anstream v0.3.2
    Checking clap_builder v4.3.24
    Checking clap v4.3.24
    Checking msrv-resolver v0.1.0 (/home/epage/src/personal/dump/msrv-resolver)
    Finished dev [unoptimized + debuginfo] target(s) in 2.96s
```
Success! Mixed with many tears and less hair.

How wide spread is this?  Take this with a grain of salt but based on crates.io user agents:

| Common MSRVs        | % Compatible Requests |
|--------------------:|:----------------------|
| N (`1.73.0`)        | 47.432%               |
| N-2 (`1.71.0`)      | 74.003%               |
| ~6 mo (`1.69.0`)    | 93.272%               |
| ~1 year (`1.65.0`)  | 98.766%               |
| Debian (`1.63.0`)   | 99.106%               |
| ~2 years (`1.56.0`) | 99.949%               |

*([source](https://rust-lang.zulipchat.com/#narrow/stream/318791-t-crates-io/topic/cargo.20version.20usage/near/401440149))*

People have tried to reduce the pain from MSRV with its own costs:
- Treating it as a breaking change:
  - This leads to extra churn in the ecosystem when a fraction of users are likely going to benefit
  - We have the precedence elsewhere in the Rust ecosystem for build and runtime system requirement changes not being breaking, like when rustc requires new glibc, AndroiNDK, etc
- Adding upper limits to version requirements:
  - This fractures the ecosystem by making packages incompatible with each other and the Cargo team [discourages doing this](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#multiple-requirements)

Another way the status quo exhibits pain on the ecosystem is long
arguments over what is the right policy for updating a minimum-supported Rust
version (MSRV), wearing on all parties.
For example:
- [libc](https://github.com/rust-lang/libs-team/issues/72)
- [time](https://github.com/time-rs/time/discussions/535)

Supporting older MSRVs means maintainers don't have access to all of the latest
resources for improving their project.
This indirectly affects users as it can slow maintainers down.
This can also directly affect users.
For example,
by clap updating its MSRV from 1.64.0 to 1.70.0,
it was able to drop the large [is-terminal](https://crates.io/crates/is-terminal) dependency,
[cutting the build time from 6s to 3s](https://github.com/rosetta-rs/argparse-rosetta-rs/commit/378cd2c30679afdf9b9843dbadea3e8951090809).
So if we can find a solution that allows maintainers to move forward, helping
users more on the edge, while not impacting users on older rust version, would be
a big help.

The sooner we improve the status quo, the better, as it can take years for
these changes to percolate out to those exclusively developing with an older
Rust version (in contrast with the example above).
This delay can be reduced somewhat if a newer development version can be used
without upgrading the MSRV.

In solving this, we need to keep in mind
- Users need to be aware when they are on old versions for evaluating security risk and when debugging issues
- We don't want to end up like other ecosystems where no one can use new features
  because users are stuck on 3-20 year old versions of the language specification.
  The compatibility story is fairly strong with Rust, helping us keep
  compiler and dependency upgrades cheap.
- Some people keep their development and production MSRVs the same while others keep them separate, like with a `Cargo.msrv.lock`
- A `Cargo.lock` should not resolve differently when upgrading Rust without any other action.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## The `rust-version` field

*(update to [manifest documentation](https://doc.rust-lang.org/cargo/reference/manifest.html#the-rust-version-field))*

The `rust-version` field is an optional key that tells cargo what version of the
Rust language and compiler your package can be compiled with. If the currently
selected version of the Rust compiler is older than the stated version, cargo
will exit with an error, telling the user what version is required.
To support this, Cargo will prefer dependencies that are compatible with your `rust-version`.

The first version of Cargo that supports this field was released with Rust 1.56.0.
In older releases, the field will be ignored, and Cargo will display a warning.

```toml
[package]
# ...
rust-version = "1.56"
```

The Rust version must be a bare version number with two or three components; it
cannot include semver operators or pre-release identifiers. Compiler pre-release
identifiers such as -nightly will be ignored while checking the Rust version.
The `rust-version` must be equal to or newer than the version that first
introduced the configured `edition`.

The `rust-version` may be ignored using the `--ignore-rust-version` option.

Setting the `rust-version` key in `[package]` will affect all targets/crates in
the package, including test suites, benchmarks, binaries, examples, etc.

## Rust Version

*(update to [Dependency Resolution's Other Constraints documentation](https://doc.rust-lang.org/cargo/reference/resolver.html))*

When multiple versions of a dependency satisfy all version requirements,
cargo will prefer those with a compatible `package.rust-version` over those that
aren't compatible.
Some details may change over time though `cargo check && rustup update && cargo check` should not cause `Cargo.lock` to change.

#### `build.resolver.precedence`

*(update to [Configuration](https://doc.rust-lang.org/cargo/reference/config.html))*

* Type: string
* Default: "rust-version=package"
* Environment: `CARGO_BUILD_RESOLVER_PRECEDENCE`

Controls how `Cargo.lock` gets updated on changes to `Cargo.toml` and with `cargo update`.  This does not affect `cargo install`.

* `maximum`: Prefer the highest compatible versions of dependencies
* `minimum`: Prefer the lowest versions of dependencies
* `rust-version=package`: Prefer dependencies where their `rust-version` is compatible with `package.rust-version`
* `rust-version=rustc`: Prefer dependencies where their `rust-version` is compatible with `rustc --version`
* `rust-version=<X>[.<Y>[.<Z>]]`: Prefer dependencies where their `rust-version` is compatible with the specified version

`rust-version=` values can be overridden with `--ignore-rust-version` which will fallback to `maximum`.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Cargo Resolver

Cargo's resolver will be updated to *prefer* MSRV compatible versions over
incompatible versions when resolving versions.
Dependencies without `package.rust-version` will be preferred over those without an MSRV but less than those with one.
The exact details for how preferences are determined may change over time but,
since the currently resolved dependencies always get preference,
this shouldn't affect existing `Cargo.lock` files.

This can be overridden with `--ignore-rust-version` and config's `build.resolver.precedence`.

Implications
- If you use do `cargo update --precise <msrv-incompatible-ver>`, it will work
- If you use `--ignore-rust-version` once, you don't need to specify it again to keep those dependencies though you might need it again on the next edit of `Cargo.toml` or `cargo update` run
- If a dependency doesn't specify `package.rust-version` but its transitive dependencies specify an incompatible `package.rust-version`,
  we won't backtrack to older versions of the dependency to find one with a MSRV-compatible transitive dependency.
- A package with multiple MSRVs, depending on the features selected, can still do this as version requirements can still require versions newer than the MSRV and `Cargo.lock` can depend on those as well.

As there is no `workspace.rust-version`,
the resolver will pick the lowest version among workspace members.
This will be less optimal for workspaces with multiple MSRVs and dependencies unique to the higher-MSRV packages.
Users can workaround this by raising the version requirement or using `cargo update --precise`.

If `package.rust-version` is unset among all workspace members,
we'll fallback to `rustc --version`,
ensuring a build that at least works for the current system.
As this is just a preference for resolving dependencies, rather than prescriptive,
this shouldn't cause churn.
We already call `rustc` for feature resolution, so hopefully this won't have a performance impact.

The resolver will only do this for local packages and not for `cargo install`.

## `cargo update`

`cargo update` will inform users when an MSRV or semver incompatible version is available.
`cargo update -n` will also report this information so that users can check on the status of this at any time.

**Note:** other operations that cause `Cargo.lock` entries to be changed (like
editing `Cargo.toml` and running `cargo check`) will not inform the user.
If they want to check the status of things, they can run `cargo update -n`.

## `cargo add`

`cargo add <pkg>` (no version) will pick a version requirement that is low
enough so that when it resolves, it will pick a dependency that is
MSRV-compatible.
`cargo add` will warn when it does this.

This behavior can be bypassed with `--ignore-rust-version`

## Cargo config

We'll add a `build.resolver.precedence ` field to `.cargo/config.toml` that will control the control pick the mechanisms for preferring one compatible version over another.

```toml
[build]
resolver.precedence = "rust-version=package"  # Default
```
with support values being:
- `maximum`: behavior today
  - Needed for [verifying latest dependencies](https://doc.rust-lang.org/nightly/cargo/guide/continuous-integration.html#verifying-latest-dependencies)
- `minimum` (unstable): `-Zminimal-versions`
  - As this just just precedence, `-Zdirect-minimal-versions` doesn't fit into this
- `rust-version=` (assumes `maximum` is the fallback)
  - `package`: what is defined in the package (default)
  - `rustc`: the current running version
    - Needed for "separate development / publish MSRV" workflow
  - `<x>[.<y>[.<z>]]` (future possibility): manually override the version used

If a `rust-version=` value is used, we'd switch to `maximum` when `--ignore-rust-version` is set.
This will let users effectively pass `--ignore-rust-version` to all commands,
without having to support the flag on every single command.

# Drawbacks
[drawbacks]: #drawbacks

Maintainers that commit their `Cargo.lock` and verify their latest dependencies
will need to set `CARGO_BUILD_RESOLVER_PRECEDENCE=rust-version=rustc` in their environment.
See Alternatives for more on this.

While we hope this will give maintainers more freedom to upgrade their MSRV,
this could instead further entrench rust-version stagnation in the ecosystem.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Misc
- Config was put under `build` to associate it with local development, as compared with `install` which could be supported in the future
- Dependencies with unspecified `package.rust-version`: we could mark these as always-compatible or always-incompatible; there really isn't a right answer here.
- The resolver doesn't support backtracking as that is extra complexity that we can always adopt later as we've reserved the right to make adjustments to what `cargo generate-lockfile` will produce over time.
- `CARGO_BUILD_RESOLVER_PRECEDENCE=rust-version=*` assumes maximal resolution as generally minimal resolution will pick packages with compatible rust-versions as rust-version tends to (but doesn't always) increase over time.
  - `cargo add` selecting rust-version-compatible minimum bounds helps
  - This bypasses a lot of complexity either from exploding the number of states we support or giving users control over the fallback by making the field an array of strategies.

## Add `workspace.rust-version`

Instead of using the lowest MSRV among workspace members, we could add `workspace.rust-version`.

This opens its own set of questions
- Do packages implicitly inherit this?
- What are the semantics if its unset?
- Would it be confusing to have this be set in mixed-MSRV workspaces?  Would blocking it be incompatible with the semantics when unset?
- In mixed-MSRV workspaces, does it need to be the highest or lowest MSRV of your packages?
  - For the resolver, it would need to be the lowest but there might be other use cases where it needs to be the highest

The proposed solution does not block us from later going down this road but
allows us to move forward without having to figure out all of these details.

## Make this opt-in

As proposed, CI that tries to verify against the latest dependencies will no longer do so.
Instead, they'll have to make a change to their CI, like setting `CARGO_BUILD_RESOLVER_PRECEDENCE=maximum`.

If we consider this a major incompatibility, then it needs to be opted into.
As `cargo fix` can't migrate a user's CI,
this would be out of scope for migrating to this with a new Edition.

I would argue that the number of maintainers verifying latest dependencies is
relatively low and they are more likely to be "in the know",
making them less likely to be negatively affected by this.
Therefore, I propose we consider this a minor incompatibility

If we do a slow roll out (opt-in then opt-out), the visibility for the switch
to opt-out will be a lot less than the initial announcement and we're more
likely to miss people compared to making switch over when this gets released.

If we change behavior with a new edition (assuming we treat this as a minor incompatibility),
we get the fanfare needed but it requires waiting until people bump their MSRV,
making it so the people who need it the most are the those who will least benefit.

## Make `rust-version=rustc` the default

This proposal elevates "shared development / publish rust-version" workflow over "separate development and publish rust-version" workflow.
We could instead do the opposite, picking `rust-version=rustc` as a "safe" default for assuming the development rust-version.
Users of the "shared development / publish rust-version" workflow could either set the config or use a `rust-toolchain.toml` file.

The reasons we didnn't go with this approach are
- The user explicitly told us the MSRV for the project; we do not have the granularity for different MSRVs for different workflows (or `features`) and likely the complexity would not be worth it.
- Split MSRV workflows are inherently more complex to support with more caveats of where they apply, making single MSRV workflows the path of least resistance for users.
- Without configuration, defaulting to single MSRV workflows will lead to the least number of errors from cargo as the resulting lockfile is compatible with the split MSRV workflows.
- Single MSRV workflows catch too-new API problems sooner
- We want to encourage developing on the latest version of rustc/cargo to get all of the latest workflow improvements (e.g. error messages, sparse registry for cargo, etc), rather than lock people into the MSRV with `rust-toolchain.toml`
  - The toolchain is another type of dependency so this might seem contradictory but we feel the value-add of a new toolchain outweighs the cost while the value add of new dependencies doesn't

## Configuring the resolver mode on the command-line or `Cargo.toml`

The Cargo team is very interested in [moving project-specific config to manifests](https://github.com/rust-lang/cargo/issues/12738).
However, there is a lot more to define for us to get there.  Some routes that need further exploration include:
- If its a CLI flag, then its transient, and its unclear which modes should be transient now and in the future
  - We could make it sticky by tracking this in `Cargo.lock` but that becomes less obvious what resolver mode you are in and how to change
- We could put this in `Cargo.toml` but that implies it unconditionally applies to everything
  - But we want `cargo install` to use the latest dependencies so people get bug/security fixes
  - This gets in the way of the split MSRV workflow

By relying on config we can have a stabilized solution sooner and we can work out more of the details as we better understand the relevant problems.

## Hard-error

Instead of *preferring* MSRV-compatible dependencies, the resolver could hard error if only MSRV-incompatible versions are available.
This means that we would also backtrack on transitive dependencies, trying alternative versions of direct dependencies, which would create an MSRV-compatible `Cargo.lock` in more cases.

Nothing in this solution changes our ability to do this later.

However, blocking progress on this approach would greatly delay stabilization of this because of bad error messages.
This was supported in 1.74 and 1.75 nightlies under `-Zmsrv-policy` and the biggest problem was in error reporting.
The resolver acted as if the MSRV-incompatible versions don't exist so if there was no solution, the error message was confusing:
```console
$ cargo +nightly update -Z msrv-policy
    Updating crates.io index
error: failed to select a version for the requirement `hashbrown = "^0.14"`
candidate versions found which didn't match: 0.14.2, 0.14.1, 0.14.0, ...
location searched: crates.io index
required by package `app v0.1.0 (/app)`
perhaps a crate was updated and forgotten to be re-vendored?
```

It would also be a breaking change to hard-error.
We'd need to provide a way for some people to opt-in while some people opt-out and remember that.
We could add a sticky flag to `Cargo.lock` though that could also be confusing, see "Configuring the resolver mode on the command-line or `Cargo.toml`".

This would also error or pick lower versions more than it needs to when a workspace contains multiple MSRVs.
We'd want to extend the resolver to treat Rust as yet another dependency and turn `package.rust-version` into dependencies on Rust.
This could cause a lot more backtracking which could negatively affect resolver performance for people with lower MSRVs.

If no `package.rust-version` is specified,
we wouldn't want to fallback to the version of rustc being used because that could cause `Cargo.lock` churn if contributors are on different Rust versions.

# Prior art
[prior-art]: #prior-art

- Python: instead of tying packages to a particular tooling version, the community instead focuses on their equivalent of the [`rustversion` crate](https://crates.io/crates/rustversion) combined with tool-version-conditional dependencies that allow polyfills.
  - We have [cfg_accessible](https://github.com/rust-lang/rust/issues/64797) as a first step though it has been stalled
  - These don't have to be mutually exclusive solutions as conditional compilation offers flexibility at the cost of maintenance.  Different maintainers might make different decisions in how much they leverage each
  - One big difference is Python continues to support previous releases which sets a standard within the community for "MSRV" policies.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

The config field is fairly rought
  - The location (within `build`) needs more consideration
  - The name isn't very clear
  - The values are awkward

# Future possibilities
[future-possibilities]: #future-possibilities

## Improve the experience with lack of `rust-version`

The user experience for this is based on the extent and quality of the data.
Ensuring we have `package.rust-version` populated more often (while maintaining
quality of that data) is an important problem but does not have to be solved to
get value out of this RFC and can be handled separately.

~~We could encourage people to set their MSRV by having `cargo new` default `package.rust-version`.~~
However, if people aren't committed to verifying it,
it is likely to go stale and will claim an MSRV much older than what is used in practice.
If we had the hard-error resolver mode and
[clippy warning people when using API items stabilized after their MSRV](https://github.com/rust-lang/rust-clippy/issues/6324),
this will at least annoy people into either being somewhat compatible or removing the field.

~~When missing, `cargo publish` could inject `package.rust-version` using the version of rustc used during publish.~~
However, this will err on the side of a higher MSRV than necessary and the only way to
workaround it is to set `CARGO_BUILD_RESOLVER_PRECEDENCE=maximum` which will then lose
all other protections.

~~When missing, `cargo publish` could inject based on the rustup toolchain file.~~
However, this will err on the side of a higher MSRV than necessary as well.

~~When missing, `cargo publish` could inject `package.rust-version` inferred from
`package.edition` and/or other `Cargo.toml` fields.~~
However, this will err on the side of too low of an MSRV.
While this might help with in this situation,
it would lock us in to inaccurate information which might limit what analysis we could do in the future.

Alternatively, `cargo publish` / the registry could add new fields to the Index
to represent an inferred MSRV, the published version, etc
so it can inform our decisions without losing the intent of the publisher.

On the resolver side, we could
- Assume the MSRV of the next published package with an MSRV set
- Sort no-MSRV versions by minimal versions, the lower the version the more likely it is to be compatible
  - This runs into quality issues with version requirements that are likely too low for what the package actually needs
  - For dependencies that never set their MSRV, this effectively switches us from maximal versions to minimal versions.

## Integrate `cargo audit`

If we [integrate `cargo audit`](https://github.com/rust-lang/cargo/issues/7678),
we can better help users on older dependencies identify security vulnerabilities.

## "cargo upgrade"

As we pull [`cargo upgrade` into cargo](https://github.com/rust-lang/cargo/issues/12425),
we'll want to make it respect MSRV as well

## cargo install

`cargo install` could auto-select a top-level package that is compatible with the version of rustc that will be used to build it.
This could be controlled through a config field `install.resolver.precedence`,
mirroring `build.resolver.precedence`.

See [rust-lang/cargo#10903](https://github.com/rust-lang/cargo/issues/10903) for more discussion.

**Note:** [rust-lang/cago#12798](https://github.com/rust-lang/cargo/pull/12798)
(slated to be released in 1.75) made it so `cargo install` will error upfront,
suggesting a version of the package to use and to pass `--locked` assuming the
bundled `Cargo.lock` has MSRV compatible dependencies.

## `build.rust-version = "<x>.<y>"`

We could allow people setting an effective rust-version within the config.
This would be useful for people who have a reason to not set `package.rust-version`
as well as to reproduce behavior with different Rust versions.

## rustup supporting `+msrv`

See https://github.com/rust-lang/rustup/issues/1484#issuecomment-1494058857

## Language-version lints

We could make developing with the latest toolchain with old MSRVs easier if we provided lints.
Due to accuracy of information, this might start as a clippy lint, see
[#6324](https://github.com/rust-lang/rust-clippy/issues/6324).
This doesn't have to be perfect (covering all facets of the lanuage) to be useful in helping developers identify their change is MSRV incompatible as early as possible.

If we allowed this to bypass caplints,
then you could more easily track when a dependency with an unspecified MSRV is incompatible.

## Language-version awareness for rust-analyzer

rust-analyzer could mark auto-complete options as being incompatible with the MSRV and
automatically bump the MSRV if selected, much like auto-adding a `use` statement.
