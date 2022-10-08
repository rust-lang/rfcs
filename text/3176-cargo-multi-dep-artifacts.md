- Feature Name: (`multidep`)
- Start Date: 2021-09-14
- RFC PR: [rust-lang/rfcs#3176](https://github.com/rust-lang/rfcs/pull/3176)
- Tracking Issue: [rust-lang/cargo#10030](https://github.com/rust-lang/cargo/issues/10030)

# Summary
[summary]: #summary

Allow Cargo packages to depend on the same crate multiple times with different
dependency names, to support artifact dependencies for multiple targets.

# Motivation
[motivation]: #motivation

[RFC 3028](https://github.com/rust-lang/rfcs/blob/HEAD/text/3028-cargo-binary-dependencies.md)
specified "artifact dependencies", allowing crates to depend on a compiled
binary provided by another crate, for a specified target.

Some crates need to depend on binaries for multiple targets; for instance, a
virtual machine that supports running multiple targets may need firmware for
each target platform. Sometimes these binaries may come from different crates,
but sometimes these binaries may come from the same crate compiled for
different targets.

This RFC enables that use case, by allowing multiple dependencies on the same
crate with the same version, as long as they're each renamed to a different
name. This allows multiple artifact dependencies on the same crate for
different targets.

Note that this RFC still does not allow dependencies on different
semver-compatible versions of the same crate, only multiple dependencies on
exactly the same version of the same crate.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Normally, you may only have one dependency on a given crate with the same
version. You may depend on different incompatible versions of the same crate
(for instance, versions `0.1.7` and `1.2.4`), but if you specify two or more
dependencies on a crate with the same version, Cargo will treat this as an
error.

However, Cargo allows [renaming
dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#renaming-dependencies-in-cargotoml),
to refer to a crate by a different name than the one it was published under. If
you use this feature, you may have multiple dependencies on the same version of
the same crate, as long as the dependencies have different names.  For example:

```toml
[dependencies]
example1 = { package = "example", version = "1.2.3" }
example2 = { package = "example", version = "1.2.3" }
```

This can be useful if you need to refer to the same crate by two different
names in different portions of your code.

This feature provides particular value in specifying artifact dependencies for
different targets. You may specify multiple artifact dependencies on the same
crate for different targets, as long as those dependencies have different
names:

```toml
[dependencies]
example_arm = { package = "example", version = "1.2.3", artifact = "bin", target = "aarch64-unknown-none" }
example_riscv = { package = "example", version = "1.2.3", artifact = "bin", target = "riscv64imac-unknown-none-elf" }
example_x86 = { package = "example", version = "1.2.3", artifact = "bin", target = "x86_64-unknown-none" }
```

Cargo will make the binaries from each of these artifacts available under the
specified name. For instance, in this example, binaries from `example` built
for `riscv64imac_unknown_none_elf` will appear in the directory specified by
the environment variable `CARGO_BIN_DIR_EXAMPLE_RISCV`, while binaries from
`example` built for `aarch64-unknown-none` will appear in the directory
specified by `CARGO_BIN_DIR_EXAMPLE_ARM`.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Cargo allows specifying multiple dependencies on the same crate, as long as all
such dependencies resolve to the same version, and have different dependency
names specified. Cargo will make the dependency available under each specified
name.

Multiple artifact dependencies on the same crate may have different `target`
fields. In this case, cargo will build the dependency for each specified
`target`, and make each build available via the corresponding dependency name.

Cargo provides your crate with the standard set of environment variables for
each artifact dependency: `CARGO_<ARTIFACT-TYPE>_DIR_<DEP>` for the directory
containing the artifacts (e.g.  `CARGO_BIN_DIR_EXAMPLE`) and
`CARGO_<ARTIFACT-TYPE>_FILE_<DEP>_<NAME>` for each artifact by name (e.g.
`CARGO_BIN_FILE_EXAMPLE_mybin`). Note that the name you give to the dependency
determines the `<DEP>`, but does not affect the `<NAME>` of each artifact
within that dependency.

Cargo will unify versions across all kinds of dependencies, including multiple
artifact dependencies, just as it does for multiple dependencies on the same
crate throughout a dependency tree. A dependency tree may only include one
semver-compatible version of a given crate, but may include multiple
semver-incompatible versions of a given crate. Dependency versions need not be
textually identical, as long as they resolve to the same version.

Cargo will not unify features across dependencies for different targets. One
dependency tree may have both ordinary dependencies and multiple artifact
dependencies on the same crate, with different features for the ordinary
dependency and for artifact dependencies for different targets.

Building an artifact dependency for multiple targets may entail building
multiple copies of other dependencies, which must similarly unify within a
dependency tree.

Multiple dependencies on the same crate may specify different values for
`artifact` (e.g. to build a library and/or multiple specific binaries), as well
as different values for `target`. Cargo will combine all the entries for a
given `target`, and build all the specified artifacts for that target.
Requesting a specific artifact for one target will not affect the artifacts
built for another target.

[Profile
overrides](https://doc.rust-lang.org/cargo/reference/profiles.html#overrides)
are specified in terms of the original crate name, not the dependency name;
thus, Cargo does not currently support overriding profile settings differently
for different artifact dependencies.

Until this feature is stabilized, it will require specifying the nightly-only
option `-Z multidep` to `cargo`. If `cargo` encounters multiple dependencies on
the same crate and does not have this option specified, it will continue to
emit an error.

# Drawbacks
[drawbacks]: #drawbacks

This feature will require Cargo to handle multiple copies of the same crate
within the dependencies of a single crate. While Cargo already has support for
handling multiple copies of the same crate within a full dependency tree, Cargo
currently rejects multiple copies of the same crate within the dependencies of
a single crate, and changing that may require reworking assumptions made within
some portions of the Cargo codebase.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Cargo already allows a dependency tree to contain multiple dependencies on the
same crate (whether as an artifact dependency or otherwise), by introducing an
intermediate crate. This feature provides that capability within the
dependencies of a single crate, which should avoid the multiplicative
introduction (and potentially publication) of trivial intermediate crates for
each target.

This RFC handles building an artifact dependency for multiple targets by
requiring a different name for the dependency on each target. As an
alternative, we could instead allow specifying a list of targets in the
`target` field. This would provide a more brief syntax, but it would require
Cargo to incorporate the target name into the environment variables provided
for the artifact dependency. Doing so would complicate artifact dependencies
significantly, and would also complicate the internals of Cargo. Separating
these dependencies under different names makes them easier to manage and
reference, both within Cargo and within the code of the crate specifying the
dependencies.

While this RFC has artifact dependencies as a primary use case, it also allows
specifying multiple non-artifact dependencies on the same crate with different
names. This seems like a harmless extension, equivalent to `use name1 as
name2;` and similar. However, if it adds any substantive complexity, we could
easily restrict this feature exclusively to artifact dependencies, without any
harm to the primary use case.

# Future possibilities
[future-possibilities]: #future-possibilities

This RFC does not provide a means of specifying different profile overrides for
different dependencies on the same crate. A future extension to this mechanism
could take the dependency name or target into account and allow specifying
different profile overrides for each dependency.

When building an artifact dependency for a target, the depending crate may wish
to specify more details of how the crate gets built, including target-specific
options (e.g. target features or target-specific binary layout options). Cargo
currently exposes such options via `.cargo/config.toml`, but not via
`Cargo.toml`. If and when we provide a means to specify such options via
`Cargo.toml`, we need to allow specifying those options not just by dependency
name but also by target.
