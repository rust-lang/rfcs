- Feature Name: (fill me in with a unique ident, `my_awesome_feature`)
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

The word _target_ in `cargo` has two different meanings which are both relevant to this RFC.
The following terms will be used:
- "cargo-target": The part of a package that is built (`[lib]`, `[[bin]]`, `[[example]]`, `[[test]]`, or `[[bench]]`).
- "target": The architecture/platform that `rustc` is compiling for.

# Summary

The addition of `supported-targets` to `Cargo.toml`.
This field is an array of `target-triple`/`cfg` specifications that restricts the set of targets which
a crate supports. Crates must meet the `supported-targets` of their
dependencies, and they can only be built for targets that satisfy their `supported-targets`.

# Motivation

Some crates do not support every possible `rustc` target. Currently, there is no way to formally
specify which targets a crate does, or does not support.

### Developer Experience

Trying to depend on a crate that does not support one's target often produces cryptic build errors,
or worse, fails at runtime. Being able to specify which targets are supported ensure that unsupported
targets cannot build the crate. This also makes build errors specific. This feature also enhances
developer experience when working in workspaces or with packages designed for different targets simultaneously.

### Cross Compilation 

Once it is known that a crate will only ever build for a subset of targets, it opens
the door for more advanced control over dependencies.
For example, transient dependencies under a `[target.**.dependencies]` table could be
excluded from `Cargo.lock` if the target restriction is not supported by the crate.
This is especially relevant to areas such as WebAssembly and embedded programming,
where one usually supports only a few specific targets. Currently, auditing and vendoring
is tedious because dependencies in `[target.**.dependencies]` tables always make their way
in the dependency tree, even though they may not actually be used.


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The `supported-targets` field can be added to `Cargo.toml` in any/all cargo-target
tables (`[lib]`, `[[bin]]`, `[[example]]`, `[[test]]`, and `[[bench]]`).

