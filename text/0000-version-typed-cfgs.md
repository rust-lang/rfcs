- Feature Name: version_cfgs
- Start Date: 2026-01-06
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC proposes "typed `cfg`s", a new form of conditional compilation predicate that understands types. Initially, this RFC proposes to add support for version-typed `cfg`s, allowing for ergonomic version comparisons against the *language version* supported by the compiler. This would be exposed through two new built-in `cfg` names:

*   `rust_version`, which can be compared against a language version literal, e.g., `#[cfg(rust_version >= "1.85")]`.
*   `rust_edition`, which can be compared against an edition literal, e.g., `#[cfg(rust_edition >= "2024")]`.

This design solves a long-standing problem of conditionally compiling code for different Rust versions without requiring build scripts or forcing libraries to increase their Minimum Supported Rust Version (MSRV). It also replaces the `cfg(version(..))` part of RFC 2523.

# Motivation
[motivation]: #motivation

There are two primary use cases for conditional compilation based on versions, whether that is the version of the Rust language or the version of a dependency:

1.  **Exposing new features** when code is compiled with newer versions of the language or library while maintaining compatibility with older versions.
2.  **Straddling a breaking change** in an API, allowing code to compile against both the old and new versions, which are mutually incompatible.

The `rust_version` cfg proposed in this RFC is designed for the first use case, while `rust_edition` is a tool for the second. This RFC also provides a general mechanism for `version`-typed `cfg`s that can be used for libraries, which addresses both use cases.

