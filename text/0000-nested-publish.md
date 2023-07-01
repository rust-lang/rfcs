- Feature Name: `nested_publish`
- Start Date: 2023-06-30
- RFC PR: ...
- Rust Issue: ...

# Summary
[summary]: #summary

Allow Cargo packages to be bundled within other Cargo packages when they are published (not just in unpublished workspaces).

# Motivation
[motivation]: #motivation

There are a number of reasons why a Rust developer currently may feel the need to create multiple library crates, and therefore multiple Cargo packages (since one package contains at most one library crate). These multiple libraries could be:

* A trait declaration and a corresponding derive macro (which must be defined in a separate proc-macro library).
* A library that uses a build script that uses another library or binary (e.g. for precomputation or bindings generation).
* A logically singular library broken into multiple parts to speed up compilation.

Currently, developers must publish these packages separately. This has several disadvantages (see the [Rationale](#rationale-and-alternatives) section for further details):

* Clutters the public view of the registry with packages not intended to be usable on their own, and which may even become obsolete as internal architecture changes.
* Requires multiple `cargo publish` operations (this could be fixed with bulk publication) and writing public metadata for each package.
* Can result in semver violations and thus compilation failures, due to the developer not thinking about semver compatibility within the group.

This RFC will allow developers to avoid all of these inconveniences and hazards by publishing a single package.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

By default (and always, prior to this RFC's implementation):

* If your package contains any sub-packages, Cargo [excludes](https://doc.rust-lang.org/cargo/reference/manifest.html#the-exclude-and-include-fields) them from the `.crate` archive file produced by `cargo package` and `cargo publish`.
* If your package contains any non-`dev` dependencies which do not give a `version = "..."`, it cannot be published to `crates.io`.

(By “**sub-package**” we mean a package (directory with `Cargo.toml`) which is a subdirectory of another package. We shall call the outermost such package, the package being published, the “**parent package**”.)

You can change this default by placing in the manifest (`Cargo.toml`) of a sub-package:

```toml
[package]
publish = "nested"
```

If this is done, Cargo's behavior changes as follows:

* If you publish the parent package, the sub-package is included in the `.crate` file (unless overridden by explicit `exclude`/`include`) and will be available to the parent package whenever the parent package is downloaded and compiled.
* The parent package may have a `path =` dependency upon the sub-package. (This dependency may not have a `version =` specified.)
* You cannot `cargo publish` the sub-package, just as if it had `publish = false`. (This is a safety measure against accidentally publishing the sub-package separately when this is not intended.)

Nested sub-packages may be freely placed within other nested sub-packages.

When a group of packages is published in this way, and depended on, this has a number of useful effects (which are not things that Cargo explicitly implements, just consequences of the system):

* The packages are versioned in lockstep; there is no way for a version mismatch to arise since all the code was published together. Version resolution does not apply (in the same way that it does not for any other `path =` dependency).
* The sub-package is effectively “private”: it cannot be named by any other package on `crates.io`, only by its parent package and sibling sub-packages.

## Example: trait and derive macro

Suppose we want to declare a trait-and-derive-macro package. We can do this as follows. The parent package would have this manifest `foo/Cargo.toml`:

```toml
[package]
name = "foo"
version = "0.1.0"
edition = "2021"
publish = true

[dependencies]
foo-macros = { path = "macros" }    # newly permitted
```

The sub-package manifest `foo/macros/Cargo.toml`:

```toml
[package]
name = "macros"                     # this name need not be claimed on crates.io
version = "0.1.0"                   # this version is not used for dependency resolution
edition = "2021"
publish = "nested"                  # new syntax

[lib]
proc-macro = true
```

Then you can `cargo publish` from within the parent `foo` directory, and this will create a single `foo` package on `crates.io`, with no `macros` (or `foo-macros`) package visible except when inspecting the source code or in compilation progress messages.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The following changes must be made across Cargo and `crates.io`:

* **Manifest schema**
    * The Cargo manifest now allows `"nested"` as a value for the `package.publish` key.
* **`cargo package` &amp; `cargo publish`**
    * Should refuse to publish a package if that package (not its sub-packages) has `publish = "nested"`.
    * Exclude/include rules should, upon finding a sub-package, check if it is `publish = "nested"` and not automatically exclude it. Instead, they should treat it like any other subdirectory; in particular, it should be affected by explicitly specified exclude/include rules.
    * Nested `Cargo.toml`s should be normalized in the same way the root `Cargo.toml` is, if they declare `publish = "nested"`, and not if they do not.
        * This avoids modifying the publication behavior for existing packages, even if they contain project templates or invoke `cargo` to compile sub-packages to probe the behavior of the compiler.
        * If the nested `Cargo.toml` has a syntax error such that its `package.publish` value cannot be determined, then if it is depended upon, emit an error; if it is not, emit a warning and do not normalize it.
* **`crates.io`**
    * Should allow `path` dependencies that were previously prohibited, at least provided that the named package in fact exists in the `.crate` archive file. The path must not contain any upward traversal (`../`) or other hazardous or non-portable components.
* **Build process**
    * Probably some messages will need to be adjusted; currently, `path` dependencies' full paths are always printed in progress messages, but they would be long noise here (`/home/alice/.cargo/registry/src/index.crates.io-6f17d22bba15001f/...`). Perhaps progress for sub-packages could look something like “`Compiling foo/macros v0.1.0`”.

The presence or absence of a `[workspace]` has no effect on the new behavior, just as it has no effect on existing package publication.

# Drawbacks
[drawbacks]: #drawbacks

* This increases the number of differences between “Cargo package (on disk)” from “Cargo package (that may be published in a registry, or downloaded as a unit)” in a way which may be confusing; it would be good if we have different words for these two entities, but we don't.
* If Cargo were to add support for multiple libraries per package, that would be largely redundant with this feature.
* It is not possible to publish a bug fix to a sub-package without republishing the entire parent package.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The reason for doing anything at all in this area is that publishing multiple packages is often a bad solution to the problems that motivate it; in particular:

* Non-lockstep versioning risk: If you publish `foo 1.0.0` and `foo-macros 1.0.0`, then later publish `foo 1.1.0` and `foo-macros 1.1.0`, then it is _possible_ for users' `Cargo.lock`s to get into a state where they select `foo-macros 1.1.0` and `foo 1.0.0`, and this then breaks because `foo-macros` assumed that items from `foo 1.0.0` would be present. Arguably, this is a deficiency in the proc-macro system (`foo-macros` has a _de facto_ dependency on `foo` but does not declare it), but not one that is likely to be corrected any time soon. This can be worked around by having `foo` specify an exact dependency `foo-macros = "=1.0.0"`, but this is a subtlety that library authors do not automatically think of; semver is easy to get wrong silently.
* The crates.io registry may be cluttered with many packages that are not relevant to users browsing packages. (Of course, there are many other reasons why such clutter will be found.)
* When packages are implementation details, it makes a permanent mark on the `crates.io` registry even if the implementation of the parent package stops needing that particular subdivision. By allowing sub-packages we can allow package authors to create whatever sub-packages they imagine might be useful, and delete them in later versions with no consequences.
* It is possible to depend on a published package that is intended as an implementation detail. Ideally, library authors would document this clearly and library users would obey the documentation, but that doesn't always happen. By allowing nested packages, we introduce a simple “visibility” system that is useful in the same way that `pub` and `pub(crate)` are useful within Rust crates.

The alternative to nested packages that I have heard of as a possibility would be to support multiple library targets per package. That would be arguably cleaner, but has these disadvantages:

* It would require new manifest syntax, not just for declaring the multiple libraries, but for referring to them, and for making per-target dependencies (e.g. only a proc-macro lib should depend on `proc-macro2`+`quote`+`syn`, not the rest of the libraries in the package).
* It would require many new mechanisms in Cargo.
* It might have unforeseen problems; by contrast, nested packages are compiled exactly the same way `path` dependencies currently are, and the only new element is the ability to publish them, so the risk of surprises is lower.

Also, nested packages enables nesting *anything* that Cargo packages can express now and in the future; it is composable with other Cargo functionality.

We could also do nothing, except for warning the authors of paired macro crates that they should use exact version dependencies. The consequence of this will be continued hassle for developers; it might even be that useful proc-macro features might not be written simply because the author does not want to manage a second package.

## Details within this proposal

Instead of introducing a new value for the `publish` key, we could simply allow sub-packages to be published when they would previously be errors. However, this would be problematic  when an existing package has a dev-dependency on a sub-package; either that sub-package would suddenly start being published as nested, or there would be no way to specify the sub-package *should* be published.

We could also introduce an explicit `[subpackages]` table in the manifest. However, I believe `publish = "nested"` has the elegant and worthwhile property that it simultaneously enables nested publication and prohibits accidental un-nested publication of the sub-package.

# Prior art
[prior-art]: #prior-art

I am not aware of other package systems that have a relevant similar concept, but I am not broadly informed about package systems. I have designed this proposal to be a **minimal addition to Cargo**, building on the existing concept of `path` dependencies to add lots of power with little implementation cost; not necessarily to make sense from a blank slate.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

I see no specific unclear design choices, but we might want to incorporate one or more of the below _Future possibilities_ into the current RFC, particularly omitting version numbers.

# Future possibilities
[future-possibilities]: #future-possibilities

## Omit version numbers

Nested packages don't really have any use for version numbers; arguably, they should be omitted and even prohibited, since they may mislead a reader into thinking that the version numbers are used for some kind of version resolution. However, this is a further change to Cargo that is not strictly necessary to solve the original problem, and it disagrees with the precedent of how local `path` dependencies currently work (local packages must have version numbers even though they are not used).

## Nested packages with public binary targets

One common reason to publish multiple packages is in order to have a library and an accompanying tool binary, without causing the library to have all of the dependencies that the binary does. Examples: `wasm-bindgen` (`wasm-bindgen-cli`), `criterion` (`cargo-criterion`), `rerun` (`rerun-cli`).

This RFC currently does not address that — if nothing is done, then `cargo install` will ignore binaries in sub-packages. It would be easy to make a change which supports that; for example, `cargo install` could traverse sub-packages and install all found binaries — but that would also install binaries which are intended as testing or (once [artifact dependencies] are implemented) code-generation helpers, which is undesirable. Thus, additional design work is needed to support `cargo install`ing from subpackages:

* Should there be an additional manifest key which declares the binary target “public”?
* Should targets be explicitly “re-exported” from the parent package?
* Should there be an additional option to `cargo install` which picks subpackages? (This would cancel out the user-facing benefit from having a single package name.)

## Nested packages with public library targets

Allowing nested libraries to be named and used from outside the package would allow use cases which are currently handled by Cargo `features` and conditional compilation  (optional functionality with nontrivial costs in dependencies or compilation time)  to be instead handled by defining additional public libraries within one package.

This would allow library authors to avoid writing fragile and hard-to-test conditional compilation, and allow library users to avoid accidentally depending on a feature being enabled despite not having enabled it explicitly. It would also allow compiling the optional functionality and its dependencies with maximum parallelism, by not introducing a single `feature`-ful library crate which acts as a single node in the dependency graph.

However, it requires additional syntax and semantics, and these use cases might be better served by [#3243 packages as namespaces] or some other namespacing proposal, which would allow the libraries to be published independently. (I can also imagine a world in which both of these exist, and the library implementer can transparently use whichever publication strategy best serves their current needs.)

[artifact dependencies]: https://github.com/rust-lang/rfcs/pull/3028
[#3243 packages as namespaces]: https://github.com/rust-lang/rfcs/pull/3243

## Additional privileges between crates

Since nested packages are versioned as a unit, we could relax the trait coherence rules and allow implementations that would otherwise be prohibited.

This would be particularly useful when implementing traits from large optional libraries; for example, package `foo` with subpackages `foo_core` and `foo_tokio` could have `foo_tokio` write `impl tokio::io::AsyncRead for foo_core::DataSource`. This would improve the dependency graph compared to `foo_core` having a dependency on `tokio` (which is the only way to do this currently), though not have the maximum possible benefit unless we also added public library targets as above, since the package as a whole still only exports one library and thus one dependency graph node.
