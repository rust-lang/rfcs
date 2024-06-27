- Feature Name: -
- Start Date: 2022-11-07
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add a flag to cargo that allows building and running only compile time dependencies for the host platform.

# Motivation
[motivation]: #motivation

IDEs require building and running proc-macros and build scripts to be able to fully analyze the source code of cargo projects, as these artifacts either generate code (proc-macro expansions, `include!`d build script outputs) or affect source code inclusion (`#[cfg]` predicate dependent on build-script set flags).
Aside from building and running these things, this is also needed for the IDE to figure out things like a package's `OUT_DIR` location, or the file path of a build proc-macro which is read from cargo's metadata output.

IDEs can already do this with `cargo` as is, by just running `cargo check` as this builds proc-macros and *runs* build scripts already, but this also checks the rest of the project which for this step depending on the project in question may be quite costly using up time and resources for something the IDE is not even interested in at this point.
This also has the additional downside that this pollutes the metadata output with compiler diagnostics of the project, so if the project fails to build, but the proc-macros and build scripts work fine, the IDE now sees an error in this step where none should be, possibly unnecessarily poking the user about this step failing even though it technically didn't.

It is possible to side-step these issues by making use of the [`RUSTC_WRAPPER`](https://doc.rust-lang.org/cargo/reference/config.html#buildrustc-wrapper) env variable.
This is what the major IDE projects (rust-analyzer and the intellij plugin) currently employ, what rust-analyzer does specifically is roughly the following:

When invoking `cargo check` to build proc-macros, run build-scripts and query metadata for the workspace, r-a first sets the `RUSTC_WRAPPER` to point to [itself](https://github.com/rust-lang/rust-analyzer/blob/187bee0bb100111466a3557c20f80defcc0f4db3/crates/project-model/src/build_scripts.rs#L109-L116).
When r-a acts as a wrapper, it just simply inspects the rustc commands cargo tries to invoke and if it notices that this is a command that doesn't produce relevant output, that is its just a crate check it unconditional succeeds it to skip it from running.
It's a bit more complicated as the wrappers needs to check for some env var set by cargo that it only sets for build scripts, to prevent us from succeeding build scripts that try to [probe rustc](https://github.com/rust-lang/rust-analyzer/issues/12973) for whether a snippet of code succeeds.
The current wrapper implementation can be seen [here](https://github.com/rust-lang/rust-analyzer/blob/187bee0bb100111466a3557c20f80defcc0f4db3/crates/rust-analyzer/src/bin/rustc_wrapper.rs).
This does work in practice, but it's not really robust (as can be seen by the probing issue that took some effort to figure out for the involved parties) and it makes use of features in a way that isn't intended.[^1]

[^1]: It also seems to [confuse cargo in rare occasions](https://github.com/rust-lang/rust-analyzer/pull/12808#issuecomment-1190072110), where it starts thinking that a project checks successfully, even though it doesn't after the wrapper already isn't used anymore.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Running `cargo build --compile-time-deps` unlike like `cargo build`, will only build and run artifacts that are relevant for the host platform.
This encompasses building proc-macros the workspace depends on as well as building and running build-scripts of the packages.
Future invocations of `cargo build` will not need to re-build and re-run these artifacts if they were built with the same environment.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Add a flag `--compile-time-deps` to `cargo build` that will cause cargo to only build proc-macros and build-scripts as well as running the build-scripts.
In case of the ecosystem introducing more compile time (executable) artifacts that are required to fully build a project, these artifacts should be included by this flag accordingly.

- [Package Selection](https://doc.rust-lang.org/cargo/commands/cargo-build.html#package-selection) will limit the built artifacts to the selected packages and their dependencies.
- [Target Selection](https://doc.rust-lang.org/cargo/commands/cargo-build.html#target-selection) will limit the built artifacts to the selected targets and their dependencies. This only has a noticeable effect for proc-macro packages, where not selecting their lib target will skip building it.
- [Feature Selection](https://doc.rust-lang.org/cargo/commands/cargo-build.html#feature-selection) apply the selected features to the build as usual.
- [Compilation Options](https://doc.rust-lang.org/cargo/commands/cargo-build.html#compilation-options) have no effect on the compiled artifacts. This is the same as a plain `cargo build` where build scripts and proc-macros are always built for host platform.
- [Output Options](https://doc.rust-lang.org/cargo/commands/cargo-build.html#output-options) affect the locations of the generated artifacts as usual.
- [Display Options](https://doc.rust-lang.org/cargo/commands/cargo-build.html#display-options)  affect the build as usual.
- [Manifest Options](https://doc.rust-lang.org/cargo/commands/cargo-build.html#manifest-options) affect the build as usual.
- [Miscellaneous Options](https://doc.rust-lang.org/cargo/commands/cargo-build.html#miscellaneous-options) affect the build as usual.

Note "as usual" here refers to the behavior of a `cargo build` invocation without the proposed flag.

# Drawbacks
[drawbacks]: #drawbacks

This adds a specialized flag to cargo, whose main purpose would be in use for IDEs.
The scope of this flag is somewhat unclear, what exactly fits the "compile time dependency" terminology, what does not.
It might be that in the future, cargo introduces a feature that IDEs have to somehow handle to continue to be able to fully analyze source code, but that does not fit into this flag which might need to a new command or flag to succeed this one, although the author believes this to be very unlikely.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The current alternatives are the `RUSTC_WRAPPER` or a plain `cargo check` invocation as has been outlined in the motivation section.
- The `RUSTC_WRAPPER` mostly works today, but this is not necessarily its intended usage, and it is unknown what other problems such a wrapper unconditionally succeeding invoked commands could cause next.
- The plain `cargo check` is not a decent option for IDEs, as the IDE loses the ability to discern between build-script problems and workspace diagnostics.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Should this be a flag for `cargo build`, or does it require its own subcommand?
- Compilation options do not affect build scripts or proc-macros in `cargo build` invocations, likewise the proposal for the flag here therefor defines the same. Is there a use case where it would make sense to allow compilation options to affect these artifacts when the proposed flag is supplied?
- Does the flag name fit or is there a better name for this?
- Interaction with a potential future feature for wasm compiled build dependencies
