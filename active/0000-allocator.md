- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Add a standard allocator interface and support for user-defined
allocators, with the following goals:

 1. Allow libraries to be generic with respect to the allocator, so
    that users can supply their own memory allocator and still make
    use of library types like `Vec` or `HashMap`.  In particular,
    stateful per-container allocators are supported.

 2. Support ability of garbage-collector (GC) to identify roots stored
    in statically-typed user-allocated objects outside the GC-heap,
    without sacrificing efficiency for code that does not use `Gc<T>`.

 3. Do not require an allocator itself track the size of allocations.
    Instead, force the client to supply size at the deallocation site.

 4. Incorporate data alignment constraints into the API, as many
    allocators have efficient support for meeting such constraints
    built-in, rather than forcing the client to build their own
    re-aligning wrapper around a `malloc`-style interface.

 5. (Niko) Permit dynamically-sized types to be cloned in a generic way.

This RFC does not attempt to specify a so-called "pluggable" GC
library.  We assume here that any GC support is built-in to the Rust
compiler and standard library; we leave work on pluggable GC as future
research.

# Motivation

As noted in [RFC PR 39], modern general purpose allocators are good,
but due to the design tradeoffs they must make, cannot be optimal in
all contexts.  Therefore, the standard library should allow clients to
plug in their own allocator for managing memory.

