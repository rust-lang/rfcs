- Feature Name: `inherit-default-features`)
- Start Date: 2026-04-06
- RFC PR: [rust-lang/rfcs#3945](https://github.com/rust-lang/rfcs/pull/3945)
- Cargo Issue: [rust-lang/cargo#0000](https://github.com/rust-lang/cargo/issues/0000)

## Summary
[summary]: #summary

Allow disabling default features locally when inheriting a dependency.
```toml
[workspace]

[workspace.dependencies]
serde = "1"
```
```toml
[package]
name = "foo"

[dependencies]
serde = { workspace = true, default-features = false }
```

## Motivation
[motivation]: #motivation

Say you are trying to create a package in the above workspace:
```console
$ cargo new default-false
$ cd default-false
$ cargo add serde --no-default-features
error: cannot override workspace dependency with `--default-features`,
either change `workspace.dependencies.serde.default-features` or
define the dependency exclusively in the package's manifest
$ vi Cargo.toml  # manually add the above dependency
$ cargo check
error: failed to parse manifest at `default-false/Cargo.toml`

Caused by:
  error inheriting `serde` from workspace root manifest's `workspace.dependencies.serde`

Caused by:
  `default-features = false` cannot override workspace's `default-features`
```

This gets in the way of universally recommending `[workspace.dependencies]`, e.g.
- [#15180](https://github.com/rust-lang/cargo/issues/15180): `cargo new` should add the new package to `workspace.dependencies`
- [#10608](https://github.com/rust-lang/cargo/issues/10608): `cargo add` should add the dependency to `workspace.dependencies` and use `workspace = true`
- [#15578](https://github.com/rust-lang/cargo/issues/15578): lint if a dependency does not use `workspace = true`

Granted, there are other problems, including:
- Without additional tooling support, you can't tell from looking at `git log .` in a package root all of the changes that can break compatibility
- [#12546](https://github.com/rust-lang/cargo/issues/12546): cannot inherit packages renamed in `workspace.dependencies`

[RFC 2906](https://rust-lang.github.io/rfcs/2906-cargo-workspace-deduplicate.html) said:

> For now if a `workspace = true` dependency is specified then also specifying the `default-features` value is disallowed.
> The `default-features` value for a directive is inherited from the `[workspace.dependencies]` declaration,
> which defaults to true if nothing else is specified.

See also the tracking issue discussion at <https://github.com/rust-lang/cargo/issues/8415#issuecomment-727245250>

However, initial support didn't error or even emit an "unused manifest key" warning due to bugs.
In addressing this in [#11409](https://github.com/rust-lang/cargo/pull/11409),
support was added for `workspace = true, default-features = false` but in a surgical manner.
The proposed mental model for this was that the `default` feature is additive like all other features though it didn't quite accomplish that.
When inheriting `features` the package extends but does not override the workspace.
A dependency with `default-features = true` (implicitly or explicitly) is like a package with `features = ["default"]`.
So if you have a workspace dependency with `features = ["default"]` and a package with `features = []` (implicitly or explicitly),
then the end result is `features = ["default"]`.

As this left some confusing cases,
Cargo produced warnings.
These warnings were turned into hard errors for Edition 2024 in [#13839](https://github.com/rust-lang/cargo/pull/13839).0

This left us with:

| Workspace | Member    | 1.64 behavior | 1.69 behavior | 2024 edition behavior |
|:---------:|:---------:|---------------|---------------|-----------------------|
| *nothing* | *nothing* | Enabled       | Enabled       | Enabled               |
| *nothing* | df=false  | Enabled       | **Enabled, warning that it is ignored** | **Error** |
| *nothing* | df=true   | Enabled       | Enabled       | Enabled               |
| df=false  | *nothing* | Disabled      | Disabled      | Disabled              |
| df=false  | df=false  | Disabled      | Disabled      | Disabled              |
| df=false  | df=true   | Disabled      | **Enabled**   | Enabled               |
| df=true   | *nothing* | Enabled       | Enabled       | Enabled               |
| df=true   | df=false  | Enabled       | **Enabled, warning** | **Error**      |
| df=true   | df=true   | Enabled       | Enabled       | Enabled               |

*(changes bolded)*

This eventually led to [#12162](https://github.com/rust-lang/cargo/issues/12162) being opened
because the "features are additive" model prevents some valid cases from working, including:
- `workspace.dependencies.foo = "version"`: packages cannot disable default features
- `workspace.dependencies.foo = { version = "", default-features = false }`: applies to all packages, requiring `default-features = true` in all packages that do not want it

When discussing whether to allow inheriting of [`public`](https://doc.rust-lang.org/cargo/reference/unstable.html#public-dependency),
we are starting with the answer of "no" ([#13125](https://github.com/rust-lang/cargo/pull/13125)).
The thought process being that inheritance should be about consolidating shared requirements
but `public` is unlikely to be inherently a shared requirement for every dependent in a workspace.
This likely extends to both `default-features` and `features` and may be reason enough to deprecate inheriting them,
stopping altogether in a future edition.
Instead, `workspace.dependencies` should likely focus purely on inheriting of a dependency source.
Before we even get there, it needs to be possible to not specify `default-features` in `workspace.dependencies`.

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When inheriting a dependency in Edition 2024+,
instead of treating `default-features` as a hypothetical entry in `features`,
layer the package dependency on top of the workspace dependency on top of the default.

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

In pseudo-code, this would be:
```rust
let default_features = if Edition::E2024 <= package.edition {
    package_dep.default_features
        .or_else(|| workspace_dep.default_features)
        .unwrap_or(true)
} else {
    // ... existing behavior
};
```

### Documentation update

*From [Inheriting a dependency from a workspace](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#inheriting-a-dependency-from-a-workspace)*

Along with the workspace key, dependencies can also include these keys:

- optional: Note that the `[workspace.dependencies]` table is not allowed to specify optional.
- features: These are additive with the features declared in the [workspace.dependencies]
- default-features: This overrides the value set in `[workspace.dependencies]` on Edition 2024 (requires MSRV of 1.100)

Inherited dependencies cannot use any other dependency key (such as version or default-features).

## Drawbacks
[drawbacks]: #drawbacks

More churn on the meaning of `default-features`.
However, `default-features` combined with `workspace.dependencies` likely puts this in a minority case that we can likely gloss over this in most situations.

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

### Alternatives

| Workspace | Member    | 1.64 behavior | 1.69 behavior | 2024 edition behavior | Proposed |
|:---------:|:---------:|---------------|---------------|-----------------------|----------|
| *nothing* | *nothing* | Enabled       | Enabled       | Enabled               | Enabled  |
| *nothing* | df=false  | Enabled       | **Enabled, warning that it is ignored** | **Error** | **Disabled** |
| *nothing* | df=true   | Enabled       | Enabled       | Enabled               | Enabled  |
| df=false  | *nothing* | Disabled      | Disabled      | Disabled              | Disabled |
| df=false  | df=false  | Disabled      | Disabled      | Disabled              | Disabled |
| df=false  | df=true   | Disabled      | **Enabled**   | Enabled               | Enabled  |
| df=true   | *nothing* | Enabled       | Enabled       | Enabled               | Enabled  |
| df=true   | df=false  | Enabled       | **Enabled, warning** | **Error**      | **Disabled** |
| df=true   | df=true   | Enabled       | Enabled       | Enabled               | Enabled  |

*(changes bolded)*

"Workspace always wins" model
- 1.64 behavior
- Forces sharing of `default-features`
- Has confusing cases where what you see locally (`default-features = false`) is not what happens

```rust
let default_features = workspace.default_features
    .unwrap_or(true);
```

"Almost additive" model
- 1.69 behavior
- Disabling `default-features` in one package requires touching all packages
- Has confusing cases where what you see locally (`default-features = false`) is not what happens

```rust
let default_features = match (workspace.default_features, package.default_features) {
    (Some(false), Some(true)) => Some(true),
    (Some(ws), _) => Some(ws),
    (None, _) => Some(true),
};
```

"Layered" model
- **Proposed behavior**
- Allows package-level control of default-features without using workspace-level control,
  as if support doesn't exist at the workspace

```rust
let default_features = package_dep.default_features
    .or_else(|| workspace_dep.default_features)
    .unwrap_or(true);
```

"Package always wins" model
- Potential behavior if we remove dependency feature inheritance in a later edition

```rust
let default_features = package.default_features.unwrap_or(true);
```

## Prior art
[prior-art]: #prior-art

## Unresolved questions
[unresolved-questions]: #unresolved-questions

## Future possibilities
[future-possibilities]: #future-possibilities

Deprecate dependency feature inheritance, removing it in a future edition.
