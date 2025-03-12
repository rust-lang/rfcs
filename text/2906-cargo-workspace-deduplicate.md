- Feature Name: `workspace-deduplicate`
- Start Date: 2020-04-13
- RFC PR: [rust-lang/rfcs#2906](https://github.com/rust-lang/rfcs/pull/2906)
- Rust Issue: [rust-lang/cargo#8415](https://github.com/rust-lang/cargo/issues/8415)

> **Note**: This feature was stabilized in Rust 1.64. Several design changes were made in the course of the implementation. Please see the documentation for [`[workspace.package]`](https://doc.rust-lang.org/nightly/cargo/reference/workspaces.html#the-package-table) and [`[workspace.dependencies]`](https://doc.rust-lang.org/nightly/cargo/reference/workspaces.html#the-dependencies-table) for details on how to use this feature.

# Summary
[summary]: #summary

Deduplicate common dependency and metadata directives amongst a set of workspace
crates in Cargo with extensions to the `[workspace]` section in `Cargo.toml`.

# Motivation
[motivation]: #motivation

Cargo has supported workspaces for quite some time now but when managing a large
workspace there is often a good deal of redundancy between member crates in a
workspace. Currently this proposal attempts to tackle a few major areas of
duplication. Many of these areas of duplication are managed either manually or
with scripts, and the goal of this proposal is to largely eliminate the need for
scripts and also the need to manually manage so much.

## Duplication of `[dependencies]` sections

Often when managing a workspace you'll have a lot of crates that all depend on
the same crate. For example many of your crates may depend on `log`. Today you
must write down the same `log` directive in all your manifests:

```toml
[dependencies]
log = "0.3.1"
```

Depending on how many crates you're working on, that's a lot of times to
remember `0.3.1`! Additionally if you'd like to update this dependency, say if a
`1.0.0` release is made, you need to edit every single `Cargo.toml` to make sure
they all stay in sync. This is a lot of duplicated work!

This duplication gets even worse when you start modifying the features of each
crate. For example:

```toml
[dependencies]
log = { version = "0.3.1", features = ['release_max_level_warn'] }
```

If you wanted to consistently write this across many crates it can get quite
cumbersome quite quickly.

## Duplication in inter-dependent crates

When managing a workspace you'll often have a lot workspace members that all
depend on each other. The "blessed" way to do this is actually quite verbose:

```toml
[dependencies]
other-workspace-member = { path = "../other-member", version = "0.2.3" }
```

Here you need to specify *both* `path` and `version`. Using `path` means that
you're depending on exactly that copy on the local filesystem. This also means
that if you depend on any workspace member via a `git` dependency later on it'll
correctly pull in the other workspace members from the git repo. (note that some
projects use `[patch]` to only write down `other-workspace-member = "0.2.3"` but
this causes issues when crates later use git dependencies)

If you never publish to crates.io, `path` is all you need. If crates eventually
get published, though, they also need a `version` directive to know what version
from crates.io you'll be depending on after the publication.

Naturally, with a highly-interconnected workspace which may be relatively large,
this leads to a lot of duplication very quickly. This is a lot of `path` and
`version` directives that you've got to manage.

## Duplication in crate versions

A frequent pattern in Cargo workspaces which publish to crates.io is to have all
the crate at the same semver version. These crates all move in lockstep during
publication and get bumped at the same time.

While a minor papercut this basically means that anyone and everyone who has a
workspace of a lot of crates makes their own homebrew script for updating
versions and managing updates/publications. It'd be quite convenient if we could
standardize across the Rust ecosystem how to manage this information!

## Duplication in crate metadata

The last primary area of duplication that this proposal attempts to tackle is in
crate metadata in the `[package]` section. This includes items such as:

```toml
[package]
authors = []
license = "..."
repository = "..."
documentation = "..."
```

These metadata directives are often duplicated amongst all crates, especially
author/license/repository information. This is a pretty poor experience if you'e
got to keep writing down the information in so many places!

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Cargo's manifest parsing will be updated with new features to support
deduplicating each of the areas above. While all of these new features are
pretty small in their own right, they all add up to greatly reducing the
overhead of managing a workspace of many crates. The list of new features in
Cargo will look like the following:

## Workspace-level Dependencies

The `[workspace]` section can now have a `dependencies` section which works the
same way as the `[dependencies]` section in `Cargo.toml`:

```toml
# in workspace's Cargo.toml
[workspace.dependencies]
log = "0.3.1"
log2 = { version = "2.0.0", package = "log" }
serde = { git = 'https://github.com/serde-rs/serde' }
wasm-bindgen-cli = { path = "crates/cli" }
```

Each workspace member can then reference this section in the workspace with a
new dependency directive:

```toml
# in a workspace member's Cargo.toml
[dependencies]
log = { workspace = true }
```

This directive indicates that the `log` dependency should be looked up from
`workspace.dependencies` in the workspace root. You can reference any name
defined in `[workspace.dependencies]` too:

```toml
[dependencies]
log2 = { workspace = true }
```

## No longer need both `version` and `path` to publish to Crates.io

When you have a `path` dependency, Cargo's current behavior on publication looks
like this:

* If you have a `version` specifier as well, then the `path` key is deleted and
  the crate is uploaded with the specified `version` as a dependency
  requirement.
* If you don't have a `version` specifier, then the dependency directive is
  deleted and crates.io will not learn about this dependency. This is only
  really useful for `dev-dependencies`.

Cargo's behavior will change in this second case, instead following new logic
for a missing `version` specifier. For dev-dependencies where the referenced
package is `publish = false`, then the dependency will be dropped. Otherwise
Cargo will assume that `version = "$dependency_version"` was specified, meaning
that it requires at least the current version and otherwise any
semver-compatible version.

This behavior should mean that you no longer need to write `version = "..."`
with `path` dependencies if you publish to crates.io. Coupled with the
workspace-level dependencies above this means you never have to write the
version of a path dependency anywhere!

## Package metadata can reference other workspace members

To deduplicate `[package]` directives in `Cargo.toml` workspace members, Cargo
will now support declaring that metadata directives should be inherited from the
workspace. For example to version every package the same within a workspace you
can specify:

```toml
[package]
name = "foo"
version = { workspace = true }
```

This directive tells Cargo that the version of `foo` is the same as the
`workspace.version` directive found in the workspace manifest. This means that
in addition to a new `[workspace.dependencies]` section, package metadata keys
can now also be defined inside of a `[workspace]` section:

```toml
[workspace]
version = "0.25.2"
```

Many other package metadata attributes are supported as well

```toml
[package]
authors = { workspace = true }
license = { workspace = true }
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Cargo's `[workspace]` section will first be extended with a few new attributes.
Like before the `[workspace]` table can only appear in a workspace root, not in
any other manifests. Additionally the `[workspace]` table doesn't have to be
associated with a package, it could be part of a virtual manifest.

## Updates to `[workspace]`

The first addition to the `[workspace]` table is a `dependencies` sub-table,
like so:

```toml
[workspace.dependencies]
foo = "0.1"
```

The `dependencies` sub-table has the same form as the `[dependencies]` table in
manifests with a few exceptions:

* Dependencies cannot be declared as `optional`. The `optional` key must be
  omitted or, if present, must be `false`.
* The `workspace` key (defined later in this proposal) is not allowed.

The `[workspace]` table will not support other kinds of dependencies like
`dev-dependencies`, `build-dependencies`, or `target."...".dependencies`.  Only
`[workspace.dependencies]` will be supported.

To review, the `[workspace.dependencies]` table will be key/value pairs. Each
key is the name of a dependency while the dependency is a dependency directive.
This could be a string meaning a crates.io dependency or a table which further
configures the dependency.

Dependencies declared in `[workspace.dependencies]` have no meaning as-is. They
do not affect the build nor do they force packages to depend on those
dependencies. This part comes later below.

The `[workspace]` section will also allow the definition of a number of keys
also defined in `[package]` today, namely:

```toml
[workspace]
version = "1.2.3"
authors = ["Nice Folks"]
description = "..."
documentation = "https://example.github.io/example"
readme = "README.md"
homepage = "https://example.com"
repository = "https://github.com/example/example"
license = "MIT"
license-file = "./LICENSE"
keywords = ["cli"]
categories = ["development-tools"]
publish = false
edition = "2018"

[workspace.badges]
# ...
```

Each of these keys have no meaning in a `[workspace]` table yet, but will have
meaning when they're assigned to crates internally. That part comes later though
in this design! Note that the format and accepted values for these keys are the
same as the `[package]` section of `Cargo.toml`.

For now the `metadata` key is explicitly left out (due to complications around
merging table values), but it can always be added in the future if necessary.

## Updates to a package `Cargo.toml`

The interpretation of a `Cargo.toml` manifest within Cargo will now require a
`Workspace` object to be created. This `Workspace` will be used to elaborate and
expand each member's `Cargo.toml` directive. Additionally `Cargo.toml` will
syntactically accept some more forms.

### Placeholder Values

Previously package metadata values must be declared explicitly in each
`Cargo.toml`:

```toml
[package]
version = "1.2.3"
```

Cargo will now accept a table definition of `package.$key` which defines the
`package.$key.workspace` key as a boolean. For example you can specify:

```toml
[package]
name = "foo"
license = { workspace = true }
```

This directive indicates that the license of `foo` is the same as
`workspace.license`. If `workspace.license` isn't defined then this generates an
error.

The following keys in `[package]` can be inherited from `[workspace]` with the
new `workspace = true` directive.

```toml
[package]
version = { workspace = true }
authors = { workspace = true }
description = { workspace = true }
documentation = { workspace = true }
readme = { workspace = true }
homepage = { workspace = true }
repository = { workspace = true }
license = { workspace = true }
license-file = { workspace = true }
keywords = { workspace = true }
categories = { workspace = true }
publish = { workspace = true }
```

Note that directives like `license-file` are resolved relative to their
definition, so `license-file` is relative to the `[workspace]` section that
defined it.

### New dependency directives

Dependencies in the `[dependencies]`, `[dev-dependencies]`,
`[build-dependencies]`, and `[target."...".dependencies]` sections will support
the ability to reference the `[workspace.dependencies]` definition of
dependencies. This is done with a new `workspace` key in the dependency
directive. An example of this looks like:

```toml
[dependencies]
log = { workspace = true }
```

The `workspace` key cannot be defined with other keys that configure the source
of the dependency. This means you cannot define `workspace` with keys like
`version`, `registry`, `registry-index`, `path`, `git`, `branch`, `tag`, `rev`,
or `package`. The `workspace` key can be combined with other keys, however:

* `optional` - this introduces an optional dependency as usual, as well as a
  feature named after the key (left hand side) of the dependency directive).
  Note that the `[workspace.dependencies]` table is not allowed to specify
  `optional`.

* `features` - this indicates, as usual, that extra features are being enabled
  over the already-enabled features in the directive found in
  `[workspace.dependencies]`. The result set of enabled features is the union of
  the features specified inline with the features specified in the directive in
  the workspace table.

For now if a `workspace = true` dependency is specified then also specifying the
`default-features` value is disallowed. The `default-features` value for a
directive is inherited from the `[workspace.dependencies]` declaration, which
defaults to `true` if nothing else is specified.

### Path dependencies infer `version` directive

As a final change to `Cargo.toml`, dependencies using the `path` directive and
not specifying a `version` directive will have the `version` directive inferred.

For example if we have:

```toml
# foo/Cargo.toml
[dependencies]
bar = { path = "../bar" }
```

as well as

```toml
# bar/Cargo.toml
[package]
name = "bar"
version = "1.0.1"
```

this is equivalent in `foo/Cargo.toml` to as if this were written:

```toml
# foo/Cargo.toml
[dependencies]
bar = { path = "../bar", version = "1.0.1" }
```

The `version` key for `path` dependencies, if not specified, will be inferred to
the version of the path dependency itself. Note that this is a version
requirement not an actual semver version, and the version requirement will be
interpreted as "at least the current version, and anything semver compatible
with it".

This logic of inferring, however, will also respect the `publish` key. For
example if we had this instead:

```toml
# bar/Cargo.toml
[package]
name = "bar"
version = "1.0.1"
publish = false
```

then Cargo would not alter this dependency directive:

```toml
# foo/Cargo.toml
[dependencies]
bar = { path = "../bar" }
```

## Effect on `cargo publish`

Cargo currently already "elaborates" the manifest during publication. For
example it removes `path` keys in dependency lists to only have the version
requirement pointing to crates.io. During publication Cargo will also elaborate
any substituted information from the `[workspace]`, because `[workspace]` is
also removed during publication!

This means that `workspace = true` will never be present in `Cargo.toml` files
published to crates.io, and additionally no information about `workspace = true`
will make its way to the registry index. Furthermore metadata fields like
`package.repository` will be filled in and will be present on crates.io's UI.

Put another way, `Cargo.toml` files published to crates.io, or metadata found
through crates.io, won't change from what they are today.

## Effect on `Cargo.lock`

When creating a `Cargo.lock` file Cargo will perform crate resolution as-if all
dependencies in `[workspace.dependencies]` are depended on by some crate, even
if no crate actually references an entry in `[workspace.dependencies]`. This
means that if a crate uses an entry in `[workspace.dependencies]` it's
guaranteed to have an entry in the lock file indicating what its dependencies
should be.

Note that for entries in `[workspace.dependencies]` which aren't used by any
crates in the workspace will likely trigger a warning, however, so users can
continue to prune accidentally unused entries.

## Effect on `cargo metadata`

Executing `cargo metadata` to learn about a crate graph will implicitly perform
all substitution defined in this proposal. Consumers of `cargo metadata` will
continue to get the same output they got before this proposal, meaning that
implicit substitutions, if any, will be invisible to users of `cargo metadata`.

## Effect on `cargo read-manifest`

Similar to `cargo metadata`, the `cargo read-manifest` command will perform all
necessary substitutions when presenting the output as JSON.

## Effect resolution for relative `path` dependencies

Like today, `path` dependencies will be resolved relative to the file that
defines them. This means that references to dependencies defined in the
workspace means paths are still relative to the workspace root itself.

For example if you write down a `[workspace.dependencies]` directive with a
relative path:

```toml
# Cargo.toml
[workspace.dependencies]
my-crate = { path = "crates/my-crate" }
```

And then you reference this in another crate:

```toml
# crates/other-crate/Cargo.toml
[dependencies]
my-crate = { workspace = true }
```

then the `my-crate` dependency references the crate located at `crates/my-crate`
relative to the workspace root, not located at
`crates/other-crate/crates/my-crate`.

# Drawbacks
[drawbacks]: #drawbacks

This proposal significantly complicates the process of interpreting a
`Cargo.toml`. One of the major purposes of using TOML to specify a crate
manifest was to make it easy for other tools to parse Cargo manifests and work
with them. This not only includes Rust-based tools but also tools in other
languages if necessary. Previously a TOML parser for your language was all you
really needed, but this proposal is adding a layer of indirection on top of TOML
where you have to interpret multiple manifests to figure out what one means. For
example you can no longer quickly and easily be guaranteed to parse the version
of a package, but you might have to go find the workspace root or other crates
to figure that out. Workspace discovery and membership is pretty nontrivial so
non-Cargo based tools will have a difficult time *not* using Cargo to figure out
a full elaborated form of a manifest 100% of the time.

This proposal also extends `Cargo.toml` with changes that will break any
existing tools which assume a particular format of `Cargo.toml`. For example if
a tool expects `package.version` to be a `String` that runs a risk of being
broken in the future due to the ability to specify a table there instead.

Additionally this proposal complicates a reader's understanding of `Cargo.toml`.
While verbose for maintainers having duplication of information is actually
quite nice for readers of `Cargo.toml` because you don't have to chase anything
else down to figure out what a dependency is. If this proposal is implemented
then whenever you see `foo = { workspace = true }` you've got to go consult
something else to figure out what the dependency actually is. This layer of
indirection can cause surprise for readers or otherwise add a speed-bump to
understanding the contents of a manifest.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Cargo's manifests have been a pretty carefully curated part of Cargo's design to
ensure that they're consistently readable and concise where possible. For
example many of Cargo's manifest idioms gently nudge users towards the same
standards across the community by supporting many zero-configuration situations
such as where to put and how to name tests.

This proposal is an extension of these design principles to provide a gentle
nudge to consistently, across the Rust community, manage workspaces,
dependencies, and metadata. A goal here is to increase consistency in how this
is all managed across projects in a way that still preserves Cargo's existing
flexibility for users.

Note that flexibility is a key part of this proposal where it's possible to
intermingle shorthands with longer versions. For example if the workspace
declares:

```toml
[workspace.dependencies]
log = "0.3"
```

But you really want to try out a new version of `log` in one workspace member,
you can easily do so by changing

```toml
[dependencies]
log = { workspace = true }
```

to

```toml
[dependencies]
log = "0.4"
```

Additionally you can always custom-version your packages, you've just got the
option to reference another package as well. Overall this proposal should
empower more power users of Cargo to manage workspaces easily without taking
away any of the existing configurations that Cargo already supports.

## Alternative Syntax

This proposal is largely a syntactic proposal for `Cargo.toml` and changing how
we can specify a few directives. Naturally that lends itself to quite a lot of
possible bikeshedding! Virtually all of the aspects of the proposal that modify
`Cargo.toml` can be tweaked in various ways such as names used or where they're
placed. In any case discussion about compelling alternatives is always
encouraged!

Some alternative syntaxes:

```toml
[dependencies]
# Instead of `foo = { workspace = true }`
foo = {}
foo = "ws"
foo = "workspace"
foo.workspace = true # technically the same, but idiomatically different
```

## Including metadata by default

This proposal indicates that package metadata is not inherited by default from
the workspace. This may be desired in some scenarios instead of repeating
`license = { workspace = true }` everywhere, and there's likely two possible
ways this could happen.

* Workspace directives could be implicitly and automatically inherited to
  members. In the future, however, Cargo will want to support nested
  workspaces, and it's unclear how these features will interact. In order to
  strik a reasonable middle-ground for now a simple solution which should
  address many use cases is proposed and we can continue to refine this over
  time as necessary.

* Directives could be flagged to be explicitly inherited to workspace members as
  an optional way of specifying this. For now though to keep this proposal
  simple this is left out as a possible future extension of Cargo.

## Inheriting metadata from other packages

One possible extension of this RFC is for metadata to not only be inheritable
from the `[workspace]` table but also from other packages. For example a
scenario seen in the wild is that some repositories have multiple "cliques" of
crates which are all versioned as a unit. In this scenario one "clique" can have
its version directives deduplicated with this proposal, but not multiple ones.

It's hoped though that an eventual feature of nested workspaces would solve this
issue in Cargo. That way each "clique" could correspond to one workspace, and
that way we wouldn't need extra support to inherit directives from anywhere.

## Motivating issues

Duplication throughout workspaces has been a thorn in Cargo's since practically
since the inception of workspaces. Naturally there's quite a few bugs filed on
Cargo's issue tracker about this which provide some context for why make a
proposal at all as well as how to design this proposal.

* [#3931] - updating the version of a crate in a workspace means lots of edits
* [#7552] - crates may differ slightly in versions required from crates.io
* [#7964] - current idioms push users towards usage of `[patch]` which breaks
  git dependencies
* [#5471] - an issue about shared dependencies in a workspace
* [#6126] - an issue where `[patch]` tables are used seemingly to make it easier
  to specify dependencies in a workspace, but having everything in
  `[workspace.dependencies]` makes it smaller to specify.
* [#6828] - an issue about inheriting workspace attributes

[#3931]: https://github.com/rust-lang/cargo/issues/3931
[#7552]: https://github.com/rust-lang/cargo/issues/7552
[#7964]: https://github.com/rust-lang/cargo/issues/7964
[#5471]: https://github.com/rust-lang/cargo/issues/5471
[#6126]: https://github.com/rust-lang/cargo/issues/6126
[#6828]: https://github.com/rust-lang/cargo/issues/6828

## Full templating language

One sort of far-out-there alternative we could go for is to be far more
ambitious and make our own sort of "templating language" on top of TOML. This
would arguably be much more flexible than the limited amount of deduplication
proposed here, but you could imagine things like:

```toml
[package]
name = "foo"
version = "1.{workspace.vars.minor}.0"

[dependencies]
bar = "{workspace.dependencies.bar}"
baz = { version = "1", features = "{workspace.vars.baz_features}" }
```

or "insert your own idea for how we can go all out" here. In general though I
think there's a lot to be gained from the simplicity of TOML and prioritizing
other tools reading Cargo manifests, so we may not want to go full-blown
templating language just yet.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

* One thing we'll want to resolve for sure is nailing down all the syntactical
  decision here, which is expected to evolve through consensus.

* It's not clear how complex an implementation of this proposal will be in
  Cargo. It could be prohibitively complex, but it's hoped that it's a
  relatively simple refactoring to implement this in Cargo.
