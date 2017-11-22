- Feature Name: internal-path-dependencies
- Start Date: 2017-11-22
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

This RFC proposes the support for publishing a crate with path dependencies.

# Motivation
[motivation]: #motivation

Cargo currently does not allow uploading a crate with path dependencies to the
registry, which means developers cannot utilize path dependencies to better
organize the code.

Path dependencies can serve as sub-crates of the root crate. This structure can
help to avoid clustering the specs of all external dependencies in a single
top-level manifest file. This also ease the work if later we decide to publish
a standalone path dependency.

With this feature, we don't have to publish path dependencies before the root
crate is allowed to be published. So path dependencies can be kept internal to
the project, and all the source code can be bundled into a single crate for
downloading.

As a result of this, the workspace information will be kept along with the
published crate, thus large projects can benefit more from it.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The manifest value of `package.publish` is `true` by default, which means
a path dependency is allowed to be published. By setting this value to `false`,
we can on one hand forbid a standalone crate from being published to the
registry (by accident), and on the other hand allow its content to be included
when `cargo package` or `cargo publish` is run.

Say our project "foo" has these files:

```
foo/Cargo.toml
foo/src/main.rs
foo/bar/Cargo.toml
foo/bar/src/lib.rs
```

The root crate "foo" depends on the path dependency "bar" in the subdirectory
`bar/`:

```toml
# foo/Cargo.toml
[package]
name = "foo"
version = "0.1.0"
...
[dependencies.bar]
path = "bar"
version = "*"
```

And we set `package.publish` to `false` in bar's manifest:

```toml
[package]
publish = false
name = "bar"
version = "1.0.0"
...
```

Then we can run `cargo package --list` to see what is included for publishing:

```
Cargo.toml
bar/Cargo.toml
bar/src/lib.rs
src/main.rs
```

After `cargo publish --dry-run` we can find these files:

```
target/package/foo-0.1.0/Cargo.toml
target/package/foo-0.1.0/Cargo.toml.orig
target/package/foo-0.1.0/bar/Cargo.toml
target/package/foo-0.1.0/bar/Cargo.toml.orig
target/package/foo-0.1.0/bar/src/lib.rs
target/package/foo-0.1.0/src/main.rs
target/package/foo-0.1.0.crate
```

All manifest files are sanitized and reside with their original files `*.orig`.

If you specify the workspace in the manifest, they would be kept as is.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

N/A

# Drawbacks
[drawbacks]: #drawbacks

This feature does not affect any existing projects or other functionality of
cargo. Only the published contents of the package will be changed, i.e. adding
source files of path dependencies, recursively.

# Rationale and alternatives
[alternatives]: #alternatives

There is still no other proposals related to publishing with path dependencies.

Without this feature, a project with path dependencies cannot be uploaded,
unless the path dependencies are registed at the registry. If developers insist
publishing internal crates to the registry, then the namespace would get
polluted. In addition, these separated dependencies lose support of shared
workspace and thus make it harder to keep the same versions of dependencies.

# Unresolved questions
[unresolved]: #unresolved-questions

-   Use another manifest key to specify whether path dependencies are included?
    -   Some people think `package.publish=false` means the source code should
        not be published or "public" in any way.
    -   `package.internal=true`?
    -   `dependencies.foo.internal=true`?
-   No restriction for the versions of internal path dependencies?
-   How should the manifest files be further sanitized?
