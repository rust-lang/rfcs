- Feature Name: `allow-unstable-flags`
- Start Date: 2025-05-18
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC proposes decoupling the two components of our stability policy, for CLI flags only:
still requiring feature gates, but allowing feature gates to be enabled on stable.

It does so in two ways:
1. Extend `-Z unstable-options` to take a list of option names, rather than being a simple boolean.
   Then, rename it to `--allow-unstable-flags`. `allow-unstable-flags` is always available, even on stable.
   For example: `rustc --allow-unstable-flags=annotate-moves,binary-dep-depinfo`
2. Add a new `--allow-unstable-flags` flag to Cargo, which propagates it to all invoked commands with proper caching.
   For example: `cargo build --allow-unstable-flags=rustc=annotate-moves --allow-unstable-flags=cargo=build-dir-new-layout`.

This RFC is *not* intended as a general purpose mechanism for Rust developers to use nightly features on stable;
it's specifically targeted at build systems wrapping Rustc or Cargo, such as distro packagers, external tools shipped with the toolchain, and large projects that build a custom Rust toolchain from source.
As such, it does not attempt to address the use of unstable lang features with a stable Rust compiler version, which we consider adequately addressed by `RUSTC_BOOTSTRAP=crate_name`.

# Motivation
[motivation]: #motivation

## Why allow using unstable features on stable?

Rust's stability policy has two components:
1. To the extent possible, each unstable feature comes with its own feature gate, and is disabled when that feature gate is inactive. [^1]
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
- Rust for Linux (RfL) needs a way to build a custom version of core.
  In particular, they mentioned they need to disable float support, because using float registers can cause unsoundness.
  They also have a [much larger list][rfl-wishlist] of all unstable features used; they won't get away from unstable any time soon.
- `rustc_public`'s entire mission is to wrap unstable APIs with stable ones and therefore needs access to all `rustc_private` features.

[rfl-wishlist]: https://github.com/Rust-for-Linux/linux/issues/2

Why are these uses ok? Three reasons:
- Each of these, except for rustc_public, is an external tool, not a library.
  They do not need unstable language features, only unstable tool features.
- Each of these tools accept responsibility for breakage.
  `semver-checks` and RfL both explicitly adapt to each new release of rustc, and their feedback on breakage is very useful for improving the features they use.
  rust-analyzer and RustRover don't break at all for `--print=cfg`—they're not using it in code, only in the CLI—and adapt to any changes in libtest json format.
- These tools act as a "buffer" between other projects and breakage.
  For example, `semver-checks` hides the breaking changes behind its own interface such that downstream projects are not affected.
  Similarly, RfL backports breakage fixes to stable branches such that old versions of the kernel keep building with new rust toolchains.

One might ask, well, maybe we are being too eager to gate things, but can't people just use nightly?
There are some cases where switching to nightly is not realistic.
- When using rustc packaged by a distro (e.g. Fedora or `nixpkgs`), only the stable channel is packaged.
- Tools that wrap the compiler (e.g. `rust-analyzer` or `cargo expand`) or libraries (e.g. `proc-macro2`) usually do not control the toolchain version being used.
- Nightly is not the same as stable-with-`RUSTC_BOOTSTRAP`.
  In particular, stable contains backports and nightly does not.

## Why this exact mechanism?

Currently, these tools use [`RUSTC_BOOTSTRAP=1`][rustc-bootstrap] as a workaround.
But enabling RUSTC_BOOTSTRAP for one part of the toolchain enables it for *all* parts of the toolchain; in particular:
- `proc-macro2` uses `cargo:rerun-on-env-changed=RUSTC_BOOTSTRAP`, causing cache thrashing whenever this env var changes.
- rust-analyzer wants to enable `RUSTC_BOOTSTRAP` only for cargo and libtest, but the variable enables features for rustc as well.
  `RUSTFLAGS="-Z allow-features="` fixes this for lang features, but at the price of thrashing the cache; and there is no equivalent way to disable unstable CLI features.

`--allow-unstable-features` extends this to CLI features, allowing projects to opt-in to only the unstability they choose to.

[rustc-bootstrap]: https://doc.rust-lang.org/nightly/unstable-book/compiler-flags/rustc-bootstrap.html

An important design constraint here is that the "end-user" (whoever is running the build) should always have control over which features are enabled.
To the extent that tools act as a "buffer" between feature breakage and the end-user,
they should only take responsibility for exactly the features whose breakage they know how to handle.

We mark language features out of scope, because we expect the new flags to reduce most of the use cases for RUSTC_BOOTSTRAP, and therefore to reduce the amount of needless cache thrashing going on.
We do not want to decouple our stability policy for language features,
because there's no possibility there of the library acting as a buffer between other projects and breakage[^2].

