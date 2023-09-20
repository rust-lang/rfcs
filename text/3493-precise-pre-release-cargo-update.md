- Feature Name: precise-pre-release-cargo-update
- Start Date: 2023-09-20
- RFC PR: [rust-lang/rfcs#3493](https://github.com/rust-lang/rfcs/pull/3493)

# Summary
[summary]: #summary

This RFC proposes extending `cargo update` to allow updates to pre-release versions when requested with `--precise`.
For example, a `cargo` user would be able to call `cargo update -p dep --precise 0.1.1-pre.0` as long as the version of `dep` requested by their project and its dependencies are semver compatible with `0.1.1`.
This effectively splits the notion of compatibility in `cargo`.
A pre-release version may be considered compatible when the version is explicitly requested with `--precise`.
Cargo will not automatically select that version via a basic `cargo update`.

# Motivation
[motivation]: #motivation

Pre-release crates are currently challenging to use in large projects with complex dependency trees.
For example, if a maintainer releases `dep = "0.1.1-pre.0"`.
They may ask one of their users to try the new API additions in a large project so that the user can give feedback on the release before the maintainer stabilises the new parts of the API.
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

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Consider this table where `a.b.c` is compatible with `x.y.z` and `x.y.z > a.b.c`

| Cargo.toml spec | Cargo.lock version | Target version | Selected by cargo update  | Selected by cargo update --precise  |
| --------------- | ------------------ | -------------- | ------------------------- | ----------------------------------- |
| `a.b.c`         | `a.b.c`            | `x.y.z`        | ✅                        | ✅                                  |
| `a.b.c`         | `a.b.c`            | `x.y.z-pre.0`   | ❌                        | ✅                                  |
| `a.b.c`         | `x.y.z-pre.0`       | `x.y.z-pre.1`   | ❌                        | ✅                                  |
| `a.b.c-pre.0`    | `a.b.c-pre.0`       | `a.b.c-pre.1`   | ✅¹                       | ✅                                  |
| `a.b.c-pre.0`    | `a.b.c-pre.0`       | `x.y.z`        | ✅¹                       | ✅                                  |
| `a.b.c`         | `a.b.c`            | `a.b.c-pre.0`   | ❌                        | ❌                                  |

✅: Will upgrade

❌: Will not upgrade

¹For backwards compatibility with Cargo's current behaviour (see [RFC: Precise Pre-release Deps](https://github.com/rust-lang/rfcs/pull/3263))

# Drawbacks
[drawbacks]: #drawbacks

- Pre-release versions are not easily auditable when they are only specified in the lock file.
  A change that makes use of a pre-release version may not be noticed during code review as reviewers don't always check for changes in the lock file.
- Library crates that require a pre-release version are not well supported since their lock files are ignored by their users (see [future-possibilities])

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The main alternative to this would be to accept that pre-release versions are not very usable and discourage their use.
Cargo overrides can be used instead using `[patch]`.
These provide a similar experience to pre-releases, however, they require that the library's code is somehow vendored outside of the registry, usually with git.
This can cause issues particularly in CI where jobs may have permission to fetch from a private registry but not from private git repositories.
Resolving issues around not being able to fetch pre-releases from the registry usually wastes a significant amount of time.

Another alternative would be to resolve pre-release versions in `Cargo.toml`s even when another dependency specifies a stable version.
This is explored in [future-possibilities].
This would require significant changes to the resolver since the latest compatible version would depend on the versions required by other parts of the dependency tree.
This RFC may be a stepping stone in that direction since it lays the groundwork for pre-release compatibility rules, however, I consider detailing such a change outside of the scope of this RFC.

# Prior art
[prior-art]: #prior-art

[RFC: Precise Pre-release Deps](https://github.com/rust-lang/rfcs/pull/3263) aims to solve a similar but different issue where `cargo update` opts to upgrade 
pre-release versions to new pre-releases when one is released.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

# Future possibilities
[future-possibilities]: #future-possibilities

It would be nice if dependencies could specify their requirements for pre-release versions.

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

