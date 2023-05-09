- Feature Name: `manifest-lint`
- Start Date: 2023-02-14
- RFC PR: [rust-lang/rfcs#3389](https://github.com/rust-lang/rfcs/pull/3389)
- Tracking Issue: [rust-lang/cargo#12115](https://github.com/rust-lang/cargo/issues/12115)

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
and `cargo` would pass these along as flags to `rustc`, `clippy`, or other lint tools.

This would work with
[RFC 2906 `workspace-deduplicate`](https://rust-lang.github.io/rfcs/2906-cargo-workspace-deduplicate.html?highlight=2906#):
```toml
[lints]
workspace = true

[workspace.lints.rust]
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

`priority` is a signed integer that controls which lints or lint groups override other lint groups:
- lower (particularly negative) numbers have lower priority, being overridden
  by higher numbers, and show up first on the command-line to tools like
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
`rustdoc`).  When running the underlying command for the specified package,
cargo will:
1. Transform the lints from `tool.lint = level` to `--level tool::lint`
  - Leaving off the `tool::` when it is `rust`
  - cargo will error if `lint` contains `::` as the first part is assumed to be
    a tool and it should be listed in that tool's table
2. Sort them by priority and then an unspecified order within priority that we may change in the [future](#future-possibilities).
  - On initial release, the sort will likely be reverse alphabetical which would cause `all` to be last, making it unlikely to do what the user wants which would raise awareness of the need for setting `priority` for all groups.
3. Pass them on the command line before other configuration like
`RUSTFLAGS`, allowing user configuration to override package configuration.
  - These flags will be fingerprinted so changing them will cause a rebuild only
    for the commands where they are used.  By only including the lints for the
    command in question, we reduce what is fingerprinted, reducing what gets
    rebuilt when `[lints]` is changed.

Note that this means that `[lints]` is only applied to the package where its
defined and not to its dependencies, local or not.  This avoids having to unify
`[lints]` tables across local packages.  Normally, lints for non-local
dependencies won't be shown anyways because of `--cap-lints`.  As for local
dependencies, they will likely have their own `[lints]` table, most the same
one, inherited from the workspace.

Initially, the only supported tools will be:
- `rust`
- `clippy`
- `rustdoc`

The reason for `rust` existing, despite lints not being prefixed with `rust::`, is
to avoid ambiguity in the data model between `lint.<lint>` and
`lint.<tool>.<lint>`.  A downside to naming the tool `rust` is it might be
confusing if we ever expose `rustc::` lints.

Addition of third-party tools would fall under their
[attributes for tools](https://github.com/rust-lang/rust/issues/44690).

**Note:** This reserves the tool name `workspace` to allow workspace inheritance.

# Drawbacks
[drawbacks]: #drawbacks

Since `[lints]` only affects the associated package, and not dependencies, it
will not work with `future-incompat` lints that are meant to be applied to
dependencies.  This may cause some user confusion.

There has been some user/IDE confusion about running commands like `rustfmt`
directly and expecting them to pick up configuration only associated with their
higher-level cargo-plugins despite that configuration (like `package.edition`)
being cargo-specific.  By baking the configured lint levels for rustc, rustdoc, and
clippy directly into cargo, we will be seeing more of this.  A hope is that
this will actually improve with this RFC.  Over time, tools will need to switch
to the model of running `cargo` to get configuration in response to this RFC.
As for users, if a tool's primary configuration is in `Cargo.toml`, that will
provide a strong coupling with `cargo` in users minds as compared to using an
external configuration file and overlooking the one or two fields read from
`Cargo.toml`.

As this focuses on lints, this leaves out first-party tools that need
configuration but aren't linters, namely `rustfmt`, leading to an inconsistent
experience if `clippy.toml` goes away in the future (if we act on the future
possibility of supporting linter configuration)

A concern brought up in
[rust-lang/rust-clippy#1313](https://github.com/rust-lang/rust-clippy/issues/1313)
was that this will pass lints unconditionally to the underlying tool, leading
to "undefined lint" warnings when used on earlier versions, requiring that
warning to also be suppressed, reducing its value.  However, in the "Future
possibilities" section, we mention direct support for tying lints to rust versions.

This does not allow sharing lints across workspaces.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

When designing this, we wanted to keep in mind how things work today, including
- `clippy` defines all configuration as linter/tool config and not lint config (linter/lint config is a future possibility)
- All `clippy` lint groups are disjoint
- `rustdoc` has no plans for groups outside of `all`
- `rustc` today has some intersecting groups

However, we also need to consider how decisions might limit us in the future and whether we want to bind future decisions with this RFC, including
- Whether existing decisions will be revisited
- When new tools are added, like `cargo` and `cargo-semver-check`, which haven't had lint levels and configuration long enough (or at all) to explore their problem and design space.

## Misc

This could be left to `clippy.toml` but that leaves `rustc`, `rustdoc`, and future linters without a solution.

`[lints]` could be `[package.lints]`, tying it to the package unlike `[patch]`
and other fields that are more workspace related.  Instead, we used
`[dependencies]` as our model.

`[lints]` could be `[lint]` but we decided to follow the precedence of `[dependencies]`.

## Schema

In evaluating prior art, we saw two major styles for configuring lint levels:

Python-style:
```toml
[lints]
warn = [
  { lint = "rust_2018_idioms", priority = -1 },
  { lint = "clippy::all", priority = -1 },
  "clippy::await_holding_lock",
  "clippy::char_lit_as_u8",
  "clippy::checked_conversions",
]
deny = [
  "unsafe_code",
]
```

Inspired by ESLint-style:
```toml
[lints.rust]
rust_2018_idioms = { level = "warn", priority = -1}

unsafe_code = "deny"

[lints.clippy]
all = { level = "warn", priority = -1 }

await_holding_lock = "warn"
char_lit_as_u8 = "warn"
checked_conversions = "warn"
```
- More akin to `eslint`

In a lot of areas, which to choose comes down to personal preference and what people are comfortable with:
- If you want to lookup everything for a level, Python-style works better
- If you want to look up the level for a lint, ESLint-style works better
- Python-style is more succinct
- Python-style has fewer jagged edges

We ended up favoring more of the ESLint-style because:
- ESLint-style offers more syntax choices for lint config (inline tables,
  standard tables, dotted keys).  In general, the TOML experience for deeply
  nested inline structures is not great.
  - Right now, the only other lint field beside `level` is `priority`.  In the future we may add lint configuration.  While we shouldn't exclusively design for this possibility, all things being equal, we shouldn't make that potential future's experience worse
- ESLInt-style makes it easier to visually highlight groups and the lints related to those groups
- The cargo team has seen support issues that partially arise from a user
  losing track of which `dependencies` table they are in because the list of
  dependencies is large enough to have the header far enough away (or off
  screen).  This can similarly happen with Python-style as the context of the
  level is in the table header.  See [EmbarkStudios's lint list as an example of where this could happen](https://github.com/EmbarkStudios/rust-ecosystem/blob/81d62539a57add13f4b0f1c503e267b6de358f70/lints.toml)
- If we add support for packages to override some of the lints inherited from
  the workspace, it is easier for users to map out this relationship with
  ESLint-style.

## Linter Tables vs Linter Namespaces

We started off with lints being referenced with their tool as a namespace (e.g.
`"clipp::enum_glob_use"`) like in diagnostic messages, making copy/paste easy.

However, we switched to a more hierarchical data model (e.g.
`clippy.enum_glob_use`) to avoid quoting keys with the `lints.<lint> = <level>` schema.

If we add lint/linter config in the future
- Being more hierarchical means lint and linter config are kept closer to each
  other, making it easier to evaluate their impact on each other.
- `lints.<lint> = <level>` combined with `lints.<linter>.metadata` makes it
  harder for cargo to collect all the lints to pass down into the compiler
  driver.

## Lint Precedence

Currently, `rustc` allows lints to be controlled on the command-line with the
last level for a lint winning.  They may also be specified as attributes with
the last instance winning.  `cargo` adds the `RUSTFLAGS` environment variable
and `config.toml` entry.  On top of this, there are lint groups that act as
aliases to sets of lints.  These groups may be disjoint, supersets, or they may
even intersect.

Example `RUSTFLAGS`:
- `-Aclippy::all -Wclippy::doc_markdown`
- `-Dfuture-incompatible -Asemicolon_in_expressions_from_macros`

In providing lint-level configuration in `Cargo.toml`, users will need to be
able to set the lint level for group and then override individual lints within
that group while interacting with the existing `RUSTFLAGS` system.

We have chosen **Option 6** with `priority` being in-scope for this RFC and
warnings and auto-sorting as a future possibility.

**Option 1: Auto-sort**
```rust
[lints.rust]
unsafe_code = "deny"
allow_dead_code = "allow"
all = "warn"
```
- Unable to handle if two intersecting groups are assigned different levels

**Option 2: Ordered keys**
```rust
[lints.rust]
all = "warn"
allow_dead_code = "allow"
unsafe_code = "deny"
```
- Relies on the order of keys in a TOML table which is undefined
- Without standard ordering semantics, like with `[]`, users or formatters
  might naively reformat the table which would affect the semantics

**Option 3: Array of tables**
```toml
# inline table
[lints]
rust = [
    { lint = "all", level = "warn" },
    { lint = "allow_dead_code", level = "allow" },
    { lint = "unsafe_code", level = "deny" },
]
# standard table
[[lints.clippy]]
lint = "all"
level = "warn"
[[lints.clippy]]
lint = "cyclomatic_complexity"
level = "allow"
```
- The syntax for this seems overly verbose
- Complex, nested structures aren't the easiest to work with in TOML

**Option 4: Compact array of tables**
```toml
[tools.rust]
lints = [
    { warn = "all" },
    { allow = "allow_dead_code" },
    { deny = "unsafe_code" },
]
```
- *Note:* `lints.rust = []` wasn't used as that won't work with linter configuration in the future
- *Note:* Top-level table was changed to avoid `lints.rust.lints` redundancy and would allow us to open this up to more tools in the future
- *Note:* `<level> = <lint>` (instead of the other way) to keep the keys finite so we can add more fields in the future
- Mirrors the familiar `RUSTFLAGS` syntax
- Complex, nested structures aren't the easiest to work with in TOML

**Option 5: `priority` field**
```rust
[lints.rust]
all = { level = "warn", priority = -1 }
allow_dead_code = "allow"
unsafe_code = "deny"
```
- Difficult for the user to figure out there is a problem or how to address it

**Option 6: `priority` field with warnings and maybe auto-sort**
```rust
[lints.rust]
all = "warn"
allow_dead_code = "allow"
unsafe_code = "deny"
```
- Option 1 (auto-sort) but using Option 5 (`priority` field) to break ties
- Produces warnings to tell the user when `priority` may be needed
- As `priority` is a low-level subset, we can start with that as an MVP.  Later, we can add warnings for all the ambiguity cases.  As we gain confidence in this, we can then add auto-sorting.

**Option 7: Explicit groups**
```rust
[lints.rust.groups]
all = "warn"
[lints.rust.lints]
allow_dead_code = "allow"
unsafe_code = "deny"
```
- Hard codes knowledge of `all`
- Does not solve the intersecting group problem
- Names aren't validated as being from a group without duplicating the work needed for Option 1 (auto-sort)

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

## `rustfmt`

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

## Target-specific lint

We could support platform or feature specific settings, like with
`[lints.<target>]` or `[target.<target>.lints]` but
- There isn't a defined use case for this yet besides having support for `cfg(feature = "clippy")` or
  which does not seem high enough priority to design
  around.
- `[lints.<target>]` runs into ambiguity issues around what is a `<target>`
  entry vs a `<lint>` entry in the `[lints]` table.
- We have not yet defined semantics for sharing something like this across a
  workspace

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
  - Format is `level = [lint, ...]`, due to past precedence in ecosystem (see above)

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

Blocking for stablization
- Are we still comfortable with our schema choice?
- Are we still comfortable with our precedence choice?
- Can we fingerprint only the lints for the tool being run?

# Future possibilities
[future-possibilities]: #future-possibilities

## Help the user with `priority`

When running linters through cargo, we could warn the user when there is ambiguity, including
- A group and a lint at the same priority
- A group that is a superset of another group at the same priority
- Two intersecting groups at the same priority
- A lint or group that is masked by a group in a later priority

We could then take this a step further and change the way we sort within a
priority level to put the most specific entry last, where ambiguity doesn't
exist.  This would nearly eliminate the need for specifying `priority` with the
current groups.

We specifically recommend warning, rather than error, so groups can evolve to
become intersecting without it being a breaking change.

To implement this, either cargo needs to pass the lints down to the tool in a
way to communicate the priority batches, allow cargo to query the group
memberships from the linter, or we hard code this at compile-time like
rust-analyzer
([lints](https://rust-lang.github.io/rust-clippy/master/lints.json),
[generate](https://github.com/rust-lang/rust-analyzer/blob/a6464392c15fa8788215d669c4c0b1e46bcadeea/crates/ide-db/src/tests/sourcegen_lints.rs)).
One thing to keep in mind is the potential for [custom
tools](https://rust-lang.github.io/rfcs/2103-tool-attributes.html) in the
future.

## rustc reporting `Cargo.toml` as lint-level source

Currently Rust tells you where a lint level was enabled when it emits a lint.
`rustc` only sees that these lints are coming in from the command-line and
doesn't know about `[lints]`.
It would be nice if it could also point to Cargo.toml for this.  This could be
as simple as a `--lint-source=Cargo.toml` with rustc knowing just enough about
the `[lints]` table to process it directly.

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

**Note:** At this time, there is no lint configuration for clippy, just tool
configuration.  `lints.clippy.cyclomatic_complexity` exists for illustrative
purposes of what linters could support and is not indicative of any future
plans for clippy itself.

## Packages overriding inherited lints

Currently, it is a hard error to mix `workspace = true` and lints.  We could
open this up in the future for the package to override lints from the
workspace.  This would not be a breaking change as we'd be changing an error
case into a working case.  We should consider the possibility of adding
configurable lints in the future and what that would look like with
overridin of lints.

## Extending the syntax to `.cargo/config.toml`

Similar to `profile` and `patch` being in both files, we could support
`[lints]` in both files.  This allows more flexibility for experimentation with
this feature, like conditionally applying them or applying them via environment
variables.  For now, users still have the option of using `rustflags`.

We would need to define whether this only affects local packages as-if the user
set it in `Cargo.toml` or if it also affects dependencies.

In doing so, we would need to define how `priority` interacts with different
sources of `[lints]`.

## Cargo Lints

The cargo team has expressed interest in producing warnings for more situations
but this requires defining a lint control system for it.  The overhead of doing
so has detered people from adding additional warnings.  This would provide an
MVP for controlling cargo lints, unblocking the cargo team from adding more
warnings.  This just leaves the question of whether these belong more in cargo
or in clippy which already has some cargo-specific lints.
