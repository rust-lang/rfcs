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

 3. Do not require an allocator itself to track the size of allocations.
    Instead, force the client to supply size at the deallocation site.
    (This can improve overall performance on certain hot paths.)

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

## Why Custom Allocators

As noted in [RFC PR 39], modern general purpose allocators are good,
but due to the design tradeoffs they must make, cannot be optimal in
all contexts.  Therefore, the standard library should allow clients to
plug in their own allocator for managing memory.

TODO: enumerate typical use cases for Allocators from C++.  Some
immediate thoughts:

  1. memory grouping, at very least in same size-class (but also
     potentially for different size classes, via a bump-pointer allocator
     with I guess a slow/no-op free?)

  2. hard memory limit (as I think EASTL offers and perhaps others),

  3. shared-memory across processes (suggested by nical, not sure if we
     can put in the static checks for vtables that he wants)

  4. memory padding to reduce/eliminate false sharing of cache lines (I
     think strcat had this in his RFC)

  5. memory usage instrumentation and debugging.

## Why this API

Also as noted in [RFC PR 39], the basic `malloc` interface
{`malloc(size) -> ptr`, `free(ptr)`, `realloc(ptr, size) -> ptr`} is
lacking in a number of ways: `malloc` lacks the ability to request a
particular alignment, and `realloc` lacks the ability to express a
copy-free "reuse the input, or do nothing at all" request.  Another
problem with the `malloc` interface is that it burdens the allocator
with tracking the sizes of allocated data and re-extracting the
allocated size from the `ptr` in `free` and `realloc` calls.

To accomplish the above, this RFC proposes a `RawAlloc` interface for
managing blocks of memory, with specified size and alignment
constraints.  The `RawAlloc` client can attempt to adjust the storage
in use in a copy-free manner by observing the memory block's
current capacity via a `usable_size` call.

Meanwhile, we would like to continue supporting a garbage collector
(GC) even in the presence of user-defined allocators.  In particular,
we wish for GC-managed pointers to be embeddable into user-allocated
data outside the GC-heap and program stack, and still be scannable by
a tracing GC implementation.  In other words, we want this to work:

```rust
let alloc: MyAlloc = ...;

// Instances of `MyVec` have their backing array managed by `MyAlloc`.
type MyVec<T> = Vec<T, MyAlloc>;
let mut v: MyVec<int> = vec![3, 4];

// Here, we move `x` into a gc-managed pointer; the vec's backing
// array is still managed by `alloc`.
let w: Gc<MyVec<int> = Gc::new(x);

// similar
let x: Gc<MyVec<int> = Gc::new(vec![5, 6]);

// And here we have a vec whose backing array, which contains GC roots,
// is held in memory managed by `MyAlloc`.
let y: MyVec<Gc<MyVec<int>>> = vec![w, x];
```

Or, as a potentially simpler example, this should also work:
```rust
let a: Rc<int> = Rc::new(3);
let b: Gc<Rc<int>> = Gc::new(x);
let c: Rc<Gc<Rc<int>>> = Rc::new(y);
```

But we do not want to impose the burden of supporting a tracing
garbage collector on all users: if a type does not contain any
GC-managed pointers (and is not itself GC-managed),
then the code path for allocating an instance of
that type should not bear any overhead related to GC.

To provide garbage-collection support without imposing overhead on
clients who do not need GC, this RFC proposes a high-level type-aware
allocator API, here called the `high_alloc` module, parameterized over
its underlying `RawAlloc`.  Libraries are meant to use the
`high_alloc` API, which will maintain garbage collection meta-data
when necessary (but only when allocating types that involve `Gc`).
The code-paths for the `high_alloc` procedures are optimized with
fast-paths for when the allocated type does not contain `Gc<T>`.

The user-specified instance of `RawAlloc` is not required to attempt
to provide GC-support itself.  The user-specified allocator is only
meant to satisfy a simple, low-level interface for allocating and
freeing memory.  The support for garbage-collection is handled at a
higher level, within the Rust standard library itself.

