- Feature Name: precise-pre-release-cargo-update
- Start Date: 2023-09-20
- RFC PR: [rust-lang/rfcs#3493](https://github.com/rust-lang/rfcs/pull/3493)
- Tracking Issue: [rust-lang/cargo#13290](https://github.com/rust-lang/cargo/issues/13290)

# Summary
[summary]: #summary

This RFC proposes extending `cargo update` to allow updates to pre-release versions when requested with `--precise`.
For example, a `cargo` user would be able to call `cargo update -p dep --precise 0.1.1-pre.0` as long as the version of `dep` requested by their project and its dependencies are semver compatible with `0.1.1`.
This effectively splits the notion of compatibility in `cargo`.
A pre-release version may be considered compatible when the version is explicitly requested with `--precise`.
Cargo will not automatically select that version via a basic `cargo update`.

One way to think of this is that we are changing from the version
requirements syntax requiring opt-in to match pre-release of higher versions to
the resolver ignoring pre-releases like yanked packages, with an override flag.

# Motivation
[motivation]: #motivation

Today, version requirements ignore pre-release versions by default,
so `1.0.0` cannot be used with `1.1.0-alpha.1`.
Specifying a pre-release in a version requirement has two affects
- Specifies the minimum compatible pre-release.
- Opts-in to matching version requirements (within a version)

However, coupling these concerns makes it difficult to try out pre-releases
because every dependency in the tree has to opt-in.
For example, a maintainer asks one of their users to try new API additions in
`dep = "0.1.1-pre.0"` in a large project so that the user can give feedback on
the release before the maintainer stabilises the new parts of the API.
Unfortunately, since `dep = "0.1.0"` is a transitive dependency of several dependencies of the large project, `cargo` refuses the upgrade, stating that `0.1.1-pre.0` is incompatible with `0.1.0`.
The user is left with no upgrade path to the pre-release unless they are able to convince all of their transitive uses of `dep` to release pre-releases of their own.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When Cargo considers `Cargo.toml` requirements for dependencies it always favours selecting stable versions over pre-release versions.
When the specification is itself a pre-release version, Cargo will always select a pre-release.
Cargo is unable to resolve a project with a `Cargo.toml` specification for a pre-release version if any of its dependencies request a stable release.

If a user does want to select a pre-release version they are able to do so by explicitly requesting Cargo to update to that version.
This is done by passing the `--precise` flag to Cargo.
Cargo will refuse to select pre-release versions that are "incompatible" with the requirement in the projects `Cargo.toml`.
A pre-release version is considered compatible for a precise upgrade if its major, minor, and patch versions are compatible with the requirement, ignoring its pre-release version.
`x.y.z-pre.0` is considered compatible with `a.b.c` when requested `--precise`ly if `x.y.z` is semver compatible with `a.b.c` and `a.b.c` `!=` `x.y.z`.

Consider a `Cargo.toml` with this `[dependencies]` section

```
[dependencies]
example = "1.0.0"
```

It is possible to update to `1.2.0-pre.0` because `1.2.0` is semver compatible with `1.0.0`

```
> cargo update -p example --precise 1.2.0-pre.0
    Updating crates.io index
    Updating example v1.0.0 -> v1.2.0-pre.0
```

It is not possible to update to `2.0.0-pre.0` because `2.0.0` is not semver compatible with `1.0.0`

```
> cargo update -p example --precise 2.0.0-pre.0
    Updating crates.io index
error: failed to select a version for the requirement `example = "^1"`
candidate versions found which didn't match: 2.0.0-pre.0
location searched: crates.io index
required by package `tmp-oyyzsf v0.1.0 (/home/ethan/.cache/cargo-temp/tmp-OYyZsF)`
```

| Cargo.toml     | Cargo.lock    | Desired       | Selectable w/o `--precise` | Selectable w/ `--precise`  |
| ---------------| --------------| --------------| -------------------------- | ---------------------------|
| `^1.0.0`       | `1.0.0`       | `1.0.0-pre.0` | ❌                         | ❌                         |
| `^1.0.0`       | `1.0.0`       | `1.2.0`       | ✅                         | ✅                         |
| `^1.0.0`       | `1.0.0`       | `1.2.0-pre.0` | ❌                         | ✅                         |
| `^1.0.0`       | `1.2.0-pre.0` | `1.2.0-pre.1` | ❌                         | ✅                         |
| `^1.2.0-pre.0` | `1.2.0-pre.0` | `1.2.0-pre.1` | ✅¹                        | ✅                         |
| `^1.2.0-pre.0` | `1.2.0-pre.0` | `1.3.0`       | ✅¹                        | ✅                         |

✅: Will upgrade

❌: Will not upgrade

¹This behaviour is considered by some to be undesirable and may change as proposed in [RFC: Precise Pre-release Deps](https://github.com/rust-lang/rfcs/pull/3263).
This RFC preserves this behaviour to remain backwards compatible.
Since this RFC is concerned with the behaviour of `cargo update --precise` changes to bare `cargo update` made in future RFCs should have no impact on this proposal.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Version requirements

Version requirement operator semantics will change to encompass pre-release versions, compared to before where the presence of pre-release would change matching modes.
This will also better align with the mathematical properties associated with some of the operators (see the closed [RFC 3266](https://github.com/rust-lang/rfcs/pull/3266)).

So before,
```
1.2.3  -> ^1.2.3 -> >=1.2.3, <2.0.0 (with implicit holes excluding pre-release versions)
```
would become
```
1.2.3  -> ^1.2.3 -> >=1.2.3, <2.0.0-0
```
Note that the old syntax implicitly excluded `2.0.0-<prerelease>` which we have have to explicitly exclude by referencing the smallest possible pre-release version of `-0`.

This change applies to all operators.

## Dependency Resolution

The intent is to mirror the behavior of yanked today.

When parsing a `Cargo.lock`, any pre-release version would be tracked in an allow-list.
When resolving, we would exclude from consideration any pre-release version unless:
- It is in the allow-list
- It matches the version requirement under the old pre-release version requirement semantics.

## `cargo update`

The version passed in via `--precise` would be added to the allow-list.

**Note:** overriding of yanked via this mechanism is not meant to be assumed to be a part of this proposal.
Support for selecting yanked with `--precise` should be decided separately from this RFC, instead see [rust-lang/cargo#4225](https://github.com/rust-lang/cargo/issues/4225)

## [`semver`](https://crates.io/crates/semver)

`cargo` will need both the old and new behavior exposed.
To reduce risk of tools in the ecosystem unintentionally matching pre-releases (despite them still needing an opt-in),
it might be reasonable for the
`semver`
package to offer this new matching behavior under a different name
(e.g. `VersionReq::matches_prerelease` in contrast to the existing `VersionReq::matches`)
(also avoiding a breaking change).
However, we leave the exact API details to the maintainer of the `semver` package.

# Drawbacks
[drawbacks]: #drawbacks

- Pre-release versions are not easily auditable when they are only specified in the lock file.
  A change that makes use of a pre-release version may not be noticed during code review as reviewers don't always check for changes in the lock file.
- Library crates that require a pre-release version are not well supported since their lock files are ignored by their users (see [future-possibilities])
- This is an invasive change to cargo with a significant risk for bugs.  This also extends out to packages in the ecosystem that deal with dependency versions.
- There is a risk that error messages from the resolver may be negatively affected and we might be limited in fixes due to the resolver's current design.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## One-time opt-in

With this proposal, pre-release is like yanked:
having `foo@1.2.3-alpha.0` in your `Cargo.lock` does not implicitly mean you can use `foo@1.2.3-alpha.1`.
To update within pre-releases, you'll have to use `--precise` again.

Instead, the lockfile could identify that `foo@1.2.3-alpha.0` is a pre-release allow updating to any `1.2.3` pre-release.
`cargo update` focuses on compatible updates and pre-releases aren't necessarily compatible with each other
(see also [RFC: Precise Pre-release Deps](https://github.com/rust-lang/rfcs/pull/3263)).
Alternatively, in [future-possibilities] is `cargo update -p foo --allow-prerelease` which would be an explicit way to update.

## Use overrides

Cargo overrides can be used instead using `[patch]`.
These provide a similar experience to pre-releases, however, they require that the library's code is somehow vendored outside of the registry, usually with git.
This can cause issues particularly in CI where jobs may have permission to fetch from a private registry but not from private git repositories.
Resolving issues around not being able to fetch pre-releases from the registry usually wastes a significant amount of time.

## Extend `[patch]`

It could be possible to build upon `[patch]` to [allow it to use crates published in the registry](https://github.com/rust-lang/cargo/issues/9227).
This could be combined with [version overrides](https://github.com/rust-lang/cargo/issues/5640) to pretend that the pre-release crate is a stable version.

My concern with this approach is that it doesn't introduce the concept of compatible pre-releases.
This would allow any version to masquerade as another.
Without the concept of compatible pre-releases there would be no path forward towards being able to express pre-release requirements in library crates.
This is explored in [future-possibilities].

## Change the version in `Cargo.toml` rather than `Cargo.lock` when using `--precise`

This [accepted proposal](https://github.com/rust-lang/cargo/issues/12425) allows cargo to update a projects `Cargo.toml` when the version is incompatible.

The issue here is that cargo will not unify a pre-release version with a stable version.
If the crate being updated is used pervasively this will more than likely cause a resolver error.
This makes this alternative unfit for our [motivation].

The [accepted proposal](https://github.com/rust-lang/cargo/issues/12425) is affected by this RFC,
insofar as it will not update the `Cargo.toml` in cases when the pre-release can be considered compatible for upgrade in `Cargo.lock`.

## Pre-releases in `Cargo.toml`

Another alternative would be to resolve pre-release versions in `Cargo.toml`s even when another dependency specifies a stable version.
This is explored in [future-possibilities].
This would require significant changes to the resolver since the latest compatible version would depend on the versions required by other parts of the dependency tree.
This RFC may be a stepping stone in that direction since it lays the groundwork for pre-release compatibility rules, however, I consider detailing such a change outside of the scope of this RFC.

# Prior art
[prior-art]: #prior-art

[RFC: Precise Pre-release Deps](https://github.com/rust-lang/rfcs/pull/3263) aims to solve a similar but different issue where `cargo update` opts to upgrade 
pre-release versions to new pre-releases when one is released.

Implementation-wise, this is very similar to how yanked packages work.
- Not selected under normal conditions
- Once its in the lockfile, that gets respected and stays in the lockfile

The only difference being that `--precise` does not allow overriding the "ignore yank" behavior
(though [it is desired by some](https://github.com/rust-lang/cargo/issues/4225)).

For `--precise` forcing a version through, we have precedence in
[an approved-but-not-implemented proposal](https://github.com/rust-lang/cargo/issues/12425)
for `cargo update --precise` for incompatible versions to force its way
through by modifying `Cargo.toml`.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

# Version ranges with pre-release upper bounds

[As stated earlier](#reference-level-explanation), this RFC proposes that semver version semantics change to encompass pre-release versions.

```
^1.2.3 -> >=1.2.3, <2.0.0-0
```

The addition of an implicit `-0` excludes `2.0.0-<prerelease>` releases.
This transformation will also be made when the user explicitly specifies a multiple-version requirement range.

```
>=1.2.3, <2.0.0 -> >=1.2.3, <2.0.0-0
```

This leaves a corner case for `>=0.14-0, <0.14.0`.
Intuitively the user may expect this range to match all `0.14` pre-releases, however, due to the implicit `-0` on the second version requirement, this range will not match any versions.
There are two ways we could go about solving this.

- Only add the implicit `-0` to the upper bound if there is no pre-release on the lower bound.
- Accept the existence of this unexpected behaviour.

Either way, it may be desirable to introduce a dedicated warning for this case.

# Future possibilities
[future-possibilities]: #future-possibilities

## Pre-release dep "allows" pre-release everywhere

It would be nice if cargo could unify pre-release version requirements with stable versions.

Take for example this dependency tree.

```
example
├── a ^0.1.0
│   └── b =0.1.1-pre.0
└── b ^0.1.0
```

Since crates ignore the lock files of their dependencies there is no way for `a` to communicate with `example` that it requires features from `b = 0.1.1-pre.0` without breaking `example`'s direct dependency on `b`.
To enable this we could use the same concept of compatible pre-releases in `Cargo.toml`, not just `Cargo.lock`.
This would require that pre-releases are specified with `=` and would allow pre-release versions to be requested anywhere within the dependency tree without causing the resolver to throw an error.

## `--allow-prerelease`

Instead of manually selecting a version with `--precise`, we could support `cargo update --package foo --allow-prerelease`.

If we made this flag work without `--package`, we could the extend it also to `cargo generate-lockfile`.
