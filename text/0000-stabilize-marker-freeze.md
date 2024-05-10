- Feature Name: `stabilize_marker_freeze`
- Start Date: 2024-05-10
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Stabilize `core::marker::Freeze` in trait bounds.

# Motivation
[motivation]: #motivation

In some contexts, guaranteeing that a value cannot be modified is a requirement to a construct behaving properly (`const` references, keys of a structured map...).

While this looks achievable by only exposing immutable references, this is actually insufficient due to the (necessary) existence of interior mutability.

Internally, the compiler uses `core::marker::Freeze` to identify types that are known not to have interior mutability.

This RFC seeks to stabilize this trait for trait bounds for the following reasons:
- As explained above, some existing constructs are left to "trust" that the values they'd need to freeze won't be mutated through interior mutability. This is a potential bug source for these constructs.
- With [this PR](https://github.com/rust-lang/rust/issues/121250), a breaking change was introduced (and stabilized in 1.78):
	- This change prevents associated consts from containing references to values that don't implement `core::marker::Freeze`. Since this bound cannot be added to generics, this prevents associated consts from containing references to generics altogether.
	- This change was done in order to prevent potential unsoundness, but it also completely prevents sound uses, such as the ones reported in [this regression issue](https://github.com/rust-lang/rust/issues/123281).

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This RFC simply seeks to act as the stabilization RFC for `core::marker::Freeze` in trait bounds.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The work necessary for this RFC has already been done and merged in [this PR](https://github.com/rust-lang/rust/issues/121675), and a [tracking issue](https://github.com/rust-lang/rust/issues/121675) was opened.

However, it was deemed that adding an Auto Trait to Rust's public API should go through the proper RFC process, but the RFC was never created.

# Drawbacks
[drawbacks]: #drawbacks

- Some people have previously argued that this would be akin to exposing compiler internals.
	- The RFC author disagrees, viewing `Freeze` in a similar light as `Send` and `Sync`: a trait that allows soundness requirements to be proven at compile time.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- This trait has existed for 7 years. Over this course, it was made a compiler internal temporarily, but its usefulness lead to it being re-exposed soon after.
- The benefits of stabilizing `core::mem::Freeze` have been highlighted in [Motivation](#motivation).
- By not stabilizing `core::mem::Freeze` in trait bounds, we are preventing useful and sound code patterns from existing which were previously supported.

# Prior art
[prior-art]: #prior-art

This feature has been available in `nightly` for 7 years, and is used internally by the compiler.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

[Should the trait be exposed under a different name?](https://github.com/rust-lang/rust/pull/121501#issuecomment-1962900148)

# Future possibilities
[future-possibilities]: #future-possibilities

One might later consider whether `core::mem::Freeze` should be allowed to be `unsafe impl`'d like `Send` and `Sync` are, possibly allowing wrappers around interiorly mutable data to hide this interior mutability from constructs that require `Freeze` if the logic surrounding it guarantees that the interior mutability will never be used.

This consideration is purposedly left out of scope for this RFC to allow the stabilization of its core interest to go more smoothly; these two debates being completely orthogonal.