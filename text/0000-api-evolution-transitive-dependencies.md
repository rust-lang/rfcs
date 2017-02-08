- Feature Name: not applicable
- Start Date: 2017-02-08
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

This RFC amends [RFC #1105][1105] to address how changing the versions of a
library's dependencies should affect stability. These guidelines are intended
for the crates.io ecosystem. They do not apply to the Rust standard library or
the language itself.

# Motivation
[motivation]: #motivation

[RFC #1105][1105] laid out the guidelines for libraries to follow for with
regards to semver and breaking changes. While it was quite thorough in types of
changes, it neglected to address when changing the version of a library's
dependencies is a breaking change.

Few crates are completely self contained. The majority of crates published to
crates.io contain at least one entry in their `[dependencies]` section, and
changing the version of those dependencies can potentially break downstream
code.

Similar to items exposed by a library, not all dependencies are publicly
visible. Unlike items, the language does not make it explicit when a dependency
is public or not. This RFC seeks to provide a comprehensive description of when
a dependency is part of your public API, and lay out concrete guidelines for
when changing the version of a dependency *must* be considered a breaking
change.

# Detailed design
[design]: #detailed-design

## Definition of Terms

The term "dependency" refers to any crate which appears in the `[dependencies]`
section of your `Cargo.toml`.

The term "transitive dependency" refers to the dependency of a dependency.

The term "downstream crate" is used to refer to a crate which depends on your
library, either directly or transitively.

The term "sibling crate" is a crate depended on by an downstream crate.

For the purposes of this RFC, we need to break dependencies into two groups:
public and private.

A dependency is defined as public when any item provided by that crate is used
publicly. A dependency is private when no items are used publicly.

Public usage means different things based on the type of item.

For structs, enums, and type aliases public usage includes:

- Using that type as the type of a public field of a public struct or enum
- Using that type in the signature of a public function or method
- Implementing a public trait for that type
- Constraining the impl of a public trait based on that type
- Referencing that type in a public type alias

For structs, enums, and type aliases public usage does not include:

- Using that type in a private field of a struct or enum
- Using that type in a position that accepts a generic argument where none of
  the bounds are provided by your library

For traits public usage includes:

- Implementing that trait for any public struct or enum
- Using that trait as a supertrait of any public trait
- Using that trait as a bound on a type parameter of any public item

For all items public usage does not include:

- Publicly re-exporting that item in a public module
  - Changes to versions are always minor changes with regards to re-exports.
    This is expanded on below.

## Policy by cargo feature

With these terms defined, we can now look at what is a major vs minor change.

### Major change: modifying the version range of a public dependency to anything that is not a superset of the previous version range

This applies even if the new version range is fully semver compatible with the
previous one. For example, changing from `=1.0.0` to `=1.0.5` is still
considered a major change.

Unlike some package managers, Cargo allows multiple versions of the same
transitive dependency to be used. However, items exported by that crate will be
considered different types if they come from different versions. Therefore,
if a version of a dependency is changed from `2.0.0` to `3.0.0`, any place where
items from that dependency appear publicly could be treated as changing from
taking `A` to taking `B`, which is a major change under [RFC #1105][1105].

It should be noted that this cannot be avoided by relying on a trait instead of
a concrete type. If we treat a struct from two different versions as concrete
types `A` and `B`, changing the version of a dependency can be seen as removing
an impl of that trait from `A`, and adding it to `B`. Removing an existing trait
is not explicitly mentioned as a major change by [RFC #1105][1105], but this RFC
considers it to be so.

As an extension of this, it should be noted that it is impossible to update a
dependency which contains a breaking change that affects your library, without
making a breaking change yourself. As such, all libraries should follow Rust's
example by committing to stability. Libraries which are stable (version is
greater than 1.0.0) should not include public dependencies which are unstable
(version is 0.x).

Some libraries such as chrono, uuid, and semver provide types which are
fundamental to their domain. These libraries are much more likely to be used as
public dependencies, and should endeavor to stabilize as quickly as possible to
avoid blocking the rest of the crates.io ecosystem.

It should be noted that a dependency is not considered public if the only
visible usage of it is publicly re-exporting items. This is because any
downstream crates can forwards compatibly protect themselves by depending on the
crate which originally provided those items directly.

### Minor change: modifying the version range of a public dependency to a superset of the previous range

An example of this would be changing a version range from `2.0.0` to `>= 2.0.0,
< 3.0.0`. This change would still break downstream code, as `cargo update` would
always resolve the transitive dependency to the highest supported version, even
if a lower version within the range is used elsewhere. However, users of the
library can always ask Cargo to resolve a transitive dependency to an exact
version as long as it's within the supported range.

### Minor change: modifying the version range of a private dependency

Since cargo allows multiple versions of a crate to be used transitively,
libraries are free to change private dependencies as they see fit.

### Minor change: changing the features of a dependency

This is mentioned briefly by [RFC #1105](https://github.com/rust-lang/rfcs/blob/d43f4cabb3c3607b5dad6d9dbc7ba3758a3ebc3a/text/1105-api-evolution.md#minor-change-altering-the-use-of-cargo-features).
However, this RFC would like to add that crates should not rely on the absence
of a feature in their dependencies. The statement "packages are supposed to
support any combination of features" applies not only to the features of that
library, but also to all the features of its dependencies.

### Major change: making a public dependency optional

This could also be generalized to "moving any public item behind a Cargo
feature".

### Minor change: adding a new dependency

Adding any dependency could potentially cause downstream crates to break. Some
crates are going to rely on platform specifics. Some crates will link against
native libraries which aren't always present. Some crates will have build
scripts that fail in certain environments. We should not put the burden on a
library author to ensure that all dependencies and transitive dependencies
compile on all supported platforms. Therefore, library authors are allowed to
generally assume that adding a dependency will not by itself break downstream
code.

That said, in cases where a library author does know that a dependency does not
support all platforms, they should make a best effort to limit its usage behind
cfg-attrs and/or cargo features.

This also applies to `dev_dependencies` and `build_dependencies`.

# How We Teach This
[how-we-teach-this]: #how-we-teach-this

This RFC is purely an amendment to an existing one, however that RFC was written
before this section.

For this RFC to be taught, the terms "public dependency" and "private
dependency" should continue to be used. In place of the exhaustive definition, a
public dependency can probably be defined as "a dependency which is visible to
consumers of your library", with a link to this RFC for the exhaustive
definition.

The most effective way to teach this to new users is likely in the Docs section
of Crates.io, with a link to this RFC or relevant added guide in the Getting
Started guide.

This does not require any additions or changes to the Rust Reference, _The Rust
Programming Language_, or _Rust by Example_.

# Drawbacks
[drawbacks]: #drawbacks

This adds specific meaning to "public dependency" and "private dependency",
which are terms Cargo may use in the future. In particular, Cargo may eventually
add an explicit notion of a public dependency. If that were to occur, this RFC
could simply be amended to refer to that meaning instead. However, there could
still potentially be bitrot and a general burden of updating outdated
information.

Additionally, by noting that changing the versions of public dependencies is a
major breaking change, we risk causing a deadlock in the crates.io ecosystem. At
the time this RFC was written, only 9.6% of the crates published are post-1.0.
Since crates would be less likely to publish a 1.0 version until their
public dependencies are post-1.0, a handful of crates could end up effectively
blocking the entire Rust ecosystem.

However, while this RFC explicitly states it as a breaking change, many members
of the ecosystem will already consider the guidelines laid out here to be the
case. It could also be argued that knowing that certain crates could block the
ecosystem from going 1.0 will encourage authors of those libraries to lay out
what their blockers for 1.0 are, and encourage the community to help fix those
blockers.

# Alternatives
[alternatives]: #alternatives

n/a

# Unresolved questions
[unresolved]: #unresolved-questions

n/a

[1105]: https://github.com/rust-lang/rfcs/blob/master/text/1105-api-evolution.md
