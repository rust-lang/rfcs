- Feature Name: `unstable-on-stable`
- Start Date: 2025-05-18
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC proposes decoupling the two components of our stability policy: still requiring feature gates, but allowing feature gates to be enabled on stable.

It does so in three ways:
1. Extending `-Z unstable-options` to take a list of option names, rather than being a simple boolean.
2. Add a new `[workspace.unstable.features]` table to Cargo.toml, allowing Cargo to proxy them through with accurate caching.
   `unstable.features` is ignored unless Cargo is passed `--unstable-features`.
3. Add a new `--unstable-flags` flag to Cargo, as well as to all other tools in the toolchain.
   `unstable-flags` does not have a feature gate.

This RFC acknowledges that in practice it will be used as a general purpose mechanism for Rust developers to use nightly features on stable.
However, it's specifically targeted at build systems wrapping cargo,
such as distro packagers, external tools shipped with the toolchain, and large projects that build a custom Rust toolchain from source.

# Motivation
[motivation]: #motivation

## Why allow using unstable features on stable?

Rust's stability policy has two components:
1. To the extent possible, each unstable feature comes with a feature gate, and is disabled when that feature gate is inactive. [^1]
2. Enabling feature gates is only allowed on the nightly toolchain.

[^1]: There are some exceptions to this, such as https://github.com/rust-lang/rust/issues/139892#issuecomment-2808505610.
     But in general we attempt to make sure all unstable features have a feature gate.

Our motivation for 1 (having feature gates) is to make sure that people do not unknowingly rely on unstable features.
This was a big problem for e.g. intra-doc links, which [people often used without knowing they were unstable][63305], making it impossible to remove the feature.

[63305]: https://github.com/rust-lang/rust/issues/63305

Our motivation for 2 (disabling feature gates on stable) is three-fold:
1. Prevent people from relying on features that may change in the future while on the stable toolchain, upholding our "stability without stagnation" motto.
2. Disallow library authors from "silently" opt-ing in to unstable features,
   such that the person running the top-level build doesn't know they're using unstable features that may break when the toolchain is updated.
   This rationale doesn't apply to nightly, where the party running the top-level build is assumed to know that nightly comes with no stability guarantees.
3. Encourage people to help stabilize the features they care about.

