- Feature Name: (`cargo-manual-targets`)
- Start Date: 2020-12-13
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Allow Cargo packages to build or otherwise satisfy bin, cdylib, or staticlib
targets via `build.rs`, and supply the resulting artifacts for artifact
dependencies.

# Motivation
[motivation]: #motivation

The `bindeps` RFC allows Cargo packages to declare "artifact dependencies" on
artifacts (binaries and libraries) from other Cargo packages.

This RFC extends that to allow Cargo packages to satisfy such binaries via
`build.rs`, rather than exclusively building such binaries via `[[bin]]`
targets. For instance, a `build.rs` script could build a binary from C sources,
or pass through a binary supplied by another artifact dependency (potentially
with modification or postprocessing), or use a system version of a binary.

This extension to artifact dependencies provides the flexibility to support
multiple alternative implementations of the same binary, or multiple means of
providing that binary. As an example, a crate might have feature flags that
determine whether it builds a specific binary from source, obtains it from a
dependency, or uses a system version.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When specifying a Cargo target with a `[[bin]]` or `[lib]` section, normally
Cargo will build the target. However, if you specify `manual = true` in the
target's configuration, Cargo will instead expect the `build.rs` script to
build that target.

For example, consider the following `Cargo.toml` snippet for a hypothetical
`cmake-bin` crate:

```toml
[[bin]]
name = "cmake"
manual = true
```

Cargo will supply the `build.rs` script with the path to a directory via the
environment variable `CARGO_ARTIFACT_DIR`, and when the `build.rs` script
completes, Cargo will expect to find an executable binary `cmake` (or
`cmake.exe` on Windows) in that directory. If that binary does not exist, the
build will fail.

The `build.rs` script could build `cmake` from source, or could find a system
copy of cmake and copy/link it into place; for instance, it might do the former
if supplied a feature flag `vendored`, or do the latter otherwise.

Crates that depend on the `cmake-bin` crate as an artifact dependency can then
reference the environment variable `CARGO_BIN_FILE_CMAKE_cmake` to find that
`cmake` binary, or `CARGO_BIN_DIR_CMAKE` to find the directory containing the
`cmake` binary.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Cargo will supply the following additional environment variable to `build.rs`
if any manual targets exist:

- `CARGO_ARTIFACT_DIR`: This contains the absolute path to the directory where
  Cargo expects the build script to provide any manual targets. Note that this
  may not necessarily be the final location; Cargo may choose to have build
  scripts target an intermediate directory and then copy the artifacts into
  place.

A target section with `manual = true` must only contain the following subset of
allowed fields (subject to the additional limitations of which fields are
allowed in which types of section):
- `name`: mandatory
- `crate-type`: mandatory for `[lib]`, may only contain `"cdylib"` or
  `"staticlib"` or both.
- `required-features`: optional

If the built artifacts are target-specific, they must be built for the target
specified via the `TARGET` environment variable to the `build.rs` script.

Artifacts supplied for a `[bin]` target must be executable on the target. They
may be binaries native to the target, scripts, or any other format runnable on
the target system.

Artifacts supplied for a `cdylib` or `staticlib` target must have a format and
name suitable for passing to the target linker as a shared or static
library (respectively), either via direct filename (e.g. `libname.so`,
`libname.a`) or via the linker's normal mechanism for specifying libraries
(e.g. `-lname`).

Artifacts must be runnable/linkable even if moved to a different directory.
(However, symlinks with relative paths are allowed, even though they must be
adjusted if moved.)

Until this feature is stabilized, it will require specifying the nightly-only
option `-Z manual-targets` to `cargo`. If `cargo` encounters a target with
`manual = true` and does not have this option specified, it will emit an error
and immediately stop building.

# Drawbacks
[drawbacks]: #drawbacks

Adding this feature will make Cargo usable for many more use cases, which may
motivate people to use Cargo in more places and stretch it even further; this
may, in turn, generate more support and more feature requests.

In particular, adding this feature will help people use Cargo to build various
tools or libraries, which may potentially lead to more vendoring. We encourage
packages to support vendoring, but also to support using system versions of
artifacts when appropriate.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

We could supply an additional environment variable for each manual target,
specifying the name of the artifact file. This would avoid needing
target-specific knowledge about the names of executables (`.exe`) or libraries
(`lib*.so`, `*.dll`, ...). However, build systems frequently already have this
information, and would derive little value from Cargo providing it; such an
environment variable would almost always be ignored.

We could allow the `build.rs` script to place the artifacts anywhere, and then
emit a `cargo:artifact=name=path` directive. Cargo could then copy or link the
artifact into place. However, this hardcodes assumptions about whether to use a
copy, symlink, hardlink, script wrapper, linker script, or other mechanism.
Those assumptions may vary depending on the nature of the artifact. We propose
having the build script handle that detail (and suggest using a hardlink or
symlink to avoid a copy if possible on the platform). We can always choose to
extend this mechanism further in the future.

Rather than an existing target type, we could define an entirely different
target type and section for manual artifacts. However, this would not allow
crates to integrate with existing types of artifact dependencies. Using
established target types has a norming and standardizing effect.

We could have different artifact directories for different types of artifacts,
such as binaries vs libraries.

# Prior art
[prior-art]: #prior-art

The [bindeps](https://github.com/rust-lang/rfcs/pull/3028/) RFC specifies a
mechanism for crates to supply artifact dependencies to each other; this RFC
serves as an extension to that.

# Future possibilities
[future-possibilities]: #future-possibilities

We could support `manual = true` for other types of targets in the future.
However, we should not allow `manual = true` for Rust library targets,
proc-macro targets, or similar; we should only allow `manual = true` for
targets where the expected format and ABI are standardized.

We could support targets that may or may not be manual depending on some
factor. We would then need a mechanism for the build script to either supply
the artifact or supply the necessary details for a Cargo target. For the time
being, we expect crates may use artifact dependencies, features, and separate
crates to handle this. We expect that crates may commonly still want such
separations for other reasons.
