- Feature Name: `cargo_target_features`
- Start Date: 2023-01-20
- RFC PR: [rust-lang/rfcs#3374](https://github.com/rust-lang/rfcs/pull/3374)
- Tracking Issue: [rust-lang/cargo#0000](https://github.com/rust-lang/cargo/issues/0000)

# Summary
[summary]: #summary

This adds a new `enable-features` field to [Cargo Targets](https://doc.rust-lang.org/cargo/reference/cargo-targets.html#configuring-a-target) which forces a [feature](https://doc.rust-lang.org/cargo/reference/features.html) to be enabled if the target is being built.

# Motivation
[motivation]: #motivation

There are several situations where a non-library target must have a particular feature enabled for it to work correctly.
Although it is possible to manually enable these features on the command-line, this can make it awkward to use.
This RFC adds a mechanism to make it easier to work with these targets.

Some example use cases are:

1. Binaries that need additional dependencies like command-line processing libraries, or logging functionality.
2. Examples illustrating how to use a library with a specific feature enabled.
3. Benchmarks which require a separate benchmarking library, but you don't want to pay the cost of building that library when not running benchmarks.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The `enable-features` field can be added to a target in `Cargo.toml` to specify features or dependencies that will be automatically enabled whenever the target is being built.
For example:

```toml
[package]
name = "myproject"
version = "0.1.0"
edition = "2021"

[[example]]
name = "fetch-http"
enable-features = ["networking"]

[[bin]]
name = "myproject"
enable-features = ["dep:clap"]

[dependencies]
clap = { version = "4.0.26", optional = true }

[features]
networking = []
```

When using `myproject` as a library dependency, by default the `networking` feature and `clap` will not be enabled.
When building the binary as in `cargo install myproject` or `cargo build`, the `clap` dependency will automatically be enabled allowing the binary to be built correctly.
Similarly, `cargo build --example fetch-http` will enable the `networking` feature so that the example can be built.

This field can be specified for any Cargo Target except the `[lib]` target.

> **Note**: Because features and dependencies are package-wide, using `enable-features` does not narrow the scope of the features or dependencies to the specific target.
> The features and dependencies will be enabled for all targets.

# Implementation Details

## Feature resolver and selection

Before doing dependency and feature resolution, Cargo will need to determine which features are being force-enabled by the targets selected on the command-line.
These additional features will be combined with any features selected on the command-line before calling the resolver.

Unfortunately this selection of targets is quite complex.
This may require some potentially invasive refactoring, as Cargo currently chooses the targets to select after resolution is done (including a separate phase for implicit binaries used for integration tests).

## Hidden `dep:` dependencies

`enable-features` should allow the use of `dep:` to enable an optional dependency.
The use of `dep:` should behave the same as-if it was specified in the `[features]` table.
That is, it should suppress the creation of the implicit feature of the same name.
This may require some challenging changes to the way the features table is handled,
as the corresponding code has no knowledge about the manifest.

For the purpose of generating the lock file, the dependency resolver will need to assume that all optional dependencies of the workspace packages are enabled.
Currently this is implicitly done by assuming `--all-features` is passed.
However, if a `enable-features` field specifies a `dep:` dependency, and nothing else enables that dependency, then the resolver will not be able to see that it is enabled.
This may require some changes to differentiate between the use of explicit and implicit `--all-features`.

## Artifact dependencies

When depending on a binary [artifact dependency](https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#artifact-dependencies), Cargo should enable any `enable-features` features of that binary.
This may require plumbing that information into the index so that the dependency and feature resolvers can determine that field is being set.
This RFC does not propose a specific plan here, but that it should eventually be supported.

## Relationship with `required-features`

[`required-features`](https://doc.rust-lang.org/cargo/reference/cargo-targets.html#the-required-features-field) is a very similar field to `enable-features`, but works in a different way.
It essentially says that the target will not be built if the feature is not already enabled.

There are legitimate use cases where `required-features` is more appropriate than `enable-features`.
For example, there may be examples or tests that only work with a specific feature enabled.
However, the project may not want to make those examples or tests automatically build when running `cargo test`.
The feature may be expensive to build, or may require specific components on the system to be installed.
In these situations, `required-features` may be the appropriate way to conditionally include a target.

Documentation will need to emphasize the difference between these seemingly similar options.

## Other cargo command behavior

[`cargo metadata`](https://doc.rust-lang.org/cargo/commands/cargo-metadata.html) and [`cargo tree`](https://doc.rust-lang.org/cargo/commands/cargo-tree.html) will behave as-if they are ignoring the `enable-features` fields.
These commands do not have a concept of targets being built, nor do they have any options for selecting them.

When using those commands with `--all-features`, any hidden `dep:` dependencies that are only enabled via `enable-features` will be included.

# Drawbacks
[drawbacks]: #drawbacks

* This adds additional complexity to `Cargo.toml` potentially making it more difficult to understand.
* Users may be easily confused between the difference of `enable-features` and `required-features`.
  Hopefully clear and explicit documentation may help avoid some of that confusion.
  The error messages with `required-features` may also be extended to mention `enable-features` as an alternative (as well as other situations like `cargo install` failing to find a binary).
* It may not be clear that features are unified across all targets.
  This may particularly come into play where it may not be clear that an optional dependency suddenly becomes *available* to all the other targets when the `enable-features` causes it to be included.
  A similar situation arises with dev-dependencies, where a user may get confused when referencing a dev-dependency outside of a `#[cfg(test)]` block or module, which causes an error.
* This may not have the same clarity as explicit dependencies as proposed in [RFC 2887](https://github.com/rust-lang/rfcs/pull/2887).
* There may be increased confusion and complexity regarding the interaction of various cargo commands and this feature (such as `cargo tree` mentioned above which has no concept of cargo target selection).
* There may be confusion about the interaction with `--no-default-features`.
  `--no-default-features` will continue to only affect the `default` feature.
  Features enabled by `enable-features` will be enabled even with `--no-default-features`.
  This may be confusing and should be mentioned in the documentation.
  * There are some alternatives to avoiding a target from being built, such as not including it in the CLI options, using `required-features` instead, or disabling it with fields such as `test=false`.
    However, not all of these options may be satisfying.
* This may add significant complexity to Cargo's implementation.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Change the `required-features` behavior

* `required-features` could be changed to behave the same as `enable-features` described in this RFC (possibly over an Edition).
However, as outlined in the [Relationship with `required-features`](#relationship-with-required-features) section, there are some use cases where the present behavior of `required-features` is desirable.
This could also lead to a breaking change for some projects if it started building targets that were previously not included.
* Instead of adding a separate field that lists features, a `force-enable-features = true` field could be added to change the behavior of `required-features` to have the behavior explained in this RFC.
  That might be less confusing, but would prevent the ability to have both behaviors at the same time.
* Only the situation where `required-features` generates an error could be changed to implicitly enable the missing features.
  This would likely make `required-features` less annoying to work with, but doesn't help for use cases like running `cargo test` where you have specific tests or examples that you want to be automatically included (where `required-features` simply makes them silently excluded).
* A new CLI argument could be added to change the behavior of `required-features` to behave the same as `enable-features`, avoiding the need to add `enable-features`.
  This is viable, but the goal of this RFC is to make it as easy as possible to work with the cargo targets without requiring additional CLI arguments.

## Alternate workflows

* Instead of using `enable-features`, developers can be diligent in passing the appropriate `--features` options on the command-line when building their projects, possibly using `required-features` to ensure they only get built in the correct scenarios.
  The intent of this RFC is to make that process easier and more seamless.
* Users can set up [aliases](https://doc.rust-lang.org/cargo/reference/config.html#alias) which pass in the feature flags they want to enable.
  This can help with a development workflow, but requires more documentation and education, and doesn't help with some commands like a remote `cargo install`.
* Developers can organize their project in a [Cargo Workspace](https://doc.rust-lang.org/cargo/reference/workspaces.html) instead of using multiple targets within a single package.
  This allows customizing dependencies and other settings within each package.
  Workspaces can add some more overhead for managing multiple packages, but offer a lot more flexibility.
  Instead of implementing this RFC, we could put more work into making workspaces easier to use, which could benefit a larger audience.
  However, it is not clear if it is feasible to make workspaces work as effortlessly compared to targets.
  Also, there is likely more work involved to get workspaces on the same level.

## Alternate designs

* [RFC 2887](https://github.com/rust-lang/rfcs/pull/2887) proposes being able to add dependency tables directly in a target definition.
  It is intended that this RFC using `enable-features` will hopefully be easier to implement, make it easier to reuse a dependency declaration across multiple targets, and allow working with features that are not related to dependencies.
* [RFC 3020](https://github.com/rust-lang/rfcs/pull/3020) proposes an enhancement similar to this RFC.

## Other alternative considerations

* [cargo#1982](https://github.com/rust-lang/cargo/issues/1982) is the primary issue requesting the ability to set per-target dependencies, and contains some discussion of the desired use cases.
* Other names may be considered for the field `enable-features`, such as `forced-features`, `force-enable-features`, etc.

# Prior art
[prior-art]: #prior-art

> NOTE: These could use vetting by people more familiar with these tools.
> More examples are welcome.

* Swift has the ability to specify dependencies within a target definition (see [SE-0226](https://github.com/apple/swift-evolution/blob/main/proposals/0226-package-manager-target-based-dep-resolution.md) and [Target Dependency](https://docs.swift.org/package-manager/PackageDescription/PackageDescription.html#target-dependency)).
  These can also specify explicit dependencies on artifacts of other targets within the package.
* Go can specify dependencies directly in the source of a module.
* Many other tools do not have a similar concept as Cargo targets, or if they have something similar, they do not have a way to specify dependencies or other settings per target.
