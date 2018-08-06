- Feature Name: universal_acq_rel
- Start Date: 2018-07-19
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add a new element `AutoAcqOrRel` to
[`Ordering`](https://doc.rust-lang.org/stable/std/sync/atomic/enum.Ordering.html)
which can be used as ordering mode for all atomic operations, meaning `Acquire`
for all reads and `Release` for all writes.

# Motivation
[motivation]: #motivation

In many cases, `Acquire`/`Release`/`AcqRel` is sufficient for synchronization.
Programs that use `Acquire`/`Release`/`AcqRel` consistently can be analyzed
fairly efficiently by thinking in terms of onwership transfer.  However, one
must take extra care to eason about any `Relaxed` access because those doe *not*
induce proper ownership transfer.

However, there is currently no easy to to ensure that no access is `Relaxed`.
One has to carefully check every operation and use `AcqRel`, `Acquire`, or
`Release`, depending on whether this is a RMW (read-modify-write), write, or
read operation.  If one accidentally uses e.g. `Acquire` instead of `AcqRel` for
`compare_and_swap` or `fetch_add`, that is a serious correctness problem: Now
the read part of the operation is `Relaxed`!

I think we could significantly improve the situation if one could just use a
single mode consistently for all operations to avoid `Relaxed`.  As a strawman,
that mode could be called `AutoAcqOrRel`.  The meaning of that ordering mode
would be "All loads are `Acquire`, all writes are `Release`".  This is different
from `AcqRel` in two ways (which are really two different ways to express the
same point):
* `AutoAcqOrRel` can be used with `load` and `store`, with the obvious meaning.
* Using `AutoAcqOrRel` does *not* on its own say that the operation is both
  `Acq` and `Rel`.  This is different to `AcqRel` which most of the time
  expresses that the operation has both modes -- with the sole exception of a
  failed `compare_and_swap(..., AcqRel)` which is just `Acq`.

With this, one just has to check that `AutoAcqOrRel` is used everywhere, and one
can be sure that no access is accidentally `Relaxed`.  That's easier to review
and less error-prone than the current situation.

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
properly "happen before" all subsequent effects of thread B.

Moreover, some operations perform *both* a load and a store, like
`compare_and_swap`.  These are called read-modify-write operations.  There, you
need to use `AcqRel` to make sure that the load is `Acquire` and the store is
`Release`; using e.g. `Acquire` will lead to an `Acquire` load, but an entirely
unordered (`Relaxed`) store, which may or may not be what you want.

To avoid accidentally using the `Relaxed` mode, you can consistently use
`AutoAcqOrRel` as ordering for all operations.  That will pick `Acquire` for all
loads, `Release` for all stores and `AcqRel` for read-modify-write operations.
Unlike `AcqRel`, using `AutoAcqOrRel` is possible for all operations.  Unlike
`Acquire` or `Release`, using `AutoAcqOrRel` will never lead to a relaxed
(unordered) memory access.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

`AutoAcqOrRel` is a new legal ordering mode for all atomic operations.  For
loads, it is equivalent to `Acquire`.  For stores, it is equivalent to
`Release`.  For operations that may both load and store, it is equivalent to
`AcqRel`.

# Drawbacks
[drawbacks]: #drawbacks

Other languages (notably C and C++) do not have this mode, so people coming from
such languages might be surprised when they read Rust code.  However, the new
mode is quickly explained in terms of terminology that they should be familiar
with.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The alternative is to do nothing and stick to the status quo.  That means
writing concurrent code using release-acquire synchronization requires a careful
choice of the right ordering mode for every operation, meaning it is
*syntactically* hard to consistently use release-acquire modes everywhere and
avoid `Relaxed`.

# Prior art
[prior-art]: #prior-art

None that I am aware of.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None I can think of.
