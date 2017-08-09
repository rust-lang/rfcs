- Feature Name: more-env-info
- Start Date: 2017-08-09
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

Introduce more potentially useful environment-related functions to `std::env`.

# Motivation
[motivation]: #motivation

Programs sometimes need to know about their working environment to do their job properly. For example, linebreak convention differs for Windows and *NIX. Such discrepancy can lead to problems easily, especially when a program needs to communicate with aged third-party libraries.

As a system programming language, it would be good for Rust to know the system it works on.

# Detailed design
[design]: #detailed-design

This RFC would like to introduce the following functions to `std::env`:

```rust
/// Platform architecture.
/// e.g. `arm`, `x86_64` and `i686`.
pub fn arch() -> String;
/// Conventional linebreak of current platform.
pub fn linebreak() -> String;
/// Current operating system.
/// e.g. `windows`, `linux` and `darwin`.
pub fn os() -> String;
/// Word size in bits the program has been compiled into.
/// Commonly 32 or 64.
pub fn word_size() -> u32;
```

Since rust is a compiled language, these information has to be derived from the compiler.

Documentation should list all possible results for `arch()` and `os()`.

# Drawbacks
[drawbacks]: #drawbacks

Introduce more items into the standard library. It also requires the compiler to provide these information, added work to be done by compiler.

# Rationale and Alternatives
[alternatives]: #alternatives

These information can partly be retrieved through system APIs. However, doing so can make programs less adaptive because there is no unified interface.

It's been considered to use enum over string to carry information for `arch()` and `os()`, but it seems less adaptive as time going and new thing coming.

# Unresolved questions
[unresolved]: #unresolved-questions

The datatype returned by `word_size()` is not yet determined.

Should we provide the entire target triplet like `x86_64-unknown-linux` in `target()` rather than separating them into `arch()` and `os()`?
