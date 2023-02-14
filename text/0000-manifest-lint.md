- Feature Name: `manifest-lint`
- Start Date: 2023-02-14
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add a `[lints]` table to `Cargo.toml` to configure reporting levels for
rustc and other tool lints.

# Motivation
[motivation]: #motivation

Currently, you can configure lints through
- `#[<level>(<lint>)]` or `#![<level>(<lint>)]`, like `#[forbid(unsafe)]`
  - But this doesn't scale up with additional targets (benches, examples,
    tests) or workspaces
- On the command line, like `cargo check -- --forbid unsafe`
  - This puts the burden on the caller
- Through `RUSTFLAGS`, like `RUSTFLAGS=--forbid=unsafe cargo check`
  - This puts the burden on the caller
- In `.cargo/config.toml`'s `target.*.rustflags`
  - This couples you to the running in specific directories and not running in
    the right directory causes rebuilds
  - The cargo team has previously stated that
    [they would like to see package-specific config moved to manifests](https://internals.rust-lang.org/t/proposal-move-some-cargo-config-settings-to-cargo-toml/13336/14?u=epage)

We would like a solution that makes it easier to share across targets and
packages for all tools.

See also
- [rust-lang/rust-clippy#1313](https://github.com/rust-lang/rust-clippy/issues/1313)
- [rust-lang/cargo#5034](https://github.com/rust-lang/cargo/issues/5034)
- [EmbarkStudios/rust-ecosystem#59](https://github.com/EmbarkStudios/rust-ecosystem/issues/59)

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

A new `lints` table would be added to configure lints:
```toml
[lints]
unsafe = "forbid"
```
and `cargo` would pass these along as flags to `rustc` and `clippy`.

This would work with
[RFC 2906 `workspace-deduplicate`](https://rust-lang.github.io/rfcs/2906-cargo-workspace-deduplicate.html?highlight=2906#):
```toml
[lints]
workspace = true

[workspace.lints]
unsafe = "forbid"
```

## Documentation Updates

## The `lints` section

*as a new ["Manifest Format" entry](https://doc.rust-lang.org/cargo/reference/manifest.html#the-manifest-format)*

Override the default level of lints by assigning them to a new level in a
table, for example:
```toml
[lints]
unsafe = "forbid"
```

Supported levels include:
- `forbid`
- `deny`
- `warn`
- `allow`

**Note:** TOML does not support `:` in unquoted keys, requiring tool-specific
lints to be quoted, like
```toml
[lints]
"clippy::enum_glob_use" = "warn"
```

## The `lints` table

*as a new [`[workspace]` entry](https://doc.rust-lang.org/cargo/reference/workspaces.html#the-workspace-section)*

The `workspace.lints` table is where you define lint configuration to be inherited by members of a workspace.

Specifying a workspace lint configuration is similar to package lints.

Example:

```toml
# [PROJECT_DIR]/Cargo.toml
[workspace]
members = ["crates/*"]

[workspace.lints]
unsafe = "forbid"
```

```toml
# [PROJECT_DIR]/crates/bar/Cargo.toml
[package]
name = "bar"
version = "0.1.0"

[lints]
workspace = true
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

When parsing a manifest, cargo will resolve workspace inheritance for
`lints.workspace = true` as it does with other fields.

When running rustc, cargo will transform the lints from `lint = level` to
`--level lint` and pass them on the command line before `RUSTFLAGS`, allowing
user configuration to override package configuration.

**Note:** This reserves the lint name `workspace` to allow workspace inheritance.

# Drawbacks
[drawbacks]: #drawbacks

A concern brought up in
[rust-lang/rust-clippy#1313](https://github.com/rust-lang/rust-clippy/issues/1313)
was that this will pass lints unconditionally to the underlying tool, leading
to "undefined lint" warnings when used on earlier versions, requiring that
warning to also be suppressed, reducing its value.  However, in the "Future
possibility's", we mention direct support for tying lints to rust versions.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This could be left to `clippy.toml` but that leaves `rustc` without a solution.

`[lints]` could be `[package.lints]`, tying it to the package unlike `[patch]`
and other fields that are more workspace related.  Instead, we used
`[dependencies]` as our model.

`[lints]` could be `[lint]` but we decided to follow the precedence of `[dependencies]`.

We could support platform or feature specific settings, like with
`[lints.<target>]` or `[target.<target>.lints]` but
- There isn't a defined use case for this yet besides having support for `cfg(feature = "clippy")` or
  which does not seem high enough priority to design
  around.
- `[lints.<target>]` runs into ambiguity issues around what is a `<target>`
  entry vs a `<lint>` entry in the `[lints]` table.
- We have not yet defined semantics for sharing something like this across a
  workspace

Instead of the `[package.lints]` table being `lint = "level"`, we could organize
it around `level = ["lint", ...]` like some other linters do (like
[ruff](https://beta.ruff.rs/docs/configuration/)) but this works better for
logically organizing lints, highlighting what changed in diffs, and for
possibly adding lint-specific configuration in the future.

# Prior art
[prior-art]: #prior-art

Python
- [flake8](https://flake8.pycqa.org/en/latest/user/configuration.html)
  - Format is `level = [lint, ...]`
- [pylint](https://github.com/PyCQA/pylint/blob/main/examples/pylintrc#L402)
  - Format is `level = [lint, ...]` but the [config file is a reflection of the CLI](https://pylint.pycqa.org/en/latest/user_guide/configuration/index.html)
- [ruff](https://beta.ruff.rs/docs/configuration/)
  - Format is `level = [lint, ...]`, likely due to past precedence in ecosystem (see above)

Javascript
- [eslint](https://eslint.org/docs/latest/use/configure/rules)
  - Format is `lint = level` / `lint = [ level, additional config ]`

Go
- [golangci-lint](https://golangci-lint.run/usage/configuration/)
  - Format is `level = [lint, ...]`

# Unresolved questions
[unresolved-questions]: #unresolved-questions

How does this affect fingerprinting / recompilation and how should it?

How should we hand rustdoc lint levels or, in the future, cargo lint levels?
The current proposal takes all lints and passes them to rustc like `RUSTFLAGS`
but rustdoc uses `RUSTDOCFLAGS` and cargo would use neither.  This also starts
to get into
[user-defined tool attributes](https://rust-lang.github.io/rfcs/2103-tool-attributes.html).

# Future possibilities
[future-possibilities]: #future-possibilities

## Configurable lints

We can extend basic lint syntax:
```toml
[package.lints]
cyclomatic_complexity = "allow"
```
to support configuration, whether for cargo or the lint tool:
```toml
[package.lints]
cyclomatic_complexity = { level = "allow", rust-version = "1.23.0", threshold = 30 }
```
Where `rust-version` is used by cargo to determine whether to pass along this
lint and `threshold` is used by the tool.  We'd need to define how to
distinguish between reserved and unreserved field names.