Also as noted in [RFC PR 39], the basic `malloc` interface
{`malloc(size) -> ptr`, `free(ptr)`, realloc(ptr, size) -> ptr`} is
lacking in a number of ways: `malloc` lacks the ability to request a
particular alignment, and `realloc` lacks the ability to express a
copy-free "reuse the input, or do nothing at all" request.  Another
problem with the `malloc` interface is that it burdens the allocator
with tracking the sizes of allocated data and re-extracting the
allocated size from the `ptr` in `free` and `realloc` calls.

To accomplish the above, this RFC proposes a `RawAlloc` interface for
managing blocks of memory, with specified size and alignment
constraints.

Meanwhile, we would like to continue supporting a garbage collector
(GC) even in the presence of user-defined allocators.  In particular,
we wish for GC-managed pointers to be embeddable into user-allocated
data outside the GC-heap and program stack, and still be scannable by
a tracing GC implementation.  In other words, we want this to work:

```rust
let x: Rc<int> = Rc::new(3);
let y: Gc<Rc<int>> = Gc::new(x);
let z: Rc<Gc<Rc<int>>> = Rc::new(y);
```

But we do not want to impose the burden of supporting a tracing
garbage collector on all users: if a type does not contain any
GC-managed pointers, then the code path for allocating an instance of
that type should not bear any overhead related to GC.

To provide garbage-collection support without imposing overhead on
clients who do not need GC, this RFC proposes a high-level type-aware
allocator API, here called the `high_alloc` module, parameterized over
its underlying `RawAlloc`.  Libraries are meant to use the
`high_alloc` API, which will maintain garbage collection meta-data
when necessary (but only when allocating types that involve `Gc`).
The code-paths for the `high_alloc` procedures are optimized with
fast-paths for when the allocated type does not contain `Gc<T>`.

The user-specified instance of `RawAlloc` is not expected to attempt
to provide GC-support itself.  The user-specified allocator is only
meant to satisfy a simple, low-level interface for allocating and
freeing memory.  The support for garbage-collection is handled at a
higher level, within the Rust standard library itself.

# Detailed design

Here is the `RawAlloc` trait design.  It is largely the same
as the design from [RFC PR 39]; points of departure are
enumerated after the API listing.

```rust
/// Low-level explicit memory management support.
pub trait RawAlloc {
    /// Returns a pointer to `size` bytes of memory, aligned to
    /// a `align`-byte boundary.
    ///
    /// Returns null if allocation fails.
    ///
    /// Behavior undefined if `size` is 0 or `align` is not a
    /// power of 2, or if the `align` is larger than the largest
    /// platform-supported page size.
    unsafe fn alloc(&self, size: uint, align: uint) -> *mut u8;

    /// Extends or shrinks the allocation referenced by `ptr` to
    /// `size` bytes of memory, retaining the alignment `align`.
    ///
    /// If this returns non-null, then the storage referenced by `ptr`
    /// may have been freed and should be considered unusable.
    ///
    /// Returns null if allocation fails; in this scenario, the original
    /// memory is unaltered.
    ///
    /// The `old_size` and `align` parameters must be the parameters
    /// last used to create `ptr` (either via `alloc` or `realloc`);
    /// otherwise behavior is undefined.  Behavior also undefined if
    /// `size` is 0.
    unsafe fn realloc(&self, ptr: *mut u8, size: uint, align: uint, old_size: uint) -> *mut u8;

    /// Returns the usable size of an allocation created with the
    /// specified `size` and `align`.
    #[inline(always)]
    unsafe fn usable_size(&self, size: uint, align: uint) -> uint {
        size
    }

    /// Deallocate the memory referenced by `ptr`.
    ///
    /// The `old_size` and `align` parameters must be the parameters
    /// last used to create `ptr` (either via `alloc` or `realloc`);
    /// otherwise behavior is undefined.  Behavior also undefined if
    /// `ptr` is null.
    unsafe fn dealloc(&self, ptr: *mut u8, size: uint, align: uint);
}
```

Here is the `high_alloc` API design.  Much of it is similar to the
`RawAlloc` trait, but there are a few extra pieces added for type-safe
allocation, dyanmically-sized types, and GC support.

```rust
mod high_alloc {
    pub struct Alloc<Raw:RawAlloc=DefaultRawAlloc> {
        raw: Raw
    }

    pub struct MemoryBlockInfo<Sized? T> {
        // compiler and runtime internal fields

        // (perhaps these)
        size: uint,
        align: uint,
    }

    impl MemoryBlockInfo<Sized? T> {
        pub fn from_type() -> MemoryBlockInfo<T>() where T : Sized {
            MemoryBlockInfo::<T>::from_size_and_align(
                mem::size_of::<T(),
                mem::align_of::<T>())
        }

        pub fn array(capacity: uint) -> MemoryBlockInfo<T> where T : Sized {
            MemoryBlockInfo<T>::from_size_and_align(
                capacity * mem::size_of::<T>(),
                mem::align_of::<T>())
        }

        /// `size` is the minimum size (in bytes) for the allocated
        /// block; `align` is the minimum alignment.
        ///
        /// If either `size < mem::size_of::<T>()` or
        /// `align < mem::min_align_of::<T>()` then behavior undefined.
        pub fn from_size_and_align(size: uint, align: uint) -> MemoryBlockInfo<T> {
            ...
        }
    }

    impl<Raw> Alloc<Raw> {
        /// Allocates a memory block suitable for holding `T`.
        /// Returns the pointer to the block if allocation succeeds.
        ///
        /// Returns null if allocation fails.
        unsafe pub fn alloc<T>(&self) -> *mut T {
            self.alloc_info(MemoryBlockInfo::<T>::from_type())
        }

        /// Frees memory block at `pointer`.
        ///
        /// `pointer` must have been previously allocated via `alloc`.
        unsafe pub fn dealloc<T>(&self, pointer: *mut T) {
            self.dealloc_info(MemoryBlockInfo::<T>::from_type())
        }

        /// Allocates a memory block suitable for holding `capacity`
        /// instances of `T`.
        ///
        /// Returns the pointer to the block if allocation succeeds.
        ///
        /// Returns null if allocation fails.
        unsafe pub fn alloc_array<T>(&self, capacity: uint) -> *mut T {
            self.alloc_info(MemoryBlockInfo::<T>::array(capacity))
        }

        /// Given a pointer to the start of a memory block allocated
        /// for holding instance(s) of `T`, returns the number of
        /// contiguous instances of `T` that `T` can hold.
        unsafe pub fn usable_capacity<T>(&self, capacity: uint) -> uint {
            let info = MemoryBlockInfo::<T>::array(capacity);
            self.raw.usable_size(info.size, info.align);
        }

        /// Given a memory block and its prior capacity, allocates a
        /// memory block suitable for holding `new_capacity` instances
        /// of `T`, reusing the given block if possible.
        ///
        /// Returns the pointer to the new (potentially reused) block
        /// if allocation succeeds.
        ///
        /// Returns null, with old_ptr_and_capacity unchanged, if
        /// allocation fails.
        unsafe pub fn realloc_array<T>(&self,
                                       old_ptr_and_capacity: (*mut T, uint),
                                       new_capacity: uint) -> *mut T {
            let (op, oc) = old_ptr_and_capacity;
            self.realloc_info(op, MemoryBlockInfo::<T>::array(new_capacity))
        }

        /// Frees memory block referenced by `ptr_and_capacity`.
        ///
        /// `pointer` must have been previously allocated via
        /// `alloc_array` or `realloc_array`, with the same argument
        /// capacity that is supplied here.
        unsafe pub fn dealloc_array<T>(&self, ptr_and_capacity: (*mut T, uint)) {
            let (op, oc) = ptr_and_capacity;
            self.dealloc_info(op, MemoryBlockInfo::<T>::array(oc))
        }

        /// Deinitializes the range of instances `[start, start+count)`.
        ///
        /// A container must call this function when (1.) it has
        /// previously initialized the instances of `T` in the
        /// aforementioned range, and (2.) it has since moved or
        /// dropped the instances of `T` out of the array.
        ///
        /// It prevents dangling references to GC-able objects.  The
        /// alternative would be to require all `Gc<T>` to have a
        /// destructor, which is counter to the goals of tracing
        /// garbage collection.
        ///
        /// XXX this probably does not belong in the API doc, unless
        /// we want to clarify motivation and/or the sorts of bugs one
        /// can encounter when one fails to uphold this obligation.
        ///
        /// XXX Niko: Am I right that failing to call this should solely
        /// result in storage leaks, *not* in other unsoundness?
        #[inline(always)]
        unsafe pub fn deinit_range<T>(&self, start: *mut T, count: uint) {
            if ! type_reaches_gc::<T>() {
                /* no-op */
                return;
            } else {
                self.deinit_range_gc(start, count)
            }
        }

        unsafe fn deinit_range_gc<T>(&self, start: *mut T, count: uint) {
            for i in range(0, count) {
                *start.offset(i) = ptr::null();
            }
        }

        /// Allocates a memory block suitable for holding `T`,
        /// with minimum size and alignment specified by `info`.
        unsafe pub fn alloc_info<Sized? T>(&self, info: BlockInfo<T>) -> *mut T {
            self.raw.alloc(info.size(), info.align())
        }

        /// The `info` must have size and align compatible with the
        /// `info` that was used to create `old_ptr`.
        ///
        /// This method is very dangerous.
        ///
        /// The only time this is safe to use on GC-referencing data
        /// is when converting from `[T]` of one length to `[T]` of a
        /// different length.  In particular, this method is not safe
        /// to use to convert between different types if either type
        /// references GC data.
        unsafe fn realloc_info<Sized? T, Sized? U>(&self, old_ptr: *mut T, info: BlockInfo<U>) -> *mut U {
            self.raw.realloc(pointer, info.size(), info.align())
        }

        /// The `info` must have size and align compatible with the
        /// `info` that was used to create `pointer`.
        unsafe pub fn dealloc_info<Sized? T>(&self, pointer: *mut T, info: BlockInfo<T>) {
            self.raw.dealloc(pointer, info.size(), info.align())
        }
    }
}
```

This is the bulk of the RFC. Explain the design in enough detail for somebody familiar
with the language to understand, and for somebody familiar with the compiler to implement.
This should get into specifics and corner-cases, and include examples of how the feature is used.

# Drawbacks

Why should we *not* do this?

# Alternatives

What other designs have been considered? What is the impact of not doing this?

# Unresolved questions

What parts of the design are still TBD?

[RFC PR 39]: https://github.com/rust-lang/rfcs/pull/39/files
