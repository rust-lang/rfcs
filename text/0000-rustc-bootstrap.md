- Feature Name: `RUSTC_ALLOW_UNSTABLE`
- Start Date: 2025-05-18
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC proposes decoupling the two components of our stability policy: still requiring feature gates, but allowing feature gates to be enabled on stable. It does so by adding `RUSTC_ALLOW_UNSTABLE_<feature_name>` environment variables which can be used to permit using feature gates on stable toolchain channels. These variable are respected by all official Rust tools that use feature gates.

This RFC is *not* intended as a general purpose mechanism for Rust developers to use nightly features on stable; it's specifically targeted at build systems wrapping cargo, such as distro packagers, external tools shipped with the toolchain, and large projects that build a custom Rust toolchain from source.

# Motivation
[motivation]: #motivation

## Why allow using unstable features on stable?

Rust's stability policy has two components:
1. To the extent possible, each unstable feature comes with a feature gate, and is disabled when that feature gate is inactive. [^1]
2. Enabling feature gates is only allowed on the nightly toolchain.

[^1]: There are some exceptions to this, such as https://github.com/rust-lang/rust/issues/139892#issuecomment-2808505610. But in general we attempt to make sure all unstable features have a feature gate.

Our motivation for 1 (having feature gates) is to make sure that people do not unknowingly rely on unstable features. This was a big problem for e.g. intra-doc links, which [people often used without knowing they were unstable][63305], making it impossible to remove the feature.

[63305]: https://github.com/rust-lang/rust/issues/63305

Our motivation for 2 (disabling feature gates on stable) is three-fold:
1. Prevent people from relying on features that may change in the future while on the stable toolchain, upholding our "stability without stagnation" motto.
2. Disallow library authors from "silently" opt-ing in to unstable features, such that the person running the top-level build doesn't know they're using unstable features that may break when the toolchain is updated. This rationale doesn't apply to nightly, where the party running the top-level build is assumed to know that nightly comes with no stability guarantees.
3. Encourage people to help stabilize the features they care about.