[^2]: Theoretically libraries can write a build script that does feature detection, but this slows down the build for everyone, and it's very very hard to write that build script properly.

## Why change Rustfmt?

Rustfmt is often run automatically by editor plugins, not explicitly.
Additionally, right now rustfmt warns and continues when a feature gate is enabled on stable, which means the whole codebase gets reformatted.
Changing Rustfmt to instead give a hard error when the feature gate is disabled avoids editors accidentally reformatting the whole codebase.
[The rustfmt team already intends to fix this](https://github.com/rust-lang/rustfmt/issues/5022).

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The following documentation will live in the [unstable book] (or Cargo's [unstable features][cargo-unstable] section), not in the [rustc book].

[unstable book]: https://doc.rust-lang.org/nightly/unstable-book/
[cargo-unstable]: https://doc.rust-lang.org/cargo/reference/unstable.html#allow-features
[rustc book]: https://doc.rust-lang.org/nightly/rustc/

The `--allow-unstable-flags` Rustc option allows you to control precisely which unstable options are used by a given crate.
It's supported by rustc, and by all tools shipped with the official toolchain.
For example, `rustdoc --allow-unstable-flags=output-format --output-format=json` allows you to see Rustdoc's JSON output on stable.

Flags are named after the CLI flag they enable.
By implication, this means that CLI flags use dashes (`-`).
Flag values are not supported, only names.

The `--allow-unstable-flags` Cargo option is almost the same, but instructs Cargo which tools need to receive the option.
For example, `cargo doc --allow-unstable-flags=rustdoc=output-format` will run `rustdoc --allow-unstable-flags=output-format`.
You can use `cargo build --allow-unstable-flags=cargo=profile-hint-mostly-unused` to allow a flag in Cargo itself.

## Stability policy

Despite being usable on stable, this is an unstable feature.
Like any other unstable feature, we reserve the right to change or remove this feature in the future, as well as any other unstable feature that it enables.
Using this feature is opting out of the normal stability/backwards compatibility guarantee of stable.

Although we do not take technical measures to prevent it from being used, we strongly discourage using this feature.
If at all possible, please contribute to stabilizing the features you care about instead of bypassing the Rust project's stability policy.

If you do use this to enable an unstable feature, please contact a member of the project who works on the feature in question, so that we know who is exposed to breakage.
For example, if you are using `rustdoc --allow-unstable-flags=output-format`, reach out to [Alona Enraght-Moony][alona] (the maintainer of rustdoc-json).
If you do not know who to contact, ask on [Zulip].
Contacting a maintainer provides no stability guarantees and does not mean the maintainer will agree to work with you,
but can help us find an alternative solution to your problem or otherwise improve the unstable feature you are using.

[alona]: https://github.com/aDotInTheVoid/
[Zulip]: https://rust-lang.zulipchat.com/

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Shared rules

`allow-unstable-flags` is a comma-separated list of CLI flag names, with the leading `-Z` or `--` (if any) removed.
Flag values are not supported, only names.

`allow-unstable-flags` does not activate any flag on its own.
You still need to combine it with the `-Z` or unstable flag that you wish to enable.

`allow-unstable-flags` is accepted on all channels.
When it's not present, the default on stable/beta is to ban all unstable flags,
and the default on nightly is to allow all unstable flags.

Unrecognized flag names in `allow-unstable-flags` are a hard error.

## Non-cargo rules

Stable/beta Rustfmt now errors instead of warning when an unstable option is set without also setting `--allow-unstable-flags`.

libtest runners accept `--allow-unstable-flags`.

Each non-Cargo flag takes one of the following strings as a value:
- An empty string, which indicates that no flags are allowed (default on stable)
- A comma-separated list of flag names.

If `allow-unstable-flags` is passed multiple times, the *intersection* of all values is used, not the union.
This matters in cases where two parties don't trust each other, such as running `cargo build --allow-unstable-flags=rustc=x` in a workspace with `build.rustflags=--allow-unstable-flags=x,y`: this should be equivalent to `rustc --allow-unstable-flags=x`.

## Cargo rules

Build scripts cannot set these flags; `cargo::rustc-flags` continues to only accept `-l` and `-L` flags.

Each Cargo flag takes a value that starts with a tool name, then the string '=', then a valid value for a non-Cargo flag.
The tool can be `cargo`, in which case the flag applies to the unstable flags of Cargo itself.
Tool names are the name of the exact binary that will be spawned: `rustdoc`, `clippy-driver`, etc.
If `RUSTC` or `RUSTDOC` is set, the tool name is still `rustc`/`rustdoc`, not the overridden value.
If `RUSTC_WRAPPER` or `RUSTC_WORKSPACE_WRAPPER` is set, the intersection of the flags for `rustc` and the wrapper are passed; this requires additional work from the user but avoids silently passing unstable flags to more tools than intended.


If `allow-unstable-flags` is passed multiple times, tools are unioned, but values are intersected.
In other words, `cargo doc --allow-unstable-flags=rustc=x --allow-unstable-flags=rustdoc=y` will pass `--allow-unstable-flags=x` to Rustc and `--allow-unstable-flags=y` to Rustdoc.

`cargo <cmd> --allow-unstable-flags` applies to both Cargo and all tools it spawns.
That is, it passes the flag to Rustc when Rustc is spawned.
This applies to all packages, not just the current workspace.
In practice, the only tool passing unstable flags to Cargo is `cargo-semver-checks`, which is building all dependencies in any case.

Unrecognized tool names are an error.

## Implementation notes

### Caching

Currently, changing `RUSTC_BOOTSTRAP` does not invalidate Cargo's build cache by itself, but in practice can cause build scripts deep in the dependency tree to re-run.
With `--allow-unstable-flags`, we separate the mechanism for enabling flags from the mechanism for enabling lang features,
greatly decreasing how often build scripts that detect `RUSTC_BOOTSTRAP` rerun.

We suggest, but do not require, that the flag is made part of the fingerprint tracking, not unit cache tracking,
so that changing the enabled features overwrites the cache rather than adding to it.

# Drawbacks
[drawbacks]: #drawbacks

- This encourages using unstable features on stable, explicitly going against our goals as a project.
  But people are doing that anyway, and keeping the status quo does not help us prevent them, while causing many other issues.
  - Note that while this could be seen as encouragement at the *policy* level, it's actually more restrictive at the *technical* level,
    since it requires people to make an exhaustive list of all features they're using.
- This may make it less likely that people help stabilize features.
  But stabilizing features is [very very hard](https://medium.com/@ElizAyer/organizational-boundary-problems-too-many-cooks-or-not-enough-kitchens-2ddedc6de26a),
  and in the meantime people have very little recourse when they need to use an unstable feature.
- This does not address the use case of lang features.
  Lang features have several drawbacks right now; for example:
  - It's possible to enable/disable features for individual crates, or for individual features, but not both at once.
    We could address this by stabilizing `-Zallow-features`, but still requiring it to be used in combination with `RUSTC_BOOTSTRAP=crate_name` (and possibly removing the `RUSTC_BOOTSTRAP=1` form).
  - Enabling/disabling lang features causes large parts of the build graph to be rebuilt.
    I do not have ideas for how to fix this; the closest I got was a `[workspace.unstable]` table in Cargo.toml,
    which would only be read with `cargo build --unstable-features` and passed through `-Zallow-features` to Rustc,
    but this seems too corrosive to our stability policy to encourage.

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
- Nix allows enabling features [in `nix.conf`](https://nix.dev/manual/nix/2.34/command-ref/conf-file.html#conf-accept-flake-config).
  In practice, this results in people widely using features throughout the ecosystem.
  We take this as a lesson telling us that an opt-in in `[workspace.unstable]` is not sufficient,
  that there needs to be a reminder on each command that unstable features are active.

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

- Will this cause flags to be de-facto stable, even moreso than they are now?
  We could emit a future-compat warning whenever this flag is used; is that sufficient?
- This RFC frames `--allow-unstable-flags` as an unstable feature.
  Can we follow through on that in practice?
  How badly will things break if we eventually remove it?
- Should we allow `name=value` filtering from the start, rather than deferring it to an extension?

[response files]: https://doc.rust-lang.org/rustc/command-line-arguments.html#path-load-command-line-flags-from-a-path

# Future possibilities
[future-possibilities]: #future-possibilities

- Rename RUSTC_BOOTSTRAP to a name that makes more sense, such as `RUSTC_ALLOW_ALL_FEATURES`.
- We could split unstable flags into "alpha" and "beta", and only allow the latter to be enabled with `--allow-unstable-flags`.
  Additionally, we could enable beta flags by default on the beta channel.
- We could add a version scheme to unstable flags, such that the opt-in has to specify exactly which version of the feature it expects
  (and gets a hard error if its expected version doesn't match the version implemented in the compiler).
  The syntax for the opt-in would look like `--allow-unstable-flags=output-format@2` (3, 4, ...), which is backwards-compatible with the current RFC proposal.
  To encourage project contributors to bump the version, we could remind them (e.g. in a Github comment when a PR is opened) whenever a test that uses the feature is modified.
