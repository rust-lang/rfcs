- Feature Name: `freeze`
- Start Date: 2020-06-12
- RFC PR: [rust-lang/rfcs#2944](https://github.com/rust-lang/rfcs/pull/2944)
- Rust Issue: TBD

# Summary
[summary]: #summary

This RFC introduces new APIs to libcore/libstd to serve as safe abstractions for data which has no "shallow" interior mutability.

```rust
pub unsafe auto trait Freeze {}
pub struct PhantomUnfrozen;
```

# Motivation
[motivation]: #motivation

It is occasionally necessary in systems programming to know whether the range of bytes occupied by a value is truly immutable. Given that rust has interior mutability, there is currently no way to represent this immutability in the type system.

## Read Only Memory

If a type is suitable for read only memory, then it cannot have any interior mutability. For example, an `AtomicU8` is a poor candidate for being put into read only memory because the type system has no way to ensure that type is not mutated. It is, however, allowed to put a `Box<AtomicUsize>` in read only memory as long as the heap allocation remains in writable memory.

The [main reason](https://github.com/rust-lang/rust/blob/84ec8238b14b4cf89e82eae11907b59629baff2c/src/libcore/marker.rs#L702) libcore has a private version of `Freeze` is to decide:
> whether a `static` of that type is placed in read-only static memory or writable static memory

Another example of read only memory includes read only memory mappings.

## Optimistic Concurrency

Optimistic concurrency (e.g. seqlocks, software transactional memory) relies heavily on retrieving shallow snapshots of memory. These snapshots can then be treated as read only references to the original data as long as no mutation occurs. In the case of interior mutability (e.g. `Mutex<T>`), this falls apart.

One example coming from [`swym`](https://docs.rs/swym/0.1.0-preview/swym/tcell/struct.TCell.html#method.borrow) is the method `borrow`. `borrow` returns snapshots of data - shallow memcpys - that are guaranteed to not be torn, and be valid for the duration of the containing transaction. These snapshots hold on to the lifetime of the `TCell` in order to act like a true reference, without blocking updates to the `TCell` from other threads. Other threads promise to not mutate the value that had its snapshot taken until the transaction has finished, but are permitted to move the value in memory. In the presence of interior mutability, these snapshots differ significantly from a true reference.

The following example uses a `Mutex` (a `Send`/`Sync`, but not `Freeze` type to create UB):

```rust
let x = TCell::new(Mutex::new("hello there".to_owned()));

// ..  inside a transaction
let shallow_copy = x.borrow(tx, Default::default())?;
// Locking a shallow copy of a lock... is not really a lock at all!
// The original String is deallocated here, likely leading to double-frees.
*shallow_copy.lock().unwrap() = "uh oh".to_owned();
```

By having snapshotting functions like `borrow` require `Freeze`, such disastrous situations are prevented at compile time, without being overly restrictive, or requiring slower heap allocation based workarounds.

Similarly to the above example, `crossbeam` would be able to expand `Atomic` to include non-copy types. See [this](https://github.com/crossbeam-rs/crossbeam/issues/379) issue.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

`Freeze` is a new marker trait, similar to `Send` and `Sync`, that is intended to only be implemented for types which have no direct interior mutability, and are therefore safe to place in read only memory.

## What types are `Freeze`?

The list of `Freeze` types is long, including primitives, `String`, `Vec`, `Option<String>`, `Box<T>`, `Arc<T>`, `Rc<T>`, etc. This is because you cannot modify the memory contained directly within these types through an immutable reference.

Types that do not implement `Freeze` include types used in parallel programming such as, `Mutex<T>`, `AtomicUsize`, etc, as well as `Cell`, `RefCell`, and `UnsafeCell`. This is because their memory can be modified via an immutable reference.

## My type doesn't implement `Freeze`, but I need it to be `Freeze`.

To convert a type which is not `Freeze`, into a `Freeze` type, all that is required is to stick it on the heap. For example, `Box<T>` is `Freeze` even if `T` is an `UnsafeCell`.

If you really know what you are doing, and promise not to mutate any data in your type through an immutable reference, then you can implement `Freeze` like so:

```rust
struct MyType { /* .. */ }
unsafe impl Freeze for MyType {}
```

This requires `unsafe`, because UB is possible if in fact the memory occupied by `MyType` is mutable through an immutable reference to `MyType`.

## How do I opt-out of `Freeze`?

This is only useful when you suspect your type might, at some point in the future, include a non-`Freeze` type. To protect your users from relying on the current implementation of your type, simply add `PhantomUnfrozen` as a member to your type.

```rust
struct MyType {
    _dont_rely_on_freeze: PhantomUnfrozen,
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

`Freeze` has been privately implemented in libcore for 3 years, and has not had major changes during that time. In that time it has been relied upon for deciding whether a `static` of a type is placed in read-only static memory or writable static memory.

`Freeze` needs to be made `pub` instead of `pub(crate)`. `PhantomUnfrozen` would be a new addition.

## Implementation

`libcore/marker.rs`:
```rust
#[lang = "freeze"]
pub unsafe auto trait Freeze {}

impl<T: ?Sized> !Freeze for UnsafeCell<T> {}
unsafe impl<T: ?Sized> Freeze for PhantomData<T> {}
unsafe impl<T: ?Sized> Freeze for *const T {}
unsafe impl<T: ?Sized> Freeze for *mut T {}
unsafe impl<T: ?Sized> Freeze for &T {}
unsafe impl<T: ?Sized> Freeze for &mut T {}

pub struct PhantomUnfrozen;
impl !Freeze for PhantomUnfrozen {}
```

# Drawbacks
[drawbacks]: #drawbacks

Adding a new `auto` trait typically complicates the language and adds cognitive overhead for public crates, `Freeze` is no exception. Crate owners have to now commit to an interior mutability story, or risk breaking changes in the future.

The community desire for `Freeze` is also currently small.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This design has been relied on by rustc for 3 years, and has not required any significant maintenence, nor does this author expect there to be much maintenence after making it `pub`.

Crate owners who incidentally have `Freeze` types in their API, and wish to add in interior mutability at a later date, can do so by simply `Box`ing up any parts of their type which may be modified through an immutable reference to avoid breaking changes.

No other designs have been considered.

The impact of not doing this would be admittedly small. Users who want this feature would have to wait for `optin-builtin-traits`, use nightly rust, `Box` up data they intend to `Freeze`, or rely on `unsafe` code. This RFC author would elect to keep [`swym`](https://github.com/mtak-/swym) on nightly rust rather than pay the performance overhead of heap allocation.

# Prior art
[prior-art]: #prior-art

This feature has existed internally in libcore for 3 years without any fuss.

The D programming language has a similar feature known as [immutable references](https://dlang.org/spec/const3.html#const_and_immutable). The main difference is that `Freeze`'s immutability is not tracked across any contained pointers, like it is in D; however, they use it for similar purposes:
>  Immutable data can be placed in ROM (Read Only Memory) or in memory pages marked by the hardware as read only. Since immutable data does not change, it enables many opportunities for program optimization, and has applications in functional style programming.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

## Design questions
- Should this trait have a different name besides `Freeze`? `Freeze` was a [public API](https://github.com/rust-lang/rust/pull/13076) long ago, and its meaning has somewhat changed. This may be confusing for oldtimers and/or newcomers who are googling the trait. Additionally, `freeze` is the name of an LLVM instruction used for turning uninitialized data into a fixed-but-arbitrary data value.
- Is `PhantomUnfrozen` desirable? Users can write their own `PhantomUnfrozen` like so:
```rust
#[repr(transparent)]
struct PhantomUnfrozen(UnsafeCell<()>);
unsafe impl Sync for PhantomUnfrozen {}
```
- Should `UnsafeCell<ZeroSizedType>` implement `Freeze`? It's a situation that might possibly occur in the wild, and could be supported.

## Out of Scope
- Discussions of whether `UnsafeCell` should or could implement `Copy`.

# Future possibilities
[future-possibilities]: #future-possibilities

It's possible that the community might want a feature similar to D's "immutable references". Basically this would be `Freeze` but transitive across pointers; however, I am unsure what the use case would be.