This field consists of an array of strings, where each string is an explicit target-triple, or a `cfg` specification
(as for the `[target.'cfg(**)']` table). The supported `cfg` syntax is the same as the one for
[platform-specific dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#platform-specific-dependencies)
(i.e., `cfg(test)`, `cfg(debug_assertions)`, and `cfg(proc_macro)` are not supported).

__For example:__
```toml
[package]
name = "hello_cargo"
version = "0.1.0"
edition = "2021"

[lib]
supported-targets = [
    'cfg(target_family = "unix")',
    "wasm32-unknown-unknown"
]
```
Here, only targets with `target_family = "unix"` __or__ the `wasm32-unknown-unknown` target are allowed
to build the library.

User experience is enhanced by providing a lint (deny by default) that fails compilation when
the supported targets of a cargo-target or one of its dependencies are not satisfied. If a crate
has `supported-targets` specified, then all of its dependencies' `supported-targets`
must be a superset of its own, for it to be publishable.

When `supported-targets` is not specified, any target is accepted as long as the target used satisfies
all dependencies' `supported-targets`.

This feature should not be eagerly used; most crates are not tailored for a specific subset of targets.
It should be used when crates clearly do not support all targets. For example: `io-uring`
requires `cfg(target_os = "linux")`, `gloo` requires `cfg(target_family = "wasm")`, and
`riscv` requires `cfg(target_arch = "riscv32")` or `cfg(target_arch = "riscv64")`.

This feature should also be used to increase cargo's knowledge of a cargo-target. For example,
when working in a package where some cargo-targets compile with `#[no_std]` and `target_os = "none"`, 
and some other cargo-targets are tools that require a desktop OS, using `supported-targets` makes
`cargo <command>` ignore cargo-targets which have `supported-targets` that are not
satisfied.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Compatibility with the selected target

When `cargo` is run on a cargo-target, it checks that the selected target satisfies the `supported-targets`
of the cargo-target, and of all dependencies. If it does not, a lint is raised and the build fails.

## Compatibility with dependencies

For each dependency `D` of a crate, the `supported-targets` of `D` must be a superset of
the `supported-targets` of the crate.

If the crate itself has no `supported-targets` specified, then all dependencies must support all targets.

This is enforced only upon publishing (and thus only for crates). Since `supported-targets` is only useful
for dependents of a library or binary, enforcing this at build time is not necessary and overly intrusive.

### Subset and superset relations
[subsets-and-supersets]: #subset-and-superset-relations

When checking if a crate's `supported-targets` are a subset of all dependencies' `supported-targets`,
the standard mathematical definition of subset is used. That is, `A ⊆ B` if and only if every element
of `A` is also an element of `B`.

- When comparing a `target-triple` 'A' against another `target-triple` 'B', A ⊆ B if
    A and B are the same.
- When comparing a `target-triple` against a `cfg(..)`, the `target-triple` is a subset
    of the `cfg(..)` if the `target-triple` satisfies the `cfg(..)`.
- When comparing a `cfg(..)` against a `target-triple`, the `cfg(..)` is never a subset of the `target-triple`.
- When comparing a `cfg(A)` against another `cfg(B)`, `cfg(A)` ⊆ `cfg(B)` if
    A and B are the same. (e.g., `target_abi = "eabi"` ⊆ `target_abi = "eabi"`)

To improve usability, two extra relations are defined:
- `cfg(target_os = "windows")` ⊆ `cfg(target_family = "windows")`.
- `cfg(target_os = <unix-os>)` ⊆ `cfg(target_family = "unix")`, where `<unix-os>` is any of
    `["freebsd", "linux", "netbsd", "redox", "illumos", "fuchsia", "emscripten", "android", "ios", "macos", "solaris"]`.
    This list needs to be updated if a new `unix` OS is supported by `rustc`'s official target list).
    
The contrapositive of these relations are also true.

The full list of configuration options is given [here](https://doc.rust-lang.org/reference/conditional-compilation.html).

### Flattening `not`, `any` and `all` in `cfg` specifications
[flattening-cfg]: #flattening-not-any-and-all-in-cfg-specifications

Since `cfg` specifications can contain `not`, `any`, and `all` operators, these must be handled.
This is done by flattening the `cfg` specification to a specific form.

The `not` operator is "passed through" `any` and `all` operators using De Morgan's laws, until it
reaches a single `cfg` specification. For example, `cfg(not(all(target_os = "linux", target_arch = "x86_64")))`
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

The result of these transformations is always a list of `cfg` specifications that 
either contains a single specification, or an `all` operator with no nested operators.

This structure can then be used to evaluate subset relations.

## Behavior with unknown entries (and custom targets)

Just as `[target.my-custom-target.dependencies]` is allowed by `cargo`, `supported-targets` can contain
unknown entries. This is important because users may have different `rustc` versions, and the set of
official `rustc` targets is unstable; Targets can change name or be removed. Also, developers 
may want to support their custom target.

To determine if an entry in `supported-targets` is a target name or a `cfg` specification, the
same mechanism as for `[target.'cfg(..)']` is used (using
[`cargo-platform`](https://docs.rs/cargo-platform/latest/cargo_platform/index.html)).

## Ignoring builds for unsupported targets

When the target used is not supported by the cargo-target being built, the cargo-target will either be
skipped or a lint will be raised, depending on how `cargo` was invoked.
`cargo` behaves identically to when the `required-features` of the cargo-target are not satisfied.

__Note:__ `required-features` is not applicable to `libraries`, while `supported-targets` is. Nevertheless, libraries
behave the same way as other cargo-targets.

## Detecting unused dependencies

An MVP implementation of this RFC does not _require_ this feature, but it is a natural extension, and a
major part of the motivation. Hence, it is informally described here.

Dependencies may have `[target.'cfg(..)'.dependencies]` tables, which may never be used because of a
`supported-targets` restriction.

When resolving dependencies for a package, each cargo-target can have a list of "unused dependencies"
if the `supported-targets` restriction is mutually exclusive with dependencies behind a `[target.'cfg(..)'.**]`
table.

For each set of dependencies (normal, build, and dev), the intersection of these unused dependencies
can be purged from the dependency tree.

This could be implemented either as a separate pass on the `Resolve` graph, or as part of dependency resolution.
If this is implemented as part of dependency resolution, it may or may not be favorable for
`supported-targets` to influence the version resolution of dependencies.

When detecting unused dependencies for a cargo-target, _all_ targets specified in `supported-targets` must
be mutually exclusive with the target from a `[target.'cfg(..)'.dependencies]` table. If an entry in
`supported-targets` is not mutually exclusive with the target from a `[target.'cfg(..)'.dependencies]`
table, then the dependency cannot be considered unused.

### Mutually exclusive `cfg` settings
[mutually-exclusive-cfg]: #mutually-exclusive-cfg-settings

Some `cfg` settings have mutually exclusive elements, while some do not. What is meant here is, for example,
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

Special case:
- `target_family = "windows` and `target_family = "unix"` are mutually exclusive, while all other `target_family`
    value pairs are not mutually exclusive.

`cfg(not(<option>))` is also mutually exclusive with `cfg(<option>)`.

`supported-targets` are [flattened][flattening-cfg] before checking for mutual exclusivity with `[target.**.dependencies]`
tables.

# Drawbacks
[drawbacks]: #drawbacks

- Performance: this is a lot of extra calls to rustc.
    Hopefully these are all compatible with the rustc cache, so they won't make things too bad.
- This feature must be learned by users wanting to publish packages that depend on crates having `supported-targets`
    specified.
- The common case of a crate not supporting `no_std` is not elegantly handled by this feature.
    The closest thing available is `supported-targets = ['cfg(not(target_os = "none"))']`.
- In its complete form, this feature increases the complexity of dependency resolution.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Naming

A few other names for this field can be considered:

- `required-targets`. Although it matches with `required-features`, `required-features` is a list of features
    that must _all_ be enabled (conjunction), whereas `supported-targets` is a list of targets
    where _any_ is allowed (disjunction).
- `target-requirements`. Feels indirect, also implies a conjunction of requirements rather than a disjunction.
- `targets`. As in "this package _targets_ ...". Ambiguous, and could be confused with the `target` table.

## Not using `cfg`

Using `cfg` complicates the implementation, and may require a substantial amount of calls to `rustc` to check
target-`cfg` compatibility. Some alternatives are discussed here along with their drawbacks.

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

## Without `cfg` relations

The set relations defined on `cfg` for the [validation of the dependency tree][subsets-and-supersets]
and for [detecting unused dependencies][mutually-exclusive-cfg] can seem artificial or hard coded.
These are a bi-product of the "state of the world." It _currently_ does not make sense to have a target-triple
with `target_os = "windows"` and `target_family = "unix"`, hence why the relation is defined.

These could be removed at a cost to user experience. For example, consider the following situation:

Suppose crate `foo` has `supported-targets = ['cfg(target_os = "linux")']` for its library,
and crate `bar` has `supported-targets = ['cfg(target_family = "unix")']` in its library.
`bar` also has some dependency `baz` through `[target.'cfg(target_os = "macos")'.dependencies]`.

If `foo` depends on `bar`, and the previously mentioned relations are removed, then `foo` would
need to have `supported-targets = ['cfg(all(target_family = "unix", target_os = "linux"))']` to be publishable.
This is not as user-friendly, but is a possible alternative.

Similarly, if `cfg` set relations are not implemented and `foo` depends on `bar`, then `baz` would
be included in the dependency tree of `foo`, even though it is never used. To solve this,
bar would need to have `supported-targets = ['cfg(all(target_family = "unix", target_os = "linux", not(target_os = "macos)))']`
for `baz` to not be included in the dependency tree. This is also not user-friendly, but again, simpler
to implement and maintain.

The problem can get substantial if many target specific dependencies are involved.

# Prior art
[prior-art]: #prior-art

Does anyone know how other build tools solve this?

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- What if one wants to exclude a single specific target? Groups can be excluded with `cfg(not(..))`, but there
	is currently no way of excluding specific targets.
- How do we make users bypass the lint (probably with some `--ignore-**` flag)?
- Should we solve for this during dependency version resolution? (the current rationale is that we do not want
	targets to affect package version resolution). In the future, this could be implemented in
    `pubgrub` using [constraints](https://github.com/pubgrub-rs/pubgrub/issues/120).

# Future possibilities
[future-possibilities]: #future-possibilities

- Have different entry points for different targets (see [#9208](https://github.com/rust-lang/cargo/issues/9208)).
- Make this process part of `pubgrub`, or whatever resolver `cargo` will be using.
- Show which targets are supported on `docs.rs`.
- Have search filters on `crates.io` for crates with specific targets.
