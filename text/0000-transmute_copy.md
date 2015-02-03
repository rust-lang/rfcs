- Start Date: 2015-02-03
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Make `transmute_copy<T,U>` with `sizeof T != sizeof U` a compile time error.

# Motivation

Given the name, it's reasonable to assume that `transmute_copy` is just a
`transmute` that doesn't consume the value. However, `transmute_copy` does not
check at compile time that the sizes match and if `sizeof U > sizeof T`, the
behavior might be undefined at runtime.

# Detailed design

Change the implementation of `transmute_copy` to

```rust
pub unsafe fn transmute_copy<T, U>(src: &T) -> U {
    transmute(ptr::read(src as *const T))
}
```

# Drawbacks

Sometimes one wants to read a prefix of `T`, i.e., `sizeof U < sizeof T`. This
is still possible via `ptr::read(src as *const T as *const U)`.
