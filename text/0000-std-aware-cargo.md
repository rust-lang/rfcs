- Feature Name: cargo_the_std_awakens
- Start Date: 2018-02-09
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

As of Rust 1.33.0, the `core` and `std` components of Rust are handled in a different way than Cargo handles other crate dependencies. This causes issues for non-mainstream targets, such as WASM, Embedded, and new not-yet-tier-1 targets. The following RFC proposes a roadmap to address these concerns in a consistent and incremental process.

# Motivation
[motivation]: #motivation

In today's Rust environment, `core` and `std` are shipped as precompiled objects. This was done for a number of reasons, including faster compile times, and a more consistent experience for users of these dependencies. This design has served the bulk of users fairly well. However there are a number of less common uses of Rust, that are not well served by this approach. Examples include:

* Supporting new/arbitrary targets, such as those defined by a custom target (".json") file
* Modifying `core` or `std` through use of feature flags
* Users who would like to make different optimizations to `core` or `std`, such as `opt-level = 's'`, with `panic = "abort"`

Previously, these needs were somewhat addressed by the external tool [xargo], which managed the recompilation of these dependencies when necessary. However, this tool has become [deprecated], and even when supported, required a nightly version of the compiler for all operation.

This approach has [gathered support] from various [rust team members]. This RFC aims to take inspiration from tools and workflows used by tools like [xargo], integrating them into Cargo itself.

[xargo]: https://github.com/japaric/xargo
[deprecated]: https://github.com/japaric/xargo/issues/193
[gathered support]: https://github.com/japaric/xargo/issues/193#issuecomment-359180429
[rust team members]: https://www.ncameron.org/blog/cargos-next-few-years/

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This proposal aims to make `core` and `std` feel a little bit less like a special case compared to other dependencies to the end users of Cargo. This proposal aims to minimize the number of new concepts introduced to achieve this, by interacting, configuring, modifying, and patching `core` and `std` in a similar manner to other dependent crates.

This RFC proposes the following concrete changes, which may or may not be implemented in this order, and may be done incrementally. The details and caveats around these stages are discussed in the [Reference Level Explanation][reference-level-explanation].

In this document, we use the term "root crate" to refer to the Rust project being built directly by Cargo. This crate contains the Cargo.toml used to guide the modifications described below. This would typically be a crate containing a binary application, or a standalone item, such as an `rlib`.

1. Allow developers of root crates to recompile `core` (and `compiler-builtins`) when their desired target does not match one available as a `rustup target add` target, without the usage of a nightly compiler. This version of `core` would be built from the same source files used to build the current version of `rustc`/`cargo`.
2. Allow the usage of Cargo features with the `core` library, additionally introducing the concept of "stable features" for `core`, which allow the end user to influence the behavior of their custom version of `core` without the use of a nightly compiler.
3. Extend the new behaviors described in step 1 and 2 for `std` (and `alloc`).
4. Allow the user to provide their own custom source versions of `core` and `std`, allowing for deep customizations when necessary. This will require a nightly version of the compiler.

As a new concept, the items above propose the existence of "stable features" for `core` and `std`. These features would be considered stable with the same degree of guarantees made for stability in the rest of the language. These features would allow configuration of certain functionalities of `core` or `std`, in a way decided at compile time.

For example, we could propose a feature called `force-tiny-fmt`, which would use different algorithms to implement `fmt` for use on resource constrained systems. The developer of the root crate would be able to choose the default behavior, or the `force-tiny-fmt` behavior while still retaining the ability of using a stable compiler.


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

A reference-level explanation is made for each of the items enumerated above.

## 1 - Allow developers of root crates to recompile `core`

### Use Case

For developers working with new targets not yet supported by the Rust project, this feature would allow the compilation of `core` for any target that can be specified as a valid [custom target specification].

[custom target specification]: https://rust-lang.github.io/rfcs/0131-target-specification.html

This functionality would be possible even with the use of a stable compiler.

Users of a nightly compiler would be able to set compile time feature flags for `core` through settings made in their `Cargo.toml`.

### Caveats

