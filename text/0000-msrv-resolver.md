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
users more the edge, while not impacting users on older rust version, would be
a big help.

The sooner we improve the status quo, the better, as it can take years for
these changes to percolate out to those exclusively developing with an older
Rust version (in contrast with the example above).

In solving this, we need to keep in mind
- Users need to be aware when they are on old versions for evaluating security risk and when debugging issues
- We don't want to end up like other ecosystems where no one can use new features
  because users are stuck on 3-20 year old versions of the language specification.
  The compatibility story is fairly strong with Rust, helping us keep
  compiler and dependency upgrades cheap.
- We also want to continue to support people whose workflow is to develop with
  latest dependencies in a `Cargo.lock` and then verify MSRV with a carefully
  crafted `Cargo.lock`.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Cargo Resolver

Cargo's resolver will be updated to *prefer* MSRV compatible versions over
incompatible versions when resolving versions.
Packages without `package.rust-version` will be treated as compatible.
This can be overridden with `--ignore-rust-version`.

Implications
- If you use do `cargo update --precise <msrv-incompatible-ver>`, it will work
- If you use `--ignore-rust-version` once, you don't need to specify it again to keep those dependencies
- If a dependency doesn't specify `package.rust-version` but its transitive dependencies specify an incompatible `package.rust-version`,
  we won't backtrack to older versions of the dependency to find one with a MSRV-compatible transitive dependency.

As there is no `workspace.rust-version`,
the resolver will pick the lowest version among workspace members.
This will be less optimal for workspaces with multiple MSRVs and dependencies unique to the higher-MSRV packages.
Users can workaround this by raising the version requirement or using `cargo update --precise`.

The resolver will only do this for local packages and not for `cargo install`.

## `cargo update`

`cargo update` will inform users when an MSRV or semver incompatible version is available.
`cargo update -n` will also report this information so that users can check on the status of this at any time.

**Note:** other operations that cause `Cargo.lock` entries to be changed (like
editing `Cargo.toml` and running `cargo check`) will not inform the user.

## `cargo add`

`cargo add <pkg>` (no version) will pick a version requirement that is low
enough so that when it resolves, it will pick a dependency that is
MSRV-compatible.
`cargo add` will warn when it does this.

This behavior can be bypassed with `--ignore-rust-version`

## Cargo config

We'll add a `build.rust-version = <true|false>` field to `.cargo/config.toml` that will control whether `package.rust-version` is respected or not.
`--ignore-rust-version` can override this.

This will let users effectively pass `--ignore-rust-version` to all commands,
without having to support it on every single one.

We can also stabilize this earlier than the rest of this so we can use it in our
[Verifying latest dependencies](https://doc.rust-lang.org/nightly/cargo/guide/continuous-integration.html#verifying-latest-dependencies)
documentation so people will be more likely to prepared for this change.

# Drawbacks
[drawbacks]: #drawbacks

Maintainers that commit their `Cargo.lock` and verify their latest dependencies
will need to set `CARGO_BUILD_RUST_VERSION=false` in their environment.
See Alternatives for more on this.

While we hope this will give maintainers more freedom to upgrade their MSRV,
this could instead further entrench rust-version stagnation in the ecosystem.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

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
Instead, they'll have to make a change to their CI, like setting `CARGO_BUILD_RUST_VERSION=false`.

If we consider this a major incompatibility, then it needs to be opted into.
As `cargo fix` can't migrate a user's CI,
this would be out of scope for migrating to with a new Edition.

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

## Fallback to users rustc version

If no `package.rust-version` is specified, we can fallback to `rustc --version`.

As the dependency resolution is just a preference, this shouldn't cause churn.

We already query `rustc` for feature resolution, so this hopefully won't impact performance.

## Sort order when `package.rust-version` is unspecified

We could give versions without `package.rust-version` a lower priority, acting
as if they are always too new.

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
We could add a sticky flag to `Cargo.lock` though that could also be confusing.

This would also error or pick lower versions more than it needs to when a workspace contains multiple MSRVs.
We'd want to extend the resolver to treat Rust as yet another dependency and turn `package.rust-version` into dependencies on Rust.
This could cause a lot more backtracking which could negatively affect resolver performance for people with lower MSRVs.

If no `package.rust-version` is specified, we wouldn't want to fallback to the version of rustc being used because that could cause `Cargo.lock` churn if contributors are on different Rust versions.

# Prior art
[prior-art]: #prior-art


# Unresolved questions
[unresolved-questions]: #unresolved-questions


# Future possibilities
[future-possibilities]: #future-possibilities

## Encourage `package.rust-version` to be set more frequently

The user experience for this is based on the extent and quality of the data.
Ensuring we have `package.rust-version` populated more often (while maintaining
quality of that data) is an important problem but does not have to be solved to
get value out of this RFC and can be handled separately.

We could encourage people to set their MSRV by having `cargo new` default `package.rust-version`
We don't want to default `package.rust-version` in `cargo new`.
If people aren't committed to verifying it,
it is likely to go stale and will claim an MSRV much older than what is used in practice.
If we had the hard-error resolver mode and
[clippy warning people when using API items stabilized after their MSRV](https://github.com/rust-lang/rust-clippy/issues/6324),
this will at least annoy people into either being somewhat compatible or removing the field.

When missing, `cargo publish` could inject `package.rust-version` using the version of rustc used during publish.
This will err on the side of a higher MSRV than necessry and the only way to
workaround it is to set `CARGO_BUILD_RUST_VERSION=false` which will then lose
all other protections.

When missing, `cargo publish` could inject based on the rustup toolchain file.
This will err on the side of a higher MSRV than necessary as well.

When missing, `cargo publish` could inject `package.rust-version` inferred from
`package.edition` and/or other `Cargo.toml` fields.
This will err on the side of too low of an MSRV.
While this might help with in this situation,
it would lock us in to inaccurate information which might limit what analysis we could do in the future.

## Integrate `cargo audit`

If we [integrate `cargo audit`](https://github.com/rust-lang/cargo/issues/7678),
we can better help users on older dependencies identify security vulnerabilities.

## "cargo upgrade"

As we pull [`cargo upgrade` into cargo](https://github.com/rust-lang/cargo/issues/12425),
we'll want to make it respect MSRV as well

## cargo install

`cargo install` could auto-select a top-level package that is compatible with the version of rustc that will be used to build it.

See [rust-lang/cargo#10903](https://github.com/rust-lang/cargo/issues/10903) for more discussion.

**Note:** [rust-lang/cago#12798](https://github.com/rust-lang/cargo/pull/12798)
(slated to be released in 1.75) made it so `cargo install` will error upfront,
suggesting a version of the package to use and to pass `--locked` assuming the
bundled `Cargo.lock` has MSRV compatible dependencies.

## `build.rust-version = "<x>.<y>"`

We could allow people setting an effective rust-version within the config.
This would be useful for people who have a reason to not set `package.rust-version`
as well as to reproduce behavior with different Rust versions.
