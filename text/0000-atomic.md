- Feature Name:
- Start Date: Sat Mar 21 18:55:24 CET 2015
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Add `AtomicI32` and `AtomicU32`.

# Motivation

Atomic operations on these types are necessary to interact with certain system
APIs, e.g., futexes.

# Detailed design

Copy `AtomicIsize` and `AtomicUsize` and then `s/size/32/`.

# Drawbacks

More wrappers for what should simply be freestanding functions.

# Alternatives

None.

# Unresolved questions

None.