For users of a stable compiler, it would not be possible to modify the source code contents of `core`, or change any compile time features of `core` from the defaults used when publishing pre-compiled versions of `core`.

The source code used to build `core` would be the same as the compiler used for building the current project.

### User Interaction

When compiling for a non-standard target, users may specify their target using a target specification file, rather than a pre-defined target.

> NOTE: The current target specification is described in JSON, and contains some
> implementation details regarding the use of LLVM as the compiler backend. This
> RFC does not prescribe any changes to the Target Specification format, and is
> intended to work with whatever the current/stable method of specifying a
> custom target is.

For example, currently a user may cross-compile by specifying a target known by Rust:

```sh
cargo build --target thumbv7em-none-eabihf
```

Users would also be able to specify a target specification file, by providing a path to the file to be used.

```sh
cargo build --target thumbv7em-freertos-eabihf.json
```

In general, any of the following would prompt Cargo to recompile `core`, rather than use a pre-compiled version:

* A custom target specification is used
* The root crate has modified the feature flags of `core`
* The root crate has set certain profile settings, such as opt-level, etc.
* The root crate has specified a `patch.sysroot` (this is defined in a later section)

Users of a stable compiler would not be able to customize `core` outside of these profile settings.

For users of a nightly compiler, compile time features of `core` may be specified using the same syntax used for other crate dependencies. These specified features may include unstable features.

```toml
[dependencies.core]
default-features = false
features = [...]
```

It is not necessary to explicitly mention the dependency of `core`, unless changes to features are necessary.

Cargo would use the source of `core` located in the user's `SYSROOT` directory. This source code would be obtained in the same was as necessary today, through the use of `rustup component add rust-src`. If this component is missing, Cargo would exit with an error code, and would prompt the user to execute the command specified above.

### Technical Implications

#### Stabilization of a Target Specification Format

As the custom target specifications (currently JSON) would become part of the stable interface of Cargo. The format used by this file must become stabilized, and further changes must be made in a backwards compatible way to guarantee stability.

#### Building of `compiler-builtins`

Currently, `compiler-builtins` contains components implemented in the C programming language. While these dependencies have been highly optimized, the use of them would require the builder of the root crate to also have a working compilation environment for compilation in C.

This RFC proposes instead to use the [pure rust implementation] when compiling for a custom target, removing the need for a C compiler.

While this may have code size or performance implications, this would allow for maximum portability.

[pure rust implementation]: https://github.com/rust-lang-nursery/compiler-builtins

#### `RUSTC_BOOTSTRAP`

It is necessary to use unstable features to build `core`. To allow users of a stable compiler to build `core`, we would set the `RUSTC_BOOTSTRAP` environment variable **ONLY** for the compilation of `core`.

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

It is not necessary to explicitly mentione the dependency of `core`, unless changes to features are necessary.

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

The building of `std` would respect the current build profile, including optimization settings.

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

> NOTE: The use of `sysroot` as a category may be changed to a less loaded
> category name. This is likely an area for bikeshedding. `sysroot` will be
> used for the remainder of the document for consistency.

### Technical Implications

The `patch.sysroot` term will be introduced for patch when referring to components such as `std` and `core`.

# Drawbacks
[drawbacks]: #drawbacks

This RFC introduces new concepts to the use of Rust and Cargo, and could be confusing for current users of Rust who have not had to consider changes to `core` or `std` previously. However, in the normal case, most users are unlikely to need these settings, while they allow users that DO need to make changes to control important steps of the build process.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

> Why is this design the best in the space of possible designs?

This approach borrows from existing behaviors used by Cargo to allow configuration of `core` and `std`, as if they were a regular crate dependency.

This approach also offers an approach that can be developed and applied incrementally, allowing for time to find coner cases not considered by this RFC

> What other designs have been considered and what is the rationale for not choosing them?

To the author of this RFC's knowledge, there are no other open designs, other than the use tools that wrap Cargo entirely, such as [xargo].

[xargo]: https://github.com/japaric/xargo

> What is the impact of not doing this?

By not doing this, Rust will continue to be difficult to use for users and platforms "on the edge", such as new platform developers or embedded and WASM users.

