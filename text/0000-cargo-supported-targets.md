- Feature Name: (fill me in with a unique ident, `my_awesome_feature`)
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

The word _target_ is extensively used in this document.
The [glossary](https://doc.rust-lang.org/cargo/appendix/glossary.html#target) defines its many meanings.
Here, _target_ refers to the "Target Architeture" for which a package is built. Otherwise, the terms
"cargo-target" and "target-triple" are used in accordance with their definitions in the glossary.

# Summary

The addition of `supported-targets` to `Cargo.toml`.
This field is an array of `target-triple`/`cfg` specifications that restricts the set of targets which
a package supports. Packages must meet the `supported-targets` of their
dependencies, and they can only be built for targets that satisfy their `supported-targets`.

# Motivation

Some packages do not support every possible `rustc` target. Currently, there is no way to formally
specify which targets a package does, or does not support.

### Developer Experience

Trying to depend on a crate that does not support one's target often produces cryptic build errors,
or worse, fails at runtime. Being able to specify which targets are supported ensures that unsupported
targets cannot build the crate, and also makes build errors specific.

This feature also enhances developer experience when working in workspaces containing packages designed for
many different targets. Commands run on a workspace ignore packages that don't support the selected target.

### Cross Compilation 

Once it is known that a package will only ever build for a subset of targets, it opens
the door for more advanced control over dependencies.
For example, transient dependencies declared under a `[target.**.dependencies]` table are
excluded from `Cargo.lock` if the dependent's `supported-targets` is mutually exclusive with
the target preconditions under which the dependencies are included.
This is especially relevant to areas such as WebAssembly and embedded programming,
where one usually supports only a few specific targets. Currently, auditing and vendoring
is tedious because dependencies under `[target.**.dependencies]` tables always make their way
in the dependency tree, even though they may not actually be used.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The `supported-targets` field can be added to `Cargo.toml` under the `[package]` table.

This field consists of an array of strings, where each string is an explicit target-triple or a `cfg` specification
(as for the `[target.'cfg(**)']` table). The supported `cfg` syntax is the same as the one for
[platform-specific dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#platform-specific-dependencies)
(i.e., `cfg(test)`, `cfg(debug_assertions)`, and `cfg(proc_macro)` are not supported).
If a selected target satisfies any entry of the `supported-targets` list, then the package can be built for that target.

__For example:__
```toml
[package]
name = "hello_cargo"
version = "0.1.0"
edition = "2021"
supported-targets = [
    "wasm32-unknown-unknown",
    'cfg(target_os = "linux")',
    'cfg(target_os = "macos")'
]
```
Here, only targets satisfying: the `wasm32-unknown-unknown` target, __or__ the `linux` OS, __or__ the `macos` OS,
are allowed to build the package. If no `supported-targets` was specified, then any target would be allowed.

User experience is enhanced by raising an error that fails compilation when the supported targets
of a package are not satisfied by the selected target. A package's `supported-targets` must be a subset
of its dependencies' `supported-targets`, otherwise the build also fails.

When `supported-targets` is not specified, any target is accepted, so all dependencies must support
all targets.

This feature should be used when a package clearly does not support all targets. For example: `io-uring`
requires `cfg(target_os = "linux")`, `gloo` requires `cfg(target_family = "wasm")`, and
`riscv` requires `cfg(target_arch = "riscv32")` or `cfg(target_arch = "riscv64")`.

This feature should also be used to increase cargo's knowledge of a package. For example,
when working in a workspace where some packages compile with `#[no_std]` and `target_os = "none"`, 
and some others are tools that require a desktop OS, using `supported-targets` makes
`cargo <command>` ignore packages which have `supported-targets` that are not satisfied
by the selected target.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

When `cargo` is run on a package, it checks that the selected target satisfies the `supported-targets`
of the package. If it does not, an error is raised and the build fails.

## Compatibility of `[dependencies]`

The set of `supported-targets` of a package must be a subset of the `supported-targets` of its
`[dependencies]`. If the crate itself has no `supported-targets` specified,
then all dependencies must support all targets.

If a dependency does not respect this requirement (if it is not compatible), an error is raised and the build fails.

If this was not enforced, a package could support targets that are not supported by its dependencies,
which does not make sense.

## Compatibility of `[dev-dependencies]`

`[dev-dependencies]` are checked the using the same method as regular `[dependencies]`. That is, the package's
`supported-targets` must be a subset of every `[dev-dependencies]`'s `supported-targets`. The rationale is
that an example, test, or benchmark has access to the package's library and binaries, and so it must respect the
`supported-targets` of the package.

## Compatibility of `[build-dependencies]`

What makes `[build-dependencies]` unique is that they are built for the host computer,
and not the selected target. As such, they are not restrained by the `supported-targets`
of the package. Hence, all dependencies are allowed in the `[build-dependencies]` table.
However, a build error is raised if one of the build dependencies does not support
the _host_'s target-triple at build time.

In the future, having all build dependencies support all targets could be enforced to ensure
that a crate can be built on any host. This is left as a [future possibility](#restrict-build-dependencies).

## Platform-specific dependencies
[platform-specific-dependencies]: #platform-specific-dependencies

Platform-specific dependencies are dependencies under the `[target.**]` table. This includes
normal dependencies, build-dependencies, and dev-dependencies.

When platform-specific dependencies are declared, the conditions under which they are
declared must be a subset of each dependency's `supported-targets`.
For example, a dependency declared under `[target.'cfg(target_os = "linux")'.dependencies]`
must at least support the `linux` OS.

For regular dependencies and dev-dependencies, it suffices for a platform-specific dependency to support the
_intersection_ of the package's supported-targets, and the target conditions it is declared under.
For example:
```toml
[package]
# ...
supported-targets = ['cfg(target_os = "linux")']

[target.'cfg(target_pointer_width = "64")'.dependencies]
foo = "0.1.0"
```
Here, it suffices for `foo` to support `cfg(all(target_os = "linux", target_pointer_width = "64"))`.

## Artifact dependencies

If an artifact dependency has a `target` field, then the dependency is not checked against the
package's `supported-targets`. However, the selected `target` for the dependency must be compatible with
the dependency's `supported-targets`, or else an error is raised. If the artifact dependency does not have a
`target` field, then it is checked against the package's `supported-targets`, like any other dependency.

## Comparing `supported-targets`

When comparing two `supported-targets` lists, it is necessary to know if one is a _subset_ of the other,
or if both are _mutually exclusive_. To proceed, both lists are flattened to
the same representation, and they are then compared. This process is done internally, and does not
affect the `Cargo.toml` file.

### Flattening `not`, `any`, and `all` in `cfg` specifications
[flattening-cfg]: #flattening-not-any-and-all-in-cfg-specifications

Since `cfg` specifications can contain `not`, `any`, and `all` operators, these must be handled.
This is done by flattening the `cfg` specification to a specific form.

The `not` operator is "passed through" `any` and `all` operators using
[De Morgan's laws](https://en.wikipedia.org/wiki/De_Morgan%27s_laws), until it reaches a single `cfg`
specification. For example, `cfg(not(all(target_os = "linux", target_arch = "x86_64")))`
is equivalent to `cfg(any(not(target_os = "linux"), not(target_arch = "x86_64")))`.

Top level `any` operators are separated into multiple `cfg` specifications. For example,
```toml
supported-targets = ['cfg(any(target_os = "linux", target_os = "macos"))']
```
is transformed into
```toml
supported-targets = ['cfg(target_os = "linux")', 'cfg(target_os = "macos")']
```

Top level `all` operators are kept as is, as long as they do not contain nested `any`s or `all`s.
If there is an `any` inside an `all`, the statement is split into multiple `all` statements.
For example,
```toml
supported-targets = ['cfg(all(target_os = "linux", any(target_arch = "x86_64", target_arch = "arm"))']
```
is transformed into
```toml
supported-targets = [
    'cfg(all(target_os = "linux", target_arch = "x86_64"))',
    'cfg(all(target_os = "linux", target_arch = "arm"))'
]
```
If an `all` contains an `all`, the inner `all` is flattened into the outer `all`.

The result of these transformations on a `cfg` specification is a list of `cfg` specifications that 
either contains a single specification, or an `all` operator with no nested operators.

This procedure is run on all `cfg` elements of a `supported-targets` list. The resulting list
can then be used to evaluate relations. In the end, the flattened representation can only contain
explicit target-triples, `cfg(A)` containing a single `A`, or `cfg(all(A, B, ...))` with all
`A, B, ...` being single elements.

### The subset relation

To determine if a `supported-targets` list "A" is a subset of another such list "B", the standard mathematical
definition of subset is used. That is, "A" is a subset of "B" if and only if each element of "A" is
contained in "B".

So each element of "A" is compared against each element of "B" using the following rules:
- A `target-triple` is a subset of another `target-triple` if they are the same.
- A `target-triple` is a subset of a `cfg(..)` if the `cfg(..)` is satisfied by the `target-triple`.
- A `cfg(..)` is not a subset of a `target-triple`.
- A `cfg(all(A, B, ...))` is a subset of a `cfg(all(C, D ...))`,
    if the list `C, D, ...` is a subset of the list `A, B, ...`.

_Note_: All possible cases have been covered, since `cfg(A) == cfg(all(A))`.

More rules could be defined, but these are left as a [future possibility](#more-cfg-relations).

### Mutual exclusivity

For a `supported-targets` list "A" to be mutually exclusive with another such list "B", each element of "A" must
be mutually exclusive with _all_ elements of "B" (The inverse is also true).

So each element of "A" is compared against each element of "B" using the following rules:
- A `target-triple` is mutually exclusive with another `target-triple` if they are different.
- A `target-triple` is mutually exclusive with a `cfg(..)` if the `cfg(..)` is not satisfied by
    the `target-triple`.
- A `cfg(all(A, B, ...))` is mutually exclusive with a `cfg(all(C, D, ...))` if any element of the list
    `A, B, ...` is mutually exclusive with any element of the list `C, D, ...`.

_Note_: All possible cases have been covered, since `cfg(A) == cfg(all(A))`.

Two `cfg` singletons are mutually exclusive under the following rules:
- `cfg(A)` is mutually exclusive with `cfg(not(A))`.
- `cfg(<option> = "A")` is mutually exclusive with `cfg(<option> = "B")` if `A` and `B` are different,
    and `<option>` has mutually exclusive elements.

Some `cfg` options have mutually exclusive elements, while some do not. What is meant here is, for example,
`target_arch = "x86_64"` and `target_arch = "arm"` are mutually exclusive (a target-triple cannot have both),
while `target_feature = "avx"` and `target_feature = "rdrand"` are not.

`cfg` options that have mutually exclusive elements:
- `target_arch`
- `target_os`
- `target_env`
- `target_abi`
- `target_endian`
- `target_pointer_width`
- `target_vendor`

Those that do not:
- `target_feature`
- `target_has_atomic`
- `target_family`

`target_family = "windows` and `target_family = "unix"` could also be defined as mutually exclusive to
enhance usability, but this is left as a [future possibility](#more-cfg-relations).

## Behavior with unknown entries (and custom targets)

Just as `[target.my-custom-target.dependencies]` is allowed by `cargo`, `supported-targets` can contain
unknown entries. This is important because users may have different `rustc` versions, and the set of
official `rustc` targets is unstable; Targets can change name or be removed. Also, developers 
may want to support their custom target.

To determine if an entry in `supported-targets` is a target name or a `cfg` specification, the
same mechanism as for `[target.'cfg(..)']` is used (using
[`cargo-platform`](https://docs.rs/cargo-platform/latest/cargo_platform/index.html)).

## Ignoring builds for unsupported targets

When the target used is not supported by the package being built, the package will either be
skipped or an error will be raised, depending on how `cargo` was invoked. If cargo is invoked
in a workspace or virtual workspace without specifying a specific package, then `cargo` skips the package.
If a specific package is specified using `--package`, or if `cargo` is invoked on a single package,
then an error is raised.

## Eliminating unused dependencies from `Cargo.lock`

A package's dependencies may themselves have `[target.'cfg(..)'.dependencies]` tables, which may never be used because of the
`supported-targets` restrictions of the package. These can safely be eliminated from the dependency tree of the package.

Consider the following example:
```toml
[package]
name = "foo"
# ...
supported-targets = ['cfg(target_os = "linux")']

[dependencies]
bar = "0.1.0"
```
```toml
[package]
name = "bar"

[target.'cfg(target_os = "macos")'.dependencies]
baz = "0.1.0"
```
Currently, `baz` is included in the dependency tree of `foo`, even though `foo` is never built for `macos`.
With the addition of `supported-targets`, `baz` can be purged from the dependency tree of `foo`, since
`target_os = "macos"` is mutually exclusive with `target_os = "linux"`.

Formally, dependencies (and transitive dependencies) under `[target.**.dependencies]` tables are
eliminated from the dependency tree of a package if the `supported-targets` of the package is mutually exclusive
with the target preconditions of the dependency.

# Drawbacks
[drawbacks]: #drawbacks

- The `cfg` syntax is very expressive, but also very complex. As outlined above,
    detecting subset and mutual exclusivity relations is not trivial.
- Comparing `supported-targets` lists is an `O(n * m)` operation, with `n` and `m` being the number of elements
    in the lists. (I doubt this will be a problem, as `n` and `m` are expected to be small).
- Performance: this is a lot of extra calls to rustc.
    Hopefully these are all compatible with the rustc cache, so they won't make things too bad.
- This feature must be learned by those wanting to use dependencies with `supported-targets` specified.
- This is the first step towards a target aware `cargo`, which may increase `cargo`'s complexity, and bring
    more feature requests along these lines.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Format

The list of strings format was chosen because of its simplicity and expressiveness. Other formats can also be considered:

- Using the `[target]` table, for example:
    ```toml
    [target.'cfg(target_os = "linux")']
    supported = true
    ```
    If the list of supported targets is long (should it ever be?), then the `Cargo.toml` file becomes very verbose
    as well.
- A `[suppported]` table, with `arch = ["<arch>", ...]`, `os = ["<os>", ...]`, `target = ["<target>", ...]`, etc.
    This is more verbose, complex to implement, learn, and remember. It is also not obvious how `not` and `all`
    could be represented in this format. For example:
    ```toml
    [supported]
    os = ["linux", "macos"]
    arch = ["x86_64"]
    ```

## Naming

Some other names for this field can be considered:

- `required-targets`. Pro: it matches with the naming of `required-features`. Con: `required-features` is a list of features
    that must _all_ be enabled (conjunction), whereas `supported-targets` is a list of targets
    where _any_ is allowed (disjunction).
- `targets`. As in "this package _targets_ ...". Pro: Concise. Con: Ambiguous, and could be confused with the `target` table.

## Package scope vs. cargo-target scope

The `supported-targets` field is placed at the package level, and not at the cargo-target
level (i.e., under, `[lib]`, `[[bin]]`, etc.). A rationale is given for why the cargo-target level is not used.

Dependencies are resolved at the package level, so even if one would have cargo-targets with different
`supported-targets`, the dependencies would still be available to all cargo-targets. So, either
cargo-targets would have access to dependencies that they cannot use, or all dependencies would need
to support the union of all `supported-targets` of all cargo-targets.

Examples, tests and benchmarks also have access to the package's library and binaries, so they must have the
same set of `supported-targets`.

It is possible to allow cargo-targets to further restrict the `supported-targets` of the package,
but this is left as a [future possibility](#future-possibilities).

See also: [using a package vs. using a workspace](package-vs-workspace).

[package-vs-workspace]: #https://blog.rust-lang.org/inside-rust/2024/02/13/this-development-cycle-in-cargo-1-77.html#when-to-use-packages-or-workspaces

## Not using `cfg`

Using `cfg` complicates the implementation (and thus maintenance), and may require a substantial
amount of calls to `rustc` to check target-`cfg` compatibility. Some alternatives are discussed
here along with their drawbacks.

### Using wildcards

Instead of using `cfg` specifications, one could use wildcards (e.g., `x86_64-*-linux-*`).
This is much simpler to implement, target-triples are syntactically checked for a match instead
of solving set relations for `cfg`. However, this is not as expressive as `cfg`, and does not correctly
represent the semantics of target triples. For example, supporting `target_family = "unix"` would
require an annoyingly long list of wildcard patterns. Things like `target_pointer_width = "32"` are
even harder to represent, and things like `target_feature = "avx"` are basically not representable.
Also, this is new syntax not currently used by cargo.

### Allowing only target triples

This is an even stricter version of the above. Being even simpler to implement, this alternative
may not be expressive enough for the common use case. Packages rarely support specific target triples,
rather they support/require specific target attributes. What would likely happen is that packages
would copy and paste the target triple list matching their requirements from somewhere or someone else.
Every time a new target with the same attribute is added, the whole ecosystem would have to be updated.

# Prior art
[prior-art]: #prior-art

Previously, crates have mainly used their documentation to specify which targets they support, or they would
leave it up to the user to infer it. Some crates also made use of compile time errors to ensure that
`cfg` requirements are met, for example:
```rust
#[cfg(not(any(…)))]
compile_error!("unsupported target cfg");
```

Locally, users can already specify which targets they want to build for by default using the `target` field in
`cargo`'s `config.toml` file. This setting is only a local configuration however, and does not affect
packages published on `crates.io`. On nightly, the `per-package-target` feature defines the `default-target` field
of the `[package]` table in `Cargo.toml` to specify the default target for a package. Both of these options
can be overwritten by the `--target` flag, and only work for a single target-triple.

The `per-package-target` feature also defines the `force-target` field, which is supposed to force the package
to build for the specific target-triple. This does not interact well when used in dependencies, as one would expect a dependency
to be built for the same target as the package. `supported-targets` supersedes `force-target` because instead of enforcing
a single target, it enforces a set of targets.

I am not aware of a package manager that solves this issue like in this RFC. In other system level languages,
vendoring dependencies is a common practice, and the user would be responsible for ensuring that the dependencies
are compatible with the target. For interpreted languages, this is a non-issue because any platform being able
to run the interpreter can run the package.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- What if one wants to exclude a single target-triple? Groups can be excluded with `cfg(not(..))`, but there
	is currently no way of excluding specific targets (Would anyone ever require this?).
- Some crates will inevitably have target requirements that are too strict, how
    do we make users bypass the error (probably with some `--ignore-**` flag)? Do we want to allow this?
- Should we solve for this during dependency version resolution? (the current rationale is that we do not want
	targets to affect package version resolution).

# Future possibilities
[future-possibilities]: #future-possibilities

## Restrict build dependencies
[restrict-build-dependencies]: #restrict-build-dependencies

Currently, a build script can have any dependency. A problem can arise if a crate's build script depends on
a package that does not support `target_os = "windows"` for example. With this RFC, it would be
possible to only allow dependencies supporting all targets in build scripts.

## Lint against useless target-specific tables

If a package has:
```toml
[package]
name = "example"
# ...
supported-targets = ['cfg(target_os = "linux")']

[target.'cfg(target_os = "windows")'.dependencies]
# ...
```
A lint could be added to highlight the fact that the `[target]` table is useless.

## More `cfg` relations
[more-cfg-relations]: #more-cfg-relations

Consider the following scenario:
```toml
[package]
name = "bar"
supported-targets = ['cfg(target_family = "unix")']
# ...
```
```toml
[package]
name = "foo"
supported-targets = ['cfg(target_os = "macos")']

[dependencies]
bar = "0.1.0"
```
With the RFC implemented, a build error would be raised when compiling `foo`, because `cargo`
does not understand that `target_os = "macos"` is a subset of `target_family = "unix"`.

To go around this issue, `foo` would need to use:
```toml
[package]
name = "foo"
supported-targets = ['cfg(all(target_family = "unix", target_os = "macos"))']

[dependencies]
bar = "0.1.0"
```

To improve usability, two extra relations can be defined:
- `cfg(target_os = "windows")` ⊆ `cfg(target_family = "windows")`.
- `cfg(target_os = <unix-os>)` ⊆ `cfg(target_family = "unix")`, where `<unix-os>` is any of
    `["freebsd", "linux", "netbsd", "redox", "illumos", "fuchsia", "emscripten", "android", "ios", "macos", "solaris"]`.
    This list needs to be updated if a new `unix` OS is supported by `rustc`'s official target list.
This would make the first example compile.

_Note:_ The contrapositive of these relations is also true.

Also, `target_family` is currently defined as not having mutually exclusive elements. This is because `target_family = "wasm"`
is not mutually exclusive with other target families. But, `target_family = "unix"` could be defined as mutually exclusive
with `target_family = "windows"` to increase usability. By extension, `target_family = "windows"` would now be mutually exclusive
with `target_os = "linux"`, for example.

_Note:_ More relations could be defined, for example `target_feature = "neon"` ⊆ `target_arch = "arm"`. With this however,
things start to get complicated.

## Misc

- Have `cargo add` check the `supported-targets` before adding a dependency.
- Make this process part of the resolver.
- Show which targets are supported on `docs.rs`.
- Have search filters on `crates.io` for crates with support for specific targets.
- Also add this field at the cargo-target level.
