- Feature Name: parking_lot
- Start Date: 2016-05-27
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

This RFC proposes replacing the `Mutex`, `Condvar`, `RwLock` and `Once` types in
the standard library with those from the [`parking_lot`](https://github.com/Amanieu/parking_lot) crate. The synchronization
primitives in the `parking_lot` crate are smaller, faster and more flexible than
those in the Rust standard library.

# Motivation
[motivation]: #motivation

The primitives provided by `parking_lot` have several advantages over those
in the Rust standard library:

1. `Mutex` and `Once` only require 1 byte of storage space, while `Condvar`
   and `RwLock` only require 1 word of storage space. On the other hand the
   standard library primitives require a dynamically allocated `Box` to hold
   OS-specific synchronization primitives. The small size of `Mutex` in
   particular encourages the use of fine-grained locks to increase
   parallelism.
2. Since they consist of just a single atomic variable, have constant
   initializers and don't need destructors, these primitives can be used as
    `static` global variables. The standard library primitives require
   dynamic initialization and thus need to be lazily initialized with
   `lazy_static!`.
3. Uncontended lock acquisition and release is done through fast inline
   paths which only require a single atomic operation.
4. Microcontention (a contended lock with a short critical section) is
   efficiently handled by spinning a few times while trying to acquire a
   lock.
5. The locks are adaptive and will suspend a thread after a few failed spin
   attempts. This makes the locks suitable for both long and short critical
   sections.
6. `Condvar`, `RwLock` and `Once` work on Windows XP, unlike the standard
   library versions of those types.
7. `RwLock` takes advantage of hardware lock elision on processors that
   support it, which can lead to huge performance wins with many readers.
8. `MutexGuard` (and the `RwLock` equivalents) is `Send`, which means it can be
   unlocked by a different thread than the one that locked it.
9. `RwLock` will prefer writers, whereas the standard library version makes no
   guarantees as to whether readers or writers are given priority.
10. `Condvar` is guaranteed not to produce spurious wakeups. A thread will only
    be woken up if it timed out or it was woken up by a notification.
11. `Condvar::notify_all` will only wake up a single thread and requeue the rest
    to wait on the associated `Mutex`. This avoids a thundering herd problem
    where all threads try to acquire the lock at the same time.

# Detailed design
[design]: #detailed-design

The API of `Mutex`, `Condvar`, `RwLock` and `Once` will mostly stay the same.
The only user-visible API changes are the following:

- `Once` is no longer required to be `'static`.
- `MutexGuard`, `RwLockReadGuard` and `RwLockWriteGuard` will be `Send` if the
  underlying type is also `Send`. This allows them to be unlocked from a
  different thread than the one that created them.
- `Condvar` is guaranteed not to produce any spurious wakeups. A thread will
  only be woken up if its wait times out or if the `Condvar` is notified by
  another thread.
- `Condvar` is no longer restricted to being associated with a single `Mutex`
  for its entire lifetime. The only restriction is that you cannot wait using
  a `Mutex` if there are currently threads waiting on the `Condvar` with a
  different `Mutex` (this is the same restriction that pthreads has). This
  situation is detected and a panic will be generated.
- `Mutex`, `Condvar` and `RwLock` will have `const fn` constructors and no not
  require any drop glue. This makes them suitable for use in `static` variables.
- Calling `RwLock::read` when already holding a read lock may result in a
  deadlock if there is a writer thread waiting. Note that this was already the
  case in the Windows `RwLock` but it is now explicitly documented.

The internal parking lot APIs `park`, `unpark_one`, `unpark_all` and
`unpark_requeue` are not publicly exposed in the standard library API. Users
who wish to use these to create their own synchronization primitives should use
the `parking_lot` crate directly.

# Drawbacks
[drawbacks]: #drawbacks

`Mutex`, `Condvar` and `RwLock` are no longer simple wrappers around OS primitives.

The implementation of `parking_lot` is quite complicated because it needs to
support many advanced features like thread requeuing, hardware lock elision and
spin waiting.

# Alternatives
[alternatives]: #alternatives

The main alternative is to keep the existing synchronization primitives as they
are, which is essentially wrappers around OS synchronization primitives. This is
undesirable since there are many issues with these, such as the lack of support
for Windows XP or glibc's support for lock elision causing memory safety issues.

# Unresolved questions
[unresolved]: #unresolved-questions

None
