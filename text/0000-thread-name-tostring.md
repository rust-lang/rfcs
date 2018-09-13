- Feature Name: thread-name-tostring
- Start Date: 2018-09-13
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Relax the type of `std::thread::Builder::name` to be templated over
`T: ToString` rather than taking a `String`.

# Motivation
[motivation]: #motivation

Avoid having to use `ToString` on arguments passed to the function when the
implementation can do it itself.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Nothing much new here.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Old signature:

```rust
fn name(mut self, name: String) -> Builder
```

New signature:

```rust
fn name<N: ToString>(mut self, name: N) -> Builder
```

# Drawbacks
[drawbacks]: #drawbacks

Insta-stable since it is an API change.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The method takes ownership of the argument, so forcing using `AsRef` would make
it impossible to pass ownership and avoid an allocation.

# Prior art
[prior-art]: #prior-art

See rust-lang/rust#38856 where `std::process::Command::args` was relaxed from
taking a slice to any iterator.

# Unresolved questions
[unresolved-questions]: #unresolved-questions
