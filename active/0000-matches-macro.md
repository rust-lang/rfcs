- Start Date: 2014-07-14
- RFC PR #:
- Rust Issue #:

# Summary

Add a `matches!(expression, pattern)` macro that returns, as a boolean, whether the value of a given expression matches a given pattern.

# Motivation

When writing parsing code (for example [in rust-url](https://github.com/SimonSapin/rust-url/blob/de6bd47a1f1ffcc1b1388ecfd754f5615ff44fc4/src/parser.rs)), I often find myself writing complex conditionnals like this:

```rust
let input: &str = /* ... */;
if input.len() >= 2
        && match input.char_at(0) { '+' | '-' => true, _ => false }
        && match input.char_at(1) { '0'..'9' => true, _ => false } {
    // Parse signed number
} else if /* ... */
```

The `=> true, _ => false` part here is obviously redundant and adds more noise than it helps the readability of this code. We can get rid of it with a simple macro:

```rust
let input: &str = /* ... */;
if input.len() >= 2
        && matches!(input.char_at(0), '+' | '-')
        && matches!(input.char_at(1), '0'..'9') {
    // Parse signed number
} else if /* ... */
```

This macro feels general-purpose enough that it should be in the standard library. Copying it over and over in many projects is unfortunate, and the packaging/distribution overhead (even with Cargo) makes it ridiculous to have a dedicated library.

**Note:** This is different from the [`if let`](https://github.com/rust-lang/rfcs/pull/160) proposal, which replaces an entire `if` expression and can bind new names, while `matches!()` can be used as part of larger boolean expression (as in the example above) and does not make available new names introduced in the pattern. I believe the two proposals are complementary rather than competing.


# Detailed design

Add the following to `src/libstd/macros.rs`

```rust
/// Return whether the given expression matches the given patterns.
///
/// # Example
///
/// ```
/// let input: &str = /* ... */;
/// if input.len() >= 2
///         && matches!(input.char_at(0), '+' | '-')
///         && matches!(input.char_at(1), '0'..'9') {
///     // Parse signed number
/// } else if /* ... */
/// ```
#[macro_export]
macro_rules! matches(
    ($expression: expr, $($pattern:pat)|*) => (
        match $expression {
            $($pattern: pat)|+ => true,
            _ => false
        }
    );
)
```

# Drawbacks

This adds a new name to the "prelude": it is available implicitly, without being imported by a `use` statement or similar.

# Alternatives

* Status quo: potential users have to use the more verbose `match` statement, or define their own copy of this macro.
* Distribute this macro with rustc somehow, but not in the prelude. At the moment, users would have to write `#![feature(phase)] #[phase(plugin)] extern crate some_crate_name;` to import all macros defined in a given crate. Assuming we want to distribute more non-prelude macros with rustc in the future, this would mean either:
  * Adding a single "catch-all" crate called `std_macros` (or something). Since the `use` statement does not apply, users can not choose to only import some macros into their namespace, this crate is all-or-nothing.
  * Adding one more crate for every macro. This seems way overkill.
  * Adding a better mechanism for namespacing and importing macros, which would need to be designed and RFC’ed separately.


# Unresolved questions

Should this be a language feature rather than a macro? If so, what would the syntax look like?

Should the macro be named `matches!`, `is_match!`, or something else?

Should the macro also support guards?

```rust
    ($expression: expr, $($pattern:pat)|* if $guard: expr) => (
        match $expression {
            $($pattern: pat)|+ if $guard => true,
            _ => false
        }
    );
```
