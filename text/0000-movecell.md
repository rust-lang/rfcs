- Feature Name: movecell
- Start Date: 2016-06-27
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Introduce a new type `MoveCell<T>` in `std::cell`.

# Motivation
[motivation]: #motivation

The type `Cell<T>` is only usable with `Copy` types to be able to safely
implement its `get` and `set` methods. To implement interior mutability
for non-`Copy` types, the type `RefCell<T>` must be used but it comes with
runtime overhead with its reference counting.

This new type `MoveCell<T>` aims to cover the use case where the user just needs
to store a non-`Copy` value elsewhere, through a single operation `replace`
which replaces the cell value and returns the older one.

# Detailed design
[design]: #detailed-design

The core of this new cell just consists of two methods:

```rust
pub struct MoveCell<T>(UnsafeCell<T>);

impl<T> MoveCell<T> {
    pub fn new(value: T) -> Self {
        MoveCell(UnsafeCell::new(value))
    }

    pub fn replace(&self, value: T) -> T {
        mem::replace(unsafe { &mut *self.0.get() }, value)
    }
}
```

We could in the future add various convenience functions,
based on real-world usage:

```rust
impl<T> MoveCell<T> {
    /// A `set` method that just drops the value returned by `replace`.
    pub fn set(&self, value: T) {
        drop(self.replace(value));
    }
}

impl<T: Default> MoveCell<T> {
    /// A `take` method replacing by the `Default` value for `T`. This coincides
    /// with `Option::take::<T>` for `Option<T>` cells.
    pub fn take(&self) -> T {
        self.replace(T::default())
    }
}
```

# Drawbacks
[drawbacks]: #drawbacks

There are no drawbacks.

# Alternatives
[alternatives]: #alternatives

@Amanieu [proposed](https://github.com/rust-lang/rfcs/pull/1651) that the
`Cell<T>` type be extended instead.

# Unresolved questions
[unresolved]: #unresolved-questions

Should convenience methods be included from start? Or should they wait users
to first use that new cell type?