This RFC also aims to solve long-standing problems that have been approached with more specific RFCs in the past. For example, the `cfg_target_version` RFC ([#3750](https://github.com/rust-lang/rfcs/pull/3750)) proposed a way to compare against the version of the target platform's SDK (e.g., `#[cfg(target_version(macos >= "10.15"))]`). This is crucial for safely using platform-specific APIs that are only available in newer OS versions. Instead of a one-off feature, the general `version` type proposed in this RFC provides a robust foundation to solve this problem. A build script could detect the platform version and emit a `version`-typed cfg (`--cfg 'macos_version=version("10.15")'`), allowing for the same ergonomic comparisons in code: `#[cfg(macos_version >= "10.15")]`. These platform-specific version keys can be added into the language in future RFCs.

The only stable tool for this today is a build script (`build.rs`). However, build scripts add significant compilation overhead and are clunky to write and maintain.

RFC [#2523](https://rust-lang.github.io/rfcs/2523-cfg-path-version.html) tried to solve this, but ran into an unfortunate issue: its proposed syntax, e.g. `#[cfg(version(1.85))]`, was a syntax error on older compilers. This means that to use the feature, a library would first have to bump its MSRV to the version that introduced the syntax, somewhat defeating the primary purpose of the feature. If we knew this was the syntax we wanted going forward, this tradeoff might be worth it. But on close inspection the earlier RFC, which merged in 2019, had left the syntax question undecided due to this very issue, and the current lang team did not have consensus on the syntax used in the RFC.

This RFC proposes a solution that avoids these pitfalls, solves related versioning problems besides just the Rust version, and builds a scaffolding for related `cfg` features we might add in the future.

One motivating example is making it ergonomic to adopt attributes that were stabilized after a crate's MSRV. For example, the `#[diagnostic::on_unimplemented]` attribute is a stable feature that library authors can use to provide better error messages. However, if a library has an MSRV from before this attribute was stabilized, they cannot use it without a build script. A build script is often too much overhead for such a small, non-essential feature.

This RFC makes it trivial to adopt even in a crate that doesn't want to use a build script. In this case, since the diagnostic attribute namespace landed before `rust_version`, you would write

```rust
#[cfg_attr(rust_version, diagnostic::on_unimplemented(
    message = "`{Self}` does not implement `MyTrait`",
    label = "missing implementation for `{Self}`",
))]
impl<T> MyTrait for T { /* ... */ }
```

With this feature we hope to see more people using useful attributes like `on_unimplemented`, even with MSRVs before when the diagnostic attribute namespace was added. Gated `diagnostic` attributes like this will not be active until the Rust version where this feature ships, but adding them still adds value. While some crates must hold a low MSRV to allow building in environments with older compilers, like Linux distros, most active Rust development still takes place on recent compiler versions. Using this gating mechanism will mean that most users of a crate benefit from the attributes, without changing the crate's MSRV or adding a build script.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

If your crate's MSRV is at least the version where typed `cfg`s were stabilized, you can directly use the version comparison. For example, imagine a new function `pretty_print()` is stabilized in Rust 1.92:

```rust
fn print_something() {
    #[cfg(rust_version >= "1.92")]
    pretty_print();
}
```

`rust_version` also allows the use of these predicates while maintaining a lower MSRV than the version `rust_version` itself ships in. The key is to first check for the existence of the `rust_version` configuration itself before trying to use it in a comparison.

```rust
fn print_something() {
    #[cfg(rust_version)]
    #[cfg(rust_version >= "1.92")]
    pretty_print();

    #[cfg_attr(rust_version, cfg(rust_version < "1.92"))]
    println!("something less pretty");
}
```

This chained config pattern is only necessary when your MSRV straddles both the `rust_version` feature and a new feature that shipped after it.

Similarly, you can check the Rust edition:

```rust
#[cfg(rust_edition >= "2021")]
fn my_function() {
    // use a feature only available from the 2021 edition onwards
}
```

Note that because new compilers can still compile older editions, the `#[cfg(rust_edition)]` stacking pattern is less useful than it is for `rust_version`. The primary use case for rust_edition is within macros or code generation that needs to produce different code depending on the edition context it's being expanded into.

For this RFC, the only supported comparison operators are `>=` and `<`.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This RFC adds a new kind of predicate to the `cfg` attribute, allowing for comparison against version identifiers.

### Version predicates

A version predicate is available for `cfg` identifiers that are declared to be of type `version`.

The grammar for `cfg` option predicates will be expanded to the following:

```text
ConfigurationOption ->
    IDENTIFIER
  | IDENTIFIER ConfigurationComparison ( STRING_LITERAL | RAW_STRING_LITERAL )

ConfigurationComparison ->
    `=`
  | `<`
  | `>=`
```

The `<` and `>=` comparisons are only valid when the `IDENTIFIER` on the left-hand side names a defined `cfg` option of type `version`.
The `=` comparison is only valid when the option is undefined or of type `default`.

A `cfg` identifier is of type `version` if:
*   It is one of the built-in identifiers `rust_version` or `rust_edition`.
*   It is declared with `--check-cfg 'cfg(name, version)'` and is passed to the compiler with the special syntax `--cfg 'name=version("...")'`.

The `ident` must be a known `cfg` identifier of type `version`. The `literal` must be a string literal that represents a valid version.

A `version` predicate evaluates to `true` if the comparison is true, and `false` otherwise. If the identifier is not a known version-typed `cfg`, or the literal is not a valid version string, a compile-time error is produced.

```rust
#[cfg(rust_version >= "1.90")]
fn new_impl() { /* ... */ }

#[cfg(rust_version < "1.90")]
fn old_impl() { /* ... */ }
```

#### Version Literals
The `STRING_LITERAL` in a version predicate must conform to the following grammar:

```text
version_literal :
    NUMERIC_COMPONENT ('.' NUMERIC_COMPONENT)*

NUMERIC_COMPONENT :
    '0'
  | ('1'...'9') ('0'...'9')*
```

This grammar defines a version as one or more non-negative integer components separated by dots. Each component must not have leading zeros, unless the component itself is `0`. For example, `"1.90"` and `"0.2.0"` are valid, but `"1.09"` is not.

There is a single, unified parsing and comparison logic that is part of the language's semantics. Additional checks for the built-in version keys are implemented as lints.

*   The comparison is performed component-by-component, filling in any missing components with `0`. For example, a predicate `my_cfg >= "1.5"` will evaluate to true for versions `1.5.0`, `1.6.0`, and `2.0`, but false for `1.4.9`.
*   Pre-release identifiers (e.g., `"1.92-beta"`) are ignored during comparison and a lint will be emitted. The comparison acts as if the pre-release was not specified. See the "Unresolved Questions" section for further discussion.

Using version-typed config values with the `=` predicate results in a hard error.

### Version-typed cfgs as options

When cfg option with a version type and value is used as a bare option, it evalutes to true:

```rust
#[cfg(rust_version)]
fn new_impl() { /* compiles */ }
```

### Builtin version-typed cfgs

#### `rust_version`

The `rust_version` cfg is version typed and contains two components (major and minor version). This may be expanded to all three components in the future.

A lint will be issued if `rust_version` is compared to more than two components (e.g., `"1.92.0"`) of a version equal to or earlier than the current compiler. This is because language features should not depend on patch releases. However, we only lint on "known" versions in case we decide to include all three components in the future.

A new lint warns for version checks that are logically guaranteed to be true or false (e.g., `rust_version >= "1.20"` when the feature was stabilized in 1.90). This lint may be expanded to include user-defined cfgs when check-cfg supports specifying useful ranges.

##### Pre-releases

This RFC does not specify how "nightly" compilers with pre-release versions of the language are handled. That may change without breaking Rust's stability guarantees.

_Note:_ The history of this question is [covered in RFC 3857](https://github.com/rust-lang/rfcs/blob/4551bbd827eb84fc6673ac0204506321274ea839/text/3857-cfg-version.md#pre-release).

#### `rust_edition`

The `rust_edition` cfg is version typed and contains one component, the year of the edition.

A lint will be issued if the literal of a known edition has more than one component, or if we know the value is never going to be a Rust edition (for example, `"2019"`).

### Defining version-typed configs

To define a `version`-typed `cfg`, the following syntax must be used:

```sh
--cfg 'my_app_version=version("2.1.0")'
```

This can also be used to override built-in version cfgs (e.g. `--cfg 'rust_version=version("1.50.0")'`), which is primarily useful for testing.

* If a version cfg is used with a string literal in a comparison that is not a valid version string, a hard error is emitted.
* If a cfg that is not a version type is used in a version comparison, a hard error is emitted. For undefined cfgs, this could be relaxed to evaluate to false in the future.
* If a cfg that is a version type is used in a non-version comparison (`=`), a hard error is emitted.
* Version typed cfgs are single-valued. Setting more than one value for the flag is a hard error. This includes values of other types, so given the example above, adding both `--cfg my_app_version` and `--cfg my_app_version="foo"` would cause a hard error.
* Setting the _same_ value multiple times on the command line should also be a hard error initially. This is a conservative choice that the compiler team may choose to relax, e.g. for build system integration reasons.

Configs defined using the existing command-line syntax `--cfg 'name="value"'` have the `default` config type. The name of this type is not user-facing and may change.

### Interaction with other compiler flags

*   **`--check-cfg`**: To inform the compiler that a `cfg` is expected to be a version, and to enable linting, use:
    ```sh
    --check-cfg 'cfg(my_app_version, version())'
    ```

    This will accept any version value, but lint when the option is used in a non-version comparison (note that this is an error if the option actually has a version-typed value). This is a more sensible default for versions, which don't have the equivalent of `values(none())`.

*   **`--print cfg`**: User-defined version cfgs are printed in the `name=version("...")` format. Whether to print the built-in `rust_version` and `rust_edition` cfgs is left as an unresolved question to be determined based on tool compatibility. In future editions, the builtin cfgs should always be printed.
    *   Note: Using editions being careful about passing `--edition` to `rustc --print cfg` invocations, which `cargo` for example does not currently do. This could introduce unexpected inconsistencies.

*   **`--print check-cfg`**: The built-in `rust_version` and `rust_edition` cfgs are implicitly included, so `rustc --print=check-cfg` will always list them. We can add these immediately because `--print check-cfg` is unstable.

### Stabilization

The features described in this RFC may be stabilized in phases:

1.  The initial stabilization can include the built-in `rust_version` and `rust_edition` cfgs and the ability to compare them with `>=` and `<`.
2.  The ability for users to define their own `version`-typed `cfg`s via `--cfg` and `--check-cfg` can be stabilized later.

This approach delivers the most critical functionality to users quickly, while allowing more time to finalize the design for user-defined version predicates.

### Lint names

* `useless_rust_version_constraint` (deny by default): A Rust version constraint will always be true or false because it names a version prior to when this feature stabilized.
* `version_constraint_unknown_version` (warn by default): A `rust_edition` or `rust_version` constraint falls outside the set of values known to this compiler. This includes future versions.
* `version_constraint_wrong_precision` (warn by default): A version constraint's precision level falls outside the expected range, e.g. `rust_version >= "1"` or `rust_edition >= "2015.0"`.

Note that the first lint is specific to `rust_version`, while the remaining lints can be generalized to any version constraint. The reason to have a lint specific to `rust_version` is that we can say for certain whether it will be true, which can impact the severity of the lint.

This section is subject to change prior to stabilization.

# Drawbacks
[drawbacks]: #drawbacks

- Making the perfect the enemy of the good. RFC 2523 was accepted, and an implementation of its `version()` predicate is ready.
- Increased compiler complexity. This introduces a new concept of "typed" `cfg`s into the compiler, which adds complexity to the parsing and evaluation logic for conditional compilation.
- Subtlety of MSRV-preserving patterns: The need for the "stacked `cfg`" pattern (`#[cfg(rust_version)] #[cfg(rust_version >= ...)]` and `#[cfg_attr(rust_version, cfg(rust_version >= ...))]`) is subtle. While we will add lints to guide users, it's less direct than a simple predicate. However, this subtlety is the explicit tradeoff made to achieve MSRV compatibility.
- The "stacked `cfg`" pattern does not work inside Cargo, so users will not be able to use this feature in Cargo until their MSRV is bumped. For cases where a dependency needs to be conditional on the Rust version, one can define a "polyfill" crate and make use of the MSRV-aware feature resolver, like the `is_terminal_polyfill` crate does.
- Conditional compilation adds testing complexity. In practice, most crate maintainers only test their MSRV and the latest stable.
- This does not support branching on specific nightly versions. rustversion supports this with syntax like `#[rustversion::since(2025-01-01)]`.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

### Why this design?
The syntax `rust_version >= "1.85"` is highly intuitive and directly expresses the user's intent. It is a general design that can be used to solve an entire class of adjacent problems, including platform versioning. It is a principled design, as by introducing a `version` type to the `cfg` system, we create a sound basis for comparison operators and other config types in the future. The syntax avoids the semantic confusion of proposals like `rust_version = "1.85"` which would have overloaded the meaning of `=` for a single special case.

This design directly solves the MSRV problem in a way that RFC 2523 did not. The fact that crates maintaining an MSRV will be able to adopt it for newer version constraints buys back some of the time that was spent designing and implementing the newer iteration of this feature.[^buy-back] While sometimes it is better to ship something functional quickly, the fact that users have an functional workaround in the form of build scripts pushes the balance more in the direction of waiting to deliver a high quality solution.

Single-valued config types give us a chance to revisit some earlier decisions like the use of `=` in predicates. For now these are a hard error. Future extensions might add `==` comparisons with a more natural meaning for single-valued configs.

[^buy-back]:
    A quick [sample][crate-sample] of two MSRV-preserving popular crates that already make use of feature gating, serde and proc-macro2, showed that those crates would be able to drop their build scripts roughly **a year earlier** with a solution that did not break MSRV compatibility. Obviously, this analysis is incomplete, but it has the benefit of emphasizing popular crates that show up in the critical path of many build graphs.

    Shipping an MSRV-incompatible feature sooner would allow immediate use by non-MSRV-preserving crates. Picking the MSRV-compatible option later allows crates that do not make use of feature gating with build scripts today to begin feature gating as soon as `rust_version` ships, without introducing build scripts and without bumping their MSRV.

[crate-sample]: https://github.com/rust-lang/rust/pull/141766#issuecomment-2942369855

### Alternative 1: `#[cfg(version(1.85))]` (RFC 2523)
This was the original accepted RFC for version-based conditional compilation.

#### Rationale for not choosing
The syntax of this RFC was [left as an open question](https://github.com/rust-lang/rfcs/pull/2523#discussion_r326361347) by the RFC author after a concern was raised by the maintainer of the libc crate about the MSRV issue. Since then, the lang team has not been able to reach a consensus on the syntax. Several problems have been identified:

* The word `version` does not sufficiently communicate that it's the Rust version we're talking about.
* The mechanism is special-purpose and geared toward one use case (detecting the Rust version).
* The function-call syntax, chosen for consistency with `cfg(accessible())`, isn't obvious enough in its meaning and does not cleanly extend to new kinds of comparisons. A recent poll of the lang team showed that most people opposed extending that syntax to include other kinds of comparisons within the quotes, like `version("< 1.2.3")`. At the same time, it adds another level of nested parantheses, which can be hard for humans to parse.
* Crates supporting old MSRVs won't be able to use the feature until bumping their MSRV.
* The RFC was accepted more than 6 years ago. During this time we've learned about more adjacent use cases and directions we would like to evolve the language. If designed today, the feature would look much more like this RFC than RFC 2523.

### Alternative 2: `#[cfg(rust_version = "1.85")]` (meaning `>=`)
This syntax is parseable by older compilers, which is a significant advantage for MSRV compatibility.

#### Rationale for not choosing
The use of `=` was highly controversial. In Rust today, the `cfg` syntax has two conceptual models: "set inclusion" for multi-valued cfgs (e.g., `feature = "serde"`) and "queries" for boolean flags (e.g., `unix`). However, people tend to think of `=` more as "equality" than as "set inclusion", and the use of `=` for versions strongly implies exact equality (`== 1.85`).

Likening `>=` to set inclusion makes sense in the narrow context of Rust versions, which do not have semver-incompatible changes, but it does not generalize well to versions that do. Checking the compiler version is is really a comparison between two values, not a check for inclusion in a set.

The interaction with `--print cfg` was unclear. See [RFC 3857](https://github.com/rust-lang/rfcs/blob/4551bbd827eb84fc6673ac0204506321274ea839/text/3857-cfg-version.md#cfgrust--195-1) for more context.

#### Advantage
This approach *could* potentially be made to work inside `Cargo.toml` (e.g., for conditional dependencies), which currently cannot use the stacked-cfg trick. However, the disadvantages in terms of semantic clarity for the language itself outweigh this benefit for an in-language feature.

### Alternative 3: `#[cfg(version_since(rust, "1.85"))]` (RFC 3857)
This alternative also avoids the MSRV problem and is extensible, similar to the current proposal.

#### Rationale for not choosing
While a good design, the "typed cfgs" approach with an actual comparison operator (`>=`, `<`) is arguably more natural and ergonomic. A language team poll indicated a preference for `rust_version >= "1.85"` if it could be made to work. This RFC provides the mechanism to make it work in a principled way.

The next runner up in the poll[^condorcet] was a tweaked version of this syntax, `version(rust, since = "1.85")`, and this version may have been accepted. However, the RFC was closed by the author before this change was made. After making some effort to resurrect it, the author of the current RFC decided to pursue the direction in this RFC instead.

[^condorcet]:
    The poll was conducted using a [Condorcet voting method](https://en.wikipedia.org/wiki/Condorcet_method) that asked team members to rank their choices from most to least preferred. This runner up did not represent the preferred syntax of every individual on the team.

# Prior art
[prior-art]: #prior-art

_Parts of this section are adapted from [RFC 3857](https://github.com/rust-lang/rfcs/pull/3857)._

## Rust Ecosystem

There are very widely used crates designed to work around the lack of native version-based conditional compilation. These rely on build scripts to detect the compiler version and set custom `cfg` flags. `rustversion` also has a proc macro component for the nicest user experience.

-   **`rustversion`**: A popular proc-macro (over 260 million downloads) that allows checks like `#[rustversion::since(1.34)]`. It supports channel checks (stable, beta, nightly), equality, and range comparisons.
-   **Build Script Helpers**: Crates like **`rustc_version`** and **`version_check`** are widely used in `build.rs` scripts to query the compiler version and emit `cargo:rustc-cfg` instructions. They provide programmatic access to version components, channels, and release dates.
-   **Release Info**: Crates like **`shadow-rs`** and **`vergen`** expose build information, including the compiler version, to the compiled binary.
-   **Polyfills**: Some crates, like `is_terminal_polyfill`, maintain separate versions for different MSRVs, relying on Cargo's [MSRV-aware resolver](https://rust-lang.github.io/rfcs/3537-msrv-resolver.html) to select the correct implementation.

This RFC aims to obviate the need for these external dependencies for the common case of checking the language version, reducing build times and complexity.

## Cargo

- **`rust-version`:** The `[package]` section of `Cargo.toml` can specify a `rust-version` field. This allows Cargo to select appropriate versions of dependencies and fail early if the compiler is too old. However, it does not provide fine-grained, in-code conditional compilation. This RFC brings a similar capability directly into the language, but for controlling code within a crate rather than for dependency resolution.

## Other languages

- **Swift (`#if compiler`, `#if swift`)**: Swift provides platform conditions for both the compiler version and the language mode.
    - `compiler(>=5)` checks the version of the compiler.
    - `swift(>=4.2)` checks the active language version mode.
    - These support `>=` and `<` operators, similar to this proposal.

- **C++ (`__cplusplus`)**: The C++ standard defines the `__cplusplus` macro, which expands to an integer literal that increases with each new version of the standard (e.g., `201103L` for C++11, `202002L` for C++20). This allows for preprocessor checks like `#if __cplusplus >= 201103L`. This is very similar to the `rust_version >= "..."` proposal in that it uses standard comparison operators against a monotonically increasing value. However, it is less granular, as several years pass between new C++ versions.

- **Clang/GCC (`__has_feature`, `__has_attribute`)**: These function-like macros allow for checking for the presence of specific compiler features, rather than the overall language version. For example, `__has_feature(cxx_rvalue_references)` checks for a specific language feature. This approach is more granular but also more verbose if one needs to check for many features at once. This approach was discussed in RFC #2523, but rejected, in part because we wanted to reinforce the idea of Rust as "one language" instead of a common subset with many compiler-specific extensions.

- **Python (`sys.version_info`)**: Python exposes its version at runtime via `sys.version_info`, a tuple of integers `(major, minor, micro, ...)`. Code can check the version with standard tuple comparison, e.g., `if sys.version_info >= (3, 8):`. This component-wise comparison is very similar to the logic proposed in this RFC. However, because Python is interpreted, a file must be syntactically valid for the interpreter that is running it, which makes it difficult to use newer syntax in a file that must also run on an older interpreter. Rust, being a compiled language with a powerful conditional compilation system, does not have this limitation, and this RFC's design takes full advantage of that.

## Versioning systems

Not every system uses Rust's standard three-part semver versioning scheme, but many are close. In this section are examples of more bespoke versioning systems that this feature can accommodate. The "escape hatch" for when your version numbers are not semver like is to split them into different version cfgs, each of which is semver like (in the simplest case, just a number).

- **Chromium**: Chromium's version format is a four-part number: MAJOR.MINOR.BUILD.PATCH, where MAJOR increments with significant releases, MINOR is often 0, BUILD tracks trunk builds, and PATCH reflects updates from a specific branch, with BUILD and PATCH together identifying the exact code revision. (Thanks to Jacob Lifshay [on github](https://github.com/rust-lang/rfcs/pull/3905#discussion_r2666956191).)

### Operating systems

Operating systems include many versions, including kernel versions, public OS version, and system API versions. Usually API versions are the most relevant for conditional compilation. Most APIs are preserved across versions. Some operating systems, like Windows, prioritize backward compatibility of applications, while others balance backward compatibility with the deprecation and removal of APIs.

- **Windows API version**: XP is 5.1, Vista is 6.0, 7 is 7.0, 7 with Service Pack 1 is 7.1, 8 is 8.0, 8.1 is 8.1 and Windows 10/11 currently ranges from 10.0.10240 to 10.0.26200. There are "friendlier" names such as 1507 for 10.0.10240 but I think those are better done as some kind of cfg alias rather than being built-in. (Thanks to Chris Denton [on zulip](https://rust-lang.zulipchat.com/#narrow/channel/213817-t-lang/topic/cfg.28version.28.2E.2E.29.29.20as.20a.20version.20comparison.20predicate/near/540907580).)
- **Android API level**: Single, sequential integer value like "34", "35".
- **macOS version**: Based on the operating system version; there is not a separate API level concept. In general these use multi-part versions like "10.15". Starting with macOS 11.0, the major version number has increased with each new version and the second component has been 0.
- **Fuchsia API version**: Single number like "30", similar to Android, but released on a cadence closer to Rust's 6-week release cadence. Fuchsia itself uses Rust along with some build system hacks to express predicates like `fuchsia_api_level_less_than = "20"`. (Thanks to Hunter Freyer [on zulip](https://rust-lang.zulipchat.com/#narrow/channel/213817-t-lang/topic/cfg.28version.28.2E.2E.29.29.20as.20a.20version.20comparison.20predicate/near/539610890).)

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- How should pre-release identifiers in version strings be handled? This RFC proposes not supporting pre-release identifiers in version strings passed on the command line for now. For comparisons, this RFC proposes that if a pre-release identifier is present in a `cfg` predicate (e.g., `rust_version < "2.0-alpha"`), the pre-release part is ignored for the comparison (so it's treated as `2.0`), and a lint is emitted. This ensures forward compatibility, as comparisons like `cfg(all(foo >= "2.0-alpha", foo < "2.0"))` become trivially false on older compilers, which is a safe outcome. This behavior can be refined before stabilization.
- Should the builtin `rust_version` and `rust_edition` be printed with `--print cfg` on the command line? We'd like the eventual answer to be "yes", but existing tools that parse the output might break with the new `rust_version=version("1.99")` syntax. If we can manage the breakage we should; otherwise we can gate it on a future edition.

# Future possibilities
[future-possibilities]: #future-possibilities

- **More expressive check-cfg:** We can support specifying an expected number of components in check-cfg, or an expected set of values to compare against, as in editions:
    - `--check-cfg 'cfg(foo, version("2018", "2022", "2025"))'`
    - `--check-cfg 'cfg(foo, version(values >= "1.75"))'`
    - `--check-cfg 'cfg(foo, version(components <= 2))'`
- **"Compatible-with" operator:** We could introduce a `~=` operator that works like Cargo's caret requirements. For example, `cfg(some_dep ~= "1.5")` would be equivalent to `cfg(all(some_dep >= "1.5", some_dep < "2.0"))`. The rationale for not doing this now is that it's easy enough to write by hand.
- **More comparison operators:** While this RFC only proposes `>=` and `<`, the underlying `version` type makes it natural to add support for `<=`, `==`, `!=`, etc., in the future.
- **Pre-releases:** The version string parsing could be extended to support pre-release identifiers (`-beta`, `-nightly`), though this adds complexity to the comparison logic. RFC 3857 discusses this possibility for [generic versions](https://github.com/rust-lang/rfcs/blob/4551bbd827eb84fc6673ac0204506321274ea839/text/3857-cfg-version.md#relaxing-version) as well as for the [language itself](https://github.com/rust-lang/rfcs/blob/4551bbd827eb84fc6673ac0204506321274ea839/text/3857-cfg-version.md#pre-release).
- **Dependency Version `cfg`s:** The "typed `cfg`" infrastructure could be extended to query the versions of direct dependencies, e.g., `#[cfg(serde >= "1.0.152")]`. This would require significant integration with Cargo.
- **System library versions supplied by `sys` crates:** Cargo could allow `sys` crates to expose the versions of their system libraries to dependents as version-typed cfgs.
- **Other `cfg` types:** We could introduce other types, such as integers or single-valued strings. This could be useful for a variety of features, from system library versioning schemes ([kconfig](https://docs.kernel.org/kbuild/kconfig-language.html)) to enabling things like [mutually exclusive global features](https://internals.rust-lang.org/t/pre-rfc-mutually-excusive-global-features/19618).
- **Namespaced `cfg`s:** We could group Rust-specific `cfg`s under a `rust::` namespace, e.g., `#[cfg(rust::version >= "1.85")]`. This RFC intentionally keeps `rust_version` at the top level to simplify the initial implementation and stabilization, but namespacing could be explored in the future to better organize the growing number of built-in `cfg`s.
- **Macro that evaluates to a cfg value:** We can add a `cfg_value!()` macro for single-valued configs that evalutes to its value.
- **Short-circuiting `cfg` predicates:** Change `any` and `all` predicates to short-circuit instead of evaluating all their arguments. This would make introducing new predicates and comparison operators much easier.