# Prior art
[prior-art]: #prior-art

* [RFC1133] - This RFC from 2015 proposed making cargo aware of std. I still need to review in more detail to find the parts and syntax that may solve some open questions.
* [xargo] - This external tool was used to achieve a similar workflow as described above, limited to use with a nightly compiler
* [Cargo Issue 5002] - This issue proposed a syntax for explicit dependency on std
* [Cargo Issue 5003] - This issue discussed how to be backwards compatible with crates that don't explicitly mention std

[RFC1133]: https://github.com/rust-lang/rfcs/pull/1133
[Cargo Issue 5002]: https://github.com/rust-lang/cargo/issues/5002
[Cargo Issue 5003]: https://github.com/rust-lang/cargo/issues/5003

# Unresolved questions
[unresolved-questions]: #unresolved-questions

## How are dependencies (or non-dependency) on `core` and `std` specified?

For example in a `no_core` or `no_std` crate, how would we tell Cargo **not** to build the `core` and/or `std` dependencies?

## Should `std` be rebuilt if `core` is rebuilt?

Is it necessary to rebuild `std` using the customized `core`, even if no changes to `std` are necessary?

## Should Cargo obtain or verify the source code for `libcore` or `libstd`?

Right now we depend on `rustup` to obtain the correct source code for these libraries, and we rely on the user not to tamper with the contents. Are these reasonable decisions?

## Should the custom built `libcore` and `libstd` reside locally or globally?

e.g., should the build artifacts be placed in `target/`, only usable by this project, or in `.cargo/`, to be possibly reused by multiple projects, if they happen to have the same settings?

## How do we handle `libcore` and `libstd`'s `Cargo.lock` file?

Right now these are built using the global lock file in `rust-lang/rust`. Should this always be true? How should Cargo handle this gracefully?

## Should profile changes always prompt a rebuild of `core`/`std`?

For example, if a user sets their debug build to use `opt-level = 'z'`, should this rebuild `core`/`std` to use that opt level? Or should an additional flag, such as `apply-to-sysroot` be required to opt-in to this behavior, unless otherwise needed?

This could increase compile times for users that have set profile overrides, but have not previously needed a custom `core` or `std`.

Another option in this area is to force the use of profile overrides, as specified by [RFC2822](https://github.com/rust-lang/rfcs/blob/master/text/2282-profile-dependencies.md).

## Should providing a custom `core` or `std` require a nightly compiler?

It is currently unknown whether it is possible to provide a custom version of `core` or `std` without unstable features, as there are some compiler intrinsics and "magic" that are necessary (the format macros and box keyword come to mind).

I initially wrote the RFC in this manner, however I was later convinced this was not possible to do.

I am of the opinion that if you could, then it should be allowed to use a stable compiler, but that might be too theoretical for this RFC.

We could also move forward with the current restriction to nightly, and allow that to be lifted later by a follow-on RFC if this is possible and necessary.

## Should we allow configurable `core` and `std`

If we are to uphold stability guarantees for all configurations of `core` and `std`, this could require testing 2^(n+m) versions of Rust, where `n` is the number of `core` features, and `m` is the number of `std` features. This would have a negative impact on CI times.

# Future possibilities
[future-possibilities]: #future-possibilities

## Unified `core` and `std`

With the mechanisms specified above, it could be possible to remove the concept of `core` and `std` from the user, leaving only `std`.

By using stable feature flags for `std`, we could say that `std` as a crate with `default-features = false` would essentially be `no_core`, or with `features = ["core"]`, we would be the same as `no_std`.

This abstraction may not map to the actual implementation of `libcore` or `libstd`, but instead be an abstraction layer for the end developer.

## Stop shipping pre-compiled `core` and `std`

With the ability to build these crates on demand, we may want to decide not to ship `target` bundles for any users.

This would come at a cost of increased compile times, at least for the first build, if the artifacts are cached globally. However it would remove a mental snag of having to sometimes run `rustup target add`, and confusion from some users why parts of `std` and `core` have different optimization settings (particularly for debug builds) when debugging.
