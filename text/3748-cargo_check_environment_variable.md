- Feature Name: `cargo_check_environment_variable`
- Start Date: 2024-12-20
- RFC PR: [rust-lang/rfcs#3748](https://github.com/rust-lang/rfcs/pull/3748)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add a new environment variable `CARGO_CHECK` that is set to `1` when running `cargo check` or similar type-checking operations so build scripts can skip expensive compilation steps that are unnecessary for Rust type checking, such as compiling external C++ code in cxx based projects.

# Motivation
[motivation]: #motivation

Rust development heavily relies on IDE tooling like rust-analyzer, which frequently invokes `cargo check` to provide real-time type information and diagnostics. Many projects use build scripts (`build.rs`) to generate Rust code and compile external dependencies. For example:

- cxx-rs generates Rust bindings for C++ code and compiles C++ source files
- cxx-qt generates Rust bindings for Qt code and runs the Qt Meta-Object Compiler (MOC)
- Projects using Protocol Buffers generate Rust code from .proto files
- bindgen generates Rust bindings from C/C++ headers

Currently, every time rust-analyzer runs `cargo check`, the build script in the changed crate must execute its full build process, including steps like compiling C++ code that are only needed for linking but not for type checking. Normally the build script would only be run when a file added by `cargo::rerun-if-changed` is changed, which generally doesn't include the Rust source code. However, when using `cxx` to create bridges between C++ and Rust, the build script must be run for every change in the Rust bridges. Usually `cxx` bindings are rarely changed but in projects like `cxx-qt` that interface between Rust and Qt types, they receive signficantly more changes. This impacts IDE responsiveness, especially in projects with complex build scripts.

This is particularly important for projects using cxx-qt and similar frameworks where the build scripts perform extensive code generation and compilation.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When writing a build script (`build.rs`), you can now check the `CARGO_CHECK` environment variable to determine if the build is being performed for type checking purposes:

```rust
fn main() {
    generate_rust_bindings();

    // Only compile external code when not type checking
    if std::env::var("CARGO_CHECK").is_ok() {
        compile_cpp_code();
    }
}
```

This allows build scripts to optimize their behavior based on the build context. When rust-analyzer or a developer runs `cargo check`, the build script can skip time-consuming steps that aren't necessary for type checking.

This feature primarily benefits library authors who maintain build scripts, especially those working with external code generation and compilation. Regular Rust developers using these libraries will automatically benefit from improved IDE performance without needing to modify their code.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Cargo will set the `CARGO_CHECK` environment variable to `1` when running `cargo check`

The environment variable will not be set for commands that require full compilation:
- `cargo build`
- `cargo run`
- `cargo test`

# Drawbacks
[drawbacks]: #drawbacks

1. **Potential for Inconsistencies**: Build scripts might behave differently during type checking vs. full compilation, which could theoretically lead to different type checking results compared to the final build.

2. **Increased Complexity**: Build script authors need to consider an additional factor when determining their behavior, which adds some complexity to the build system. On the other hand, they can ignore the feature entirely and just run all build steps regardless.

3. **Maintenance Burden**: The Rust and Cargo teams will need to maintain this feature and ensure it remains consistent across different commands and contexts.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-alternatives

Alternative designs considered:

1. Define a standard environment variable that isn't set by `cargo check` but is officially encouraged by Rust for RLS and other IDE tooling. This would avoid any unexpected behavior from build scripts with other `cargo check` consumers but still provide a standard way for build scripts to skip unnecessary steps.

2. Do Nothing: If we do nothing, build scripts will continue to run all build steps even when it's not necessary, significantly impacting Rust ergonomics when interfacing with exernal languages.

# Prior art
[prior-art]: #prior-art

1. **Go Build Tags**: Go allows conditional compilation using build tags, which can be used to skip certain build steps based on the build context.

2. **Bazel's Configuration Transitions**: Bazel provides mechanisms to modify build behavior based on the target being built.

3. **Cargo Features**: The existing feature flag system in Cargo demonstrates the value of conditional build behavior.

4. **Other Cargo Environment Variables**: Cargo already sets several environment variables during builds:
   - `CARGO_CFG_TARGET_OS`
   - `CARGO_MANIFEST_DIR`
   - `OUT_DIR`

This proposal follows the established pattern of using environment variables to communicate build context to scripts.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

1. Should the environment variable be set for other commands that don't require full compilation?
   - `cargo doc`
   - `cargo clippy`

2. How should this interact with parallel builds where some targets need full compilation and others only need type checking? (Is this even a thing?)

3. Should we provide additional variables to distinguish between different types of type-checking operations (IDE, clippy, etc.)?

4. How do we ensure build scripts don't diverge too much between type checking and full compilation modes?

# Future possibilities
[future-possibilities]: #future-possibilities

1. **Extended Build Contexts**: Introduce additional environment variables for other build contexts:
   - `CARGO_DOC` for documentation generation
   - `CARGO_IDE` specifically for IDE tooling