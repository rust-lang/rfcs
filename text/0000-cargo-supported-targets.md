- Feature Name: (fill me in with a unique ident, `my_awesome_feature`)
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

The addition of `target-requirements` (name to be discussed) to `Cargo.toml`.
This field is an array of `target-triple`/`cfg` specification that restrict the set of target which a package
supports. dependents of packages must meet the `target-requirements` of their dependencies, and
binaries can only be built for targets that meet their `target-requirements`.

# Motivation
[motivation]: #motivation

Some packages don't support every possible rustc target. Trying to depend on a package that does
not support one's target often produces cryptic build errors, or fails at runtime. Allowing libraries to
specify target requirements makes build errors nicer, and then opens the door for more advanced
cross-compilation control. This is especially relevant to embedded programming, where one usually
supports only a few specific targets. Wasm projects also benefit. Along with `default-target`, this feature
has the potential to enhance DX when working in workspaces with packages designed for different targets.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The `target-requirements` field can be added to `Cargo.toml` in any/all Cargo-target
tables (`[lib]`, `[[bin]]`, `[[example]]`, `[[test]]`, and `[[bench]]`).

This field consists of an array of strings, where each string is an explicit target-triple, or a `cfg` requirement
like for the `[target.'cfg(**)']` table. The supported `cfg` syntax is the same as the one for
[platform-specific dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#platform-specific-dependencies)
(i.e., we do not support `cfg(test)`, `cfg(debug_assertions)`, and `cfg(proc_macro)`.

__For example:__
```toml
[package]
name = "hello_cargo"
version = "0.1.0"
edition = "2021"

[lib]
target-requirements = [
    'cfg(target_family = "unix")',
	"wasm32-unknown-unknown"
]
```
Here we only support targets with `target_family = "unix"` __or__ the `wasm32-unknown-unknown` target.

User experience is enhanced by providing a lint (deny by default) that fails compilation when
the target requirements of a package or one of its dependencies are not satisfied. If a package
has `target-requirements` specified, then all of its dependencies' `target-requirements`
must be a superset of its own (see [[#Reference-level explanation]] for details).

When the field is not specified, any target is accepted. _But_, dependencies are checked for compatibility
only when building for a specific target. This is best illustrated with an example. Consider our package
`foo`, which depends on `bar`. `bar` has `target-requirements = ['cfg(target_os = "linux")']`.

If `foo`:
- does not specify `target-requirements`, and tries to build for a linux target, compilation succeeds.
- specifies  `target-requirements = ["cfg(all())"]` (this is a tautology, "true for any target"),
	Then building for a linux target will fail (denied by lint), because `foo`'s requirements are not a subset
	of `bar`'s requirements.

This feature should not be eagerly used; most packages are not tailored for a specific subset of targets.
It should be used when packages clearly don't support all targets. For example: `io-uring`
requires `cfg(target_os = "linux")`, `gloo` requires `cfg(target_family = "wasm")`, and
`riscv` requires `cfg(target_arch = "riscv32")` or `cfg(target_arch = "riscv64")`.
This feature should also be used to enhance cargo's knowledge about your package. For example,
when working in a workspace where some packages compile with `#[no_std]` and `target_os = "none"`, 
and some other packages are tools that require a desktop OS, using `target-requirements`, makes
`cargo <command> --workspace` ignore packages which have `target-requirements` that are not
satisfied.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

_This is incomplete, semantics are not decided yet_.

Please [[#Unresolved questions]] and [[#Rationale and alternatives]] before this section to
understand issues discussed.

## Cargo procedure overview
There will be two new steps that `cargo` will perform.

1. Some moment after `cargo` is done with dependency resolution, if a package contains a
	 `target-requirements` field, the following is done:
	For each requirement `R`, and for each dependency `D`, `R` is checked against `D`, making sure that
	`R` fully satisfies the `target-requirements` of `D`. That is, no target can satisfy the requirement
	`R` and fail the requirements of `D` (one can see this as a "subset" relationship).
	If this procedure fails, then a lint is raised. The behavior for checking `cfg` set relationships is
	defined further below. _Note:_ This step is listed in [[#Rationale and alternatives]] as somewhat
	optional, we do not _need_ it, but I think it would be nice to validate such things.

2. Some moment after step 1, the build target is checked against the package's `target-requirements`,
	and also against every dependency for the same thing. If it does not satisfy all dependencies the
	package itself, a lint is raised.

## Behavior with custom targets
Since we allow `cfg(..)` requirements, custom targets have well defined behavior. The problem arises
if we only supported target names and wildcards, because in this case custom targets would "never match".
## Determining `cfg` set belonging
If it is decided that a package's `target-requirements` are checked to make sure that they are a
subset of all dependencies `target-requirements`, then relation between `cfg` settings need to
be established. For example:
-  `target_os = "windows"`  ⊆ `target_family = "windows"` (although the inverse is true as well
	currently).
- `target_os not in ["wasm", "wasi", "windows", "none", "uefi", "cuda"]` ⊆
	`target_family = "unix"`.

These are the only two relations that I believe could be made (I have checked them against every
target available). The full list of configuration options is given
[here](https://doc.rust-lang.org/reference/conditional-compilation.html),
and I believe there are not
any other relations that can naturally be made. Apart from the cases above, `cfg` requirements
are only satisfied by themselves (e.g., `target_abi = "eabi"` satisfies `target_abi = "eabi"`).

__TODOs:__
- How this interacts with `[target.'cfg(..)']`.
- How this interacts with `bindeps`.


# Drawbacks
[drawbacks]: #drawbacks

- It adds yet another field to `Cargo.toml`.
- It complicates how `Cargo` works.
- Perhaps an external tool could achieve similar results? (I personally don't know how).
- from @epage:
> Performance: this is a lot of extra calls to rustc. Hopefully these
> are all compatible with our rustc cache so they won't make things too bad.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The feature described above is the most comprehensive that I could think of. Other simpler alternatives
are described here.
## Alternatives
- Have something more like `required-targets` (similar to `required-features`) that
	also supports target glob expressions like `x86_64-*-linux-*`.
- Do not validate that a package's `target-requirements` satisfy all dependencies'
	`target-requirements`. Only check that a selected target satisfies the package's and the dependencies'
	requirements.
- Maybe we could use the `[target.**]` table instead. Having something like
	```toml
	[target.'cfg(..)']
	allowed = false
	```
- Naming alternatives: `supported-targets`, `required-targets`.


# Prior art
[prior-art]: #prior-art

Does any one know how other build tools solve this?

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- What if we want to exclude specific target? We can exclude groups with `cfg(not(..))`, but there
	is currently no way of excluding specific targets.
- If the field is not set for a cargo-target, and some dependencies have `target-requirements`, what 
	should we do?
- How do we make users bypass the lint?
- Should we solve for this during version solving? (the current rationale is that we don't want
	targets to affect package version decisions).

# Future possibilities
[future-possibilities]: #future-possibilities

- Make targets have an effect on which vendored dependencies make their way into `Cargo.lock` (I don't
have experience with this, someone please add corrections/details).
- Have different entry points for different targets (see [#9208](https://github.com/rust-lang/cargo/issues/9208)).
