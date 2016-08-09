- Feature Name: tool-dependencies
- Start Date: 2016-08-08
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Allow crates to declare versioned dependencies on tools they use, such as cargo
and rustc.  Declare these as dependencies on special crate names such as
`tool:cargo`, which can appear anywhere a dependency can currently appear.

# Motivation
[motivation]: #motivation

Build tools, such as cargo, develop new features over time.  For instance,
Cargo 0.10 introduced new environment variables `$CARGO_PKG_NAME`,
`$CARGO_PKG_HOMEPAGE`, and `$CARGO_PKG_DESCRIPTION`.  A crate that expects
valid values for these environment variables will fail to build on an older
version of Cargo.  However, such a crate cannot currently declare a dependency
on the version of Cargo it expects.

To avoid conflicts with any existing crate, this RFC introduces a namespace for
"tool" crate names, `tool:`, and defines one such build tool, `tool:cargo`.

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
"tool:cargo" = "^1.42"
```

Cargo should recognize specific tool names internally, and behave as though the
corresponding `tool:` crate existed with an associated version.  In particular,
Cargo should recognize `tool:cargo`, and behave as though that crate existed
with a version number corresponding to the version of Cargo itself.

A `tool:` name does not refer to a crate; Cargo must recognize tool names
intrinsically.  If Cargo sees a tool dependency that it does not recognize, it
should treat that dependency as unsatisfiable, as it would a crate dependency
for a non-existent crate.  (This may not lead to a failed build, such as if a
crate declares the dependency as optional, or if a crate declares it in a
`[dev-dependencies]` section and the current build does not require satisfying
dev-dependencies.)

Typically, Cargo will provide only one version of a build tool.  However, a
future definition of a new build tool could specify different behavior, such as
building a new version of a build tool before building a crate; such a
definition would need to consider the case of multiple crates specifying
incompatible version dependencies on build tools.  This RFC specifies only one
build tool, `tool:cargo`, and Cargo should only provide one version of that
build tool, corresponding to its own version number

Cargo could also choose to look for an older version of a crate whose build
tool dependencies it can meet, rather than failing a build if the latest
version of that crate requires a newer version of a build tool.

Future definitions of build tools may support defining feature names for build
tools, analogous to features for crates.

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
expect to resolve build tools such as `tool:cargo` as crates, and fail.  (Tools
that use the cargo API should continue to work.)

# Alternatives
[alternatives]: #alternatives

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

# Unresolved questions
[unresolved]: #unresolved-questions

This RFC intentionally avoids introducing `tool:rustc`, and associated
versioning and feature names for rustc.  Any such versioning scheme or feature
names would likely need to take nightly versions into account, to allow crates
that require nightly rustc feature-gates to express corresponding dependencies.
