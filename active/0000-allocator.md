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

This RFC deliberately leaves some implementation details unspecified
and left for a future RFC after we have more direct experience with
the API's proposed here; for more discussion, see the section:
[Why is half of the implementation missing][#wheres-the-beef].

# Table of Contents

* [Summary][#summary]
* [Table of Contents][#table-of-contents]
* [Motivation][#motivation]
  * [Why custom allocators][#why-custom-allocators]
  * [Why this API][#why-this-api]
  * [Why is half of the implementation missing][#wheres-the-beef]
* [Detailed design][#detailed-design]
  * [The RawAlloc trait][#the-rawalloc-trait]
  * [The typed_alloc module][#the-typedalloc-module]
    * [Properties of high-level allocators][#properties-of-high-level-allocators]
    * [The high-level allocator API][#the-high-level-allocator-api]
* [Drawbacks][#drawbacks]
* [Alternatives][#alternatives]
  * [Type-carrying Alloc][#type-carrying-alloc]
  * [No Alloc traits][#no-alloc-traits]
* [Unresolved Questions][#unresolved-questions]
  * [Platform supported page size][#platform-supported-page-size]
  * [What is the type of an alignment][#what-is-the-type-for-an-alignment]
  * [Reap support][#reap-support]
* [Appendices][#appendices]
  * [Bibligraphy][#bibliography]
  * [Terminology][#terminology]
  * [Non-normative high-level allocator implementation][#non-normative-high-level-allocator-implementation]

# Motivation

## Why Custom Allocators

As noted in [RFC PR 39], modern general purpose allocators are good,
but due to the design tradeoffs they must make, cannot be optimal in
all contexts.  (It is worthwhile to also read discussion of this claim
in papers such as [ReCustomMalloc] and [MemFragSolvedP].)
FIXME: is [MemFragSolvedP] actually relevant to the point here?)

Therefore, the standard library should allow clients to plug in their
own allocator for managing memory.

The typical reasons given for use of custom allocators in C++ are among the
following:

  1. Speed: A custom allocator can be tailored to the particular
     memory usage profiles of one client.  This can yield advantages
     such as:

     * A bump-pointer based allocator, when available, is faster
       than calling `malloc`.

     * Adding memory padding can reduce/eliminate false sharing of
       cache lines.

  2. Stability: By segregating different sub-allocators and imposing
     hard memory limits upon them, one has a better chance of handling
     out-of-memory conditions.  If everything comes from a global
     heap, it becomes much harder to handle out-of-memory conditions
     because the handler is almost certainly going to be unable to
     allocate any memory of its own work.

  3. Instrumentation and debugging: One can swap in a custom
     allocator that collects data such as number of allocations
     or time for requests to be serviced.

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
constraints (the latter was originally overlooked in the C++ `std`
STL).  The `RawAlloc` client can attempt to adjust the storage in use
in a copy-free manner by observing the memory block's current capacity
via a `usable_size` call.

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
allocator API, here called the `typed_alloc` module, parameterized over
its underlying `RawAlloc`.  Libraries are meant to use the
`typed_alloc` API, which will maintain garbage collection meta-data
when necessary (but only when allocating types that involve `Gc`).
The code-paths for the `typed_alloc` procedures are optimized with
fast-paths for when the allocated type does not contain `Gc<T>`.

The user-specified instance of `RawAlloc` is not required to attempt
to provide GC-support itself.  The user-specified allocator is only
meant to satisfy a simple, low-level interface for allocating and
freeing memory.  The support for garbage-collection is handled at a
higher level, within the Rust standard library itself.

## Where's the beef
or, Why is half of the implementation "missing"

This RFC only specifies the API's that one would use to implement a
custom low-level allocator and then use it (indirectly) from client
library code.  In particular, it specifies both a low-level API for
managing blocks of raw bytes, and a high-level client-oriented API for
for managing blocks holding instances of typed objects, but does *not*
provide a formal specification of all the primitive intrinsic
operations one would need to actually *implement* the high-level API
directly.

(There is a non-normative [appendix](#non-normative-high-level-allocator-implementation)
that contains a sketch of how the high-level API might be implemented,
to show concretely that the high-level API is *implementable*.)

This RFC includes the specification of the high-level API because it
is what libraries are expected to be written against, and thus its
interface is arguably even more important than the interface than the
low-level API.

This RFC also specifies that the standard library will provide types
for building high-level allocators up from instances of the low-level
allocators.  We expect that for most use-cases where custom allocators
are required, it should suffice to define a [low-level
allocator](#the-rawalloc-trait), which this RFC *does* include enough
information for users to work with today, and then construct a
high-level allocator directly from that low-level one.  Note
especially that when GC data is not involved, all of the standard
high-level allocator operations are meant to map directly to low-level
allocator methods, without any added runtime overhead.

The reason that this RFC does not include the definitions of the
intrinsics needed to implement instances of the high-level allocator
is that we do not want to commit prematurely to a set of intrinsics
that we have not yet had any direct experience working with.
Additionally, the details of those intrinsics are probably
uninteresting to the majority of users interested in custom
allocators.  (If you are curious, you are more than welcome to read
and provide feedback on the sketch in the non-normative
[appendix](#non-normative-high-level-allocator-implementation); just
keep in mind that the intrinsics and helper methods there are not part
of the specification proposed by this RFC.

# Detailed design

## The RawAlloc trait

Here is the `RawAlloc` trait design.  It is largely the same
as the design from [RFC PR 39]; points of departure are
enumerated after the API listing.

```rust
type Size = uint;
type Capacity = uint;
type Alignment = uint;

/// Low-level explicit memory management support.
///
/// Defines a family of methods for allocating and deallocating
/// byte-oriented blocks of memory.  A user-implementation of
/// this trait need only reimplement `alloc_bytes` and `dealloc_bytes`,
/// though it 
///
/// Several of these methods state as a condition that the input
/// `size: Size` and `align: Alignment` must *fit* an input pointer
/// `ptr: *mut u8`.
///
/// What it means for `(size, align)` to "fit" a pointer `ptr` means
/// is that the following two conditions must hold:
///
/// 1. The `align` must have the same value that was last used to
///    create `ptr` via one of the allocation methods,
///
/// 2. The `size` parameter must fall in the range `[orig, usable]`, where:
///
///    * `orig` is the value last used to create `ptr` via one of the
///      allocation methods, and
///
///    * `usable` is the capacity that was (or would have been)
///      returned when (if) `ptr` was created via a call to
///      `alloc_bytes_excess` or `realloc_bytes_excess`.
///
/// "The allocation methods" above refers to one of `alloc_bytes`,
/// `realloc_bytes`, `alloc_bytes_excess`, and `realloc_bytes_excess")
///
/// Note that due to the constraints in the methods below, a
/// lower-bound on `usable` can be safely approximated by a call to
/// `usable_size_bytes`.


pub trait RawAlloc {
    /// Returns a pointer to `size` bytes of memory, aligned to
    /// a `align`-byte boundary.
    ///
    /// Returns null if allocation fails.
    ///
    /// Behavior undefined if `size` is 0 or `align` is not a
    /// power of 2, or if the `align` is larger than the largest
    /// platform-supported page size.
    unsafe fn alloc_bytes(&self, size: Size, align: Alignment) -> *mut u8;

    /// Deallocate the memory referenced by `ptr`.
    ///
    /// `(size, align)` must *fit* the `ptr` (see above).
    ///
    /// Behavior is undefined if above constraints on `align` and
    /// `size` are unmet. Behavior also undefined if `ptr` is null.
    unsafe fn dealloc_bytes(&self, ptr: *mut u8, size: Size, align: Alignment);

    /// Returns a pointer to `size` bytes of memory, aligned to
    /// a `align`-byte boundary, as well as the capacity of the
    /// referenced block of memory.
    ///
    /// Returns `(null, c)` for some `c` if allocation fails.
    ///
    /// A successful allocation will by definition have capacity
    /// greater than or equal to the given `size` parameter.
    /// A successful allocation will also also have capacity greater
    /// than or equal to the value of `self.usable_size_bytes(size,
    /// align)`.
    ///
    /// Behavior undefined if `size` is 0 or `align` is not a
    /// power of 2, or if the `align` is larger than the largest
    /// platform-supported page size.
    unsafe fn alloc_bytes_excess(&self, size: Size, align: Alignment) -> (*mut u8, Capacity) {
        // Default implementation: just conservatively report the usable size
        // according to the underlying allocator.
        (self.alloc_bytes(size, align), self.usable_size_bytes(size, align))
    }

    /// Extends or shrinks the allocation referenced by `ptr` to
    /// `size` bytes of memory, retaining the alignment `align`.
    ///
    /// `(old_size, align)` must *fit* the `ptr` (see above).
    ///
    /// If this returns non-null, then the memory block referenced by
    /// `ptr` may have been freed and should be considered unusable.
    ///
    /// Returns null if allocation fails; in this scenario, the
    /// original memory block referenced by `ptr` is unaltered.
    ///
    /// Behavior is undefined if above constraints on `align` and
    /// `old_size` are unmet. Behavior also undefined if `size` is 0.
    unsafe fn realloc_bytes(&self, ptr: *mut u8, size: Size, align: Alignment, old_size: Size) -> *mut u8 {
        if size <= self.usable_size_bytes(old_size, align) {
            return ptr;
        } else {
            let new_ptr = self.alloc_bytes(size, align);
            if new_ptr.is_not_null() {
                ptr::copy_memory(new_ptr, ptr as *const u8, cmp::min(size, old_size));
                self.dealloc_bytes(ptr, old_size, align);
            }
            return new_ptr;
        }
    }

    /// Extends or shrinks the allocation referenced by `ptr` to
    /// `size` bytes of memory, retaining the alignment `align`,
    /// and returning the capacity of the (potentially new) block of
    /// memory.
    ///
    /// `(old_size, align)` must *fit* the `ptr` (see above).
    ///
    /// When successful, returns a non-null ptr and the capacity of
    /// the referenced block of memory.  The capacity will be greater
    /// than or equal to the given `size`; it will also be greater
    /// than or equal to `self.usable_size_bytes(size, align)`.
    ///
    /// If this returns non-null (i.e., when successful), then the
    /// memory block referenced by `ptr` may have been freed, and
    /// should be considered unusable.
    ///
    /// Returns `(null, c)` for some `c` if reallocation fails; in
    /// this scenario, the original memory block referenced by `ptr`
    /// is unaltered.
    ///
    /// Behavior is undefined if above constraints on `align` and
    /// `old_size` are unmet. Behavior also undefined if `size` is 0.
    unsafe fn realloc_bytes_excess(&self, ptr: *mut u8, size: Size, align: Alignment, old_size: Size) -> (*mut u8, Capacity) {
        (self.realloc_bytes(ptr, size, align, old_size), self.usable_size_bytes(size, align))
    }

    /// Returns the minimum guaranteed usable size of a successful
    /// allocation created with the specified `size` and `align`.
    ///
    /// Clients who wish to make use of excess capacity are encouraged
    /// to use the `alloc_bytes_excess` and `realloc_bytes_excess`
    /// instead, as this method is constrained to conservatively
    /// report a value less than or equal to the minimum capacity for
    /// all possible calls to those methods.
    ///
    /// However, for clients that do not wish to track the capacity
    /// returned by `alloc_bytes_excess` locally, this method is
    /// likely to produce useful results; e.g. in an allocator with
    /// bins of blocks categorized by size class, the capacity will be
    /// the same for any given `(size, align)`.
    #[inline(always)]
    unsafe fn usable_size_bytes(&self, size: Size, align: Alignment) -> Capacity {
        size
    }
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

  * Added `_with_excess` variants of the allocation methods that
    return the "true" usable capacity for the block.  This variant
    was discussed a bit in the comment thread for [RFC PR 39].
    I chose not to make the tuple-returning forms the *only* kind
    allocation method, so that simple clients who will not
    use the excess capacity can remain simple.

## The typed_alloc module

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

### Properties of high-level allocators

When splitting between a high-level `Alloc` and a low-level `RawAlloc`,
there are questions that arise regarding how the high-level operations
of `Alloc` actually map to the low-level methods provided by `RawAlloc`.
Here are a few properties of potential interest when thinking about
this mapping.

#### Headerless high-level allocation

A "header-free" high-level allocation is one where the high-level
allocator implementation adds no headers to the block associated with
the storage for one value; more specfically, the size of the memory
block allocated to represent a type `T` is (at most) the size of what
the underlying `RawAlloc` would return for a request a block of size
`mem::size_of::<T>()` and alignment `mem::align_of::<T>()`.  (We say
"at most" because the `Alloc` implementation may choose to use
`mem::min_align_of::<T>()`; that detail does not matter in terms of
the spirit of what "header-free allocation" means.

#### Call correspondence

A "call correspondence" between a high-level allocator and one of
its underlying `RawAllocs` is a summary of how many calls will be
made to the methods of the `RawAlloc` in order to implement the
corresponding method of the high level allocator.

Every high-level allocator provides at least the methods for
allocating and deallocating data (which can correspond to
`alloc_bytes` and `dealloc_bytes`), and potentially also a method
for attempting to reallocate data in-place (which can correspond to
`realloc_bytes`).  I call these methods the "allocation methods",
though note that they include both allocate and deallocate methods.
I have identified three potentially interesting call
correspondences, which I have labelled as "1:1", "1:1+", and "1:n".

  * If a high-level allocator has a "1:1" call correspondence with a
    raw allocator, that means that every successful call to an
    allocation method corresponds to exactly one successful call to
    the corresponding method of the underlying raw allocator.

    Note that the successful raw allocator call may have been preceded
    by zero or more unsuccessful calls, depending on what kind of
    policy the high-level allocator is using to respond to allocation
    failure.

    If the raw allocator is serving just that single high-level
    allocator, then a "1:1" call correspondence also indicates that
    every successful call to a raw allocator method can be matched
    with exactly one call to some method of the high-level allocator.

    This latter feature makes the "1:1" correspondence a fairly strong
    property to satisfy, since it means that the high-level allocator
    must be using the raw allocator solely to service the requests of
    the end client directly, without using the raw allocator to
    allocate memory for the high-level allocator to use internally.

    The "1:1" correspondence describes high-level allocators that
    massage their requests and then pass them over to the low-level
    raw allocator, but do not use that raw allocator to dynamically
    allocate any state for their own private usage.

  * If a high-level allocator has a "1:1+" call correspondence with
    a raw allocator, then every successful call to an allocation method
    corresponds to one successful call to the corresponding method of the
    underlying raw allocator, plus zero or more calls to other methods
    of the raw allocator.

    This is a weaker property than "1:1" correspondence, because it
    allows for the high-level allocator to use the raw allocator
    to construct internal meta-data for the high-level allocator,
    as well as provide the backing storage for end client requests.

  * If a high-level allocator has a "1:n" call correspondence with a
    raw allocator, then a successful call to an allocation method
    implies that at least one successful call to *some* method of the
    raw allocator was made at some point, potentially during some
    prior allocation method call.

    This is a very weak property; it essentially means that no
    prediction should be made a priori about how calls to the
    high-level allocator will map to the low-level raw allocator.

    The "1:n" correspondence describes high-level allocators that, for
    example, create private bins of storage via the low-level
    allocator and then service end-client requests directly from those
    bins without going through the raw allocator until their
    high-level internal policy demands.  This kind of allocator can
    subvert the goals of a low-level raw allocator, since it can hide
    the memory usage patterns of the end client from the raw
    allocator.

### The high-level allocator API

Here is the `typed_alloc` API design.  Much of it is similar to the
`RawAlloc` trait, but there are a few extra pieces added for type-safe
allocation, dyanmically-sized types, and GC support.

```rust
mod typed_alloc {
    trait InstanceAlloc {
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
    //
    // * `type_invoves_gc::<T>()`: a short-hand for
    //   `type_is_gc_allocated::<T>() || type_reaches_gc::<T>()`

    // Notes on the abstract "standard library" implementation
    //
    // * The standard library will be able to allocate GC managed data.
    //
    // * For non GC-root carrying data, it provides a "1:1" call
    //   correspondence with its given raw alloc.
    //
    // * We make no guarantees about the call correspondence for
    //   GC-root carrying (but not GC-managed) data; it will probably
    //   have the "1:n" call correspondence (which, as noted above, is
    //   a very weak property).
    //
    // * As for GC-managed data: It may or may not attempt to use the
    //   given raw allocator to provide backing storage for the
    //   GC-heap.

    struct Alloc<Raw:RawAlloc> { ... }

    impl<Raw:RawAlloc> Alloc {
        unsafe fn alloc_info_gc<Sized? T>(&self, info: MemoryBlockInfo<T>) -> *mut T { ... }
        unsafe fn realloc_info_gc<Sized? T, Sized? U>(&self, old_ptr: *mut T, info: MemoryBlockInfo<U>) -> *mut U { ... }
        unsafe fn dealloc_info_gc<Sized? T>(&self, pointer: &mut T, info: MemoryBlockInfo<T>) { ... }
    }

    // FIXME: *none* of these AllocCore impls are DST aware.

    impl<Raw:RawAlloc> AllocCore for Alloc {
        #[inline(always)]
        unsafe fn alloc_info<Sized? T>(&self, info: MemoryBlockInfo<T>) -> *mut T {
            // (compile-time evaluated conditions)
            if ! type_involves_gc::<T>() {
                self.alloc_bytes(info.size(), info.align())
            } else {
                self.alloc_info_gc(info)
            }
        }

        #[inline(always)]
        unsafe fn realloc_info<Sized? T, Sized? U>(&self, old_ptr: *mut T, info: MemoryBlockInfo<U>) -> *mut U {
            // (compile-time evaluated conditions)
            if ! type_involves_gc::<T>() && ! type_involves_gc::<U>() {
                self.realloc_bytes(old_ptr, info.size(), info.align())
            } else {
                self.realloc_info_gc(old_ptr, info)
            }
        }
        #[inline(always)]
        unsafe fn dealloc_info<Sized? T>(&self, pointer: *mut T, info: MemoryBlockInfo<T>) {
            // (compile-time evaluated conditions)
            if ! type_involves_gc::<T>() {
                self.dealloc_bytes(pointer, info.size(), info.align())
            } else {
                self.dealloc_info_gc(pointer, info)
            }
        }
    }

    // Notes on the direct `RawAlloc` based implementation.
    //
    // * Raw allocators cannot directly allocate on the gc-heap.
    //
    // * When treated as a high-level allocator, a raw allocator has a
    //   1:1 call correspondence with itself.

    /// This high-level allocator dispatches all calls to the underlying
    /// underlying raw allocator, adding a header when the allocated type
    /// contains GC roots.  Note that it cannot allocate to the GC-heap
    /// itself.

    struct Direct<Raw:RawAlloc>(Raw)

    impl<Raw:RawAlloc> AllocCore for Direct<Raw> {
        #[inline(always)]
        unsafe fn alloc_info<Sized? T>(&self, info: MemoryBlockInfo<T>) -> *mut T {
            // (compile-time evaluated conditions)
            assert!(!type_is_gc_allocated::<T>());
            if ! type_reaches_gc::<T>() {
                self.alloc_bytes(info.size(), info.align())
            } else {
                allocate_and_register_rooted_memory(self, info)
            }
        }

        #[inline(always)]
        unsafe fn realloc_info<Sized? T, Sized? U>(&self, old_ptr: *mut T, info: MemoryBlockInfo<U>) -> *mut U {
            // (compile-time evaluated conditions)
            assert!(!type_is_gc_allocated::<T>());
            assert!(!type_is_gc_allocated::<U>());
            if ! type_reaches_gc::<T>() && ! type_reaches_gc::<U>() {
                self.realloc_bytes(old_ptr, info.size(), info.align())
            } else {
                reallocate_and_register_rooted_memory(self, old_ptr, info)
            }
        }

        #[inline(always)]
        unsafe fn dealloc_info<Sized? T>(&self, pointer: *mut T, info: MemoryBlockInfo<T>) {
            // (compile-time evaluated conditions)
            assert!(!type_is_gc_allocated::<T>());
            if ! type_reaches_gc::<T>() {
                self.dealloc_bytes(pointer, info.size(), info.align())
            } else {
                deallocate_and_unregister_rooted_memory(self, pointer, info)
            }
        }
    }

    impl<A:AllocCore> Alloc for A {
        unsafe fn alloc<T>(&self) -> *mut T {
            self.alloc_info(MemoryBlockInfo::<T>::from_type())
        }

        unsafe fn dealloc<T>(&self, pointer: *mut T) {
            self.dealloc_info(MemoryBlockInfo::<T>::from_type())
        }
    }

    impl<A:AllocCore> ArrayAlloc for A {
        unsafe fn alloc_array<T>(&self, capacity: uint) -> *mut T {
            self.alloc_info(MemoryBlockInfo::<T>::array(capacity))
        }

        // FIXME: self does not have a `raw` in this context.
        // (And this interface may need to change anyway.)
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

## Type-carrying Alloc
(aka "objects verus generics")

While it will sometimes make sense to provide a low-level allocator as
an raw allocator object type `&RawAlloc`, e.g. to control
code-duplication, in general we here define the high-level type-aware
methods as type-parametric methods of a high-level trait, such as the
method `fn alloc<T>(&self) -> *mut T` of the `Alloc` trait.  (Note
that since all of the methods of `Alloc` are type-parametric, a
trait-object type `&Alloc` has no callable methods, because Rust does
not allow one to invoke type-parametric methods of trait objects.)

That is, we did not attempt to encode the high-level interface using
solely traits with type-specific implementations, such as suggested
by a signature like:
```rust
trait AllocJust<Sized? T> { fn alloc(&self) -> *mut T; ... }
```

While a trait like `AllocJust<T>` is attractive, since it could then
be realized as a (useful) object-type `&AllocJust<T>`, this API is not
terribly useful as the basis for an allocator in a library
(at least not unless it is actually a trait for a higher-kinded
type `AllocJust: type -> type`), because:

1. The library developer would be forced to take a distinct `AllocJust<T>`
   for each `T` allocated in the library, and

2. It would force the library developer to expose some types `T` that would
   otherwise be private implementation details of the libraries.

A concrete example of this is the reference-counted type `Rc<T>`, assuming we
make it allocator-parametric: if we used an `AllocJust<T>` API, the resulting
code would look like this:
```rust
/// This struct definition should be private to the internals of `rc` ...
struct RcBox<Sized? T> {
    ref_count: uint,
    data: T,
}

/// ... but `RcBox` cannot be private, because of the `A` parameter here.
struct Rc<Sized? T, A:AllocJust<RcBox> = DefaultAllocJust<RcBox>> {
    box: RcBox<T>,
}
```

## No `Alloc` traits
No `Alloc` traits; just `RawAlloc` parameteric methods in `typed_alloc`

When the two-level approach was first laid out, we thought we might
just have a single standard high-level allocator, and clients would
solely implement instances of the `RawAlloc` trait.  The single
standard high-level allocator would be encoded as struct provided
in the `typed_alloc` module, and much like the trait implementations above,
it would directly invoke the underlying `RawAlloc` on requests involving
non GC-root carrying data.

The reason I abandoned this approach was that when I reconsidered the
various [call correspondence][#call-correspondence] properties, I
realized that our initial approach to how the standard high-level
allocator would handle GC data, via a "1:n" call correspondence, was
arguably *subverting* the goals of a custom allocator.  For example,
if the goal of the custom allocator is to instrument the allocation
behavior of a given container class, then a "1:n" call correspondence
is probably not acceptable, since the data captured by the low-level
allocator does not correspond in a meaningful way to the actual
allocation calls being made by the container library.

Therefore, I decided to introduce the family of `Alloc` traits, along
with a few standard implementations of those traits (namely, the
`Alloc` and `Direct` structs) that we know how to implement, and let
the end user decide how they want GC-root carrying data to be handled
by selecting the appropriate implementation of the trait.

## try_realloc

I have seen some criticisms of the C `realloc` API that say that
`realloc` fails to capture some important use cases, such as a request
to "attempt to extend the data-block in place to match my new needed
size, but if the attempt fails, do not allocate a new block (or in
some variations, do allocate a new block of the requested size, but do
not waste time copying the old data over to it."

(A use-case where this arises is when one is expanding some given
object, but one is also planning to immediately fill it with fresh
data, making the copy step useless.)

We could offer specialized methods like these in the `RawAlloc` interface,
with a signature like
```rust
fn try_grow_bytes(&self, ptr: *mut u8, size: uint, align: uint, old_size: uint) -> bool
```

Or we could delay such additions to hypothetical subtraits e.g. `RawTryAlloc`.

Or we could claim that given `RawAlloc` API, with its 

## ptr parametric `usable_size`

I considered extending `usable_size_bytes` to take the `ptr` itself as an
argument, as another way to handle hypothetical allocators who may choose
different sized bins given the same `(size, align)` input.

But in the end I agreed with the assertion from [RFC PR 39], that in
practice most allocators that override the default implementation will
actually return a constant-expression computed solely from the given
`size` and `align`, and I decided it was simpler to support the above
hypothetical allocators with different size bins via the
`alloc_bytes_excess` and `realloc_bytes_excess` methods.

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

## Reap support

After (re-)reading [ReCustomMalloc], I am thinking that we should make
sure our allocator design allows for easy use of a *Reap* abstraction.

A "Reap" is like an arena, in that it optimizes for stack-like
allocation patterns where large amounts of data is predicted to all
die at the same time, but a reap also allows for individual deletions
of objects within its structure.

If possible, I do not want worry about trying to extend this API with
"Reap" support.  I will be happy if I can be convinced that a library
will be able to implement and supply a reap abstraction that is
compatible with this RFC.  For example, would it suffice to define a
trait-extension of `RawAlloc`, such as
```rust
trait RawReapAlloc : RawAlloc { ... }
```
and make corresponding variants of the high-level allocator traits?

# Appendices

## Bibliography

[RFC PR 39]: https://github.com/rust-lang/rfcs/pull/39/files

[ReCustomMalloc]: http://dl.acm.org/citation.cfm?id=582421

[MemFragSolvedP]: http://dl.acm.org/citation.cfm?id=286864

[jemalloc]: http://www.canonware.com/jemalloc/

[tcmalloc]: http://goog-perftools.sourceforge.net/doc/tcmalloc.html

[Hoard]: http://www.hoard.org/

[tracing garbage collector]: http://en.wikipedia.org/wiki/Tracing_garbage_collection

[malloc/free]: http://en.wikipedia.org/wiki/C_dynamic_memory_allocation

[EASTL]: http://www.open-std.org/jtc1/sc22/wg21/docs/papers/2007/n2271.html

[Halpern proposal]: http://www.open-std.org/jtc1/sc22/wg21/docs/papers/2005/n1850.pdf

## Terminology

* Size-tracking allocator: An allocator which embeds all allocator
  meta-data such as block size into the allocated block itself, either
  explicitly (e.g. via a header), or implicitly (e.g. by maintaining a
  separate map from address-ranges to sizes).  The C [malloc/free]
  functions form an example of such an API: since the `free` function
  takes only a pointer, the allocator is forced to embed that
  meta-data into the block itself.

* Stateful allocator:

* GC-root carrying data:

## Non-normative high-level allocator implementation

The high-level allocator traits in [the-typed_alloc-module] 
