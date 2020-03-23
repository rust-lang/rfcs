- Feature Name: `cargo_artifact_specific_dependencies`
- Start Date: 2019-03-24
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Allow specifying dependencies that are only used for one build artifact (`[lib]`, `[[bin]]`, etc) in a single crate.

# Motivation
[motivation]: #motivation

When developing a crate, you may want to have both a library and a binary in the same crate - for example, to implement a command line tool but also to export the functionality as a library so it may be easily used by other developers. This is possible by adding both a `[lib]` and a `[[bin]]` section to the crate's `Cargo.toml`, but both artifacts share the dependencies specified in the `[dependencies]` section. This can be an issue if you want to specify a dependency which is required for the binary, but not the library. With this example, the `clap` crate may be very useful for parsing arguments in the binary, but adding it to the `[dependencies]` section would also mean that it would be included as a dependency when using the library, adding unnecessary bloat.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

A single project may include multiple artifacts - zero or one libraries, and any number of binaries, like the following `Cargo.toml`:

```toml
[package]
name = "myproject"
version = "0.1.0"
edition = "2018"

[dependencies]

[lib]
name = "myproject"
path = "src/lib.rs"

[[bin]]
name = "myproject_cli"
path = "src/main.rs"
```

This project contains a library usable by other crates, but it also includes a command-line interface to allow users to invoke features of the library from the command line.

To streamline the user experience, we may want to use an argument parsing library, lke `clap` or `structopt`. However, adding it as a dependency would add it for all artifacts - including the library, which doesn't use it at all.

To solve this, we can add dependencies for a specific artifact, like so:

```toml
[package]
name = "myproject"
version = "0.1.0"
edition = "2018"

[dependencies]

[lib]
name = "myproject"
path = "src/lib.rs"

[[bin]]
name = "myproject_cli"
path = "src/main.rs"
dependencies = { clap = "*" }
```

Now, `clap` is required for the binary `myproject_cli`, but not for the library `myproject`, and can be used in the same way as crate-wide dependencies. In this example, `clap` could be used from the binary artifact, but attempting to use it from the library artifact would result in an unresolved import error. This has the potential to improve compile time and reduce binary size by omitting dependencies where they're not actually required. 

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This feature would add a new `dependencies` key to the `bin`, `lib`, `test`, `bench`, and `example` sections of the `Cargo.toml` file, describing dependencies that are used only for that artifact, in addition to the ones defined in the top-level `dependencies` and `dev-dependencies` sections. These dependencies would be specified in the same way they would be in the top-level `dependencies` section, allowing version patterns, features, and alternative sources to be specified [as described in the Cargo manual](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html).

When compiling the given artifacts, the additional dependencies would be made available and linked in the same fashion as regular dependencies, allowing them to be imported with `use` and `extern crate` statements. Attempting to use the dependency from other artifacts would fail to resolve the import, raising [E0432](https://doc.rust-lang.org/error-index.html#E0432). A new help message could be added to cargo to make it easier for the user to debug the issue - e.g. `dependency specified for artifact A is not available when compiling artifact B, did you mean to specify it as a crate dependency?`.

When dependencies with the same name are specified as both an artifact-specific dependency and a crate-wide dependency, a new error will be raised by cargo, even if the dependencies are identical, indicating that this isn't allowed. This could be changed in the future, with some ideas described below in the [future possibilities](#future-possibilities) section.

# Drawbacks
[drawbacks]: #drawbacks

- This is a relatively uncommon use case, and there is overlap with the functionality of the `required-features` key.
  - It may be a better idea to improve upon the existing `required-features` functionality (e.g. automatically enabling required features when compiling a specific artifact) than to add another method of configuring dependencies.
- There's also some overlap with the existing functionality to specify `dev-dependencies` - it's hard to imagine many cases where you'd want to have dependencies specific to an individual `test`, `bench`, or `example` that wouldn't be desirable as crate-wide `dev-dependencies`.
- It's more difficult for a reader to determine what dependencies a project uses while reading the `Cargo.toml`.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

While the `required_features` key offers similar functionality, the feature flags must be manually enabled at compile time, and still apply to the entire crate. This adds friction for both project developers, who need to properly separate their dependencies with features and test that these configurations work properly, as well as for users, who need to manually enable the features for their use case.

Specifically for test/bench/example artifacts, the `dev-dependencies` section may be used, but this is only useful for dependencies which are needed by test/bench/example artifacts and not needed by lib/bin artifacts.

# Prior art
[prior-art]: #prior-art

- CMake allows, and in fact, recommends, specifying library dependencies that only apply to a specific build artifact through its `target_link_libraries` command
- Gradle allows different build artifacts to use different source sets/dependency configurations

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- How would this interact with features? Ideally, a feature could be defined that enables an artifact-specific dependency, but how would this affect compilation of other artifacts that don't include this dependency?
- Is cargo written in a way that it's possible to separate dependencies for different artifacts? Currently, using feature flags or `dev-dependencies` to conditionally enable dependencies results in the dependencies being disabled/enabled for all artifacts of the given crate.

# Future possibilities
[future-possibilities]: #future-possibilities

- Specifying an artifact-specific and crate-wide dependency with the same name could be allowed under certain circumstances, such as:
  - Enabling a feature(s) of a dependency for a specific artifact, but not for the entire crate
  - Indicating more specific version requirements for the same dependency as long as the constraints are compatible according to semantic versioning