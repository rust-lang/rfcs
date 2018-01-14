- Feature Name: mem_zero
- Start Date: 2018-01-14
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

This RFC proposes to add a `mem::zero` function to `core` and transitively also
to `std`. The purpose of this function is to zero out all bytes of a value
pointed to by a mutable reference. This would be the in-place alternative or
complement of `mem::zeroed`.

# Motivation
[motivation]: #motivation

Currently there is no simple way to zero out unsized data. `mem::zeroed`, by its
nature of returning a value, requires the generic type to be `Sized`. This would
allow for (re-)initializing data whose size is unknown at compile-time.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

One use case is to clear (zero out) a slice of bytes. This is a safe operation
and it should be done efficiently. One may make a call to `mem::zero` to
facilitate this operation.

```rust
use std::mem;

fn clear_bytes(slice: &mut [u8]) {
    unsafe { mem::zero(slice) }
}
```

Notice the `unsafe` block around the call site. Not all types have a valid state
of all zeroes. Just like `mem::zeroed`, this function must be used with caution.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This implementation of this function is very straightforward:

```rust
use std::{mem, ptr};

pub unsafe fn zero<T: ?Sized>(val: &mut T) {
    let len = mem::size_of_val(val);
    let ptr = val as *mut T as *mut u8;
    ptr::write_bytes(ptr, 0, len);
}
```

Because this uses a compiler intrinsic, the emitted instructions may vary
depending on the size of `T`. This same behavior is exhibited by `mem::zeroed`.

Here `T` is optionally `Sized`. As a result, this function will work with both
slices and trait objects, which `mem::zeroed` cannot.

# Drawbacks
[drawbacks]: #drawbacks

- It adds yet another function to the standard library.

- It may be accidentally misused in cases such as `&mut &mut T`. If we pass such
a pointer in without first dereferencing it, then we end up with a reference to
a null pointer.

# Rationale and alternatives
[alternatives]: #alternatives

This function provides a simple interface to an operation that would require a
bit of boilerplate.

The alternative is to do nothing and have people keep their boilerplate code.

# Unresolved questions
[unresolved]: #unresolved-questions

What to do about the second drawback (`&mut &mut T`).
