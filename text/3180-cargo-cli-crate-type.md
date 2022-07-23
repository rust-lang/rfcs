# RFC: Cargo `--crate-type` CLI Argument

- Feature Name: `cargo_cli_crate_type`
- Start Date: 2021-10-07
- RFC PR: [rust-lang/rfcs#3180](https://github.com/rust-lang/rfcs/pull/3180)
- Tracking Issue: [rust-lang/cargo#10083](https://github.com/rust-lang/cargo/issues/10083)

# Summary
[summary]: #summary

Add the ability to provide `--crate-type <crate-type>` as an argument to `cargo rustc`. This would have the same affect of adding `crate-type` in the `Cargo.toml`, while taking higher priority than any value specified there.

[Previous implementation PR](https://github.com/rust-lang/cargo/pull/8789)

# Motivation
[motivation]: #motivation

A crate can declare in its `Cargo.toml` manifest what sort of sort of compilation artifact to produce. However, there are times when the *user* of such a crate, as opposed to the author, would want to alter what artifacts are produced.

Some crates may provide both a Rust API and an optional C API. A current example is the [hyper](https://github.com/hyperium/hyper) crate. Most users of the Rust API only need an `rlib`, so forcing the compilation of a `cdylib` as well is a waste. It can also cause problems for people including such a crate as a dependency when cross-compiling, or when combining with `-C prefer-dynamic` ([example](https://github.com/rust-lang/rust/issues/82151)).

Another usecase is sharing a library across different platforms (e.g. iOS, Android, WASM). iOS requires static linking (`staticlib`) [[1]](https://github.com/rust-lang/cargo/issues/4881#issuecomment-732751642), [[2]](https://github.com/rust-lang/rust/pull/77716), Android and WASM require dynamic linking (`cdylib`) and in order to use it as a dependency in Rust requires `rlib`.

Lastly, being able to pick a specific crate type also decreases build times when you already know which platform you're targeting.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When a user builds a library, using `cargo rustc`, they can provide a `--crate-type` argument to adjust the crate type that is compiled. The argument can be any that can also be listed in the `Cargo.toml`.

Some examples:

```shell
cargo rustc --crate-type staticlib

cargo rustc --crate-type cdylib --features ffi
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

A new command-line argument, `--crate-type`, will be added to Cargo. It must be provided a comma-separated list of 1 or more crate types, of which the allowed values are the same as can be [provided in the manifest](https://doc.rust-lang.org/cargo/reference/cargo-targets.html#the-crate-type-field).

The argument will be added for `cargo rustc`.

As with the existing `crate-type` manifest property, this will only work when building a `lib` or `example`.

If the manifest contains a list, and this new command-line argument is provided by the user, the command-line argument value will override what is in the manifest. For example:

```toml
[lib]
crate-type = ["lib", "staticlib", "cdylib"]
```

```shell
cargo rustc --crate-type staticlib
```

This will produce output only as a `staticlib`, ignoring the other values in the manifest.

# Drawbacks
[drawbacks]: #drawbacks

The usual reasons to not do this apply here:

- An additional feature means more surface area to maintain, and more possibility of bugs.
- The Cargo team is already stretched too thin to ask for another feature. However, in this case, an implementation is already written.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This gives direct control of the compilation to the end user, instead of making them depend on whatever the crate author put in the `Cargo.toml`.

An alternative detail to this proposal is to make it so specifying `--crate-type` adds to the list in the `Cargo.toml`, instead of overriding it. However, there would still be a need for end users to override, so there would need to be an additional argument, such as `--no-default-crate-type`. Overriding feels like the less complex solution for a user to comprehend.

The story around compiling Rust for different targets, and especially in ways that are compatible with C, needs to grow stronger. Choosing not to do this would mean this pain point would continue to exist, which hurts the adoption of writing libraries in Rust instead of C.

# Prior art
[prior-art]: #prior-art

There are a couple similar-looking features already in cargo:

- `--target`: When building a crate, a user can specify the specific target architecture of the compilation output. When not specified, it defaults to the host architecture.
- `--features`: A user can specify a list of features to enable when building a crate directly with `cargo build`. The `Cargo.toml` can provide a default set of features to compile. This differs from the other art, since specifying `--features` will *add* to the default, instead of *overriding* it.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

**Should a user be able to configure the crate-type of _all_ crates in the dependency graph?**

When this feature was first proposed, [it was suggested](https://github.com/rust-lang/cargo/pull/8789#issuecomment-713161246) that a user may wish to configure many dependencies at once, not just the top level crate. This RFC doesn't propose how to solve that, but claims that it can be safely considered out of scope.

First, such a feature is much larger, and there isn't prior art *in Cargo* to have command-line arguments configuring other dependencies. Designing that would take much more effort. There doesn't seem to be resources available to explore that.

Additionally, the small focus of the feature proposed in this RFC doesn't prevent that larger design from being explored and added at a later point. Figuring out a way to specify configuration arguments for other dependencies would likely need to work for the existing `--features` argument. Therefore, this isn't a 1-way-door decision, and thus we don't need to stop fixing this particular pain before that is figured out.

# Future possibilities
[future-possibilities]: #future-possibilities

The command-line argument may be useful for other `cargo` commands in the future. This RFC starts with a conservative list.

It may also be of interest to allow a crate "feature" enable different crate-types, such as `--features ffi` enabling `--crate-type cdylib`.
