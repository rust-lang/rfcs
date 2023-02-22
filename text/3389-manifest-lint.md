- Feature Name: `manifest-lint`
- Start Date: 2023-02-14
- RFC PR: [rust-lang/rfcs#3389](https://github.com/rust-lang/rfcs/pull/3389)
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
- On the command line, like `cargo clippy -- --forbid unsafe`
  - This puts the burden on the caller
- Through `RUSTFLAGS`, like `RUSTFLAGS=--forbid=unsafe cargo clippy`
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
- [Proposal: Cargo Lint configuration](https://internals.rust-lang.org/t/proposal-cargo-lint-configuration/9135)

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

A new `lints` table would be added to configure lints:
```toml
[lints.rust]
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

Override the default level of lints from different tools by assigning them to a new level in a
table, for example:
```toml
[lints.rust]
unsafe = "forbid"
```

This is short-hand for:
```toml
[lints.rust]
unsafe = { level = "forbid", priority = 0 }
```

`level` corresponds to the lint levels in `rustc`:
- `forbid`
- `deny`
- `warn`
- `allow`

`priority` is a signed value that controls which lints override other lints:
- lower (particularly negative) numbers have lower priority, being overridden
  by higher numbers, and shows up first on the command-line to tools like
  `rustc`

To know which table under `[lints]` a particular lint belongs under, it is the part before `::` in the lint
name.  If there isn't a `::`, then the tool is `rust`.  For example a warning
about `unsafe` would be `lints.rust.unsafe` but a lint about
`clippy::enum_glob_use` would be `lints.clippy.enum_glob_use`.

## The `lints` table

*as a new [`[workspace]` entry](https://doc.rust-lang.org/cargo/reference/workspaces.html#the-workspace-section)*

The `workspace.lints` table is where you define lint configuration to be inherited by members of a workspace.

Specifying a workspace lint configuration is similar to package lints.

Example:

```toml
# [PROJECT_DIR]/Cargo.toml
[workspace]
members = ["crates/*"]

[workspace.lints.rust]
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
`lints.workspace = true` as it does with basic fields, when `workspace` is
present, no other fields are allowed to be present.  This precludes having the
package override the workspace on a lint-by-lint basis.

cargo will contain a mapping of tool to underlying command (e.g. `rust` to
`rustc`, `clippy` to `rustc` when clippy is the driver, `rustdoc` to
`rustdoc`).  When running the underlying command, cargo will transform the
lints from `lint = level` to `--level lint`, sort them by priority and then
lint name, and pass them on the command line before other configuration,
`RUSTFLAGS`, allowing user configuration to override package configuration.
These flags will be fingerprinted so changing them will cause a rebuild only
for the commands where they are used.

Initially, the only supported tools will be:
- `rust`
- `clippy`
- `rustdoc`

Addition of third-party tools would fall under their
[attributes for tools](https://github.com/rust-lang/rust/issues/44690).

**Note:** This reserves the tool name `workspace` to allow workspace inheritance.

# Drawbacks
[drawbacks]: #drawbacks

There has been some user/IDE confusion about running commands like `rustfmt`
directly and expecting them to pick up configuration only associated with their
higher-level cargo-plugins despite that configuration (like `package.edition`)
being cargo-specific.  By baking the configuration for rustc, rustdoc, and
clippy directly into cargo, we will be seeing more of this.  A hope is that
this will actually improve with this RFC.  Over time, tools will need to switch
to the model of running `cargo` to get confuguratio in response to this RFC.
As for users, if a tool's primary configuration is in `Cargo.toml`, that will
provide a strong coupling with `cargo` in users minds as compared to using an
external configuration file and overlooking the one or two fields read from
`Cargo.toml`.

As this focuses on lints, this leaves out first-party tools that need
configuration but aren't linters, namely `rustfmt`, leading to an inconsistent
experience if `clippy.toml` goes away.

A concern brought up in
[rust-lang/rust-clippy#1313](https://github.com/rust-lang/rust-clippy/issues/1313)
was that this will pass lints unconditionally to the underlying tool, leading
to "undefined lint" warnings when used on earlier versions, requiring that
warning to also be suppressed, reducing its value.  However, in the "Future
possibility's", we mention direct support for tying lints to rust versions.

This does not allow sharing lints across workspaces.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This could be left to `clippy.toml` but that leaves `rustc` without a solution.

`[lints]` could be `[package.lints]`, tying it to the package unlike `[patch]`
and other fields that are more workspace related.  Instead, we used
`[dependencies]` as our model.

`[lints]` could be `[lint]` but we decided to follow the precedence of `[dependencies]`.

Instead of `<tool>.<lint>`, we could use `<tool>::<lint>` (e.g.
`"clipp::enum_glob_use"` instead of `clippy.enum_glob_use`), like in the
diagnostic messages.  This would make it easier to copy/paste lint names but it
will requiring quoting the keys and is more difficult to add tool-level
configuration in the future.

We could possibly extend this new field to `rustfmt` by shifting the focus from
"lints" to "rules" (see
[eslint](https://eslint.org/docs/latest/use/configure/rules)).  However, the
more we generalize this field, the fewer assumptions we can make about it.  On
one extreme is `package.metadata` which is so free-form we can't support it
with workspace inheritance.  A less extreme example is if we make the
configuration too general, we would preclude the option of supporting
per-package overrides as we wouldn't know enough about the shape of the data to
know how to merge it.  There is likely a middle ground that we could make work
but it would take time and experimentation to figure that out which is at odds
with trying to maintain a stable file format.  Another problem with `rules` is
that it is divorced from any context.  In eslint, it is in an eslint-specific
config file but a `[rules]` table is not a clear as a `[lints]` table as to
what role it fulfills.

We could support platform or feature specific settings, like with
`[lints.<target>]` or `[target.<target>.lints]` but
- There isn't a defined use case for this yet besides having support for `cfg(feature = "clippy")` or
  which does not seem high enough priority to design
  around.
- `[lints.<target>]` runs into ambiguity issues around what is a `<target>`
  entry vs a `<lint>` entry in the `[lints]` table.
- We have not yet defined semantics for sharing something like this across a
  workspace

Instead of the `[lints]` table being `lint = "level"`, we could organize
it around `level = ["lint", ...]` like some other linters do (like
[ruff](https://beta.ruff.rs/docs/configuration/)) but this works better for
logically organizing lints, highlighting what changed in diffs, and for
possibly adding lint-specific configuration in the future.

## Workspace Inheritance

Instead of using workspace inheritance for `[lint]`, we could make it
workspace-level configuration, like `[patch]` which is automatically applied to
all workspace members.  However, `[patch]` and friends are because they affect
the resolver / `Cargo.toml` and so they can only operate at the workspace
level.  `[lints]` is more like `[dependencies]` in being something that applies
at the package level but we want shared across workspaces.

Instead of traditional workspace inheritance where there is a single value to
inherit with `workspace = true`, we could have `[workspace.lints.<preset>]`
which defines presets and the user could do `lints.<preset> = true`.  The user
could then name them as they wish to avoid collision with rustc lints.

## Lint Predence

The priority field is meant to allow mimicing
- `-Aclippy::all -Wclippy::doc_markdown`
- `-D future-incompatible -A semicolon_in_expressions_from_macros`

We can't order lints based on the level as which we want first is dependent on the context.

We can't rely on the order of the keys in the table as that is undefined in TOML.

We could use an array instead of a table:

Unconfigurable:
```toml
[lints]
clippy = [
  { all = "Alow" },
  { doc_markdown = "Worn" },
]
```

Configurable:
```toml
[[lints.clippy.all]
level = "Alow"
[[lints.clippy.doc_markdown]
level = "Worn"
```
Where the order is based on how to pass them on the command-line.

Complex TOML arrays tend to be less friendly to work with including the fact
that TOML 1.0 does not allow multi-line inline tables.

For the most part, people won't need granularity, so we could instead start
with a `priority: bool` field.  This might get confusing to mix with numbers
though (what does `false` and `true` map to?).  There is also the problem that
generally people will want to opt a specific lint into being low-priority (the
group) and the leave the exceptions at default but making `priority = true` the
default would read weird (everything is a priorty but one or two items).

# Prior art
[prior-art]: #prior-art

Rust
- [cargo cranky](https://github.com/ericseppanen/cargo-cranky)

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

Ruby
- [rubocop](https://docs.rubocop.org/rubocop/1.45/configuration.html)
  - Format is `Lint: Enabled: true`

# Unresolved questions
[unresolved-questions]: #unresolved-questions

# Future possibilities
[future-possibilities]: #future-possibilities

## External file

Like with `package.license`, users might want to refer to an external file for
their lints.  This especially becomes useful for copy/pasting lints between
projects.

## Configurable lints

We can extend basic lint syntax:
```toml
[lints.clippy]
cyclomatic_complexity = "allow"
```
to support configuration, whether for cargo or the lint tool:
```toml
[lints.clippy]
cyclomatic_complexity = { level = "allow", rust-version = "1.23.0", threshold = 30 }
```
Where `rust-version` is used by cargo to determine whether to pass along this
lint and `threshold` is used by the tool.  We'd need to define how to
distinguish between reserved and unreserved field names.

Tool-wide configuration would be in in the `lints.<tool>.metadata` table and be
completely ignored by `cargo`.  For example:
```toml
[lints.clippy.metadata]
avoid-breaking-exported-api = true
```

Tools will need `cargo metadata` to report the `lints` table so they can read
it without re-implementing workspace inheritance.

## Packages overriding inherited lints

Currently, it is a hard error to mix `workspace = true` and lints.  We could
open this up in the future for the package to override lints from the
workspace.  This would not be a breaking change as we'd be changing an error
case into a working case.  We'd need to ensure we had a path forward for the
semantics for configurable lints.

## Extending the syntax to `.cargo/config.toml`

Similar to `profile` and `patch` being in both files, we could support
`[lints]` in both files.  This allows more flexibility for experimentation with
this feature, like conditionally applying them or applying them via environment
variables.  For now, users still have the option of using `rustflags`.

## Cargo Lints

The cargo team has expressed interest in producing warnings for more situations
but this requires defining a lint control system for it.  The overhead of doing
so has detered people from adding additional warnings.  This would provide an
MVP for controlling cargo lints, unblocking the cargo team from adding more
warnings.  This just leaves the question of whether these belong more in cargo
or in clippy which already has some cargo-specific lints.
