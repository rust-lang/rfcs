- Feature Name: (`bindeps`)
- Start Date: 2020-11-30
- RFC PR: [rust-lang/rfcs#3028](https://github.com/rust-lang/rfcs/pull/3028)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Allow Cargo packages to depend on `bin`, `cdylib`, and `staticlib` crates, and use the artifacts built by those crates.

# Motivation
[motivation]: #motivation

There are many different possible use cases.

- [Testing the behavior of a binary](https://github.com/rust-lang/cargo/issues/4316#issuecomment-361641639). Currently, this requires invoking `cargo build` recursively, or running `cargo build` before running `cargo test`. 
- [Running a binary that depends on another](https://github.com/rust-lang/rustc-perf/tree/master/collector#how-to-benchmark-a-change-on-your-own-machine). Currently, this requires running `cargo build`, making it difficult to keep track of when the binary was rebuilt. The use case for `rustc-perf` is to have a main binary that acts as an 'executor', which executes `rustc` many times, and a smaller 'shim' which wraps `rustc` with additional environment variables and arguments.
- [Building tools needed at build time](https://github.com/rust-lang/rust/pull/79540#unresolved-questions). Currently, this requires either splitting the tool into a library crate (if written in Rust), or telling the user to install the tool on the host and detecting the availability of it. This feature would allow building the necessary tool from source and then invoking it from a `build.rs` script later in the build.
- Building and embedding binaries for another target, such as firmware or WebAssembly. This feature would allow a versioned dependency on an appropriate crate providing the firmware or WebAssembly binary, and then embedding the binary (or a compressed or otherwise transformed version of it) into the final crate. For instance, a virtual machine could build its system firmware, or a WebAssembly runtime could build helper libraries.
- Building and embedding a shared library for use at runtime. For instance, a binary could depend on a shared library used with [`LD_PRELOAD`](https://man7.org/linux/man-pages/man8/ld.so.8.html#ENVIRONMENT), or used in the style of the Linux kernel's [VDSO](https://man7.org/linux/man-pages/man7/vdso.7.html).

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Cargo allows you to depend on binary or C ABI artifacts of another package; this is known as a "binary dependency" or "artifact dependency". For example, you can depend on the `cmake` binary in your `build.rs` like so:

```toml
[build-dependencies]
cmake = { version = "1.0", type = "bin" }
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

You can optionally specify specific binaries to depend on using `bins`:

```toml
[build-dependencies]
cmake = { version = "1.0", type = "bin", bins = ["cmake"] }
```

If no binaries are specified, all the binaries in the package will be built and made available.

You can obtain the directory containing all binaries built by the `cmake` crate with `CARGO_BIN_DIR_CMAKE`, such as to add it to `$PATH` before invoking another build system or a script.

Cargo also allows depending on `cdylib` or `staticlib` artifacts. For example, you can embed a dynamic library in your binary:

```rust
// main.rs
const MY_PRELOAD_LIB: &[u8] = include_bytes!(env!("CARGO_CDYLIB_FILE_MYPRELOAD"));
```

Note that cargo cannot help you ensure these artifacts are available at runtime for an installed version of a binary; cargo can only supply these artifacts at build time. Runtime requirements for installed crates are out of scope for this change.

If you need to depend on multiple variants of a crate, such as both the binary and library of a crate, you can supply an array of strings for `type`: `type = ["bin", "lib"]`.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

There are four `type`s available:
1. `"lib"`, the default
2. `"bin"`, a crate building one or more binaries
3. `"cdylib"`, a C-compatible dynamic library
4. `"staticlib"`, a C-compatible static library

`"lib"` corresponds to all crates that can be depended on currently,
including `lib`, `rlib`, and `proc-macro` libraries.
See [linkage](https://doc.rust-lang.org/reference/linkage.html) for more information.

Artifact dependencies can appear in any of the three sections of dependencies (or in target-specific versions of these sections):
- `[build-dependencies]`
- `[dependencies]`
- `[dev-dependencies]`

By default, `build-dependencies` are built for the host, while  `dependencies` and `dev-dependencies` are built for the target. You can specify the `target` attribute to build for a specific target, such as `target = "wasm32-wasi"`; a literal `target = "target"` will build for the target even if specifing a build dependency. (If the target is not available, this will result in an error at build time, just as if building the specified crate with a `--target` option for an unavailable target.)

Cargo provides the following environment variables to the crate being built:

- `CARGO_<TYPE>_DIR_<CRATE>`, where `<TYPE>` is the `type` of the artifact (uppercased) and `<CRATE>` is the package of the crate being depended on. (As with other Cargo environment variables, crate names are converted to uppercase, with dashes replaced by underscores.) This is the directory containing all the artifacts from the crate.
- `CARGO_<TYPE>_FILE_<CRATE>_<ARTIFACT>`, where `<TYPE>` is the `type` of the artifact, `<CRATE>` is the package of the crate being depended on, and `<ARTIFACT>` is the name of the artifact. (Note that `<ARTIFACT>` is *not* modified in any way from the `name` specified in the crate supplying the artifact, or the crate name if not specified; for instance, it may be in lowercase, or contain dashes.) This is the full path to the artifact.
    - For the crate types `cdylib` and `staticlib` that can (currently) only build one artifact, cargo additionally supplies this variable with the `_<ARTIFACT>` suffix omitted.

For each kind of dependency, these variables are supplied to the same part of the build process that has access to that kind of dependency:
- For `build-dependencies`, these variables are supplied to the `build.rs` script, and can be accessed using `std::env::var_os`. (As with any OS file path, these may or may not be valid UTF-8.)
- For `dependencies`, these variables are supplied during the compilation of the crate, and can be accessed using `env!`.
- For `dev-dependencies`, these variables are supplied during the compilation of examples, tests, and benchmarks, and can be accessed using `env!`.

(See the "Future possibilities" section for a note about the use of `env!`.)

Similar to features, if other crates in your dependencies also depend on the same binary crate, and request different binaries, Cargo will build the union of all binaries requested.

Cargo will unify features and versions across dependencies of all types, just as it does for multiple dependencies on the same crate throughout a dependency tree.

`type` may be a string, or a list of strings; in the latter case, this specifies a dependency on the crate with each of those types, and is equivalent to specifying multiple dependencies with different `type` values. For instance, you may specify a build dependency on both the binary and library of the same crate. You may also specify separate dependencies of different `type`s; for instance, you may have a build dependency on the binary of a crate and a dependency on the library of the same crate.

Cargo does not take the specified `type` values into account when resolving a crate's version; it will resolve the version as normal, and then produce an error if that version does not support all the specified `type` values. Similarly, Cargo will produce an error if that version does not build all the binaries required by the `bins` value. Removing a crate type or binary is a semver-incompatible change. (Any further semver requirements on the interface provided by a binary or library depend on the nature of the binary or library in question.)

Until this feature is stabilized, it will require specifying the nightly-only option `-Z bindeps` to `cargo`. If `cargo` encounters a binary dependency or artifact dependency and does not have this option specified, it will emit an error and immediately stop building.

# Drawbacks
[drawbacks]: #drawbacks

Some of the motivating use cases have alternative solutions, such as extracting a library from a tool written in Rust, and making the tool a thin wrapper around the library. Making this change may potentially reduce the motivation to extract such libraries. However, many of the other use cases do not currently have any solutions, and extracted libraries have additional value even after this feature becomes available, so we don't see this as a reason to avoid introducing this feature.

Adding this feature will make Cargo usable for many more use cases, which may motivate people to use Cargo in more places and stretch it even further; this may, in turn, generate more support and more feature requests.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This RFC proposes supplying both the root directory and the path to each specific artifact. The path to specific artifacts is useful for accessing that specific artifact, and avoids needing target-specific knowledge about the names of executables (`.exe`) or libraries (`lib*.so`, `*.dll`, ...). The root directory is useful for `$PATH`, `$LD_LIBRARY_PATH`, and similar. Going from one to the other requires making assumptions. We believe there's value in supplying both.

We could specify a `target = "host"` value to build for the host even for `[dependencies]` or `[dev-dependencies]` which would normally default to building for the target. If any use case arises for such a dependency, we can easily add that.

We could make information about artifact dependencies in `[dependencies]` available to the `build.rs` script, which would allow running arbitrary Rust code to work with such dependencies at build time (rather than being limited to `env!`, proc macros, and constant evaluation). However, we can achieve the same effect with an entry in `[build-dependencies]` that has `target = "target"`, and that model seems simpler to explain and to work with.

We could install all binaries into a common binary directory with a well-known path under `$OUT_DIR`, and expect crates to use that directory, rather than passing in paths via environment variables. `npm` takes an approach like this. However, this would not allow dependencies on multiple distinct binaries with the same name, either provided by different crates or provided by the same crate built for different targets. Hardcoded paths would also reduce the flexibility of Cargo to change these paths in the future, such as to accommodate new features or extensions.

# Prior art
[prior-art]: #prior-art

- Cargo already provides something similar to this for C library dependencies of -sys crates. A `-sys` crate can supply arbitrary artifact paths, for libraries, headers, and similar. Crates depending on the `-sys` crate can obtain those paths via environment variables supplied via Cargo, such as to compile other libraries using the same C library. This proposal provides a similar feature for other types of crates and libraries.
- `make`, `cmake`, and many other build systems allow setting arbitrary goals as the dependencies of others. This allows building a binary and then running that binary in a rule that depends on that binary.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

How easily can Cargo handle a dependency with a different target specified? How will that interact with dependency resolution? Cargo already has to handle dependencies for both host and target (for cross-compilation), so those cases should already work.

# Future possibilities
[future-possibilities]: #future-possibilities

Currently, there's no mechanism to obtain an environment variable's value at compile time if that value is not valid UTF-8. In the future, we may want an `env_os!` macro, analogous to `std::env::var_os`, which returns a `&'static OsStr` rather than a `&'static str`. This is already an issue for existing environment variables supplied to the build that contain file paths.

In some cases, a crate may want to depend on a binary without unifying features or dependency versions with that binary. A future extension to this mechanism could allow cargo to build a binary crate in isolation, without attempting to do any unification.

Just as a `-sys` crate can supply additional artifacts other than the built binary, this mechanism could potentially expand in the future to allow building artifacts other than the built binary, such as C-compatible include files, various types of interface definition or protocol definition files, or arbitrary data files.
