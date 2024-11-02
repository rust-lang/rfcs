- Feature Name: `feature-unification`
- Start Date: 2024-09-11
- RFC PR: [rust-lang/rfcs#3692](https://github.com/rust-lang/rfcs/pull/3692)
- Tracking Issue: [rust-lang/cargo#14774](https://github.com/rust-lang/cargo/issues/14774)

# Summary
[summary]: #summary

Give users control over the feature unification that happens based on the packages they select.
- A way for `cargo check -p foo -p bar` to build like `cargo check -p foo && cargo check -p bar`
- A way for `cargo check -p foo` to build `foo` as if `cargo check --workspace` was used

Related issues:
- [#5210: Resolve feature and optional dependencies for workspace as a whole](https://github.com/rust-lang/cargo/issues/5210)
- [#4463: Feature selection in workspace depends on the set of packages compiled](https://github.com/rust-lang/cargo/issues/4463)
- [#8157: --bin B resolves features differently than -p B in a workspace](https://github.com/rust-lang/cargo/issues/8157)
- [#13844: The cargo build --bins re-builds binaries again after cargo build --all-targets](https://github.com/rust-lang/cargo/issues/13844)

# Motivation
[motivation]: #motivation

Today, when Cargo is building, features in dependencies are enabled based on the set of packages selected to build.
This is an attempt to balance
- Build speed: we should reuse builds between packages within the same invocation
- Ability to verify features for a given package

This isn't always ideal.

If a user is building an application, they may be jumping around the application's components which are packages within the workspace.
The final artifact is the same but Cargo will select different features depending on which package they are currently building,
causing build churn for the same set of dependencies that, in the end, will only be used with the same set of features.
The "cargo-workspace-hack" is a pattern that has existed for years
(e.g. [`rustc-workspace-hack`](https://crates.io/crates/rustc-workspace-hack))
where users have all workspace members that depend on a generated package that depends on direct-dependencies in the workspace along with their features.
Tools like [`cargo-hakari`](https://crates.io/crates/cargo-hakari) automate this process.
To allow others to pull in a package depending on a workspace-hack package as a git dependency, you then need to publish the workspace-hack as an empty package with no dependencies
and then locally patch in the real instance of it.

This also makes testing of features more difficult because a user can't just run `cargo check --workspace` to verify that the correct set of features are enabled.
This has led to the rise of tools like [cargo-hack](https://crates.io/crates/cargo-hack) which de-unify packages.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

We'll add two new modes to feature unification:

**Unify features across the workspace, independent of the selected packages**

This would be built-in support for "cargo-workspace-hack".

This would require effectively changing from
1. Resolve dependencies
2. Filter dependencies down for current build-target and selected packages
3. Resolve features

To
1. Resolve dependencies
2. Filter dependencies down for current build-target
3. Resolve features
4. Filter for selected packages

The same result can be achieved with `cargo check --workspace`,
but with fewer packages built.
Therefore, no fundamentally new "mode" is being introduced.

**Features will be evaluated for each package in isolation**

This will require building duplicate copies of build units when there are disjoint sets of features.

For example, this could be implemented as either
- Loop over the packages, resolving, and then run a build plan for that package
- Resolve for each package and generate everything into the same build plan

This is not prescriptive of the implementation but to illustrate what the feature does.
The initial implementation may be sub-optimal.
Likely, the implementation could be improved over time.

The same result can be achieved with `cargo check -p foo && cargo check -p bar`,
but with the potential for optimizing the build further.
Therefore, no fundamentally new "mode" is being introduced.

**Note:** these features do not need to be stabilized together.

##### `resolver.feature-unification`

*(update to [Configuration](https://doc.rust-lang.org/cargo/reference/config.html))*

* Type: string
* Default: "selected"
* Environment: `CARGO_RESOLVER_FEATURE_UNIFICATION`

Specify which packages participate in [feature unification](https://doc.rust-lang.org/cargo/reference/features.html#feature-unification).

* `selected`: merge dependency features from all package specified for the current build
* `workspace`: merge dependency features across all workspace members, regardless of which packages are specified for the current build
* `package`: dependency features are only considered on a package-by-package basis, preferring duplicate builds of dependencies when different sets of feature are activated by the packages.

# Drawbacks
[drawbacks]: #drawbacks

This increases entropy within Cargo and the universe at large.

As `workspace` unifcation builds dependencies the same way as `--workspace`, it has the same drawbacks as `--workspace`, including
- If a build would fail with `--workspace`, then it will fail with `workspace` unification as well.
  - For example, if two packages in a workspace enable mutually exclusive features, builds will fail with both `--workspace` and `workspace` unification.
    Officially, features are supposed to be additive, making mutually exclusive features officially unsupported.
    Instead, effort should be put towards [official mutually exclusive globals](https://internals.rust-lang.org/t/pre-rfc-mutually-excusive-global-features/19618).
- If `--workspace` would produce an invalid binary for your requirements, then it will do so with `workspace` unification as well.
  - For example, if you have regular packages and a `no_std` package in the same workspace, the `no_std` package may end up with dependnencies built with `std` features.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This is done in the config instead of the manifest:
- As this can change from run to run, this covers more use cases.
- As this fits easily into the `resolver` table, there is less design work.

We could extend this with configuration to exclude packages for the various use cases mentioned.
Supporting excludes adds environment/project configuration complexity as well as implementation complexity.

This field will not apply to `cargo install` to match the behavior of `resolver.incompatible-rust-versions`.

The `workspace` setting breaks down if there are more than one "application" in
a workspace, particularly if there are shared dependencies with intentionally
disjoint feature sets.
What this use case is really modeling is being able to tell Cargo "build package X as if its a dependency of package Y".
There are many similar use cases to this (e.g. [cargo#2644](https://github.com/rust-lang/cargo/issues/2644), [cargo#14434](https://github.com/rust-lang/cargo/issues/14434)).
While a solution that targeted this higher-level need would cover more uses cases,
there is a lot more work to do within the design space and it could end up being more unwieldy.
The solution offered in this RFC is simple in that it is just a re-framing of what already happens on the command line.

# Prior art
[prior-art]: #prior-art

[`cargo-hakari`](https://crates.io/crates/cargo-hakari) is a "cargo-workspace-hack" generator that builds a graph off of `cargo metadata` and re-implements feature unification.

[cargo-hack](https://crates.io/crates/cargo-hack) can run each selected package in a separate `cargo` invocation to prevent unification.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- How to name the config field to not block the future possibilities

# Future possibilities
[future-possibilities]: #future-possibilities

### Support in manifests

Add a related field to manifests that the config can override.

### Dependency version unification

Unlike feature unification, dependency versions are always unified across the
entire workspace, making `Cargo.lock` the same regardless of which package you
select or how you build.

This can mask minimal-version bugs.
If a version-req is lower than it needs, `-Zminimal-versions` won't resolve down to that to show the problem if another version req in the workspace is higher.
We have `-Zdirect-minimal-versions` which will error if workspace members do not have the lowest version reqs of all of the workspace but that is brittle.

If you have a workspace with multiple MSRVs, you can't verify your MSRV if you
set a high-MSRV package's version req for a dependency that invalidates the
MSRV-requirements of a low-MSRV package.

We could offer an opt-in to per-package `Cargo.lock` files.  For builds, this
could be implemented similar to `resolver.feature-unification = "package"`.

This could run into problems with
- `cargo update` being workspace-focused
- third-party updating tools

As for the MSRV-case, this would only help if you develop with the latest
versions locally and then have a job that resolves down to your MSRVs.

### Unify features in other settings

[`workspace.resolver = "2"`](https://doc.rust-lang.org/cargo/reference/resolver.html#features) removed unification from the following scenarios
- Cross-platform build-target unification
- `build-dependencies` / `dependencies` unification
- `dev-dependencies` / `dependencies` unification unless a dev build-target is enabled

Depending on how we design this, the solution might be good enough to
re-evaluate
[build-target features](https://github.com/rust-lang/rfcs/pull/3374) as we
could offer a way for users to opt-out of build-target unification.

Like with `resolver.incompatible-rust-version`, a solution for this would override the defaults of `workspace.resolver`.

`cargo hakari` gives control over `build-dependencies` / `dependencies` unification with
[`unify-target-host`](https://docs.rs/cargo-hakari/latest/cargo_hakari/config/index.html#unify-target-host):
- [`none`](https://docs.rs/hakari/0.17.4/hakari/enum.UnifyTargetHost.html#variant.None): Perform no unification across the target and host feature sets.
  - The same as `resolver = "2"`
- [`unify-if-both`](https://docs.rs/hakari/0.17.4/hakari/enum.UnifyTargetHost.html#variant.UnifyIfBoth): Perform unification across target and host feature sets, but only if a dependency is built on both the platform-target and the host.
- [`replicate-target-on-host`](https://docs.rs/hakari/0.17.4/hakari/enum.UnifyTargetHost.html#variant.ReplicateTargetOnHost): Perform unification across platform-target and host feature sets, and also replicate all target-only lines to the host.
- [`auto`](https://docs.rs/hakari/0.17.4/hakari/enum.UnifyTargetHost.html#variant.Auto) (default): select `replicate-target-on-host` if a workspace member may be built for the host (used as a proc-macro or build-dependency)

`unify-target-host` might be somewhat related to [`-Ztarget-applies-to-host`](https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#target-applies-to-host)

For Oxide `unify-target-host` reduced build units from 1900 to 1500, dramatically improving compile times, see https://github.com/oxidecomputer/omicron/pull/4535
If integrated into cargo, there would no longer be a use case for the current maintainer of `cargo-hakari` to continue maintenance.

If we supported `dev-dependencies` / `dependencies` like `resolver = "1"`, it
could help with cases like `cargo miri` where through `dev-dependencies` a
`libc` feature is enabled. preventing reuse of builds between `cargo build` and
`cargo test` for local development.

In helping this case, we should make clear that this can also break people
- `fail` injects failures into your production code, only wanting it enabled for tests
- Tests generally enabled `std` on dependencies for `no_std` packages
- We were told of use cases around private keys where `Clone` is only provided when testing but not for production to help catch the leaking of secrets
