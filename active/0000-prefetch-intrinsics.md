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
their type being `fn(*i8)` for reads, `fn(*mut i8)` for writes.
They should be named like `prefetch` for a read, extreme locality, 
data cache, `prefetch_write` for a write with a same parameters, 
`prefetch_none` for a read, no locality, data cache prefetch,
`prefetch_write_low_icache` for a write, low locality, instruction cache one.

# Drawbacks

I can't really see any.

# Alternatives

It could be implemented in a single intrinsic that somehow makes sure it supplies constant values for llvm, this seemed to be much more complicated.

# Unresolved questions

The instruction cache prefetches could have some different type.