# Detailed design

## The RawAlloc trait

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
    unsafe fn alloc_bytes(&self, size: uint, align: uint) -> *mut u8;

    /// Extends or shrinks the allocation referenced by `ptr` to
    /// `size` bytes of memory, retaining the alignment `align`.
    ///
    /// If this returns non-null, then the memory block referenced by
    /// `ptr` may have been freed and should be considered unusable.
    ///
    /// Returns null if allocation fails; in this scenario, the
    /// original memory block referenced by `ptr` is unaltered.
    ///
    /// The `align` parameter must have the value used to create
    /// `ptr` (via `alloc_bytes` or `realloc_bytes`).
    /// The `old_size` parameter must fall in the range `[orig,
    /// usable]`, where:
    ///   * `orig` is the value last used to create `ptr`
    ///     (either via `alloc_bytes` or `realloc_bytes`), and
    ///   * `usable` is the value returned from `usable_size` when
    ///     given `ptr`, `orig`, and `align`.
    ///
    /// Behavior is undefined if above constraints on `align` and
    /// `old_size` are unmet. Behavior also undefined if `size` is 0.
    unsafe fn realloc_bytes(&self, ptr: *mut u8, size: uint, align: uint, old_size: uint) -> *mut u8;

    /// Returns the usable size of an allocation created with the
    /// specified `size` and `align`.
    ///
    /// The `align` parameter must have the value used to create
    /// `ptr` (via `alloc_bytes` or `realloc_bytes`).
    /// The `size` parameter must fall in the range
    /// `[orig, usable]`, where:
    ///
    ///   * `orig` is the value last used to create `ptr`
    ///     (either via `alloc_bytes` or `realloc_bytes`), and
    ///   * `usable` is the value returned from `usable_size` when
    ///     given `ptr`, `orig`, and `align`.
    ///
    #[inline(always)]
    unsafe fn usable_size_bytes(&self, ptr: *mut u8, size: uint, align: uint) -> uint {
        size
    }

    /// Deallocate the memory referenced by `ptr`.
    ///
    /// Returns null if allocation fails; in this scenario, the
    /// original memory block referenced by `ptr` is unaltered.
    ///
    /// The `align` parameter must have the value used to create
    /// `ptr` (via `alloc_bytes` or `realloc_bytes`).
    /// The `old_size` parameter must fall in the range `[orig,
    /// usable]`, where:
    ///   * `orig` is the value last used to create `ptr`
    ///     (either via `alloc_bytes` or `realloc_bytes`), and
    ///   * `usable` is the value returned from `usable_size` when
    ///     given `ptr`, `orig`, and `align`.
    ///
    /// Behavior is undefined if above constraints on `align` and
    /// `old_size` are unmet. Behavior also undefined if `ptr` is
    /// null.
    unsafe fn dealloc_bytes(&self, ptr: *mut u8, size: uint, align: uint);
}
```

Points of departure from [RFC PR 39]:

  * Changed names to include the suffix `_bytes`, to differentiate
    these methods from the high-level allocation API below.

  * Extended interface of `realloc_bytes` and `dealloc_bytes` to allow
    `old_size` to take on a range of values between the originally
    given allocation size and the usable size for the pointer.

    The intention is to allow the client to locally adjust its own
    interpretation of how much of the memory is in use, without
    forcing it to round-trip through the `realloc` interface each
    time, and without forcing it to record the original parameter fed
    to `alloc_bytes` or `realloc_bytes` that produced the pointer.


  * Extended `usable_size_bytes` to take the `ptr` itself as an
    argument, to handle hypothetical allocators who may choose
    different sized bins given the same `(size, align)` input.

    (I expect in practice that most allocators that override the
    default implementation will actually return a constant-expression
    computed solely from the given `size` and `align`, but I do not
    yet see a compelling reason to drop `ptr` from the argument list.)

## The high_alloc mod

The `RawAlloc` trait defines the low-level API we expect users to be
able to provide easily, either by dispatching to other native
allocator libraries (such as [jemalloc], [tcmalloc], [Hoard], et
cetera), or by implementing their own in Rust (!).

But the low-level API is not sufficient.  It is lacking in two ways:

* It does not integrate dynamically sized types (DST), in the sense
  that it still leaves the problem of shifting from a thin-pointer
  (`*mut u8`) to a potentially fat-pointer (e.g. `*mut [T]` for some
  `T:Sized`).

* It does not integrate garbage collection (GC).  Rust has been
  designed to allow an optional [tracing garbage collector] to manage
  graph-structured memory, but if all collections relied solely on the
  `RawAlloc` interface without also registering the potential GC roots
  held in the natively allocated blocks, we could not hope to put in a
  tracing GC.

Therefore, this RFC defines a high-level interface that is intended
for direct use by libraries.  The high-level interface defines a small
group of `Alloc` traits that correspond to how one allocates instances
of a sized type or arrays.

Crucially, the intention in this design is that every implementation
of the `Alloc` trait be parameterized over one or more `RawAlloc`
instances (usually just one) that dictate the actual low-level memory
allocation.

When splitting between a high-level `Alloc` and a low-level `RawAlloc`,
there are questions that arise regarding how the high-level operations
of `Alloc` actually map to the low-level methods provided by `RawAlloc`.
Here are a few properties of potential interest when thinking about
this mapping:

* A "header-free" high-level allocation is one where the high-level
  allocator implementation adds no headers to the block associated
  with the storage for one value; more specfically, the size of the
  memory block allocated to represent a type `T` is (at most) the size
  of what the underlying `RawAlloc` would return for a request a block
  of size `mem::size_of::<T>()` and alignment `mem::align_of::<T>()`.
  (We say "at most" because the `Alloc` implementation may choose to
  use `mem::min_align_of::<T>()`; that detail does not matter in terms
  of the spirit of what "header-free allocation" means.

* A "1:1 call correspondence" between a high-level allocator

Here is the `high_alloc` API design.  Much of it is similar to the
`RawAlloc` trait, but there are a few extra pieces added for type-safe
allocation, dyanmically-sized types, and GC support.

```rust
mod high_alloc {
    trait Alloc {
        /// Allocates a memory block suitable for holding `T`.
        /// Returns the pointer to the block if allocation succeeds.
        ///
        /// Returns null if allocation fails.
        unsafe fn alloc<T>(&self) -> *mut T;

