- Feature Name: `cfg_version`
- Start Date: 2025-09-13
- RFC PR: [rust-lang/rfcs#3857](https://github.com/rust-lang/rfcs/pull/3857)
- Rust Issue: [rust-lang/rust#64796](https://github.com/rust-lang/rust/issues/64796)

# Summary
[summary]: #summary

Allow Rust-version conditional compilation by adding
- a built-in `--cfg=rust --cfg=rust="<version>"`, for the Rust language version
- `#[cfg(version_since(cfg_name, "<version>"))]`, a minimum-version `cfg` predicate

Say this was added before 1.70, you could do:
```toml
[target.'cfg(not(version_since(rust, "1.70")))'.dependencies"]
is-terminal = "0.4.16"
```

```rust
fn is_stderr_terminal() -> bool {
    #[cfg(version_since(rust, "1.70"))]
    use std::io::IsTerminal as _;
    #[cfg(not(version_since(rust, "1.70")))]
    use is_terminal::IsTerminal as _;

    std::io::stderr().is_terminal()
}
```

This supersedes the `cfg_version` subset of [RFC 2523](https://rust-lang.github.io/rfcs/2523-cfg-path-version.html).

# Motivation
[motivation]: #motivation

These problems mostly have solutions today through third-party crates or other patterns but
- That requires knowledge of them when users expect this to "just work" like in other ecosystems
- They slow down build times, requiring at least one build script to be fully built (even in `cargo check`) and then executed.  In one sample "simple webserver", in the dependency tree there were 10 build scripts and 2 proc-macros built for the sake of version detection.
- The workarounds for Rust-version-specific dependencies are less straightforward and difficult to get right.

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
rather than raising it to what the language or standard library feature needs.
This can be accomplished by conditionally compiling the code for that feature.

As its hard to talk about features and versions in the future,
we're going to step through this in an alternate reality where:
- `--check-cfg` (warn on invalid conditional compilation) was stabilized in 1.0
- `--cfg rust` and `#[cfg(version_since)]` were stabilized in 1.20
- `#[must_use]` (an example language feature) was still stabilized in 1.27

For instance, say you have an MSRV of 1.20, to use `#[must_use]` you would do:
```rust
#[cfg_attr(version_since(rust, "1.27"), must_use)]
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
> #[cfg_alias(must_use_exists, version_since(rust, "1.27"))]
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

Now,
let's say your MSRV is 1.10,
the above code would error when compiling with your MSRV because the `version_since` predicate does not exist with that version.
However, the presence of `--cfg rust` implies that we are on 1.27,
so you can "detect" support for `version_since` by changing your code to:
```rust
#[cfg_attr(rust, cfg_attr(version_since(rust, "1.27"), must_use))]
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
unexpected_cfgs = { level = "warn", check-cfg = ['cfg(rust)'] }
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## `version_since` cfg predicate

A `version_since` cfg predicate will be added to Rust.
As Cargo mirrors Rust's `#[cfg]` syntax, it too will gain this predicate.

The [syntax](https://doc.rust-lang.org/reference/conditional-compilation.html#grammar-ConfigurationPredicate) is:
```
ConfigurationSince -> `version_since` `(` IDENTIFIER `,` ( STRING_LITERAL | RAW_STRING_LITERAL ) `)`

Version = NumericVersionField `.` NumericVersionField

NumericVersionField ->
    `0`
  | ( [`1`..`9`] DEC_DIGIT* )
```

When evaluating `version_since`,
1. If the string literal does not conform to the syntax `Version`, the compiler will error.
2. If `IDENTIFIER` is unset, this will evaluate to `false`.
3. If any of the following evaluates to `true` for any cfg entry for `IDENTIFIER`, `version_since` will evaluate to `true`, otherwise `false`.
    1. If `IDENTIFIER` is name-only, this entry will evaluate to `false`.
    2. If `IDENTIFIER`'s value is not a valid `Version`, the compiler will error.
    3. Otherwise, if `IDENTIFIER`s value has the same or higher precedence, this entry will evaluate to `true`
       For example, `#[cfg(version_since(rust, "1.90"))]` will be interpreted as `precedence_of(1.95.2) >= precedence_of(1.90.0)`.

With the precedence of:
- Precedence is calculated by separating the `Version` into the respective `NumericVersionField`s
- Precedence is determined by the first difference when comparing each field from left to right of `Version`
  - `NumericVersionField` is compared numerically

This was adopted from [SemVer](https://semver.org/) with the following changes:
- Removed pre-release
- Dropped down to 2 fields
- Removed build metadata (which does not affect precedence)

Examples:
- `cfg(version_since(unset_name, "1.0"))` will be false
- `--cfg name_only` and `cfg(version_since(name_only, "1.0"))` will be false
- `--cfg foo="bird"` and `cfg(version_since(name_only, "1.0"))` will be a compiler error
- `--cfg foo="1.1"` and `cfg(version_since(foo, "bird"))` will be a compiler error
- `--cfg foo="1.1"` and `cfg(version_since(foo, "1.0"))` will be true
- `--cfg foo="1.1"` and `cfg(version_since(foo, "1.2"))` will be false
- `--cfg foo --cfg foo="1.1" --cfg foo="1.0"` and `cfg(version_since(foo, "1.1"))` will be true

The compiler implementation currently treats cfgs as `HashSet<(String, Option<String>)>`
and would likely need to change this to `HashMap<String, HashSet<Option<String>>>``
to accommodate this predicate.

## `--check-cfg`

A new predicate will be added of the form:
```
CheckConfigurationSince -> `version_since` `(` ( STRING_LITERAL | RAW_STRING_LITERAL ) `)`
```

The syntax for the contents of the string literal is a `Version`.

This will specify that for the given cfg, string literals will be valid if:
- `Version` syntax
- from the specified version and up

When checking a `version_since` predicate,
- the string literal must be a minimum version requirement that specifies a subset of what the `--check-cfg` specifies

*note: non-version string literals are already a compiler error*

This composes with all other values specified with the `values()` predicate

So given `--check-cfg 'cfg(foo, values(version_since("1.95")))'`,
- ✅ `#[cfg(foo = "1.100")]`
- ⚠️ `#[cfg(foo = "1.100.0")]`: not `Version` syntax
- ✅ `#[cfg(version_since(foo, "1.95"))]`
- ✅ `#[cfg(version_since(foo, "1.100"))]`
- ✅ `#[cfg(version_since(foo, "3.0"))]`
- ✅ `#[cfg(version_since(foo, "1.95"))]`
- ⚠️ `#[cfg(version_since(foo, "1.90"))]`: matches a superset of `--check-cfg`
- ⚠️ `#[cfg(version_since(foo, "1"))]`: not `Version` syntax
- ⚠️ `#[cfg(version_since(foo, "bar"))]`: not `Version` syntax

## `rust` cfg

A new built-in cfg `--cfg=rust --cfg=rust="<version>"` will be added by the compiler
that specifies the language version.
This will be the version of `rustc` without the patch field and with the behavior for pre-release versions being unspecified.
We expect rustc to treat beta and nightly versions as an "incomplete" implementation of that language version,
reporting some number less than the current nightly.

`rust` will be specified as `--check-cfg 'cfg(rust, values(version_since("1.95")))'`
(or whatever version this gets stabilized in).
Like with `--check-cfg` for Cargo `features`,
the compiler may choose to add additional context for why this lower bound is present (not stabilized).

This will be reported back through `--print=cfg`.

Because this gets reported back in `--print=cfg`,
Cargo will expose `rust` in:
- build scripts as `CARGO_CFG_RUST`
- `[target."cfg()".dependencies]`

## clippy

Clippy has a [`clippy::incompatible_msrv`](https://rust-lang.github.io/rust-clippy/master/index.html#incompatible_msrv) lint
which will fire whenever a standard library item is used with a `#[stable(version_since)]` newer than `package.rust-version`.
However, it will be perfectly reasonable to use those items when guarded by a `#[cfg(version_since)]`.

Clippy may wish to:
- Find a way to reduce false positives, e.g. evaluating the `cfg(version_since)`s that led to the item's usage or disabling the lint within `#[cfg(version_since)]`
- Suggest `#[cfg(version_since)]` in the `clippy::incompatible_msrv` diagnostic report (maybe along with offering to bump MSRV as that is a reasonable alternative)

# Drawbacks
[drawbacks]: #drawbacks

People may be using `--cfg rust` already and would be broken by this change.
There are no compatibility concerns with predicate names.
At least a preliminary search of GitHub did not uncover uses
but that search may have been incomplete
and that data set is biased towards open source and not all uses of Rust.

Ignoring the logic, a straight-English reading of `#[cfg(not(version_since("1.95")))]` is unnatural and could cause confusion.
This can be mitigated by use of `#[cfg_alias]`
which will let users provide a semantic name for the positive case that works with the negative case,
on top of the other benefits of providing a central, semantic name.
This could also be helped by supporting a `#[cfg(version_before("1.95"))]`.
This was left to [a future possibility][future-possibilities].

While Rust can stacks `cfg`s to test for the presence of this feature on older versions,
this does not include a solution for adopting this within `Cargo.toml` without waiting for an MSRV bump to the version `version_since` is stabilized in.

Traditionally, maintainers only test their MSRV and latest stable, assuming those will catch every issue.
While that isn't always true today (e.g. some Cargo features go from "unknown" warning to "unstable" error to supported and MSRV might be in the warning phase),
having distinct implementations for different Rust versions can make the testing matrix more complex.
Tools like [`cargo hack`](https://crates.io/crates/cargo-hack) can help which can run commands on not just one toolchain version but also the every version starting with the MSRV with a command like `cargo hack --rust-version --version-step 1 check`.

As we don't expose a nightly's date,
this does not cover the use case from [rustversion](https://crates.io/crates/rustversion) represented by
`#[rustversion::since(2025-01-01)]`.

Without patch, there will be more difficulty in using the Language version as a proxy for the Vendor version for working around bugs or knowing what compiler version was used in a bug report.

Libraries could having ticking time bombs that accidentally break or have undesired behavior for some future Rust version that can't be found until we hit that version.

Compared to the more specialized alternative designs,
this more general solution may take more time in design discussions, implementation, and vetting the implementation
as there are more corner cases to cover, particularly with how this integrates with future possibilities.

## Pre-releases for major versions

Pre-releases of major versions isn't a consideration for `rust` but in the general use of `version_since`.

If wanting to split a continuous range with minor and patch versions,
`#[cfg(version_since(foo, "1.1"))]` and `#[cfg(not(version_since(foo, "1.1")))]`
works reasonably well.

The problem comes into play when doing so with major versions when pre-releases are involved,
like `#[cfg(version_since(foo, "2.0"))]` and `#[cfg(not(version_since(foo, "2.0")))]`.
In this situation, a `2.0.0-dev.5` will match the second condition when the user likely only wanted to include `1.*`.
Instead, they should do `#[cfg(version_since(foo, "2.0.0-0"))]` and `#[cfg(not(version_since(foo, "2.0.0-0")))]` or have a third case for pre-releases of `foo@2.0.0`.

This came up in Cargo when considering how to improve interactions with pre-releases.
Cargo has the advantages of:
- Not working with splitting continuous ranges, so special cases can be made that cause discontinuities
- Simpler expressions that can be analyzed for considering global knowledge.

For more information on Cargo's experiments with this (all unstable),
see [cargo#14305](https://github.com/rust-lang/cargo/pull/14305).

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## `version_since` cfg predicate rationale

The `version_since` name was taken from
[`rustversion::since`](https://crates.io/crates/rustversion) and the `#[deprecated(since)]` / `#[stable(version_since)]` attributes.
We need to express both what comparison is happening (`>=`) and the comparison algorithm (version sort).
This also leaves room for related predicates like `version_before`.
In particular, as this is a general feature and not just for Rust version comparisons,
we need to consider cases with the original `version` name like `version(python, "2.8")` and whether people would interpret that as an exact match, a SemVer compatible match (`^` in cargo), or a `>=` match (the winner).
We could also call this `version_minimum` or `version_ge`, or support comparison operators in the spirit of [RFC 3796](https://github.com/rust-lang/rfcs/pull/3796).
Including `since` reduces the risk if we gain support for other data types in cfgs, like integers for embedded development.
Naming it only `since` might apply in some situations but not others and its unclear if we'd want to generalize it past versions.
Having a more specific name like `version_since` / `since_version` could avoid these concerns.
We chose `version_since` since it can read as "for versions since rust 1.20", follows the type-method naming style, help with version-related auto-completions, and version-related predicates will naturally sort next to each other.
However, the name `version_since` can seem excessively verbose, especially compared to `version` or `since`.

We could swap the order of parameters and make `rust` a default for the second parameter to allow `#[cfg(version_since("1.95"))]` as a shorthand.
However, this would look confusing in Cargo and doesn't seem like its offering enough of a benefit to be worth the costs.

`ConfigurationSince` requires the `IDENTIFIER` and string literal to be a `Version`,
erroring otherwise,
so we can have the flexibility to relax the syntax over time without it being a breaking change
For example, if `--cfg=foo="1.0"` caused `cfg(version_since(foo, "1.0"))` to be `false` and we later allowed `"1.0"` for the `IDENTIFIER`, it would now be `true` and would change behavior.
Because we'll have `version_since(rust, _)` at that point, it won't require an MSRV bump.
This does leave the door open for us to relax this in the future once we become comfortable with the flexibility of our version syntax.
Alternatively, we could try to determine a flexible-enough version syntax now though that comes with the risk that it isn't sufficient.
Another benefit to erroring is so `not(version_since(invalid, "<invalid>"))` is not `true`.

Having a unset or name-only `IDENTIFIER` evaluate to `false` is consistent with `cfg(IDENTIFIER)` and `cfg(IDENTIFIER = "value")`.
When a version can be conditionally present,
it avoids the need to gate an expression which would either require including `--cfg IDENTIFIER` with `--cfg IDENTIFIER="<version>"` (like `--cfg rust`) to check for its presence or for us to add an `is_set` predicate.
However, this would also apply to a `version_before` predicate, making `version_before` not the same as `not(version_since)`.
If we did error on unset or name-only `IDENTIFIER`s,
we'd need it to be done lazily so as to not error if the expression is gated.

Deferring the more flexible syntax avoids having to couple this decision to what syntax should be allowed
which will allow us to better evaluate the ramifications for each time we relax things.
For instance, in the [future-possibilities] we go so far as to allow alphabetic characters in any field while making the precision arbitrary.
This can have side effects like allowing comparing words like with `#[cfg(version_since(hello, "world"))]`,
whether intended by the users (potential abuse of the feature) or not (masking errors that could help find bugs).

Deferring `+build` metadata field support for `IDENTIFIER`s value because a non-precedence setting field can cause confusion (as shown in Cargo/crates.io),
its likely best to hold off for us to evaluate the use of it when the need arrives.
Like with Cargo, the `+build` metadata field should probably not be supported in the string literal (version requirement) because it does not affect precedence.

If we were stricter on the syntax,
we could allow for version numbers to be directly accepted, without quotes 
(e.g. `#[cfg(version_since(rust, 1.95.0))]`).
If we ever decided to support operators (e.g.`#[cfg(version_since(rust, "=1.95"))]`, see `--check-cfg`), then we'd need to decide if those also go outside the string or then require a string, being inconsistent.
This may limit us if we decide to allow for alternative version formats like with [target_version](#cfg_target_version) as they may not have formats that map well to `Version`.
Worst case, we'd need to accept arbitrary bare words.
This would also be inconsistent with other uses of `cfg`s
*but* maybe that would just be the start to natively supporting more types in `cfg`,
like integers which are of interest to embedded folks.

A user could do `--cfg=foo --cfg=foo="1.2" --cfg=foo"1.3"`, leading to `cfg` to be a set of:
- `("foo", None)`
- `("foo", "1.2")`
- `("foo", "1.3")`

meaning `cfg(all(foo, foo = "1.2", foo = "1.3"))` is `true`.

We take this into account by checking if any cfg with the name `foo` matches `version_since`.
Alternatively, we could fail the match in this case but that prevents `--cfg rust` for checking if this feature is stable.

## `--check-cfg` rationale

The `--check-cfg` predicate and the value for `rust` ensures users get warnings about
- Invalid syntax
- Using this with versions from before its supported, e.g. `#[cfg(version_since(rust, "1.0")]`

`--check-cfg` requires a `Version`, rather than a version requirement,
in case we want the future possibility of relaxing `Version`
*and* we want to infer from the fields used in `--check-cfg` to specify the maximum number of fields accepted in comparisons.

Like with the cfg's string literal,
check-cfg's string literal does not support the `+build` metadata field as it has no affect on precedence.

We could have the check-cfg `version_since` predicate only apply to the cfg `version_since` predicate,
causing `#[cfg(rust = "1.100")]` to warn.
However,
- the `version_since` predicates are a general feature intended to be used with other version numbers where exact matches may also be appropriate.
- this would get in the way of approximating the vendor version by the language version for working around compiler bugs and snapshotting of compiler output.

Possibly there could be a clippy lint specifically about `rust = "<something>"`.
Alternatively, we could try to find a way to structure `--check-cfg` to allow the person setting the `check-cfg` to decide whether it can be used with `=` or not.
One way of doing this is by allowing the check-cfg `version_since` predicate outside of the `values` predicate,
meaning it works with the cfg `version_since` predicate and not the `=` operator.
Another way would be for the check-cfg `version_since` predicate to never work with `=` but to instead
allow operators inside of the cfg `version_since` predicate, e.g. `#[cfg(version_since(rust, "=1.95"))]`.
However, with the rename of the predicate from `version` to `version_since`, operators don't fit in as easily.
If someone wanted to support equality checks, there wouldn't be a way to support a continuous range of `values()` but would instead have to manually specify each likely potential version.

## `rust` cfg rationale

While there was concern over `rust` appearing in the name of `cfg(rust_version("1.95"))`,
I feel that `rust` as its own entity makes sense and avoids that problem.

Rust does appear in some parts of the language,
but is capitalized like with [`repr(Rust)`](https://doc.rust-lang.org/reference/type-layout.html?#the-rust-representation).
However, the convention for `--cfg`s is generally lower case.

Alternatively, we could call this `rust_version`.
The lack of a qualifier happens to work in this case but that might not be universally true
and adding the qualifier now may improve consistency with the future.

`--cfg=rust` is added to allow `#[cfg(rust)]` checks so packages can immediately adopt this feature without bumping an MSRV.
This does lock us into how a `cfg_value!(rust)` would work from the [future-possibilities].
Alternatively, we could add a separate cfg, like `has_rust`, `rust_is_set`, `has_version_since`.

`--check-cfg` will cause the following to warn:
```rust
fn is_stderr_terminal() -> bool {
    #[cfg(rust)]
    #[cfg(version_since(rust, "1.70"))]
    use std::io::IsTerminal as _;
    #[cfg(rust)]
    #[cfg(not(version_since(rust, "1.70")))]
    use is_terminal::IsTerminal as _;
    #[cfg(not(rust))]
    use is_terminal::IsTerminal as _;

    std::io::stderr().is_terminal()
}
```

To allow checking for the presence of `rust`, add the following to your `Cargo.toml`:
```toml
[lints.rust]
unexpected_cfgs = { level = "warn", check-cfg = ['cfg(rust)'] }
```
Alternatively, we could have the built-in `--check-cfg` for `rust` include `values(none())` but:
- When building on an old version, users will see the warning and will likely want to add it anyways.
- We lose out on `--check-cfg` identifying misuses.
  Instead, we may wish to add a dedicated predicate intended for "is set".
- The lint is an opportunity to tell people how to suppress it in old versions
- However, this does "punish" people who need it but don't care about warnings on old versions

### Patch

The patch field is left off as the Language is not generally affected by patch releases.
We can always add the patch field at a later point.
In the future possibilities for relaxing `Version`,
we allow for variable precision with a max precision determined by what is used in check-cfg.
If we switch check-cfg to use 3 fields,
all 2-3 field precision versions would work.

If nighties are still considered incomplete at that point,
there is the question of how we determine what version to report nightlies as.
We could either track the latest patch release at the time of the nightly, assume `x.y.0`, or assume `x.y.99`.
The compiler may choose to offer an unstable flag to mark a nightly as "complete" to allow for testing of features with `version_since`.

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
- the case of package `bleeding-edge` starting to use a new feature behind `#[cfg(version_since)]` and package `nightly-only` has their toolchain pinned to a nightly before the feature was stabilized (to ensure consistent behavior of unstable features), package `nightly-only` cannot add or update their dependency on `bleeding-edge` without getting a "feature gate needed" error.
- bisecting nightlies.

This was [changed back to complete](https://github.com/rust-lang/rust/pull/81468) after
[some more discussion](https://github.com/rust-lang/rust/issues/64796#issuecomment-634546711).
In particular, this is important for
- keeping friction down for packages preparing for stabilized-on-nightly features as their `#[cfg(version_since)]`s can be inserted and "just work" which can be important for getting feedback quickly while the feature is easier to adapt to feedback that can be gained from these users
  - releasing the package while its in this state puts it at risk to be broken if the feature is changed after stabilization

For RFC 2523, they settled on pre-releases being incomplete,
favoring maintainers to adopt stabilized-on-nightly features immediately
while letting people on pinned nightlies or bisecting nightlies to set a `-Z` to mark the version as incomplete.

Originally, this RFC chose to translate `-nightly` to `-incomplete` because:
- Maintainers can adopt stabilized-on-nightly features with `#[cfg(version_since(rust, "1.100.0-0"))]` (the lowest pre-release for `1.100.0`), keeping friction low while explicitly acknowledging that the unstable feature may change
  - `-0` is recommended over `-incomplete` or any other value as the exact pre-release value is unspecified.
- Allows build scripts to experiment with other logic when approximating the vendor version from the language version with less of a chance of needing to invoke `rustc` (e.g. detecting nightly)
- It provides extra context when approximating the vendor version from the language version when populating build information

We called the pre-release `-incomplete` to speak to the relationship to the language version.
Other terms like `partial` could as easily apply.
The term `-nightly` would be more discoverable but would convey more of a relationship to the vendor than the language.

As for differentiating between nightlies,
that corresponds more to the vendor version than the language version,
so we do not include that information.

However, we've decided to punt on the question of nightlies to reduce the scope of this RFC
and out of concern for published packages using unstable features that will automatically get enabled somehow.

## Alternative designs

### `cfg(rust >= "1.95")`

[RFC #3796](https://github.com/rust-lang/rfcs/pull/3796)
will be allowing operators in addition to predicates and it stands to reason that we can extend that
to version comparisons as well.

The expression `rust >= "1.95"` without any other changes would be a string comparison and not a version precedence comparison.
We'd need to add the concept of types to cfg.
We could make check-cfg load-bearing by relying on its type information
or we could add coercion functions to cfg.

So given `--cfg=rust --cfg=rust=version("1.95")`, you could do `cfg(rust >= version("1.95"))`.

With typing,
`cfg_values!` (a future possibility) could evaluate to the given type.
So for `--cfg foo=integer("1')` (maybe even `--cfg foo=1`), `cfg_value!(foo)` would be as if you typed `1`.
For versions,
as there is no native Rust type,
we'd likely have it evaluate to a `&'static str`.

[RFC #3796](https://github.com/rust-lang/rfcs/pull/3796)
does not address questions around binary operators,
requiring us to work it out.
For example, are the sides of the operator fully swappable?
If we define all comparisons, would `==` be different than `=`?
How will these operators work in the presence of multiple values or a name-only cfg?

Would we allow implicit coercion so you can skip the `version` inside of `cfg`, like `cfg(rust >= "1.95")`?
I would suggest not because this would make it harder to catch bugs where
- The `--cfg` is not a version but you thought it was
- The `--cfg` should be a version but `version()` was left off

Currently, check-cfg does not apply at all to `--cfg` because it is commonly used with `RUSTFLAGS` which
are applied to all packages and would warn that an unknown `IDENTIFIER` is in use for packages that don't care.
We could still skip checking for unknown `IDENTIFIER`s and instead warn on misuse of `IDENTIFIER`s which would increase the chance of catching a mistake (unless a person duplicated there `--cfg` mistake with `--check-cfg`.

Another is how to handle check-cfg.
The proposed syntax is a binary operator but there is no left-hand side in check-cfg.
Would we accept `cfg(rust, values(>="1.95"))`?
How would we specify types?  Would we replace `values` with `versions`?
Or do we deviate from the check-cfg syntax and go with `cfg(rust >= version("1.95"))`?
This would make editions `--check-cfg 'cfg(edition, values(version("2015"), version("2018"), version("2021"), version("2024")))' --check-cfg 'cfg(edition >= version("2025"))'`

Adding typing to cfg,
while likely something we'll do one day,
greatly enlarges the scope of this RFC.
This makes it harder to properly evaluate each part,
making it more likely we'll make mistakes.
This further delays the feature as the unstable period is likely to be longer.
We also are not ready to evaluate other use cases for typing to evaluate the impact
and likely won't until we move forward with [global features](https://internals.rust-lang.org/t/pre-rfc-mutually-excusive-global-features/19618)
and `cfg_values!`,
allowing us to cover use cases like embedded using [toml_cfg](https://crates.io/crates/toml-cfg).

If we defer typing, we'll have to allow implicit coercion of values so we can mark `rust` as a version in the future without it being a breaking change.

If we consider typing the correct long term solution but defer it,
we may want to consider the most narrowly scoped solution in the short term,
like `rust_version("1.95")`.
These "big questions" can then have dedicated issues and versions can be built on top of that.

### `version(rust, ">=1.95")`

Instead of having an assumed operator for the predicate,
we could require an operator or predicate as either:
- `version(rust, ">=1.95")`
- `version(rust >= "1.95")`
- `version(rust, since("1.95"))`

For Cargo, operators do not match pre-release versions unless the operand uses them
though this may be relaxed, see [cargo#14305](https://github.com/rust-lang/cargo/pull/14305).
This does not fit with out use cases because it causes discontinuities
while users of the `cfg` need continuity.

[RFC #3796](https://github.com/rust-lang/rfcs/pull/3796)
does not address questions around binary operators,
requiring us to work it out.
For example, are the operands fully swappable?
If not, that could lead to impedance mismatches for users.
What all operators do we support?
All `Ord` operators increases the scope.
Not having all can lead to impedance mismatches for users.

This splits our specific predicate name (`version_since`) into smaller parts without losing conveyed meaning.
This also provides a natural home for common documentation for cfg version concepts,
if we gain more predicates.

If the operator is outside of the string literal
- we could also make it a bare word but that could lead to problems when dealing with relaxing of the version syntax
- this creates a DSL inside our existing DSL which feels tacked on like using [rustversion](https://crates.io/crates/rustversion)
- We'd need to decide how far to extend this DSL
- We have not considered the syntax implications for check-cfg which would not have a left-hand side for the operator.

If the operator is inside the string literal
- this would feel comfortably familiar due to Cargo
- users may stumble and be frustrated with missing features from cargo (do we include all unary and binary operators?)
- behavior differences with Cargo may be needed due to different use cases but could lead to user bugs and frustration as it might not match what users are familiar with

If we nest `version_since` inside `version`,
- If there is a concern with boundary with `version_since` conditions that aren't alleviated by the discussion elsewhere,
  then this isn't helped because we are still using `version_since`
- It's not clear how a user is expected to reason about this (i.e. how do we teach this?)
  especially in light of how the existing predicates work
- This creates a DSL inside our existing DSL which feels tacked on like using [rustversion](https://crates.io/crates/rustversion)
- Users are likely to hit impedance mismatches between principles they expect to work within the parent DSL and this DSL (e.g. using `all`)
- Nesting APIs puts more of a burden on the user, their editing experience, and our documentation structure to navigate compared to a flat structure
  - If this is just to make the name `version_since` more specific,
    we could just as well be served by naming it `version_version_since`

If we want to consider typed-config in the future (and by extension `cfg(rust >= "1.95")`),
we may want to hold off on the use of operators to give ourselves more flexibility in defining how they work.

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
For example, if Cargo told rustc the minimum compatible version for a dependency, `#[deprecated(version_since)]`` warnings could not emit if the minimum version bound is lower than `version_since`.
Similarly, if we stabilized `#[stable(version_since)]`, a linter could report when a version requirement is too low.

We could rename this to `version` and stabilize it as-is,
with this RFC being a future possibility that adds an optional second parameter for specifying which version is being referred to.

This ends up being a one-off solution,
requiring other one-off solutions for `edition`, [`target_version`](https://github.com/rust-lang/rfcs/pull/3750), etc.

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

Swift:
- Similar syntax, an attribute [`@available`](https://docs.swift.org/swift-book/documentation/the-swift-programming-language/attributes#available) with name/value pairs. Examples: `@available(swift 3.0.2)`, `@available(iOS 10.0, macOS 10.12)`.

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

- `rust` or `rust_version`?
- `--cfg rust` or `--cfg has_rust` for using now without an MSRV bump?
  - Should the `check-cfg` include `values(none())` or not?
- How strict should the version syntax be at this stage?
- `version_since(rust, "1.95")`, `version_version_since(rust, "1.95")`, `version(rust, ">=1.95")`, `version(rust >= "1.95")`, or `version(rust, version_since("1.95"))`
- Is `"1.95.0-incomplete"` an acceptable compromise on the question of whether to treat nightlies as complete or incomplete?
  - How much do we care about the name?
  - Are beta's incomplete?   Strictly speaking, yes.  However, in most cases they will be complete.

# Future possibilities
[future-possibilities]: #future-possibilities

- In the future the `--check-cfg` `version_since()` predicate could make the minimum-version field optional,
  matching all version numbers.
- Adding `#[cfg(version_before("1.95"))]` could resolve the unnatural grammar of `#[cfg(not(version_since("1.95")))]`.
  - Deferring to keep this minimal and to get more real world input on the usefulness of this
  - Another possible name is `#[cfg(version_until("1.95"))]` which reads well as `#[cfg(not(version_until("1.95")))]`

## Relaxing `Version`

Instead of requiring the `IDENTIFIER` in the cfg `version_since` predicate to be strictly `Version` `major.minor`,
we could allow abbreviated forms like `major` or more precision like `major.minor.patch`.
This would make the predicate more inclusive for other cases, like `edition`.

The syntax for a version could be:
```
Version ->
  ReleaseVersion
  PrereleaseVersion?
  BuildMetadata?

ReleaseVersion ->
  VersionField
  ( `.` VersionField)*

PrereleaseVersion ->
  `-`
  VersionField
  ( `.` VersionField)*

BuildMetadata ->
  `+`
  VersionField
  ( `.` VersionField)*

VersionField -> ( NumericVersionField | AlphanumericVersionField )

NumericVersionField ->
    `0`
  | ( [`1`..`9`] DEC_DIGIT* )

AlphanumericVersionField -> (
      DEC_DIGIT
    | [`a`..`z`]
    | [`A`..`Z`]
    | `-`
  )+
```

With the precedence of:
- Precedence is calculated by separating the `Version` into the respective `VersionField`s, ignoring `BuildMetadata`
- Precedence is determined by the first difference when comparing each field from left to right of `ReleaseVersion`
  - `NumericVersionField` is compared numerically
  - `AlphanumericVersionField` is compared lexically in ASCII sort order
  - Numeric identifiers always have lower precedence than non-numeric identifiers
  - When two versions have different number of fields, the missing fields are assumed to be `0`
- When the two `ReleaseVersion`s are equal, a `Version` with a `PrereleaseVersion` has lower precedence than one without
- Precedence for two `Version`s with the matching `ReleaseVersion`s but different `PrereleaseVersion`s is determined by the first difference when comparing each field from left to right of `PrereleaseVersion`
  - `NumericVersionField` is compared numerically
  - `AlphanumericVersionField` is compared lexically in ASCII sort order
  - Numeric identifiers always have lower precedence than non-numeric identifiers
  - `PrereleaseVersion` with more `VersionField`s has a higher precedence than one with less, if all of the preceding `VersionField`s are equal.

This was adopted from [SemVer](https://semver.org/) with the following changes:
- Arbitrary precision for `ReleaseVersion`
  - Unlike `PrereleaseVersion`, missing fields is assumed to be `0`, rather than lower precedence
- Alphanumerics are allowed in release version fields

Compared to the base `Version` syntax, this adds back from SemVer:
- pre-release versions
- build metadata

The version requirement (string literal) for cfg `version_since` and check-cfg `version_since` would be similarly updated
except  `BuildMetadata` would not be allowed.
A user would see the `unexpected_cfgs` lint if their cfg `version_since` string literal had more precision (more `VersionField`s) than the check-cfg `version_since` predicate.

Note: for `--cfg foo="bar"`, `"bar"` would be a valid version.

We could always relax this incrementally, e.g.
- Variable precision for `edition`
- `BuildMetadata` for dependency versions
- Whatever `target_version` requires

## Incomplete language versions

Rustc could indicate that it implements an incomplete version of the compiler by having an `-incomplete` pre-release field.
This would be used for nightlies and there would be a question of whether beta should be incomplete or not.

This would build ion the above relaxing of `Version` for at least being able to specify pre-release versions.

This can be done later as its unstable.

A guide-level explanation would be:

> Say you were wanting to test out `#[must_use]` after it got stabilized on nightly to provide feedback and to be ready
> for when it hits stable,
> you would instead use `"1.27.0-0"` to match all pre-release versions of 1.27.0:
> ```rust
> #[cfg_attr(version_since(rust, "1.27.0-0"), must_use)]
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

## `--cfg edition`

In adding a `cfg` for the Edition, we could model it as either:
- An integer
- A single-field version

Assuming the latter,
we could have the following definition, building on the above relaxing of `Version` for at least variable alternative precision:

`--cfg edition="<year>"`

`--check-cfg cfg(edition, values(2015, 2018, 2021, 2024, version_since(2025)))`
- The discrete values for known editions is there to help catch mistakes
- `version_since(2025)` is used so packages don't have to deal with `unexpected_cfgs` when operating with edition versions higher than their current compiler recognizes and without having to try to predict what our future edition versions and policies may be
- `version_since(2025)` also ensures that a user gets an `unexpected_cfgs` warning if they do `cfg(version_since(edition, 2028.10))` as that matches the `version_since(2025)` but has more precision

## `cfg_target_version`

Instead of defining a new `#[cfg]` predicate, [RFC 3750](https://github.com/rust-lang/rfcs/pull/3750)
could reuse the `#[cfg(version_since)]` predicate.

Building on the above relaxing of `Version`, we should meet the needs of most versioning systems.
The one known exception is "post releases"
(e.g. [`1.2.0.post1`](https://packaging.python.org/en/latest/discussions/versioning/)
which, if we translated it to `Version`'s syntax of `1.2.0-post1`, would be treated as a pre-release.
We can translate this to extra precision, e.g. `1.2.0-post1` could be `1.2.0.post.1`.
This would require the check-cfg `version_since` to use the appropriate amount of precision to not warn.

If this is still not sufficient, we some options include:
- Add an optional third field for specifying the version format (e.g. `#[cfg(version_since(windows, "10.0.10240", <policy-name>)]`)
- Make `--check-cfg` load-bearing by having the version policy name be specified in the `--check-cfg` predicate

## Conditional compilation for dependency versions

As the ecosystem grows and matures,
the Rust language and standard library may not be the only dependencies users wish to support multiple versions of.
We may want to allow `#(cfg(version_since(serde, "1.0.900")]`.

As dependency versions can have a `+build` metadata field,
we'd need to decide whether to further relax version numbers by allowing a `+build` metadata field
which would not affect precedence or whether the caller is responsible for stripping them,
losing potential release information.

## Vendor name and version

We could add `--cfg`s for the compiler vendor name and version.
In addition to the use cases given in the Motivation section,
this will allow users to check for specific nightly versions.

Some challenges for this with `rustc --version`:
- Nightly versions for a given release are mutable,
  all mapping to the `-nightly` pre-release version rather than including the date within the pre-release
- This does not conform to `Version`'s relaxed precedence rules,
  as `-nightly` is an older version than `-beta.4` while `Version`s relaxed precedence rules say the opposite
- Crater runs and local builds don't necessarily have a version that fits within this picture

## `#[cfg(nightly)]`

Depending on what is meant by this,
we either need the language version or the vendor name and version as well as a way to check for the presence of `pre-release`.

See also [`#[cfg(nightly)]`](https://rust-lang.github.io/rfcs/2523-cfg-path-version.html#cfgnightly) in the previous RFC.

## Provide a way to get a `--cfg`s value

Use cases:
- Allows application to use `rust` to approximate the vendor version in `--bugreport` / `-v --version` without a build script.
  As other versions get represented in `cfg`, this may be desired for the same reason.
- See also [mutually exclusive features](https://internals.rust-lang.org/t/pre-rfc-mutually-excusive-global-features/19618) for `cfg_value!`,

Similar to how `cfg!` allows doing conditionals in Rust code,
provide macros for reading the values set for a `cfg`.

The most general form maybe `cfg_values!(foo)` but a `cfg_value!(foo)` could offer some convenience.

Open questions:
- How does `cfg_values!(foo)` deal with unset and name-only cfg's?
  - Most strict would be `Iterator<Option<&'static str>>`, requiring users to do `cfg_values!(foo).filter_map(std::convert::identity)` in most cases
  - Could auto-skip name-only.  Empty iterator would then be ambiguous.
- How does `cfg_value!(foo)` deal being unset?
  - Compiler error, like `env!`.  Could provide an `option_cfg_value!`.
- How does `cfg_value!(foo)` deal with name-only cfgs??
  - Ignoring them would work best for the purpose of `--cfg=rust --cfg=rust="1.95"`
- How does `cfg_value!(foo)` deal with multiple cfg vales?
  - Compiler error

## `check-cfg` support for a version without a minimum

`--check-cfg 'cfg(foo, values(version_since("1.95")))'` requires setting a minimum version.
If a user did not need that when setting a `cfg`,
they would have to do `--check-cfg 'cfg(foo, values(version_since("0.0.0-0")))'`.
A user may want a shorthand for this.
With the name `version_since`, defaulting it to `"0.0.0-0"` doesn't read too well (--check-cfg 'cfg(foo, values(version_since()))'`).
Maybe a new predicate can be added `version()`.
A shorthand may be limited to `Version` versions if we use the `version_since(version)` syntax to specify the supported version syntax, see [`--check-cfg` rationale][#--check-cfg-rationale].

## An `is_set` predicate

There isn't a way to check if a `cfg` name is set, whether with or without values
which would work like a `cfg` version of
[`cfg_accessible`](https://dev-doc.rust-lang.org/stable/unstable-book/library-features/cfg-accessible.html)
so long as the `cfg` is unconditionally set.

## Clippy lint: prefer using `cfg(version_since)` via `cfg_alias!`

Once we have this RFC and [RFC 3804](https://github.com/rust-lang/rfcs/pull/3804),
we may want a restriction lint that would encourage people to use `cfg(version_since)` through `cfg_alias`
so people work off of names in `cfg()` throughout their code base,
rather than versions that may be duplicated and lack semantic meaning.

This will also make the more awkward name of `version_since` less of an issue.
