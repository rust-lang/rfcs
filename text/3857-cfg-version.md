- Feature Name: `cfg_version`
- Start Date: 2025-09-13
- RFC PR: [rust-lang/rfcs#3857](https://github.com/rust-lang/rfcs/pull/3857)
- Rust Issue: [rust-lang/rust#64796](https://github.com/rust-lang/rust/issues/64796)

# Summary
[summary]: #summary

Allow Rust-version conditional compilation by adding a built-in `--cfg rust=<version>` and a minimum-version `#[cfg]` predicate.

Say this was added before 1.70, you could do:
```toml
[target.'cfg(not(since(rust, "1.70")))'.dependencies"]
is-terminal = "0.4.16"
```

```rust
fn is_stderr_terminal() -> bool {
    #[cfg(since(rust, "1.70"))]
    use std::io::IsTerminal as _;
    #[cfg(not(since(rust, "1.70")))]
    use is_terminal::IsTerminal as _;

    std::io::stderr().is_terminal()
}
```

This supersedes the `cfg_version` subset of [RFC 2523](https://rust-lang.github.io/rfcs/2523-cfg-path-version.html).

# Motivation
[motivation]: #motivation

These problems mostly have solutions today through third-party crates or other patterns but
- The workarounds for Rust-version-specific dependencies are less straightforward and difficult to get right.
- That requires knowledge of them when users expect this to "just work" like in other ecosystems
- They slow down build times, requiring at least one build script to be fully built (even in `cargo check`) and then executed.  In one sample "simple webserver", in the dependency tree there were 10 build scripts and 2 proc-macros built for the sake of version detection.

## Use cases

In considering use cases, there can be different needs.

Specificity:
- Display version: the format is opaque and only intended for showing to users
- Programmatic version: the format is specified and relied on for comparing values

Semantics:
- Language version: versioning of expected / defined behavior, based on the canonical compiler
- Vendor name/version: identifying the specific compiler being used

We are scoping this RFC down to "language version" but considering "vendor version"
as it can be approximated by the "language version" and in case it breaks any ties in decisions.

### Supporting an MSRV Policy

Requires: programmatic, language version

When maintaining an [MSRV policy](https://doc.rust-lang.org/cargo/reference/rust-version.html#setting-and-updating-rust-version),
maintainers can be caught between:
- The needs of users on older toolchains, regardless of the reason
- The needs of users on the latest toolchain that expect integration with new features (e.g. `Error` in `core`) or faster compile times (dropping `is-terminal` dep in favor of `std::io::IsTerminal`)
- The expectations they have set with their policy

For a simple case, like `Error` in `core`,
maintainers want to conditionally add the `impl core::error::Error` if its supported.

In cases like `std::io::IsTerminal`,
maintainers need to trim dependencies in Cargo for newer Rust versions to maintain reasonable build times for users on newer toolchains.

A challenge with this is that in order to solve this,
we need to add a new feature that requires waiting for an MSRV bump before it can be used.
Being able to check for the presence of this feature would allow immediate adoption.

### Testing proc-macros

*(non-motivating)*

Requires: programmatic, vendor version
- Can be approximated by using the language version

Error reporting can be a major usability issue for proc-macros.
Packages like [`trybuild`](https://crates.io/crates/trybuild) exist to demonstrate and track
the quality of errors reported by proc-macros by compiling sample code and snapshotting the compiler output.
However, compiler output is dependent on the vendor and changes from release to release, so maintainers need to restrict the tests to specific Rust versions.

For example, in `clap`'s [`derive_ui`](https://github.com/clap-rs/clap/blob/master/tests/derive_ui.rs) test:
```rust
#[cfg(feature = "derive")]
#[rustversion::attr(not(stable(1.89)), ignore)] // STABLE
#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/derive_ui/*.rs");
}
```

### Working around compiler bugs

*(non-motivating)*

Requires: programmatic, vendor version
- Can be approximated by using the language version

At times, a vendor's compiler has  bugs that need to be worked around,
e.g. see [error-chain#101](https://github.com/rust-lang-deprecated/error-chain/issues/101).

### Build information

*(non-motivating)*

Requires: display, vendor version
- Can be approximated by using the language version

Some applications choose to include build information in their verbose-version or `--bugreport`.
This can include the compiler vendor and version used to build the application.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When using a new language or standard library feature,
circumstances may warrant doing so while maintaining an existing [MSRV](https://doc.rust-lang.org/cargo/reference/rust-version.html),
rather than raising to what the language or standard library feature needs.
This can be accomplished by conditionally compiling the code for that feature.

For instance, say you have an MSRV of 1.10 and `#[cfg(since)]` feature was available in 1.20, you would have been able to do:
```rust
#[cfg_attr(since(rust, "1.27"), must_use)]
fn double(x: i32) -> i32 {
    2 * x
}

fn main() {
    double(4);
    // warning: unused return value of `double` which must be used
    // ^--- This warning only happens if we are on Rust >= 1.27.
}
```

> Side note: if we also had [RFC 3804](https://github.com/rust-lang/rfcs/pull/3804),
> we can give this condition a semantic name and avoid duplicating it, reducing the chance of bugs:
> ```rust
> #[cfg_alias(must_use_exists, since(rust, "1.27"))]
>
> #[cfg_attr(must_use_exists, must_use)]
> fn double(x: i32) -> i32 {
>     2 * x
> }
>
> fn main() {
>     double(4);
>     // warning: unused return value of `double` which must be used
>     // ^--- This warning only happens if we are on Rust >= 1.27.
> }
> ```

Now, let's say `#[cfg(since)]` was stabilized in Rust 1.27 or later, you can check for support for it with:
```rust
#[cfg_attr(rust, cfg_attr(since(rust, "1.27"), must_use))]
fn double(x: i32) -> i32 {
    2 * x
}

fn main() {
    double(4);
    // warning: unused return value of `double` which must be used
    // ^--- This warning only happens if we are on Rust >= 1.27.
}
```
However, this would produce an `unexpected_cfgs` lint and you would need to add the following to `Cargo.toml`:
```toml
[lints.rust]
unexpected_cfgs = { level = "warn", check-cfg = ['cfg(rust,values(none()))'] }
```

Say you were wanting to test out `#[must_use]` after it got stabilized on nightly to provide feedback and to be ready for when it hits stable,
you would instead use `"1.27.0-0"` to match all pre-release versions of 1.27.0:
```rust
#[cfg_attr(since(rust, "1.27.0-0"), must_use)]
fn double(x: i32) -> i32 {
    2 * x
}

fn main() {
    double(4);
    // warning: unused return value of `double` which must be used
    // ^--- This warning only happens if we are on Rust >= 1.27.
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## `since` cfg predicate

A `since` cfg predicate will be added to Rust.
As Cargo mirrors Rust's `#[cfg]` syntax, it too will gain this predicate.

The [syntax](https://doc.rust-lang.org/reference/conditional-compilation.html#grammar-ConfigurationPredicate) is:
```
ConfigurationVersion -> `since` `(` IDENTIFIER `,` ( STRING_LITERAL | RAW_STRING_LITERAL ) `)`
```

If `IDENTIFIER` has no value or is undefined, this will evaluate to `false`.

If `IDENTIFIER` is not a valid [SemVer](https://semver.org/) value, this will evaluate to `false`.

If the string literal does not conform to the syntax from `<major>` to `<major>.<minor>.<patch>-<pre-release>` where the first three fields must be integers, this will evaluate to `false`.
Note that this excludes support for the `+build` field.

Otherwise, the `IDENTIFIER` will be compared to the string literal according to
[Cargo's `>=` version requirements](https://doc.rust-lang.org/nightly/cargo/reference/specifying-dependencies.html#comparison-requirements).
For example, `#[cfg(since(rust, "1.90"))]` will be treated as `1.95.2 >= 1.90.0`.

See also `--check-cfg`.

## `--check-cfg`

A new predicate will be added of the form:
```
ConfigurationVersion -> `since` `(` ( STRING_LITERAL | RAW_STRING_LITERAL ) `)`
```

The syntax for the contents of the string literal is a SemVer value without the build field.

This will specify that for the given cfg, values will be valid if:
- SemVer syntax
- from the specified version and up

Specifically when the given cfg is used with the `cfg` `since` predicate:
- the string literal should not be of a syntax that evaluates to `false`
- the string literal must be a minimum version requirement that specifies a subset of what the `--check-cfg` specifies

So given `--check-cfg 'cfg(foo, values(since("1.95.0")))'`,
- ✅ `#[cfg(foo = "1.100.0")]`
- ⚠️ `#[cfg(foo = "1.0")]`: not SemVer syntax
- ✅ `#[cfg(since(foo, "1.95.0"))]`
- ✅ `#[cfg(since(foo, "1.100.0"))]`
- ✅ `#[cfg(since(foo, "3.0.0"))]`
- ✅ `#[cfg(since(foo, "1.95"))]`
- ⚠️ `#[cfg(since(foo, "1.90.0"))]`: matches a superset of `--check-cfg`
- ⚠️ `#[cfg(since(foo, "1.95.0-0"))]`: matches a superset of `--check-cfg`
- ⚠️ `#[cfg(since(foo, "1"))]`: matches a superset of `--check-cfg`
- ⚠️ `#[cfg(since(foo, "bar"))]`: invalid string literal syntax

## `rust` cfg

A new built-in cfg `--cfg rust=<version>` will be added by the compiler
that specifies the language version.
This will be the version of `rustc` with the behavior for pre-release versions being unspecified.
We expect rustc to:
- Translate the `-nightly` pre-release to `-incomplete`
- Strip the `-beta.5` pre-release

`rust` will be specified as `--check-cfg 'cfg(rust, values(since("1.95.0")))'`
(or whatever version this gets stabilized in).

This will be reported back through `--print=cfg`.

Because this gets reported back in `--print=cfg`,
Cargo will expose `rust` in:
- build scripts as `CARGO_CFG_RUST`
- `[target."cfg()".dependencies]`

## clippy

Clippy has a [`clippy::incompatible_msrv`](https://rust-lang.github.io/rust-clippy/master/index.html#incompatible_msrv) lint
which will fire whenever a standard library item is used with a `#[stable(since)]` newer than `package.rust-version`.
However, it will be perfectly reasonable to use those items when guarded by a `#[cfg(since)]`.

Clippy may wish to:
- Find a way to reduce false positives, e.g. evaluating the `cfg(since)`s that led to the item's usage or disabling the lint within `#[cfg(since)]`
- Suggest `#[cfg(since)]` in the `clippy::incompatible_msrv` diagnostic report (maybe along with offering to bump MSRV as that is a reasonable alternative)

# Drawbacks
[drawbacks]: #drawbacks

People may be using `--cfg rust` already and would be broken by this change.
There are no compatibility concerns with predicate names.

This does not include a solution for adopting this within `Cargo.toml` without waiting for an MSRV bump.

Traditionally, maintainers only test their MSRV and latest, assuming those will catch every issue.
While that isn't always true today (e.g. some Cargo features go from "unknown" warning to "unstable" error to supported and MSRV might be in the warning phase),
having distinct implementations for different Rust versions can make the testing matrix more complex.
Tools like [`cargo hack`](https://crates.io/crates/cargo-hack) can help which can run commands on not just one toolchain version but also the every version starting with the MSRV with a command like `cargo hack --rust-version --version-step 1 check`.

This does not help with identifying nightlies that a feature is available on or compatible with which will still require a `build.rs`.
In terms of doing this via build probes,
Cargo team has previously rejected support for build probes
([source](https://github.com/rust-lang/cargo/issues/11244#issuecomment-2326780810)).
Whether build probes or nightly version checks,
auto-enabling nightly features
(rather than having users opt-in)
runs counter to the spirit of nightly (works like stable except where you opt-in)
and cause problems if the checks are incorrect which has broken many crates' nightly builds in the past
and has even caused friction in changing unstable features within the compiler.

Libraries could having ticking time bombs that accidentally break or have undesired behavior for some future Rust version that can't be found until we hit that version.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## `since` cfg predicate

We could offer a `before` predicate but that is already covered by `not(since)`.

The `since` name was taken from 
[rustversion](https://crates.io/crates/rustversion) and the `#[deprecated(since)]` / `#[stable(since)]` attributes.
This better conveys what operation is being performed than the original `version` name
and leaves room for related predicates like `before`.
We could also call this `minimum`, or support comparison operators in the spirit of [RFC 3796](https://github.com/rust-lang/rfcs/pull/3796).
The risk with a general word like `since` is if we gain support for other data types in cfgs, like integers for embedded development.
The name `since` might apply in some situations but not others and its unclear if we'd want to generalize it past versions.
While having a specific name avoids these concerns.

We could swap the order of parameters and make `rust` a default for the second parameter to allow `#[cfg(since("1.95"))]` as a shorthand.
However, this would look confusing in Cargo and doesn't seem like its offering enough of a benefit to be worth the costs.

The `ConfigurationVersion` is sloppy with the string literal's syntax (relying on `--check-cfg`) so that
- Allows evolution without requiring an MSRV bump
- Its consistent with other predicates, e.g. `#[cfg(foo = "1.0")]`

If we were stricter on the syntax,
we could allow for version numbers to be directly accepted, without quotes 
(e.g. `#[cfg(since(rust, 1.95.0))]`).
If we ever decided to support operators (e.g.`#[cfg(since(rust, "=1.95.0"))]`, see `--check-cfg`), then we'd need to decide if those also go outside the string or then require a string, being inconsistent.
This may limit us if we decide to allow for alternative version formats like with [target_version](#cfg_target_version) as they may not have formats that map well to SemVer.
Worst case, we'd need to accept arbitrary bare words.
This would also be inconsistent with other uses of `cfg`s
*but* maybe that would just be the start to natively supporting more types in `cfg`,
like integers which are of interest to embedded folks.

## `--check-cfg`

The `--check-cfg` predicate and the value for `rust` ensures users get warnings about
- Invalid syntax
- Using this with versions from before its supported, e.g. `#[cfg(since(rust, "1.0.0")]`

`--check-cfg` requires a SemVer version, rather than a version requirement,
in case we want the future possibility of relaxing SemVer versions
*and* we want to infer from the fields used in `--check-cfg` to specify the maximum number of fields accepted in comparisons.

We could have the `check-cfg` `since` predicate only apply to the `cfg` `since` predicate,
causing `#[cfg(rust = "1.100.0")]` to warn.
However,
- the `since` predicates are a general feature intended to be used with other version numbers where exact matches may also be appropriate.
- this would get in the way of approximating the vendor version by the language version for working around compiler bugs and snapshotting of compiler output.

Possibly there could be a clippy lint specifically about `rust = "<something>"`.
Alternatively, we could try to find a way to structure `--check-cfg` to allow the person setting the `check-cfg` to decide whether it can be used with `=` or not.
One way of doing this is by allowing the `check-cfg` `since` predicate outside of the `values` predicate,
meaning it works with the `cfg` `since` predicate and not the `=` operator.
Another way would be for the `check-cfg` `since` predicate to never work with `=` but to instead
allow operators inside of the `cfg` `since` predicate, e.g. `#[cfg(since(rust, "=1.95.0"))]`.
However, with the rename of the predicate from `version` to `since`, operators don't fit in as easily.
If someone wanted to support equality checks, there wouldn't be a way to support a continuous range of `values()` but would instead have to manually specify each likely potential version.

`--check-cfg` will cause the following to warn:
```rust
fn is_stderr_terminal() -> bool {
    #[cfg(rust)]
    #[cfg(since(rust, "1.70"))]
    use std::io::IsTerminal as _;
    #[cfg(rust)]
    #[cfg(not(since(rust, "1.70")))]
    use is_terminal::IsTerminal as _;
    #[cfg(not(rust))]
    use is_terminal::IsTerminal as _;

    std::io::stderr().is_terminal()
}
```

To allow checking for the presence of `rust`, add the following to your `Cargo.toml`:
```toml
[lints.rust]
unexpected_cfgs = { level = "warn", check-cfg = ['cfg(rust,values(none()))'] }
```
Alternatively, we could have the built-in `--check-cfg` for `rust` include `values(none())` but:
- When building on an old version, users will see the warning and will likely want to add it anyways.
- We lose out on `--check-cfg` identifying misused.
  Instead, we may wish to add a dedicated predicate intended for "is set".

## `rust` cfg

While there was concern over `rust` appearing in the name of `cfg(rust_version("1.95"))`,
I feel that `rust` as its own entity makes sense and avoids that problem.

Rust does appear in some parts of the language,
but is capitalized like with [`repr(Rust)`](https://doc.rust-lang.org/reference/type-layout.html?#the-rust-representation).
However, the convention for `--cfg`s is generally lower case.

### Pre-release

When translating `rustc --version` to a language version, we have several choices when it comes to pre-releases, including:
- Treat the nightly as fully implementing that language version
- Treat the nightly as not implementing that language version at all, only the previous
- Leave a marker that that language version is incomplete, while the previous language version is complete

In RFC 2523, this was left as an
[unresolved question](https://rust-lang.github.io/rfcs/2523-cfg-path-version.html#unresolved-questions).

The initial implementation treated nightlies as complete.
This was [changed to incomplete](https://github.com/rust-lang/rust/pull/72001) after
[some discussion](https://github.com/rust-lang/rust/issues/64796#issuecomment-624673206).
In particular, this is important for
- the case of package `bleeding-edge` starting to use a new feature behind `#[cfg(since)]` and package `nightly-only` has their toolchain pinned to a nightly before the feature was stabilized (to ensure consistent behavior of unstable features), package `nightly-only` cannot add or update their dependency on `bleeding-edge` without getting a "feature gate needed" error.
- bisecting nightlies.

This was [changed back to complete](https://github.com/rust-lang/rust/pull/81468) after
[some more discussion](https://github.com/rust-lang/rust/issues/64796#issuecomment-634546711).
In particular, this is important for
- keeping friction down for packages preparing for stabilized-on-nightly features as their `#[cfg(since)]`s can be inserted and "just work" which can be important for getting feedback quickly while the feature is easier to adapt to feedback that can be gained from these users
  - releasing the package while its in this state puts it at risk to be broken if the feature is changed after stabilization

For RFC 2523, they settled on pre-releases being incomplete,
favoring maintainers to adopt stabilized-on-nightly features immediately
while letting people on pinned nightlies or bisecting nightlies to set a `-Z` to mark the version as incomplete.

In this RFC, we settled on translating `-nightly` to `-incomplete` because:
- Maintainers can adopt stabilized-on-nightly features with `#[cfg(since(rust, "1.100.0-0"))]` (the lowest pre-release for `1.100.0`), keeping friction low while explicitly acknowledging that the unstable feature may change
- Allows build scripts to experiment with other logic when approximating the vendor version from the language version with less of a chance of needing to invoke `rustc` (e.g. detecting nightly)
- It provides extra context when approximating the vendor version from the language version when populating build information

We called the pre-release `-incomplete` to speak to the relationship to the language version.
Other terms like `partial` could as easily apply.
The term `-nightly` would be more discoverable but would convey more of a relationship to the vendor than the language.

As for differentiating between nightlies,
that corresponds more to the vendor version than the language version,
so we do not include that information.

## Alternative designs

### `cfg(rust_version(1.95))`

*(this is [RFC 2523](https://rust-lang.github.io/rfcs/2523-cfg-path-version.html))*

Add a new predicate that knows the current Rust version and can compare a value against it.
Cargo would need to duplicate this lint.

A lint would be needed to ensure the version is newer than when the predicate was added.

To support Rust versions from before this predicate was added,
we could add `--cfg has_rust_version`.

On the [stabilization issue](https://github.com/rust-lang/rust/pull/141766),
there was concern about the name "rust" in this predicate not fitting in with the rest of the language.
However, dropping it to `version` would make things awkward in Cargo where there wouldn't be enough context for which item's `version` is being referred to.
There is also a future possibility of better integrating dependency versions into the language.
If done, then `version` may become more ambiguous even in Rust.
For example, if Cargo told rustc the minimum compatible version for a dependency, `#[deprecated(since)]`` warnings could not emit if the minimum version bound is lower than `since`.
Similarly, if we stabilized `#[stable(since)]`, a linter could report when a version requirement is too low.

We could rename this to `version` and stabilize it as-is,
with this RFC being a future possibility that adds an optional second parameter for specifying which version is being referred to.

### `cfg(rust = "1.95")`

*(this [idea](https://github.com/rust-lang/rust/pull/141766#issuecomment-2940720778) came up on the stabilization PR for RFC 2523)*

`rust` could represent the "versions supported" and would be a set of all versions, `<major>.<minor>` and `<major>.<minor>.<patch>`,
between the version it was first supported up to the current version,
making the `=` operate as a "contains" operator,
rather than an equality operator,
like with `#[cfg(feature = "std")]`.
This was proposed to allow `#[cfg_attr(rust, cfg(rust = "1.95"))]` to more naturally allow adoption before the feature is stabilized.

This could be implemented statically, by hard coding the versions.
This would work with `--print-cfg` and so would automatically work with Cargo.
However, there would `unexpected_cfgs` warnings if someone specified a point release unknown to the current toolchain.
As for `--check-cfg`, it would either hard-code the list of potential future version up to a certain limit, have a new predicate, or be handled through a different lint mechanism.
The list of `--print-check-cfg` items would be large and the list of `--print-cfg` items would only grow.
We could drop support for patch releases but then maintainers couldn't approximate the vendor version to work around bugs or to report build information.

Alternatively, whether a value is contained in `rust` could be determined dynamically.
`rust` would not show up in `--print-cfg`.
As for `--check-cfg`, it would either need to also be dynamic (and not printed by `--print-check-cfg`), a new predicate, or handled through a different lint mechanism.
Cargo would need to duplicate this dynamic value.
**Note that this in was [rejected in RFC 2523](https://rust-lang.github.io/rfcs/2523-cfg-path-version.html#the-bikeshed---argument-syntax) due to this dynamic nature.**

The "contains" behavior of `=` is not too obvious.
For the `feature` set,
I presume it was named in the singular
(as opposed to being consistent with the `[features]` table or plural to convey it is a set)
to fit in with looking like an equality operation (`#[cfg(feature = "foo")]`).
We could add a new predicate to convey set containment.

# Prior art
[prior-art]: #prior-art

## Rust

### `rustversion`

[crates.io](https://crates.io/crates/rustversion)
- MSRV of 1.31
- proc-macro that queries rustc through a build script
- 531 reverse dependencies with ~260 million downloads

Provides
- channel checks: `#[rustversion::stable]`, `#[rustversion::beta]`, `#[rustversion::nightly]`
- equality checks: `#[rustversion::stable(1.34)]`, `#[rustversion::nightly(2025-01-01)]`
- `>=` version: `#[rustversion::since(1.34)]`
- `>=` nightly: `#[rustversion::since(2025-01-01)]`
- `<` version: `#[rustversion::before(1.34)]`
- `<` nightly: `#[rustversion::before(2025-01-01)]`

### `rustc_version`

[crates.io](https://crates.io/crates/rustc_version)
- MSRV of 1.32
- library for use in build scripts for conditional compilation
- 680 reverse dependencies with ~330 million downloads

Accessible
- Channel
- Version
- Release metadata (e.g. commit hash)

### `version_check`

[crates.io](https://crates.io/crates/version_check)
- library for use in build scripts for conditional compilation
- 152 reverse dependencies with ~450 million downloads

Accessible
- Query channel, version, and date
- Min, max, and equality operators for the above

### Polyfills

The `is_terminal_polyfill` maintains
[versions](https://crates.io/crates/is_terminal_polyfill/versions)
for each MSRV with distinct implementations,
relying on the [MSRV-aware resolver](https://rust-lang.github.io/rfcs/3537-msrv-resolver.html)
to pick the appropriate version.

### `shadow-rs`

[crates.io](https://crates.io/crates/shadow-rs)
- library for use in build scripts for release information
- 81 reverse dependencies with ~5 million downloads

Accessible
- Release information
- Channel
- Cargo version

### `vergen`

[crates.io](https://crates.io/crates/vergen)
- library for use in build scripts for release information
- 182 reverse dependencies with ~26 million downloads

Accessible
- Channel
- Commit date
- Commit hash
- LLVM version
- Version

## Other

Python
- Programmatic version: [`sys.version_info`](https://docs.python.org/3/library/sys.html#sys.version_info)
- Vendor display version: [`sys.version`](https://docs.python.org/3/library/sys.html#sys.version)
- [Dependency specifiers](https://packaging.python.org/en/latest/specifications/dependency-specifiers/)
  - e.g. `requests [security,tests] >= 2.8.1, == 2.8.* ; python_version < "2.7"`

C++
- Numeric value representing the version of the C++ standard: [`__cplusplus`](https://en.cppreference.com/w/cpp/preprocessor/replace):

C:
- Implementation-defined value of the C++ standard: [__STDC_VERSION__](https://en.cppreference.com/w/cpp/preprocessor/replace)

Haskell:
- Numeric value representing the vendor's version, e.g. `#if __GLASGOW_HASKELL__ >= 706`

# Unresolved questions
[unresolved-questions]: #unresolved-questions

# Future possibilities
[future-possibilities]: #future-possibilities

- In the future the `--check-cfg` `since()` predicate could make the minimum-version field optional,
  matching all version numbers.

## Relaxing SemVer

Instead of requiring the `IDENTIFIER` in the `check-cfg` `since` predicate to be strictly SemVer `major.minor.patch`,
we could allow abbreviated forms like `major.minor` or even `major`.
This would make the predicate more inclusive for other cases, like `edition`.

## Vendor name and version

We could add `--cfg`s for the compiler vendor name and version.
In addition to the use cases given in the Motivation section,
this will allow users to check for specific nightly versions.

Some challenges for this with `rustc --version`:
- Nightly versions for a given release are mutable,
  all mapping to the `-nightly` pre-release version rather than including the date within the pre-release
- This does not conform to SemVer's precedence rules,
  as `-nightly` is an older version than `-beta.4` while [SemVer's precedence rules](https://semver.org/#spec-item-11) say the opposite
- Crater runs and local builds don't necessarily have a version that fits within this picture

## `#[cfg(nightly)]`

Depending on what is meant by this,
we either need the language version or the vendor name and version as well as a way to check for the presence of `pre-release`.

See also [`#[cfg(nightly)]`](https://rust-lang.github.io/rfcs/2523-cfg-path-version.html#cfgnightly) in the previous RFC.

## `cfg_target_version`

Instead of defining a new `#[cfg]` predicate, [RFC 3750](https://github.com/rust-lang/rfcs/pull/3750)
could reuse the `#[cfg(since)]` predicate.

As not all systems use SemVer, we can either
- Contort the version into SemVer
  - This can run into problems either with having more precision (e.g. `120.0.1.10` while SemVer only allows `X.Y.Z`) or post-release versions (e.g. [`1.2.0.post1`](https://packaging.python.org/en/latest/discussions/versioning/) which, if we translated it to SemVer's syntax of `1.2.0-post1`, would be treated as a pre-release).
- Add an optional third field for specifying the version format (e.g. `#[cfg(since(windows, "10.0.10240", <policy-name>)]`)
- Make `--check-cfg` load-bearing by having the version policy name be specified in the `--check-cfg` predicate

## Provide a way to get a `--cfg`s value

Similar to how `cfg!` allows doing conditionals in Rust code, provide a "`cfg_value!`" for reading the value.
On top of [other use cases](https://internals.rust-lang.org/t/pre-rfc-mutually-excusive-global-features/19618) for `cfg_value!`,
this would allow an application to approximate the vendor version `--bugreport` / `-v --version` without a build script.

## Conditional compilation for dependency versions

As the ecosystem grows and matures,
the Rust language and standard library may not be the only dependencies users wish to support multiple versions of.
We may want to allow `#(cfg(since(serde, "1.0.900")]`.
