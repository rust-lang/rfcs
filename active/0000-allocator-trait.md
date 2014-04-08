- Start Date: 2014-04-07
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Rust is in need of a trait to generalize low-level memory allocators. This will enable a pluggable
default allocator, along with the possibility of supporting per-container allocators with the option
of statefulness.

# Motivation

Modern general purpose allocators are quite good, but need to make many design tradeoffs. There is
no "best" global allocator so it should be a configurable feature of the standard library. In order
for this to happen, an allocator interface needs to be defined.

Some applications may also have a use case for cache aligned nodes with concurrent data structures
to avoid contention, or very fast naive allocators (like a bump allocator) shared between data
structures with related lifetimes.

The basic `malloc`, `realloc` and `free` interface is quite lacking, since it's missing alignment
and the ability to obtain an allocation's size. In order to have good support an alignment
specification on types, it needs to be possible for types like `Vec<T>` to ask the allocator for an
alignment.  This can be done inefficiently by building a wrapper around the `malloc` API, but many
allocators have efficient support for this need built-in.

# Detailed design

Trait design:

```rust
pub trait Allocator {
    /// Return a pointer to `size` bytes of memory.
    ///
    /// A null pointer may be returned if the allocation fails.
    ///
    /// Behavior is undefined if the requested size is 0 or the alignment is not a power of 2. The
    /// alignment must be no larger than the largest supported page size on the platform.
    unsafe fn alloc(&self, size: uint, align: u32) -> *mut u8;

    /// Extend or shrink the allocation referenced by `ptr` to `size` bytes of memory.
    ///
    /// A null pointer may be returned if the allocation fails and the original memory allocation
    /// will not be altered.
    ///
    /// Behavior is undefined if the requested size is 0 or the alignment is not a power of 2. The
    /// alignment must be no larger than the largest supported page size on the platform.
    ///
    /// The `old_size` and `align` parameters are the parameters that were used to create the
    /// allocation referenced by `ptr`. The `old_size` parameter may also be the value returned by
    /// `usable_size` for the requested size.
    unsafe fn realloc(&self, ptr: *mut u8, size: uint, align: u32, old_size: uint) -> *mut u8;

    /// Deallocate the memory referenced by `ptr`.
    ///
    /// The `ptr` parameter must not be null.
    ///
    /// The `size` and `align` parameters are the parameters that were used to create the
    /// allocation referenced by `ptr`. The `size` parameter may also be the value returned by
    /// `usable_size` for the requested size.
    unsafe fn dealloc(&self, ptr: *mut u8, size: uint, align: u32);

    /// Return the usable size of an allocation created with the specified the `size` and `align`.
    #[inline(always)]
    #[allow(unused_variable)]
    unsafe fn usable_size(&self, size: uint, align: u32) -> uint { size }
}
```

The trait takes an `align` parameter alongside every `size` parameter in order to support large
alignments without overhead for all allocations. For example, aligned AVX/AVX-512 vectors,
cache-aligned memory for concurrent data structures or page-aligned memory for working with
hardware.

The `usable_size` method provides a way of asking for the real size/alignment the allocator will
produce for a specific request. While it is possible for the `alloc` and `realloc` methods to
return this dynamically, it adds complexity and does not seem to have a use case. There is no
reason for it to return an alignment, since the dynamic alignment information is trivially
obtainable from any pointer.

The `realloc` and `dealloc` methods require passing in the size and alignment of the existing memory
allocation. The alignment should be a known constant, so this will not place a burden on the caller.
The *guarantee* of having a size permits much more optimization than simply *sometimes* being passed
a size. It allows simple allocators to forgo storing a size altogether. For example, this permits an
implementation of a free list for variable size types without metadata overhead. C++ allocators use
this design, with the `deallocate` method always taking a size parameter.

It is left up to the caller to choose how to handle zero-size allocations, such as the current
wrapping done by the `rt::global_heap` allocator. Allocators like jemalloc do not provide a
guarantee here, and some callers may want a null pointer while others will want a non-null
sentinel pointing at a global.

