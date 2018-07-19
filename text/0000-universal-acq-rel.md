- Feature Name: universal_acq_rel
- Start Date: 2018-07-19
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Allow using
[`Ordering::AcqRel`](https://doc.rust-lang.org/stable/std/sync/atomic/enum.Ordering.html#variant.AcqRel)
as ordering mode for all atomic operations, meaning `Acquire` or `Release` as
appropriate.

# Motivation
[motivation]: #motivation

In many cases, `Acquire`/`Release`/`AcqRel` is sufficient for synchronization.
However, using this consistently is syntactically more complex than using
`SeqCst` consistently: Depending on the operation
(`load`/`store`/`compare_and_swap` or similar), one has to use one of
`Acquire`/`Release`/`AcqRel`.  Using the wrong mode either leads to a run-time
error (when using `AcqRel` with `load` or `store`), or to plainly incorrect
run-time behavior (accidentally using `Acquire` instead of `AcqRel` with a
`compare_and_swap` means the write part of the operation will *not* be a
`Release` write).  Needless to say, both are not a great failure mode.

I think we could significantly improve the situation if one could just use
`AcqRel` consistently for all operations, meaning "All loads are `Acquire`, all
writes are `Release`".  That's easier to review and less error-prone than the
current situation.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

All atomic operations (methods of the
[`sync::atomic::Atomic*`](https://doc.rust-lang.org/beta/std/sync/atomic/index.html)
types) take as a parameter the "synchronization mode" or "ordering mode" that is
to be used for this operation.  That mode defines how strong the memory barrier
for that operation is.

The strongest mode is `SeqCst`, which guarantees that atomic operations behave
like they are "interleaved": Either operation A occurs before operation B, or
vice versa.  This is the safest, most conservative and least performant choice.

The next weaker family of modes is the release-acquire family: `Acquire`,
`Release` and `AcqRel`.  Loads can be annotated with `Acquire` and stores with
`Release`.  Whenever an `Acquire` load in thread B reads from a `Release` store
in thread A, the two threads are synchronized and all prior effects of thread A
properly "happen before" all subsequent effects of thread B.  For simplicity,
you can just use `AcqRel` as mode for both load and store operations, and the
corresponding mode appropriate for this operation (`Acquire` for loads,
`Release` for stores) will be picked.

Moreover, some operations perform *both* a load and a store, like
`compare_and_swap`.  There, you need to use `AcqRel` to make sure that the load
is `Acquire` and the store is `Release`; using e.g. `Acquire` will lead to an
`Acquire` load, but an entirely unordered (`Relaxed`) store, which may or may
not be what you want.  Consistently using `AcqRel` ensures that you do not have
any unordered (`Relaxed`) operations in your code.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

`AcqRel` is a legal ordering mode for all atomic operations.  For loads, it is
equivalent to `Acquire`.  For stores, it is equivalent to `Release`.  For
operations that may both load and store, it means that the load part is
`Acquire` and the store part is `Release`.

# Drawbacks
[drawbacks]: #drawbacks

Other languages (notably C and C++) do not permit using the combined
acquire-release mode for pure `load`/`store` operations.  By allowing this in
Rust, we might surprise people that expect an exact equivalent of the C/C++ API.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

I laid out the reasons for why I think this is better than what C/C++ do (aka,
the status quo in Rust) in the motivation section above.  In fact, it turns out
that
[at least one seasoned Rust developer thinks this RFC is already implemented](https://github.com/rust-lang/rust/pull/52349#issuecomment-405104966).

The alternative is to do nothing and stick to the status quo.  That means
writing concurrent code using release-acquire synchronization requires a careful
choice of the right ordering mode for every operation.


# Prior art
[prior-art]: #prior-art

I can only speculate why C/C++ reject the combined release-acquire mode for pure
load/store operations.  One reason may be that seeing `load(AcqRel)` could give
the impression that a `Release` operation is happening here, while in reality
this is just an `Acquire` operation.  Note, however, that `compare_and_swap(...,
AcqRel)` also looks like a `Release` operation is happening, but if the CAS
fails, then this is just an `Acquire` as well.

Also, the only case where this makes Rust behave different from C/C++ is code
that would error at run-time in C/C++.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None I can think of.
