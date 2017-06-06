- Start Date: 2014-06-17
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Add support for the `llvm.prefetch` intrinsic, via a number of rust intrinsics.

# Motivation

Prefetch can speed up program execution in some cases.

# Detailed design

Add a number of intrinsics representing the possible constant parameters of 
`void @llvm.prefetch(i8* <address>, i32 <rw>, i32 <locality>, i32 <cache type>)`, 
their type being `fn<T>(*T)` for reads, `fn<T>(*mut T)` for writes.

the proposed intrinsics:

```rust
/// Prefetch the pointer `address` for read to the data cache with maximum locality
pub fn prefetch<T>(address: *T);

/// Prefetch the pointer `address` for write to the data cache with maximum locality
pub fn prefetch_write<T>(address: *mut T);

/// Prefetch the pointer `address` for read to the data cache with high locality
pub fn prefetch_high<T>(address: *T);

/// Prefetch the pointer `address` for write to the data cache with high locality
pub fn prefetch_write_high<T>(address: *mut T);

/// Prefetch the pointer `address` for read to the data cache with low locality
pub fn prefetch_low<T>(address: *T);

/// Prefetch the pointer `address` for write to the data cache with low locality
pub fn prefetch_write_low<T>(address: *mut T);

/// Prefetch the pointer `address` for read to the data cache with no locality
pub fn prefetch_none<T>(address: *T);

/// Prefetch the pointer `address` for write to the data cache with no locality
pub fn prefetch_write_none<T>(address: *mut T);

/// Prefetch the pointer `address` for read to the instruction cache with maximum locality
pub fn prefetch_icache<T>(address: *T);

/// Prefetch the pointer `address` for write to the instruction cache with maximum locality
pub fn prefetch_write_icache<T>(address: *mut T);

/// Prefetch the pointer `address` for read to the instruction cache with high locality
pub fn prefetch_high_icache<T>(address: *T);

/// Prefetch the pointer `address` for write to the instruction cache with high locality
pub fn prefetch_write_high_icache<T>(address: *mut T);

/// Prefetch the pointer `address` for read to the instruction cache with low locality
pub fn prefetch_low_icache<T>(address: *T);

/// Prefetch the pointer `address` for write to the instruction cache with low locality
pub fn prefetch_write_low_icache<T>(address: *mut T);

/// Prefetch the pointer `address` for read to the instruction cache with no locality
pub fn prefetch_none_icache<T>(address: *T);

/// Prefetch the pointer `address` for write to the instruction cache with no locality
pub fn prefetch_write_none_icache<T>(address: *mut T);
```

# Drawbacks

I can't really see any.

# Alternatives

It could be implemented in a single intrinsic that somehow makes sure it supplies constant values for llvm, this seemed to be much more complicated.

# Unresolved questions

The instruction cache prefetches could have some different type, maybe?