The alignment is given a reasonable restriction, by capping it at the largest huge page size on
the system. It should never be dynamic, so this is easily satisfiable. This is to meet the
requirement of allocators like jemalloc for the alignment to be satisfiable without overflow while
still meeting every use case. It is given as 32-bit because LLVM uses a 32-bit integer for
alignment.

Sample default allocator:

```rust
extern crate libc;

use std::intrinsics::cttz32;
use libc::{c_int, c_void, size_t};

#[link(name = "jemalloc")]
extern {
    fn nallocx(size: size_t, flags: c_int) -> size_t;
    fn mallocx(size: size_t, flags: c_int) -> *mut c_void;
    fn rallocx(ptr: *mut c_void, size: size_t, flags: c_int) -> *mut c_void;
    fn dallocx(ptr: *mut c_void, flags: c_int);
}

pub struct DefaultAllocator;

// MALLOCX_ALIGN(a) macro
fn mallocx_align(a: u32) -> c_int { unsafe { cttz32(a as i32) as c_int } }

impl Allocator for DefaultAllocator {
    unsafe fn alloc(&self, size: uint, align: u32) -> *mut u8 {
        mallocx(size as size_t, mallocx_align(align)) as *mut u8
    }

    unsafe fn realloc(&self, ptr: *mut u8, size: uint, align: u32, _: uint) -> *mut u8 {
        rallocx(ptr as *mut c_void, size as size_t, mallocx_align(align)) as *mut u8
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _: uint, align: u32) {
        dallocx(ptr as *mut c_void, mallocx_align(align))
    }

    #[inline(always)]
    unsafe fn usable_size(&self, size: uint, align: u32) -> uint {
        nallocx(size as size_t, mallocx_align(align)) as uint
    }
}

pub static default: DefaultAllocator = DefaultAllocator;
```

# Alternatives

## Zeroed memory

The `Allocator` trait does not provide a way to ask for zeroed memory.  Allocators based on
mmap/mremap already pay this cost for large allocations but the tide is likely going to shift to
new underlying APIs like the Linux vrange work. This optimization (`calloc`) is not currently
used by anything in Rust, as it's quite hard to fit it into any generic code.

The `alloc` and `realloc` functions could take a `zero: bool` parameter for leveraging the guarantee
provided by functions like `mmap` and `mremap`. It would be possible to add support for this in an
almost completely backwards compatible way by adding two new default methods with the parameter.

## Sized reallocation/deallocation

The old size passed to `realloc` and `dealloc` is an optional performance enhancement. There is some
debate about whether this is worth having. I have included it because there is no drawback within
the current language and standard libraries, so it's an obvious performance enhancement.

In a future version of Rust, `~[T]` will be an owned slice stored as `(ptr, len)`. Converting from
an owned slice to a `Vec<T>` will be a no-op. However, conversion from `Vec<T>` to `~[T]` can not be
free due to the Option-like `enum` optimization for non-nullable pointers. There is a choice between
calling `free` on zero-size allocations during the conversion, or branching in every `~[T]`
destructor based on a comparison with the reserved sentinel address. If `dealloc` requires a size, a
`shrink_to_fit()` call will be the minimum requirement.

The niche for `~[T]` is not yet known, so it would be premature to sacrifice performance relative to
C++ allocators to optimize one aspect of it. A vector should be left as `Vec<T>` to avoid losing
track of excess capacity, and except in recursive data structures there is little cost for the
capacity field.

## Alignment parameter

This parameter is required to be a power of 2, so it could take `log2(alignment)` instead. Either
way, the standard library is going to need to expose a convenience function for retrieving this for
a specific type.

# Unresolved questions

The finer details of the API for allocator support in containers is beyond the scope of this RFC. It
only aims to define the `Allocator` trait for making Rust's default allocator configurable and
building the foundation for containers. A sample design would be `Vec<T, A = DefaultAllocator>` with
extra static methods taking an *instance* of the allocator type in order to support stateful
allocators.