        /// Frees memory block at `pointer`.
        ///
        /// `pointer` must have been previously allocated via `alloc`.
        unsafe fn dealloc<T>(&self, pointer: *mut T);
    }

    trait ArrayAlloc {
        /// Allocates a memory block suitable for holding `capacity`
        /// instances of `T`.
        ///
        /// Returns the pointer to the block if allocation succeeds.
        ///
        /// Returns null if allocation fails.
        unsafe fn alloc_array<T>(&self, capacity: uint) -> *mut T;

        /// Given a pointer to the start of a memory block allocated
        /// for holding instance(s) of `T`, returns the number of
        /// contiguous instances of `T` that `T` can hold.
        unsafe fn usable_capacity<T>(&self, capacity: uint) -> uint;

        unsafe fn realloc_array<T>(&self,
                                   old_ptr_and_capacity: (*mut T, uint),
                                   new_capacity: uint) -> *mut T;

        /// Frees memory block referenced by `ptr_and_capacity`.
        ///
        /// `pointer` must have been previously allocated via
        /// `alloc_array` or `realloc_array`, with the same argument
        /// capacity that is supplied here.
        unsafe fn dealloc_array<T>(&self, ptr_and_capacity: (*mut T, uint));

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
        unsafe fn deinit_range<T>(&self, start: *mut T, count: uint);
    }

