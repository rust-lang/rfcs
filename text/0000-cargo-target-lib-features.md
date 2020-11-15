- Feature Name: `cargo_target_lib_features`
- Start Date: 2020-11-15
- RFC PR: [rust-lang/rfcs#3020](https://github.com/rust-lang/rfcs/pull/3020)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Allow specifying features of the implicit lib dependency that need to be enabled by default on non-lib targets (`[[bin]]`, `[[example]]`, etc) in a single crate.

# Motivation
[motivation]: #motivation

When developing a crate, there are several scenarios where the user might want one of the non-library targets to activate certain features in their library. This can either be when one of the examples in the crate is documenting a non-default feature or when a user might want to have both a library and a binary in the same crate - for example, if the user wants to implement a command line tool and also to export the underlying functionality as a library so that it may be easily used by other developers.

The second case is currently possible by adding a `[[bin]]` target to the crate's `Cargo.toml`, and adding any binary specific dependencies (ex: `clap`) as optional dependencies of the library (to reduce unnecessary bloat) under a feature flag (ex: `cli`), and then adding `required-features` to the binary target. But the issue here is that if the end-user does not specify the `cli` feature, then the binary target gets skipped because of how `required-features` is designed.

Similarly, with examples, benches and test targets.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

A single project may include multiple artifacts - zero or one libraries, and any number of other targets, like the following `Cargo.toml`:

```toml
[package]
name = "myproject"
version = "0.1.0"
edition = "2018"

[lib]
name = "myproject"
path = "src/lib.rs"

[[example]]
name = "yaml"
path = "src/examples/yaml.rs"
required-features = ["yaml"]

[dependencies]
yaml-rust = { version = "*", optional = true }

[features]
default = []
yaml = ["yaml-rust"]
```

This project contains a library usable by other crates, and an example that can be run to showcase one of the non-default features of the library.

Running `cargo run --example yaml` in the crate gives us a compiler error since the `yaml` feature of the library has not been activated.

To solve this, we can specify the library features for specific targets, like so:

```toml
[package]
name = "myproject"
version = "0.1.0"
edition = "2018"

[lib]
name = "myproject"
path = "src/lib.rs"

[[example]]
name = "yaml"
path = "src/examples/yaml.rs"
required-features = ["yaml"]
lib-features = ["default", "yaml"]

[dependencies]
yaml-rust = { version = "*", optional = true }

[features]
default = []
yaml = ["yaml-rust"]
```

Now, when a user tries `cargo run --example yaml`, the `yaml` feature and the default features of the library will be implicity activated and thus the example will compile and execute as designed.

For a target, specifying `lib-features` does not implicity activate the default features of the library dependency. If needed, the target can specify `default` in the list of values for `lib-features`.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This feature would add a new `lib-features` key to the `bin`, `test`, `bench`, and `example` sections of the `Cargo.toml` file, a list of strings that represent the features that should be activated for the implicit library dependency.

```toml
[[example]]
name = "yaml"
lib-features = ["yaml"]
```

Adding `lib-features` to a target changes the behaviour of `cargo-run`, `cargo-test` and `cargo-install` subcommands to implicity activate only the described features thus making `no-default-features` flag irrelevant.

# Drawbacks
[drawbacks]: #drawbacks

- None as of yet

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

While the `required-features` key offers similar functionality, the feature flags must be manually enabled at compile time, and still apply to the entire crate. This adds friction for both project developers, who need to properly separate their dependencies with features and test that these configurations work properly, as well as for users, who need to manually enable the features for their use case.

We could automatically enable the required features when compiling that specific target, but it still does not solve the issue of disabling library's default features for that target.

Also, we find that `required-features` and skipping targets has their own niche use cases.

# Prior art
[prior-art]: #prior-art

- Cargo allows users to specify the features that need to be activated for dependencies as described in the [reference](https://doc.rust-lang.org/cargo/reference/features.html#features)

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Currently, using feature flags for some targets results in them being disabled/enabled for all targets of the given crate. Since we will be doing implicit activation of features in this RFC, can we find a way to make them not activated for other targets?

# Future possibilities
[future-possibilities]: #future-possibilities

- Specified features can be used with the new [feature resolver](https://doc.rust-lang.org/cargo/reference/unstable.html#features).