There are some cases in which none of those goals are applicable, but we still prevent people from using nightly features.
This is particularly bad when projects *must* depend on unstable features to ship another feature they care about.
Some examples:
- rust-analyzer and RustRover need `./some-libtest-binary --format=json` to determine the list of possible tests to run.
- rust-analyzer and RustRover need all values in `rustc --print=cfg` to build the standard library.
  (see [#139892](https://github.com/rust-lang/rust/issues/139892#issuecomment-2808505610) for an explanation of why this is affected by unstable features)
- `cargo semver-checks` needs `rustdoc --output-format=json` in order to work at all.
- Rust for Linux needs a way to build a custom version of core.
  In particular, they mentioned they need to disable float support, because using float registers can cause unsoundness.
- `rustc_public`'s entire mission is to wrap unstable APIs with stable ones and therefore needs access to all `rustc_private` features.

Why are these uses ok? Two reasons:
- Each of these tools accept responsibility for breakage.
  `semver-checks` and RfL both explicitly adapt to each new release of rustc, and their feedback on breakage is very useful for improving the features they use.
  rust-analyzer and RustRover don't break at all for `--print=cfg`—they're not using it in code, only in the CLI—and adapt to any changes in libtest json format.
- These tools act as a "buffer" between other projects and breakage.
  For example, semver-checks hides the breaking changes behind its own interface such that downstream projects are not affected.
  Similar, Rust for Linux backports breakage fixes to stable branches such that old versions of the kernel keep building with new rust toolchains.

One might ask, well, maybe we are being too eager to gate things, but can't people just use nightly? There are some cases where switching to nightly is not realistic.
- When using rustc packaged by a distro (e.g. Fedora or `nixpkgs`), only the stable channel is packaged.
- Tools that wrap the compiler (e.g. `rust-analyzer` or `cargo expand`) or libraries (e.g. `proc-macro2`) usually do not control the toolchain version being used.
- Stable with `RUSTC_BOOTSTRAP` is not the same as nightly.
  In particular, stable contains backports and nightly does not.

## Why this exact mechanism?

Currently, these tools use [`RUSTC_BOOTSTRAP=1`][rustc-bootstrap] as a workaround. But this workaround has many downsides:

[rustc-bootstrap]: https://doc.rust-lang.org/nightly/unstable-book/compiler-flags/rustc-bootstrap.html

- Enabling RUSTC_BOOTSTRAP for one part of the toolchain enables it for *all* parts of the toolchain; in particular:
    - `proc-macro2` uses `cargo:rerun-on-env-changed=RUSTC_BOOTSTRAP`, causing cache thrashing whenever this env var changes.
    - rust-analyzer wants to enable RUSTC_BOOTSTRAP only for cargo and libtest, but the variable enables features for rustc as well.
      `RUSTFLAGS="-Z allow-features="` fixes this for lang features, but at the price of thrashing the cache; and there is no equivalent way to disable unstable CLI features.
- Libraries that detect RUSTC_BOOTSTRAP sometimes do it incorrectly (in particular, `-Zallow-features` often messes things up).
  To do this correctly, one must compile a full rust program that uses the api the library wants to enable; but in practice doing this is rare.
  Limiting the opt-in to a specific feature makes it less likely that a single misbehaving library can break the whole build.

An important design constraint here is that the "end-user" (whoever is running the build) should always have control over which features are enabled.
To the extent that tools act as a "buffer" between feature breakage and the end-user, they should only take responsibility for exactly the features whose breakage they know how to handle.

`[workspace.unstable]` only permits the top-level build to allow feature gates.
Build scripts cannot modify Cargo.toml files, and Cargo only respects `[workspace]` for the root manifest.
This allows developers to experiment locally with nightly features on a stable toolchain, but without allowing them to opt-in silently when their library is published to crates.io.
`[workspace.unstable]`

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The following documentation will live in the [unstable book] (or Cargo's [unstable features][cargo-unstable] section), not in the [rustc book].

[unstable book]: https://doc.rust-lang.org/nightly/unstable-book/
[cargo-unstable]: https://doc.rust-lang.org/cargo/reference/unstable.html#allow-features
[rustc book]: https://doc.rust-lang.org/nightly/rustc/

## For Cargo users

The `[workspace.unstable.flags]` and `[workspace.unstable.features]` tables allow you to use unstable features on the stable and beta channels.
In particular, they allow you to use `-Z` flags and `#![feature(..)]` attributes, respectively.
These can *only* be used in a local build of your workspace; they cannot be published to crates.io, nor used in a git/path dependency.
Cargo will warn you each time you use these features, as a reminder that they are not stable and may break in the future.

**NOTE:** This was previously done using a single `RUSTC_BOOTSTRAP` environment variable.
**Please** avoid using `RUSTC_BOOTSTRAP`; it causes the Rust Project maintainers many issues.
These `[workspace.unstable]` features are designed to replace all places where you might need it.

### Enabling features

Each option in the `flags` or `features` table is named after the corresponding flag or feature it enables.
For example, the following config says "enable `-Z allow-moves` and `#![feature(allocator_internals)]`, but not any other feature":
```toml
[workspace.unstable.flags]
allow-moves = true
[workspace.unstable.features]
allocator_internals = true
```
That enables unstable features for all packages in your dependency tree.
You may wish to only enable them for packages in your workspace:
```toml
allow-moves = { workspace = true }
```
or for specific packages in your dependency tree:
```toml
allow-moves = ["cfg-if"]
```

### Disabling features

So far, we've been assuming that you have a stable toolchain, which disallows all features by default.
But you might also use a nightly toolchain, which allows all features by default.
If so, you might wish to ban all features for all packages.
You can do that by having empty `unstable` tables:
```toml
[workspace.unstable.flags]
[workspace.unstable.features]
# end of TOML file
```
Note that there is no way to change a stable toolchain to allow features by default.

## For alternate build systems, or Cargo implementors

The `--unstable-flags` and `--unstable-features` flags allow you to control precisely which unstable flags/features are used by a given crate.
They are supported by rustc, and by all tools shipped with the official toolchain.

Each flag takes one of the following strings as values:
- An empty string, which indicates that no flags/features are allowed (default on stable)
- A comma-separated list of flags/features names.

These flags are specific to the current crate; you can pass different values to different crates and they will interoperate, similar to the `--edition` flag.
This should not be construed to imply that a library crate can opt-in once for the whole build; each crate must opt-in to each feature it uses, or it will get a stability error.

Flags are named after the feature or CLI flag they enable.
By implication, this means that features use underscores (`_`) and CLI flags use dashes (`-`).

Unrecognized flags/features are ignored.
As a quality-of-implementation concern, the tool should warn if an unrecognized flag/feature is passed.

## Build scripts

Build scripts have no mechanism for setting rustc flags (other than `-l` and `-L`) and so cannot use these mechanisms.
This is a feature, not a bug.

Reading these variables from `CARGO_ENCODED_RUSTFLAGS` and using them to do feature detection is allowed, but strongly discouraged.
Be sure to read both `unstable-flags` and `unstable-features` and **compile a real crate** to make sure that your expected usage matches up with the version of the feature that's implemented.
Do *not* simply check whether this is a nightly compiler or not.

## Stability policy

Despite being usable on stable, this is an unstable feature.
Like any other unstable feature, we reserve the right to change or remove this feature in the future, as well as any other unstable feature that it enables.
Using this feature is opting out of the normal stability/backwards compatibility guarantee of stable.

Although we do not take technical measures to prevent it from being used, we strongly discourage using this feature.
If at all possible, please contribute to stabilizing the features you care about instead of bypassing the Rust project's stability policy.

If you do use this to enable an unstable feature, please contact a member of the project who works on the feature in question, so that we know who is exposed to breakage.
For example, if you are using `rustdoc --unstable-flags=output-format`, reach out to [Alona Enraght-Moony][alona] (the maintainer of rustdoc-json).
If you do not know who to contact, ask on [Zulip].
Contacting a maintainer provides no stability guarantees and does not mean the maintainer will agree to work with you,
but can help us find a alternative solution to your problem or otherwise improve the unstable feature you are using.

[alona]: https://github.com/aDotInTheVoid/
[Zulip]: https://rust-lang.zulipchat.com/
# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Caching

Currently, changing `RUSTC_BOOTSTRAP` does not invalidate Cargo's build cache.
With this flag, Cargo will know exactly which crates are affected by each flag,
and can choose to rebuild only the crates it needs to.

Build scripts do not need to explicitly tell Cargo when they are rebuilt;
Cargo should rerun them when it rebuilds their package.

We suggest, but do not require, that flag is made part of the fingerprint tracking, not unit cache tracking,
so that changing the enabled features overwrites the cache rather than adding to it.

## Existing flags

The existing `-Z allow-features` and `-Z unstable-options` flags in Rustc/tools will be removed.
`cargo -Z allow-features` will remain, but it will only apply to Cargo itself, not to any invoked tools.

# Drawbacks
[drawbacks]: #drawbacks

- This encourages using unstable features on stable, explicitly going against our goals as a project.
  But people are doing that anyway, and keeping the status quo does not help us prevent them, while causing many other issues.
  - Note that while this could be seen as encouragement at the *policy* level, it's actually more restrictive at the *technical* level,
    since it requires people to make an exhaustive list of all features they're using.
- Rustfmt is often run automatically by editor plugins, not explicitly.
  Additionally, right now rustfmt warns and continues when a feature gate is enabled on stable, which means the whole codebase gets reformatted.
  We should make sure rustfmt is changed to instead give a hard error when the feature gate is disabled, which will avoid editors accidentally reformatting the whole codebase.
  [The rustfmt team intends to fix this](https://github.com/rust-lang/rustfmt/issues/5022).
- This may make it less likely that people help stabilize features.
  But stabilizing features is [very very hard](https://medium.com/@ElizAyer/organizational-boundary-problems-too-many-cooks-or-not-enough-kitchens-2ddedc6de26a),
  and in the meantime people have very little recourse when they need to use an unstable feature.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- We could allow specifying individual values of a CLI flag, not just the name of the value.
  For example, this could be useful for libtest's `--format` flag, to only allow `--format=json` but not `--format=junit`.
  I think in practice it will not cause issues to lump these together.
  We always have the option to extend the syntax in the future; and because this whole feature is perma-unstable, we have the option to rename existing flags as well.
- We could use environment variables instead of flags.
  This requires no coordination with Cargo and makes the feature seem less "official", which might discourage people from using it.
  But it makes the caching situation much worse, and runs into platform-specific limitations like not being able to set an env var more than one time, or hitting implementation limits on the number of env vars that can be set.
  Flags avoid this by using [response files] and allowing allow-features to be additive.
- We can """simply""" tell people to stop using nightly features on stable (either politely, or with technical measures).
  This will have a large negative impact on the ecosystem -
  rust-analyzer and RustRover will not support running unit tests on stable; `cargo semver-checks` will not work at all on stable; Rust for Linux will break entirely.
- We can leave the status quo.
  This is in many ways the worst of all worlds - people still use unstable features on stable, but in hacky ways that break.
- We can separate the toolchain into "stable" and "unstable" channels, and tell distros to package both.
  This is a big ask from distros, and does not actually help with many of the problems
  (for example cargo semver-checks cannot rely on it being installed, and rust-analyzer will still have caching issues).

This cannot be done in a library or macro.
# Prior art
[prior-art]: #prior-art

- Go has the [`goexperiment` module]. This is enabled at compile time with an environment variable that takes a list of features to enable.
- Java has implementation-specific [`-X` flags][java-x] (which are roughly equivalent to `-Z` flags in Rust). They do not have feature gates. Java also has [preview features], which are guaranteed to exist in all implementations, but require opting in with `--enable-preview` *both* at compile time (with `javac`) and at runtime (with the `java` binary).
- Python distributes separate binaries that [disable the GIL by default][free-threaded python]. Python also has [`-X` flags][python-x], which do not have feature gates.
- Scala allows marking library APIs as [experimental]. Experimental APIs are "infectious" - any code using an experimental API must also be marked as experimental. Additionally, experimental APIs can be upgraded to [preview], meaning that they are guaranteed to exist in the future but might change their exact details. Unlike experimental APIs, preview APIs are not infectious. To enable experimental/preview features for all functions in a module at once, the compiler takes `-experimental`/`-preview` flags.
- Kubernetes allows [enabling features][kubernetes-features] with `--feature-gates=Feature1=true,Feature2=true`. Additionally, it splits features into "Alpha" (experimental, can be removed altogether) and "Beta" (enabled by default, tested, can be changed but not removed).

[`goexperiment` module]: https://pkg.go.dev/internal/goexperiment
[java-x]: https://docs.oracle.com/cd/E13150_01/jrockit_jvm/jrockit/jrdocs/refman/optionX.html
[preview features]: https://docs.oracle.com/en/java/javase/22/language/preview-language-and-vm-features.html
[python-x]: https://docs.python.org/3/using/cmdline.html#cmdoption-X
[free-threaded python]: https://docs.python.org/3/whatsnew/3.13.html#whatsnew313-free-threaded-cpython
[kubernetes-features]: https://kubernetes.io/docs/reference/command-line-tools-reference/feature-gates/
[experimental]: https://docs.scala-lang.org/scala3/reference/other-new-features/experimental-defs.html
[preview]: https://docs.scala-lang.org/scala3/reference/preview/index.html#

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None to my knowledge.

[response files]: https://doc.rust-lang.org/rustc/command-line-arguments.html#path-load-command-line-flags-from-a-path

# Future possibilities
[future-possibilities]: #future-possibilities

- Break hard on RUSTC_BOOTSTRAP now that people have an alternative. For example, we could remove the `RUSTC_BOOTSTRAP=crate_name` syntax and instead require `RUSTC_BOOTSTRAP=<commit_hash>` of the commit rustc was built with. The goal here is for no one to use the variable except bootstrap itself.
- Rename RUSTC_BOOTSTRAP to a name that makes more sense, such as `RUSTC_ALLOW_ALL_FEATURES`.
- We could split unstable features into "alpha" and "beta", and only allow the latter to be enabled with `--unstable-features`.
  Additionally, we could enable beta features by default on the beta channel.
- We could add a version scheme to unstable features, such that the opt-in has to specify exactly which version of the feature it expects
  (and gets a hard error if its expected version doesn't match the version implemented in the compiler).
  The syntax for the opt-in would look like `--unstable-features=allow-moves=2` (3, 4, ...), which is backwards-compatible with the current RFC proposal.
  To encourage project contributors to bump the version, we could remind them (e.g. in a Github comment when a PR is opened) whenever a test that uses the feature is modified.
