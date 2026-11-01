- Feature Name: Store
- Start Date: 2023-06-17
- RFC PR: [rust-lang/rfcs#3446](https://github.com/rust-lang/rfcs/pull/3446)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

#   Summary

Store offers a more flexible allocation API, suitable for in-line memory store, shared memory store, compaction of
allocations, and more.

A companion repository implementing the APIs presented here, and using them, can be explored at
https://github.com/matthieu-m/storage.


#   Motivation

The Allocator API supports many usecases, but unfortunately falls short in a number of scenarios, due to the use of
pointers.

Specifically:

-   Pointers preclude in-line memory store, ie, an allocator cannot return a pointer pointing within the allocator
    itself, as any move of the allocator instance invalidates the pointer.
-   Pointers to allocated memory cannot be returned from a const context, preventing the use of non-empty regular
    collections in const or static variables.
-   Pointers are often virtual addresses, preventing the use of non-empty regular collections in shared memory.
-   Pointers are often 32 to 64 bits, which is overkill in many situations.

The key idea of the Store API is to do away with pointers and instead return abstract, opaque, handles which can be
tailored to fit the particular restrictions of a given scenario.


#   Guide-level explanation

##  Overview

The `Store` trait is designed to allow allocating blocks of memory and referring to them by opaque handles. The handles
are not meant to be exposed directly, instead the `Store` should be used to parameterize a collection which will
internally use the store provided, and its handles, to allocate and deallocate memory as needed.

The `Store` API is very closely related to the `Allocator` API, and largely mirrors it. The important exceptions are:

-   The `Handle` returned is opaque, and must be resolved into pointers by the instance of `Store` which allocated it,
    in general.
-   The `StoreDangling` super trait, which allows acquiring a `dangling` handle, which can safely be resolved into a
    well-aligned pointer, if an invalid one.
-   Unless a specific store type implements `StoreStable`, there is no guarantee that resolving the same handle after
    calling another method on the API -- including `resolve` with a different handle -- will return the same pointer. In
    particular, a call to `resolve` may lead to cache-eviction (think LRU), an allocation may result in reallocating the
    entire block of memory used underneath by the `Store`, and a deallocation may result in consolidating existing
    allocations (GC style).
-   Unless a specific store type implements `StorePinning`, there is no guarantee that resolving the same handle after
    moving the store will return the same pointer.


##  Points of View

There are 3 point of views when it comes to using the `Store` API:

-   The user, who gets to mix and match collection and store based on their usecase.
-   The implementer of a collection parameterized over `Store`.
-   The implementer of a `Store`.

Check each section according to your usecase.


##  User Guide

As a developer for latency-sensitive code, using an in-line store allows me to avoid the latency uncertainty of memory
allocations, as well as the extra latency uncertainty of accessing a different cache line.

This is as simple as parameterizing the collection I wish to use with an appropriate in-line store.

```rust
use core::{collections::Vec, string::String};

//  A store parameterized by a type `T`, which provides a single block of memory suitable for `T`, that is: at least
//  aligned for `T` and sized for `T`.
use third_party::InlineSingleStore;

type InlineString<const N: usize> = String<InlineSingleStore<[u8; N]>>;
type InlineVec<T, const N: usize> = Vec<T, InlineSingleStore<[T; N]>>;

//  A struct keeping the N greatest values of `T` submitted, and discarding all others.
pub struct MaxPick<T, const N: usize>(InlineVec<T, N>);

impl<T, const N: usize> MaxPick<T, N> {
    pub fn new() -> Self {
        Self(InlineVec::with_capacity(N))
    }

    pub fn as_slice(&self) -> &[T] { &self.0 }

    pub fn clear(&mut self) { self.0.clear(); }
}

impl<T: Ord, const N: usize> MaxPick<T, N> {
    pub fn add(&mut self, value: T) {
        if N == 0 {
            return;
        }

        if let Some(last) = self.0.get(N - 1) {
            if *last >= value {
                return;
            }

            self.0.pop();
        }

        self.0.push_within_capacity(value);
        self.0.sort();
    }
}
```

As a developer for performance-sensitive code, using a small store allows me to avoid the cost of memory allocations in
the majority of cases, whilst retaining the flexibility of unbounded allocations when I need them.

```rust
use std::future::Future;

//  A store parameterized by a type `T`, which provides an in-line block of memory suitable for `T` -- that is at least
//  aligned for `T` and sized for `T` -- and otherwise defaults to a heap allocation.
use third_party::SmallSingleStore;

//  A typed-erased future:
//  -   If the future fits within `[usize; 3]`, apart from its metadata, no memory allocation is performed.
//  -   Otherwise, the global allocator is used.
pub type RandomFuture = Box<dyn Future<Output = i32>, SmallSingleStore<[usize; 3]>>;

pub trait Randomizer {
    fn rand(&self) -> RandomFuture;
}

pub struct FairDie;

impl Randomizer for FairDie {
    fn rand(&self) -> RandomFuture {
        Box::new(async { 4 })
    }
}

pub struct CallHome;

impl Randomizer for CallHome {
    fn rand(&self) -> RandomFuture {
        Box::new(async {
            //  Connect to https://example.com
            //  Hash the result.
            //  Truncate the hash to fit.
            todo!()
        })
    }
}
```

In either case, this allows me to reuse battle-tested collections, and all the ecosystem built around them, rather than
having to implement, or depend on, ad-hoc specialized collections which tend to lag behind in terms of soundness and/or
features.

It also allows me to use the APIs I am used to, rather than slightly different APIs for each specific situation, thereby
allowing me to extert maximum control and extract maximum performance from my code without compromising my productivity.


##  Collection Implementer Guide

As an implementer of collection code, using the `Store` abstraction gives maximum flexibility to my users as to how
they'll be able to use my collection.

```rust
pub struct Either<L, R, S: Store> {
    is_left: bool,
    handle: S::Handle,
    store: ManuallyDrop<S>,
}

impl<L, R, S: Store> Either<L, R, S> {
    pub fn left(value: L) -> Result<Self, AllocError>
    where
        S: Default,
    {
        let store = ManuallyDrop::new(S::default());
        let (handle, _) = store.allocate(Layout::new::<L>())?;

        //  Safety:
        //  -   `handle` was allocated by `store`.
        //  -   `handle` is still valid.
        let pointer = unsafe { store.resolve(handle) };

        //  Safety:
        //  -   `pointer` points to a block of memory fitting `value`.
        //  -   `pointer` points to a writeable block of memory.
        unsafe { ptr::write(pointer.cast().as_ptr(), value) };

        Ok(Self { is_left: true, handle, store })
    }

    pub fn as_left(&self) -> Option<&L> {
        self.is_left.then(|| {
            //  Safety:
            //  -   `handle` was allocated by `store`.
            //  -   `handle` is still valid.
            let pointer = unsafe { self.store.resolve(self.handle) };

            //  Safety:
            //  -   `pointer` points to a live instance of `L`.
            //  -   The reference will remain valid for its entire lifetime, since it borrows `self`, thus preventing
            //      any allocation via or move or destruction of `self.store`.
            //  -   No mutable reference to this instance exists, nor will exist during the lifetime of the resulting
            //      reference, since the reference borrows `self`.
            unsafe { pointer.as_ref() }
        })
    }

    pub fn into_left(mut self) -> Option<core::boxed::Box<L, S>> {
        self.is_left.then(|| {
            let handle = self.handle;

            //  Safety:
            //  -   `self.store` will no longer be used.
            let store = unsafe { ManuallyDrop::take(&mut self.store) };

            mem::forget(self);

            //  Safety:
            //  -   `handle` was allocated by `store`.
            //  -   `handle` is valid.
            //  -   The block of memory associated to `handle` contains a live instance of `L`.
            unsafe { core::boxed::Box::from_raw_parts(handle, store) }
        })
    }

    //  And implementations of `as_left_mut`, `right`, `as_right`, `as_right_mut`, ...
}

impl<L, R, S: Store> Drop for Either<L, R, S> {
    fn drop(&mut self) {
        //  Safety:
        //  -   `handle` was allocated by `store`.
        //  -   `handle` is still valid.
        let pointer = unsafe { self.store.resolve(self.handle) };

        if self.is_left {
            let pointer: *mut L = pointer.cast().as_ptr();

            //  Safety:
            //  -   `pointer` is valid for both reads and writes.
            //  -   `pointer` is properly aligned.
            unsafe { ptr::drop_in_place(pointer) }
        } else {
            let pointer: *mut R = pointer.cast().as_ptr();

            //  Safety:
            //  -   `pointer` is valid for both reads and writes.
            //  -   `pointer` is properly aligned.
            unsafe { ptr::drop_in_place(pointer) }
        };

        let layout = if self.is_left {
            Layout::new::<L>()
        } else {
            Layout::new::<R>()
        };

        //  Safety:
        //  -   `self.store` will no longer be used.
        let store = unsafe { ManuallyDrop::take(&mut self.store) };

        //  Safety:
        //  -   `self.handle` was allocated by `self.store`.
        //  -   `self.handle` is still valid.
        //  -   `layout` fits the block of memory associated to `self.handle`.
        unsafe { store.deallocate(self.handle, layout) }
    }
}
```

By using `Store`, I empower my users to use my type in a wide variety of scenarios, some of which I cannot even fathom.

The overhead of using `Store` over `Allocator` is also fairly low, so that the added flexibility comes at little to no
cost to me.

More examples of collections can be found at https://github.com/matthieu-m/storage/tree/main/src/collection, including
a complete linked list, a box draft, a concurrent vector draft, and a skip list draft.


##  Store Implementer Guide

The API of `StoreSingle` only requires that `resolve` and `resolve_mut` resolve to the same pointer. I can otherwise
provide as few or as many guarantees as I wish.

```rust
/// An implementation of `Store` providing a single, inline, block of memory.
///
/// The block of memory is aligned and sized as per `T`.
pub struct InlineSingleStore<T>(MaybeUninit<T>);

impl<T> InlineSingleStore<T> {
    /// Creates a new instance.
    pub const fn new() -> Self {
        Self(MaybeUninit::uninit())
    }
}

impl<T> Default for InlineSingleStore<T> {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl<T> const StoreDangling for InlineSingleStore<T> {
    type Handle = ();

    fn dangling(&self, alignment: Alignment) -> Result<Self::Handle, AllocError> {
        if alignment.as_usize() <= Alignment::of::<T>().as_usize() {
            Ok(())
        } else {
            Err(AllocError)
        }
    }
}

unsafe impl<T> const StoreSingle for InlineSingleStore<T> {
    unsafe fn resolve(&self, _handle: Self::Handle) -> NonNull<u8> {
        let pointer = self.0.as_ptr() as *mut T;

        //  Safety:
        //  -   `self` is non null.
        unsafe { NonNull::new_unchecked(pointer) }.cast()
    }

    unsafe fn resolve_mut(&mut self, _handle: Self::Handle) -> NonNull<u8> {
        let pointer = self.0.as_mut_ptr();

        //  Safety:
        //  -   `self` is non null.
        unsafe { NonNull::new_unchecked(pointer) }.cast()
    }

    fn allocate(&mut self, layout: Layout) -> Result<(Self::Handle, usize), AllocError> {
        if Self::validate_layout(layout).is_err() {
            return Err(AllocError);
        }

        Ok(((), mem::size_of::<T>()))
    }

    unsafe fn deallocate(&mut self, _handle: Self::Handle, _layout: Layout) {}

    unsafe fn grow(
        &mut self,
        _handle: Self::Handle,
        _old_layout: Layout,
        new_layout: Layout,
    ) -> Result<(Self::Handle, usize), AllocError> {
        debug_assert!(
            new_layout.size() >= _old_layout.size(),
            "new_layout must have a greater size than _old_layout"
        );

        if Self::validate_layout(new_layout).is_err() {
            return Err(AllocError);
        }

        Ok(((), mem::size_of::<T>()))
    }

    unsafe fn shrink(
        &mut self,
        _handle: Self::Handle,
        _old_layout: Layout,
        _new_layout: Layout,
    ) -> Result<(Self::Handle, usize), AllocError> {
        debug_assert!(
            _new_layout.size() >= _old_layout.size(),
            "_new_layout must have a smaller size than _old_layout"
        );

        Ok(((), mem::size_of::<T>()))
    }

    fn allocate_zeroed(&mut self, layout: Layout) -> Result<(Self::Handle, usize), AllocError> {
        if Self::validate_layout(layout).is_err() {
            return Err(AllocError);
        }

        let pointer = self.0.as_mut_ptr() as *mut u8;

        //  Safety:
        //  -   `pointer` is valid, since `self` is valid.
        //  -   `pointer` points to at an area of at least `mem::size_of::<T>()`.
        //  -   Access to the next `mem::size_of::<T>()` bytes is exclusive.
        unsafe { ptr::write_bytes(pointer, 0, mem::size_of::<T>()) };

        Ok(((), mem::size_of::<T>()))
    }

    unsafe fn grow_zeroed(
        &mut self,
        _handle: Self::Handle,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<(Self::Handle, usize), AllocError> {
        debug_assert!(
            new_layout.size() >= old_layout.size(),
            "new_layout must have a greater size than old_layout"
        );

        if Self::validate_layout(new_layout).is_err() {
            return Err(AllocError);
        }

        let pointer = self.0.as_mut_ptr() as *mut u8;

        //  Safety:
        //  -   Both starting and resulting pointers are in bounds of the same allocated objects as `old_layout` fits
        //      `pointer`, as per the pre-conditions of `grow_zeroed`.
        //  -   The offset does not overflow `isize` as `old_layout.size()` does not.
        let pointer = unsafe { pointer.add(old_layout.size()) };

        //  Safety:
        //  -   `pointer` is valid, since `self` is valid.
        //  -   `pointer` points to at an area of at least `mem::size_of::<T>() - old_layout.size()`.
        //  -   Access to the next `mem::size_of::<T>() - old_layout.size()` bytes is exclusive.
        unsafe { ptr::write_bytes(pointer, 0, mem::size_of::<T>() - old_layout.size()) };

        Ok(((), mem::size_of::<T>()))
    }
}

//  Safety:
//  -   `self.resolve(handle)` always returns the same address, as long as `self` doesn't move.
unsafe impl<T> StoreStable for InlineSingleStore<T> {}

impl<T> fmt::Debug for InlineSingleStore<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        let layout = Layout::new::<T>();

        f.debug_struct("InlineSingleStore")
            .field("size", &layout.size())
            .field("align", &layout.align())
            .finish()
    }
}

//  Safety:
//  -   Self-contained, so can be sent across threads safely.
unsafe impl<T> Send for InlineSingleStore<T> {}

//  Safety:
//  -   Immutable (by itself), so can be shared across threads safely.
unsafe impl<T> Sync for InlineSingleStore<T> {}

impl<T> InlineSingleStore<T> {
    const fn validate_layout(layout: Layout) -> Result<(), AllocError> {
        let own = Layout::new::<T>();

        if layout.align() <= own.align() && layout.size() <= own.size() {
            Ok(())
        } else {
            Err(AllocError)
        }
    }
}
```

And that's it!

I need not implement `StoreMultiple`, and thus do not have to track allocations and deallocations. And I need not
implement `StorePinning`, and thus do not have to ensure that memory remains pinned.

More examples of `Store` can be found at https://github.com/matthieu-m/storage/tree/main/src/store, including an inline
bump store.


#   Reference-level explanation

##  Overview

This RFC introduces 5 new traits.

The core of this RFC is the `Store` trait, with `StoreDangling` as its super-trait:

```rust
/// A trait allowing to get a dangling handle.
pub unsafe trait StoreDangling {
    /// Handle to a block of memory.
    type Handle: Copy;

    /// Returns a dangling handle.
    ///
    /// A dangling handle can be resolved to a pointer of at least the specified alignment, but the resulting pointer is
    /// invalid
    fn dangling(&self, alignment: Alignment) -> Result<Self::Handle, AllocError>;
}

/// Allocates and deallocates handles to blocks of memory, which can be temporarily resolved to pointers to actually
/// access said memory.
pub unsafe trait Store: StoreDangling {
    /// Returns a pointer to the block of memory associated to `handle`.
    unsafe fn resolve(&self, handle: Self::Handle) -> NonNull<u8>;

    //  The following methods are similar to `Allocator`, reformulated in terms of `Self::Handle`.

    /// Allocates a new handle to a block of memory.
    fn allocate(&self, layout: Layout) -> Result<(Self::Handle, usize), AllocError>;

    /// Deallocates a handle.
    unsafe fn deallocate(&self, handle: Self::Handle, layout: Layout);

    /// Grows the block of memory associated to a handle. On success, the handle is invalidated.
    unsafe fn grow(
        &self,
        handle: Self::Handle,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<(Self::Handle, usize), AllocError>;

    /// Shrinks the block of memory associated to a handle. On success, the handle is invalidated.
    unsafe fn shrink(
        &self,
        handle: Self::Handle,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<(Self::Handle, usize), AllocError>;

    /// Allocates a new handle to a block of zeroed memory.
    fn allocate_zeroed(&self, layout: Layout) -> Result<(Self::Handle, usize), AllocError> {
        ...
    }

    /// Grows the block of memory associated to a handle with zeroed memory. On success, the handle is invalidated.
    unsafe fn grow_zeroed(
        &self,
        handle: Self::Handle,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<(Self::Handle, usize), AllocError> {
        ...
    }
}
```

_Note:  full-featured documentation of the trait and methods can be found in the companion repository at_
        https://github.com/matthieu-m/store/blob/main/src/interface.rs.

A specialized form of the `Store` trait exists for stores only supporting a single allocation at a time.

```rust
/// Allocates and deallocates a single handle to a single block of allocated memory at a time, which can be resolved to
/// a pointer to actually access said memory.
pub unsafe trait StoreSingle: StoreDangling {
    /// Returns a (const) pointer to the block of memory associated to `handle`.
    unsafe fn resolve(&self, handle: Self::Handle) -> NonNull<u8>;

    /// Returns a (mut) pointer to the block of memory associated to `handle`.
    unsafe fn resolve_mut(&mut self, handle: Self::Handle) -> NonNull<u8>;

    //  The following methods are similar to `Allocator`, reformulated in terms of `Self::Handle`.

    /// Allocates a new handle to a block of memory.
    fn allocate(&mut self, layout: Layout) -> Result<(Self::Handle, usize), AllocError>;

    /// Deallocates a handle.
    unsafe fn deallocate(&mut self, handle: Self::Handle, layout: Layout);

    /// Grows the block of memory associated to a handle. On success, the handle is invalidated.
    unsafe fn grow(
        &mut self,
        handle: Self::Handle,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<(Self::Handle, usize), AllocError>;

    /// Shrinks the block of memory associated to a handle. On success, the handle is invalidated.
    unsafe fn shrink(
        &mut self,
        handle: Self::Handle,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<(Self::Handle, usize), AllocError>;

    /// Allocates a new handle to a block of zeroed memory.
    fn allocate_zeroed(&mut self, layout: Layout) -> Result<(Self::Handle, usize), AllocError>;

    /// Grows the block of memory associated to a handle with zeroed memory. On success, the handle is invalidated.
    unsafe fn grow_zeroed(
        &mut self,
        handle: Self::Handle,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<(Self::Handle, usize), AllocError>;
```

The `Store` and `StoreSingle` traits are supplemented by 2 additional marker traits, providing extra guarantees:

```rust
/// A refinement of store which does not invalidate existing pointers on allocation, resolution, or deallocation, but
/// may invalidate them on moves.
pub unsafe trait StoreStable {}

/// A refinement of store which does not invalidate existing pointers, not even on moves. That is, this refinement
/// guarantees that the blocks of memory are pinned in memory until the instance of the store is dropped, or until
/// the lifetime bound of store concrete type expires, whichever comes first.
pub unsafe trait StorePinning: StoreStable {}
```


##  Safety & Guarantees

The `Store` trait is used to manage non-overlapping blocks of memory through opaque `Handle`s, temporarily resolving a
`Handle` to the address of the block of memory as needed.

There are therefore essentially 3 moving pieces to keep track of: handles, pointers resolved from those handles, and
the operations that may be soundly executed on those.


### Handles

A `Handle` may be in one of 3 states:

-   Invalid: it is Undefined Behavior to call any method of a store with this handle.
-   Valid, but Dangling: the handle can be _resolved_ into a pointer, but the resulting pointer itself is dangling.
-   Valid: the handle can be used with any method of a store, and the pointer it _resolves_ into can be used to access
    the associated memory block.

Creation of a `Handle`:

-   `dangling` produces a Valid, but Dangling, handle. This handle may then become Invalid as usual.
-   `allocate`, `allocate_zeroed`, `grow`, `grow_zeroed`, and `shrink` produce a Valid handle. This handle may then
    become Invalid as usual.
-   A copy of a handle may be made by replicating its bitwise state, in any way. All copies of a handle share the same
    state, at any time.

All handles created by a specific instance of a store, and the copies of those handles, are associated to this one
instance and no other, unless otherwise specified.

Invalidation of a `Handle`:

-   Calling `allocate`, `allocate_zeroed`, `grow`, `grow_zeroed`, `shrink`, or `deallocate` on an instance of `Store`
    shall NOT invalidate any existing handle associated to this instance.
    -   On the other, calling those methods on an instance of `StoreSingle` which does not _also_ implement `Store` may
        invalidate any and all instances associated to this instance.
-   A handle is immediately invalidated when used as an argument to `deallocate`.
-   A handle is invalidated in case of success when used as an argument to `grow`, `grow_zeroed` or `shrink`. In case of
    failure, the handle remains Valid.

An instance of a store may provide extended guarantees.


### Pointers

Creation of a `NonNull<u8>` from a `Handle`:

-   A Valid but Dangling handle may be _resolved_ into a pointer via `resolve` or `resolve_mut`. The resulting pointer
    is itself dangling, as if obtained by `NonNull::dangling`.
-   A Valid handle may be _resolved_ into a pointer via `resolve` or `resolve_mut`. The resulting pointer is valid, and
    points to the first byte of the block of memory associated to the handle.
-   A Valid possibly Dangling handle may be _resolved_ into a pointer by other means, such as the `Into` or `TryInto`
    traits. The resulting pointer must be equal to the result of calling `resolve` or `resolve_mut` with the handle.

All pointers resolved from a handle, or any of its copies, share the same state, at any time.

All pointers resolved from a handle associated to a specific instance of a store are themselves associated to this one
instance and no other, unless otherwise specified.

Invalidation of a `NonNull<u8>`:

-   All pointers resolved from a handle are invalidated when this handle is invalidated.
-   All pointers associated to an instance of a store may be invalidated by dropping this instance.
-   All pointers associated to an instance of a store may be invalidated by moving this instance, unless otherwise
    specified.
    -   An instance of `StorePinning` does not invalidate existing pointers on moves.
-   All pointers associated to an instance of a store may be invalidated when calling any of `allocate`,
    `allocate_zeroed`, `grow`, `grow_zeroed`, `shrink`, or `deallocate` on this instance, unless otherwise specified.
    -   An instance of `StoreStable` does not invalidate existing pointers on those calls.
-   All pointers associated to an instance of a store may be invalidated when calling `resolve` on this instance, unless
    otherwise specified.
    -   Pointers resolved from a copy of the handle passed to `resolve` are not invalidated.
    -   An instance of `StoreStable` does not invalidate existing pointers on those calls.

An instance of a store may provide extended guarantees, such as instances of a store also implementing `StoreStable` or
`StorePinning` do.


### Consistency

When multiple methods can be used to achieve the same task, they should result in the same result. Specifically:

-   `Store::resolve`, `StoreSingle::resolve` and `StoreSingle::resolve_mut` should resolve the same handle into the same
    pointer.
-   When a method exists both for `Store` and `StoreSingle`, calling one or the other should have the same effect.

It is recommended that when a type implements both `Store` and `StoreSingle` all the methods of `StoreSingle` simply
delegate to the methods of `Store`, to ensure consistency in their behavior.


##  Library Organization

This RFC proposes to follow the lead of the `Allocator` trait, and add the `Store` traits to the `core` crate, either in
the `alloc` module or in a new `store` module.

It leaves to a follow-up RFC the introduction of a `store` or `store-collections` crate which would contain the code of
the various standard collections: `Box`, `Vec`, `String`, `BinaryHeap`, `BTreeMap`, `BTreeSet`, `LinkedList`, and
`VecDeque`, all adapted for the `Store` API.

Those types would then be re-exported as-is in the `alloc` crate, drastically reducing its size.


#   Drawbacks

This RFC increases the surface of the standard library, with yet another `Allocator`.

Furthermore, the natural consequence of adopting this RFC would be rewriting the existing collections in terms of
`Store` or `StoreSingle`, rather than `Allocator`. A mostly mechanical task, certainly, but an opportunity to introduce subtle bugs in the process, even if Miri would hopefully catch most such bugs.

Finally, having two allocator-like APIs, `Store` and `Allocator`, means that users will forever wonder which trait they
should implement[^1], and which trait they should use when implementing a collection[^2].

[^1]: Implement `Allocator` if you plan to return pointers, it's simpler, and `Store` otherwise.
[^2]: Use `Store` to parameterize your collections, it gives more flexibility to your users.


#   Rationale and Alternatives

##  Don't Repeat Yourself.

The fact that `Allocator` is unsuitable for many usecases is amply demonstrated by the profileration of ad-hoc rewrites
of existing `std` types for particular scenarios. A non-exhaustive list of crates seeking to work around those short-
comings today is presented here:

-   https://crates.io/crates/arraystring
-   https://crates.io/crates/arrayvec
-   https://crates.io/crates/const-arrayvec
-   https://crates.io/crates/const_lookup_map
-   https://crates.io/crates/generic-vec
-   https://crates.io/crates/phf
-   https://crates.io/crates/smallbox2
-   https://crates.io/crates/stackbox
-   https://crates.io/crates/stackfuture
-   https://crates.io/crates/string-wrapper
-   https://crates.io/crates/toad-string

Those are the alternatives to `Store`: rather than adapting a data-structure flexible enough to be used in various
situations, the entire data-structure is copy/pasted and then tweaked as necessary or re-implemented. The downsides are
inherent to any violation of DRY:

-   Bugs or soundness issues may be introduced, or may not be fixed when fixed in the "original".
-   The new types are not compatible with APIs taking the standard types.
-   The new types do not implement the latest features implemented by the standard types.
-   The new types do not implement all the traits implemented by the standard types, most especially 3rd-party traits.

There are a few advantages to brand new types. For example, a nul-terminated in-line string does not have the 16 bytes
overhead that a "naive" `String<InlineSingleStore<[u8; N]>>` would have. The Store API will not eliminate the potential
need for such specialized collections, but it may eliminate the need for most alternatives in most situations.


##  Allocator-like

The API of Stores is intentionally kept very close to that of `Allocator`.

This similarility, and the similarity of the safety requirements, means that any developer accustomed to the current
`Allocator` API can quickly dive in, and also means that bridging between `Store` and `Allocator` is as easy as
possible.

There are 3 extra pieces:

-   The `Handle` associated type is used, rather than `NonNull<u8>`. This is the key to the flexibility of `Store`.
-   The `dangling` method, reminiscent of `NonNull::dangling`, and to be used for the same purposes. It is part of
    `Store` as the maximum available alignment may be limited by the type of `Store`.
-   The `resolve` and `resolve_mut` methods, which bridges from `Handle` to `NonNull<u8>`, since access to the allocated
    blocks of memory require pointers to them.

Otherwise, the bulk of the API is a straightforward translation of the `Allocator` API, substituting `Handle` anytime
`NonNull<_>` appears.


##  Allocator-like (bis)

The API of Stores being intentionally kept very close to that of `Allocator` means that some questions worth asking on
the `Allocator` API are also worth asking here:

-   Should allocation methods return the actual allocated size? In theory, this may allow optimizations, in practice
    it seems unimplemented by `Allocator` and unused by callers.
-   Should we have `reallocate` rather than `grow`/`shrink`?
-   Should we have `try_grow`/`try_shrink` (in-place only)?

It may be best to consider those questions orthogonal to this RFC, they can be resolved at once for both APIs.

_Note: the companion repository extensions do use the allocated size: for inline vectors, it allows immediately setting
        the maximum capacity once and for all._


##  Guarantees, or absence thereof

The Stores API is minimalist, providing a minimum of guarantees.

Beyond being untyped and unchecked, there are also a few oddities, compared to the `Allocator` API:

-   By default, calling any method -- including `resolve` -- invalidates all resolved pointers[^1]. This oddity stems
    from the desire to leave the API flexible enough to allow caching stores, single-allocation stores, or copying
    stores.
-   By default, moving `Store` invalidates all resolved pointers. This oddity stems from the fact that when using an
    in-line store the pointers point within the block of memory covered by the store, and thus after it moved, are left
    pointing into the void.

When the above should be guaranteed, extra marker traits can be implemented to provide compile-time checking that these
properties hold, which in turn allows the final user to safely mix and match collection and store, relying on compiler
errors to prevent erroneous couplings.

[^1]: With the exception, in the case of a call to `resolve`, of any pointer derived from a copy of the handle argument.


##  Store, Allocator, and Aliasing

The raison d'être of a Store or Allocator API is to allocate blocks of memory for their user, in general by carving them
out of a bigger block of memory. While the resulting blocks of memory are all non-overlapping, by necessity they do overlap with the block they are carved out of. Aliasing has entered the chat.

The Rust language, and the LLVM IR, have strict rules governing aliasing. In particular, `&mut` references and `noalias`
represent a guarantee of _no aliasing_. Implementations of a Store or Allocator must strive to honor those rules, or be
unsound.

In particular, it is unsound to obtain an `&mut` reference to the block of memory allocations are carved out of while at
the same time having an `&mut` reference to one of the carved out memory allocations, since they overlap. As a result,
any block of memory to carve out allocations of must either:

-   Be accessed only via a pointer, not a reference.
-   Be accessed only via `&UnsafeCell`.

The _one_ exception to the above is that blocks of memory with no active oustanding allocation can be accessed via
`&mut` references.

This inherent modelling limitation drives the existing of the mostly parallel `Store` and `StoreSingle` traits:

-   `Store` uses `&` references _exclusively_ in its API so that it is sound to call its methods even in the presence of
    active outstanding `&mut` references to associated allocated memory blocks.
-   `StoreSingle` uses `&mut` references as appropriate, shielded by the fact that with a single outstanding allocation
    at a time, there cannot be an active outstanding `&mut` reference to an associated allocated memory block when its
    methods are called.


##  Is `StoreSingle` worth it?

The API of `StoreSingle` is _very_ similar to that of `Store`. The differences being:

-   Different `resolve` and `resolve_mut`.
-   Other methods taking `&mut` instead of `&`, and offering weaker guarantees.

There is a cost in having two separate-but-so-similar APIs, is this cost worth it?

Any implementation of `Store` using in-line memory _must_ use `UnsafeCell` to wrap the in-line memory in order to be
able to obtain an `&mut` reference to an associated allocated memory block starting from an `&` reference to the store
itself.

This seemingly innocuous requirement, unfortunately, does not play well with LLVM's attributes. `noalias`, `readonly`,
`writeonly`, etc... are not granular: they apply to the entire span of memory pointed to when specified. Hence, any time
a span of memory contains `UnsafeCell`, these attributes cannot be specified, and therefore `UnsafeCell` "infects" any
struct it is a field of, recursively.

One of the main motivations of this RFC is to allow in-line collections, and in particular `InlineBox` and `InlineVec`.
Those types _can_ be built on top of `Store`, but at the cost of inner `UnsafeCell`, and the unfortunate ripple effects
at the LLVM IR level.

As noted in _Store, Allocator, and Aliasing_, however, `UnsafeCell` is only necessary in the presence of active
outstanding allocations. Yet, a number of collections such as `Box`, `Vec`, `VecDeque`, or `HashMap` only ever require
a _single_ allocation at a time.

And while the set of collections only requiring a single allocation at a time is small, these few types are the most
widely used in the wild!

It seems, therefore, worth it to provide a specialized API avoiding the optimization woes of `UnsafeCell`.


##  Typed Handles

A previous incarnation of the API used GATs to provide typed handles.

This is tempting, but I now deem it a mistake, most notably thanks to discussions with @CAD97 on the topic.

Specifically:

1.  A user may wish to allocate raw memory, for example to further parcel it themselves. Thus any API must, in any
    case, offer the ability to allocate raw memory. Providing a typed API on top means doubling the API surface.
2.  Typing can easily be added externally. See the `TypedHandle` possible future extension, which is non-intrusive
    and can be implemented in 3rd-party code.

And the final nail in the coffin, for me, is that even typed handles would not make the API safe. There are many other
invariants to respect -- handle invalidation, pointer invalidation, liveness of the value, borrow-checking -- which
would require the `unsafe` methods to remain `unsafe`.

In comparison to tracking all that, types are a minor concern: in most collections, there's a single type _anyway_.


##  Pointer vs Reference

A previous incarnation of the API provided borrow-checking. That is, resolving a handle would yield a reference and
appropriately borrow the store.

This is tempting, but I deem it a mistake.

Specifically:

1.  A mutably borrowed store cannot allocate, deallocate, nor further resolve any other reference. This makes
    implementing any recursive data-structure -- such as a tree -- quite a bit more challenging than it ought to be.
2.  A reference requires a type, for a fat reference this means metadata. Requiring the metadata to be provided when
    calling `resolve` precludes the use of thin handles, which are quite welcome in graphs of objects with a great
    number of copies of each handle.

And the final nail in the coffin, for me, is that borrow-checking is best provided at the _handle_ level, rather than
at the store level. The `UniqueHandle` possible future extension, which is non-intrusive and can be implemented in
3rd-party code:

-   Borrows the (unique) handle mutably or immutably, ensuring no misgotten access is provided.
-   Borrows the store immutably, ensuring it is not moved nor dropped, which would potentially invalidate the pointers.

This solution is more flexible, and more minimalist, generally a good sign with regard to API design.


##  Resolve with or without Layout

There is an inherent trade-off in `Store::resolve`:

-   Not requiring the layout allows thin pointers (`ThinBox`) as well as untyped manipulations (`RawTable` style).
-   Requiring a layout allows optimized `SmallSingleStore` optimizations which decide whether to resolve to in-line
    memory or off-line memory based on the layout, without having to store the state in the store or handle.

The current API leans in favor of thin pointers and untyped manipulations as they seem slightly more common, but it is
debatable.

Another possibility would be to provide _both_, letting the implementer decide which it can support, and the collection
which it requires. This would likely mean introducing another 2 traits, though, something like:

```rust
trait StoreResolver: Store {
    fn resolve(&self, handle: Self::Handle) -> NonNull<u8>;
}

trait StoreLayoutResolver: Store {
    fn resolve_with_layout(&self, handle: Self::Handle, layout: Layout) -> NonNull<u8>;
}
```

Most `Store` would implement both -- any store which can resolve without layout should be able to resolve with layout,
after all -- but `SmallSingleStore` would only implement the latter.

Introducing those two traits, though, is then a trade-off of flexbility vs API-surface and ease of use. Most notably,
a potential trap for collection implementers is that switching from one to the other would be a breaking change, which
may prevent introducing an untyped core to their collection to reduce monomorphization bloat, for example.


##  `StoreDangling` independent trait.

A previous version of the companion repository associated the `Handle` and `dangling` method directly to the `Store`
trait.

A separate trait adds some complexity for implementers of stores and collections alike, yet it seems the best compromise
as of today.

Firstly, there is desire for `Vec::new()` (and similar) to be `const`[^1], so that they can be easily stored in `static`
variables without requiring `OnceCell` or similar. This, in turn, requires `dangling` to be `const`, much like
`NonNull::dangling` is `const` today.

A `const` dangling, however, is no simple feat:

1.  Trait associated functions cannot be `const`, today.
    -   Even so, it may not be possible to have _conditional_ `const` associated functions.
2.  For flexibility, it should be possible for `Store` implementations NOT to be `const`, with a `const` `dangling`.
    -   In particular, it should be noted that the `System` allocations cannot easily be `const`.

In light of the above, one simple solution emerges: a separate, `StoreDangling` trait.

_Note: see unresolved questions about `Store::dangling`._

[^1]: _A separate `StoreDangling` is not sufficient, the `Default` trait needs to be marked `#[const_trait]` as well._


##  `StoreDangling` super-trait

There are multiple options to weave `StoreDangling` with the other traits:

-   It could stand apart, entirely.
-   It could have `Store` as a super-trait.
-   It can, as of now, be a super-trait of `Store`.

As of today, a `dyn` trait type can only feature one trait with associated methods, hence having `StoreDangling` stand
apart is incompatible. This could be resolved at some point in the future, certainly, but would be one more impediment
to the implementation of this RFC.

Given that `StoreDangling` provides a _much_ simpler functionality than `Store`, it seems sensible to make it a basis on
which `Store` is built, rather than the contrary.

In particular, while `const Trait` is still under-developed, it could be reasonable in the future to require that a type
may only implement a `const Trait` if and only if it implements all its super-traits constly.


##  Alignment-enabled dangling method

A previous version of the companion repository used first an argument-less `Store::dangling` method, then a version
taking `&self` for `dyn Store` friendliness.

At the moment, the `NonNull::dangling` API provides an _aligned_ dangling pointer, which neither of the previous
versions allowed. The fact that the dangling pointer is aligned is crucial for the performance of `Vec::as_ptr`, which
can blissfully return the pointer without any check, and in turn enable branchless `as_slice`, `get_unchecked`, etc...
(credits @scottcm for pointing this out).

In order to avoid pessimizing such a core operation, it is thus crucial that `StoreDangling::dangling` provide a handle
which can be resolved into a sufficiently aligned pointer:

-   Since `StoreDangling::dangling` is untyped, this requires passing `Alignment` as an argument.
-   Since the alignments that can be provided dependent on the alignment of `Store` itself -- for inline stores -- this
    requires passing `&self` as an argument.

Furthermore, the latter point leads to `dangling` being fallible: a store may not be able to guarantee pointers aligned
to large alignments.


##  Adapter vs Blanket Implementation

A previous version of the companion repository used an `AllocatorStore` adapter struct, instead of a blanket
implementation.

There does not seem to be any benefit to doing so, and it prevents using collections defined in terms of a `Store`
directly with an `Allocator`:

-   Either library makers suffer when collections switch from `Allocator` to `Store`, having to use a feature to wrap
    or not since there's no "capability" concept.
-   Or type aliases are used to preserve the `Allocator`-based API in `collections` and `std`, but then using the
    `Store`-based API requires reaching for `core` directly which is odd.


##  Marker granularity

As a reminder, there are 2 marker traits:

-   `StoreStable`: ensures existing pointers remain valid across calls to the API methods.
-   `StorePinning`: ensures existing pointers remain valid even across moves.


Those traits could be merged, or further split.

I would suggest not splitting any further now. Taken to the extreme a marker trait could be introduced for each
guarantee and each operation, for a total of 10 marker traits: 2 guarantees x 4 groups of methods + 2 guarantees on
moves. Such a fine-grained approach is used in C++, and I remember writing generic methods which would static-assert
that the elements they manipulate need be noexcept-movable, noexcept-move-assignable, and noexcept-destructible, then
further divide the method based on whether the elements were noexcept-copyable and trivially copyable. There always is
the nagging doubt of having missed one key guarantee, and therefore while conductive to writing finely tuned code, it
is unfortunately not conductive to writing robust code: the risk of error is too high.

The current set of traits has thus been conceived to provide a reasonable trade-off:

-   A small enough number of markers that developers of collections are not overwhelmed, and thus less likely to miss
    a key requirement leading to unsoundness in their unsafe code.
-   A split on "natural" boundaries: one hierarchy for handle invalidation and one hierarchy for pointer invalidation.

From there, feedback can be gathered as to whether further splitting or merging should be considered before
stabilization.


#   Prior Art

##  C++

In C++, `std::allocator_traits` allows one to create an Allocator which uses handles that are not (strictly) pointers.

The impetus behind this design was to allow greater flexibility, much like this proposal, unfortunately it failed
spectacularly:

1.  While one can specify a non-pointer `pointer` type, this type MUST still be pointer-like: it must be
    dereferenceable, etc... This requirement mostly requires the type to embed a pointer -- possibly augmented -- and
    thus makes it unsuitable for in-line store, unsuitable for compaction, and only usable for shared memory usage
    with global/thread-local companion state.
2.  While one can specify a non-reference `reference` type, the lack of `Deref` means that such a type does not,
    actually, behave as a reference, and while proxies can be crafted for specific types (`std::vector<bool>`
    demonstrates it) it's impossible to craft transparent proxies in the generic case.

The mistake made in the C++ allocator API is to require returning pointer-like/reference-like objects directly usable
by the user of the collection based upon the allocator.

This RFC learns from C++ mistake by introducing a level of indirection:

1.  An opaque `Handle` is returned by the `Store`, which can be stored and copied freely, but cannot be dereferenced.
    It is intended to be kept as an implementation detail within the collection, and invisible to the final user.
2.  A `resolve` method to resolve a `Handle` into a pointer, and from there into a reference.

Throwing in flexible invalidation guarantees ties the knot, allowing this API to be much more flexible than the C++
allocator API.


##  Previous attempts

I have been seeking a better allocator API for years, now. This RFC draws from this experience:

-   I implemented an API with a similar goal _specifically_ for vector-like collections in C++. It was much less
    flexible, and tailored for C++ requirements, but did prove that a somewhat minimalist API _was_ sufficient to build
    a collection that could then be declined in Inline, Small, and "regular" versions.
-   Early 2021, I demonstrated the potential for stores in https://github.com/matthieu-m/storage-poc. It was based
    on my C++ experience, from which it inherited strong typing, which itself required GATs...
-   Early 2022, @CAD97 demonstrated that a much leaner API could be made in https://github.com/CAD97/storages-api.
    After reviewing their work, I concluded that the API was not suitable to replace `Allocator` in a number of
    situations as discussed in the Alternatives section, and that further adjustments needed to be made.

And thus in early 2023 I began work on a 3rd revision of the API, a revision I am increasingly confident in for 2
reasons:

1.  It is nearly a direct translation of the `Allocator` API, which has been used within the Rust standard library for
    years now.
2.  A core trait providing functionality and a set of marker traits providing guarantees is much easier to deal with
    than multiple traits each providing related but subtly different functionalities and guarantees.

The ability to develop 3rd-party extensions for increased safety also confirms, to me, that @CAD97 was on the right
track when they removed the strong typing, and on the wrong track when they attempted to bake in borrow-checking: if
it's easy enough to add safety, then it seems better for the "core" API to be minimalist instead.


#   Unresolved Questions

##  (Major) How to make `StoreBox<T, S>` coercible?

Unfortunately, at the moment, `Box` is only coercible because it stores a `NonNull<T>`, which is coercible. Splitting
`NonNull<T>` into `S::Handle` and `<T as Pointee>::Metadata`, as `StoreBox<T, S>` does, leads to losing coercibility.

A separate solution is required to regain coercibility, which is out of scope of this RFC, but would have to be solved
if `StoreBox<T, S>` were to become `Box`, which seems preferable to keeping it separate.

A suggestion would be to have a `TypedMetadata<T>` lang item, which would implement `CoerceUnsized` appropriately, and
[the companion repository showcases](https://github.com/matthieu-m/storage/blob/main/src/extension/typed_metadata.rs)
how building upon this `StoreBox` gains the ability to be `CoerceUnsized`. This is a topic for another RFC, however.


##  (Major) How to make `StoreBox<T, S>` `noalias`?

At the moment, `Box` is `noalias`, which is fairly crucial for optimizations.

Unfortunately, it is not clear (to me) how this is achieved, with `Box` being a lang-item itself _and_ being built atop
`Unique` which is also a lang-item.

It is possible to use ~~dark magic~~ specialization to have `UniqueHandle<T, H>` own a `Unique<T>` if necessary, as
@CAD97 [worked out](https://github.com/rust-lang/rfcs/pull/3446#discussion_r1242780520), though it is unclear whether
that is sufficient... or desirable.

While this RFC does _not_ propose to re-implement `Box` in terms of `StoreSingle` immediately, it is a long-term goal,
and thus it is crucial to ensure that the `StoreSingle` API allows achieving it.


##  (Medium) To `Clone`, to `Default`?

The _Safety_ section of the [`Allocator`](https://doc.rust-lang.org/nightly/std/alloc/trait.Allocator.html#safety)
documentation notes that a `Clone` of an `Allocator` must be interchangeable with the original, and that all allocated
pointers must remain until the last of the clones or copies of the allocator is dropped.

The standard library then proceed to require `A: Allocator + Clone` to clone a `Box` or a `Vec`, when arguably it is not
necessary to have an interchangeable allocator, and instead _semantically_ an independent allocator is required.

This RFC, instead, favors using the `Default` bound for the `Clone` implementation of `StoreBox`:

1.  It matches the desired semantics better -- a brand new store is required, not an interchangeable one.
2.  A cloneable `InlineStore` cannot match the semantics of `Clone` required of Allocators.

`Default` does have the issue that it may not mesh well with `dyn Store` or `dyn Allocator`, and while `Clone` can
reasonably be implemented for `&dyn Store`, or `Rc<dyn Store>`, such is not the case for `Default`.

This leaves 4 possibilities:

-   Use `Clone` despite the poor semantics match.
-   Use `Default` despite it being at odds with `dyn Store` use.
-   Add a new `SpawningStore` trait to create an independent instance, though mixing several non-empty traits in a `dyn`
    context is not supported yet.
-   Add a method to `Store` to create an independent instance, fixing the semantics of `Clone`. Possibly a faillible
    one.

Note that technically switching from the `Clone` bound to another bound for `Box` and `Vec` is a breaking change,
however since `Allocator` is an unstable API it is still early enough to effect such change.


##  (Minor) To `Clone` or to share?

As mentioned above, whenever an `Allocator` also implements the `Clone` trait, the clone or copy of the `Allocator` must
fulfill specific requirements. In particular, all clones or copies of a given allocator behave as a single allocator
sharing the backing memory and metadata. While the `Clone` trait does fit _creating_ a new clone/copy of an allocator,
it is insufficient however to _query_ whether another instance is a clone/copy of a given allocator.

The standard library runs headlong into this insufficience, and while `LinkedList::split_off` is implemented for any
`Allocator` which also implements `Clone`, `LinkedList::append` is only implemented for `Global`.

There are at least 2 possibilities, here:

-   Add a requirement on `PartialEq` implementation for `Allocator` and `Store` that comparing equal means that they are
    clones or copies of each others.
-   Add a separate `StoreSharing` trait -- see future possibilities.

It should be noted that `dyn` usage of `Allocator` and `Store` suffers from the requirement of using unrelated traits as
it is not possible to have a `dyn Allocator + Clone + PartialEq` trait today, though those traits can be implemented for
`&dyn Allocator` or `Rc<dyn Allocator>`.

Given that the problem is unsolved for `Allocator`, it can be punted on in the context of this RFC.


##  (Minor) What should the capabilities of `Handle` be?

Since any capability specified in the associated type definition is "mandatory", I am of the opinion that it should be
kept to a minimum, and users can always specify additional requirements if they need to:

-   At the moment, the only required is `Copy`. It could be downgraded to `Clone`, or removed entirely.
    -   Although, do mind that just using `Store::grow`, or `Store::shrink` requires a copy/clone.
-   `Eq`, `Hash`, and `Ord` are obvious candidates, yet they are unused in the current proposal:
    -   Implementing `Eq`, `Hash`, or `Ord` for a collection does not require the handles to implement any of them.
-   `Send` and `Sync` should definitely be kept out. `Allocator`-based stores could not use `NonNull<u8>` otherwise.


##  (Minor) Would `Store::dangling` be better than `StoreDangling`?

While const trait associated functions are still a maybe, it seems reasonable to ask ourselves whether some of the
associated functions of `Store` or `StoreSingle` should be `const` if it were possible.

There doesn't seem to be a practical advantage in doing so for most of the associated functions of `Store`: if
allocation and deallocation need be executed in a const context, then a `const Store` is necessary, and there's no need
to single out any of those.

There is, however, a very practical advantage in making `Store::dangling` const: it allows initializing an empty
collection in a const context even with a non-const `Store` implementation.

The one downside is that this would preclude some implementations of `dangling` which would rely on global state, or
I/O. @CAD97 notably mentioned the possibility of using randomization for debugging or hardening purposes. Still, it
would still be possible to initialize the instance of `Store` with a random seed, then use a PRNG within `dangling`.

On the other, it leads to a simpler API than a separate `StoreDangling` base trait.


#   Future Possibilities

##  StoreSharing

One (other) underdevelopped aspect of the `Allocator` API at the moment is the handling of fungibility of pointers, that
is the description -- in trait -- of whether a pointer allocated by one `Allocator` can be grown, shrunk, or deallocated
by another instance of `Allocator`. The immediate consequence is that `Rc` is only `Clone` for `Global`, and the
`LinkedList::append` method is similarly only available for `Global` allocator.

A possible future extension for the Storage proposal is the introduction of the `StoreSharing` trait:

```rust
trait StoreSharing: StorePinning {
    type SharingError;

    fn is_sharing_with(&self, other: &Self) -> bool;

    fn share(&self) -> Result<Self, Self::SharingError> where Self: Sized;
}
```

This trait introduces the concept of set of sharing stores, that is when multiple stores share the same "backing" memory
and allocation metadata.

The `share` method creates a new instance of the store which shares the same "backing" memory and metadata as `self`,
while the `is_sharing_with` method allows querying whether two stores share the same "backing" memory and metadata.

A set of sharing stores can be thought of as a single store instance: handles created by one of the stores can be used
with any of the stores of the set, in any way, and as long as one store of the set has not been dropped, dropping a
store of the set does not invalidate the handles. Informally, the "backing" memory and metadata can be thought of as
being reference-counted.

The requirement of `StorePinning` is necessary as moving any one instance should not invalidate the pointers resolved by
other instances of the set, and the `SharingError` type allows modelling potentially-sharing stores, such as a small
store which cannot be shared if its handles currently point to inline memory.


##  TypedHandle

A straightforward extension is to define a `TypedHandle<T, H>` type, which wraps a handle (`H`) to a block of memory
suitable for an instance of `T`, and also wraps the metadata of `T`.

The `Store` methods can then be mimicked, with the additional type information:

-   `resolve` returns an appropriate pointer type, complete with metadata.
-   Layouts are computed automatically based on the type and metadata.
-   Growing and Shrinking on slices take the target number of elements, rather than a more complex layout.

And because `resolve` and `resolve_mut` can return references -- being typed -- they can borrow the `store` (shared) to
ensure it's not moved nor dropped.


##  UniqueHandle

A further extension is to define a `UniqueHandle<T, H>` type, which adds ownership & borrow-checking over a
`TypedHandle<T, H>`.

That is:

```rust
impl<T: ?Sized, H: Copy> UniqueHandle<T, H> {
    pub unsafe fn resolve<'a, S>(&'a self, store: &'a S) -> &'a T
    where
        S: Store;

    pub unsafe fn resolve_mut<'a, S>(&'a mut self, store: &'a S) -> &'a mut T
    where
        S: Store;
}
```

Those two methods are `unsafe` as a dangling handle cannot be soundly resolved, and a valid handle may not necessarily
be associated with a block of memory containing a valid instance of `T` -- it may never have been constructed, it may
have been moved or dropped, etc...

On the other hand, those two methods guarantee:

-   Proper typing: if the handle is valid, and a value exists, then that value is of type `T`.
-   Proper borrow-checking: the handle is the unique entry point to the instance of `T`, hence the name.
-   Proper pinning: even if the store does not implement `StorePinning`, borrowing it ensures that it cannot be moved
    nor dropped. If the store implements `StoreStable`, this means that the result of `resolve` and `resolve_mut` can be
    used without fear of invalidation.

And that's pretty good, given how straightforward the code is.


##  Compact Vectors

How far should DRY go?

One limitation of `Vec<u8, InlineStore<[u8; 16]>>` is that it contains 2 `usize` for the length and capacity
respectively, which is rather unfortunate.

There are potential solutions to the issue, using separate traits for those values so they can be stored in more compact
ways or even elided in the case of fixed capacity.

The `Store` API could be augmented with a new marker trait with associated constants / types describing the limits of
what it can offer such as minimum/maximum capacity, to support automatically selecting (or constraint-checking) the
appropriate types.

Since those extra capabilities can be brought in by user traits for now, I would favor adopting a wait-and-see approach
here.
