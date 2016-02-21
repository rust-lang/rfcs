- Feature Name: generic_atomic
- Start Date: 21-02-2016
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

This RFC proposes adding a generic `Atomic<T>` type which can accept any `T: Copy`. The actual set of types that are accepted is restricted based on what the target supports: if `T` is too big then compilation will fail with an error indicating that atomics of the required size are not supported. The actual atomic API is the same as the existing one, so there is nothing new there. However this will require compiler support for an `AtomicWrapper` type which is used internally.

```rust
#[atomic_wrapper]
struct AtomicWrapper<T: Copy>(T);

pub struct Atomic<T: Copy> {
    val: UnsafeCell<AtomicWrapper<T>>
}

impl<T: Copy> Atomic<T> {
    pub const fn new(val: T) -> Atomic<T>;
    pub fn load(&self, order: Ordering) -> T;
    pub fn store(&self, val: T, order: Ordering);
    pub fn swap(&self, val: T, order: Ordering) -> T;
    pub fn compare_exchange(self, current: T, new: T, success: Ordering, failure: Ordering) -> T;
    pub fn compare_exchange_weak(self, current: T, new: T, success: Ordering, failure: Ordering) -> (T, bool);
}

impl Atomic<bool> {
    pub fn fetch_and(&self, val: bool, order: Ordering) -> bool;
    pub fn fetch_nand(&self, val: bool, order: Ordering) -> bool;
    pub fn fetch_or(&self, val: bool, order: Ordering) -> bool;
    pub fn fetch_xor(&self, val: bool, order: Ordering) -> bool;
}

impl Atomic<i8> { // And other integer types. i64/u64 only if supported by target.
    pub fn fetch_add(&self, val: i8, order: Ordering) -> i8;
    pub fn fetch_sub(&self, val: i8, order: Ordering) -> i8;
    pub fn fetch_and(&self, val: i8, order: Ordering) -> i8;
    pub fn fetch_or(&self, val: i8, order: Ordering) -> i8;
    pub fn fetch_xor(&self, val: i8, order: Ordering) -> i8;
}
```

# Motivation
[motivation]: #motivation

Why are we doing this? What use cases does it support? What is the expected outcome?

# Detailed design
[design]: #detailed-design

## `Atomic<T>`

This is fairly straightforward: `load`, `store`, `swap`, `compare_exchange` and `compare_exchange_weak` are implemented for all atomic types. `bool` and integer types have additional `fetch_*` functions, which match those in `AtomicBool`, `AtomicIsize` and `AtomicUsize`.

The only complication is that `compare_exchange` does a bitwise comparison, which may fail due to differences in the padding bytes of `T`. This is solved with an intrinsic that explicitly clears the padding bytes of a value using a mask. We have to do this ourselves rather than relying on the user because Rust struct layouts are imlpementation-defined.

## `AtomicWrapper<T>`

This is an implementation detail which requires special compiler support. It has two functions:

- Rounds the size and alignment of `T` up to the next power of two.

- Gives an error message if `T` is too big for the current target's atomic operations.

Any extra padding added by `AtomicWrapper<T>` will be cleared before it is used in an atomic operation.

## Target support

One problem is that it is hard for a user to determine if a certain type `T` can be placed inside an `Atomic<T>`. After a quick survey of the LLVM and Clang code, architectures can be classified into 3 categories:

- The architecture does not support any form of atomics (mainly microcontroller architectures).
- The architecture supports all atomic operations for integers from i8 to iN (where N is the architecture word/pointer size).
- The architecture supports all atomic operations for integers from i8 to i(N*2).

A new target cfg is added: `target_has_atomic`. It will have multiple values, one for each atomic size supported by the target. For example:

```rust
#[cfg(target_has_atomic = "128")]
static ATOMIC: Atomic<(u64, u64)> = Atomic::new((0, 0));
#[cfg(not(target_has_atomic = "128"))]
static ATOMIC: Mutex<(u64, u64)> = Mutex::new((0, 0));

#[cfg(target_has_atomic = "64")]
static COUNTER: Atomic<u64> = Atomic::new(0);
#[cfg(not(target_has_atomic = "64"))]
static COUTNER: Atomic<u32> = Atomic::new(0);
```

In addition to this, we will guarantee that atomics with sizes less than or equal to `usize` will always be available. This is reasonable since `AtomicIsize`, `AtomicUsize` and `AtomicPtr` are always available as well. However it may limit our portability to some microcontroller architectures.


# Drawbacks
[drawbacks]: #drawbacks

`AtomicWrapper` relies on compiler magic to work.

Having certain atomic types get enabled/disable based on the target isn't very nice, but it's unavoidable.

`Atomic<bool>` will have a size of 1, unlike `AtomicBool` which uses a `usize` internally. This may cause confusion.

# Alternatives
[alternatives]: #alternatives

Rather than generating a compiler error, unsupported atomic types could be translated into calls to external functions in `compiler-rt`, like C++'s `std::atomic<T>`. However these functions use locks to implement atomics, which makes them unsuitable for some situations like communicating with a signal handler.

Several other designs have been suggested [here](https://internals.rust-lang.org/t/pre-rfc-extended-atomic-types/3068).

# Unresolved questions
[unresolved]: #unresolved-questions

Should we also rename `swap` to `exchange` while we're at it? It's more consistent with the new `compare_exchange` functions.
