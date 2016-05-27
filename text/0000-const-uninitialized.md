- Feature Name: const_uninitialized
- Start Date: 2016-05-27
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Turn `std::mem::uninitialized()` and `std::mem::zeroed()` into `const fn` so
that they can be used to initialize `static` variables.

# Motivation
[motivation]: #motivation

The main motivation for this is to allow static variables to be initialized to
and undefined value which is guaranteed to be overwritten later on. It is mainly
intended to be used in macros like `lazy_static!` and `thread_local!` rather
than invoked by users directly. This would allow these macros to avoid the need
to wrap values in an `Option` which can be statically initialized as `None`.

Here is an example implementation of `lazy_static!` that uses this feature. Note
that the uninitialized data is never read because it is guarded by a `Once`.
The code is based off [this file](https://github.com/rust-lang-nursery/lazy-static.rs/blob/5c70ffba0c135c4f18fd139fdceb539bdf3cda3f/src/nightly_lazy.rs).

```rust
pub struct Lazy<T: Sync>(UnsafeCell<T>, Once);

impl<T: Sync> Lazy<T> {
    #[inline(always)]
    pub const fn new() -> Self {
        Lazy(UnsafeCell::new(unsafe { mem::uninitialized() }), ONCE_INIT)
    }

    #[inline(always)]
    pub fn get<F>(&'static self, f: F) -> &T
        where F: FnOnce() -> T
    {
        unsafe {
            self.1.call_once(|| {
                unsafe {
                    ptr::write(self.0.get(), f());
                }
            });

            &*self.0.get()
        }
    }
}

unsafe impl<T: Sync> Sync for Lazy<T> {}

#[macro_export]
#[allow_internal_unstable]
macro_rules! __lazy_static_create {
    ($NAME:ident, $T:ty) => {
        static $NAME: $crate::lazy::Lazy<$T> = $crate::lazy::Lazy::new();
    }
}
```

# Detailed design
[design]: #detailed-design

These functions:

- `std::mem::uninitialized()`
- `std::mem::zeroed()`

... and these intrinsics:

- `std::intrinsics::uninit()`
- `std::intrinsics::init()`

... will become `const fn`. This will require extra work in the early stages of
the compiler to recognize these intrinsics and handle them as constants.

# Drawbacks
[drawbacks]: #drawbacks

The compiler will have to deal with invalid values properly during constant
evaluation. For example:

```rust
static X: Option<&T> = unsafe { Some(std::mem::zeroed()) };
```

# Alternatives
[alternatives]: #alternatives

Currently all `static` variables that require dynamic initialization have to be
wrapped inside an `Option` and initialized with `None`. While this is safe, it
is less efficient that allowing the type to be used directly.

# Unresolved questions
[unresolved]: #unresolved-questions

How will the compiler deal with invalid values in constants?
