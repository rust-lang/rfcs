- Feature Name: (`bindeps`)
- Start Date: 2020-11-30
- RFC PR: [rust-lang/rfcs#3028](https://github.com/rust-lang/rfcs/pull/3028)
- Tracking Issue: [rust-lang/cargo#9096](https://github.com/rust-lang/cargo/issues/9096)

# Summary
[summary]: #summary

Allow Cargo packages to depend on `bin`, `cdylib`, and `staticlib` crates, and use the artifacts built by those crates.

# Motivation
[motivation]: #motivation

There are many different possible use cases.

- [Running a binary that depends on another](https://github.com/rust-lang/rustc-perf/tree/master/collector#how-to-benchmark-a-change-on-your-own-machine). Currently, this requires running `cargo build`, making it difficult to keep track of when the binary was rebuilt. The use case for `rustc-perf` is to have a main binary that acts as an 'executor', which executes `rustc` many times, and a smaller 'shim' which wraps `rustc` with additional environment variables and arguments. This RFC would allow splitting the shim into a separate crate, building that crate as an artifact dependency, and invoking it as part of the top-level crate.
- Building tools needed at build time. Currently, this requires either splitting the tool into a library crate (if written in Rust), or telling the user to install the tool on the host and detecting the availability of it. This feature would allow building the necessary tool from source and then invoking it from a `build.rs` script later in the build.
- Building tools needed for testing. A crate might build a binary or module designed to work in conjunction with some other tool. The test harness for the top-level crate could have an artifact dependency on the tool, and invoke that tool as part of the testsuite.
- Building and embedding binaries for another target, such as firmware, WebAssembly, or SPIR-V shaders. This feature would allow a versioned dependency on an appropriate crate providing the binary, and then embedding the binary (or a compressed or otherwise transformed version of it) into the final crate. For instance, a virtual machine could build its system firmware, or a WebAssembly runtime could build helper libraries.
- Building and embedding a shared library for use at runtime. For instance, a tool for profiling or debugging other programs could depend on a shared library that it loads into those programs using [`LD_PRELOAD`](https://man7.org/linux/man-pages/man8/ld.so.8.html#ENVIRONMENT). Or, an operating system kernel could build a userspace API library that it loads into userspace applications running on it, in the style of the Linux kernel's [VDSO](https://man7.org/linux/man-pages/man7/vdso.7.html).

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Cargo allows you to depend on binary or C ABI artifacts of another package; this is known as a "binary dependency" or "artifact dependency". For example, you can depend on the `cmake` binary in your `build.rs` like this:

```toml
[build-dependencies]
cmake = { version = "1.0", artifact = "bin" }
```

Cargo will build the `cmake` binary, then make it available to your `build.rs` through an environment variable:

```rust
// build.rs
use std::{env, process::Command};

fn main() {
    let cmake_path = env::var_os("CARGO_BIN_FILE_CMAKE_cmake").expect("cmake binary");
    let mut cmake = Command::new(cmake_path).arg("--version");
    assert!(cmake.status().expect("cmake --version failed").success());
}
```

If you need to depend on multiple types of artifacts from a crate, such as both a binary and a cdylib from of a crate, you can supply an array of strings for `artifact`: `artifact = ["bin", "cdylib"]`.

You can optionally depend on specific binary artifacts from a crate using `bin:name`:

```toml
[build-dependencies]
somedep = { version = "1.0", artifact = ["bin:somebinary", "bin:anotherbinary"] }
```

If no binaries are specified, all the binaries in the package will be built and made available.

You can obtain the directory containing all binaries built by the `cmake` crate with `CARGO_BIN_DIR_CMAKE`, such as to add it to `$PATH` before invoking another build system or a script.

Cargo also allows depending on `cdylib` or `staticlib` artifacts. For example, you can embed a dynamic library in your binary:

```toml
[dependencies]
mypreload = { version = "1.2.3", artifact = "cdylib" }
```

```rust
// main.rs
const MY_PRELOAD_LIB: &[u8] = include_bytes!(env!("CARGO_CDYLIB_FILE_MYPRELOAD"));
```

Note that cargo only supplies these dependencies when building your crate. If your program or library requires artifacts at runtime, you will still need to handle that yourself by some other means. Runtime requirements for installed crates are out of scope for this change.

By default, a dependency with `artifact` specified will serve only as an artifact dependency, and will not serve as a normal Rust dependency, even if the dependency normally supplies a Rust library. If you need to depend on artifacts from a crate, and also express a normal Rust dependency on the same crate, you can add `lib = true` to the dependency; for instance: `cratename = { version = "1.2.3", lib = true, artifact = "bin" }`. (This applies to Rust `lib`, `rlib`, or `proc-macro` crates, all of which use the same `lib = true` option.)

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

There are three valid values for `artifact` available:
1. `"bin"`, a compiled binary, corresponding to a `[[bin]]` section in the dependency's manifest.
2. `"cdylib"`, a C-compatible dynamic library, corresponding to a `[lib]` section with `crate-type = "cdylib"` in the dependency's manifest.
3. `"staticlib"`, a C-compatible static library, corresponding to a `[lib]` section with `crate-type = "staticlib"` in the dependency's manifest.

`"lib"` corresponds to all crates that can be depended on currently,
including `lib`, `rlib`, and `proc-macro` libraries.
See [linkage](https://doc.rust-lang.org/reference/linkage.html) for more information.

Artifact dependencies can appear in any of the three sections of dependencies (or in target-specific versions of these sections):
- `[build-dependencies]`
- `[dependencies]`
- `[dev-dependencies]`

By default, `build-dependencies` are built for the host, while  `dependencies` and `dev-dependencies` are built for the target. You can specify the `target` attribute to build for a specific target, such as `target = "wasm32-wasi"`; a literal `target = "target"` will build for the target even if specifying a build dependency. (If the target is not available, this will result in an error at build time, just as if building the specified crate with a `--target` option for an unavailable target.)

Cargo provides the following environment variables to the crate being built:

- `CARGO_<ARTIFACT-TYPE>_DIR_<DEP>`, where `<ARTIFACT-TYPE>` is the `artifact` specified for the dependency (uppercased) and `<DEP>` is the name of the dependency. (As with other Cargo environment variables, dependency names are converted to uppercase, with dashes replaced by underscores.) This is the directory containing all the artifacts from the dependency.
    - If your manifest [renames the dependency](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#renaming-dependencies-in-cargotoml), `<DEP>` corresponds to the name you specify, not the original package name.
- `CARGO_<ARTIFACT-TYPE>_FILE_<DEP>_<NAME>`, where `<ARTIFACT-TYPE>` is the `artifact` specified for the dependency (uppercased as above), `<DEP>` is the package of the crate being depended on (transformed as above), and `<NAME>` is the name of the artifact from the dependency. This is the full path to the artifact.
    - Note that `<NAME>` is *not* modified in any way from the `name` specified in the crate supplying the artifact, or the crate name if not specified; for instance, it may be in lowercase, or contain dashes.
    - For convenience, if the artifact name matches the original package name, cargo additionally supplies a copy of this variable with the `_<NAME>` suffix omitted. For instance, if the `cmake` crate supplies a binary named `cmake`, Cargo supplies both `CARGO_BIN_FILE_CMAKE` and `CARGO_BIN_FILE_CMAKE_cmake`.

For each kind of dependency, these variables are supplied to the same part of the build process that has access to that kind of dependency:
- For `build-dependencies`, these variables are supplied to the `build.rs` script, and can be accessed using `std::env::var_os`. (As with any OS file path, these may or may not be valid UTF-8.)
- For `dependencies`, these variables are supplied during the compilation of the crate, and can be accessed using `env!`.
- For `dev-dependencies`, these variables are supplied during the compilation of examples, tests, and benchmarks, and can be accessed using `env!`.

(See the "Future possibilities" section for a note about the use of `env!`.)

Similar to features, if other crates in your dependencies also depend on the same binary crate, and request different binaries, Cargo will build the union of all binaries requested.

Cargo will unify versions across all kinds of dependencies, including artifact dependencies, just as it does for multiple dependencies on the same crate throughout a dependency tree.

Cargo will not unify features across dependencies for different targets. One dependency tree may have both ordinary dependencies and artifact dependencies on the same crate, with different features for the ordinary dependency and for artifact dependencies for different targets.

`artifact` may be a string, or a list of strings; in the latter case, this specifies a dependency on the crate with each of those artifact types, and is equivalent to specifying multiple dependencies with different `artifact` values. For instance, you may specify a build dependency on both a binary and a cdylib from the same crate. You may also specify separate dependencies with different `artifact` values, as well as dependencies on the same crate without `artifact` specified; for instance, you may have a build dependency on the binary of a crate and a normal dependency on the Rust library of the same crate.

Cargo does not take the specified `artifact` values into account when resolving a crate's version; it will resolve the version as normal, and then produce an error if that version does not support all the specified `artifact` values. Similarly, Cargo will produce an error if that version does not build all the binary artifacts required by `"bin:name"` values. Removing a crate type or an artifact is a semver-incompatible change. (Any further semver requirements on the interface provided by a binary or library depend on the nature of the binary or library in question.)

As with other kinds of dependencies, you can specify profile settings used to build artifact dependencies using [overrides](https://doc.rust-lang.org/cargo/reference/profiles.html#overrides). If not overridden, artifact dependencies in `build-dependencies` compiled for the host will build using the [`build-override` settings](https://doc.rust-lang.org/cargo/reference/profiles.html#build-dependencies), and all other artifact dependencies will inherit the same profile settings being used to build the crate depending on them.

Until this feature is stabilized, it will require specifying the nightly-only option `-Z bindeps` to `cargo`. If `cargo` encounters an artifact dependency and does not have this option specified, it will emit an error and immediately stop building.

The placement of artifact directories is an implementation detail of Cargo, and subject to change. The proposed implementation will place the artifact directory for each crate in `target/<TARGET>/artifact/<CRATE_NAME>-<METADATA_HASH>/<ARTIFACT_TYPE>`, where `<TARGET>` is the target triple the artifact dependency is built for (which may be the target triple of the host), `<CRATE_NAME>` is the name of the crate, `<METADATA_HASH>` is the usual hash that Cargo appends to crate-related file and directory names to ensure that changing properties (such as features) that affect the build of the crate will build into different paths, and `<ARTIFACT_TYPE>` is the artifact type (`bin`, `cdylib`, or `staticlib`).

If Cargo needs to build a crate for multiple targets, and that crate has an artifact dependency with `target="target"`, Cargo will build the artifact dependency for each target and supply it to the corresponding build of the depending crate.

# Drawbacks
[drawbacks]: #drawbacks

Some of the motivating use cases have alternative solutions, such as extracting a library from a tool written in Rust, and making the tool a thin wrapper around the library. Making this change may potentially reduce the motivation to extract such libraries. However, many of the other use cases do not currently have any solutions available (other than using an alternative build system, per the alternatives section), and extracted libraries have additional value even after this feature becomes available, so we don't see this as a reason to avoid introducing this feature.

Adding this feature will make Cargo usable for many more use cases, which may motivate people to use Cargo in more places and stretch it even further; this may, in turn, generate more support and more feature requests.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This RFC teaches Cargo to understand artifact dependencies. As an alternative, people writing crates with artifact dependencies could invoke `cargo` from `build.rs`, or could wrap the entire build in a separate build system that invokes Cargo multiple times. This would have many drawbacks, including:
- Cargo could not do dependency resolution in a unified way across dependencies, and thus could not help ensure consistency of dependency versions. This would break several use cases, without substantial additional complexity (e.g. vendored crates, or replacement of more of Cargo).
- Crates that have artifact dependencies would be less usable as dependencies themselves. Crates using a different build system would not work as Cargo dependencies at all. Crates using recursive invocations of cargo would introduce fragility, quirks, and limitations.
- Encouraging people to use build systems other than Cargo will remove the opportunity for Cargo and its defaults to set norms across the ecosystem.
- Crates manually implementing this via other build systems or recursive cargo invocations would make crates less uniform, and reduce consistency for users of Rust crates.
- Multiple/recursive invocations of Cargo will introduce challenges for Linux distributions, enterprises, and others who need to carefully manage/package/vendor dependencies. Crate metadata would not reflect its full dependencies. Manual invocations of cargo may handle dependency versioning inconsistently or not at all. Invocations of cargo may or may not pass through necessary options that were supplied to the top-level cargo invocation. Users may not have as many abilities to limit network access.

This RFC proposes supplying both the root directory and the path to each specific artifact. The path to specific artifacts is useful for accessing that specific artifact, and avoids needing target-specific knowledge about the names of executables (`.exe`) or libraries (`lib*.so`, `*.dll`, ...). The root directory is useful for `$PATH`, `$LD_LIBRARY_PATH`, and similar. Going from one to the other requires making assumptions. We believe there's value in supplying both.

We could specify a `target = "host"` value to build for the host even for `[dependencies]` or `[dev-dependencies]` which would normally default to building for the target. If any use case arises for such a dependency, we can easily add that.

We could make information about artifact dependencies in `[dependencies]` available to the `build.rs` script, which would allow running arbitrary Rust code to work with such dependencies at build time (rather than being limited to `env!`, proc macros, and constant evaluation). However, we can achieve the same effect with an entry in `[build-dependencies]` that has `target = "target"`, and that model seems simpler to explain and to work with.

We could install all binaries into a common binary directory with a well-known path under `$OUT_DIR`, and expect crates to use that directory, rather than passing in paths via environment variables. `npm` takes an approach like this. However, this would not allow dependencies on multiple distinct binaries with the same name, either provided by different crates or provided by the same crate built for different targets. Hardcoded paths would also reduce the flexibility of Cargo to change these paths in the future, such as to accommodate new features or extensions.

This RFC does not preclude future support in Cargo for more "native" handling of cdylib/staticlib dependencies, if Cargo can provide a reasonable default; such a dependency could use a different syntax (e.g. `somedep = { version = "...", link = ["cdylib-name"] }`).

In place of `lib = true`, we could rename `artifact` and have a `"lib"` or similar value for that field. This would provide simpler syntax (with a single list of dependency types), but could potentially conflate different dependency types (since a `"lib"` dependency type would express a normal dependency on a Rust library, while `"bin"` would express an artifact dependency).

Instead of `artifact = ["bin:binary-name", "bin:another-binary"]` to specify dependencies on specific binaries, we could use a separate field `bins = ["binary-name", "another-binary"]`. This seems unnecessarily verbose, and separates the indication of an artifact dependency from the list of binaries.

As another alternative to specify dependencies on specific binaries, we could use table-based structures, such as: `artifact = [{bin = ["binary-name", "another-binary"]}, "cdylib"]`. This would avoid parsing values like `bin:binary-name`, but it seems excessively complex and excessively nested. Other variations on this theme seem similarly complex. The proposed syntax feels like the right balance.

# Prior art
[prior-art]: #prior-art

- Cargo already provides something similar to this for C library dependencies of -sys crates. A `-sys` crate can supply arbitrary artifact paths, for libraries, headers, and similar. Crates depending on the `-sys` crate can obtain those paths via environment variables supplied via Cargo, such as to compile other libraries using the same C library. This proposal provides a similar feature for other types of crates and libraries.
- The Swift package manager has a concept of ["products"](https://docs.swift.org/package-manager/PackageDescription/PackageDescription.html#product), which can be either libraries or executables. Expressing a dependency on a package allows you to make use of either the library or executable products of that package.
- `make`, `cmake`, and many other build systems allow setting arbitrary goals as the dependencies of others. This allows building a binary and then running that binary in a rule that depends on that binary.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

How easily can Cargo handle a dependency with a different target specified? How will that interact with dependency resolution? Cargo already has to handle dependencies for both host and target (for cross-compilation), so those cases should already work.

# Future possibilities
[future-possibilities]: #future-possibilities

Currently, there's no mechanism to obtain an environment variable's value at compile time if that value is not valid UTF-8. In the future, we may want macros like `env_os!` or `env_path!`, which return a `&'static OsStr` or `&'static Path` respectively, rather than a `&'static str`. This is already an issue for existing environment variables supplied to the build that contain file paths.

In some cases, a crate may want to depend on a binary without unifying dependency versions with that binary. A future extension to this mechanism could allow cargo to build a binary crate in isolation, without attempting to unify versions.

Just as a `-sys` crate can supply additional artifacts other than the built binary, this mechanism could potentially expand in the future to allow building artifacts other than the built binary, such as C-compatible include files, various types of interface definition or protocol definition files, or arbitrary data files.

If a dependency has a specific `target` (other than the host or target), and the target is not available, cargo can only emit an error at build time that tells the user to install the target. Some projects may wish to use `rustup`'s support for `rust-toolchain` TOML files to specify targets they or their dependencies require. However, in the future, Cargo could have more native support for targets, either by downloading precompiled targets as rustup does, or by building support for those targets using `build-std` or equivalent. Integrating such support into Cargo would improve support for cross-compiled artifact dependencies.
