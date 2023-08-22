- Feature Name: `cargo-check-lang-policy`
- Start Date: 2023-08-22
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC helps to codify the T-lang policy regarding `cargo check` vs `cargo build`.

This is a policy RFC rather than a change to the language itself, and is thus "instantly stable" once accepted. There's no associated feature within the compiler, and no further stabilization report necessary.

# Motivation
[motivation]: #motivation

It has often come up within GitHub issues what *exactly* the intended line is between Cargo's `check` and `build` commands should be, what users should expect, and so on.

The RFC gives a clear policy from T-lang's perspective so that both other teams within the Rust project as well as users of the Rust project can have the same expectations.

# Decision
[decision]: #decision

* `cargo build` catches all Rust compilation errors.
* `cargo check` only catches some subset of the possible compilation errors.
* A Rust program **must** compile with `cargo build` to be covered by Rust's standard stability guarantee.

Specifically, if a given Rust program passes `cargo check` but **not** `cargo build` in one version of Rust, then in any future version of Rust that program *can* begin to also fail `cargo check`, and this is **not** considered a breaking change.

`cargo check` should catch as many errors as possible, but the emphasis is on giving a fast answer rather than giving a complete answer. If you need a complete answer then you need to use `cargo build`.

The optimization level of the compiler **should not** affect if the program compiles or not (using `build` or `check`). Any such case is very likely a bug, and T-lang will have to make a determination on a case-by-case basis.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

* Is there any situation when we would *want* to allow optimization level to affect if a program passes or fails a build? This seems unlikely.

# Future possibilities
[future-possibilities]: #future-possibilities

* Any future changes in this policy would require a future RFC so that such changes are as clear and visible as possible.