    /// A `MemoryBlockInfo` (or more simply, "block info") represents
    /// information about what kind of memory block must be allocated
    /// to hold data of (potentially unsized) type `U`, and also what
    /// extra-data, if any, must be attached to a pointer to such a
    /// memory block to create `*mut U` or `*U` pointer.
    ///
    /// This opaque struct is used as the abstraction for
    /// communicating with the high-level type-aware allocator.
    ///
    /// It is also used as the abstraction for reasoning about the
    /// validity of calls to "realloc" in the high-level allocator; in
    /// particular, given a memory block allocated for some type `U`,
    /// you can only attempt to reuse that memory block for another
    /// type `T` if the `MemoryBlockInfo<U>` is *compatible with* the
    /// `MemoryBlockInfo<T>`.
    ///
    /// Definition of "info_1 compatible with info_2"
    ///
    /// For non GC-root carrying data, "info_1 is compatible with
    /// info_2" means that the two block infos have the same
    /// alignment.  (They need not have the same size, as that would
    /// defeat the point of the `realloc` interface.)  For GC-root
    /// carrying data, according to this RFC, "compatible with" means
    /// that they are either two instances of the same array type [T]
    /// with potentially differing lengths, or, if they are not both
    /// array types, then "compatible with" means that they are the
    /// same type `T`.
    ///
    /// (In the future, we may widen the "compatible with" relation to
    /// allow distinct types containing GC-roots to be compatible if
    /// they meet other yet-to-be-defined constraints.  But for now
    /// the above conservative definition should serve our needs.)
    ///
    /// Definiton of "info_1 extension of info_2"
    ///
    /// For non GC-root carrying data, "info_1 is an extension of
    /// info_2" means that info_1 is *compatible with* info_2, *and*
    /// also the size of info_1 is greater than or equal to info_2.
    /// The notion of a block info extending another is meant to
    /// denote when a memory block has been locally reinterpreted to
    /// use more of its available capacity (but without going through
    /// a round-trip via `realloc`).
    pub struct MemoryBlockInfo<Sized? U> {
        // compiler and runtime internal fields

        // naive impl (notably, a block info probably carries these
        // fields; but in practice it may have other fields describing
        // the format of the data within the block, to support tracing
        // GC).

        // The requested size of the memory block
        size: uint,

        // The requested alignment of the memory block
        align: uint,

        // The extra word of data to attach to a fat pointer for unsized `T`
        unsized_metadata: uint,

        // Explicit marker indicates that a block info is neither co-
        // nor contra-variant with respect to its type parameter `U`.
        marker: InvariantType<U>
    }

    impl<Sized? U> MemoryBlockInfo<T> {
        /// Returns minimum size of memory block for holding a `T`.
        pub fn size(&self) -> uint { self.size }

        /// Produces a normalized block info that can be compared
        /// against other similarly normalized block infos.
        ///
        /// If two distinct types `T` and `U` are indistinguishable
        /// from the point of view of the garbage collector, then they
        /// will have equivalent normalized block infos.  Thus,
        /// allocations may be categorized into bins by the high-level
        /// allocator according to their normalized block infos.
        pub fn forget_type(&self) -> MemoryBlockInfo<()> {
            // naive impl

            // (I assume even a naive implementation still needs to
            // construct a fresh `marker` to placate the type system)
            MemoryBlockInfo { marker: InvariantType, ..*self }
        }

        /// Returns a block info for a sized type `T`.
        pub fn from_type() -> MemoryBlockInfo<T>() where T : Sized {
            // naive impl
            MemoryBlockInfo::<T>::new(
                mem::size_of::<T(),
                mem::align_of::<T>(),
                0u)
        }

        /// Returns a block info for an array of (unsized) type `[U]`
        /// capable of holding at least `length` instances of `U`.
        pub fn array<U>(length: uint) -> MemoryBlockInfo<[U]> {
            // naive impl
            MemoryBlockInfo<[U]>::from_size_and_align(
                length * mem::size_of::<T>(),
                mem::align_of::<T>(),
                length)
        }

