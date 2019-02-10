- Feature Name: cargo_the_std_awakens
- Start Date: 2018-02-09
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Currently, the `core` and `std` components of Rust are handled in a different way than Cargo handles other crate dependencies. This causes issues for non-mainstream targets, such as WASM, Embedded, and new not-yet-tier-1 targets. The following RFC proposes a roadmap to address these concerns in a consistent and incremental process.

# Motivation
[motivation]: #motivation

In today's Rust environment, `core` and `std` are shipped as precompiled objects. This was done for a number of reasons, including faster compile times, and a more consistent experience for users of these dependencies. This design has served fairly well for the bulk of users, however there are a number of less common, but not esoteric uses, that are not well served by this approach. Examples include:

* Supporting new/arbitrary targets, such as those defined by a ".json" file
* Making modifications to `core` or `std` through use of feature flags
* Users who would like to make different optimizations to `core` or `std`, such as `opt-level = 'z'`, with `panic = "abort"`

Previously, these needs were somewhat addressed by the external tool [xargo], which managed the recompilation of these dependencies when necessary. However, this tool has become [deprecated], and even when supported, required a nightly version of the compiler for all operation.

This approach has [gathered support] from various [rust team members], and this RFC aims to take inspiration from tools and workflows like [xargo], and integrate them into Cargo itself.

[xargo]: https://github.com/japaric/xargo
[deprecated]: https://github.com/japaric/xargo/issues/193
[gathered support]: https://github.com/japaric/xargo/issues/193#issuecomment-359180429
[rust team members]: https://www.ncameron.org/blog/cargos-next-few-years/

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This proposal aims to make `core` and `std` feel a little bit less like a special case compared to other dependencies to the end users of Cargo. This will be achieved by using idioms used for interacting, configuring, modifying, and patching other crates in a similar method for `core` and `std`.

This RFC proposes the following concrete changes, which may or may not be implemented in this order, and may be done incrementally. The details and caveats around these stages are discussed in the Reference Level Explanation.

1. Allow developers of root crates to recompile `core` (and `compiler-builtins`) when their desired target does not match one available as a `rustup target add` target, without the usage of a nightly compiler. This version of `core` would be built from the same source files used to build the current version of `rustc`/`cargo`.
2. Introduce the concept of "stable features" for `core`, which allow the end user to influence the behavior of their custom version of `core`, without the use of a nightly compiler.
3. Extend the new behaviors described in step 1 and 2 for `std` (and `alloc`).
4. Allow the user to provide their own custom source versions of `core` and `std`, allowing for deep customizations when necessary. This will require a nightly version of the compiler.

As a new concept, the items above propose the existence of "stable features" for `core` and `std`. These features would be considered stable with the same degree of guarantees made for stability in the rest of the language. These features would allow configuration of certain functionalities of `core` or `std`, in a way decided at compile time.

For example, we could propose a feature called `force-tiny-fmt`, which would use different algorithms to implement `fmt` for use on resource constrained systems. The developer of the root crate would be able to choose the default behavior, or the `force-tiny-fmt` behavior while still retaining the ability of using a stable compiler.


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

A reference-level explanation is made for each of the items enumerated above.

## 1 - Allow developers of root crates to recompile `core`

### Use Case

For developers working with new targets not yet supported by the Rust project, this feature would allow the compilation of `core` for any target that can be specified as a valid target json format.

This functionality would be possible even with the use of a stable compiler.

Users of a nightly compiler would be able to set compile time feature flags for `core` through settings made in their `Cargo.toml`.

### Caveats

For users of a stable compiler, it would not be possible to modify the source code contents of `core`, or change any compile time features of `core` from the defaults used when publishing pre-compiled versions of `core`.

The source code used to build `core` would be the same as the compiler used for building the current project.

### User Interaction

When compiling for a non-standard target, users may specify their target using a json file, rather than a pre-defined target.

For example, currently a user may cross-compile by specifying a target known by Rust:

```sh
cargo build --target thumbv7em-none-eabihf
```

Users would also be able to specify a json file, by providing a path to the json file to be used.

```sh
cargo build --target thumbv7em-freertos-eabihf.json
```

By using a json target file, Cargo will rebuild `core` for use in the current project. When rebuilding `core`, Cargo will respect the profile settings used in the current project, including settings such as `opt-level`.

Users of a stable compiler would not be able to customize `core` outside of these profile settings.

For users of a nightly compiler, compile time features of `core` may be specified using the same syntax used for other crate dependencies. These specified features may include unstable features.

```toml
[dependencies.core]
default-features = false
features = [...]
```

It is not necessary to explicitly mention the dependency of `core`, unless changes to features are necessary.

### Technical Implications

#### Stabilization of JSON target format

As the custom target json files would become part of the stable interface of Cargo. The format used by this JSON file must become stabilized, and further changes must be made in a backwards compatible way to guarantee stability.

#### Building of `compiler-builtins`

