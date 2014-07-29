- Start Date: 2014-07-29
- RFC PR #:
- Rust Issue #:

# Summary

Deprecate and later remove the in-tree url crate, in favor of rust-url.


# Motivation

This code [has](https://github.com/rust-lang/rust/issues/8486)
a [number](https://github.com/rust-lang/rust/issues/10705)
of [issues](https://github.com/rust-lang/rust/issues/10706)
that are [non-trivial](https://github.com/rust-lang/rust/issues/10707)
to fix incrementally.

[rust-url](http://servo.github.io/rust-url/) is a rewrite from scratch.
It can be used with Cargo.


# Detailed design

Replace `#![experimental]` with `#![deprecated]` in `src/liburl/lib.rs`.
([PR #16076](https://github.com/rust-lang/rust/pull/16076))

Later, after a deprecation cycle to be determined, remove `src/liburl` entirely.


# Drawbacks

Users will have to upgrade to a slightly different API.


# Alternatives

If someone wants to keep using the old code for some reason,
it could be extracted and put in a separate Cargo-enabled repository.


# Unresolved questions

How long should be the deprecation cycle?