        /// `size` is the minimum size (in bytes) for the allocated
        /// block; `align` is the minimum alignment.
        ///
        /// If either `size < mem::size_of::<T>()` or
        /// `align < mem::min_align_of::<T>()` then behavior undefined.
        fn new(size: uint, align: uint, extra: uint) -> MemoryBlockInfo<T> {
            // naive impl
            MemoryBlockInfo {
                size: size,
                align: align,
                unsized_metadata: extra,
                marker: InvariantType,
            }
        }
    }

    trait AllocCore {
        /// Allocates a memory block suitable for holding `T`,
        /// according to the  `info`.
        unsafe fn alloc_info<Sized? T>(&self, info: MemoryBlockInfo<T>) -> *mut T;

        /// Attempts to recycle the memory block for `old_ptr` to create
        /// a memory block suitable for an instance of `U`.
        ///
        /// Requirement 1: `info` must be *compatible with* `old_info`.
        ///
        /// Requirement 2: `old_info` must be an *extension of* the
        /// `mbi_orig`, the size of `old_info` must be in the range
        /// `[mbi_orig.size(),self.usable_size_info(old_ptr,mbi_orig)]`
        /// (where `mbi_orig` is the `MemoryBlockInfo` originally used
        /// to create `old_ptr`, be it via `alloc_info` or
        /// `realloc_info`).
        ///
        /// (The requirements use terms in the sense defined in the
        /// documentation for `MemoryBlockInfo`.)
        ///
        /// Here is the executive summary of what the above
        /// requirements mean: This method is dangerous.  It is
        /// especially dangerous for data that may hold GC roots.
        ///
        /// The only time this is safe to use on GC-referencing data
        /// is when converting from `[T]` of one length to `[T]` of a
        /// different length.  In particular, this method is not safe
        /// to use to convert between different non-array types `S`
        /// and `T` (or between `[S]` and `[T]`, etc) if either type
        /// references GC data.
        unsafe fn realloc_info<Sized? T, Sized? U>(&self, old_ptr: *mut T, old_info: MemoryBlockInfo<T>, info: MemoryBlockInfo<U>) -> *mut U;

        /// The `info` must be an *extension of* the `MemoryBlockInfo`
        /// originally used to create `ptr`, be it via `alloc_info` or
        /// `realloc_info`.
        unsafe fn usable_size_info<Sized? T>(&self, ptr: *mut u8, info: MemoryBlockInfo<T>) -> uint;

        /// Deallocates the memory block at `pointer`.
        ///
        /// The `info` must be an *extension of* the `MemoryBlockInfo`
        /// used to create `pointer`.
        unsafe fn dealloc_info<Sized? T>(&self, pointer: *mut T, info: MemoryBlockInfo<T>);

        /// Returns true if this allocator exposes its internals to
        /// the garbage collector for root extraction and scanning.
        ///
        /// Note: We leave it to a future RFC to define the criteria
        /// by which an implementation of `AllocCore` can return
        /// `true` for this method.  The standard library should
        /// eventually provide at least one implementation of
        /// `AllocCore` that is tightly GC-integrated, but the manner
        /// in which this will be accomplished is not a component of
        /// this RFC.
        fn tightly_gc_integrated() -> bool { false }
    }

    // What follows is a sketch of how the above extension traits are
    // implemented atop the `RawAlloc` interface, with GC hooks
    // included as needed (but optimized away when the type does not
    // involve GC).  This sketch assumes that the Rust has been
    // extended with type-level compile-time intrinsic functions to
    // indicate whether allocation of involves coordination with the
    // garbage collector.
    //
    // These compile-time intrinsic functions are:
    //
    // * `type_is_gc_allocated::<T>()`, which indicates whether `T` is
    //   itself allocated on the garbage-collected heap (e.g. `T` is
    //   some `GcBox<_>`, where `Gc<_>` holds a reference to a
    //   `GcBox<_>`), and
    //
    // * `type_reaches_gc::<T>()`, which indicates if the memory block
    //   for `T` needs to be treated as holding GC roots (e.g. `T`
    //   contains some `Gc<_>` within its immediate contents).

    // Notes on the direct `RawAlloc` based implementation.
    // * Raw allocators cannot directly allocate on the gc-heap.

