- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

`*mut T` and `&mut T` should coerce to `*const T`.

# Motivation

Precedence:

- `&mut T` coerces to `&T`
- `&T` coerces to `*const T`
- `&mut T` coerces to `*mut T`

Converting from `*mut` to `*const` is a safe operation which happens regularly in FFI code:

```c
T *new(void);
void non_mutating_operation(T const *);
void mutating_operation(T *);
```

```rust
struct TWrapper {
    raw: *mut T,
}

impl TWrapper {
    fn non_mutating_operation(&self) {
        unsafe { non_mutating_operation(self.raw as *const T); }
    }

    fn mutating_operation(&self) {
        unsafe { mutating_operation(self.raw); }
    }
}
```

# Drawbacks

None right now.

# Detailed design

Add these rules:

- `*mut T` coerces to `*const T`
- `&mut T` coerces to `*const T`

So that the following code compiles:

```rust
let a = &mut 1u;
let b: *mut uint = a;

let x: *const uint = a;
let y: *const uint = b;
```

# Alternatives

Leave it the way it is.

# Unresolved questions

None right now.
