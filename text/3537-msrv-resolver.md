- Feature Name: `msrv-resolver`
- Start Date: 2023-11-14
- Pre-RFC: [internals](https://internals.rust-lang.org/t/pre-rfc-msrv-aware-resolver/19871)
- RFC PR: [rust-lang/rfcs#3537](https://github.com/rust-lang/rfcs/pull/3537)
- Rust Issue: [rust-lang/cargo#9930](https://github.com/rust-lang/cargo/issues/9930)

# Summary
[summary]: #summary

Provide a happy path for developers needing to work with older versions of Rust by
- Preferring MSRV (minimum-supported-rust-version) compatible dependencies when Cargo resolves dependencies
- Ensuring compatible version requirements when `cargo add` auto-selects a version
- Smoothing out the path for setting and maintaining a verified MSRV so the above will be more likely to pick a working version.

Note: `cargo install` is intentionally left out for now to decouple discussions on how to handle the security ramifications.

**Note:** Approval of this RFC does not mean everything is set in stone, like with all RFCs.
This RFC will be rolled out gradually as we stabilize each piece.
In particular, we expect to make the `cargo new` change last as it is dependent on the other changes to work well.
In evaluating stabilization, we take into account changes in the ecosystem and feedback from testing unstable features.
Based on that evaluation, we may make changes from what this RFC says.
Whether we make changes or not, stabilization will then require approval of the cargo team to merge
(explicit acknowledgement from all but 2 members with no concerns from any member)
followed by a 10 days Final Comment Period (FCP) for the remaining 2 team members and the wider community.
Cargo FCPs are now tracked in This Week in Rust to ensure the community is aware and can participate.
Even then, a change like `cargo new` can be reverted without an RFC,
likely only needing to follow the FCP process.

# Motivation
[motivation]: #motivation

## Status Quo

<details><summary>Ensuring you have a <code>Cargo.lock</code> with dependencies compatible with your minimum-supported Rust version (MSRV) is an arduous task of running <code>cargo update &lt;dep&gt; --precise &lt;ver&gt;</code> until it works</summary>

Let's step through a simple scenario where a user develops with the
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

</details>

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

This was aided by the presence of `package.rust-version`.
Of all packages (137,569), only 8,857 (6.4%) have that field set.
When limiting to the 61,758 "recently" published packages (an upload since the start of 2023),
only 8,550 (13.8%) have the field set.

People have tried to reduce the pain from MSRV with its own costs:
- Treating it as a breaking change:
  - This leads to extra churn in the ecosystem when a fraction of users are likely going to benefit
  - We have the precedence elsewhere in the Rust ecosystem for build and runtime system requirement changes not being breaking, like when rustc requires newer versions of glibc, Android NDK, etc.
- Adding upper limits to version requirements:
  - This fractures the ecosystem by making packages incompatible with each other and the Cargo team [discourages doing this](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#multiple-requirements)
- Avoiding dependencies, re-implementing it themselves at the cost of their time and the risk for bugs, especially if `unsafe` is involved
- Ensuring dependencies have a more inclusive MSRV policy then themselves
  - This has lead to long arguments in the ecosystem over what is the right
    policy for updating a minimum-supported Rust version (MSRV),
    wearing on all
    (e.g.
    [libc](https://github.com/rust-lang/libs-team/issues/72),
    [time](https://github.com/time-rs/time/discussions/535)
    )

The sooner we improve the status quo, the better, as it can take years for
these changes to percolate out to those exclusively developing with an older
Rust version (in contrast with the example above).
This delay can be reduced somewhat if a newer toolchain can be used for
development version without upgrading the MSRV.

## Workflows

In solving this, we need to keep in mind how people are using Cargo and how to prioritize when needs of different workflows conflict.
We will then look at the potential designs within the context of this framework.

Some design criteria we can use for evaluating workflows:
- Cargo should not make major breaking changes
- Low barrier to entry
- The costs of “non-recommended” setups should focused on those that need them
- Encourage a standard of quality within the ecosystem, including
  - Assumption that things will work as advertised (e.g. our success with MSRV)
  - A pleasant experience (e.g. meaningful error messages)
  - Secure
- Every feature has a cost and we should balance the cost against the value we expect
  - Features can further constrain what can be done in the future due to backwards compatibility
  - Features increase maintenance burden
  - The larger the user-facing surface, the less likely users will find the feature they need and instead use the quickest shortcut
- Being transparent makes debugging easier, helps in evaluating risks (including security), and builds confidence in users
- Encourage progress and avoid stagnation
  - Proactively upgrading means the total benefit to developers from investments made in Rust is higher
  - Conversely, when most of the community is on old versions, it has a chilling effect on improving Rust
  - This also means feedback can come more quickly, making it easier and cheaper to pivot with user needs
  - Spreading the cost of upgrades over time makes forced-upgrades (e.g. for a security vulnerability) less of an emergency
  - Our commitment to compatibility helps keep the cost of upgrade low
- When not competing with the above, we should do the right thing for the user rather than disrupt their flow to tell them what they should instead do

And keeping in mind
- The Rust project only supports the latest version
  (e.g bug and security fixes)
  and the burden for support for older versions is on the vendor providing the older Rust toolchain.
- Even keeping upgrade costs low, there is still a re-validation cost that mission critical applications must pay
- Dependencies in `Cargo.lock` are not expected to change from contributors using different versions of the Rust toolchain without an explicit action like changing `Cargo.toml` or running `cargo update`
  - e.g. If the maintainer does `cargo add foo && git commit && git push`,
    then a contributor doing `git pull && cargo check` should not have a different selection of dependencies, independent of their toolchain versions (which might mean the second user sees an error about an incompatible package).

Some implications:
- "Support" in MSRV implies the same quality and responsiveness to bug reports, regardless of Rust version
- MSRV applies to all interactions with a project within the maintainers control
  (including as a registry dependency, `cargo install --locked`, as a git dependency, contributor experience; excluding transitive dependencies, rust-analyzer, etc),
  unless documented otherwise like
  - Some projects may document that enabling a feature will affect the MSRV (e.g. [moka](https://docs.rs/moka/0.12.1/moka/#minimum-supported-rust-versions))
  - Some projects may have a higher MSRV for building the repo (e.g. `Cargo.lock` with newer dependencies, reliance on cargo features that get stripped on publish)
- We should focus the cost for maintaining support for older versions of Rust on the user of the old version and away from the maintainer or the other users of the library or tool
  - Costs include lower developer productivity due to lack of access to features,
    APIs that don't integrate with the latest features,
    and slower build times due to pulling in extra code to make up for missing features
    (e.g. clap dropping its dependency on
    [is-terminal](https://crates.io/crates/is-terminal) in favor of
    [`IsTerminal`](https://doc.rust-lang.org/std/io/trait.IsTerminal.html)
    cut build time from [6s to 3s](https://github.com/rosetta-rs/argparse-rosetta-rs/commit/378cd2c30679afdf9b9843dbadea3e8951090809))

### Latest Rust with no MSRV

A user runs `cargo new` and starts development.

A maintainer may also want to avoid constraining their dependents, for a variety of reasons, and leave MSRV support as a gray area.

**Priority 1 because:**
- ✅ No MSRV is fine as pushing people to have an MSRV would lead to either
  - an inaccurate reported MSRV from it going stale which would lower the quality of the ecosystem
  - raise the barrier to entry by requiring more process for packages and pushing the cost of old Rust versions on people who don't care

**Pain points:**

The Rust toolchain does not provide a way to help users know new dependency versions are available, to support the users in staying up-to-date.
MSRV build errors from new dependency versions is one way to do it though not ideal as this disrupts the user.
Otherwise, they must actively run `rustup update` or follow Rust news.

For dependents, this makes it harder to know what versions are "safe" to use.

### Latest Rust as the MSRV

A maintainer regularly updates their MSRV to latest.
They can choose to provide a level of support for old MSRVs by reserving MSRV
changes to minor version bumps,
giving them room to backport fixes.
Due to the pain points listed below, the target audience for this workflow is likely small,
likely pushing them to not specify their MSRV.

**Priority 2 because:**
- ✅ Low barrier to maintaining a high quality of support for their MSRV
- ✅ Being willing to advertising an MSRV, even if latest, improves the information available to developers, increasing the quality of the ecosystem
- ✅ Costs for dealing with old Rust toolchains is shifted from the maintainer and the users on a supported toolchain to those on an unsupported toolchain
- ✅ By focusing new development on latest MSRV, this provides a carrot to encourage others to actively upgrading

**Pain points (in addition to the prior workflow):**

In addition to their toolchain version, the Rust toolchain does not help these users with keeping
their MSRV up-to-date.
They can use other tools like
[RenovateBot](https://github.com/rust-lang/cargo/blob/87eb374d499100bc945dc0e50ae5194ae539b964/.github/renovate.json5#L12-L24)
though that causes extra churn in the repo.

A package could offer a lower MSRV in an unofficial capacity or with a lower quality of support
but the requirement that dependents always pass `--ignore-rust-version` makes this disruptive.

### Extended MSRV

This could be people exclusively running one version or that support a range of versions.
So why are people on old versions?
- Not everyone is focused on Rust development and might only touch their Rust code once every couple of months,
  making it a pain if they have to update every time.
  - Think back to slow git index updates when you've stepped away and consider people who we'd be telling to run `rustup update` every time they touch Rust
- While a distribution provides rust to build other packages in the distribution,
  users might assume that is a version to use, rather than getting Rust through `rustup`
- Re-validation costs for updating core parts of the image for an embedded Linux developers can be high, keeping them on older versions
- Updates can be slow within tightly controlled environments (airgaps, paperwork, etc)
- Qualifying Rust toolchains takes time and money, see [Ferrocene](https://ferrous-systems.com/ferrocene/)
- Built on or for systems that are no longer supported by rustc (e.g. old glibc, AndroidNDK, etc)
- Library and tool maintainers catering to the above use cases

The MSRV may extend back only a couple releases or to a year+ and
they may choose to update on an as-need basis or keep to a strict cadence.

Depending on the reason they are working with an old version,
they might be developing the project with it or they might be using the latest toolchain.
For some of these use cases, they might controlling their "MSRV" via `rust-toolchain.toml`, rather than `package.rust-version`, as its their only supported Rust version (e.g. an application with a vetted toolchain).

When multiple Rust versions are supported, like with library and tool maintainers,
they will need to verify at least their MSRV and latest.
Ideally, they also [verify their latest dependencies](https://doc.rust-lang.org/cargo/guide/continuous-integration.html#verifying-latest-dependencies)
though this is already a recommended practice when people follow the
[default choice to commit their lockfile](https://doc.rust-lang.org/cargo/faq.html#why-have-cargolock-in-version-control).
The way they verify dependencies is restricted as they can't rely on always updating via Dependabot/RenovateBot as a way to verify them.
Maintainers likely only need to do a compilation check for MSRV as their regular CI runs ensure that the behavior (which is usually independent of rust version) is correct for the MSRV-compatible dependencies.

**Priority 3 because:**
- ✅ Several use cases for this workflow have little alternative
- ✅ MSRV applies to all interactions to the project which also means that the level of "support" is consistent
- ❌ This implies stagnation and there are cases where people could more easily use newer toolchains, like Debian users, but that is less so the case for other users
- ❌ For library and tool maintainers, they are absorbing costs from these less common use cases
  - They could shift these costs to those that need old versions by switching to the "Latest MSRV" workflow by allowing their users to backport fixes to prior MSRV releases

**Pain points:**

Maintaining a working `Cargo.lock` is frustrating, as demonstrated earlier.

When developing with the latest toolchain,
feedback is delayed until CI or an embedded image build process which can be frustrating
(using too-new dependencies, using too-new Cargo or Rust features, etc).

### Extended published MSRV w/ latest development MSRV

This is where the published package for a project claims an extended MSRV but interactions within the repo require the latest toolchain.
The requirement on the latest MSRV could come from the `Cargo.lock` containing dependencies with the latest MSRV or they could be using Cargo features that don't affect the published package.
In some cases, the advertised MSRV might be for a lower tier of support than what is supported for the latest version.
For instance, a project might intentionally skip testing against their MSRV because of known bugs that will fail the test suite.

In some cases, the MSRV-incompatible dependencies might be restricted to `dev-dependencies`.
Though local development can't be performed with the MSRV,
the fact that the tests are verifying (on a newer toolchain) that the build/normal dependencies work gives a good amount of confidence that they will work on the MSRV so long as they compile.

Compared to the above workflow, this is likely targeted at just library and tool maintainers as other use cases don't have access to the latest version or they are needing the repo to be compatible with their MSRV.

**Priority 4 because:**
- ❌ The MSRV has various carve outs, providing an inconsistent experience compared to other packages using other workflows and affecting the quality of the ecosystem
  - For workspaces with bins, `cargo install --locked` is expected to work with the MSRV but won't
  - If they use new Cargo features, then `[patch]`ing in a git source for the dependency won't work
  - For contributors, they must be on an unspecified Rust toolchain version
- ❌ The caveats involved in this approach (see prior item) would lead to worse documentation which lowers the quality to users
- ❌ This still leads to stagnation despite being able to use the latest dependencies as they are limited in what they can use from them and they can't use features from the latest Rust toolchain
- ❌ These library and tool maintainers are absorbing costs from the less common use cases of their dependents
  - They could shift these costs to those that need old versions by switching to the "Latest MSRV" workflow by allowing their users to backport fixes to prior MSRV releases

**Pain points:**

Like the prior workflow, when developing with the latest toolchain,
feedback is delayed until CI or an embedded image build process which can be frustrating
(using too-new dependencies, using too-new Cargo or Rust features, etc).

When `Cargo.lock` is resolved for latest dependencies, independent of MSRV,
verifying the MSRV becomes difficult as they  must either juggle two lockfiles, keeping them in sync, or use the unstable `-Zminimal-versions`.
The two lockfile approach also has all of the problems shown earlier in writing the lockfile.

When only keeping MSRV-incompatible `dev-dependencies`,
one lockfile can be used but it can be difficult to edit the `Cargo.lock` to ensure you get new `dev-dependencies` without infecting other dependency types.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

We are introducing several new concepts
- A v3 resolver (`package.resolver`) that will prefer packages compatible with your `package.rust-version` over those that aren't
  - If `package.rust-version` is unset, then your current Rust toolchain version will be used
  - This resolver version will be the default for the next edition
  - A `.cargo/config.toml` field will be added to disable this, e.g. for CI
- Cargo will ensure users are aware their dependencies are behind the latest in a unobtrusive way
- `cargo add` will select version requirements that can be met by a dependency with a compatible version
- A new value for `package.rust-version`, `"tbd-name-representing-currently-running-rust-toolchain"`, which will advertise in your published package your current toolchain version as the minimum-supported Rust version
  - `cargo new` will default to `package.rust-version = "tbd-name-representing-currently-running-rust-toolchain"`
- A deny-by-default lint will replace the build error from a package having an incompatible Rust version, allowing users to opt-in to overriding it

## Example documentation updates

### The `rust-version` field

*(update to [manifest documentation](https://doc.rust-lang.org/cargo/reference/manifest.html#the-rust-version-field))*

The `rust-version` field is an optional key that tells cargo what version of the
Rust language and compiler you support compiling your package with. If the currently
selected version of the Rust compiler is older than the stated version, cargo
will exit with an error, telling the user what version is required.
To support this, Cargo will prefer dependencies that are compatible with your `rust-version`.

```toml
[package]
# ...
rust-version = "1.56"
```

The Rust version can be a bare version number with two or three components; it
cannot include semver operators or pre-release identifiers. Compiler pre-release
identifiers such as -nightly will be ignored while checking the Rust version.
The `rust-version` must be equal to or newer than the version that first
introduced the configured `edition`.

The Rust version can also be `"tbd-name-representing-currently-running-rust-toolchain"`.
This will act the same as if it was set to the version of your Rust toolchain.
Your published manifest will have `"tbd-name-representing-currently-running-rust-toolchain"` replaced with the version of your Rust toolchain.

Setting the `rust-version` key in `[package]` will affect all targets/crates in
the package, including test suites, benchmarks, binaries, examples, etc.

*Note: The first version of Cargo that supports this field was released with Rust 1.56.0.
In older releases, the field will be ignored, and Cargo will display a warning.*

### Rust Version

*(update to [Dependency Resolution's Other Constraints documentation](https://doc.rust-lang.org/cargo/reference/resolver.html))*

When multiple versions of a dependency satisfy all version requirements,
cargo will prefer those with a compatible `package.rust-version` over those that
aren't compatible.
Some details may change over time though `cargo check && rustup update && cargo check` should not cause `Cargo.lock` to change.

##### `resolver.precedence`

*(update to [Configuration](https://doc.rust-lang.org/cargo/reference/config.html))*

* Type: string
* Default: "rust-version"
* Environment: `CARGO_RESOLVER_PRECEDENCE`

Controls how `Cargo.lock` gets updated on changes to `Cargo.toml` and with `cargo update`.  This does not affect `cargo install`.

* `maximum`: prefer the highest compatible versions of dependencies
* `rust-version`: prefer dependencies where their `package.rust-version` is less than or equal to your `package.rust-version`

`rust-version` can be overridden with `--ignore-rust-version` which will fallback to `maximum`.

## Example workflows

We'll step through several scenarios to highlight the changes in the user experience.

### Latest Rust with MSRV

I'm learning Rust and wanting to write my first application.
The book suggested I install using `rustup`.

<details><summary>Expand for step through of this workflow</summary>

I've recently updated my toolchain
```console
$ rustup update
Downloading and install 1.92
```

At some point, I start a project:
```console
$ cargo new foo
$ cat foo/Cargo.toml
```

```toml
[package]
name = "foo"
version = "0.1.0"
edition = "2024"
rust-version = "tbd-name-representing-currently-running-rust-toolchain"

# See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

[dependencies]
```

```console
$ cargo add clap -F derive
Adding clap 5.10.30
```

*(note: this user would traditionally be a "Latest Rust" user but `package.rust-version` automatically them moved to "Latest Rust with MSRV" without extra validation effort or risk of their MSRV going stale)*

After some time, I get back to my project and decide to add completion support:
```console
$ cargo add clap_complete
Adding clap_complete 5.10.40
warning: clap_complete 5.11.0 exists but requires Rust 1.93 while you are running 1.92.
To use the clap_complete@5.11.0 with a compatible Rust version, run `rustup update && cargo add clap_complete@5.11.0`.
To force the use of clap_complete@5.11.0 independent of your toolchain, run `cargo add clap_complete@5.11.0`
```

Wanting to be on the latest version, I run
```console
$ rustup update
Downloading and install 1.94
$ cargo update
Updating clap v5.10.30 -> v5.11.0
Updating clap_complete v5.10.40 -> v5.11.0
```

**Alternate:** But what if I manually edited `Cargo.toml` instead of `cargo add`?
Here, we can shortcut some questions about version requirements because clap aligns on minor releases.
```toml
[package]
name = "foo"
version = "0.1.0"
edition = "2024"
rust-version = "tbd-name-representing-currently-running-rust-toolchain"

# See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

[dependencies]
clap = { version = "5.10.30", features = ["derive"] }
clap_complete = "5.10"  # <-- new
```

And away I go:
```console
$ cargo check
Warning: adding clap_complete@5.10.40 because 5.11.0 requires Rust 1.93 while you are running 1.92.
To use the clap_complete@5.11.0 with a compatible Rust version, run `rustup update && cargo update`.
To force the use of clap_complete@5.11.0 independent of your toolchain, run `cargo update --ignore-rust-version`
```

But I am in a hurry and don't want to disrupt my flow.
`clap_complete@5.10.40` is likely fine.
I am running `clap@5.10.30` and that has been working for me.
I might even run [`cargo deny`](https://crates.io/crates/cargo-deny) to see if there are known vulnerabilities.
So I continue development.

Later I run:
```console
$ cargo update
Name          Current Latest Note
============= ======= ====== ==================
clap          5.10.30 5.11.0 requires Rust 1.93
clap_complete 5.10.40 5.11.0 requires Rust 1.93
note: To use the latest depednencies, run `rustup update && cargo update`.
To force the use of the latest dependencies, independent of your toolchain, run `cargo update --ignore-rust-version`
$ rustup update
Downloading and install 1.94
$ cargo update
Updating clap v5.10.30 -> v5.11.0
Updating clap_complete v5.10.40 -> v5.11.0
```

At this point, I want to publish
```console
$ cargo publish
... crates.io error about missing fields
$ $EDITOR `Cargo.toml`
$ cargo publish
Published foo 0.1.0
```

If I look on crates.io, the new 0.1.0 version shows up with a rust-version of 1.94
without me having to manual update the field and
relying on the `cargo publish`s verify step to verify the correctness of that MSRV.

</details>

### Extended "MSRV" with an application

I am developing an application using a certified toolchain.
I specify this toolchain using a `rust-toolchain.toml` file.

Rust 1.94 is the latest but my certified toolchain is 1.92.

<details><summary>Expand for step through of this workflow</summary>

At some point, I start a project:
```console
$ cargo new foo
$ cat foo/Cargo.toml
```

```toml
[package]
name = "foo"
version = "0.1.0"
edition = "2024"
rust-version = "tbd-name-representing-currently-running-rust-toolchain"

# See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

[dependencies]
```

```console
$ cargo add clap -F derive
Adding clap 5.10.30
warning: clap 5.11.0 exists but requires Rust 1.93 while you are running 1.92.
To use the clap@5.11.0 with a compatible Rust version, run `rustup update && cargo add clap@5.10.0`.
To force the use of clap_complete@5.11.0 independent of your toolchain, run `cargo add clap@5.10.0`
```

At this point, I have a couple of options
1. I check and clap advertises that they "support" Rust 1.92 by cherry-picking fixes into 5.10 and I feel comfortable with that
2. I check `cargo deny` and don't see any vulnerabilities and that is good enough for me, knowing that the majority of my users are likely on newer versions
3. I decide that clap doesn't align with my interests and use something else

Assuming (1) or (2) applies, I ignore the warning and move on.

</details>

### Extended MSRV with an application targeting multiple Rust versions

*(this is a re-imagining of the Motivation's example)*

I'm building an application that is deployed to multiple embedded Linux targets.
Each target's image builder uses a different Rust toolchain version to avoid re-validating the image.

<details><summary>Expand for step through of this workflow</summary>

I've recently updated my toolchain
```console
$ rustup update
Downloading and install 1.94
```

At some point, I start a project:
```console
$ cargo new foo
$ cat foo/Cargo.toml
```

```toml
[package]
name = "foo"
version = "0.1.0"
edition = "2024"
rust-version = "tbd-name-representing-currently-running-rust-toolchain"

# See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

[dependencies]
```

```console
$ cargo add clap -F derive
Adding clap 5.11.0
```

I send this to my image builder and I get this failure for one of my embedded targets:
```console
$ cargo build
error: clap 5.11.0 requires Rust 1.93.0 while you are running 1.92.0

note: downgrade to 5.10.30 for a version compatible with Rust 1.92.0
note: set `package.rust-version = "1.92.0"` to ensure compatible versions are selected in the future
note: lint `cargo::incompatible-msrv` is denied by default

```

I make the prescribed changes:
```toml
[package]
name = "foo"
version = "0.1.0"
edition = "2024"
rust-version = "1.92"  # <-- was "tbd-name-representing-currently-running-rust-toolchain" before I edited it

# See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

[dependencies]
clap = { version = "5.10.30", features = ["derive"] }  # <-- downgraded
```

And my image build works!

After some time, I run:
```console
$ cargo update
Name          Current Latest Note
============= ======= ====== ==================
clap          5.10.30 5.11.0 requires Rust 1.93
clap_complete 5.10.40 5.11.0 requires Rust 1.93
note: To use the latest depednencies, run `rustup update && cargo update`.
To force the use of the latest dependencies, independent of your toolchain, run `cargo update --ignore-rust-version`
```

We've EOLed the last embedded target that supported 1.92 and so we can update our `package.rust-version`,
so we can update it and our dependencies:
```console
$ cargo update --update-rust-version
Updating clap 5.10.30 to 5.11.0
Updating foo's rust-version from 1.92 to 1.93
```

</details>

### Extended MSRV for a Library

I'm developing a new library and am willing to take on some costs for supporting people on older toolchains.

<details><summary>Expand for step through of this workflow</summary>

I've recently updated my toolchain
```console
$ rustup update
Downloading and install 1.94
```

At some point, I start a project:
```console
$ cargo new foo --lib
```

I've decided on an "N-2" MSRV policy:
```toml
[package]
name = "foo"
version = "0.1.0"
edition = "2024"
rust-version = "1.92"  # <-- was "tbd-name-representing-currently-running-rust-toolchain" before I edited it

# See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

[dependencies]
```

```console
$ cargo add clap -F derive
Adding clap 5.10.30
warning: clap 5.11.0 exists but requires Rust 1.93 while `foo` has `package.rust-version = "1.92"`
To use clap@5.11.0 with a compatible package.rust-version, run `cargo add clap@5.11.0 --update-rust-version`
To force the use of clap@5.11.0 independent of your toolchain, run `cargo add clap@5.11.0`
```

At this point, I have a couple of options
1. I check and clap advertises that they "support" Rust 1.92 by cherry-picking fixes into 5.10 and I feel comfortable with that
2. I check `cargo deny` and don't see any vulnerabilities and that is good enough for me, knowing that the majority of my users are likely on newer versions
3. I decide that clap doesn't align with my interests and use something else

Assuming (1) or (2) applies, I ignore the warning and move on.

After some time, I run:
```console
$ cargo update
Name          Current Latest Note
============= ======= ====== ==================
clap          5.10.30 5.11.0 requires Rust 1.93
clap_complete 5.10.40 5.11.0 requires Rust 1.93
note: To use the latest depednencies, run `rustup update && cargo update`.
To force the use of the latest dependencies, independent of your toolchain, run `cargo update --ignore-rust-version`
```

At this point, 1.95 is out, so I'm fine updating my MSRV and I run:
```console
$ cargo update --update-rust-version
Updating clap 5.10.30 to 5.11.0
Updating foo's rust-version from 1.92 to 1.93
```

Instead, if a newer clap version was out needing 1.94 or 1.95, I would instead edit `Cargo.toml` myself.

</details>

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

We expect these changes to be independent enough and beneficial on their own that they can be stabilized as each is completed.

## Cargo Resolver

We will be adding a v3 resolver, specified through `workspace.resolver` / `package.resolver`.
This will become default with the next Edition.

When `resolver = "3"` is set, Cargo's resolver will change to *prefer* MSRV compatible versions over
incompatible versions when resolving new dependencies, except for `cargo install`.
Initially, dependencies without `package.rust-version` will be preferred over
MSRV-incompatible packages but less than those that are compatible.
The exact details for how preferences are determined may change over time,
particularly when no MSRV is specified,
but this shouldn't affect existing `Cargo.lock` files since the currently
resolved dependencies always get preference.

This can be overridden with `--ignore-rust-version` and config's `resolver.precedence`.

Implications
- If you use `cargo update --precise <msrv-incompatible-ver>`, it will work
- If you use `--ignore-rust-version` once, you don't need to specify it again to keep those dependencies though you might need it again on the next edit of `Cargo.toml` or `cargo update` run
- If a dependency doesn't specify `package.rust-version` but its transitive dependencies specify an incompatible `package.rust-version`,
  we won't backtrack to older versions of the dependency to find one with a MSRV-compatible transitive dependency.
- A package with multiple MSRVs, depending on the features selected, can still do this as version requirements can still require versions newer than the MSRV and `Cargo.lock` can depend on those as well.

As there is no `workspace.rust-version`,
the resolver will pick the lowest version among workspace members.
This will be less optimal for workspaces with multiple MSRVs and dependencies unique to the higher-MSRV packages.
Users can workaround this by raising the version requirement or using `cargo update --precise`.

When `rust-version` is unset,
we'll fallback to `rustc --version` if its not a pre-release.
This is primarily targeted at helping users with a
[`rust-toolchain.toml` file](https://rust-lang.github.io/rustup/overrides.html#the-toolchain-file)
(to reduce duplication)
though this would also help users who happen to be on an old rustc, for whatever reason.
As this is just a preference for resolving dependencies, rather than prescriptive,
this shouldn't cause churn of the `Cargo.lock` file.
We already call `rustc` for feature resolution, so hopefully this won't have a performance impact.

## Cargo config

We'll add a `resolver.precedence ` field to `.cargo/config.toml` which will control the package version prioritization policy.

```toml
[build]
resolver.precedence = "rust-version"  # Default with `v3`
```

with potential values being:
- `maximum`: behavior today (default for v1 and v2 resolvers)
  - Needed for [verifying latest dependencies](https://doc.rust-lang.org/nightly/cargo/guide/continuous-integration.html#verifying-latest-dependencies)
- `minimum` (unstable): `-Zminimal-versions`
  - As this is just precedence, `-Zdirect-minimal-versions` doesn't fit into this
- `rust-version`:  what is defined in the package (default for v3 resolver)
- `rust-version=` (future possibility)
  - `package`: long form of `rust-version`
  - `rustc`: the current running version
    - Needed for "separate development / publish MSRV" workflow
  - `<x>[.<y>[.<z>]]`: manually override the version used

If a `rust-version` value is used, we'd switch to `maximum` when `--ignore-rust-version` is set.

## `cargo build`

The MSRV-compatibility build check will be demoted from an error to a `deny`-by-default workspace
[diagnostic](https://github.com/rust-lang/cargo/issues/12235),
allowing users to intentionally use dependencies on an unsupported (or less supported) version of Rust
without requiring `--ignore-rust-version` on every invocation.

Ideally, we present all of the MSRV issues upfront to be resolved together.
At minimum, we should present a top-down message, rather than bottom up.

If `package.rust-version` is unset or `"tbd-name-representing-currently-running-rust-toolchain"`, the diagnostic should suggest setting it
to help raise awareness of `package.rust-version` being able to reduce future
resolution errors.
This would benefit from knowing the oldest MSRV.

## `cargo update`

`cargo update` will inform users when an MSRV or semver incompatible version is available.
`cargo update --dry-run` will also report this information so that users can check on the status of this at any time.

Users may pass
- `--ignore-rust-version` to pick the latest dependencies, ignoring all `rust-version` fields (your own and from dependencies)
- `--update-rust-version` to pick the `rustc --version`-compatible dependencies, updating your `package.rust-version` if needed to match the highest of your dependencies
- `<pkgname> --precise <version>` to pick a specific version, independent of the `rust-version` field

We expect the notice to inform users of these options for allowing them to upgrade.

Those flags will also be added to `cargo generate-lockfile`

## Syncing `Cargo.toml` to `Cargo.lock` on any Cargo command

In addition to the `cargo update` output to report when things are held back (both MSRV and semver),
we will try having dependency resolves highlight newly selected dependency versions that were held back due to MSRV or semver.
Whether we do this and how much will be subject to factors like noisy output, performance, etc.

Some approaches we can take for doing this include:

After resolving, we can do a depth-first diff of the trees, stopping and reporting on the first different node.
This would let us report on any command that changes the way the tree is resolved
(from explicit changes with `cargo update` to `cargo build` syncing `Cargo.toml` changes to `Cargo.lock`).
We'd likely want to limit the output to only the sub-tree that changed.
If there wasn't previously a `Cargo.lock`, this would mean everything.

We could either always do the second resolve or only do the second resolve if the resolver changed anything,
whichever is faster.

Its unknown whether making the inputs available for multiple resolves would have a performance impact.

While a no-change resolve is fast, if this negatively impacts it enough, we
could explore hashing the resolve inputs and storing that in the lockfile,
allowing us to detect if the inputs have changed and only resolving then.


## `cargo add`

`cargo add <pkg>` (no version) will pick a version requirement that is low
enough so that when it resolves, it can pick a dependency that is
MSRV-compatible.
`cargo add` will warn when it does this.

Users may pass
- `--ignore-rust-version` to pick the latest dependencies, ignoring all `rust-version` fields (your own and from dependencies)
- `--update-rust-version` to pick the `rustc --version`-compatible dependencies, updating your `package.rust-version` if needed to match the highest of your dependencies

## `cargo publish`

`package.rust-version` will gain support for an `"tbd-name-representing-currently-running-rust-toolchain"` value, in addition to partial versions.
On `cargo publish` / `cargo package`, the generated `*.crate`s `Cargo.toml` will have `"tbd-name-representing-currently-running-rust-toolchain"` replaced with `rustc --version`.
If `rustc --version` is a pre-release, publish will fail.

`cargo new` will include `package.rust-version = "tbd-name-representing-currently-running-rust-toolchain"`.

# Drawbacks
[drawbacks]: #drawbacks

Users upgrading to the next Edition (or changing to `resolver = '3"`), will have to manually update their CI to test the latest dependencies with `CARGO_RESOLVER_PRECEDENCE=maximum`.

Workspaces have no `edition`, so its easy for users to not realize they need to set `resolver = "3"` or to update their `resolver = "2"` to `"3"`
(Cargo only warns on [virtual manifests without an explicit `workspace.resolver`](https://github.com/rust-lang/cargo/pull/10910)).

While we hope this will give maintainers more freedom to upgrade their MSRV,
this could instead further entrench rust-version stagnation in the ecosystem.

For projects with larger MSRVs than their dependencies,
this introduces another form of drift from the latest dependencies
(in addition to [lockfiles](https://doc.rust-lang.org/cargo/faq.html#why-have-cargolock-in-version-control)).
However, we already recommend people
[verify their latest dependencies](https://doc.rust-lang.org/nightly/cargo/guide/continuous-integration.html#verifying-latest-dependencies),
so the only scenario this further degrades is when lockfiles are verified by always updating to the latest, like with RenovateBot,
and only in the sense that the user needs to know to explicitly take action to add another verification job to CI.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Misc alternatives
- Dependencies with unspecified `package.rust-version`: we could mark these as always-compatible or always-incompatible; there really isn't a right answer here.
- The resolver doesn't support backtracking as that is extra complexity that we can always adopt later as we've reserved the right to make adjustments to what `cargo generate-lockfile` will produce over time.
- `CARGO_RESOLVER_PRECEDENCE` is used, rather than a CLI option (e.g. ensuring every command has `--ignore-rust-version` or a `--rust-version <x.y.z>`)
  - This is unlikely to be used in one-off cases but across whole interactions which is better suited for config / env variables, rather than CLI options
  - Minimize CLI clutter
- `CARGO_RESOLVER_PRECEDENCE=rust-version` implies maximal resolution among MSRV-compatible dependencies.   Generally MSRV doesn't decrease over versions, so minimal resolution will likely pick packages with compatible rust-versions.
  - `cargo add` helps by selecting rust-version-compatible minimum bounds
  - This bypasses a lot of complexity either from exploding the number of states we support or giving users control over the fallback by making the field an array of strategies.
- Instead of `resolver = "3"`, we could just change the default for everyone
  - The number of maintainers verifying latest dependencies is likely
    relatively low and they are more likely to be "in the know",
    making them less likely to be negatively affected by this.
    Therefore, we could probably get away with treating this as a minor incompatibility
  - Either way, the concern is to ensure that the change receives attention.
    We wouldn't want this to be like sparse registries where a setting exists and we change the default and people hardly notice (besides any improvements)
- `cargo build` will treat incompatible MSRVs as a workspace-level lint, rather than a package level lint, to avoid the complexity of mapping the dependency to a workspace-member to select `[lint]` tables to respect and then dealing with unifying conflicting levels in between `[lint]` tables among members.
- `--ignore-rust-version` picks absolutely the latest dependencies to support both users on latest rustc and users wanting "unsupported" dependencies, at the cost of users not on the latest rustc but still wanting latest more up-to-date dependencies than their MSRV allows
- Compilation commands (e.g. `cargo check`) will take on two meanings for `--ignore-rust-version`, (1) `allow` the workspace diagnostic and (2) resolve changed dependencies to latest when syncing `Cargo.toml` to `Cargo.lock`.
  - This expansion of scope is for consistency
  - Being a flag to turn the `deny` into an `allow` is a high friction workflow that we expect users to not be too negatively impacted by this expansion.
  - With the resolver config and the configurable lint, we also expect the flag on compilation commands to be diminished in value.  Maybe in the future we could even deprecate it and/or hide it.
- `--update-rust-version` picks `rustc --version`-compatible dependencies so users can easily walk the treadmill of updating their dependencies / MSRV , no matter their `rustc` version.
  - There is little reason to select an MSRV higher than their Rust toolchain
  - We should still be warning the user that new dependencies are available if they upgrade their Rust toolchain
  - This comes at the cost of inconsistency with `--ignore-rust-version`.
- Nightly `cargo publish` with `"tbd-name-representing-currently-running-rust-toolchain"` fails because there isn't a good value to use and this gives us flexibility to change it later (e.g. just leaving the `rust-version` as unset).

## Ensuring the registry Index has `rust-version` without affecting quality

The user experience for this is based on the extent and quality of the data.
Ensuring we have `package.rust-version` populated more often (while maintaining
quality of that data) is an important problem but does not have to be solved to
get value out of this RFC and can be handled separately.

We chose an opt-in for populating `package.rust-version` based on `rustc --version` (`"tbd-name-representing-currently-running-rust-toolchain"`).
This will encourage a baseline of quality as users are developing with that version and `cargo publish` will do a verification step, by default.
This will help seed the Index with more `package.rust-version` data for the resolver to work with.
The downside is that the `package.rust-version` will likely be higher than it absolutely needs.
However, considering our definition of "support" and that the user isn't bothering to set an MSRV themself,
aggressively updating is likely fine in this case, especially since we'll let dependents override the build failure for MSRV-incompatible packages.

Some alternative solutions include:

When missing, `cargo publish` could inject `package.rust-version` using the version of rustc used during publish.
**However**, this will err on the side of a higher MSRV than necessary and the only way to
work around it is to set `CARGO_RESOLVER_PRECEDENCE=maximum` which will then lose
all other protections.
As we said, this is likely fine but then there will be no way to opt-out for the subset of maintainers who want to keep their support definition vague.
As things evolve, we could re-evaluate making `"tbd-name-representing-currently-running-rust-toolchain"` the default.

~~We could encourage people to set their MSRV by having `cargo new` default `package.rust-version`.~~
**However**, if people aren't committed to verifying that was implicitly set,
it is likely to go stale and will claim an MSRV much older than what is used in practice.
If we had the hard-error resolver mode and
[clippy warning people when using API items stabilized after their MSRV](https://github.com/rust-lang/rust-clippy/issues/6324),
this will at least annoy people into either being somewhat compatible or removing the field.

~~When missing, `cargo publish` could inject `package.rust-version` inferred from
`package.edition` and/or other `Cargo.toml` fields.~~
**However**, this will err on the side of too low of an MSRV.
These fields have an incomplete picture.
While this helps ensure there is more data for the MSRV-aware resolver,
future analysis wouldn't be able to distinguish between inferred and explicit `package.rust-version`s.
We'd also need an explicit opt-out for those who intentionally don't want one set.

Alternatively, `cargo publish` / the registry could add new fields to the Index
to represent an inferred MSRV, the published version, etc
so it can inform our decisions without losing the intent of the publisher.

We could help people keep their MSRV up to date, by letting them specify a policy
(e.g. `rust-version-policy = "stable - 2"` or `rust-version-policy = "stable"`);
then, every time the user runs `cargo update`,
we could automatically update their `rust-version` field as well.
This would also be an alternative to `--update-rust-version` that can be further explored in the future if desired.
There are aspects of this that need to be worked out before going down this route
- Without gating this behind a flag, this will push people away from bumping their MSRV only on minor version bumps.
- Tying this to `cargo update` encourages other side effects by default (`--workspace` flag would be needed to do no other update) which pushes people to a more casual approach to MSRV updating, even if we have a flag
- We need to figure out what policies are appropriate and what syntax to use for them
  - While a continuous sliding window (`N-M`) is most commonly used today,
    it is unclear if that is the right policy to bake in compared to others like periodic updates (`*/M` in cron syntax) to be helping the "Extended MSRV" users along with everyone else.
- Is `stable` clear enough to mean "current version a time of `cargo update` with ratcheting semantics"?  What name can work best?

When there still isn't an MSRV set, the resolver could
- Assume the MSRV of the next published package with an MSRV set
- Sort no-MSRV versions by minimal versions, the lower the version the more likely it is to be compatible
  - This runs into quality issues with version requirements that are likely too low for what the package actually needs
  - For dependencies that never set their MSRV, this effectively switches us from maximal versions to minimal versions.

## Configuring the resolver mode on the command-line or `Cargo.toml`

The Cargo team is very interested in [moving project-specific config to manifests](https://github.com/rust-lang/cargo/issues/12738).
However, there is a lot more to define for us to get there.  Some routes that need further exploration include:
- If its a CLI flag, then its transient, and its unclear which modes should be transient now and in the future
  - We could make it sticky by tracking this in `Cargo.lock` but that becomes less obvious what resolver mode you are in and how to change
- We could put this in `Cargo.toml` but that implies it unconditionally applies to everything
  - But we want `cargo install` to use the latest dependencies so people get bug/security fixes
  - This gets in the way of "Extended published MSRV w/ latest development MSRV" being able to change it in CI to verify MSRV and "Extended MSRV" being able to change it in CI to verify latest dependencies

By relying on config we can have a stabilized solution sooner and we can work out more of the details as we better understand the relevant problems.

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

## Resolver behavior

Effects of current solution on workflows (including non-resolver behavior):
1. Latest Rust with no MSRV
  - ✅ `cargo new` setting `package.rust-version = "tbd-name-representing-currently-running-rust-toolchain"` moves most users to "Latest Rust as the MSRV" with no extra maintenance cost
  - ✅ Dealing with incompatible dependencies will have a friendlier face because the hard build error after changing dependencies is changed to a notification during update suggesting they upgrade to get the new dependency because we fallback to `rustc --version` when `package.rust-version` is unset (as a side effect of us capturing `rust-toolchain.toml`)
2. Latest Rust as the MSRV
  - ✅ Packages can more easily keep their MSRV up-to-date with
    - `package.rust-version = "tbd-name-representing-currently-running-rust-toolchain"` (no policy around when it is changed) though this is dependent on your Rust toolchain being up-to-date (see "Latest Rust with no MSRV" for more)
    - `cargo update --update-rust-version` (e.g. when updating minor version) though this is dependent on what you dependencies are using for an MSRV
  - ✅ Packages can more easily offer unofficial support for an MSRV due to shifting the building with MSRV-incompatible dependencies from an error to a `deny` diagnostic
3. Extended MSRV
  - ✅ `Cargo.lock` will Just Work
4. Extended published MSRV w/ latest development MSRV
  - ❌ Maintainers will have to opt-in to latest dependencies, in a `.cargo/config.toml`
  - ✅ Verifying MSRV will no longer require juggling `Cargo.lock` files or using unstable features

A short term benefit (hence why this is separate) is that an MSRV-aware resolver by default is that we can use it as a polyfill for
[`cfg(version)`](https://dev-doc.rust-lang.org/stable/unstable-book/language-features/cfg-version.html)
(which will likely need a lot of work in cargo after we finish stabilizing it for rustc).
A polyfill package can exist that has multiple maintained semver-compatible versions with different MSRVs with the older ones leveraging external libraries while the newer ones leverage the standard library.

### Make this opt-in rather than opt-out

Instead of adding `resolver = "3"`, we could keep the default resolver the same as today but allow opt-in to MSRV-aware resolver via `CARGO_RESOLVER_PRECEDENCE=rust-version`.
- When building with old Rust versions, error messages could suggest re-resolving with `CARGO_RESOLVER_PRECEDENCE=rust-version`.
  The next corrective step (and suggestion from cargo) depends on what the user is doing and could be either
  - `git checkout main -- Cargo.lock && cargo check`
  - `cargo generate-lockfile`
- We'd drop from this proposal `cargo update [--ignore-rust-version|--update-rust-version]` as they don't make sense with this new default

This has no impact on the other proposals (`cargo add` picking compatible versions, `package.rust-version = "tbd-name-representing-currently-running-rust-toolchain"`, `cargo build` error to diagnostic).

Effects on workflows (including non-resolver behavior):
1. Latest Rust with no MSRV
  - ✅ `cargo new` setting `package.rust-version = "tbd-name-representing-currently-running-rust-toolchain"` moves most users to "Latest Rust as the MSRV" with no extra maintenance cost
  - 🟰 ~~Dealing with incompatible dependencies will have a friendlier face because the hard build error after changing dependencies is changed to a notification during update suggesting they upgrade to get the new dependency because we fallback to `rustc --version` when `package.rust-version` is unset (as a side effect of us capturing `rust-toolchain.toml`)~~
2. Latest Rust as the MSRV
  - ✅ Packages can more easily keep their MSRV up-to-date with
    - `package.rust-version = "tbd-name-representing-currently-running-rust-toolchain"` (no policy around when it is changed) though this is dependent on your Rust toolchain being up-to-date (see "Latest Rust with no MSRV" for more)
    - ~~`cargo update --update-rust-version` (e.g. when updating minor version) though this is dependent on what you dependencies are using for an MSRV~~
  - ❌ Without `cargo update --update-rust-version`, `"tbd-name-representing-currently-running-rust-toolchain"` will be more of a default path, leading to more maintainers updating their MSRV more aggressively and waiting until minors
  - ✅ Packages can more easily offer unofficial support for an MSRV due to shifting the building with MSRV-incompatible dependencies from an error to a `deny` diagnostic
3. Extended MSRV
  - ✅ Users will be able to opt-in to MSRV-compatible dependencies, in a `.cargo/config.toml`
  - ❌ Users will be frustrated that the tool knew what they wanted and didn't do it
4. Extended published MSRV w/ latest development MSRV
  - 🟰 ~~Maintainers will have to opt-in to latest dependencies, in a `.cargo/config.toml`~~
  - ✅ Verifying MSRV will no longer require juggling `Cargo.lock` files or using unstable features

### Make `CARGO_RESOLVER_PRECEDENCE=rustc` the default

Instead of `resolver = "3"` changing the behavior to `CARGO_RESOLVER_PRECEDENCE=rust-version`,
it is changed to `CARGO_RESOLVER_PRECEDENCE=rustc` where the resolver selects packages compatible with current toolchain,
matching the `cargo build` incompatible dependency error.
- We would still support `CARGO_RESOLVER_PRECEDENCE=rust-version` to help "Extended MSRV" users
- We'd drop from this proposal `cargo update [--ignore-rust-version|--update-rust-version]` as they don't make sense with this new default

This has no impact on the other proposals (`cargo add` picking compatible versions, `package.rust-version = "tbd-name-representing-currently-running-rust-toolchain"`, `cargo build` error to diagnostic).

This is an auto-adapting variant where
- If they are on the latest toolchain, they get the current behavior
- If their toolchain matches their MSRV, they get an MSRV-aware resolver

Effects on workflows (including non-resolver behavior):
1. Latest Rust with no MSRV
  - ✅ `cargo new` setting `package.rust-version = "tbd-name-representing-currently-running-rust-toolchain"` moves most users to "Latest Rust as the MSRV" with no extra maintenance cost
  - ✅ Dealing with incompatible dependencies will have a friendlier face because the hard build error after changing dependencies is changed to a notification during update suggesting they upgrade to get the new dependency because we fallback to `rustc --version` when `package.rust-version` is unset (as a side effect of us capturing `rust-toolchain.toml`)
2. Latest Rust as the MSRV
  - ✅ Packages can more easily keep their MSRV up-to-date with
    - `package.rust-version = "tbd-name-representing-currently-running-rust-toolchain"` (no policy around when it is changed) though this is dependent on your Rust toolchain being up-to-date (see "Latest Rust with no MSRV" for more)
    - ~~`cargo update --update-rust-version` (e.g. when updating minor version) though this is dependent on what you dependencies are using for an MSRV~~
  - ❌ Without `cargo update --update-rust-version`, `"tbd-name-representing-currently-running-rust-toolchain"` will be more of a default path, leading to more maintainers updating their MSRV more aggressively and waiting until minors
  - ✅ Packages can more easily offer unofficial support for an MSRV due to shifting the building with MSRV-incompatible dependencies from an error to a `deny` diagnostic
3. Extended MSRV
  - ✅ Users will be able to opt-in to MSRV-compatible dependencies, in a `.cargo/config.toml`
  - ❌ Users will be frustrated that the tool knew what they wanted and didn't do it
  - ❌ This may encourage maintainers to develop using their MSRV, reducing the quality of their experience (not getting latest lints, not getting latest cargo features like "wait for publish", etc)
4. Extended published MSRV w/ latest development MSRV
  - ❌ Maintainers will have to opt-in to ensure they get the latest dependencies in a `.cargo/config.toml`
  - ✅ Verifying MSRV will no longer require juggling `Cargo.lock` files or using unstable features

### Hard-error

Instead of *preferring* MSRV-compatible dependencies, the resolver could hard error if only MSRV-incompatible versions are available.
- `--ignore-rust-version` would need to be "sticky" in the `Cargo.lock` to avoid the next run command from rolling back the `Cargo.lock` which might be confusing because it is "out of sight; out of mind".
- To avoid `Cargo.lock` churn, we can't fallback to `rustc --version` when `package.rust-version` is not present

In addition to errors, differences from the "preference" solutions include:
- Increase the chance of an MSRV-compatible `Cargo.lock` because the resolver can backtrack on MSRV-incompatible transitive dependencies, trying alternative versions of direct dependencies
- When workspace members have different MSRVs, dependencies exclusive to a higher MSRV package can use higher versions

To get the error reporting to be of sufficient quality will require major work in a complex, high risk area of Cargo (the resolver).
This would block stabilization indefinitely.
We could adopt this approach in the future, if desired

Effects on workflows (including non-resolver behavior):
1. Latest Rust with no MSRV
  - ✅ `cargo new` setting `package.rust-version = "tbd-name-representing-currently-running-rust-toolchain"` moves most users to "Latest Rust as the MSRV" with no extra maintenance cost
  - ❌ Dealing with incompatible dependencies will have a friendlier face because the hard build error after changing dependencies is changed to a notification during update suggesting they upgrade to get the new dependency because we fallback to `rustc --version` when `package.rust-version` is unset (as a side effect of us capturing `rust-toolchain.toml`)
2. Latest Rust as the MSRV
  - ✅ Packages can more easily keep their MSRV up-to-date with
    - `package.rust-version = "tbd-name-representing-currently-running-rust-toolchain"` (no policy around when it is changed) though this is dependent on your Rust toolchain being up-to-date (see "Latest Rust with no MSRV" for more)
    - `cargo update --update-rust-version` (e.g. when updating minor version) though this is dependent on what you dependencies are using for an MSRV
  - ✅ Packages can more easily offer unofficial support for an MSRV due to shifting the building with MSRV-incompatible dependencies from an error to a `deny` diagnostic
3. Extended MSRV
  - ✅ `Cargo.lock` will Just Work for `package.rust-version`
  - ❌ Application developers using `rust-toolchain.toml` will have to duplicate that in `package.rust-version` and keep it in sync
4. Extended published MSRV w/ latest development MSRV
  - ❌ A design not been worked out to allow this workflow
  - ❌ If this is done unconditionally, then the `Cargo.lock` will change on upgrade
  - ❌ This is incompatible with per-`feature` MSRVs

# Prior art
[prior-art]: #prior-art

- Python: instead of tying packages to a particular tooling version, the community instead focuses on their equivalent of the [`rustversion` crate](https://crates.io/crates/rustversion) combined with tool-version-conditional dependencies that allow polyfills.
  - We have [cfg_accessible](https://github.com/rust-lang/rust/issues/64797) as a first step though it has been stalled
  - These don't have to be mutually exclusive solutions as conditional compilation offers flexibility at the cost of maintenance.  Different maintainers might make different decisions in how much they leverage each
  - One big difference is Python continues to support previous releases which sets a standard within the community for "MSRV" policies.
- [PHP Platform Packages](https://getcomposer.org/doc/01-basic-usage.md#platform-packages) is a more general mechanism than MSRV that allows declaring dependencies on external runtime requirements, like the interpreter version, interpreter extensions presence and version, or even whether the interpreter is 64-bit.
  - Resolves to current system
  - Can be overridden to so current system is always considered compatible
  - Not tracked in their lockfile
  - When run on an incompatible system, it will error and require running a command to re-resolve the dependencies for the current system
  - One difference is that PHP is interpreted and that their lockfile must encompass not just development dependencies but deployment dependencies.  This is in contrast to Rust which has development and deployment-build dependencies tracked with a lockfile while deployment uses OS-specific dependencies, like shared-object dependencies of ELF binaries which are not locked by their nature but instead developers rely on other technologies like docker or Nix (not even static linking can help as they that still leaves them subject to the kernel version in non-bare metal deployments).

# Unresolved questions
[unresolved-questions]: #unresolved-questions

The config field is fairly rough
- The name isn't very clear
- The values are awkward
- Should we instead just have a `resolver.rust-version = true`?
  - If we later add "resolve to toolchain" version, this might be confusing.
  - Maybe enumeration, like `resolver.rust-version = <manifest|toolchain|ignore>`?

`rust-version = "tbd-name-representing-currently-running-rust-toolchain"`'s field name is unsettled and deciding on it is not blocking for stabilization.
Ideally, we make it clear that this is not inferred from syntax,
that this is the currently running toolchain,
that we ignore pre-release toolchains,
and the name works well for resolver config if we decide to add "resolve to toolchain version" and want these to be consistent.
Some options include:
- `"tbd-name-representing-currently-running-rust-toolchain"` can imply "infer from syntactic minimum"
- `latest` can imply "latest globally (ie from rust-lang.org)
- `stable` can imply "latest globally (ie from rust-lang.org)
- `toolchain` might look weird?
- `local` implies a `remote`
- `current` is like `latest` but a little softer and might work

Resolving with an unset `package.rust-version` falls back to `rustc --version` only if its a non-pre-release.
Should we instead pick the previous stable release (e.g. nightly 1.77 would resolve for 1.76)?

Whether we report stale dependencies only on `cargo update` or on every command.
See "Syncing `Cargo.toml` to `Cargo.lock` on any Cargo command".

# Future possibilities
[future-possibilities]: #future-possibilities

## Integrate `cargo audit`

If we [integrate `cargo audit`](https://github.com/rust-lang/cargo/issues/7678),
we can better help users on older dependencies identify security vulnerabilities,
reducing the risks associated with being on older versions.

## "cargo upgrade"

As we pull [`cargo upgrade` into cargo](https://github.com/rust-lang/cargo/issues/12425),
we'll want to make it respect MSRV as well

## cargo install

`cargo install` could auto-select a top-level package that is compatible with the version of rustc that will be used to build it.

This could be controlled through a config field and
a smaller step towards this is we could stabilize the field
without changing the default away from `maximum`,
allowing people to intentionally opt-in to auto-selecting a compatible top-level paclage.

Dependency resolution could be controlled through a config field `install.resolver.precedence`,
mirroring `resolver.precedence`.
The value add of this compared to `--locked` is unclear.

See [rust-lang/cargo#10903](https://github.com/rust-lang/cargo/issues/10903) for more discussion.

**Note:** [rust-lang/cago#12798](https://github.com/rust-lang/cargo/pull/12798)
(released in 1.75) made it so `cargo install` will error upfront,
suggesting a version of the package to use and to pass `--locked` assuming the
bundled `Cargo.lock` has MSRV compatible dependencies.

## cargo publish

If you publish a library using your MSRV and MSRV-incompatible dependencies exist, the publish verification step will fail.
You can workaround this by
- Upgrading
- Running with `--no-verify`

See [rust-lang/cargo#13306](https://github.com/rust-lang/cargo/issues/13306).

## `resolver.precedence = "rust-version=<X>[.<Y>[.<Z>]]"`

We could allow people setting an effective rust-version within the config.
This would be useful for people who have a reason to not set `package.rust-version`
as well as to reproduce behavior with different Rust versions.

## rustup supporting `+msrv`

See https://github.com/rust-lang/rustup/issues/1484#issuecomment-1494058857

## Language-version lints

We could make developing with the latest toolchain with old MSRVs easier if we provided lints.
Due to accuracy of information, this might start as a clippy lint, see
[#6324](https://github.com/rust-lang/rust-clippy/issues/6324).
This doesn't have to be perfect (covering all facets of the language) to be useful in helping developers identify their change is MSRV incompatible as early as possible.

If we allowed this to bypass caplints,
then you could more easily track when a dependency with an unspecified MSRV is incompatible.

## Language-version awareness for rust-analyzer

rust-analyzer could mark auto-complete options as being incompatible with the MSRV and
automatically bump the MSRV if selected, much like auto-adding a `use` statement.

## Establish a policy on MSRV

For us to say "your MSRV should be X" would likely be both premature and would have a lot of caveats for different use cases.

With [rust-lang/cargo#13056](https://github.com/rust-lang/cargo/pull/13056),
we at least made it explicit that people should verify their MSRV.

Ideally, we'd at least facilitate people in setting their MSRV.  Some data that could help includes:
- A report of rust-versions used making requests to crates.io as determined by the user-agent
- A report of `package.rust-version` for the latest versions of packages on crates.io
- A report of `package.rust-version` for the recently downloaded versions of packages on crates.io

Once people have more data to help them in picking an MSRV policy,
it would help to also document trade-offs on whether an MSRV policy should proactive or reactive on when to bump it.

## Warn when adding dependencies with unspecified MSRVs

When adding packages without an MSRV,
its not clear whether it will work with your project.
Knowing that they haven't declared support for your toolchain version could be important,
after we've made it easier to declare an MSRV.

## Track version maintenance status on crates.io

If you `cargo add` a dependency and it says that a newer version is available but it supports a dramatically different MSRV than you,
it would be easy to assume there is a mismatch in expectations and you shouldn't use that dependency.
However, you may still be supported via an LTS but that information can only be captured in documentation which is not within the flow of the developer.

If crates.io had a mutable package and package version metadata database,
maintainers could report the maintenance status of specific versions (or maybe encode their maintenance policy),
allowing cargo to report not just whether you are on latest, but whether you are supported.
