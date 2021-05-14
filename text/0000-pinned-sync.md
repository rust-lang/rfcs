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

In all methods of these pinned versions a `self: Pin<&Self>` argument is taken, therefore the type must be constructed with, for example, `Arc::pin` or `Box::pin`.

In order to initialize a `pinned::Mutex`, for example, one of these can be used:

```
// Directly initialize using ergonomic constructors
let boxed_mutex: Pin<Box<Mutex<T>>> = Mutex::boxed(...);
let arc_mutex: Pin<Arc<Mutex<T>>> = Mutex::arc(...);

// Explicitly initialize after creation with a custom constructor
// Nothing here is unsafe, the library checks for initialization
let mut mutex: Pin<Box<Mutex<T>>> = Box::pin(Mutex::uninit(...));
mutex.as_mut().init();
```

# Reference-level explanation

[reference-level-explanation]: #reference-level-explanation

The std `sys_common` already has a pretty good infrastructure for implementing this.

- The explicit initialization is done by first creating the structure using `uninit(value: T) -> Self`, then calling `init(self: Pin<&mut Self>)`.
- The library adds the necessary assertions to make this pattern safe. Using before `init` causes a `panic!`. In particular, the implementation can use an `Option` to wrap the OS primitive. The assertions can be placed such that they are inlined, so if one method is used after another, the assertion can be optimized away for the second method.

# Drawbacks

[drawbacks]: #drawbacks

- Adds further complexity to the `std` library.

# Rationale and alternatives

[rationale-and-alternatives]: #rationale-and-alternatives

- Alternative for the explicit initialization: have unsafe `uninit()`. This can remove assertion overhead but can lead to some hard to detect undefined behaviour, as in POSIX - for example - this would cause mutexes to work as normal until a double lock happens, which would cause undefined behaviour.
- [Replace the primitives with `parking_lot`](https://github.com/rust-lang/rust/pull/56410). This has been proposed in the past and has its fair share of drawbacks, such as non-trivial space overhead which can be a deal breaker for some applications.

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