    impl<Raw:RawAlloc> AllocInfo for Raw {
        #[inline(always)]
        unsafe fn alloc_info<Sized? T>(&self, info: MemoryBlockInfo<T>) -> *mut T {
            assert!(!type_is_gc_allocated::<T>());
            if ! type_reaches_gc::<T>() {
                self.alloc_bytes(info.size(), info.align())
            } else {
                allocate_and_register_rooted_memory(self, info)
            }
        }

        #[inline(always)]
        unsafe fn realloc_info<Sized? T, Sized? U>(&self, old_ptr: *mut T, info: MemoryBlockInfo<U>) -> *mut U {
            assert!(!type_is_gc_allocated::<T>());
            if ! type_reaches_gc::<T>() {
                self.realloc_bytes(old_ptr, info.size(), info.align())
            } else {
                reallocate_and_register_rooted_memory(self, old_ptr, info)
            }
        }

        #[inline(always)]
        unsafe fn dealloc_info<Sized? T>(&self, pointer: *mut T, info: MemoryBlockInfo<T>) {
            if ! type_reaches_gc::<T>() {
                self.dealloc_bytes(pointer, info.size(), info.align())
            } else {
                deallocate_and_unregister_rooted_memory(self, pointer, info)
            }
        }
    }

    impl<A:AllocInfo> Alloc for A {
        unsafe fn alloc<T>(&self) -> *mut T {
            self.alloc_info(MemoryBlockInfo::<T>::from_type())
        }

        unsafe fn dealloc<T>(&self, pointer: *mut T) {
            self.dealloc_info(MemoryBlockInfo::<T>::from_type())
        }
    }

    impl<A:AllocInfo> ArrayAlloc for A {
        unsafe fn alloc_array<T>(&self, capacity: uint) -> *mut T {
            self.alloc_info(MemoryBlockInfo::<T>::array(capacity))
        }

        unsafe fn usable_capacity<T>(&self, capacity: uint) -> uint {
            let info = MemoryBlockInfo::<T>::array(capacity);
            self.raw.usable_size(info.size, info.align);
        }

        unsafe fn realloc_array<T>(&self,
                                       old_ptr_and_capacity: (*mut T, uint),
                                       new_capacity: uint) -> *mut T {
            let (op, oc) = old_ptr_and_capacity;
            self.realloc_info(op, MemoryBlockInfo::<T>::array(new_capacity))
        }

        unsafe fn dealloc_array<T>(&self, ptr_and_capacity: (*mut T, uint)) {
            let (op, oc) = ptr_and_capacity;
            self.dealloc_info(op, MemoryBlockInfo::<T>::array(oc))
        }

        #[inline(always)]
        unsafe fn deinit_range<T>(&self, start: *mut T, count: uint) {
            if ! type_reaches_gc::<T>() {
                /* no-op */
                return;
            } else {
                deinit_range_gc(start, count)
            }
        }
    }

