- Feature Name: cargo_rust_version
- Start Date: 2016-08-08
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Allow crates to specify the version of Rust in which they are written.

# Motivation
[motivation]: #motivation

In https://github.com/rust-lang/rfcs/issues/1619 there is some contention over whether library should make a breaking release when they bump their language requirement.

If they must, then all their reverses dependencies need to push an update to allow the breaking change.
If types from the current crate are used in the reverse dependencies, then the need to upgrade and release cascades further downstream.
Clearly, this option doesn't scale.

Yet if a breaking change isn't made, users of older versions of Rust are adversely affected.
Existing packages will continue to work based on their lockfile, but new package can no longer be created very easily.
Absent other constraints, Cargo will pick the highest version to resolve a dependency.
This means if the only different between two minor versions is a bump of the language, Cargo will always the higher version written in the later version of Rust.
This will cause building with the older compiler to fail, even if using the older crate would result in a succeeding build plan.

With explicit langauge versions, we get the best of both worlds.
Packages don't have to introduce a breaking change for any dependency bump, be it language or library, so nothing cascades.
On the other hand, Cargo can rule out packages too new for the current compiler so as to not be "trapped" trying unsuitable build plans.

# Detailed design
[design]: #detailed-design

A new field is added to Cargo.toml:
```toml
rust-version = "$version"
```
Formally, we are answering the questions "which languages include the given program", so a version requirement, not version makes sense here.
In practice, we rely on Rust obeying semver so deeply that it might make sense to disallow anything but "major.minor".
If the field is absent, we assume the package is written in Rust 1.0.

Compilers besides rustc may have version numbers distinct from the version of Rust they implement.
For this purpose, the verbose version output (`$COMPILER -vV`) should contain an additional line:
```
rust-version: $version
```
For now, this is a version, not a version requirement.
[Were it a verion requirement, it would be contravariant (e.g. all minor versions *up to* the given on are recognized), but the idea of a contravariant requirement is not well known or given a standardized syntax.]
If the field is absent, a language version is deduced from the compiler version.

Cargo, when constructing the build plan, ensures that all crates are accepted by the language implemented by the compiler.

# Drawbacks
[drawbacks]: #drawbacks

If we generalize for nighties, semver is insufficient to account for unstable language features while not introducing too many breaking changes.

# Alternatives
[alternatives]: #alternatives

 - Have versioned, explicit standard library deps, and effectively deduce language version from them.

# Unresolved questions
[unresolved]: #unresolved-questions

None?
