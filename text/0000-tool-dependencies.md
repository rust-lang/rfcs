- Feature Name: tool-dependencies
- Start Date: 2016-08-08
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Allow crates to declare versioned dependencies on tools they use, such as cargo
and rustc.  Declare these as dependencies on special crate names such as
`tool:cargo` and `tool:rustc`, which can appear anywhere a dependency can
currently appear.

# Motivation
[motivation]: #motivation

Build tools, such as cargo, develop new features over time.  For instance,
Cargo 0.10 introduced new environment variables `$CARGO_PKG_NAME`,
`$CARGO_PKG_HOMEPAGE`, and `$CARGO_PKG_DESCRIPTION`.  A crate that expects
valid values for these environment variables will fail to build on an older
version of Cargo.  However, such a crate cannot currently declare a dependency
on the version of Cargo it expects.

In addition, some crates want to remain compatible with older versions of Rust,
avoiding newer language and library features to preserve that compatibility.
Declaring minimum versions will make it easier to detect intended
compatibility, and preserve that compatibility, such as by detecting when a
crate's dependencies have a newer version requirement than the crate itself, or
by using the metadata to run test builds with the corresponding versions.

This would also avoid conflating the semantic version of a crate with the
version of rust that it depends on.  Some crates incorporate changes in minimum
Rust versions into their semver as either minor-version or major-version bumps;
tool dependencies would allow expressing such changes via explicitly declared
dependency versions instead.

To avoid conflicts with any existing crate, this RFC introduces a namespace for
"tool" crate names, `tool:`, and defines two such build tools, `tool:cargo` and
`tool:rustc`.

# Detailed design
[design]: #detailed-design

In Cargo.toml files, anywhere the name of another crate can appear,
allow a name prefixed by `tool:`, such as `"tool:cargo"`.  For example:

```toml
[dependencies]
"tool:cargo" = "^1.5"

[dev-dependencies]
"tool:cargo" = "^1.8"

[target.'cfg(unix)'.dependencies]
"tool:rustc" = "^1.13"

[features]
nightly = ["tool:rustc/unstable"]
```

Cargo should recognize specific tool names internally, and behave as though the
corresponding `tool:` crate existed with an associated version.  In particular,
Cargo should behave as though `tool:cargo` exists with a version number equal
to that of Cargo itself, and behave as though `tool:rustc` exists with a
version number equal to rustc.

For `tool:rustc`, Cargo should additionally provide a feature `unstable`, if
building with a compiler that allows unstable features; this allows crates that
depend on nightly Rust to declare such a requirement, and allows environments
that require stable Rust (such as Linux distributions) to recognize and avoid
such dependencies.  Future work may introduce additional feature names for
`tool:` crates, such as specific Rust feature-gate names.

A `tool:` name does not refer to a crate; Cargo must recognize tool names
intrinsically.  If Cargo sees a tool dependency that it does not recognize, it
should treat that dependency as unsatisfiable, as it would a crate dependency
for a non-existent crate.  (This may not lead to a failed build, such as if a
crate declares the dependency as optional, or if a crate declares it in a
`[dev-dependencies]` section and the current build does not require satisfying
dev-dependencies.)

Typically, Cargo will provide only one version of a build tool at a time.  A
future definition of a new build tool could specify different behavior.  Other
tools may provide mechanisms to take advantage of having multiple versions
available.

Cargo could also choose to look for an older version of a crate with compatible
tool dependencies, rather than failing a build if the latest version of that
crate requires a newer version of a build tool.

Future definitions of build tools may support defining other feature names for
build tools.

Note that versions of Cargo without support for this RFC will still parse such
a Cargo.toml file, and process it correctly, only failing if the dependencies
actually require such a crate.  In particular, if a crate declares an optional
build tool dependency in its Cargo.toml, builds that do not require satisfying
that dependency will still complete successfully with older versions of Cargo.

In addition to Cargo's own handling of build tool dependencies, other tools
working with crates, such as package management systems, can translate these
declarative dependencies into build time dependencies on the corresponding
packages of build tools.

# Drawbacks
[drawbacks]: #drawbacks

Tools that parse Cargo.toml files without using cargo itself could potentially
expect to resolve build tools such as `tool:cargo` as crates, and fail.  Such
tools would require updates.  (Tools that use the cargo API should continue to
work.)

# Alternatives
[alternatives]: #alternatives

Today, packages could detect the version of Cargo or other build tools in
`build.rs`, and fail the build if they don't have a new enough version.  This
RFC provides a purely declarative dependency instead.  Cargo can parse such a
declarative dependency in advance to determine if the build will succeed.
Other tools, such as package management systems, can translate such a
declarative dependency into a build dependency on their package of the
corresponding build tool.

Rather than introducing a new crate namespace, Cargo and crates.io could
instead reserve special crate names, such as `cargo-bin` or `tool-cargo`.
However, this would potentially conflict with an existing crate name, including
a local crate.  In addition, this would complicate backward compatibility;
Cargo could not recognize any unknown build tool dependency.

Rather than using a prefix of `tool:`, Cargo could use a syntax that works as a
TOML "bare key", such as `-tool-cargo` or just `-cargo`.  This would avoid the
need to quote key names.  Using a syntax that crates.io prohibits in crate
names would prevent conflicts between crates and build tools.  However, cargo
does not actually prohibit such names in crates; only crates.io does.  So these
names could still potentially conflict with a local crate.

Cargo could introduce a separate key type or section for dependencies on a
build tool, such as `[tool-dependencies]`.  However, this would require
analogous extensions to every type of dependency Cargo supports, such as
dev-dependencies, dependencies for a specific crate feature flag, dependencies
for a specific platform, and so on.  In addition, existing versions of Cargo
would ignore the new section, rather than treating it as an unknown dependency
that it doesn't know how to satisfy.  Adding the `tool:` namespace allows a
build tool dependency to appear anywhere a crate dependency can currently
appear, and improves backward compatibility with existing versions of Cargo.