    fn allocate_and_register_rooted_memory<Raw:RawAlloc, Sized? T>(raw: &Raw, info: MemoryBlockInfo<T>) -> *mut T {
        // Standard library provided method.
        //
        // Allocates memory with an added (hidden) header that allows
        // the GC to scan the memory for roots.
        ...
    }
    fn reallocate_and_register_rooted_memory<Raw:RawAlloc, Sized? T>(raw: &Raw, old_ptr: *mut T, info: MemoryBlockInfo<T>) -> *mut T {
        // Standard library provided method.
        //
        // Adjusts `old_ptr` to compensate for hidden header;
        // reallocates memory with an added header that allows the GC
        // to scan the memory for roots.
        ...
    }
    fn deallocate_and_unregister_rooted_memory<Raw:RawAlloc, Sized? T>(raw: &Raw, old_ptr: *mut T, info: MemoryBlockInfo<T>) -> *mut T {
        // Standard library provided method.
        //
        // Adjusts `old_ptr` to compensate for hidden header; removes
        // memory from the GC root registry, and deallocates the
        // memory.
        ...
    }
    fn deinit_range_gc<T>(&self, start: *mut T, count: uint) {
        // Standard library provided method.
        //
        // Zeros the address range so that the GC will not mistakenly
        // interpret words there as roots.
        ...
    }
}
```


TODO: Reap support? [ReCustomMalloc]  More generally, how do
extensions to the allocator API work?  Do they need to come in
pairs, i.e. where you use a trait-extension of RawAlloc, and then
    make another struct like `Alloc` that exposes the new method?
But really, that seems like a non-starter to me.
(Using a wrapper struct that hides the underlying RawAlloc may
itself also be a non-starter; maybe better to add the mixed-in
methods via an adapter trait.

TODO: spell out more of GC integration?  E.g.: can the GC use the
given RawAlloc to manage some of its own meta-data?  (How could that
be sane, unless it is directly coupled with the block being allocated
itself?)

GC correspondence option 1: say that GC is *allowed* to piggy-back headers on an
allocated block, but all state with extent not in 1:1 correspondence
with explicitly managed objects will *not* be handled by the given
RawAlloc.

GC correspondence option 2: say that all bets are off in terms of predicting
correspondence of Alloc calls with RawAlloc calls when GC-reaching
types are involved.  This provides us with more freedom when it
comes to choosing strategies for tracking the roots (e.g. managing
buckets)

GC extracting roots design issues: When it is time to trace the heap,
the GC will want to find all roots among the natively-allocated objects.
There are a few basic approaches to this that I can imagine, but they
all amount to the following options.

GC root tracking option 1 (block registry): When an allocator
allocates a block, it is responsible for registering that block in the
task-local GC state.  The manner of registration is a detail of the GC
design (it could adds a header that includes fields for a
doubly-linked list that is traversed by the GC; or it could record an
entry in a bitmap aka pagemap).

GC root tracking option 2 (allocator registry): When an allocator is
first used to allocate GC storage, it is added to a task-local
registry of of GC-enabled allocators.  Then when the GC does root
scanning, it iterates over the registered allocators, asking each to
provide the addresses it needs to scan.  Allocators in this design
need to carry state (to enumerate the roots in address ranges they
have allocated); they also must implement Drop so that they can remove
themselves from the allocator registry.  This variant basically
necessitates that we have a real wrapper around the RawAlloc.

GC root tracking potential solution: Put the choice in the hands of
the user, by supporting both options.  Option 1 is the default
provided by the normal RawAlloc interface.  Extend the AllocInfo
interface to allow opt-in support for option 2, so that the user (and
more importantly, the system runtime) can provide support for more
direct address scanning.

TODO: Niko's RFC had much discussion of PointerData and the mechanics
of how DST allocation worked.  However, I think that was an artifact
of a particular viewpoint in that RFC, namely the assumption that users
would be reimplementing those bits themselves and thus would need to
know by what mechanism that all worked.  I think now that we are controlling
    
# Drawbacks

Why should we *not* do this?

# Alternatives

What other designs have been considered? What is the impact of not doing this?

# Unresolved questions

## Platform supported page size

It is a little ugly that the `RawAlloc` has an error case for an
`align` that is too large, but there is no way in the interface for
the user to ask for that limit.  We could make the limit an associated
constant on `RawAlloc`.


## What is the type for an alignment

[RFC PR 39] deliberately used a `u32` for alignment, for compatibility
with LLVM.  This RFC is currently using `uint`, but only because our
existing `align_of` primitives expose `uint`, not `u32`.

# References

[RFC PR 39]: https://github.com/rust-lang/rfcs/pull/39/files

[ReCustomMalloc]: http://dl.acm.org/citation.cfm?id=582421

[jemalloc]: http://www.canonware.com/jemalloc/

[tcmalloc]: http://goog-perftools.sourceforge.net/doc/tcmalloc.html

[Hoard]: http://www.hoard.org/

[tracing garbage collector]: http://en.wikipedia.org/wiki/Tracing_garbage_collection
