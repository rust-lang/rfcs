- Feature Name: `supported-targets`
- Start Date: 2025-01-08
- Pre-RFC: [Rust
  internals](https://internals.rust-lang.org/t/pre-rfc-allow-packages-to-specify-a-set-of-supported-targets/21979)
- RFC PR: [rust-lang/rfcs#3759](https://github.com/rust-lang/rfcs/pull/3759)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

The word _target_ is extensively used in this document. The
[glossary](https://doc.rust-lang.org/cargo/appendix/glossary.html#target) defines its many meanings.
Here, _target_ refers to the "Target Architecture" for which a package is built. Otherwise, the
terms "cargo-target" and "target-triple" are used in accordance with their definitions in the
glossary.

# Summary

The addition of `supported-targets` to `Cargo.toml`. This field is an array of `target-triple`/`cfg`
specifications that restricts the set of targets which a package supports. Packages can only be
built for targets that satisfy their `supported-targets`.

# Motivation

Some packages do not support every possible `rustc` target. Currently, there is no way to formally
specify which targets a package does, or does not support.

This feature enhances developer experience when working in workspaces containing packages designed
for many different targets. Commands run on a workspace ignore packages that don't support the
    selected target.

## Long-term Motivations

_These do not motivate the RFC itself, rather they motivate the [future
possibilities](#future-possibilities) section._

### Developer Experience

Trying to depend on a crate that does not support one's target often produces cryptic build errors,
or worse, fails at runtime. Being able to specify which targets are supported ensures that
unsupported targets cannot build the crate, and also makes build errors specific.

### Dependency Management 

Once it is known that a package will only ever build for a subset of targets, it opens the door for
more advanced control over dependencies. For example, transitive dependencies declared under a
`[target.**.dependencies]` table could be excluded from `Cargo.lock` if the dependent's
`supported-targets` are mutually exclusive with the target preconditions under which the
dependencies are included. This is especially relevant to areas such as WebAssembly and embedded
programming, where one usually supports only a few specific targets. Currently, auditing and
vendoring is tedious because dependencies under `[target.**.dependencies]` tables always make their
way in the dependency tree, even though they may not actually be used.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The `supported-targets` field can be added to `Cargo.toml` under the `[package]` table.

This field consists of an array of strings, where each string is an explicit target-triple or a
`cfg` specification (as for the `[target.'cfg(**)']` table). The supported `cfg` syntax is the same
as the one for [platform-specific
dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#platform-specific-dependencies)
(i.e., `cfg(test)`, `cfg(debug_assertions)`, and `cfg(proc_macro)` are not supported). If a selected
target satisfies any entry of the `supported-targets` list, then the package can be built for that
target.

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
Here, only targets satisfying: the `wasm32-unknown-unknown` target, __or__ the `linux` OS, __or__
the `macos` OS, are allowed to build the package. User experience is enhanced by raising an error
that fails compilation when the supported targets of a package are not satisfied by the selected
target.

This feature should be used when a package clearly does not support all targets. For example:
`io-uring` requires `cfg(target_os = "linux")`, `gloo` requires `cfg(target_family = "wasm")`, and
`riscv` requires `cfg(any(target_arch = "riscv32", target_arch = "riscv64"))`.

This feature increases cargo's knowledge of a package. For example, when working in a workspace
where some packages are for a platform with `target_os = "none"`, and some others are tools that
require a desktop OS, using `supported-targets` makes `cargo <command>` ignore packages which have
`supported-targets` that are not satisfied by the selected target.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

When a `cargo` build command (e.g. `check`, `build`, `run`, `clippy`) is run, it checks that the
selected target satisfies the `supported-targets` of the package being built. If it does not, the
package is skipped or an error is raised, depending on how [`cargo` was invoked](ignoring-builds).
However, `supported-targets` are _not_ checked if the cargo command does not require compilation
(e.g., `cargo fmt`).

## Behavior with unknown entries (and custom targets)

Just as `[target.my-custom-target.dependencies]` is allowed by `cargo`, `supported-targets` can
contain unknown entries. This is important because users may have different `rustc` versions, and
the set of official `rustc` targets is unstable; Targets can change name or be removed. Also,
developers may want to support their custom target.

To determine if an entry in `supported-targets` is a target name or a `cfg` specification, the same
mechanism as for `[target.'cfg(..)']` is used (using
[`cargo-platform`](https://docs.rs/cargo-platform/latest/cargo_platform/index.html)).

## Ignoring builds for unsupported targets
[ignoring-builds]: #igonring-builds-for-unsupported-targets

If cargo is invoked in a workspace or virtual workspace without specifying a package as
build-target, then `cargo` skips any package that does not support the selected target. If a package
is specified using `--package` or if `cargo` is invoked on a single package, and the selected target
does not satisfy the `supported-targets` of the package, then an error is raised.

# Drawbacks
[drawbacks]: #drawbacks

- Once implemented this does not yet solve the problem of dependency pruning in `Cargo.lock`, which
  is a common use case for this feature.
- This is the first step towards a target aware `cargo`, which may increase `cargo`'s complexity,
  and bring more feature requests along these lines.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Format

The list of strings format was chosen because of its simplicity and expressiveness. It can be
unintuitive to understand however, as the list implies a union of all its elements, which is not
immediately obvious.

Other formats can be considered:

Using a single `cfg` string, without explicit target-triples:
```toml
supported-targets = 'cfg(any(target_family = "unix", target_family = "wasm"))'
```

Using the `[target]` table, for example:
```toml
[target.'cfg(target_os = "linux")']
supported = true
```
If the list of supported targets is long (should it ever be?), then the `Cargo.toml` file becomes
very verbose as well.

A `[suppported]` table, with `arch = ["<arch>", ...]`, `os = ["<os>", ...]`, `target = ["<target>",
...]`, etc. This is more verbose, complex to implement, learn, and remember. It is also not obvious
how `not` and `all` could be represented in this format. For example:
```toml
[supported]
os = ["linux", "macos"]
arch = ["x86_64"]
```

## Naming

Some other names for this field can be considered:

- `required-targets`. Pro: it matches with the naming of `required-features`. Con:
  `required-features` is a list of features that must _all_ be enabled (conjunction), whereas
  `supported-targets` is a list of targets where _any_ is allowed (disjunction).
- `targets`. As in "this package _targets_ ...". Pro: Concise. Con: Ambiguous, and could be confused
  with the `target` table.

## Package scope vs. cargo-target scope

The `supported-targets` field is placed at the package level, and not at the cargo-target level
(i.e., under, `[lib]`, `[[bin]]`, etc.)

Dependencies are resolved at the package level, so even if one would have cargo-targets with
different `supported-targets`, the dependencies would still be available to all cargo-targets. So,
either cargo-targets would have access to dependencies that they cannot use, or all dependencies
would need to support the union of all `supported-targets` of all cargo-targets.

Examples, tests and benchmarks also have access to the package's library and binaries, so they must
have the same set of `supported-targets`.

It is possible to allow cargo-targets to further restrict the `supported-targets` of the package,
but this is left as a [future possibility](#future-possibilities).

See also: [using a package vs. using a workspace](package-vs-workspace).

[package-vs-workspace]:
#https://blog.rust-lang.org/inside-rust/2024/02/13/this-development-cycle-in-cargo-1-77.html#when-to-use-packages-or-workspaces

## Not using `cfg`

Using `cfg` complicates the implementation (and thus maintenance), and may require a substantial
amount of calls to `rustc` to check target-`cfg` compatibility. Some alternatives are discussed here
along with their drawbacks.

### Using wildcards

Instead of using `cfg` specifications, one could use wildcards (e.g., `x86_64-*-linux-*`). This is
much simpler to implement, target-triples are syntactically checked for a match instead of solving
set relations for `cfg`. However, this is not as expressive as `cfg`, and does not correctly
represent the semantics of target-triples. For example, supporting `target_family = "unix"` would
require an annoyingly long list of wildcard patterns. Things like `target_pointer_width = "32"` are
even harder to represent, and things like `target_feature = "avx"` are basically not representable.
Also, this is new syntax not currently used by cargo.

### Allowing only target-triples

This is an even stricter version of the above. Set relations between `supported-targets` lists are
exact, and the resolver can determine if a platform-specific dependency can be pruned from the
dependency tree more easily, hence why the original proposal chose this format. Being even simpler
to implement, this alternative may not be expressive enough for the common use case. Packages rarely
support specific target-triples, rather they support/require specific target attributes. What would
likely happen is that packages would copy and paste the target-triple list matching their
requirements from somewhere or someone else. Every time a new target with the same attribute is
added, the whole ecosystem would have to be updated.

# Prior art
[prior-art]: #prior-art

Locally, users can already specify which targets they want to build for by default using the
`target` field in `cargo`'s `config.toml` file. This setting does not affect packages published on
`crates.io`. On nightly, the `per-package-target` feature defines the `package.default-target` field
in `Cargo.toml` to specify the default target for a package. Both of these options can be
overwritten by the `--target` flag, and only work for a single target-triple.

The `per-package-target` feature also defines the `force-target` field, which is supposed to force
the package to build for the specific target-triple. This does not interact well when used in
dependencies, as one would expect a dependency to be built for the same target as the package.
`supported-targets` supersedes `force-target` because instead of enforcing a single target, it
enforces a set of targets.

Published crates have mainly used their documentation to specify which targets they support, or they
would leave it up to the user to infer it. Some crates also made use of compile time errors to
ensure that `cfg` requirements are met, for example:
```rust
#[cfg(not(any(…)))]
compile_error!("unsupported target cfg");
```
[`getrandom`](https://github.com/rust-random/getrandom/blob/9fb4a9a2481018e4ab58d597ecd167a609033149/src/backends.rs#L156-L160)
is an example of a crate utilizing this method.

I am not aware of a package manager that solves this issue like in this RFC. In other system level
languages, vendoring dependencies is a common practice, and the user would be responsible for
ensuring that the dependencies are compatible with the target. For interpreted languages, this is a
non-issue because any platform being able to run the interpreter can run the package.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- What if one wants to exclude a single target-triple? Groups can be excluded with `cfg(not(..))`,
  but there is currently no way of excluding specific targets (Would anyone ever require this?).

# Future possibilities
[future-possibilities]: #future-possibilities

## Ensuring proper use of dependencies

By leveraging compilation errors, it would be possible to ensure that a package only uses
dependencies that support the package's `supported-targets`. This would be done by comparing the
`supported-targets` of the package with the `supported-targets` of the dependencies.

Cargo's documentation should give clear guidance for when to use this field, and should not suggest
using it by default. In particular, we should steer users to use this when they have good reason to
believe the crate will not compile or work as expected (e.g. because it uses target-specific APIs),
and not use it merely for "I haven't personally tested this on other targets".

### Compatibility of `[dependencies]`

One could restrict the set of `supported-targets` of a package to be a subset of the
`supported-targets` of its `[dependencies]`. If the crate itself had no `supported-targets`
specified, then all dependencies would need to support all targets.

If a dependency does not respect this requirement (if it is not compatible), an error would be
raised and the build would fail.

Enforcing this means a package cannot support targets that are not supported by its dependencies,
which is a good thing assuming the dependencies have correctly specified their `supported-targets`.

### Compatibility of `[dev-dependencies]`

`[dev-dependencies]` should be checked using the same method as regular `[dependencies]`. That is,
the package's `supported-targets` needs to be a subset of every `[dev-dependencies]`'s
`supported-targets`. The rationale is that an example, test, or benchmark has access to the
package's library and binaries, and so it must respect the `supported-targets` of the package.

### Compatibility of `[build-dependencies]`

What makes `[build-dependencies]` unique is that they are built for the host computer, and not the
selected target. As such, they are not restrained by the `supported-targets` of the package. Hence,
all dependencies are allowed in the `[build-dependencies]` table. However, a build error could be
raised if one of the build dependencies does not support the _host_'s target-triple at build time.

A problem can arise if a crate's build script depends on a package that does not support `target_os
= "windows"` for example. It would be possible to only allow dependencies supporting all targets in
`[build-dependencies]`.


### Platform-specific dependencies

Platform-specific dependencies are dependencies under the `[target.**]` table. This includes normal
dependencies, build-dependencies, and dev-dependencies. Rules could be defined to ensure that
platform-specific dependencies are declared correctly.

When platform-specific dependencies are declared, the conditions under which they are declared
should be a subset of each dependency's `supported-targets`. For example, a dependency declared
under `[target.'cfg(target_os = "linux")'.dependencies]` should at least support the `linux` OS.

For regular dependencies and dev-dependencies, it would suffice for a platform-specific dependency
to support the _intersection_ of the package's supported-targets, and the target conditions it is
declared under. For example:
```toml
[package]
# ...
supported-targets = ['cfg(target_os = "linux")']

[target.'cfg(target_pointer_width = "64")'.dependencies]
foo = "0.1.0"
```
Here, it would suffice for `foo` to support `cfg(all(target_os = "linux", target_pointer_width =
"64"))`.

This would ensure that a package properly uses dependencies that are not available on all targets.
Assuming that the crate `io-uring` has `supported-targets = ['cfg(target_os = "linux")']`, a crate
could depend on it using:
```toml
[package]
# ...

[target.'cfg(target_os = "linux")'.dependencies]
io-uring = "0.1.0"
```
This would not be required if the package itself had `supported-targets = ['cfg(target_os =
"linux")']`.

### Artifact dependencies

If an artifact dependency has a `target` field, then the dependency would not be checked against the
package's `supported-targets`. However, the selected `target` for the dependency would need to be
compatible with the dependency's `supported-targets`, or else an error is raised. If the artifact
dependency does not have a `target` field, then it would be checked against the package's
`supported-targets`, like any other dependency.


## Eliminating unused dependencies from `Cargo.lock`

A package's dependencies may themselves have `[target.'cfg(..)'.dependencies]` tables, which may
never be used because of the `supported-targets` restrictions of the package. These can safely be
eliminated from the dependency tree of the package.

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
Currently, `baz` is included in the dependency tree of `foo`, even though `foo` is never built for
`macos`. `baz` could be pruned from the dependency tree of `foo`, since `target_os = "macos"` is
mutually exclusive with `target_os = "linux"`.

Formally, dependencies (and transitive dependencies) under `[target.**.dependencies]` tables are
eliminated from the dependency tree of a package if the `supported-targets` of the package is
mutually exclusive with the target preconditions of the dependency.

## Comparing `supported-targets`

To prune the dependency tree, and to ensure proper use of dependencies, it becomes necessary to
compare `supported-targets` lists. When comparing two `supported-targets` lists, it is necessary to
know if one is a _subset_ of the other, or if both are _mutually exclusive_. To proceed, both lists
are flattened to the same representation, and they are then compared. This process is done
internally, and does not affect the `Cargo.toml` file.

### Flattening `not`, `any`, and `all` in `cfg` specifications [flattening-cfg]:
#flattening-not-any-and-all-in-cfg-specifications

Since `cfg` specifications can contain `not`, `any`, and `all` operators, these must be handled.
This is done by flattening the `cfg` specification to a specific form. This form is equivalent to
[disjunctive normal form](https://en.wikipedia.org/wiki/Disjunctive_normal_form).

The `not` operator is "passed through" `any` and `all` operators using [De Morgan's
laws](https://en.wikipedia.org/wiki/De_Morgan%27s_laws), until it reaches a single `cfg`
specification. For example, `cfg(not(all(target_os = "linux", target_arch = "x86_64")))` is
equivalent to `cfg(any(not(target_os = "linux"), not(target_arch = "x86_64")))`.

Top level `any` operators are separated into multiple `cfg` specifications. For example,
```toml
supported-targets = ['cfg(any(target_os = "linux", target_os = "macos"))']
```
is transformed into
```toml
supported-targets = ['cfg(target_os = "linux")', 'cfg(target_os = "macos")']
```

Top level `all` operators are kept as is, as long as they do not contain nested `any`s or `all`s. If
there is an `any` inside an `all`, the statement is split into multiple `all` statements. For
example,
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

This procedure is run on all `cfg` elements of a `supported-targets` list. The resulting list can
then be used to evaluate relations. In the end, the flattened representation can only contain
explicit target-triples, `cfg(A)` containing a single `A`, or `cfg(all(A, B, ...))` with all `A, B,
...` being single elements.

### The subset relation

To determine if a `supported-targets` list "A" is a subset of another such list "B", the standard
mathematical definition of subset is used. That is, "A" is a subset of "B" if and only if each
element of "A" is contained in "B".

So each element of "A" is compared against each element of "B" using the following rules:
- A `target-triple` is a subset of another `target-triple` if they are the same.
- A `target-triple` is a subset of a `cfg(..)` if the `cfg(..)` is satisfied by the `target-triple`.
- A `cfg(..)` is not a subset of a `target-triple`.
- A `cfg(all(A, B, ...))` is a subset of a `cfg(all(C, D ...))`, if the list `C, D, ...` is a subset
  of the list `A, B, ...`.

_Note_: All possible cases have been covered, since `cfg(A) == cfg(all(A))`.

### Mutual exclusivity

For a `supported-targets` list "A" to be mutually exclusive with another such list "B", each element
of "A" must be mutually exclusive with _all_ elements of "B" (The inverse is also true).

So each element of "A" is compared against each element of "B" using the following rules:
- A `target-triple` is mutually exclusive with another `target-triple` if they are different.
- A `target-triple` is mutually exclusive with a `cfg(..)` if the `cfg(..)` is not satisfied by the
  `target-triple`.
- A `cfg(all(A, B, ...))` is mutually exclusive with a `cfg(all(C, D, ...))` if any element of the
  list `A, B, ...` is mutually exclusive with any element of the list `C, D, ...`.

_Note_: All possible cases have been covered, since `cfg(A) == cfg(all(A))`.

Two `cfg` singletons are mutually exclusive under the following rules:
- `cfg(A)` is mutually exclusive with `cfg(not(A))`.
- `cfg(<option> = "A")` is mutually exclusive with `cfg(<option> = "B")` if `A` and `B` are
  different, and `<option>` has mutually exclusive elements.

Some `cfg` options have mutually exclusive elements, while some do not. What is meant here is, for
example, `target_arch = "x86_64"` and `target_arch = "arm"` are mutually exclusive (a target-triple
cannot have both), while `target_feature = "avx"` and `target_feature = "rdrand"` are not.

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

### More `cfg` relations

Even more relations could be defined. Consider the following scenario:
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
This could compile if `target_os = "macos"` was a subset of `target_family = "unix"`.

Specifically, two extra relations can be defined:
- `cfg(target_os = "windows")` ⊆ `cfg(target_family = "windows")`.
- `cfg(target_os = <unix-os>)` ⊆ `cfg(target_family = "unix")`, where `<unix-os>` is any of
  `["freebsd", "linux", "netbsd", "redox", "illumos", "fuchsia", "emscripten", "android", "ios",
  "macos", "solaris"]`. This list needs to be updated if a new `unix` OS is supported by `rustc`'s
  official target list. This would make the first example compile.

_Note:_ The contrapositive of these relations is also true.

Also, `target_family` is currently defined as not having mutually exclusive elements. This is
because `target_family = "wasm"` is not mutually exclusive with other target families. But,
`target_family = "unix"` could be defined as mutually exclusive with `target_family = "windows"` to
increase usability. By extension, `target_family = "windows"` would now be mutually exclusive with
`target_os = "linux"`, for example.

_Note:_ More relations could be defined, for example `target_feature = "neon"` ⊆ `target_arch =
"arm"`. With this however, things start to get complicated.

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

## `supported-targets` at the cargo-target level

The `supported-targets` field could also be added at the cargo-target level to have more
fine-grained control over which targets a cargo-target supports. This would function similarly to
the `edition` field, which is available at both the package and the cargo-target level. The
`supported-targets` of a cargo-target would most likely need to be a subset of the package's
`supported-targets`.

## Interaction with crate features

Currently, crate `[features]` and `supported-targets` do not interact. It is possible however that a
crate feature interacts with the set of `supported-targets`, either by restraining or expanding it.
It could be possible to allow crate features to modify the `supported-targets` of a package.

## Misc

- Have `cargo add` check the `supported-targets` before adding a dependency.
- Show which targets are supported on `docs.rs`.
- Have search filters on `crates.io` for crates with support for specific targets.
