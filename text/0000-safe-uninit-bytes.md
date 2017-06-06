- Feature Name: safe_uninit_bytes
- Start Date: 2015-07-21
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Expose an std function for marking potentially uninitialized bytes as
initialized.

# Motivation

This would make it possible to safely define (i.e. the behavior would both be
safe and defined):

```rust
impl Vec<u8> {
    pub fn grow_uninitialized(&mut self, amount: usize) {
        unsafe {
        let len = self.len();
        self.reserve(amount);
        mem::assume_initialized(self.as_mut_ptr().offset(len), amount);
        self.set_len(len + amount);
    }
}
```

And would make it much easier to implement fast IO code.

# Detailed Design

Specifically, this proposal recommends that the following function be added to
the standard library along with an associated intrinsic for telling LLVM that a
section of memory should be considered initialized.

```rust
mod mem {
    unsafe fn assume_initialized(ptr: *mut u8, size: usize);
    // ...
}
```

This function takes a pointer and a size to avoid introducing uninitialized but
safely dereferencable types.

# Why

Every addressable byte in allocated memory is a valid u8 (byte) by definition.
On Linux at least, one can (usually) read `/proc/self/mem` into a buffer so the
following two functions are (virtually) indistinguishable at runtime (on Linux):

```rust
use std::fs::File;
use std::io::prelude::*;
use std::io::SeekFrom;

fn fake_uninitialized() -> Vec<u8> {
    let mut v = Vec::new();
    v.reserve(100);

    let mut tmp_buf = vec![0; 100];
    let mut mem = File::open("/proc/self/mem").unwrap();
    mem.seek(SeekFrom::Start(v.as_ptr() as u64)).unwrap();
    assert!(mem.read(&mut tmp_buf[..]).unwrap() == 100);
    v.extend(tmp_buf);
    v
}

fn real_uninitialized() -> Vec<u8> {
    let mut v = Vec::new();
    v.reserve(100);
    // tell LLVM that v should be considered initialized.
    v.set_len(100);
    v
}
```

Given that this is already possible to do without writing any unsafe code,
there's no reason not to make it safe to do efficiently (i.e. replace all
instances of `fake_uninitialized` with `real_uninitialized`).

# Drawbacks

None that I can think of.

# Alternatives

Specialize (or override trait defaults) where needed. This leads to more code
and larger binaries.

# Unresolved questions

The constraints in this proposal could be relaxed significantly. However, the
less constrained this proposal is, the more constrained rustc becomes so I'd
prefer to keep this proposal minimal. Although, it might be useful to allow
uninitialized `[u*]` and `[i*]`.
