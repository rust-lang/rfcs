- Feature Name: typed_cfgs
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

1.  **Exposing new features** when code is compiled with newer versions of the language or a library.
2.  **Straddling a breaking change** in an API, allowing code to compile against both the old and new versions.

The `rust_version` cfg proposed in this RFC is designed for the first use case, while `rust_edition` is a tool for the second. This RFC also provides a general mechanism for `version`-typed `cfg`s that can be used for libraries, which addresses both use cases.

This RFC also aims to solve long-standing problems that have been approached with more specific RFCs in the past. For example, the `cfg_target_version` RFC ([#3750](https://github.com/rust-lang/rfcs/pull/3750)) proposed a way to compare against the version of the target platform's SDK (e.g., `#[cfg(target_version(macos >= "10.15"))]`). This is crucial for safely using platform-specific APIs that are only available in newer OS versions. Instead of a one-off feature, the general `version` type proposed in this RFC provides a robust foundation to solve this problem. A build script could detect the platform version and emit a `version`-typed cfg (`--cfg 'macos_version=version("10.15")'`), allowing for the same ergonomic comparisons in code: `#[cfg(macos_version >= "10.15")]`. These platform-specific version keys can be added into the language in future RFCs.

The primary blockers for existing solutions have been:

- **Build Scripts are a Poor Solution:** The only stable tool for this today is a build script (`build.rs`). However, build scripts add significant compilation overhead and are clunky to write and maintain.
- **Previous Attempts had Flaws:** Past RFCs have tried to solve this, but ran into an unfortunate issue: their proposed syntax, e.g. `#[cfg(version(1.85))]`, was a syntax error on older compilers. This means that to use the feature, a library would first have to bump its MSRV to the version that introduced the syntax, defeating the primary purpose of the feature.

This RFC proposes a solution that avoids these pitfalls.

A key motivating example is making it ergonomic to adopt attributes that were stabilized after a crate's MSRV. For example, the `#[diagnostic::on_unimplemented]` attribute is a stable feature that library authors can use to provide better error messages. However, if a library has an MSRV from before this attribute was stabilized, they cannot use it without a build script. A build script is often too much overhead for such a small, non-essential feature.

This RFC makes it trivial to adopt even in a crate that doesn't want to use a build script. In this case, since the diagnostic attribute namespace landed before `rust_version`, you would write

```rust
#[cfg_attr(rust_version, diagnostic::on_unimplemented(
    message = "`{Self}` does not implement `MyTrait`",
    label = "missing implementation for `{Self}`",
))]
impl<T> MyTrait for T { /* ... */ }
```

With this feature, we will hopefully see more people using useful attributes like `on_unimplemented` even with MSRVs before when the diagnostic attribute namespace was added. The ability to conditionally add attributes for newer features is further detailed in the guide-level explanation.

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

### `version` predicates

A `version` predicate is available for `cfg` identifiers that are declared to be of type `version`.

The following grammar will be added for `cfg` predicates:

```text
cfg_predicate :
    IDENTIFIER ('>=' | '<') STRING_LITERAL
  | ...
```

This form of predicate is only valid when the `IDENTIFIER` on the left-hand side is a known `cfg` of type `version`. This RFC proposes two built-in `version` identifiers, `rust_version` and `rust_edition`, and a mechanism for build scripts and command-line tools to introduce new ones.

A `cfg` identifier is of type `version` if:
*   It is one of the built-in identifiers `rust_version` or `rust_edition`.
*   It is declared with `--check-cfg 'cfg(name, version)'` and is passed to the compiler with the special syntax `--cfg 'name=version("...")'`.

The `ident` must be a known `cfg` identifier of type `version`. The `literal` must be a string literal that represents a valid version.

A `version` predicate evaluates to `true` if the comparison is true, and `false` otherwise. If the identifier is not a known version `cfg`, or the literal is not a valid version string, a compile-time error is produced.

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
*   For `rust_version`, a lint will be issued if the literal has more than two components (e.g., `"1.92.0"`). This is because language features should not depend on patch releases.
    *   A new lint, `useless_version_constraint`, warns for version checks that are logically guaranteed to be true or false (e.g., `rust_version >= "1.20"` when the feature was stabilized in 1.90).
*   For `rust_edition`, a lint will be issued if the literal has more than one component or if we know the value is never going to be a Rust edition (for example, `"2019"`).
*   Pre-release identifiers (e.g., `"1.92-beta"`) are not supported in this RFC. They will be ignored during comparison and a lint will be emitted. See the "Unresolved Questions" section for further discussion.

Using version-typed config values with the `=` predicate results in a hard error.

### Interaction with Compiler Flags

The `version` type integrates with existing compiler flags.

*   **`--cfg`**: To define a `version`-typed `cfg`, the following syntax must be used:
    ```sh
    --cfg 'my_app_version=version("2.1.0")'
    ```
    This can also be used to override built-in version cfgs (e.g. `--cfg 'rust_version=version("1.50.0")'`), which is primarily useful for testing.

    * If a version cfg is used with a string literal in a comparison that is not a valid version string, a hard error is emitted.
    * If a cfg that is not a version type is used in a version comparison, a hard error is emitted. For undefined cfgs, this could be relaxed to evaluate to false in the future.
    * If a cfg that is a version type is used in a non-version comparison (`=`), a hard error is emitted.

*   **`--print cfg`**: For the built-in `rust_version` and `rust_edition` cfgs, this flag will *not* print them by default to avoid breaking tools that parse this output. They are only printed if overridden via `--cfg`. User-defined version cfgs are printed in the `name=version("...")` format.

*   **`--check-cfg`**: To inform the compiler that a `cfg` is expected to be a version, and to enable linting, use:
    ```sh
    --check-cfg 'cfg(my_app_version, version)'
    ```
    The built-in `rust_version` and `rust_edition` cfgs are implicitly included, so `rustc --print=check-cfg` will always list them.

### Stabilization

The features described in this RFC can be stabilized in phases:

1.  The initial stabilization can include the built-in `rust_version` and `rust_edition` cfgs and the ability to compare them with `>=` and `<`.
2.  The ability for users to define their own `version`-typed `cfg`s via `--cfg` and `--check-cfg` can be stabilized later.

This approach delivers the most critical functionality to users quickly, while allowing more time to finalize the design for user-defined version predicates.

# Drawbacks
[drawbacks]: #drawbacks

- Increased compiler complexity. This introduces a new concept of "typed" `cfg`s into the compiler, which adds a small amount of complexity to the parsing and evaluation logic for conditional compilation.
- Subtlety of MSRV-preserving patterns: The need for the "stacked `cfg`" pattern (`#[cfg(rust_version)] #[cfg(rust_version >= ...)]` and `#[cfg_attr(rust_version, cfg(rust_version >= ...))]`) is subtle. While we will add lints to guide users, it's less direct than a simple predicate. However, this subtlety is the explicit tradeoff made to achieve MSRV compatibility.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

### Why this design?
This design directly solves the MSRV problem in a way that previous attempts did not. The syntax `rust_version >= "1.85"` is highly intuitive and directly expresses the user's intent once the feature is available. It is a **principled** design, as by introducing a `version` type to the `cfg` system, we create a sound basis for comparison operators. This avoids the semantic confusion of proposals like `rust_version = "1.85"` which would have overloaded the meaning of `=` for a single special case. Furthermore, it's an extensible design that paves the way for other comparison operators, `cfg(some_dependency >= "1.2.3")`, or other typed `cfg`s in the future.

### Alternative 1: `#[cfg(version(1.85))]` (RFC 2523)
This was the original accepted RFC for version-based conditional compilation.

#### Rationale for not choosing
This syntax has several drawbacks. Most importantly, it introduces a new syntax that is a hard error on older compilers, making it unusable for its primary purpose of maintaining a low MSRV. The syntax `version("1.85")` is ambiguous; it is not clear from context whether this refers to the Rust version, the crate version, or some other dependency's version. The function-call-like syntax adds a level of nesting and is not necessarily intuitive for a `cfg` predicate. It evolves the language along two axes at once: adding a new capability *and* a new syntax paradigm for `cfg`. The current proposal, by contrast, builds on the existing `cfg` syntax in a more minimal way.

### Alternative 2: `#[cfg(rust_version = "1.85")]` (meaning `>=`)
This syntax is parseable by older compilers, which is a significant advantage for MSRV compatibility.

#### Rationale for not choosing
The use of `=` was highly controversial. In prior art, the `cfg` syntax has two conceptual models: "set inclusion" for multi-valued cfgs (e.g., `feature = "serde"`) and "queries" for boolean flags (e.g., `unix`). For set inclusion, `=` makes sense. However, checking the compiler version is a query for a single value, not a check for inclusion in a set. In this context, `=` strongly implies exact equality (`== 1.85`), while the overwhelming use case is for a lower bound (`>= 1.85`). Overloading `=` for this purpose was considered unprincipled and confusing. This RFC's approach of introducing a `version` type provides a proper semantic foundation for comparison operators like `>=`.

#### Advantage
This approach *could* potentially be made to work inside `Cargo.toml` (e.g., for conditional dependencies), which currently cannot use the stacked-cfg trick. However, the disadvantages in terms of semantic clarity for the language itself outweigh this benefit for an in-language feature.

### Alternative 3: `#[cfg(version_since(rust, "1.85"))]` (RFC 3857)
This alternative also avoids the MSRV problem and is extensible, similar to the current proposal.

#### Rationale for not choosing
While a good design, the "typed cfgs" approach with an actual comparison operator (`>=`, `<`) is arguably more natural and ergonomic. A language team poll indicated a preference for `rust_version >= "1.85"` if it could be made to work. This RFC provides the mechanism to make it work in a principled way.

# Prior art
[prior-art]: #prior-art

## Rust

- **Cargo's `rust-version`:** The `[package]` section of `Cargo.toml` can specify a `rust-version` field. This allows Cargo to select appropriate versions of dependencies and fail early if the compiler is too old. However, it does not provide fine-grained, in-code conditional compilation. This RFC brings a similar capability directly into the language, but for controlling code within a crate rather than for dependency resolution.

## Other languages

- **C++ (`__cplusplus`)**: The C++ standard defines the `__cplusplus` macro, which expands to an integer literal that increases with each new version of the standard (e.g., `201103L` for C++11, `202002L` for C++20). This allows for preprocessor checks like `#if __cplusplus >= 201103L`. This is very similar to the `rust_version >= "..."` proposal in that it uses standard comparison operators against a monotonically increasing value. However, it is less granular, as several years pass between new C++ versions.

- **Clang/GCC (`__has_feature`, `__has_attribute`)**: These function-like macros allow for checking for the presence of specific compiler features, rather than the overall language version. For example, `__has_feature(cxx_rvalue_references)` checks for a specific language feature. This approach is more granular but also more verbose if one needs to check for many features at once. This approach was discussed in RFC #2523, but rejected, in part because we wanted to reinforce the idea of Rust as "one language" instead of a common subset with many compiler-specific extensions.

- **Python (`sys.version_info`)**: Python exposes its version at runtime via `sys.version_info`, a tuple of integers `(major, minor, micro, ...)`. Code can check the version with standard tuple comparison, e.g., `if sys.version_info >= (3, 8):`. This component-wise comparison is very similar to the logic proposed in this RFC. However, because Python is interpreted, a file must be syntactically valid for the interpreter that is running it, which makes it difficult to use newer syntax in a file that must also run on an older interpreter. Rust, being a compiled language with a powerful conditional compilation system, does not have this limitation, and this RFC's design takes full advantage of that.

## Versioning systems

Not every system uses Rust's standard three-part semver versioning scheme, but many are close. In this section are examples of more bespoke versioning systems that this feature can accommodate.

- **Chromium**: Chromium's version format is a four-part number: MAJOR.MINOR.BUILD.PATCH, where MAJOR increments with significant releases, MINOR is often 0, BUILD tracks trunk builds, and PATCH reflects updates from a specific branch, with BUILD and PATCH together identifying the exact code revision.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- How should pre-release identifiers in version strings be handled? This RFC proposes not supporting pre-release identifiers in version strings passed on the command line for now. For comparisons, this RFC proposes that if a pre-release identifier is present in a `cfg` predicate (e.g., `rust_version < "2.0-alpha"`), the pre-release part is ignored for the comparison (so it's treated as `2.0`), and a lint is emitted. This ensures forward compatibility, as comparisons like `cfg(all(foo >= "2.0-alpha", foo < "2.0"))` become trivially false on older compilers, which is a safe outcome. This behavior can be refined before stabilization.

# Future possibilities
[future-possibilities]: #future-possibilities

- **"Compatible-with" operator:** We could introduce a `~=` operator that works like Cargo's caret requirements. For example, `cfg(some_dep ~= "1.5")` would be equivalent to `cfg(all(some_dep >= "1.5", some_dep < "2.0"))`. The rationale for not doing this now is that it's easy enough to write by hand.
- **More comparison operators:** While this RFC only proposes `>=` and `<`, the underlying `version` type makes it natural to add support for `<=`, `==`, `!=`, etc., in the future.
- **More flexible version strings:** The version string parsing could be extended to support pre-release identifiers (`-beta`, `-nightly`), though this adds complexity to the comparison logic.
- **Dependency Version `cfg`s:** The "typed `cfg`" infrastructure could be extended to query the versions of direct dependencies, e.g., `#[cfg(serde >= "1.0.152")]`. This would require significant integration with Cargo.
- **Other `cfg` types:** We could introduce other types, such as integers or single-valued strings. This could be useful for a variety of features, from system library versioning schemes ([kconfig](https://docs.kernel.org/kbuild/kconfig-language.html)) to enabling things like [mutually exclusive global features](https://internals.rust-lang.org/t/pre-rfc-mutually-excusive-global-features/19618).
- **Namespaced `cfg`s:** We could group Rust-specific `cfg`s under a `rust::` namespace, e.g., `#[cfg(rust::version >= "1.85")]`. This RFC intentionally keeps `rust_version` at the top level to simplify the initial implementation and stabilization, but namespacing could be explored in the future to better organize the growing number of built-in `cfg`s.
- **Short-circuiting `cfg` predicates:** Change `any` and `all` predicates to short-circuit instead of evaluating all their arguments. This would make introducing new predicates and comparison operators much easier.
- **More expressive check-cfg:** We can support specifying an expected number of components in check-cfg, or an expected set of values to compare against, as in editions.
