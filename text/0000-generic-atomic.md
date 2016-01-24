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

Several new language items are added which correspond to the intrinsics listed
in appendix A. For example, the following language item corresponds to the
`atomic_cxchg` intrinsic:

```rust
#[lang = "atomic_cxchg"]
fn atomic_cxchg<T>(dst: *mut T, old: T, src: T) -> T {
    // implementation omitted
}
```

When a call to one of the atomic intrinsics cannot be translated to a native
operation, it is translated to a call to the corresponding language item. This
language item then emulates the atomic operation. For example, it might acquire
a global lock, perform the corresponding non-atomic operation, and release the
global lock.

One intrinsic is added:

```rust
fn has_native_atomic_ops<T>() -> bool;
```

This intrinsic returns `false` iff a call to one of the aforementioned atomic
operations is translated to a library call.

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

This appendix contains the full list of intrinsics for which a corresponding
language item is added. The name of the language item is the name of the
intrinsic, the language item is a function, and the signature of the function is
the signature of the intrinsic.

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
