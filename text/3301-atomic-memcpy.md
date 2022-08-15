- Feature Name: `atomic_memcpy`
- Start Date: 2022-08-14
- RFC PR: [rust-lang/rfcs#3301](https://github.com/rust-lang/rfcs/pull/3301)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary

This is a proposal to add `AtomicPerByte<T>`, to represent _tearable atomics_.
This makes it possible to properly implement a _sequence lock_ in Rust.

# The Problem

It's currently not possible to implement an efficient and perfectly
(theoretically) correct sequence lock in Rust.

Unlike most locking mechanisms, a sequence lock doesn't prevent a race
to access the data it projects.
Instead, it detects a race only after the load operation already happened,
and retries it if the load operation raced with a write operation.

A sequence lock in Rust looks something like this:

```rust
// Incomplete example

pub struct SeqLock<T> {
    seq: AtomicUsize,
    data: UnsafeCell<T>,
}

unsafe impl Sync<T: Copy + Send> for SeqLock<T> {}

impl<T: Copy> SeqLock<T> {
    /// Safety: Only call from one thread.
    pub unsafe fn write(&self, value: T) {
        self.seq.fetch_add(1, Relaxed);
        write_data(&mut self.data, value, Release);
        self.seq.fetch_add(1, Release);
    }

    pub fn read(&self) -> T {
        loop {
            let s1 = self.seq.load(Acquire);
            let data = read_data(&self.data, Acquire);
            let s2 = self.seq.load(Relaxed);
            if s1 & 1 == 0 && s1 == s2 {
                return unsafe { assume_valid(data) };
            }
        }
    }
}
```

The `write_data` and `read_data` calls can happen concurrently.
The `write` method increments the counter before and after,
such that the counter is odd during `write_data`.
The `read` function will repeat `read_data` until
the counter was identical and even both before and after reading.
This way, `assume_valid` is only ever called on data that was
not the result of a race.

A big question is how to implement `write_data`, `read_data`, and `assume_valid`
in Rust in an efficient way while satisfying the memory model.

The somewhat popular `seqlock` crate and similar implementations found in the ecosystem
all use a regular non-atomic write (preceded by an atomic fence) for writing,
and `ptr::read_volatile` (followed by an atomic fence) for reading.
This "works" "fine", but is technically undefined behavior.

The C++ and Rust memory model don't allow for data races,
so don't allow for a data race to be detected after the fact;
that's too late.

All of the data would have to be written and read through
atomic operations to prevent such a data race.
We don't need the atomicity of the data as a whole though;
it's fine if there's tearing, since we re-start on a race anyway.

# The C++ Solution

C++'s [P1478] proposes the addition of these two functions to the C++ standard
library to solve this problem:

```cpp
void atomic_load_per_byte_memcpy(void *dest, void *source, size_t, memory_order);
void atomic_store_per_byte_memcpy(void *dest, void *source, size_t, memory_order);
```

The first one is effectively a series of `AtomicU8::load`s followed by a memory fence,
while the second one is basically a memory fence followed by series of `AtomicU8::store`s.
Except the implementation can be much more efficient.
The implementation is allowed to load/store the bytes in any order,
and doesn't have to operate on individual bytes.

The memory order can only be relaxed, acquire (for load), and release (for store).
Sequentially consistent ordering for these operations is disallowed,
since it's not obvious what that means for these tearable operations.

# The Rust Solution

While C++'s solution can be easily copy-pasted into Rust with a nearly identical signature,
it wouldn't fit with the rest of our atomic APIs.

All our atomic operations happen through the `Atomic*` types,
and we don't have atomic operations that operate on raw pointers.
(Other than as unstable intrinsics.)

Adding this functionality as a variant on `copy_nonatomic`, similar to the C++ solution,
would not be very ergonomic an can easily result in subtle bugs causing undefined behavior.

Instead, I propose to add a `AtomicPerByte<T>` type
similar to our existing atomic types: a `Sync` storage for a `T`
that can be written to and read from by multiple threads concurrently.

The `SeqLock` implementation above would use this type instead of an `UnsafeCell`.
It'd no longer need an unsafe `Sync` implementation,
since the `AtomicPerByte<T>` type can be shared between threads safely.

This type has a (safe!) `store` method consuming a `T`,
and a (safe!) `load` method producing a `MaybeUninit<T>`.
The `MaybeUninit` type is used to represent the potentially invalid state
the data might be in, since it might be the result of tearing during a race.

Only after confirming that there was no race and the data is valid
can one safely use `MaybeUninit::assume_init` to get the actual `T` out.

# Full API Overview

The `AtomicPerByte<T>` type can be thought of as
the `Sync` (data race free) equivalent of `MaybeUninit<T>`.
It can contain a `T`, but it might be invalid in various ways
due to concurrent store operations.
Its interface resembles a mix of the interfaces of `MaybeUninit` and the atomic types.

```rust
#[repr(transparent)]
struct AtomicPerByte<T> { inner: UnsafeCell<MaybeUninit<T>> }

unsafe impl<T: Send> Sync for AtomicPerByte<T> {}

impl<T> AtomicPerByte<T> {
    pub const fn new(value: T) -> Self;
    pub const fn uninit() -> Self;

    pub fn load(&self, ordering: Ordering) -> MaybeUninit<T>;
    pub fn store(&self, value: T, ordering: Ordering);

    pub fn load_to(&self, dest: &mut MaybeUninit<T>, ordering: Ordering);
    pub fn store_from(&self, src: &MaybeUninit<T>, ordering: Ordering);

    pub fn load_to_slice(this: &[Self], dest: &mut [MaybeUninit<T>], ordering: Ordering);
    pub fn store_from_slice(this: &[Self], src: &[MaybeUninit<T>], ordering: Ordering);

    pub const fn into_inner(self) -> MaybeUninit<T>;

    pub const fn as_ptr(&self) -> *const T;
    pub const fn as_mut_ptr(&self) -> *mut T;

    pub const fn get_mut(&mut self) -> &mut MaybeUninit<T>;
    pub const fn get_mut_slice(this: &mut [Self]) -> &mut [MaybeUninit<T>];

    pub const fn from_mut(value: &mut MaybeUninit<T>) -> &mut Self;
    pub const fn from_mut_slice(slice: &mut [MaybeUninit<T>]) -> &mut [Self];
}

impl<T> Debug for AtomicPerByte<T>;
impl<T> From<T> for AtomicPerByte<T>;
impl<T> From<MaybeUninit<T>> for AtomicPerByte<T>;
```

Note how the entire interface is safe.
All potential unsafety is captured by using `MaybeUninit`.

The load functions panic if the `ordering` is not `Relaxed` or `Acquire`.
The store functions panic if the `ordering` is not `Relaxed` or `Release`.
The slice functions panic if the slices are not of the same length.

# Drawbacks

- In order for this to be efficient, we need an additional intrinsic hooking into
  special support in LLVM. (Which LLVM needs to have anyway for C++.)

- It's not immediately obvious this type behaves like a `MaybeUninit`,
  making it easy to forget to manually drop any values that implement `Drop`.

  This could be solved by requiring `T: Copy`, or by using a better name for this type.

- `MaybeUninit<T>` today isn't as ergonomic as it should be.

  For a simple `Copy` type like `u8` it might be nicer to be able to use types like `&[u8]`
  rather than `&[MaybeUninit<u8>]`, etc.
  (But that's a larger problem affecting many other things, like `MaybeUninit`'s interface,
  `Read::read_buf`, etc. Maybe this should be solved separately.)

# Alternatives

- Instead of a type, this could all be just two functions on raw pointers,
  such as something like `std::ptr::copy_nonoverlaping_load_atomic_per_byte`.

  This means having to use `UnsafeCell` and more unsafe code wherever this functionality is used.

  It'd be inconsistent with the other atomic operations.
  We don't have e.g. `std::ptr::load_atomic` that operates on pointers either.

- Require `T: Copy` for `AtomicPerByte<T>`, such that we don't need to worry about
  duplicating non-`Copy` data.

  There might be valid use cases with non-`Copy` data.
  Also, not all "memcpy'able" data is always marked as `Copy` (e.g. to prevent implicit copies).

- Leave this to the ecosystem, outside of the standard library.

  Since this requires special compiler support, a community crate would
  have to use (probably technically unsound) hacks like volatile operations
  or inline assembly.

# Unresolved questions

- Should we require `T: Copy`?

  There might be some valid use cases for non-`Copy` data,
  but it's easy to accidentally cause undefined behavior by using `load`
  to make an extra copy of data that shouldn't be copied.

- Naming: `AtomicPerByte`? `TearableAtomic`? `NoDataRace`? `NotQuiteAtomic`?

[P1478]: https://www.open-std.org/jtc1/sc22/wg21/docs/papers/2022/p1478r7.html
