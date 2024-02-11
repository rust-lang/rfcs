- Feature Name: `nested_publish`
- Start Date: 2023-06-30
- RFC PR: [rust-lang/rfcs#3452](https://github.com/rust-lang/rfcs/pull/3452)
- Rust Issue: ...

# Summary
[summary]: #summary

Allow published Cargo packages to depend on other packages stored within themselves, as is currently possible in unpublished packages.

# Motivation
[motivation]: #motivation

There are a number of reasons why a Rust developer currently may feel the need to create multiple library crates, and therefore multiple Cargo packages (since one package contains at most one library crate). These multiple libraries could be:

* A trait declaration and a corresponding derive macro (which must be defined in a separate proc-macro library).
* A library that uses a build script that uses another library or binary (e.g. for precomputation or bindings generation).
* A logically singular library broken into multiple parts to speed up compilation.

Currently, developers must publish these packages separately. This has several disadvantages (see the [Rationale](#rationale-and-alternatives) section for further details):

* Can result in semver violations and thus compilation failures, due to the developer not thinking about semver compatibility within the group.
* Requires multiple `cargo publish` operations (though this could be fixed with a bulk publication feature) and writing public metadata for each package.
* Clutters the public view of the registry with packages not intended to be usable on their own, and which may even become obsolete as internal architecture changes.
* Requires each package to have a [full set of metadata](https://doc.rust-lang.org/cargo/reference/publishing.html#before-publishing-a-new-crate).

This RFC will allow developers to avoid all of these inconveniences and hazards by publishing a single package.

There are also some uses which are not strictly cases of one library package versus multiple library packages:

* It may sometimes be desirable to share a small amount of code between some published packages, without making the shared code a separately published library with an appropriate public API subject to semver.
* A package intended to distribute a binary or binaries may have a library target for internal purposes (such as sharing modules between multiple binaries, or testing), but not intend for that library to be usable by other packages as a dependency.

# Definitions
[definitions]: #definitions

* (existing) A “**package**” is a directory with a `Cargo.toml` file, where that `Cargo.toml` file contains `[package]` metadata. (Note that valid `Cargo.toml` files can also declare `[workspace]`s without being packages; such files are irrelevant to this RFC.)
* (existing) A “**sub-package**” is a package (directory with `Cargo.toml`) which is located in a subdirectory of another package. (This is an existing term in Cargo documentation, though only once.)
* (new) A “**parent package**”, in the context of this RFC, is a package which is being or has been published, and which may contain sub-packages.
* (new) A “**nested package**” is a package which is published as part of some parent package, using this mechanism to allow dependencies, rather than independently.
    * A published package may contain sub-packages that are not nested packages, as a simple file inclusion for the package's own build-time purposes.
    * Note that a nested package is not necessarily a direct dependency of the parent package, though that will be the typical case.
* (new) A “**nested dependency**” is an entry in `[dependencies]` that refers to a nested package, and is declared as such.
* (not for documentation but for discussion in this RFC) “**nested publishing**” means a `cargo publish` operation that includes one or more nested packages, or the act of actually making use of the fact that some packages are marked as nested packages.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

By default (and always, prior to this RFC's implementation):

* If your package contains any sub-packages in its directory structure, Cargo [excludes](https://doc.rust-lang.org/cargo/reference/manifest.html#the-exclude-and-include-fields) them from the `.crate` archive file produced by `cargo package` and `cargo publish`.
* If your package contains any non-`dev` dependencies which do not give a `version = "..."`, it cannot be published to `crates.io`.
* If your package contains `[dev-dependencies]` which do not give a `version = "..."`, they are stripped out on publication.

You can change this default in your manifests. First, in the manifest (`Cargo.toml`) of a sub-package, add `publish = "nested"`:

```toml
[package]
name = "foo-macros"
publish = "nested"
```

Then, in the manifest of the parent package, declare the dependency as `publish = "nested"`:

```toml
[dependencies]
foo-macros = { path = "macros", publish = "nested" }
```

If both of these steps are done, the `foo-macros` package is considered a **nested package**, and Cargo's behavior changes as follows:

* If you publish the parent package, the sub-package is included in the `.crate` archive file and will be available to the parent package whenever the parent package is downloaded and compiled. If the dependency `path` does not lead to a subdirectory, then the sub-package will be automatically copied into a location under the `.cargo` directory inside the top level of the `.crate` archive file, and the `path` value will be rewritten to match.
* The parent package (and other sub-packages) may have `path =` dependencies upon the sub-package. (Such dependencies must not have a `version =` or `git =`; that is, the `path` must be the _only_ source for the dependency. They do not need to also declare `publish = "nested"`.)
* You cannot `cargo publish` the sub-package, just as if it had `publish = false`. (This is a safety measure against accidentally publishing the sub-package separately _in addition_ to its nested copies; the presumption here is that nested packages are not designed to present public API themselves.)

Nested packages may contain other dependencies on nested packages, and these too are included in the published package.

When a group of packages is published in this way, this has a number of useful effects for its dependents (which are not things that Cargo explicitly implements, just consequences of the system):

*   The packages are a single unit for all versioning purposes; there is no way for a version mismatch to arise among them since all the code was published together. Version resolution does not apply (in the same way that it does not for any other `path =` dependency).

    This allows package authors to avoid needing to think about SemVer correctness for their nested packages.
*   The sub-package is effectively “private” (in a sense like the Rust language's visibility system): it cannot be named as a dependency by any other package on `crates.io`, only by its parent package and sibling sub-packages. The parent package may still re-export items from it, or even the entire crate, in the same ways as it could do with a dependency on a normally published package.

## Example: trait and derive macro

Suppose we want to declare a trait-and-derive-macro package. We can do this as follows. The parent package would have this manifest `foo/Cargo.toml`:

```toml
[package]
name = "foo"
version = "0.1.0"
edition = "2021"
publish = true

[dependencies]
foo-macros = { path = "macros", publish = "nested" } # new syntax
```

The sub-package manifest `foo/macros/Cargo.toml`:

```toml
[package]
name = "macros"            # this name need not be claimed on crates.io
# version = "0.0.0"        # version number is not used and may be omitted
edition = "2021"
publish = "nested"         # new syntax

[lib]
proc-macro = true
```

Then you can `cargo publish` from within the parent directory `foo/`, and this will create a single `foo` package on `crates.io`, with no `macros` (or `foo-macros`) package visible except when inspecting the source code or in compilation progress messages.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Cargo: Manifest

Two new possible values are added to the manifest.

* The `package.publish` field allows `"nested"` as a value, in addition to existing `false` and `true`. This value affects `cargo publish` and nested dependencies as described below.
* The `dependencies.*.publish` field is newly defined, with the only currently allowed value being `"nested"`, to declare that that dependency is a nested dependency. (If desired, `publish = false` could be also be permitted to explicitly document an intent not to nest; this would be identical to the status quo and to omitting the key.)
    * It is an error if a nested dependency does not have a `path` field, or if it has a `version`, `git`, or any other package source field, unless future work defines a meaning for that combination.

When a nested dependency is present (making its referent be a nested package), the following additional requirements apply:

* The nested package must have `package.publish = "nested"`; both `false` and `true` are errors.

* If the nested package specifies `package.license`, its value must be identical to the parent package's.

  This check is intended only to prevent accidents (such as vendoring a third-party package without considering the implications of redistributing it). It is always valid to omit `package.license` from the nested package, thus making no machine-readable claims about its licensing.

It is an error for any two of the packages in the transitive closure of nested dependencies (including the parent package) to share a package name. This is validated by all Cargo operations that would generate or read a lockfile.

## **`cargo package` &amp; `cargo publish`**

If `cargo package` or `cargo package` is directed to operate on a package whose manifest specifies `package.publish = "nested"`, that is an error. (Users wishing to make crates in a nested package public should either write `package.publish = true` or should re-export their contents from another package.)

When a valid parent package is packaged, each of its transitive nested dependencies must be included in the `.crate` archive file. This has two sub-cases:

* The nested package may be in a subdirectory of the parent package directory. In this case, it is copied to the same location in the archive.
* Otherwise, it is copied to `.cargo/packages/<package name>/` within the archive.

`Cargo.toml` files for nested packages are rewritten in the same way as is already done for all packages, except that `path` dependencies which are nested dependencies are kept rather than stripped out or rejected as they currently are. Their path values may need to be rewritten to point to the nested paackages' new location in the archive.

## **`crates.io`**

`crates.io` will allow uploading of packages that contain `path` dependencies that were previously prohibited, as long as:

* The dependency is a valid nested dependency as defined above. This includes that the the named package in fact exists in the `.crate` archive file, and has a valid `Cargo.toml` which declares `package.publish = "nested"`.
* The `path`s in all contained manifests must not contain any upward traversal outside of the parent package (`../../`) or other hazardous or non-portable components as determined to be necessary.

The package index, and the `crates.io` user interface, does not explicitly represent nested packages; instead, nested packages' dependencies are flattened into the dependencies of the parent package when they are added to the index. This adequately reflects what can be expected when using the parent package. (Note that required dependencies of optional nested packages should become optional in the flattened form.)

## `cargo build` and friends

The `Cargo.lock` format will need to be modified to handle entries for nested packages differently, as `path` dependencies are currently not allowed to introduce multiple packages with the same name, which could happen though different packages' nested packages. This modification could consist of omitting them entirely and using the same flattened dependency graph as the `crates.io` index will use (which follows the logic that they are not truly versioned separate from their parent), or giving them some namespacing scheme; I leave this choice up to the Cargo team's judgement.

Some new build message formatting would ideally be added; currently, `path` dependencies' full paths are always printed in progress messages, but they would be long noise here (`/home/alice/.cargo/registry/src/index.crates.io-6f17d22bba15001f/...`). Perhaps progress for sub-packages could look something like “`Compiling foo/macros v0.1.0`”, or “`Compiling foo v0.1.0 (crate macros)`”.

## Documentation of nested packages' crates

Nested packages' library crates should not be documented as full crates in `rustdoc` generated documentation, since they are an implementation detail of the parent package and their names are not unique among the set of all dependencies of the current build (e.g. many different packages could have nested libraries called just `macros`). I think `rustdoc` should use its [`doc(inline)`](https://doc.rust-lang.org/rustdoc/write-documentation/the-doc-attribute.html#inline-and-no_inline) support so that any re-exported items are documented in the parent package's crates' documentation. I believe this will require a new command-line option to `rustdoc` to tell it to treat a certain dependency crate as if it were a private module (which already triggers documentation inlining automatically).

However, if this is not (yet) done, the documentation will still be usable; just with more implementation details visible, and a chance of name collision (which is already possible).

## Workspaces

The presence or absence of a `[workspace]` has no effect on nested packages, just as it has no effect on existing package publication. Nested packages may use workspace inheritance when they are workspace members. Nested packages may be in different workspaces than their parent package.

# Drawbacks
[drawbacks]: #drawbacks

* This increases the number of differences between “Cargo package (on disk)” from “Cargo package (that may be published in a registry, or downloaded as a unit)” in a way which may be confusing; it would be good if we have different words for these two entities, but we don't.

* It is not possible to publish a bug fix to a nested package without republishing the entire parent package; this is the cost we pay for the benefit of not needing to take care with versioning for nested packages.

* Suppose `foo` has a nested package `foo-core`. Multiple major versions of `foo` cannot share the same instance of `foo-core` as they could if `foo-core` were separately published and the `foo`s depended on the same version of `foo-core`. Thus, choosing nested publishing may lead to type incompatibilities (and greater compile times) that would not occur if the same libraries had been separately published.
     * If this situation comes up, it can be recovered from by newly publishing `foo-core` separately (as would have been done if nested publishing were not used) and using the [semver trick](https://github.com/dtolnay/semver-trick) to maintain compatibility.

* Support for duplicative nested publishing (that is, nested packages that are nested within more than one parent package) has the following consequences:
    * May increase the amount of source code duplicated between different published packages, increasing download sizes and compilation time. It's currently possible to duplicate code into multiple packages via symlinks, but this would make it an “official feature”.
    * If packages A and B are separately published with nested package C, and A also depends on B, then A may see two copies of C's items, one direct and one transitive. This may cause a set of packages to fail to compile due to type/trait mismatches when published. [RFC 3516 public/private dependencies](https://rust-lang.github.io/rfcs/3516-public-private-dependencies.html) may be able to reduce problems of this type if we encourage, by documentation and lint, authors to think twice before allowing a multiply-used nested dependency to also be a RFC 3516 public dependency.

* Build and packaging systems that replace or wrap Cargo (e.g. mapping Cargo packages into Linux distribution packages) may have 1 library:1 package assumptions that are broken by this change.

* In the discussion of [RFC 2224], it came up that a feature like this could be used for vendoring libraries with patches not yet accepted upstream, where people sometimes currently resort to publishing forks to crates.io. Using nested packages for vendoring has the advantage of not cluttering crates.io, but also results in hidden code duplication. We may wish to decide whether to encourage or discourage this use of the feature.


# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The reason for doing anything at all in this area is that publishing multiple packages is often a bad solution to the problems that motivate it; in particular:

* Non-lockstep versioning risk: If you publish `foo 1.0.0` and `foo-macros 1.0.0`, then later publish `foo 1.1.0` and `foo-macros 1.1.0`, then it is _possible_ for users' `Cargo.lock`s to get into a state where they select `foo-macros 1.1.0` and `foo 1.0.0`, and this then breaks because `foo-macros` assumed that items from `foo 1.0.0` would be present. Arguably, this is a deficiency in the proc-macro system (`foo-macros` has a _de facto_ dependency on `foo` but does not declare it), but not one that is likely to be corrected any time soon. This can be worked around by having `foo` specify an exact dependency `foo-macros = "=1.0.0"`, but this is a subtlety that library authors do not automatically think of; semver is easy to get wrong silently.
* The crates.io registry may be cluttered with many packages that are not relevant to users browsing packages. (Of course, there are many other reasons why such clutter will be found.)
* When packages are implementation details, it makes a permanent mark on the `crates.io` registry even if the implementation of the parent package stops needing that particular subdivision. By allowing sub-packages we can allow package authors to create whatever sub-packages they imagine might be useful, and delete them in later versions with no consequences.
* It is possible to depend on a published package that is intended as an implementation detail. Ideally, library authors would document this clearly and library users would obey the documentation, but that doesn't always happen. By allowing nested packages, we introduce a simple “visibility” system that is useful in the same way that `pub` and `pub(crate)` are useful within Rust crates.

## Alternatives to nested packages

*   Support multiple library targets per package. That would be arguably cleaner, but has these disadvantages:

    * It would require new manifest syntax, not just for declaring the multiple libraries, but for referring to them, and for making per-target dependencies (e.g. only a proc-macro lib should depend on `proc-macro2`+`quote`+`syn`, not the rest of the libraries in the package).
    * It would require many new mechanisms in Cargo.
    * It might have unforeseen problems; by contrast, nested packages are compiled exactly the same way `path` dependencies currently are, and the only new element is the ability to publish them, so the risk of surprises is lower.

    Also, nested packages enables nesting *anything* that Cargo packages can express now and in the future; it is composable with other Cargo functionality.

*   [Extend the language and `rustc` to support “inline crates”][inline-crates]; support triggering the compilation of child crates declared within the source code of the parent. This would be extremely convenient for proc-macros or when simply wants to cause fully parallel and independently-cached compilation of subsets of their code. However:

    * It does not permit specifying different dependencies for each crate (includes false dependency edges).
    * The compilations cannot be started until the dependent crate has gotten past the macro expansion phase to discover crate declarations.
    * Does not naturally contain a way to define binary crates.
    * I expect it would also require significant changes to the Rust language and `rustc`, reaching beyond just “spawn another `rustc`” and including problems like computing the implied dependencies among inline crates that depend on other inline crates (unless inline crates are only allowed to depend on external dependencies and their own child inline crates, which is likely undesirable because it prohibits establishing common core vocabulary among a set of crates to be compiled in parallel).

*   Do nothing, except for warning the authors of paired macro crates that they should use exact version dependencies. The consequence of this will be continued hassle for developers; it might even be that useful proc-macro features might not be written simply because the author does not want to manage a second package.

## Details within this proposal

There are several ways we could mark packages for nested publishing, rather than using the `package.publish` and `dependencies.*.publish` keys:

* Instead of declaring each _dependency_ as being nested, we could only use `package.publish = "nested"` to make the determination. This would be problematic when a workspace has a root package, because that root package cannot avoid publishing all its `nested` workspace members except by writing `include`/`exclude` rules.

* Instead of introducing `package.publish = "nested"`, we could only require that dependencies be declared as nested. The disadvantages of this are:
    * May unintentionally duplicate published code between a standalone published package and a nested package
    * Does not make both ends of the relationship explicit to readers of the code.

* We could not permit nested packages that are not sub-packages (not in subdirectories of the parent package). This would avoid needing to define a place to copy the nested packages, but would make it impossible for two published packages to both nest the same package, which is useful for managing utility libraries too small and specific to be worth making public.

* Instead of declaring anything, we could simply allow sub-packages to be published when they would previously be errors. This would be problematic when an existing package has a dev-dependency on a sub-package; either that sub-package would suddenly start being published as nested, or there would be no way to specify the sub-package *should* be published.

* We could introduce an explicit `[subpackages]` table in the manifest, instead of `dependencies.*.publish`. This is just a syntactic distinction, but I think it would be more cumbersome to use; forgetting to remove an entry would result in publishing dead code, and forgetting to add one would not be detected until `cargo publish` time.
    * However, we might want to add something like this for [the future possibility of][future-possibilities] public targets in nested packages (particularly, installable binary targets in sub-packages, which will frequently depend on the parent and not vice versa).

* We could reuse `workspace.members` to also describe nested packages somehow; this constrains packages to be published to be structured similar to workspaces.


# Prior art
[prior-art]: #prior-art

*   Postponed [RFC 2224] (2017) is broadly similar to this RFC, and proposed using `publish = false` to mean what we mean by `publish = "nested"`.
    This RFC is more detailed and addresses the questions that were raised in discussion of 2224.

*   Blog post [Inline crates, by Yoshua Wuyts (2022)][inline-crates] proposes that additional library crates can be declared using Rust syntax in the manner `crate foo;` or `crate foo {}`, like modules.
    This is discussed above in the alternatives section.

*   Blog post [Rust 2030 Christmas list: Subcrate dependencies, by Olivier Faure (2023)](https://poignardazur.github.io/2023/01/24/subcrates/) proposes a mechanism of declaring nested dependencies similar to this RFC, but instead of embedding the files in one package, the “subcrates” are packaged separately on crates.io, but published as a single command and are not usable by other packages. Thus, it is similar to a combination of the also-desired features of “publish an entire workspace” and “namespacing on crates.io”, plus the subcrates being private on crates.io.

    This alternative implementation has some advantages such as the potential of publishing updates to subcrates alone. It requires complex features such as the addition of package namespacing to crates.io, but that feature is desired itself for other applications. I consider that a worthy alternative to this RFC, and would be happy to see either one implemented.

Among this space of possibilities, I see the place of this particular RFC as attempting to be a **minimal addition to Cargo**, building on the existing concept of `path` dependencies to add lots of power with little implementation cost; not necessarily to make sense from a blank slate.

I am not aware of other package systems that have a similar concept, but I am not broadly informed about package systems.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

* We could choose to explicitly prohibit nested packages from specifying a `package.version`, to avoid giving the misleading impression that it means anything. This would be notably stricter than the current meaning of absent `package.version` as of Cargo 1.75, which is that it is completely equivalent to `version = "0.0.0"`. It would also prohibit having a package that is both nested and published to a registry, if that is desired.

# Future possibilities
[future-possibilities]: #future-possibilities

## Nested packages with public binary targets

One common reason to publish multiple packages is in order to have a library and an accompanying tool binary, without causing the library to have all of the dependencies that the binary does. Examples: `wasm-bindgen` (`wasm-bindgen-cli`), `criterion` (`cargo-criterion`), `rerun` (`rerun-cli`).

This RFC currently does not address that — if nothing is done, then `cargo install` will ignore binaries in nested packages, and it is unlikely that those packages would actually be in the nested package dependency graph anyway. It would be easy to make a change which supports that; for example, `cargo install` could traverse nested packages and install all found binaries — but that would also install binaries which are intended as testing or (once [artifact dependencies] are implemented) code-generation helpers, which is undesirable. Thus, additional design work is needed to support `cargo install`ing from subpackages:

* There must be a way to, in the parent package manifest, declare nested packages to be published even though they are not dependencies of the parent package (but are likely *dependents* instead). This could also serve as the means to declare the binary target, or the nested package, “public”.
* Should individual targets be explicitly “re-exported” from the parent package?
* Should there be an additional option to `cargo install` which picks nested packages? (This would cancel out the benefit to the `cargo install` user from having a single package name.)

## Nested packages with public library targets

Allowing nested libraries to be named and used from outside the package would allow use cases which are currently handled by Cargo `features` and conditional compilation  (optional functionality with nontrivial costs in dependencies or compilation time)  to be instead handled by defining additional public libraries within one package.

This would allow library authors to avoid writing fragile and hard-to-test conditional compilation, and allow library users to avoid accidentally depending on a feature being enabled despite not having enabled it explicitly. It would also allow compiling the optional functionality and its dependencies with maximum parallelism, by not introducing a single `feature`-ful library crate which acts as a single node in the dependency graph.

However, it requires additional syntax and semantics, and these use cases might be better served by [#3243 packages as namespaces] or some other namespacing proposal, which would allow the libraries to be published independently. (I can also imagine a world in which both of these exist, and the library implementer can transparently use whichever publication strategy best serves their current needs.)

## Additional privileges between crates

Since nested packages are versioned as a unit, we could relax the trait coherence rules and allow implementations that would otherwise be prohibited.

This would be particularly useful when implementing traits from large optional libraries; for example, package `foo` with subpackages `foo_core` and `foo_tokio` could have `foo_tokio` write `impl tokio::io::AsyncRead for foo_core::DataSource`. This would improve the dependency graph compared to `foo_core` having a dependency on `tokio` (which is the only way to do this currently), though not have the maximum possible benefit unless we also added public library targets as above, since the package as a whole still only exports one library and thus one dependency graph node.

## Git dependencies

This RFC does not propose implementing a dependency declared as `{ git = "...", publish = "nested" }`. The obvious meaning is to copy the files from the target Git repository into the package, similarly to a Git submodule checkout. However, there might be things that need further consideration:

* Exactly what set of files should be copied?
* It would become possible to depend on (and thus copy during publication) a nested package someone else wrote for their own packages' use, which creates hazards for versioning and for non-compliance with source code licenses; while these are already possible, now Cargo would be doing it invisibly for you, which seems risky.

## Testing

A noteworthy benefit of nesting over separately-published packages is that the entire package can be verified to build outside its development repository/workspace by running `cargo publish --dry-run` or `cargo package`. It might be interesting to add a flag which does not just build the package, but also test it; while this is not at all related to nested packages *per se*, it might be a particular benefit to the kind of large project which currently uses multiple packages.

## License compatibility checking

The rule about nested packages' `package.license` could be made more lenient, only requiring the parent package's (not necessarily the dependent's) license expression to comply with the nested package's, in terms of the operators in the license expression. For example, if two nested packages contain licenses of `MIT` and `BSD-3-Clause`, then the parent package's expression could be `MIT AND BSD-3-Clause` or similar.

[artifact dependencies]: https://github.com/rust-lang/rfcs/pull/3028
[#3243 packages as namespaces]: https://github.com/rust-lang/rfcs/pull/3243
[RFC 2224]: https://github.com/rust-lang/rfcs/pull/2224
[inline-crates]: https://blog.yoshuawuyts.com/inline-crates/
