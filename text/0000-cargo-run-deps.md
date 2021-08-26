- Feature Name: `cargo-run-deps`
- Start Date: 2021-08-24
- RFC PR: [rust-lang/rfcs#3168](https://github.com/rust-lang/rfcs/pull/3168)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

In addition to binaries from workspace members, teach `cargo run` to execute binaries from dependent packages.

# Motivation
[motivation]: #motivation

Right now, `cargo` lacks a way to run binaries that belong to packages which are not members of the current workspace.

This is desirable for multiple reasons:

- to make binaries from dependencies available to external scripts, especially those for Continuous Integration (CI).
- to ensure an exact version of the binary as specified by the manifest is run, which might differ from the version installed globally.
- to avoid clashes in the global installation directory.
- to make binaries callable from `build.rs`, through `cargo run` acting as a shim.
- to make the build process self-contained, instead of requiring the user a manual installation step.
- to lazily build binary dependencies only when they are needed, thus avoiding needlessly inflating build times.

Several issues related to this request have been opened against `cargo` by users:

- [rust-lang/cargo/issues/2267](https://github.com/rust-lang/cargo/issues/2267): Make commands in dev-dependencies available to run
- [rust-lang/cargo/issues/872](https://github.com/rust-lang/cargo/issues/872): Cargo needs a way to run an executable out of a build-dependency

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Cargo allows you to depend on other crates as part of its manifest. While some crates might define one or more executable binary targets, listing a dependency in your manifest implies building only their library targets.

For instance, you can depend on `mdbook` when building your project as follows:

```toml
[build-dependencies]
mdbook = { version = "0.4.12" }
```

When building your own crate, this will build the `mdbook` crate's library, but not the corresponding binary. But what if you wanted to be able to invoke a binary provided by a dependent crate as a tool to use during build or testing?

If you wanted to invoke `mdbook` inside your `build.rs` script, you could use `cargo run --package mdbook` as a wrapper.

```rust
// build.rs
use std::{env, process::Command};

fn main() {
    let cargo_path = env::var("CARGO").unwrap();
    let mut mdbook = Command::new(cargo_path).args(&[
        "run",
        "--package",
        "mdbook",
        "--",
        "--version",
    ]);

    assert!(mdbook.status().expect("mdbook v0.4.12").success());
}
```

This will take care of (re)building the `mdbook` binary if needed, at the version specified in the manifest, and then invoke it with the passed parameters. As usual, in case multiple binaries are available for one package, the `--bin` option can be passed to select the right executable.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The scope of this RFC is to alter the behavior of `cargo run` so that in addition to binaries part of the workspace, it will allow also to run binaries from direct dependencies which are listed inside the locked manifest for the workspace.

In order to do that, we would relax the constraints of `cargo run` to accept a package id describing any package part of `Cargo.lock` (by virtue of the `--package=<pkgid>` command line parameter). The required steps would be:

- if the specified `pkgid` does not match the package id for any workspace member:
    - use the resolver against the locked manifest to determine direct matching dependency versions. Note that transitional dependencies are excluded.
    - restrict the resulting set to development and build dependencies only, as the main program would not be able to use `cargo run` anyway once installed.
- with the resulting package set:
    - if it is empty, then we fail with an error.
    - if it has a size of more than one, fail with an error alerting the user that there is ambiguity and that they need to specify an exact version as part of the package id. Providing a list of alternatives would be a nice touch.
    - if it has a size of exactly one, then proceed.
- from the resulting package, select either the target specified by the `--bin` option, or the default binary target if none is specified. If we get no match, we fail while alerting the user.
- trigger compilation of the corresponding target with the host system as the target system. This is important when cross-compiling.
- execute the resulting compiled binary, passing the trailing arguments of `cargo run` to it.

# Drawbacks
[drawbacks]: #drawbacks

[rust-lang/rfcs#3028](https://github.com/rust-lang/rfcs/pull/3028) has an overlap with this feature. They are not mutually exclusive and can be implemented mostly independently, but at some point we expect the implementation to require a rework dependending on the whichever is merged first.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Right now, users are required to manually install dependencies to either a global location, or to a dedicated location but then to also manually adjust their `PATH` variable to compensate. In case a global location is used, multiple versions of binaries cannot be installed in parallel.

This RFC keeps binary dependencies built within the workspace's `target/` directory and avoids for them to pollute or overwrite the user global installation.

It also ensures reproducibility of builds, since it avoids a different, potentially incompatible, version of a binary to be picked up by mistake. This is particularly relevant for some tools such as e.g. `mdbook-bib`, which complain if run with a different library version of `mdbook` they were built against as part of the manifest spec.

The design mimicks several of other package managers of related programming languages. See the [prior art](#prior-art) section. As such, it allows newcomers to get a quick and intuitive way to run binaries.

# Prior art
[prior-art]: #prior-art

Many other package managers from other languages provide the same functionality:

- Node.js provides the `npx` binary,
- PHP's Composer has `php composer exec`.
- In Python, several alternatives exist, such as `pipenv run`.
- Ruby's Bundler allows to invoke binaries via `bundle exec`

[rust-lang/rfcs#3028](https://github.com/rust-lang/rfcs/pull/3028) is a more extensive, alternative proposal that also allows embeddding the resulting binary into the crate. This RFC provides a simpler alternative; additionally, it provides a way to run the built binaries from the command line, which is something not explicitly covered by RFC 3028. This is especially useful with CI integration, to consistently bind the buid and tests to the same version of tools as used on the developer's machine. It also lazily allows a selective build of some tools, such as `mdbook`, for those CI scenarios that do not require a full build (for instance, when just wanting to run `cargo doc`).

[rust-lang/cargo/pull/9833](https://github.com/rust-lang/cargo/pull/9833) provides an initial implementation of this RFC, showing that it is likely a localized change; it might make it faster to merge than RFC 3028.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

All open issues were resolved as part of the RFC process.

# Future possibilities
[future-possibilities]: #future-possibilities

By virtue of accepting a fully qualified package id, `cargo run` can be extended to allow running any specific binary crate, regardless of whether it is part of the locked manifest. This would help to run several different versions of a binary in parallel without installing them first, which results in the previous binary to be overwritten. It might be particularly desirable e.g. for regression testing of tools such as `mdbook`, `tarpaulin`, `rusty-hook`, etc.
