- Start Date: 2014-04-29
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Allow the use of some intrinsics in static evaluation context, specifically
`init`, `uninit` and `size_of`.

# Motivation

Intrinsics provide valuable details or initialization for statics, that are much
more widely used in embedded. While all the operations are unsafe by nature,
the careful use by programmer should be allowed.

# Detailed design

Most of the use cases are related to instantiation of a struct in a pre-defined
memory location. This might or might not be solved by any custom allocators.

Example 1: initialize an object in `.bss`:

```rust
static mut inst: T = unsafe { uninit() };
```

this would allocate the required chunk in .bss, the access to the `inst` is
undefined, until it's properly initialized.

Example 2: initialize a buffer for objects:

```rust
static mut inst_buf: [u8, ..size_of::<T>() * 4] = [0, ..size_of::<T>() * 4];
```

this would allocate the chuck sufficient to hold 4 `T`s in .bss. This case
doesn't solve the instantiation, though, `memcpy` might be used for copying an
object from stack to `inst_buf`, but that's a really unsafe option.


# Alternatives

Provide more management for `~T`s, so that the actual heap / location in heap
could be specified. Anything that might be allocated in heap must be strictly
verified on case by case basis.

# Unresolved questions

Is it `static mut inst_buf: [u8, ..size_of::<T>() * 4]` or
`static mut inst_buf: [T, ..4]`? If we can use `init`/`uninit`, the second
option is acceptable.
