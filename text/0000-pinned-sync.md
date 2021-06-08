- Feature Name: `pinned-sync`
- Start Date: 2021-04-29
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)
# Summary

[summary]: #summary

Create a new module `std::sync::pinned` for OS synchronization primitives which make use of the `Pin` dialect instead of relying on boxing.

# Motivation

[motivation]: #motivation

The current synchronization primitives at `std::sync` box the OS primitives for some OSes, including all Posix-compliant. This is suboptimal as it increases the possibility of cache misses, and adds overhead to creation of these primitives.

With the new `Pin` type, we can expose the primitives directly and have the functions simply take `self: Pin<&Self>`.

This gives more flexibility as it is possible to have, for example, `Arc<Mutex<T>>` without double-boxing the `Mutex`.

# Guide-level explanation

[guide-level-explanation]: #guide-level-explanation

For the following types:

* `Barrier`

* `Condvar`

* `Mutex<T>`

* `RwLock<T>`

Pinned versions are available at the module `std::sync::pinned`. These versions are all `!Unpin`.

In all methods of these pinned versions a `self: Pin<&Self>` argument is taken.

In order to initialize a `pinned::Mutex`, for example, one of these can be used:

```rust
// Directly initialize using ergonomic constructors
let boxed_mutex: Pin<Box<Mutex<T>>> = Mutex::boxed(...);
let arc_mutex: Pin<Arc<Mutex<T>>> = Mutex::arc(...);

// Explicitly initialize after creation with a custom constructor
// Nothing here is unsafe, the library checks for initialization
let mutex: Pin<Box<Mutex<T>>> = Box::pin(Mutex::uninit(...));
mutex.as_ref().init();
```

# Reference-level explanation

[reference-level-explanation]: #reference-level-explanation

The std `sys_common` already has a pretty good infrastructure for implementing this.

- The explicit initialization is done by first creating the structure using `uninit(value: T) -> Self`, then calling `init(self: Pin<&Self>)`.
- `init(self: Pin<&Self>)` is atomic and safe.
    - Implementation-wise, an atomic flag can be used. The states for the flag are `UNINIT`, `LOCK` and `INIT`. When `init(self: Pin<&Self>)` is called, the flag is asserted to be `UNINIT` and is changed to `LOCK`, then the initialization is performed and then the flag is changed to `INIT`. For all other operations, the flag is asserted to be `INIT`.
    - This pattern is completely safe and will panic on race conditions,double initialization and use-before-initialize.
    - If we are on release mode and the primitive can be created in the`const uninit()`, the `init(self: Pin<&Self>)` is a no-op.

# Drawbacks

[drawbacks]: #drawbacks

- Adds further complexity to the `std` library.
- The assertions for initialization add some overhead on every call.

# Rationale and alternatives

[rationale-and-alternatives]: #rationale-and-alternatives

- `unsafe uninit()`
    - Causes UB if a method is used before `init` is called.
    - Pros:
        - No assertions on most methods.
    - Cons:
        - Unsafety.
        - UB can be very hard to debug, as many platforms can initialize the structure in `unsafe uninit()`, making the code work when it shouldn't.
- `init(self: Pin<&mut Self>)`
    - This was the initial proposal for the initialization pattern.
    - Pros:
        - No need for atomicity on the initialization flag.
        - As the flag isn't atomic hopefully the compiler could optimize it away (in the moment it doesn't).
    - Cons:
        - It isn't possible to initialize, for example, `Pin<Arc<(Mutex<T>, Condvar)>>` safely. This a major dealbreaker as it puts one of the major advantages of this change behind unsafety.
        - Initialization can not be done via a shared reference, so `static` also needs to be wrapped with `UnsafeCell` or made `mut`, both alternatives being `unsafe` as well.
- [Replace the primitives with `parking_lot`](https://github.com/rust-lang/rust/pull/56410).
    - Pros:
        - Completely constant initialization, so no `init` needed.
        - No boxing either, so the whole `Pin` pattern is not necessary.
        - It can be faster than system library high level primitives.
    - Cons:
        - The hash table introduces overhead which can be a dealbreaker when in very memory constrained environments.
        - We lose the benefit of using system libraries which can be updated independently of the program to reflect the introduction of new system calls.
- Implement the primitives from the ground up using the lowest level primitives available (futexes for example).
    - Pros:
        - It can be faster than system library high level primitives.
        - Most platforms support implementations which need no boxing or intialization.
    - Cons:
        - There can be some platforms which still require usage of primitives which need to be boxed and initialized. For example, POSIX-compliant platforms which do not expose lower level primitives.
        - We lose the benefit of using system libraries which can be updated independently of the program to reflect the introduction of new system calls.

# Prior art

[prior-art]: #prior-art

- System languages such as C and C++ expose the OS primitives with thin wrappers, without the boxing our current implementation uses. This is possible without `Pin` as these languages have different move semantics, on which types are not moved by default.
- https://users.rust-lang.org/t/pin-arc-mutex/41940
- `sync` package for the Linux kernel: https://github.com/Rust-for-Linux/linux/blob/rust/rust/kernel/sync/mod.rs

# Unresolved questions

[unresolved-questions]: #unresolved-questions

- Should `Once` be moved to `pinned` for consistency? The current implementation does not box any primitives, however if we were to use `pthread_once_t` in the future, it would be necessary.

# Future possibilities

[future-possibilities]: #future-possibilities

- Lints for double boxing (suggesting `Pin<Arc<pinned::Mutex<T>>>` when using `Arc<sync::Mutex<T>>`).