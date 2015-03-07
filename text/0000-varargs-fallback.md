- Feature Name: varargs-fallback
- Start Date: Sat Mar  7 07:56:02 CET 2015
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Don't allow numeric types with fallback-type in varargs position.

# Motivation

Rust allows extern varargs functions like this:

```rust
extern {
    fn printf(fmt: *const c_char, ...);
}
```

These functions can then be used as they would be used in C:

```rust
unsafe {
    let x: c_int = 1;
    printf("value: %d\n\0".as_ptr() as *const _, x);
}
```

However, the following usage is also allowed:

```rust
unsafe {
    printf("value: %d\n\0".as_ptr() as *const _, 1);
}
```

Note that the numeric argument doesn't have an inferable type and therefore it
falls back to the integer-fallback type `i32`. In C such a literal would have
type `c_int` which coincides with `i32` on all supported platforms. However,
this might not always be the case.

Using the wrong type in varargs position can cause memory unsafety.

# Detailed design

Let the expression `X` refer to a numeric variable or literal without an
inferred type. Then `X` cannot be used in varargs position.

# Drawbacks

No serious ones.

# Alternatives

None right now.

# Unresolved questions

None right now.
