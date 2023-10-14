- Feature Name: `remove-implicit-feature`
- Start Date: 2023-09-18
- RFC PR: [rust-lang/rfcs#3491](https://github.com/rust-lang/rfcs/pull/3491)
- Tracking Issue: [rust-lang/cargo#12826](https://github.com/rust-lang/cargo/issues/12826)

# Summary
[summary]: #summary

By default, cargo will treat any optional dependency as a
[feature](https://doc.rust-lang.org/cargo/reference/features.html).
As of cargo 1.60, these can be disabled by declaring a feature that activates
the optional dependency as `dep:<name>`
(see [RFC #3143](https://rust-lang.github.io/rfcs/3143-cargo-weak-namespaced-features.html)).

On the next edition, cargo will stop exposing optional dependencies as features
implicitly, requiring users to add `foo = ["dep:foo"]` if they still want it exposed.

# Motivation
[motivation]: #motivation

While implicit features offer a low overhead way of defining features,
- It is easy to overlook using `dep:` when the optional dependency is not intended to be exposed
  - Making it easy for crate authors to use the wrong syntax and be met with errors ([rust-lang/cargo#10125](https://github.com/rust-lang/cargo/issues/10125))
  - Potentially breaking people if they are later removed ([rust-lang/cargo#12687)](https://github.com/rust-lang/cargo/pull/12687))
  - Leading to confusing choices when `cargo add` lists features that look the same (e.g. `cargo add serde` showing `derive` and `serde_derive`)
  - Leading to confusing errors for callers when they reference the dependency, instead of the feature, and things don't work right
- Tying feature names to dependency names is a code smell because it ties the API to the implementation
- It requires people and tools to deal with this special case ([rust-lang/cargo#10543](https://github.com/rust-lang/cargo/issues/10543))
  - Granted, anything having to deal with old editions will still have to deal with this

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation



Updated documentation:

## Optional Dependencies
*(Replaces the [same section in the book](https://doc.rust-lang.org/cargo/reference/features.html#optional-dependencies))*

Dependencies can be marked "optional", which means they will not be compiled
by default.

For example, let's say that our 2D image processing library uses
an external package to handle GIF images. This can be expressed like this:

```toml
[features]
gif = ["dep:giffy"]

[dependencies]
giffy = { version = "0.11.1", optional = true }
```

This means that this dependency will only be included if the `gif`
feature is enabled.

> **Note**: Prior to the 202X edition, features were implicitly created for
> optional dependencies not referenced via `dep:`.

> **Note**: Another way to optionally include a dependency is to use
> [platform-specific dependencies]. Instead of using features, these are
> conditional based on the target platform.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Existing editions

As is expected for edition changes, `cargo fix --edition` will add `foo = ["dep:foo"]` features as needed.
Where undesired, users can remove these and switch their references to the
dependency from `foo` to `dep:foo`,
dealing with the [potential breaking changes](https://doc.rust-lang.org/cargo/reference/semver.html#cargo-remove-opt-dep).

Ideally, this will be accomplished by `cargo` emitting an allow-by-default
warning when parsing a workspace member's package when an optional dependency
is not referenced via `dep:` in the features
([rust-lang/cargo#9088](https://github.com/rust-lang/cargo/issues/9088))
using the planned warning control system
([rust-lang/cargo#12235](https://github.com/rust-lang/cargo/issues/12235)).
The warning will be named something like `cargo::implicit_feature` and be part
of the `cargo::rust-202X-compatibility` group.

Suggested text:
```
implicit features for optional dependencies is deprecated and will be unavailable in the 202X edition.
```
This would be machine applicable with a suggestion to add `foo = ["dep:foo"]`.  `cargo fix` would then insert this feature.

If that system is not ready in time, we can always hard code the change in `cargo fix`.

## Next edition

On the next edition, this warning will be a hard error.

Suggested text:
```
unused optional dependency `foo`.  To use it, a feature must activate it with `dep:foo` syntax
```
This could be machine applicable with a suggestion to add `foo = ["dep:foo"]`.

## Other

To help users through this, `cargo add` will be updated so that `cargo add foo
--optional` will create a `foo = ["dep:foo"]` if its not already referenced by
another features
([rust-lang/cargo#11010](https://github.com/rust-lang/cargo/issues/11010)).

# Drawbacks
[drawbacks]: #drawbacks

- Some boilerplate is needed for all features, rather than just a subset
- Extra ecosystem churn on the next edition update

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Instead of a `cargo::rust-202X-compatibility` group, we could put this in
`rust-202X-compatibility` so there is one less group to enable to prepare for
the next edition at the risk of user confusion over a "rust" lint controlling cargo.

Instead of an error, optional dependencies could be
- Make optional dependencies private features ([RFC #3487](https://github.com/rust-lang/rfcs/pull/3487))
  - Seems like the need for this would be relatively low and be less obvious
  - We could still transition to this in the future
- Allow access to the feature via `#[cfg(accessible)]` ([RFC #2523](https://rust-lang.github.io/rfcs/2523-cfg-path-version.html))

# Prior art
[prior-art]: #prior-art

# Unresolved questions
[unresolved-questions]: #unresolved-questions

# Future possibilities
[future-possibilities]: #future-possibilities
