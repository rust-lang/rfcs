- Start Date: (fill me in with today's date, 2014-08-31)
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

This RFC does not attempt to specify a so-called "pluggable" GC
library.  We assume here that any GC support is built-in to the Rust
compiler and standard library; we leave work on pluggable GC as future
research.

This RFC deliberately leaves some implementation details unspecified
and left for a future RFC after we have more direct experience with
the API's proposed here; for more discussion, see the section:
[Why is half of the implementation missing](#wheres-the-beef).

# Table of Contents

* [Summary](#summary)
* [Table of Contents](#table-of-contents)
* [Motivation](motivation)
  * [Why custom allocators](#why-custom-allocators)
  * [Why this API](#why-this-api)
  * [Why is half of the implementation missing](#wheres-the-beef)
* [Detailed design](#detailed-design)
  * [The RawAlloc trait](#the-rawalloc-trait)
  * [The high-level allocator API](#the-high-level-allocator-api)
    * [Potential properties of high-level allocators](#potential-properties-of-high-level-allocators)
      * [Header-free high-level allocation](#header-free-high-level-allocation)
      * [Call correspondences](#call-correspondence)
      * [Backing memory correspondence](#backing-memory-correspondence)
      * [Why these properties are useful](#why-these-properties-are-useful)
    * [The typed_alloc module](#the-typed_alloc-module)
* [Drawbacks](#drawbacks)
* [Alternatives](#alternatives)
  * [Type-carrying Alloc](#type-carrying-alloc)
  * [No Alloc traits](#no-alloc-traits)
  * [RawAlloc variations](#rawalloc-variations)
    * [try_realloc](#try_realloc)
    * [ptr parametric `usable_size`](#ptr-parametric-usable_size)
  * [High-level allocator variations](#high-level-allocator-varitions)
    * [Make `ArrayAlloc` extend `InstanceAlloc`](#make-arrayalloc-extend-instancealloc)
    * [make-alloccore-non-normative](#make-alloccore-non-normative)
* [Unresolved Questions](#unresolved-questions)
  * [Platform supported page size](#platform-supported-page-size)
  * [What is the type of an alignment](#what-is-the-type-for-an-alignment)
  * [Reap support](#reap-support)
* [Appendices](#appendices)
  * [Bibliography](#bibliography)
  * [Glossary](#glossary)
  * [Non-normative high-level allocator implementation](#non-normative-high-level-allocator-implementation)

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
freeing memory.  Built-in support for garbage-collection is handled at a
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
of the specification proposed by this RFC.)

# Detailed design

The allocator API is divided into two levels: a byte-oriented
low-level *raw allocator* trait (named [`RawAlloc`](#the-rawalloc-trait)),
and a family of type-driven high-level *typed allocator* traits
(defined in the [typed_alloc module](#the-typed_alloc-module)).

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
/// The phrase "the allocation methods" above refers to one of
/// `alloc_bytes`, `realloc_bytes`, `alloc_bytes_excess`, and
/// `realloc_bytes_excess")
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

## The high-level allocator API

The `RawAlloc` trait defines the low-level API we expect users to be
able to provide easily, either by dispatching to other native
allocator libraries (such as [jemalloc], [tcmalloc], [Hoard], et
cetera), or by implementing their own in Rust (!).

But the low-level API is not sufficient.  It is lacking in two ways:

* It does not integrate dynamically sized types (DST), in the sense
  that it does not address the problem of shifting from a thin-pointer
  (`*mut u8`) to a potentially fat-pointer (e.g. `*mut [T]`).

* It does not integrate garbage collection (GC).  Rust has been
  designed to allow an optional [tracing garbage collector] to manage
  graph-structured memory, but if all collections relied solely on the
  `RawAlloc` interface without also registering the potential GC roots
  held in the natively allocated blocks, we could not hope to put in a
  tracing GC.

Therefore, this RFC defines a high-level interface that is intended
for direct use by libraries.  The high-level interface defines a
small family  of allocator traits that correspond to how one allocates
instances of a sized type, arrays, or unsized types in general.

Crucially, the intention in this design is that every implementation
of a high-level allocator trait be parameterized over one or more raw
allocator instances (usually just one) that dictate the actual
low-level memory allocation.

This RFC does not define all of the primitive operations one would
need to implement an instance of a high-level allocator trait.
Instead, for now all this RFC says is that there will be at least
two constructor functions available to build a high-level allocator from a
raw allocator; these constructors are named `

### Potential properties of high-level allocators

When splitting between a high-level `Alloc` and a low-level `RawAlloc`,
there are questions that arise regarding how the high-level operations
of `Alloc` actually map to the low-level methods provided by `RawAlloc`.
Here are a few properties of potential interest when thinking about
this mapping.

#### Header-free high-level allocation

A "header-free" high-level allocation is one where the high-level
allocator implementation adds no headers to the block associated with
the storage for one value; more specfically, the size of the memory
block allocated to represent a type `T` is (at most) the size of what
the underlying `RawAlloc` would return for a request a block of size
`mem::size_of::<T>()` and alignment `mem::align_of::<T>()`.  (We say
"at most" because the `Alloc` implementation may choose to use
`mem::min_align_of::<T>()`; that detail does not matter in terms of
the spirit of what "header-free allocation" means.

#### Call correspondences

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

#### Backing memory correspondence

A high-level allocator will usually be parameterized over one or more
raw allocators.  One might infer that when one has a high-level
allocator with a single raw allocator parameter, then all of the
memory allocated by that high-level allocator must be provided solely
by its raw allocator.

This seems like a good design principle to follow, but may not be true
in general.  In particular, a high-level allocators may be
deliberately parameterized over a single raw allocator for simplicity
in its interface, but may still allocate internal state via means
other than the raw allocator.

Thus, as a way to distinguish these design choices, we say that a
high-level allocator is "fully-backed" by a given raw allocator if all
of its dynamically allocated state is managed by that raw allocator.
This is called the "full backing" property.  If some of a high-level
allocator's dynamically allocated state is not managed by any of its
raw allocators, then we say that it has "hidden backing."

#### Why these properties are useful

The properties above are worth discussing because they establish
constraints that may be important for clients of user-defined
allocators, and/or for the GC integration.

For example, when integrating the GC with a user-defined allocator
without requiring the user-defined allocator to be GC-aware, we could
piggy-back a header holding GC metadata on the block itself; this
option would lose header-free allocation, but would enable maintenance
of 1:1 call correspondence.

If we wanted to maintain header-free allocation with GC support, we
could store the meta-data elsewhere; this would force us to sacrifice
either the 1:1 call correspondence (e.g. if the high-level allocator
stored the meta-data in other blocks allocated via the same raw
allocator), or the full backing property (e.g. if the high-level
allocator stored the meta-data on a different raw allocator, or in
memory allocated via direct calls to native system `malloc`
functionality).

### The typed_alloc module

Here is the `typed_alloc` API design.  Much of it is similar to the
`RawAlloc` trait, but there are a few extra pieces added for type-safe
allocation, dyanmically-sized types, and GC support.

```rust
pub mod typed_alloc {
    /// High-level allocator for instances of some sized type.
    pub trait InstanceAlloc {
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

    /// High-level allocator for arrays.
    ///
    /// Some of the methods below say that an input length `len` must
    /// *fit* a memory block.
    ///
    /// What it means for `len` to "fit" a memory block `b` allocated
    /// via `ArrayAlloc` is that the `len` must fall in the range
    /// `[orig_len, usable]`, where:
    ///
    /// * `orig_len` is the original length used to create `b` via
    ///   one of the allocation methods of `ArrayAlloc`.
    ///
    /// * `usable` is `max(usable_cap, excess_cap)` (see below),
    ///
    /// * `usable_cap` is the result of any prior call to the
    ///   `usable_capacity` method for `len`,
    ///
    /// * `excess_cap` is the excess capacity returned if `b` was
    ///   allocated via one of the excess allocation methods, or 0
    ///   if `b` was allocated by another of the allocation methods.
    ///
    /// The phrase "the allocation methods of `ArrayAlloc`" above
    /// refers to one of `alloc_array`, `alloc_array_excess`,
    /// `realloc_array`, or `realloc_array_excess`; the "excess
    /// allocation methods" are the ones that end with the suffix
    /// "excess".
    pub trait ArrayAlloc {
        /// Allocates a memory block suitable for holding `len`
        /// instances of `T`.
        ///
        /// Returns the pointer to the block if allocation succeeds.
        ///
        /// Returns null if allocation fails.
        unsafe fn alloc_array<T>(&self, len: uint) -> *mut T;

        /// Allocates a memory block suitable for holding `len`
        /// instances of `T`, as well as the capacity (in
        /// number of `T`) of the referenced block of memory.
        ///
        /// Returns the pointer to the block if allocation succeeds.
        ///
        /// Returns null if allocation fails.
        unsafe fn alloc_array_excess<T>(&self, len: uint) -> (*mut T, uint);

        /// Given `len`, a length (in number of instances of `T`) that
        /// *fits* the memory block allocated for an array, returns a
        /// conservative bound on the capacity of that memory block,
        /// i.e. the number of contiguous instances of `T` that it can
        /// hold.
        ///
        /// Regardless of whether the condition above is met, always
        /// returns a value greater than or equal to `len`.
        unsafe fn usable_capacity<T>(&self, len: uint) -> uint;

        /// Extends or shrinks the allocation referenced by `old_ptr`
        /// to hold at least `new_len` instances of `T`.
        ///
        /// The `old_len` must *fit* the block referenced by `old_ptr`.
        ///
        /// If this returns non-null, then the memory block referenced by
        /// `old_ptr` may have been freed and should be considered unusable.
        ///
        /// Returns null if allocation fails; in this scenario, the
        /// original memory block referenced by `old_ptr` is unaltered.
        unsafe fn realloc_array<T>(&self,
                                   (old_ptr, old_len): (*mut T, uint),
                                   new_len: uint) -> *mut T;


        /// Extends or shrinks the allocation referenced by `old_ptr`
        /// to hold at least `new_len` instances of `T`.
        ///
        /// The `old_len` must *fit* the block referenced by `old_ptr`.
        ///
        /// If reallocation succeeds, returns `(p, c)` where `p` is
        /// the pointer to the memory block to hold `new_len`
        /// instances of `T`, and `c` is the capacity (in number of
        /// `T`) of the referenced block of memory.
        ///
        /// If this returns a non-null pointer, then the memory block
        /// referenced by `old_ptr` may have been freed and should be
        /// considered unusable.
        ///
        /// If the reallocation attempt fails, returns `(null, c)` for
        /// some `c`; in this scenario, the original memory block
        /// referenced by `old_ptr` is unaltered.
        unsafe fn realloc_array_excess<T>(&self,
                                          (old_ptr, old_len): (*mut T, uint),
                                          new_len: uint) -> (*mut T, uint);

        /// Frees memory block referenced by `ptr`.
        ///
        /// `len` must *fit* the block referenced by `ptr`.
        unsafe fn dealloc_array<T>(&self, (ptr, len): (*mut T, uint));

        /// Deinitializes the range of instances `[start, start+count)`.
        ///
        /// A container must call this function when (1.) it has
        /// previously initialized the instances of `T` in the
        /// aforementioned range, and (2.) it has since moved or
        /// dropped the instances of `T` out of the array.
        ///
        /// This method is solely to prevents dangling references to
        /// GC-able objects.  The alternative would be to require all
        /// `Gc<T>` to have a destructor, which is counter to the
        /// goals of tracing garbage collection (and also would make
        /// `Gc<T>` non-`Copy`, another non-starter).
        ///
        /// Note: Failing to call this should solely result in storage
        /// leaks, not in arbitrary unsound behavior.  (This note may
        /// not belong in the API doc, unless we want to clarify
        /// motivation and/or the sorts of bugs one can encounter when
        /// one fails to uphold the obligation.)
        /// XXX Niko please double-check my reasoning here.
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
    }

    pub trait AllocCore {
        /// Allocates a memory block suitable for holding `T`,
        /// according to the  `info`.
        ///
        /// Returns null if allocation fails.
        unsafe fn alloc_info<Sized? T>(&self, info: MemoryBlockInfo<T>) -> *mut T;

        /// Allocates a memory block suitable for holding `T`,
        /// according to the `info`, as well as the capacity (in
        /// bytes) of the referenced block of memory.
        ///
        /// Returns `(null, c)` for some `c` if allocation fails.
        unsafe fn alloc_info_excess<Sized? T>(&self, info: MemoryBlockInfo<T>) -> (*mut T, Capacity);

        /// Given a pointer to the start of a memory block allocated
        /// for holding instance(s) of `T`, returns the number of
        /// contiguous instances of `T` that `T` can hold.
        unsafe fn usable_capacity_bytes<Sized? T>(&self, len: uint) -> uint;

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

    /// The default standard library implementation of a high-level allocator.
    ///
    /// * Instances of `Alloc` are able to allocate GC managed data.
    ///
    /// * For data not involving the GC, the `Alloc<R>` provides
    ///   header-free allocation and a "1:1" call correspondence with
    ///   its given raw alloc `R`.
    ///
    /// * The implementation makes no guarantees about the call
    ///   correspondence for GC-root carrying (but not GC-managed)
    ///   data.  It will probably have the "1:n" call correspondence
    ///   (which, as noted above, is a very weak property).
    ///
    /// * For GC-managed data: the `Alloc` may or may not attempt to
    ///   use the given raw allocator to provide backing storage for
    ///   the GC-heap.
    pub struct Alloc<Raw:RawAlloc> { /* private fields */ }

    /// Constructs an `Alloc` from a raw allocator.
    pub fn Alloc<Raw:RawAlloc>(r: Raw) -> Alloc<Raw> {
        // implementation hidden; see Non-normative high-level
        // allocator implementation in the appendices
    }

    /// The direct raw allocator implementation of a high-level
    /// allocator.
    ///
    /// * `Direct<R>` dispatches all calls to its underlying raw
    ///   allocator `R`, adding a header when the allocated type
    ///   contains GC roots.
    ///
    /// * `Direct<R>` cannot allocate on the gc-heap.
    ///
    /// * For data not involving the GC, the `Direct<R>` provides
    ///   header-free allocation and a "1:1" call correspondence with
    ///   its given raw alloc `R`.
    ///
    /// * For GC-root carrying data, `Direct<R>` has a "1:1" call
    ///   correspondence with `R`.
    struct Direct<Raw:RawAlloc>(Raw)

    impl<Raw:RawAlloc> AllocCore for Direct<Raw> { ... }

    impl<A:AllocCore> InstanceAlloc for A { ... }

    impl<A:AllocCore> ArrayAlloc for A { ... }

}
```

The design above is meant to provide a great deal of flexibility in
how the GC is implemented in the default system allocator,
while ensuring that when the GC is not involved, there will
be no overhead added by the glue that connects the high-level allocator
to the underlying raw allocator.

As an example of the kind of flexibility the design offers, we point
out a few different design options for GC integration:

* GC root tracking option 1 (block registry): When an allocator
  allocates a block, it is responsible for registering that block in
  the task-local GC state.  The manner of registration is a detail of
  the GC design (it could add a header that includes fields for a
  doubly-linked list that is traversed by the GC; or it could record
  an entry in a bitmap aka pagemap).

* GC root tracking option 2 (allocator registry): When an allocator is
  first used to allocate GC storage, it is added to a task-local
  registry of of GC-enabled allocators.  Then when the GC does root
  scanning, it iterates over the registered allocators, asking each to
  provide the addresses it needs to scan.  Allocators in this design
  need to carry state (to enumerate the roots in address ranges they
  have allocated); they also must remove themselves from the allocator
  registry when they are no longer in use (note that this latter
  requirement seems to imply that such an allocator implement `Drop`,
  which may be too arduous to work with in practice; I have largely
  side-stepped the question of whether allocators should implement the
  `Copy` bound or not).

The point of spelling out the root tracking options above is *not* to
claim that the Rust standrd library will actually support all of these
varieties of GC/allocator integration.  It is rather to justify the
level of abstraction being used in the high-level allocator API: it
provides a great deal of freedom for the future as we research GC
implementation strategies for Rust.

# Drawbacks

A two level API may seem overly engineered, or at least intimidating.
But then again, I do not see much of an alternative without giving up
on GC entirely.

If people really do object to the two-level API, we could side-step it
somewhat by making the `RawAlloc` directly implement the traits in the
`typed_alloc` module; then clients would be able to pass in their
`RawAlloc` instances without using the `typed_alloc::Direct` wrapper.
But I think it is better to include the wrapper, since it opens up the
potential for other future wrapper structs, rather than giving one
specific implementation strategy a syntactic advantage.

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

## RawAlloc variations

Here are collected some variations alterations of the low-level
[`RawAlloc`](#the-rawalloc-trait) API.

### try_realloc

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

Or we could claim that given `RawAlloc` API, between
`usable_capacity_bytes` and its `excess` allocation and methods,
suffices for expected practice.

### ptr parametric `usable_size`

I considered extending `usable_size_bytes` to take the `ptr` itself as an
argument, as another way to handle hypothetical allocators who may choose
different sized bins given the same `(size, align)` input.

But in the end I agreed with the assertion from [RFC PR 39], that in
practice most allocators that override the default implementation will
actually return a constant-expression computed solely from the given
`size` and `align`, and I decided it was simpler to support the above
hypothetical allocators with different size bins via the
`alloc_bytes_excess` and `realloc_bytes_excess` methods.

## High-level allocator variations

### Make `ArrayAlloc` extend `InstanceAlloc`

It might simplify clients to do `trait ArrayAlloc : InstanceAlloc`.

But even then, we would probably want the same set of methods, and
we'd have to specify whether e.g. one is allowed to deallocate, via
`fn dealloc`, a block that had been allocated via `fn alloc_array`.

### Make `AllocCore` non-normative

Many clients would not need the low-level control offered by `AllocCore`.
Arguably `InstanceAlloc` and `ArrayAlloc` might suffice for the high-level
API; I could move `AllocCore` to the non-normative appendix for now.

But I assume that some clients will want to make use of `realloc` on
non-array data; that is my main motivation for leaving `AllocCore`
in the RFC itself.

# Unresolved questions

## Platform supported page size

It is a little ugly that the `RawAlloc` has an error case for an
`align` that is too large, but there is no way in the interface for
the user to ask for that limit.  We could make the limit an associated
constant on `RawAlloc`.  (Note that the [high-level API](#the-typed_alloc-module)
is already relying on `where` clauses, namely in its `from_type` method.)

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

### RFC Pull Request #39: Allocator trait
[RFC PR 39]: https://github.com/rust-lang/rfcs/pull/39/files

Daniel Micay, 2014. RFC: Allocator trait. https://github.com/thestinger/rfcs/blob/ad4cdc2662cc3d29c3ee40ae5abbef599c336c66/active/0000-allocator-trait.md

### Reconsidering custom memory allocation
[ReCustomMalloc]: http://dl.acm.org/citation.cfm?id=582421

Emery D. Berger, Benjamin G. Zorn, and Kathryn S. McKinley. 2002. Reconsidering custom memory allocation. In Proceedings of the 17th ACM SIGPLAN conference on Object-oriented programming, systems, languages, and applications (OOPSLA '02).

### The memory fragmentation problem: solved?
[MemFragSolvedP]: http://dl.acm.org/citation.cfm?id=286864

Mark S. Johnstone and Paul R. Wilson. 1998. The memory fragmentation problem: solved?. In Proceedings of the 1st international symposium on Memory management (ISMM '98).

### EASTL: Electronic Arts Standard Template Library
[EASTL]: http://www.open-std.org/jtc1/sc22/wg21/docs/papers/2007/n2271.html

Paul Pedriana. 2007. [EASTL] -- Electronic Arts Standard Template Library. Document number: N2271=07-0131

### Towards a Better Allocator Model
[Halpern proposal]: http://www.open-std.org/jtc1/sc22/wg21/docs/papers/2005/n1850.pdf

Pablo Halpern. 2005. [Halpern proposal]: Towards a Better Allocator Model. Document number: N1850=05-0110


[jemalloc]: http://www.canonware.com/jemalloc/

[tcmalloc]: http://goog-perftools.sourceforge.net/doc/tcmalloc.html

[Hoard]: http://www.hoard.org/

[tracing garbage collector]: http://en.wikipedia.org/wiki/Tracing_garbage_collection

[malloc/free]: http://en.wikipedia.org/wiki/C_dynamic_memory_allocation

## Glossary

* Size-tracking allocator: An allocator which embeds all allocator
  meta-data such as block size into the allocated block itself, either
  explicitly (e.g. via a header), or implicitly (e.g. by maintaining a
  separate map from address-ranges to sizes).  The C [malloc/free]
  functions form an example of such an API: since the `free` function
  takes only a pointer, the allocator is forced to embed that
  meta-data into the block itself.

* Stateful allocator: An allocator whose particular instances can
  carry local state. (Versions of the C++ standard prior to 2011
  required that to allocators of the same type compare equal.)
  Containers that are parameteric with respect to an allocator are
  expected to carry it as a field (rather than invoking it via static
  methods on the allocator type); this is quite workable in Rust
  without imposing unnecessary overhead since Rust supports zero-sized
  types.

* GC-root carrying data:
  A block of memory that is not allocated on the garbage-collected
  heap (e.g., stack allocated or acquired from the host memory via
  `malloc`), but has fields directly within it of type `Gc<T>` (i.e.
  that point to objects allocated on the garbage-collected heap).

  Some examples: When not GC-heap allocated, `HasDirectRoots` is
  GC-root carrying data:

```rust
struct HasDirectRoots {
    some_int: Gc<int>,
    some_array: Gc<[int]>,
}
```

  For comparison, `OnlyIndirectRoots` is *not* GC-root carrying data:

```rust
struct OnlyIndirectRoots {
    some_int: Box<Gc<int>>,
    some_array: Box<Gc<[int]>>,
}
```

The `OnlyIndirectRoots` struct does have fields that *point* to blocks
of GC-root carrying data, but `OnlyIndirectRoots` itself does not
itself contain direct pointers into the GC-heap.  This is important,
because it means that as long as the memory blocks for the two `Box`
instances are always scanned for roots, the root scanner does *not*
need to ever scan instances of `OnlyIndirectRoots`.

## Non-normative high-level allocator implementation

The high-level allocator traits in [the-typed_alloc-module] are just
API surface; the description above does not specify the manner in
which one *implements* instances of these traits.

Here follows a sketch of how the `typed_alloc` traits might be
implemented for the `Alloc` and `Direct` structs atop the `RawAlloc`
interface, with GC hooks included as needed (but optimized away when
the type does not involve GC).

(Much of this design was contributed by Niko Matsakis. Niko deserves
credit for the insights, and much of the text was directly
cut-and-pasted from text he authored. Nonetheless, Felix takes
responsibility for any erroneous reasoning below.)

This sketch assumes that the Rust has been extended with type-level
compile-time intrinsic functions to indicate whether allocation of a
given type involves coordination with the garbage collector.

These compile-time intrinsic functions for GC introspection are:

  * `type_is_gc_allocated::<T>()`: indicates whether `T` is itself
    allocated on the garbage-collected heap (e.g. `T` is some
    `GcBox<_>`, where `Gc<_>` holds a reference to a `GcBox<_>`),

  * `type_reaches_gc::<T>()`: indicates if the memory block for `T`
    needs to be treated as holding GC roots (e.g. `T` contains some
    `Gc<_>` within its immediate contents), and

  * `type_invoves_gc::<T>()`: a short-hand for
    `type_is_gc_allocated::<T>() || type_reaches_gc::<T>()`

In addition, we need some primitive operations for converting from raw
byte pointers to fat-pointers to dynamically sized types (DSTs) and
back again.

The `MemoryBlockInfo<T>` struct below carries the necessary word of
data that is attached to fat pointers, in the `unsized_metadata`
field.

If you have an actual pointer `&U`, you can extract its pointer data
using the intrinsic `pointer_data`, which takes a (possibly fat)
pointer and returns the pointer data associated with it. The interface
actually takes 

    // Given fat (or thin) pointer of type U1, extract data suitable
    // for `U2`. The types U1 and U2 must require the same sort of
    // meta data: in particular U2 may be a struct type (or tuple type etc)
    // whose field field is of the type U1.
    /*intrinsic*/ fn pointer_data<Sized? U1, Sized? U2>(x: &U1) -> PointerData<U2>;

There is also the `pointer_mem` intrinsic, which extracts out the raw
thin pointer. It also returns a `*u8`.

    // Given fat (or thin) pointer, extract the memory
    /*intrinsic*/ fn pointer_mem<Sized? U>(x: &U) -> *mut u8;


Next, we have the intrinsics for computing the size and alignment
requirements of a type. Note that these operator over any type, sized
of unsized, but a pointer data is required:

    /*intrinsic*/ fn sizeof_type<Sized? U>(data: PointerData<U>) -> uint;
    /*intrinsic*/ fn alignof_type<Sized? U>(data: PointerData<U>) -> uint;
    
Finally, we have an intrinsic to make a pointer. If `U` is a sized
type, this is the same as a transmute, but if `U` is unsized, it will
pair up `p` with the given pointer data (note that pointer data is
always POD).
    
    // Make a fat (or thin) pointer from some memory
    /*intrinsic*/ fn make_pointer<Sized? U>(p: *mut u8,
                                            data: PointerData<U>)
                                            -> *mut U;

Based on the above intrinsics you can build the following user-facing
functions for computing sizes:

    fn sizeof<T>() -> uint {
        sizeof_type(thin_pointer_data::<T>())
    }

    fn sizeof_value<Sized? U>(x: &U) -> uint {
        let data = pointer_data::<U,U>(x);
        sizeof_type<U>(data)
    }

Note: intrinsics are needed because the appropriate code will depend
on whether `U` winds up being sized or not. We might be able to get
away with one core intrinsic `ty_is_dst()` or something like that.
This is relatively minor detail: the same helper functions will exist
either way.

```rust
mod typed_alloc_impl {
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

    impl<A:AllocCore> InstanceAlloc for A {
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

        unsafe fn alloc_array_excess<T>(&self, capacity: uint) -> (*mut T, uint) {
            self.alloc_info_excess(MemoryBlockInfo::<T>::array(capacity))
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

    impl<Raw:RawAlloc> Alloc {
        unsafe fn alloc_info_gc<Sized? T>(&self, info: MemoryBlockInfo<T>) -> *mut T { ... }
        unsafe fn realloc_info_gc<Sized? T, Sized? U>(&self, old_ptr: *mut T, info: MemoryBlockInfo<U>) -> *mut U { ... }
        unsafe fn dealloc_info_gc<Sized? T>(&self, pointer: &mut T, info: MemoryBlockInfo<T>) { ... }
    }

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

}
```
