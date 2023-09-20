- Feature Name: `cargo-check-lang-policy`
- Start Date: 2023-08-22
- RFC PR: [rust-lang/rfcs#3477](https://github.com/rust-lang/rfcs/pull/3477)

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

Specifically, if a given Rust program does not compile with `cargo build` then it might or might not pass `cargo check`. If a program does not compile with `cargo build` but does pass `cargo check` it still might not pass a `cargo check` in a future version of Rust. Changes in `cargo check` outcome when `cargo build` does not work are not considered a breaking change in Rust.

`cargo check` **should** catch as many errors as possible, but the emphasis of `cargo check` is on giving a "fast" answer rather than giving a "complete" answer.
If you need a complete answer with all possible errors accounted for then you **must** use `cargo build`.
The rationale for this is that giving a "complete" answer requires (among other things) doing full monomorphization (since some errors, such as those related to associated consts, can only be caught during monomorphization).
Monomorphization is expensive: instead of having to check each function only once, each function now has to be checked once for all choices of generic parameters that the crate needs.
Given this performance cost and the fact that errors during monomorphization are fairly rare, `cargo check` favors speed over completeness.

Examples where the optimization level can affect if a program passes `cargo check` and/or `cargo build` are considered bugs unless there is a documented policy exception, approved by T-lang. One example of such an exception is [RFC #3016](https://rust-lang.github.io/rfcs/3016-const-ub.html), which indicated that undefined behavior in const functions cannot always be detected statically (and in particular, optimizations may cause the UB to be undetectable).

# Frequently Asked Questions

## Why doesn't `check` catch everything?

The simplest example here is linker errors.  There's no practical way to confirm that linking will work without actually going through all the work of generating the artifacts and actually calling the linker, but that that point one might as well run `build` instead.

An important part of what can make `check` faster than `build` is just *not* doing that kind of thing.  And linker errors are rare in pure Rust code, so this is often a good trade-off.

## Why not let more things through in optimized builds?

Rust takes [stability without stagnation] very seriously.  We want to make sure stuff keeps compiling if it did before, but we also want to be able to work on improving rust without being so constrained as to make that functionally impossible.

If an optimization might allow something more to compile, that means that every small tweak to that optimization requires careful oversight for exactly what it's committing to support *forever*, which results in extreme overhead for rustc's developers.  The best way to avoid that is to have optimizations be about making things faster, not about what compiles *at all*.

For things where people want a certain behaviour, that should be something guaranteed as an intentional language semantic, which we can restrict appropriately to make it feasible with or without optimization.

As an example, there are various *lints* that can detect more cases when optimizations are run, but that's part of why they're lints -- which are fundamentally not *guaranteed* -- rather than part-of-the-language *errors*.

[stability without stagnation]: https://blog.rust-lang.org/2014/10/30/Stability.html#the-plan

# Unresolved questions
[unresolved-questions]: #unresolved-questions

* Is there any situation when we would *want* to allow optimization level to affect if a program passes or fails a build? This seems unlikely.

# Future possibilities
[future-possibilities]: #future-possibilities

* Any future changes in this policy would require a future RFC so that such changes are as clear and visible as possible.
