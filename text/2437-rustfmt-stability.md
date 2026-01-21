- Feature Name: `rustfmt_stability`
- Start Date: 2018-05-10
- RFC PR: [rust-lang/rfcs#2437](https://github.com/rust-lang/rfcs/pull/2437)
- Rust Issue: [rust-lang/rust#54504](https://github.com/rust-lang/rust/issues/54504)

## Summary
[summary]: #summary

With any luck, Rustfmt 1.0 will happen very soon. The Rust community takes promises of stability very seriously, and Rustfmt (due to being a tool as well as a library) has some interesting constraints on stability. Users should be able to update Rustfmt without hurting their workflow. Where it is used in scripts or on CI, updating Rustfmt should not cause operational errors or unexpected failure.

Some changes would clearly be non-breaking (e.g., performance improvements) or clearly breaking (e.g., removing an API function or changing formatting in violation of the specification). However, there is a large grey area of changes (e.g., changing unspecified formatting) that must be addressed so that Rustfmt can evolve without hurting users.

The goal is for formatting to only ever change when a user deliberately upgrades Rustfmt. For a project using Rustfmt, the version of Rustfmt (and thus the exact formatting) can be controlled by some artifact which can be checked-in to version control; thus all project developers and continuous integration will have the same formatting (until Rustfmt is explicitly upgraded).

I propose handling versioning internally in Rustfmt, by formatting according to the rules of previous versions and having users opt-in to breaking changes.


## Motivation
[motivation]: #motivation

Instability is annoying.

One particularly tricky use case is Rustfmt being used to check formatting in CI. Here, we are not (with the current standard installation path) in control of exactly which version of Rustfmt is run, but if formatting varies between versions then the CI check will give a false negative, which will be infuriating for contributors and maintainers (there is already evidence of this blocking the continuous use of Rustfmt).

For users running Rustfmt locally, having formatting change frequently is distracting and produces confusing diffs. It is important that the formatting done locally by a developer matches the formatting checked on the CI - if the version of Rustfmt changes and formatting changes too, then the developer could have run Rustfmt but still fail CI. Finally, a big motivator for an automated formatting tool like Rustfmt is that formatting is consistent across the community. This benefit is harmed if different projects are using different versions of Rustfmt with different formatting.

Rustfmt has a programmatic API (the RLS is a major client), the usual backwards compatibility concerns apply here.

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

If you're using Rustfmt, formatting won't change without an explicit upgrade (i.e., a major version increase). This covers all formatting to all code, subject to the following restrictions:

* using the default options
* code must compile using stable Rust
* formatting with Rustfmt is *error free*

'Formatting is *error free*' means that when Rustfmt formats a program, it only changes the formatting of the program and does not change the semantics of the program or any names. This caveat means that we can fix bugs in Rustfmt where the changed formatting cannot affect any users, because previous versions of Rustfmt could cause an error.

Furthermore, any program which depends on Rustfmt and uses it's API, or script that runs Rustfmt in any stable configuration will continue to build and run after an update.

I do expect that there will be major version increments to Rustfmt (i.e., there will be a 2.0 some day). However, I hope these are rare and infrequent. I think these can be rare because backwards compatibility is more valuable for most users than slightly better formatting. On the other hand, I think as the language evolves it is likely that preferred formatting idioms will change, and that when Rustfmt can do more (for example, better format macros or comments), users will want to take advantage of these features.

If a user uses Rustfmt in CI, I do not propose that they will always be able to update their Rust version without having to update their Rustfmt version, and that may cause some formatting changes. But, it should be a conscious decision by the user to do so, and they should not be *surprised* by formatting changes.


## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

### Background

#### Specification

Rustfmt's formatting has been specified in the [formatting RFC process](https://github.com/rust-lang-nursery/fmt-rfcs) by the Rust community style team. The guidelines are described by [RFC 2436](https://github.com/rust-lang/rfcs/pull/2436). The guide aims to be complete and precise, however, it does not aim to totally constrain how a tool formats Rust code. In particular, it allows tools some freedom in formatting 'small' instances of some items in a more compact format. It also does not totally specify the interaction of nested items, especially expressions.

For example, the guide specifies how a method call and a chain of field field access are formatted across both single and multiple lines. However, if a chain of field accesses is nested inside a method call, and the whole expression does not fit on one line, it does not specify whether the method call or the field access or both should use the multiple line form.

In terms of the implementation, there are [limitations](https://github.com/rust-lang-nursery/rustfmt#limitations) on Rustfmt 1.0; for example, Rustfmt will not format comments or many macros. The stability guidelines proposed here apply to all code, not just the code which Rustfmt can format.


#### Use cases

##### General use

Either on the command line (`rustfmt` or `cargo fmt`) or via an editor (where format-on-save is a common work flow).

Major changes to formatting are distracting and pollute diffs. Changing command line options might break scripts or editor integration.

Users can typically choose when to update (`rustup update`), but might be constrained by the toolchain they are using (i.e., they have a minimum Rustfmt version which supports the Rust version they are using). If they use Rustfmt via the RLS, then the version is dependent on the RLS version, not Rustfmt (see below).

##### CI

Any formatting change could cause erroneous CI errors. Ideally, users want to avoid a long build of Rustfmt, but relying on Rustup means getting the latest version of Rustfmt. Effectively, cannot control when an update happens. Important that developers can get the same results locally as on the CI, or it becomes impossible to land patches.

##### API clients

Can control versioning using Cargo, but there is likely to be pressure from end-users to have an up to date version. API breaking changes would cause build errors. Formatting changes might break tests.

##### Options

Rustfmt can be configured with [many options](https://github.com/rust-lang-nursery/rustfmt/blob/master/Configurations.md) from the command line and a config file (rustfmt.toml). Options can be stable or unstable. Unstable options can only be used on the nightly toolchain and require opt-in with the `--unstable-features` command line flag. All options have default values and users are strongly encouraged to use these defaults.

There is currently an unstable `required_version` option which enforces that the program is being formatted with a given version of Rustfmt, however, there is no mechanism to get the specified version.


#### Distribution and versioning

Rustfmt can be built and run [from source](https://github.com/rust-lang-nursery/rustfmt), but requires a nightly toolchain. It can be used as a library or installed via Cargo. It is versioned in the usual way (currently 0.6.0). There are two crates on crates.io - `rustfmt-nightly` is up to date and requires a nightly toolchain, `rustfmt` is deprecated.

If using Rustfmt as a tool, the recommended way to install is via Rustup (`rustup component add rustfmt-preview`). This method does not require a nightly toolchain. Versioning is linked to the Rust toolchain (Rustfmt is not meaningfully versioned outside of the Rust version). The version of Rustfmt available on the nightly channel depends on the version of the rustfmt submodule in the Rust repo. This is manually updated approximately once per week. The version of Rustfmt available on beta and stable is the version on nightly when it became beta. Updates to rustfmt on the beta channel happen occasionally.

If using Rustup, then there is no way to get a specific version of Rustfmt, only the version associated with a specific version of Rust.

A common way to use Rustfmt is in an editor via the RLS. The RLS is primarily distributed via Rustup. When installed in this way, the version of Rustfmt used is the same as if Rustfmt were installed directly via Rustup (note that this is the version in the Rust repo submodule and not necessarily the same version as indicated by the RLS's Cargo.toml).


### Definition of changes

In this section we define what constitutes different kinds of breaking change for Rustfmt.


#### API breaking change

A change that could cause a dependent crate not to build, could break a script using the executable, or breaks specification-level formatting compatibility. A formatting change in this category would be frustrating even for occasional users.

Examples:

* remove a stable option (config or command line)
* remove or change the variants of a stable option (however, changing the
  formatting *behaviour* of non-default variants is *not* a breaking change)
* change public API (usual back-compat rules), see [issue](https://github.com/rust-lang-nursery/rustfmt/issues/2639)
* change to formatting which breaks the specification
* a bug fix which changes formatting from breaking the specification to abiding by the specification

Any API breaking change will require a major version increment. Changes to formatting at this level (other than bug fixes) will require an amendment to the specification RFC

An API breaking change would cause a semver major version increment.

#### Major formatting breaking change

Any change which would change the formatting of code which was previously correctly formatted. In particular when run on CI, any change which would cause `rustfmt --check` to fail where it previously succeeded.

This only applies to formatting with the default options. It includes bug fixes, and changes at any level of detail or to any kind of code.

A major formatting breaking change would cause a semver minor version increment, however, users would have to opt-in to the change.


#### Minor formatting breaking change

These are changes to formatting which cannot cause regressions for users using default options and stable Rust. That is any change to formatting which only affects formatting with non-default options or only affects code which does not compile with stable Rust.

A minor formatting breaking change would cause a semver minor version increment.


#### Non-breaking change

These changes cannot cause breakage to any user.

Examples:

* formatting changes to code which does not compile with nightly Rust (including bug fixes where the source compiles, but the output does not or has different semantics from the source)
* a change to formatting with unstable options
* backwards compatible changes to the API
* adding an option or variant of an option
* stabilising an option or variant of an option
* performance improvements or other non-formatting, non-API changes

Such changes only require a patch version increment.


### Proposal

Dealing with API breaking changes and non-breaking changes is trivial so won't be covered here.

* Stabilise the `required_version` option (probably renamed)
* API changes are a major version increment; major and minor formatting changes are a minor formatting increment, BUT major formatting changes are opt-in with a version number, e.g, using rustfmt 1.4, you get 1.0 formatting unless you specify `required_version = 1.4`
* Each published rustfmt supports formatting using all minor versions of the major version number, e.g., rustfmt 2.4 would support `2.0, 2.1, 2.2, 2.3, 2.4`.
* Even if the API does not change, we might periodically (and infrequently) publish a major version increment to end support for old formatting versions.
* The patch version number is not taken into account when choosing how to format.
* if you want older versions, you must use Cargo to get the older version of Rustfmt and build from source.
* internally, `required_version` is supported just like other configuration options
* alternative: the version could be specified in Cargo.toml as a dev-dependency/task and passed to rustfmt


#### Publishing

Rustfmt can be used via three major channels: via Cargo, via Rustup, and via the RLS. To ensure there are no surprises between the different distribution mechanisms, we will only distribute published versions, i.e., we will not publish a Git commit which does not correspond to a release via Rustup or the RLS.

## Drawbacks
[drawbacks]: #drawbacks

We want to make sure Rustfmt can evolve and stability guarantees make that more complex. However, it is certainly a price worth paying, and we should just ensure that we can still make forwards progress on Rustfmt.


## Rationale and alternatives
[alternatives]: #alternatives

### External handling

* Major formatting changes cause a major version increment, minor formatting changes cause a minor version increment
  - QUESTION - how do we distinguish API breaking changes from major formatting changes?
* add Cargo support for specifying a rustfmt version (needs an extension to Cargo, but this is sort-of planned in any case)
  - Cargo would download and build correct version of rustfmt before running
  - Rustfmt uses unstable features (and this is hard to avoid). We'd need to find a way to permit this even when building on a stable toolchain. I think the technical solution is an easy fix in Cargo, but there would be questions about who is allowed to use that feature and how it is enabled.
* if rustup is added to Cargo, then it could download binaries as an optimisation (however, this would require significant work)
* remove `required_version` option
* QUESTION - could there be incompatabilites with the toolchain (e.g., Rustfmt at the version specified can't handle a Rust feature used in the project)? Is this just a user problem?
* QUESTION - how do we handle RLS integration? I think we'd have to call out to Rustfmt rather than compile it in, and the RLS would need to ensure the correct version via Cargo.
* alternative - rather than use Cargo, have a program dedicated to managing Rustfmt versions

Rustfmt would have to maintain a branch for every supported release and backport 'necessary' changes. Hopefully we would minimise these - there should be no security fixes, and most bug fixes would be breaking. Anyone who expects to get changes to unstable Rustfmt should be using the latest version, so we shouldn't backport unstable changes. I'm sure there would be some backports though.


### Rationale for choosing internal handling

The internal handling approach adds complexity to Rustfmt (but no worse than current options). Every bug fix or improvement would need to be gated on either the `required_version` or an unstable option.

On the other hand, all changes are internal to Rustfmt and we don't require changes to any other tools. Users would rarely need to install or build different versions of Rustfmt. Non-breaking changes get to all users quickly.

It is not clear how to integrate the external handling with Rustup, which is how many users get Rustfmt. It would also be complicated to manage branches and backports under the external handling approach.


### Other alternatives

Two alternative are spelled out above. A third alternative is to version according to semver, but not make any special effort to constrain breaking changes. This would result in either slowing down development of Rustfmt or frequent breaking changes. Due to the nature of distribution of rustfmt, that would make it effectively impossible to use in CI.


## Prior art
[prior-art]: #prior-art

Rust itself has had a very strict backwards compatibility guarantee. Rust sticks strictly to semver versions and avoids any major version increment; even the [2018 edition](https://github.com/rust-lang/rfcs/blob/master/text/2052-epochs.md) avoids a breaking change by requiring an opt-in. However, it has been possible to fix bugs without being strictly backwards compatible, due to the way Rustfmt will be used in CI, it is not clear if that will be possible for Rustfmt.

Other formatters (Gofmt, Clang Format) have not dealt with the stability/versioning issue. I believe this is possible because they are not widely used in CI and because they are fairly mature and do not change formatting much.


## Unresolved questions
[unresolved]: #unresolved-questions

Whether we want to specify the version in Cargo instead of/as well as in rustfmt.toml.
