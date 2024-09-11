- Feature Name: `feature-unification`
- Start Date: 2024-09-11
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Allow users to control feature unifcation.

Related issues:
- [#4463: Feature selection in workspace depends on the set of packages compiled](https://github.com/rust-lang/cargo/issues/4463)
- [#8157: --bin B resolves features differently than -p B in a workspace](https://github.com/rust-lang/cargo/issues/8157)
- [#13844: The cargo build --bins re-builds binaries again after cargo build --all-targets](https://github.com/rust-lang/cargo/issues/13844)

# Motivation
[motivation]: #motivation

Today, when Cargo is building, features in dependencies are enabled baed on the set of packages selected to build.
This is an attempt to balance
- Build speed: we should reuse builds between packages within the same invocation
- Ability to verify features for a given package

This isn't always ideal.

If a user is building an application, they may be jumping around the application's components which are packages within the workspace.
The final artifact is the same but Cargo will select different features depending on which package they are currently building,
causing build churn for the same set of dependencies that, in the end, will only be used with the same set of features.
The "cargo-workspace-hack" is a pattern that has existed for years
(e.g. [`rustc-workspace-hack`](https://crates.io/crates/rustc-workspace-hack))
where users have all workspace members that depend on a generated package that depends on direct-dependemncies in the workspace along with their features.
Tools like [`cargo-hakari`](https://crates.io/crates/cargo-hakari) automate this process.
To allow others to pull in a package depending on a workspace-hack package as a git dependency, you then need to publish the workspace-hack as an empty package with no dependencies
and then locally patch in the real instance of it.

This also makes testing of features more difficult because a user can't just run `cargo check --workspace` to verify that the correct set of features are enabled.
This has led to the rise of tools like [cargo-hack](https://crates.io/crates/cargo-hack) which de-unify packages.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Explain the proposal as if it was already included in the language and you were teaching it to another Rust programmer. That generally means:

- Introducing new named concepts.
- Explaining the feature largely in terms of examples.
- Explaining how Rust programmers should *think* about the feature, and how it should impact the way they use Rust. It should explain the impact as concretely as possible.
- If applicable, provide sample error messages, deprecation warnings, or migration guidance.
- If applicable, describe the differences between teaching this to existing Rust programmers and new Rust programmers.
- Discuss how this impacts the ability to read, understand, and maintain Rust code. Code is read and modified far more often than written; will the proposed feature make code easier to maintain?

For implementation-oriented RFCs (e.g. for compiler internals), this section should focus on how compiler contributors should think about the change, and give examples of its concrete impact. For policy RFCs, this section should provide an example-driven introduction to the policy, and explain its impact in concrete terms.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

### Rust Version

We'll add two new modes to feature unifcation:

**Unify features across the workspace, independent of the selected packages**

This would be built-in support for "cargo-workspace-hack".

This would require effectively changing from
1. Resolve dependencies
2. Filter dependencies down for current target and selected packages
3. Resolve features

To
1. Resolve dependencies
2. Filter dependencies down for current target
3. Resolve features
4. Filter for selected packages

**Features will be evaluated for each package in isolation**

This will require building duplicate copies of build units when there is disjoint sets of features.

For example purposes., this could be implemented as either
- Loop over the packages, resolving, and then run a build plan for that package
- Resolve for each package and generate everything into the same build plan

This is not prescriptive of the implementation but to illustrate what the feature does.

**Note:** these features do not need to be stabilized together.

##### `resolver.feature-unification`

*(update to [Configuration](https://doc.rust-lang.org/cargo/reference/config.html))*

* Type: string
* Default: "selected"
* Environment: `CARGO_RESOLVER_FEATURE_UNIFICATION`

Specify which packages participate in [feature unification](https://doc.rust-lang.org/cargo/reference/features.html#feature-unification).

* `selected`: merge dependency features from all package specified for the current build
* `workspace`: merge dependency features across all workspace members, regardless of which packages are specified for the current build
* `package`: dependency features are only enabled for each package, preferring duplicate builds of dependencies to when different feature sets are selected

# Drawbacks
[drawbacks]: #drawbacks

This increases entropy within Cargo and the universe at large.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This is done in the config instead of the manifest:
- As this can change from run-to-run, this covers more use cases
- As this fits easily into the `resolver` table. there is less design work

This will not support exceptions for mutually exclusive features because those are officially unsupported.
Instead, effort should be put towards [official mutually exclusive globals](https://internals.rust-lang.org/t/pre-rfc-mutually-excusive-global-features/19618).

# Prior art
[prior-art]: #prior-art

[`cargo-hakari`](https://crates.io/crates/cargo-hakari) is a "cargo-workspace-hack" generator that builds a graph off of `cargo metadata` and re-implements feature unification.

[cargo-hack](https://crates.io/crates/cargo-hack) can run each selected package in a separate `cargo` invocation to prevent unification.

# Unresolved questions
[unresolved-questions]: #unresolved-questions


# Future possibilities
[future-possibilities]: #future-possibilities

Add a related field to manifests that the config can override.