There are some cases in which none of those goals are applicable, but we still prevent people from using nightly features. This is particularly bad when projects *must* depend on unstable features to ship another feature they care about. Some examples:
- rust-analyzer and RustRover need `./some-libtest-binary --format=json` to determine the list of possible tests to run
- rust-analyzer and RustRover need all values in `rustc --print=cfg` to build the standard library (see [#139892](https://github.com/rust-lang/rust/issues/139892#issuecomment-2808505610) for an explanation of why this is affected by unstable features)
- `cargo semver-checks` needs `rustdoc --output-format=json` in order to work at all
- Rust for Linux needs a way to build a custom version of core. In particular, they mentioned they need to disable float support, because using float registers can cause unsoundness.

Why are these uses ok? Two reasons:
- Each of these tools accept responsibility for breakage. `semver-checks` and RfL both explicitly adapt to each new release of rustc, and their feedback on breakage is very useful for improving the features they use. rust-analyzer and RustRover don't break at all for `--print=cfg`—they're not using it in code, only in the CLI—and adapt to any changes in libtest json format.
- These tools act as a "buffer" between other projects and breakage. For example, semver-checks hides the breaking changes behind its own interface such that downstream projects are not affected. Similar, Rust for Linux backports breakage fixes to stable branches such that old versions of the kernel keep building with new rust toolchains.

One might ask, well, maybe we are being too eager to gate things, but can't people just use nightly? There are some cases where switching to nightly is not realistic.
- When using rustc packaged by a distro (e.g. Fedora or `nixpkgs`), only the stable channel is packaged.
- Tools that wrap the compiler (e.g. `rust-analyzer` or `cargo expand`) or libraries (e.g. `proc-macro2`) usually do not control the toolchain version being used.
- Stable with `RUSTC_BOOTSTRAP` is not the same as nightly. In particular, stable contains backports and nightly does not.

## Why this exact mechanism?

Currently, these tools use [`RUSTC_BOOTSTRAP=1`][rustc-bootstrap] as a workaround. But this workaround has many downsides:

[rustc-bootstrap]: https://doc.rust-lang.org/nightly/unstable-book/compiler-flags/rustc-bootstrap.html

- Enabling RUSTC_BOOTSTRAP for one part of the toolchain enables it for *all* parts of the toolchain; in particular:
    - `proc-macro2` uses `cargo:rerun-on-env-changed=RUSTC_BOOTSTRAP`, causing cache thrashing whenever this env var changes. <!--Using different env variables makes it so that we only rebuild when the feature the build script actually cares about changes.-->
    - rust-analyzer wants to enable RUSTC_BOOTSTRAP only for cargo and libtest, but the variable enables features for rustc as well. `RUSTFLAGS="-Z allow-features="` fixes this for lang features, but at the price of thrashing the cache; and there is no equivalent way to disable unstable CLI features.
- Libraries that detect RUSTC_BOOTSTRAP sometimes do it incorrectly (in particular, `-Zallow-features` often messes things up). To do this correctly, one must compile a full rust program that uses the api the library wants to enable; but in practice doing this is rare. Using more specific environment variables makes it less likely that a single misbehaving library can break the whole build.

An important design constraint here is that the "end-user" (whoever is running the build) should always have control over which features are enabled. To the extent that tools act as a "buffer" between feature breakage and the end-user, they should only take responsibility for exactly the features whose breakage they know how to handle.

We continue to use environment variables because that only permits the top-level build to allow feature gates. Cargo has `[env]` blocks, but they are only enabled for the top-level build, and it's usually easy for wrapping build systems to ignore `.cargo/config.toml` files. As a side effect, `[env]` makes it easy for developers to experiment locally with nightly features on a stable toolchain, but without allowing them to opt-in silently when their library is published to crates.io. This seems good, actually.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The following documentation will live in the [unstable book], not in the [rustc book].

[unstable book]: https://doc.rust-lang.org/nightly/unstable-book/
[rustc book]: https://doc.rust-lang.org/nightly/rustc/

The `RUSTC_ALLOW_UNSTABLE_*` environment variables allow you to use unstable features on the stable and beta channels. In particular, it allows you to use `-Z` flags and `#![feature(..)]` attributes.

**NOTE:** This was previously done using a single `RUSTC_BOOTSTRAP` environment variable. **Please** avoid using `RUSTC_BOOTSTRAP`; it causes the Rust Project maintainers many issues. These variables are designed to replace all places where you might need it.

Each variable is named after the feature or CLI flag it enables. Tools are pseudo-namespaced. For example, `RUSTC_ALLOW_UNSTABLE_VALIDATE_MIR=1` allows using `rustc -Z validate-mir`, `RUSTC_ALLOW_UNSTABLE_ASM_UNWIND` allows using `#![feature(asm_unwind)]`, and `RUSTC_ALLOW_UNSTABLE_RUSTDOC_OUTPUT_FORMAT` allows using `rustdoc --output-format=json`.

Each variable takes one of three arguments:
1. `1` indicates that all crates can use the feature. This is the default on nightly.
2. `-1` indicates that no crates can use the feature. This is the default on stable and beta, but can be useful when using a nightly toolchain.
3. `ident` indicates that only the crate named `ident` can use the feature. Multiple crates can be specified at once by separating them with commas; for example, `RUSTC_ALLOW_UNSTABLE_ASM_UNWIND=tokio,hyper` allows specifying `#![feature(asm_unwind)]` in the `tokio` and `hyper` crates.

Note that on most platforms, it is impossible to set an environment variable more than once, so be careful not to overwrite any existing variable.

## Stability policy

Despite being usable on stable, this is an unstable feature. Like any other unstable feature, we reserve the right to change or remove this feature in the future, as well as any other unstable feature that it enables. Using this feature is opting out of the normal stability/backwards compatibility guarantee of stable.

Although we do not take technical measures to prevent it from being used, we strongly discourage using this feature. If at all possible, please contribute to stabilizing the features you care about instead of bypassing the Rust project's stability policy.

If you do use this to enable an unstable feature, please contact a member of the project who works on the feature in question, so that we know who is exposed to breakage. For example, if you are using `RUSTC_ALLOW_UNSTABLE_RUSTDOC_OUTPUT_FORMAT=1`, reach out to [Alona Enraght-Moony][alona] (the maintainer of rustdoc-json). If you do not know who to contact, ask on [Zulip]. Contacting a maintainer provides no stability guarantees and does not mean the maintainer will agree to work with you, but can help us find a alternative solution to your problem or otherwise improve the unstable feature you are using.

[alona]: https://github.com/aDotInTheVoid/
[Zulip]: https://rust-lang.zulipchat.com/
# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Each environment variable name is determined as follows:
1. Start with the prefix `RUSTC_ALLOW_UNSTABLE_`
2. Add the name of the tool in uppercase followed by an underscore, if present. For example, clippy would append `CLIPPY_`. For the purpose of this RFC, any generated libtest binaries are counted as a tool and append `TEST_`.
3. Append the feature name.
    a. For language and library features, append the name of the feature gate in uppercase. For example, `#![feature(asm_unwind)]` would append `ASM_UNWIND`.
    b. For cli flags, append the name of the flag in uppercase, excluding any `-Z` prefix, and replacing dashes with underscores. For example, `-Z validate-mir` would append `VALIDATE_MIR`.

As a quality-of-implementation concern, the tool may warn when a `crate_name` passed to an environment variable is not a valid Rust identifier (this may happen if, e.g., a cargo package name is used instead of the proper crate name).

As a quality-of-implementation concern, the tool should warn when an unrecognized feature is permitted.

As a quality-of-implementation concern, the compiler should verify (i.e. through testing, not at runtime) that no CLI flag name would cause its environment variable to overlap with a feature gate.
Existing feature gates that cause such a conflict should be renamed.
For example, `-Z ub-checks` and `feature(ub_checks)` cause an overlap under this proposal; `feature(ub_checks)` should be renamed to avoid the overlap.
Note that this is only relevant to the compiler, since other tools are already pseudo-namespaced and can't have conflicts.

As a quality-of-implementation concern, the compiler should verify (through testing) that there are no conflicts between a compiler feature and an official tool feature; for example, it should verify that `feature(rustdoc_internals)` does not conflict with a rustdoc flag named `-Z internals`.

# Drawbacks
[drawbacks]: #drawbacks

- This encourages using unstable features on stable, explicitly going against our goals as a project. But people are doing that anyway, and keeping the status quo does not help us prevent them, while causing many other issues.
- Rustfmt is often run automatically by editor plugins, not explicitly. Additionally, right now rustfmt warns and continues when a feature gate is enabled on stable, which means the whole codebase gets reformatted. We should make sure rustfmt is changed to instead give a hard error when the environment variable is missing, which will avoid editors accidentally reformatting the whole codebase. [The rustfmt team intends to fix this](https://github.com/rust-lang/rustfmt/issues/5022).
- This may make it less likely that people help stabilize features. But stabilizing features is [very very hard](https://medium.com/@ElizAyer/organizational-boundary-problems-too-many-cooks-or-not-enough-kitchens-2ddedc6de26a), and in the meantime people have very little recourse when they need to use an unstable feature.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- We could allow specifying individual values of a CLI flag, not just the name of the value. For example, this could be useful for libtest's `--format` flag, to only allow `--format=json` but not `--format=junit`. I think in practice it will not cause issues to lump these together. We always have the option to add more variables in the future; and because this whole feature is perma-unstable, we have the option to rename existing vars as well.
- We could have a single environment variable which takes the feature name as a value. That does not allow composing the variables - setting it in one place breaks all other places that set it. Additionally, it does not allow build scripts to do fine-grained caching, because cargo only has `rerun-on-env-changed` and not anything more granular.
- We could use CLI flags instead of an environment variable. That breaks caching, because rust-analyzer cannot pass `RUSTFLAGS=-Zallow-features` without cargo rebuilding, and the cargo team does not wish to inspect the contents of RUSTFLAGS.
- We can """simply""" tell people to stop using nightly features on stable (either politely, or with technical measures). This will have a large negative impact on the ecosystem - rust-analyzer and RustRover will not support running unit tests on stable; `cargo semver-checks` will not work at all on stable; Rust for Linux will break entirely.
- We can leave the status quo. This is in many ways the worst of all worlds - people still use unstable features on stable, but in hacky ways that break.
- We can separate the toolchain into "stable" and "unstable" channels, and tell distros to package both. This is a big ask from distros, and does not actually help with many of the problems (for example cargo semver-checks cannot rely on it being installed, and rust-analyzer will still have caching issues).

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
- Will this run into platform-specific limitations on [env variable lengths][limits.h] when many env variables are passed? The "minimum maximum" is 4096 but in practice most platforms seem to be much higher. Unlike flags, environment variables cannot be passed in [response files].

[limits.h]: https://pubs.opengroup.org/onlinepubs/009695399/basedefs/limits.h.html#:~:text=ARG_MAX
[response files]: https://doc.rust-lang.org/rustc/command-line-arguments.html#path-load-command-line-flags-from-a-path

# Future possibilities
[future-possibilities]: #future-possibilities

- Break hard on RUSTC_BOOTSTRAP now that people have an alternative. For example, we could remove the `RUSTC_BOOTSTRAP=crate_name` syntax and instead require `RUSTC_BOOTSTRAP=<commit_hash>` of the commit rustc was built with. The goal here is for no one to use the variable except bootstrap itself.
- Rename RUSTC_BOOTSTRAP to a name that makes more sense, such as `RUSTC_ALLOW_ALL_FEATURES`.
- We could split unstable features into "alpha" and "beta", and only allow the latter to be enabled with `RUSTC_ALLOW_UNSTABLE`. Additionally, we could enable beta features by default on the beta channel.
- We could add a version scheme to unstable features, such that the opt-in has to specify exactly which version of the feature it expects (and gets a hard error if its expected version doesn't match the version implemented in the compiler). The syntax for the opt-in would look like `RUSTC_ALLOW_UNSTABLE_NAME=2` (3, 4, ...), which is backwards-compatible with the current RFC proposal. To encourage contributors to bump the version, we could remind them (e.g. in a Github comment when a PR is opened) whenever a test that uses the feature is modified.