Currently, `compiler-builtins` contains components implemented in the C programming language. While these dependencies have been highly optimized, the use of them would require the builder of the root crate to also have a sane compilation environment for compilation in C.

This RFC proposes instead to use the [pure rust implementation] when compiling for a custom target, removing the need for a C compiler.

While this may have code size or performance implications, this would allow for maximum portability.

[pure rust implementation]: https://github.com/rust-lang-nursery/compiler-builtins

#### `RUSTC_BOOTSTRAP`

It is necessary to use unstable features to build `core`. In order to allow users of a stable compiler to build `core`, we would set the `RUSTC_BOOTSTRAP` environment variable **ONLY** for the compilation of `core`.

This should be considered sound, as stable users may not change the source used to build `core`, or the features used to build `core`.

## 2 - Introduce the concept of "stable features" for `core`

### Use Case

In some cases, it may be desirable to modify `core` in set of predefined manners. For example, on some targets it may be necessary to have lighter weight machinery for `fmt`.

This step would provide a path for stabilization of compile time `core` features, which would be a subset of all total compile time features of `core`.

### Caveats

Initially, the list of stable compile time features for `core` would be empty, as none of the current features have had an explicit decision to be stable or not.

### User Interaction

Compile time features for `core` may be specified using the same Cargo.toml syntax used for other crates.

The syntax is the same when using `unstable` and `stable` features, however the former may only be used with a nightly compiler, and use of an `unstable` feature with a stable compiler would result in a compile time error.

The syntax for these features would look as follows:

```toml
[dependencies.core]
default-features = false
features = [...]
```

It is not necessary to explicitly mentioned the dependency of `core`, unless changes to features are necessary.

### Technical Implications

#### Path to stabilization

The stabilization of a `core` feature flag would require a process similar to the stabilization of a feature in the language:

* Any new feature begins as unstable, requiring a nightly compiler
* When the feature is sufficiently explored, an RFC/PR can be made to `libcore` to promote this feature to stable
* When this has been accepted, the feature of `core` may be used with the stable compiler.

#### Implementation of Stable Features

There would be some mechanism of differentiating between flags used to build core, sorting them into the groups `unstable` and `stable`. This RFC does not prescribe a certain way of implementation.

## 3 - Extend the new behaviors described for `std` (and `alloc`)

### Use Case

Once the design and implications of the changes have been made for `core`, it will be necessary to extend these abilities for `std`, including components like `liballoc`.

### Caveats

In general, the same restrictions for building `core` will apply to building `std`. These include:

* Users of the stable compiler must use the source used to build the current rust compiler
* Only compile time features considered `stable` may be used outside of nightly. Initially the list of `stable` features would be empty, and stabilizing these features would require a PR/RFC to `libstd`.

### User Interaction

The building of `std` would respect the current build profile, including

The syntax for these features would look as follows:

```toml
[dependencies.std]
default-features = false
features = [
    "force_alloc_system",
]
```

It is not necessary to explicitly mention the dependency of `std`, unless changes to features are necessary.

### Technical Implications

None beyond the technical implications listed for `core`.

## 4 - Allow the user to provide their own custom source versions of `core` and `std`

### Use Case

This will allow users of a nightly compiler to provide a custom version of `core` and `std`, without requiring the recompilation of the compiler itself.

### Caveats

As stability guarantees cannot be made around modified versions of `core` or `std`, a nightly compiler must always be used.

### User Interaction

For this interaction, the existing `patch` syntax of Cargo.toml will be used. For example:

```toml
[patch.sysroot]
core = { path = 'my/local/core' }
std  = { git = 'https://github.com/example/std' }
```

### Technical Implications

The `patch.sysroot` term will be introduced for patch when referring to components such as `std` and `core`.

# Drawbacks
[drawbacks]: #drawbacks

Why should we *not* do this?

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- Why is this design the best in the space of possible designs?
- What other designs have been considered and what is the rationale for not choosing them?
- What is the impact of not doing this?

# Prior art
[prior-art]: #prior-art

* https://github.com/rust-lang/rfcs/pull/1133
* https://github.com/japaric/xargo

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- What parts of the design do you expect to resolve through the RFC process before this gets merged?
- What parts of the design do you expect to resolve through the implementation of this feature before stabilization?
- What related issues do you consider out of scope for this RFC that could be addressed in the future independently of the solution that comes out of this RFC?

# Future possibilities
[future-possibilities]: #future-possibilities

Think about what the natural extension and evolution of your proposal would
be and how it would affect the language and project as a whole in a holistic
way. Try to use this section as a tool to more fully consider all possible
interactions with the project and language in your proposal.
Also consider how the this all fits into the roadmap for the project
and of the relevant sub-team.

This is also a good place to "dump ideas", if they are out of scope for the
RFC you are writing but otherwise related.

If you have tried and cannot think of any future possibilities,
you may simply state that you cannot think of anything.

Note that having something written down in the future-possibilities section
is not a reason to accept the current or a future RFC; such notes should be
in the section on motivation or rationale in this or subsequent RFCs.
The section merely provides additional information.
