- Feature Name: `cargo-features2`
- Start Date: 2020-05-09
- RFC PR: [rust-lang/rfcs#2957](https://github.com/rust-lang/rfcs/pull/2957)
- Cargo Issue: [rust-lang/cargo#8088](https://github.com/rust-lang/cargo/issues/8088)

# Summary

This RFC is to gather final feedback on stabilizing the new feature resolver
in Cargo. This new feature resolver introduces a new algorithm for computing
[package features][docs-old-features] that helps to avoid some unwanted
unification that happens in the current resolver. This also includes some
changes in how features are enabled on the command-line.

These changes have already been implemented and are available on the nightly
channel as an unstable feature. See the [unstable feature docs] for
information on how to test out the new resolver, and the [unstable package
flags] for information on the new flag behavior.

> *Note*: The new feature resolver does not address all of the enhancement
> requests for feature resolution. Some of these are listed below in the
> [Feature resolver enhancements](#feature-resolver-enhancements) section.
> These are explicitly deferred for future work.

[docs-old-features]: https://doc.rust-lang.org/nightly/cargo/reference/features.html
[unstable feature docs]: https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#features
[unstable package flags]: https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#package-features

# Motivation

## Feature unification

Currently, when features are computed for a package, Cargo takes the union of
all requested features in all situations for that package. This is relatively
easy to understand, and ensures that packages are only built once during a
single build. However, this has problems when features introduce unwanted
behavior, dependencies, or other requirements. The following three situations
illustrate some of the unwanted feature unification that the new resolver aims
to solve:

* Unused targets: If a dependency shows up multiple times in the resolve
  graph, and one of those situations is a target-specific dependency, the
  features of the target-specific dependency are enabled on all platforms. See
  [target dependencies](#target-dependencies) below for how this problem is
  solved.

* Dev-dependencies: If a dependency is shared as a normal dependency and a
  dev-dependency, then any features enabled on the dev-dependency will also
  show up when used as a normal dependency. This only applies to workspace
  packages; dev-dependencies in packages on registries like [crates.io] have
  always been ignored. `cargo install` has also always ignored
  dev-dependencies. See [dev-dependencies](#dev-dependencies) below for how
  this problem is solved.

* Host-dependencies: Similarly to dev-dependencies, if a build-dependency or
  proc-macro has a shared dependency with a normal dependency, then the
  features are unified with the normal dependency. See [host
  dependencies](#host-dependencies) below for how this problem is solved.

[crates.io]: https://crates.io/

## Command-line feature selection

Cargo has several flags for choosing which features are enabled during a
build. `--features` allows enabling individual features, `--all-features`
enables all features, and `--no-default-features` ensures the "default"
feature is not automatically enabled.

These are fairly straightforward when used with a single package, but in a
workspace the current behavior is limited and confusing. There are several
problems in a workspace:

* `cargo build -p other_member --features …` — The listed features are for the
  package in the current directory, even if that package isn't being built!
  This also makes it difficult or impossible to build multiple packages at
  once with different features enabled.
* `--features` and `--no-default-features` flags are not allowed in the root
  of a virtual workspace.

See [New command-line behavior](#new-command-line-behavior) below for how
these problems are solved.

# Guide-level explanation

## New resolver behavior

When the new feature resolver is enabled, features are not always unified when
a dependency appears multiple times in the dependency graph. The new behaviors
are described below.

For [target dependencies](#target-dependencies) and
[dev-dependencies](#dev-dependencies), the general rule is, if a dependency is
not built, it does not affect feature resolution. For [host
dependencies](#host-dependencies), the general rule is that packages used for
building (like proc-macros) do not affect the packages being built.

The following three sections describe the new behavior for three difference
situations.

### Target dependencies

When a package appears multiple times in the build graph, and one of those
instances is a target-specific dependency, then the features of the
target-specific dependency are only enabled if the target is currently being
built. For example:

```toml
[dependency.common]
version = "1.0"
features = ["f1"]

[target.'cfg(windows)'.dependencies.common]
version = "1.0"
features = ["f2"]
```

When building this example for a non-Windows platform, the `f2` feature will
*not* be enabled.

### dev-dependencies

When a package is shared as a normal dependency and a dev-dependency, the
dev-dependency features are only enabled if the current build is including
dev-dependencies. For example:

```toml
[dependencies]
serde = {version = "1.0", default-features = false}

[dev-dependencies]
serde = {version = "1.0", features = ["std"]}
```

In this situation, a normal `cargo build` will build `serde` without any
features. When built with `cargo test`, Cargo will build `serde` with its
default features plus the "std" feature.

Note that this is a global decision. So a command like `cargo build
--all-targets` will include examples and tests, and thus features from
dev-dependencies will be enabled.

### Host dependencies

When a package is shared as a normal dependency and a build-dependency or
proc-macro, the features for the normal dependency are kept independent of the
build-dependency or proc-macro. For example:

```toml
[dependencies]
log = "0.4"

[build-dependencies]
log = {version = "0.4", features=['std']}
```

In this situation, the `log` package will be built with the default features
for the normal dependencies. As a build-dependency, it will have the `std`
feature enabled. This means that `log` will be built twice, once without `std`
and once with `std`.

Note that a dependency shared between a build-dependency and proc-macro are
still unified. This is intended to help reduce build times, and is expected to
be unlikely to cause problems that feature unification usually cause because
they are both being built for the host platform, and are only used at build
time.

## Resolver opt-in

Testing has been performed on various projects. Some were found to fail to
compile with the new resolver. This is because some dependencies are written
to assume that features are enabled from another part of the graph. Because
the new resolver results in a backwards-incompatible change in resolver
behavior, the user must opt-in to use the new resolver. This can be done with
the `resolver` field in `Cargo.toml`:

```toml
[package]
name = "my-package"
version = "1.0.0"
resolver = "2"
```

Setting the resolver to `"2"` switches Cargo to use the new feature resolver.
It also enables backwards-incompatible behavior detailed in [New command-line
behavior](#new-command-line-behavior). A value of `"1"` uses the previous
resolver behavior, which is the default if not specified.

The value is a string (instead of an integer) to allow for possible extensions
in the future.

The `resolver` field is only honored in the top-level package or workspace, it
is ignored in dependencies. This is because feature-unification is an
inherently global decision.

If using a virtual workspace, the root definition should be in the
`[workspace]` table like this:

```toml
[workspace]
members = ["member1", "member2"]
resolver = "2"
```

For packages that encounter a problem due to missing feature declarations, it
is backwards-compatible to add the missing features. Adding those missing
features should not affect projects using the old resolver.

It is intended that `resolver = "2"` will likely become the default setting in
a future Rust Edition. See ["Default opt-in"](#default-opt-in) below for more
details.

## New command-line behavior

The following changes are made to the behavior of selecting features on the
command-line.

* Features listed in the `--features` flag no longer pay attention to the
  package in the current directory. Instead, it only enables the given
  features for the selected packages. Additionally, the features are enabled
  only if the the package defines the given features.

  For example:

      cargo build -p member1 -p member2 --features foo,bar

  In this situation, features "foo" and "bar" are enabled on the given members
  only if the member defines that feature. It is still an error if none of the
  selected packages defines a given feature.

* Features for individual packages can be enabled by using
  `member_name/feature_name` syntax. For example, `cargo build --workspace
  --feature member_name/feature_name` will build all packages in a workspace,
  and enable the given feature only for the given member.

* The `--features` and `--no-default-features` flags may now be used in the
  root of a virtual workspace.

The ability to set features for non-workspace members is not allowed, as the
resolver fundamentally does not support that ability.

The first change is only enabled if the `resolver = "2"` value is set in the
workspace manifest because it is a backwards-incompatible change. The other
changes are intended to be stabilized for everyone, as they only extend
previously invalid usage.

## `cargo metadata`

At this time, the `cargo metadata` command will not be changed to expose the
new feature resolver. The "features" field will continue to display the
features as computed by the original dependency resolver.

Properly expressing the dependency graph with features would require a number
of changes to `cargo metadata` that can add complexity to the interface. For
example, the following flags would need to be added to properly show how
features are selected:

* Workspace selection flags (`-p`, `--workspace`, `--exclude`).
* Whether or not dev-dependencies are included (`--dep-kinds`?).

Additionally, the current graph structure does not expose the host-vs-target
dependency relationship, among other issues.

It is intended that this will be addressed at some point in the future.
Feedback on desired use cases for feature information will help define the
solution. A possible alternative is to stabilize the [`--unit-graph`] flag,
which exposes Cargo's internal graph structure, which accurately indicates the
actual dependency relationships and uses the new feature resolver.

For non-parseable output, `cargo tree` will show features from the new
resolver.

[`--unit-graph`]: https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#unit-graph

# Drawbacks

There are a number of drawbacks to this approach:

* In some situations, dependencies will be built multiple times where they
  were previously only built once. This causes two problems: increased build
  times, and potentially broken builds when transitioning to the new resolver.
  It is intended that if the user wants to build a dependency once that now
  has non-unified features, they will need to add feature declarations within
  their dependencies so that they once again have the same features. The
  `cargo tree` command has been added to help the user identify and remedy
  these situations. `cargo tree -d` will expose dependencies that are built
  multiple times, and the `-e features` flag can be used to see which packages
  are enabling which features.

  Unfortunately the error message is not very clear when a feature that was
  previously assumed to be enabled is no longer enabled. Typically these
  appear in the form of unresolved paths. In testing so far, this has come up
  occasionally, but is usually fairly easy to identify what is wrong. Once
  more of the ecosystem starts using the new resolver, these errors should
  become less frequent.

* Feature unification with dev-dependencies being a global decision can result
  in some artifacts including features that may not be desired. For example, a
  project with a binary and a shared dependency that is used as a
  dev-dependency and a normal dependency. When running `cargo test` the binary
  will include the shared dev-dependency features. Compare this to a normal
  `cargo build --bin name`, where the binary will be built without those
  features. This means that if you are testing a binary with an integration
  test, you end up not testing the same thing as what is normally built.
  Changing this has significant drawbacks. Cargo's dependency graph
  construction will require fundamental changes to support this scenario.
  Additionally, it has a high risk that will cause increased build times for
  many projects that aren't affected or don't care that it may have slightly
  different features enabled.

* This adds complexity to Cargo, and adds boilerplate to `Cargo.toml`. It can
  also be confusing when switching between projects that use different
  settings. It is intended in the future that new resolver will become the
  default via the "edition" declaration. This will remove the extra
  boilerplate, and hopefully most projects will eventually adopt the new
  edition, so that there will be consistency between projects. See ["Default
  opt-in"](#default-opt-in) below for more details

* This may not cover all of the backwards-incompatible changes that we may
  want to make to the feature resolver. At this time, we do not have any
  specific enhancements planned that are backwards-incompatible, but there is
  a risk that additional enhancements will require a bump to version `"3"` of
  the resolver field, causing further ecosystem churn. Since there aren't any
  specific changes on the horizon that we know will cause problems, I am
  reluctant to force the new resolver to wait until some uncertain point in
  the future. See [Future possibilities](#future-possibilities) for a list of
  possible changes.

* The new resolver has not had widespread testing. It is unclear if it covers
  most of the concerns that motivated it, or if there are shortcomings or
  problems. It is difficult to get sufficient testing, particularly when only
  available as an unstable feature.

## Subtle behaviors

The following are behaviors that may be confusing or surprising, and are
highlighted here as potential concerns.

### Optional dependency feature names

* `dep_name/feat_name` will always enable the feature `dep_name`, even if it
  is an inactive optional dependency (such as a dependency for another
  platform). The intent here is to be consistent where features are always
  activated when explicitly written, but the *dependency* is not activated.

* `--all-features` enables features for inactive optional dependencies (but
  does not activate the *dependency*). This is consistent with `--features
  foo` enabling `foo`, even if the `foo` dependency is not activated.

Code that needs to have a `cfg` expression for a dependency of this kind
should use a `cfg` that matches the condition (like `cfg(windows)`) or use
`cfg(accessible(dep_name))` when that syntax is stabilized.

This is somewhat intertwined with the upcoming [namespaced features]. For an
optional dependency, the feature is decoupled from the activating of the
dependency itself.

### Proc-macro unification in a workspace

If there is a proc-macro in a workspace, and the proc-macro is included as a
"root" package along with other packages in a workspace (for example with
`cargo build --workspace`), then there can be some potentially surprising
feature unification between the proc-macro and the other members of the
workspace. This is because proc-macros *may* have normal targets such as
binaries or tests, which need feature unification with the rest of the
workspace.

This issue is detailed in [issue #8312].

At this time, there isn't a clear solution to this problem. If this is an
issue, projects are encouraged to avoid using `--workspace` or use `--exclude`
or otherwise avoid building multiple workspace members together. This is also
related to the [workspace unification issue].

[namespaced features]: https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#namespaced-features
[issue #8312]: https://github.com/rust-lang/cargo/issues/8312
[workspace unification issue]: https://github.com/rust-lang/cargo/issues/4463

# Rationale and alternatives

* These changes could be forced on all users without an opt-in. The amount of
  breakage is not expected to be widespread, though limited testing has
  exposed that it will happen some of the time. Generally, Cargo tries to
  avoid breaking changes that affect a significant portion of users, and we
  feel that breakage will come up often enough that an opt-in is the best
  route.

* An alternative approach would be to give the user manual control over which
  specific dependencies are unified and which aren't. A similar option would
  be feature masks. This would likely be a tedious process, whereas hopefully
  this RFC's approach is more automatic and streamlined for the common case.

# Prior art

Other tools have various ways of controlling conditional compilation, but none
are quite exactly like Cargo to our knowledge. The following is a survey of a
few tools with similar capabilities.

- [Ivy] has [module configurations][ivy-conf] for conditionally selecting
  dependencies. It also has [pluggable resolvers][ivy-resolvers].
- [Maven] has [optional dependencies][maven-opt] with the ability to express
  exclusions.
- [Gradle] has [feature variants][gradle-features], with
  [capabilities][gradle-capabilities] indicating what is provided.
  [Conflicts][gradle-conflicts] can be resolved with user-defined code.
- [Bazel] has [configurable build attributes][bazel-select] to change build
  rules on the command-line.
- Several build tools, like [make], rely on user scripting to inspect
  variables to make decisions on build settings.
- [Meson] has [optional dependencies][meson-deps] which are skipped if not
  available. [Build options][meson-opt] provide a way to set different
  settings, including enabled/disabled/auto features.
- [go] has [build constraints][go-constraints] which can conditionally include
  a file.
- [NuGet] dependencies can use the [PackageReference][nuget-reference] to
  specify conditions for inclusion.
- [Cabal] has [conditional features][cabal-conditional] to control
  configuration flags.
- [Bundler] can use arbitrary Ruby code to define conditions. [Optional
  dependency groups][bundler-optional] can be toggled by the user.
- [pip] dependencies can have [constraints][pip-constraints], and can have
  ["extras"][pip-extras] which can be enabled by dependencies. [Environment
  markers][pip-env] also provide a way to further restrict when a dependency
  is used.
- [CPAN] [dependencies][cpan-deps] use a
  requires/recommends/suggests/conflicts model. [Optional
  features][cpan-features] are also available.
- [npm] and [yarn] have optional dependencies that are skipped if they fail to
  install.
- [Gentoo Linux Portage][gentoo] has one of the most sophisticated feature
  selection capabilities of the common system packagers. Its [USE
  flags][gentoo-use] control dependencies and features.
  [Dependencies][gentoo-deps] can specify USE flag requirements.
  [REQUIRED_USE][gentoo-required-use] supports expressions for USE
  restrictions, mutually exclusive flags, etc. [Profiles][gentoo-profiles]
  provide a way to group USE flags.

[bazel-select]: https://docs.bazel.build/versions/master/configurable-attributes.html
[bazel]: https://www.bazel.build/
[bundler-optional]: https://bundler.io/guides/groups.html#optional-groups
[bundler]: https://bundler.io/
[cabal-conditional]: https://www.haskell.org/cabal/users-guide/developing-packages.html#resolution-of-conditions-and-flags
[cabal]: https://www.haskell.org/cabal/
[cpan-deps]: http://blogs.perl.org/users/neilb/2017/05/specifying-dependencies-for-your-cpan-distribution.html
[cpan-features]: https://metacpan.org/pod/CPAN::Meta::Spec#optional_features
[cpan]: https://www.cpan.org/
[gentoo-deps]: https://devmanual.gentoo.org/general-concepts/dependencies/index.html
[gentoo-profiles]: https://wiki.gentoo.org/wiki/Profile_(Portage)
[gentoo-required-use]: https://devmanual.gentoo.org/ebuild-writing/variables/#required_use
[gentoo-use]: https://wiki.gentoo.org/wiki/Handbook:X86/Working/USE
[gentoo]: https://wiki.gentoo.org/wiki/Portage
[go-constraints]: https://golang.org/pkg/go/build/#hdr-Build_Constraints
[go]: https://golang.org/
[gradle-capabilities]: https://docs.gradle.org/6.0.1/userguide/component_capabilities.html
[gradle-conflicts]: https://docs.gradle.org/current/userguide/dependency_capability_conflict.html
[gradle-features]: https://docs.gradle.org/current/userguide/feature_variants.html
[gradle]: https://gradle.org/
[ivy-conf]: http://ant.apache.org/ivy/history/latest-milestone/tutorial/conf.html
[ivy-resolvers]: https://ant.apache.org/ivy/history/latest-milestone/settings/resolvers.html
[ivy]: https://ant.apache.org/ivy/
[make]: https://www.gnu.org/software/make/manual/make.html
[maven-opt]: https://maven.apache.org/guides/introduction/introduction-to-optional-and-excludes-dependencies.html
[maven]: https://maven.apache.org/
[meson-deps]: https://mesonbuild.com/Dependencies.html
[meson-opt]: https://mesonbuild.com/Build-options.html
[meson]: https://mesonbuild.com/
[npm]: https://www.npmjs.com/
[nuget-reference]: https://docs.microsoft.com/en-us/nuget/consume-packages/package-references-in-project-files
[nuget]: https://docs.microsoft.com/en-us/nuget/
[pip-constraints]: https://setuptools.readthedocs.io/en/latest/setuptools.html#declaring-dependencies
[pip-env]: https://www.python.org/dev/peps/pep-0508/#environment-markers
[pip-extras]: https://setuptools.readthedocs.io/en/latest/setuptools.html#declaring-extras-optional-features-with-their-own-dependencies
[pip]: https://pypi.org/project/pip/
[yarn]: https://yarnpkg.com/

# Unresolved questions

None at this time.

# Motivating issues

The Cargo issue tracker contains historical context for some of the requests that
have motivated these changes:

- [#8088] Features 2.0 meta tracking issue.
- [#7914] Tracking issue for -Z features=itarget
  - [#1197] Target-specific features
  - [#2524] Conditional compilation of dependency feature based on target doesn't work
- [#7915] Tracking issue for -Z features=host_dep
  - [#2589] Build Deps getting mixed in with dependencies
  - [#4361] Shared build+target dependency crates conflate features
  - [#4866] build-dependencies and dependencies should not have features unified
  - [#5730] Features of dependencies are enabled if they're enabled in build-dependencies; breaks no_std libs
- [#7916] Tracking issue for -Z features=dev_dep
  - [#1796] Incorrect dev-dependency feature resolution
  - [#4664] Don't pass `--features` from `dev-dependencies` to `dependencies`
- [#5364] New behavior of `--feature` + `--package` combination
  - [#4106] Testing workspace package with features expects the root package to have those features
  - [#4753] Add support for --features and --no-default-features flags in workspace builds
  - [#5015] building workspaces can't use --features flag
  - [#5362] Cargo sometimes doesn't ungate crate features
  - [#6195] Testing whole workspace with features enabled in some crate(s)

[#1197]: https://github.com/rust-lang/cargo/issues/1197
[#1796]: https://github.com/rust-lang/cargo/issues/1796
[#2524]: https://github.com/rust-lang/cargo/issues/2524
[#2589]: https://github.com/rust-lang/cargo/issues/2589
[#4106]: https://github.com/rust-lang/cargo/issues/4106
[#4361]: https://github.com/rust-lang/cargo/issues/4361
[#4664]: https://github.com/rust-lang/cargo/issues/4664
[#4753]: https://github.com/rust-lang/cargo/issues/4753
[#4866]: https://github.com/rust-lang/cargo/issues/4866
[#5015]: https://github.com/rust-lang/cargo/issues/5015
[#5362]: https://github.com/rust-lang/cargo/issues/5362
[#5364]: https://github.com/rust-lang/cargo/issues/5364
[#5730]: https://github.com/rust-lang/cargo/issues/5730
[#6195]: https://github.com/rust-lang/cargo/issues/6195
[#7914]: https://github.com/rust-lang/cargo/issues/7914
[#7915]: https://github.com/rust-lang/cargo/issues/7915
[#7916]: https://github.com/rust-lang/cargo/issues/7916
[#8088]: https://github.com/rust-lang/cargo/issues/8088

# Future possibilities

## Feature resolver enhancements

The following changes are things we are thinking about, but are not in a
fully-baked state. It is uncertain if they will require backwards-incompatible
changes or not.

* Workspace feature unification. Currently the features enabled in a workspace
  depend on which workspace members are built (and those members' dependency
  tree). Sometimes projects want to ensure a dependency is only built once,
  regardless of which member included it, to avoid duplicate builds, or
  surprising changes in behavior. Sometimes projects want to ensure
  dependencies are *not* unified, since they don't want unrelated workspace
  members to affect one another. It seems likely this may require explicit
  notation to control the behavior, so it may be possible to add in a
  backwards-compatible fashion. There are also workarounds for this behavior,
  so it is not as urgent.
* Automatic features. This allows a dependency to automatically be enabled
  if it is already enabled somewhere else in the graph. [rfc#1787]
* Profile and target default features.
* Namespaced features. [rust-lang/cargo#5565]
* Mutually-exclusive features. [rust-lang/cargo#2980]
* Private and unstable features.
* And many other issues and enhancements in the Cargo tracker: [A-features]

[rust-lang/cargo#5565]: https://github.com/rust-lang/cargo/issues/5565
[rfc#1787]: https://github.com/rust-lang/rfcs/pull/1787
[rust-lang/cargo#2980]: https://github.com/rust-lang/cargo/issues/2980
[A-features]: https://github.com/rust-lang/cargo/issues?q=is%3Aopen+is%3Aissue+label%3AA-features

## Default opt-in

We are planning to make it so that in the next Rust Edition, Cargo will
automatically use the new resolver. It will assume you specify
`resolver = "2"` when a workspace specifies the next edition. This may help
reduce the boilerplate in the manifest, and make the preferred behavior the
default for new projects. Cargo has some precedent for this, as in the 2018
edition several defaults were changed. It is unclear how this would work in a
virtual workspace, or if this will cause additional confusion, so this is left
as a possibility to be explored in the future.

## Default `cargo new`

In the short term, `cargo new` (and `init`) will not set the `resolver` field.
After this feature has had some time on stable and more projects have some
experience with it, the default manifest for `cargo new` will be modified to
set `resolver = "2"`.
