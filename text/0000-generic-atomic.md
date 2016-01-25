- Feature Name:
- Start Date: 2016-01-22
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add compiler support for generic atomic operations.

# Motivation
[motivation]: #motivation

It is sometimes necessary or useful to use atomic operations in a generic
context.

```rust
Atomic<T>
```

where `T` is any type. This is currently possible but fails to compile if `T` is
too large for the native atomic operations of the target architecture.

# Detailed design
[design]: #detailed-design

The behavior of each of the atomic intrinsics in appendix A is changed as
follows: When a call to one of the atomic intrinsics cannot be translated to a
native operation, it is translated to an unspecified but valid instruction. If
it is executed, the behavior is undefined at runtime.

One intrinsic is added:

```rust
fn has_native_atomic_ops<T>() -> bool;
```

This intrinsic returns `false` iff a call to one of the aforementioned atomic
operations cannot be translated to a native operation.

# Drawbacks
[drawbacks]: #drawbacks

None.

# Alternatives
[alternatives]: #alternatives

None.

# Unresolved questions
[unresolved]: #unresolved-questions

None.

# Appendix A

```rust
fn atomic_cxchg<T>(dst: *mut T, old: T, src: T) -> T;
fn atomic_cxchg_acq<T>(dst: *mut T, old: T, src: T) -> T;
fn atomic_cxchg_rel<T>(dst: *mut T, old: T, src: T) -> T;
fn atomic_cxchg_acqrel<T>(dst: *mut T, old: T, src: T) -> T;
fn atomic_cxchg_relaxed<T>(dst: *mut T, old: T, src: T) -> T;

fn atomic_load<T>(src: *const T) -> T;
fn atomic_load_acq<T>(src: *const T) -> T;
fn atomic_load_relaxed<T>(src: *const T) -> T;
fn atomic_load_unordered<T>(src: *const T) -> T;

fn atomic_store<T>(dst: *mut T, val: T);
fn atomic_store_rel<T>(dst: *mut T, val: T);
fn atomic_store_relaxed<T>(dst: *mut T, val: T);
fn atomic_store_unordered<T>(dst: *mut T, val: T);

fn atomic_xchg<T>(dst: *mut T, src: T) -> T;
fn atomic_xchg_acq<T>(dst: *mut T, src: T) -> T;
fn atomic_xchg_rel<T>(dst: *mut T, src: T) -> T;
fn atomic_xchg_acqrel<T>(dst: *mut T, src: T) -> T;
fn atomic_xchg_relaxed<T>(dst: *mut T, src: T) -> T;
```
