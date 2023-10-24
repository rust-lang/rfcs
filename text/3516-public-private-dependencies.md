- Feature Name: `public_private_dependencies`
- Start Date: 2023-10-13
- Prior RFC PR: [rust-lang/rfcs#1977](https://github.com/rust-lang/rfcs/pull/1977)
- Pre-RFC: [Pre-RFC: Superseding public/private dependencies](https://internals.rust-lang.org/t/pre-rfc-superseding-public-private-dependencies/19708)
- RFC PR: [rust-lang/rfcs#3516](https://github.com/rust-lang/rfcs/pull/3516)
- Rust Issue: [rust-lang/rust#44663](https://github.com/rust-lang/rust/issues/44663)

# Summary
[summary]: #summary

Introduce a public/private distinction to crate dependencies.

Note: this supersedes [RFC 1977]
Enough has changed in the time since that RFC was approved that we felt we needed to go back and get wider input on this, rather than handling decisions through the tracking issue.
- [RFC 1977] was written before Editions, `cfg` dependencies, and package renaming which can all affect it
- The resolver changes were a large part of [RFC 1977] but there are concerns with it and we feel it'd be best to decouple it, offering a faster path to stabilization

Note: The 2024 Edition is referenced in this RFC but that is a placeholder for
whatever edition next comes up after stabilization.

# Motivation
[motivation]: #motivation

The crates ecosystem has greatly expanded since Rust 1.0. With that, a few patterns for
dependencies have evolved that challenge the existing dependency declaration
system in Cargo and Rust. The most common problem is that a crate `A` depends on another
crate `B` but some of the types from crate `B` are exposed through the API in crate `A`.

- Brittle semver compatibility as `A` might not have intended to expose `B`,
  like when adding `impl From<B::error> for AError` for convenience in using `?` in the implementation of `A`.
- When self-hosting documentation, you may want to render documentation for all of your public dependencies as well
- When running `cargo doc`, users may way to render [documentation for their accessible dependencies](https://github.com/rust-lang/cargo/issues/2025) [without the cost of their inaccessible dependencies](https://github.com/rust-lang/cargo/issues/4049)
- When linting for semver compatibility [there isn't enough information to reason about dependencies](https://github.com/obi1kenobi/cargo-semver-checks/issues/121)

Related problems with this scenario not handled by this RFC:
- Poor error messages when a user directly depends on `A` and `B` but with a
  version requirement on `B` that is semver incompatible with `A`s version
  requirement on `B`.
  - See [Dependency visibility and the resolver](#rationale-and-alternatives) for why this is excluded.
- Allow mutually exclusive features or overly-constrained version requirements
  by not requiring private dependencies to be unified.
  - Private dependencies are not sufficient on their own for this
  - There are likely better alternatives, like [Pre-RFC: Mutually-exclusive, global features](https://internals.rust-lang.org/t/pre-rfc-mutually-excusive-global-features/19618)
- Help check for missing feature declarations by duplicating dependencies, rather than unifiying features
  - See [Missing feature declaration check](#future-possibilities)

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

As a trivial, artificial example:
```toml
[package]
name = "diagnostic"
version = "1.0.0"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```
```rust
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct Diagnostic {
    code: String,
    message: String,
    file: std::path::PathBuf,
    span: std::ops::Range<usize>,
}

impl std::str::FromStr for Diagnostic {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}
```

The dependencies `serde` and `serde_json` are both public dependencies, meaning their types are referenced in the public API.
This has the implication that a semver incompatible upgrade of these dependencies is a breaking change for this package.

With this RFC, in pre-2024 editions,
you can enable add `lints.rust.external_private_dependency = "warn"` to your
manifest and rustc will warn saying that `serde` and `serde_json` are private
dependencies in a public API.
In 2024+ editions, this will be an error.

To resolve the warning in a semver compatible way, they would need to declare both dependencies as public:
```toml
[package]
name = "diagnostic"
version = "1.0.0"

[dependencies]
serde = { version = "1", features = ["derive"], pub = true }
serde_json = { version = "1", pub = true }
```
For edition migrations, `cargo fix` will look for the warning code and mark those dependencies as `pub`.

However, for this example, it was an oversight in exposing `serde_json` in the public API.
Note that removing it from the public API is a semver incompatible change.
```toml
[package]
name = "diagnostic"
version = "1.0.0"

[dependencies]
serde = { version = "1", features = ["derive"], pub = true }
serde_json = "1"
```
```rust
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct Diagnostic {
    code: String,
    message: String,
    file: std::path::PathBuf,
    span: std::ops::Range<usize>,
}

impl std::str::FromStr for Diagnostic {
    type Err = Error

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s).map_err(Error)
    }
}

pub struct Error(serde_json::Error);
```

If you then had a public dependency on `diagnostic`,
then `serde` would automatically be considered a public dependency of yours.

At times, some public dependencies are effectively private.
Take this code from older versions of `clap`
```rust
#[doc(hidden)]
#[cfg(feature = "derive")]
pub mod __derive_refs {
    #[doc(hidden)]
    pub use once_cell;
}
```
Since the proc-macro can only guarantee that the namespace `clap` is accessible,
`clap` must re-export any functionality that is needed at runtime by the generated code.
As a last-ditch way of dealing with this, a user may allow the error:
```rust
#[doc(hidden)]
#[allow(external_private_dependency)]
#[cfg(feature = "derive")]
pub mod __derive_refs {
    #[doc(hidden)]
    pub use once_cell;
}
```
A similar case is pub-in-private:
```rust
mod private {
    #[allow(external_private_dependency)]
    pub struct Foo { pub x: some_dependency::SomeType }
}
```
Though this might be worked around by reducing the visibility to `pub(crate)`.

I say "last ditch" because in most other cases,
a user would be better served by wrapping the API which would be helped with
features like `impl Trait` in type aliases if we had it.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## rustc

The main change to the compiler will be to accept a new modifier on the `--extern` flag that Cargo
supplies which marks it as a private dependency.
The modifier will be called `priv` (e.g. `--extern priv:serde`).
The compiler then emits the lint `external_private_dependency` if it encounters private
dependencies exposed as `pub`.

`external_private_dependency` will be `allow` by default for pre-2024 editions.
It will be a member of the `rust-2024-compatibility` lint group so that it gets automatically picked up by `cargo fix --edition`.
In the 2024 edition, this lint will be `deny`.

In some situations, it can be necessary to allow private dependencies to become
part of the public API. In that case one can permit this with
`#[allow(external_private_dependency)]`. This is particularly useful when
paired with `#[doc(hidden)]` and other already existing hacks.
This most likely will also be necessary for the more complex relationship of
`libcore` and `libstd` in Rust itself.

## cargo

A new dependency field, `pub = <bool>` will be added that defaults to `false`.
This field can be specified in `workspace.dependencies` and be overridden when `workspace = true` is in a dependency.
When building a `lib`, Cargo will use the `priv` modifier with `--extern` for all private dependencies.
What is private is what is left after recursively walking public dependencies (`pub = true`).
We'll ignore this for other build target kinds (e.g. `bin`) as that would create extra noise.

Old cargo versions will emit a warning when this key is encountered but otherwise continue,
even if the feature is present but unstable.
While it is unstable, `cargo publish` will strip the field.

Cargo will not force a `rust-version` bump when using this feature as someone
building with an old version of cargo depending on packages that set `pub =
true` will not start to fail when upgrading to new versions of cargo.

`cargo add` will gain `--pub <bool>` flags to control this field.
When adding a dependency today, the version requirement is reused from other dependency tables within your manifest.
With this RFC, that will be extended to also checking your dependencies for any `pub` dependencies, and reusing their version requirement.
This would be most easily done by having the field in the Index but `cargo add` could also read the `.crate` files as a fallback.

## crates.io

Crates.io should show public dependencies more prominently than private ones.

# Drawbacks
[drawbacks]: #drawbacks

It might not be clear how to resolve the warning/error, as it's emitted
by rustc but is resolved by changing information in the build system,
generally, but not always, cargo.
As a last resort, we could put a hack in cargo to intercept the lint and emit a
new version of it that explains things in terms of cargo.

There are risks with the `cargo fix` approach as it requires us to take a non-machine applicable lint,
parsing out the information we need to identify the corresponding `Cargo.toml`,
and translate it into a change for `Cargo.toml`.

In the case where you depend on `foo = "300"`, there isn't a way to clarify that what is public is actually from `foo-core = "1"` without explicitly depending on it.

This doesn't cover the case where a dependency is public only if a feature is enabled.

The warning/error is emitted even when a `pub` item isn't accessible.

You can't definitively lint when a `pub = true` is unused since it may depend on which platform or features.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Misc

- `Cargo.toml`: instead of `pub` (named after the [Rust keyword](https://doc.rust-lang.org/reference/visibility-and-privacy.html), we could name the field `public` (like [RFC 1977]) or name the field `visibility = "<public|private>"`
  - The parallel with Rust seemed worth pursuing
  - `pub` could be seen as ambiguous with `publish`
  - While `visibility` would offer greater flexibility, it is unclear if we need that flexibility and if the friction of any feature leveraging it would be worth it
  - In the end, we went with `pub` because `public` would prevent our "no MSRV bump" approach because cargo versions exist with `public` being reserved
- `rustc`: Instead of `allow` by default for pre-2024 editions, we could warn by default
  - More people would get the benefit of the feature now
  - However, this would be extremely noisy and likely make people grumpy
- `Cargo.toml`: Instead of `pub = false` being the default and changing the warning level on an edition boundary, we could instead start with `pub = true` and change the default on an edition boundary.
  - This would require `cargo fix` marking all dependencies as `pub = true`, while using the warning means we can limit it to only those dependencies that need it.
- `Cargo.toml`: Instead of `pub = false` being the default, we could have a "unchecked" / "unset" state
  - This would require `cargo fix` marking all dependencies as `pub = true`, while using the warning means we can limit it to only those dependencies that need it.
- `Cargo.toml`: In the long term, we decided on the default being `pub = false` as that is the common case and gives us more information than supporting a `pub = "unchecked"` and having that be the long term solution.
- `cargo add`: instead of `--pub <bool>` it could be `--pub` / `--no-pub` like `--optional` or `--public` / `--private`
- `cargo add`: when adding a dependency, we could automatically add all of its `pub` dependencies.
  - This was passed up as being too noisy, especially when dealing with facade crates, those that fully re-export their `pub = true` dependency

## Minimal version resolution

[RFC 1977] included the idea of verifying version requirements are high enough.
This is a problem whether the dependency is private or not.
This should be handled independent of this RFC.

## Dependency visibility and the resolver

This is deferred to [Future possibilities](#future-possibilities)
- This has been the main hang-up for stabilization over the last 6 years since the RFC was approved
  - For more on the complexity involved, see the thread starting at [this comment](https://github.com/rust-lang/rust/issues/44663#issuecomment-881965668)
- More thought is needed as we found that making a dependency `pub = true` can be a breaking change if the caller also depends on it but with a different semver incompatible version
- More thought is needed on what happens if you have multiple versions of a package that are public (via renaming like `tokio_03` and `tokio_1`)

Related problems potentially blocked on this
- It is hoped that the resolver change would help with [cargo#9029](https://github.com/rust-lang/cargo/issues/9029)
- If we allow duplication of private semver compatible dependencies, it would help with [cargo#10053](https://github.com/rust-lang/cargo/issues/10053)

# Prior art
[prior-art]: #prior-art

Within the cargo ecosystem:
- [cargo public-api-crates](https://github.com/davidpdrsn/cargo-public-api-crates)

# Unresolved questions
[unresolved]: #unresolved-questions

# Future possibilities
[future-possibilities]: #future-possibilities

## Help keep versions in-sync

When upgrading one dependency, you might need to upgrade another because you
use it to interact with the first, like `clap` and `clap_complete`.
The existing error messages are not great, along the lines of "expected `clap::Command`, found `clap::Command`".
Ideally, you would be presented instead with a message saying "clap_complete
3.4 is not compatiblw with clap 4.0, try upgrading to clap_complete 4.0".
Even better if we could help users do this upgrade automatically.

As solving this, via the resolver, has been the main sticking point for [RFC 1997],
this was deferred out to take smaller,
more incremental steps,
that open the
door for more experimentation in the future to understand how best to solve
these problems.

Some possible routes:

### Dependency visibility and the resolver

[RFC 1977] originally proposed handling this within the resolver

Cargo will specifically reject graphs that contain two different versions of the
same crate being publicly depended upon and reachable from each other. This will
prevent the strange errors possible today at version resolution time rather than at
compile time.

How this will work:

* First, a resolution graph has a bunch of nodes. These nodes are "package ids"
  which are a triple of (name, source, version). Basically this means that different
  versions of the same crate are different nodes, and different sources of the same
  name (e.g. git and crates.io) are also different nodes.
* There are *directed edges* between nodes. A directed edge represents a dependency.
  For example if A depends on B then there's a directed edge from A to B.
* With public/private dependencies, we can now say that every edge is either tagged
  with public or private.
* This means that we can have a collection of subgraphs purely connected by public
  dependency edges. The directionality of the public dependency edges within the
  subgraph doesn't matter. Each of these subgraphs represents an "ecosystem" of
  crates publicly depending on each other. These subgraphs are "pools of public
  types" where if you have access to the subgraph, you have access to all types
  within that pool of types.
* We can place a constraint that each of these "publicly connected subgraphs" are
  required to have exactly one version of all crates internally. For example, each
  subgraph can only have one version of Hyper.
* Finally, we can consider all pairs of edges coming out of one node in the
  resolution graph. If the two edges point to *two distinct publicly connected
  subgraphs from above* and those subgraphs contain two different versions of the
  same crate, we consider that an error. This basically means that if you privately
  depend on Hyper 0.3 and Hyper 0.4, that's an error.

If we want to go this route, some hurdles to overcome include:
- Difficulties in working with cargo's resolver as this has been the main hang-up for stabilization over the last 6 years since the [RFC 1977] was approved
  - For more on the complexity involved, see the thread starting at [this comment](https://github.com/rust-lang/rust/issues/44663#issuecomment-881965668)
- More thought is needed as we found that making a dependency `pub = true` can be a breaking change if the caller also depends on it but with a different semver incompatible version
- More thought is needed on what happens if you have multiple versions of a package that are public (via renaming like `tokio_03` and `tokio_1`)

### Caller-declared relations

As an alternative, when declaring dependencies,
a user could [explicitly delegate the version requirement to another package](https://github.com/rust-lang/cargo/issues/4641)

One possible approach for this:
```toml
[package]
name = "some-cli"

[dependencies]
clap = { version.from ["clap_complete"] }
clap_complete = "3.4"
```
When resolving the dependencies for `some-cli`,
the resolver will not explicitly choose a version for `clap` but will continue resolving the graph.
Upon completion, it will look to see what version of `clap_complete` was
resolved and act as if that was what was specified inside of the in-memory
`clap` dependency.

The packakge using `version.from` must be a public dependency of the `from` package.
In this case, `clap` must be a public dependency of `clap_complete`.
If the different packages in `version.from` do not agree on what the package
version should resolve to (clap 3.4 vs clap 4.0), then it is an error.

Compared to the resolver doing this implicitly
- It is unclear if this would be any more difficult to implement in the resolver
- Changing a dependency from `pub = false` to `pub = true` is backwards compatible because it has no affect on existing callers.
- It is unclear how this would handle multiple versions of a package that are public

The downside is it feels like the declaration is backwards.
If you have one core crate (e.g. `clap`) and many crates branching off (e.g. `clap_complete`, `clap_mangen`),
it seems like those helper crates should have their version picked from `clap`.

## Missing feature declaration check

It is easy for packages to accidentally rely on a dependency enabling a feature for them.
We could add a mode that limits feature unification to reachable dependencies,
forcing duplication and longer builds for the sake of checking if any features
need specifying.

However, this will still likely miss a lot of cases, making the pay off questionable.
This also has the risk of being abused as a workaround so people can use
mutually exclusive features.
If packages start relying on it,
it could coerce callers into abusing this mechanism,
having a cascading effect in the ecosystem in the wrong direction.

## Warn about semver incompatible public dependency

`cargo update` (or a manifest linter) could warn about public new incompatible
public dependencies that are available to help the ecosystem progress in
lockstep.

## Warn about pre-1.0 public dependencies in 1.0+ packages

See [rust-lang/cargo#6018](https://github.com/rust-lang/cargo/issues/6018)

[RFC 1977]: https://github.com/rust-lang/rfcs/pull/1977
