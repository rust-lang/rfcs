- Feature Name: thread_local_cell_methods
- Start Date: 2021-10-17
- RFC PR: [rust-lang/rfcs#3184](https://github.com/rust-lang/rfcs/pull/3184)
- Rust Issue: [rust-lang/rust#92122](https://github.com/rust-lang/rust/issues/92122)

# Summary
[summary]: #summary

Adding methods to `LocalKey` for `LocalKey<Cell<T>>` and `LocalKey<RefCell<T>>` to make thread local cells easier to use.

# Motivation
[motivation]: #motivation

Almost all real-world usages of `thread_local! {}` involve a `Cell` or `RefCell`.
Using the resulting `LocalKey` from a `thread_local! {}` declaration gets verbose due to having to use `.with(|_| ..)`.
(For context: `.with()` is necessary because there's no correct lifetime for the thread local value.
This method makes sure that any borrows end before the thread ends.)

```rust
thread_local! {
    static THINGS: RefCell<Vec<i32>> = RefCell::new(Vec::new());
}

fn f() {
    THINGS.with(|things| things.borrow_mut().push(1));

    // ...

    THINGS.with(|things| {
        let things = things.borrow();
        println!("{:?}", things);
    });
}
```

In addition, using `.set()` on a thread local cell through `.with()` results in unnecessary initialization,
since `.with` will trigger the lazy initialization, even though `.set()` will overwrite the value directly afterwards:

```rust
thread_local! {
    static ID: Cell<usize> = Cell::new(generate_id());
}

fn f() {
    ID.with(|id| id.set(1)); // Ends up calling generate_id() the first time, while ignoring its result.

    // ...
}
```

# Proposed additions

We add `.set()`, `.get()`\*, `.take()` and `.replace()` on `LocalKey<Cell<T>>` and `LocalKey<RefCell<T>>` such that they can used directly without using `.with()`:

(\* `.get()` only for `Cell`, not for `RefCell`.)

```rust
thread_local! {
    static THINGS: RefCell<Vec<i32>> = RefCell::new(Vec::new());
}

fn f() {
    THINGS.set(vec![1, 2, 3]);

    // ...

    let v: Vec<i32> = THINGS.take();
}
```

For `.set()`, this *skips the initialization expression*:

```rust
thread_local! {
    static ID: Cell<usize> = panic!("This thread doesn't have an ID yet!");
}

fn f() {
    // ID.with(|id| ..) at this point would panic.

    ID.set(123); // This does *not* result in a panic.
}
```

In addition, we add `.with_ref` and `.with_mut` for `LocalKey<RefCell<T>>` to do `.with()` and `.borrow()` or `.borrow_mut()` at once:

```rust
thread_local! {
    static THINGS: RefCell<Vec<i32>> = RefCell::new(Vec::new());
}

fn f() {
    THINGS.with_mut(|v| v.push(1));

    // ...

    let len = THINGS.with_ref(|v| v.len());
}
```

# Full reference of the proposed additions

```rust
impl<T: 'static> LocalKey<Cell<T>> {
    /// Sets or initializes the contained value.
    ///
    /// Unlike the other methods, this will *not* run the lazy initializer of
    /// the thread local. Instead, it will be directly initialized with the
    /// given value if it wasn't initialized yet.
    ///
    /// # Panics
    ///
    /// Panics if the key currently has its destructor running,
    /// and it **may** panic if the destructor has previously been run for this thread.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::cell::Cell;
    ///
    /// thread_local! {
    ///     static X: Cell<i32> = panic!("!");
    /// }
    ///
    /// // Calling X.get() here would result in a panic.
    ///
    /// X.set(123); // But X.set() is fine, as it skips the initializer above.
    ///
    /// assert_eq!(X.get(), 123);
    /// ```
    pub fn set(&'static self, value: T);

    /// Returns a copy of the contained value.
    ///
    /// This will lazily initialize the value if this thread has not referenced
    /// this key yet.
    ///
    /// # Panics
    ///
    /// Panics if the key currently has its destructor running,
    /// and it **may** panic if the destructor has previously been run for this thread.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::cell::Cell;
    ///
    /// thread_local! {
    ///     static X: Cell<i32> = Cell::new(1);
    /// }
    ///
    /// assert_eq!(X.get(), 1);
    /// ```
    pub fn get(&'static self) -> T where T: Copy;

    /// Takes the contained value, leaving `Default::default()` in its place.
    ///
    /// This will lazily initialize the value if this thread has not referenced
    /// this key yet.
    ///
    /// # Panics
    ///
    /// Panics if the key currently has its destructor running,
    /// and it **may** panic if the destructor has previously been run for this thread.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::cell::Cell;
    ///
    /// thread_local! {
    ///     static X: Cell<Option<i32>> = Cell::new(Some(1));
    /// }
    ///
    /// assert_eq!(X.take(), Some(1));
    /// assert_eq!(X.take(), None);
    /// ```
    pub fn take(&'static self) -> T where T: Default;

    /// Replaces the contained value, returning the old value.
    ///
    /// This will lazily initialize the value if this thread has not referenced
    /// this key yet.
    ///
    /// # Panics
    ///
    /// Panics if the key currently has its destructor running,
    /// and it **may** panic if the destructor has previously been run for this thread.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::cell::Cell;
    ///
    /// thread_local! {
    ///     static X: Cell<i32> = Cell::new(1);
    /// }
    ///
    /// assert_eq!(X.replace(2), 1);
    /// assert_eq!(X.replace(3), 2);
    /// ```
    pub fn replace(&'static self, value: T) -> T;
}
```

```rust
impl<T: 'static> LocalKey<RefCell<T>> {
    /// Acquires a reference to the contained value.
    ///
    /// This will lazily initialize the value if this thread has not referenced
    /// this key yet.
    ///
    /// # Panics
    ///
    /// Panics if the value is currently borrowed.
    ///
    /// Panics if the key currently has its destructor running,
    /// and it **may** panic if the destructor has previously been run for this thread.
    ///
    /// # Example
    ///
    /// ```
    /// use std::cell::RefCell;
    ///
    /// thread_local! {
    ///     static X: RefCell<Vec<i32>> = RefCell::new(Vec::new());
    /// }
    ///
    /// X.with_ref(|v| assert!(v.is_empty()));
    /// ```
    pub fn with_ref<F, R>(&'static self, f: F) -> R where F: FnOnce(&T) -> R;

    /// Acquires a mutable reference to the contained value.
    ///
    /// This will lazily initialize the value if this thread has not referenced
    /// this key yet.
    ///
    /// # Panics
    ///
    /// Panics if the value is currently borrowed.
    ///
    /// Panics if the key currently has its destructor running,
    /// and it **may** panic if the destructor has previously been run for this thread.
    ///
    /// # Example
    ///
    /// ```
    /// use std::cell::RefCell;
    ///
    /// thread_local! {
    ///     static X: RefCell<Vec<i32>> = RefCell::new(Vec::new());
    /// }
    ///
    /// X.with_mut(|v| v.push(1));
    ///
    /// X.with_ref(|v| assert_eq!(*v, vec![1]));
    /// ```
    pub fn with_mut<F, R>(&'static self, f: F) -> R where F: FnOnce(&mut T) -> R;

    /// Sets or initializes the contained value.
    ///
    /// Unlike the other methods, this will *not* run the lazy initializer of
    /// the thread local. Instead, it will be directly initialized with the
    /// given value if it wasn't initialized yet.
    ///
    /// # Panics
    ///
    /// Panics if the key currently has its destructor running,
    /// and it **may** panic if the destructor has previously been run for this thread.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::cell::RefCell;
    ///
    /// thread_local! {
    ///     static X: RefCell<Vec<i32>> = panic!("!");
    /// }
    ///
    /// // Calling X.with() here would result in a panic.
    ///
    /// X.set(vec![1, 2, 3]); // But X.set() is fine, as it skips the initializer above.
    ///
    /// X.with_ref(|v| assert_eq!(*v, vec![1, 2, 3]));
    /// ```
    pub fn set(&'static self, value: T);

    /// Takes the contained value, leaving `Default::default()` in its place.
    ///
    /// This will lazily initialize the value if this thread has not referenced
    /// this key yet.
    ///
    /// # Panics
    ///
    /// Panics if the value is currently borrowed.
    ///
    /// Panics if the key currently has its destructor running,
    /// and it **may** panic if the destructor has previously been run for this thread.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::cell::RefCell;
    ///
    /// thread_local! {
    ///     static X: RefCell<Vec<i32>> = RefCell::new(Vec::new());
    /// }
    ///
    /// X.with_mut(|v| v.push(1));
    ///
    /// let a = X.take();
    ///
    /// assert_eq!(a, vec![1]);
    ///
    /// X.with_ref(|v| assert!(v.is_empty()));
    /// ```
    pub fn take(&'static self) -> T where T: Default;

    /// Replaces the contained value, returning the old value.
    ///
    /// # Panics
    ///
    /// Panics if the value is currently borrowed.
    ///
    /// Panics if the key currently has its destructor running,
    /// and it **may** panic if the destructor has previously been run for this thread.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::cell::RefCell;
    ///
    /// thread_local! {
    ///     static X: RefCell<Vec<i32>> = RefCell::new(Vec::new());
    /// }
    ///
    /// let prev = X.replace(vec![1, 2, 3]);
    /// assert!(prev.is_empty());
    ///
    /// X.with_ref(|v| assert_eq!(*v, vec![1, 2, 3]));
    /// ```
    pub fn replace(&'static self, value: T) -> T;
}
```

# Drawbacks
[drawbacks]: #drawbacks

- We can no longer use the method names `set`, `get`, etc. on `LocalKey<T>` (if `T` can include `Cell` or `RefCell`).

- It might encourage code that's less efficient on some platforms.
  A single `THREAD_LOCAL.with(|x| ..)` is more efficient than using multiple `.set()` and `.get()` (etc.),
  since it needs to look up the thread local address every time, which is not free on all platforms.

# Alternatives

Alternatives for making it easier to work with thread local cells:

- Don't do anything, and keep wrapping everything in `.with(|x| ..)`.

- Somehow invent and implement the `'thread` or `'caller` lifetime, removing the need for `.with(|x| ..)`.

- Add `THREAD_LOCAL.borrow()` and `THREAD_LOCAL.borrow_mut()`, just like `RefCell` has.

  This wouldn't be sound.
  One could move the returned proxy object into a thread local that outlives this thread local.
  (Or just `Box::leak()` it.)

Alternatives for avoiding the initializer:

- Add a `LocalKey<T>::try_initialize` method.

  - This will be bit more complicated to implement efficiently.
    (A `LocalKey` just contains a single function pointer to the thread-local-address-getter, which is often optimized out.
    This doesn't play nice with being generic over the initialization function.)

  - Thread locals with a `const` initializer (currently unstable, but likely stabilized soon) do not have the concept of being 'uninitialized' and do not run any lazy initialization.
    With `.set()` for `LocalKey<Cell<T>>`, that doesn't make a difference, as overwriting the const-initialized value has the same effect.
    However, for the generic `LocalKey<T>` we cannot allow changes without internal mutability,
    meaning that we can allow initialization (like `.try_initialize()`),
    but not changing it later (like `.set()`).
    Since a `const` initialized thread local does not know whether its value has been observed yet,
    we can't do anything other than implement `.try_initialize()` by always failing or panicking.

  - Even if this function existed, it would still be nice to have a simple `THREAD_LOCAL.set(..)`.

# Prior art
[prior-art]: #prior-art

- [`scoped-tls`](https://docs.rs/scoped-tls/1.0.0/scoped_tls/struct.ScopedKey.html)
  provides 'scoped thread locals' which must be `.set()` before using them. (They will panic otherwise.)

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Should we use the names `with_borrow` and `with_borrow_mut` instead of `with_ref` and `with_mut`, to match `RefCell`'s method names?
- Do we also want anything for `UnsafeCell`? Maybe `LocalKey<UnsafeCell<T>>::get()` to get the `*mut T`, just like `UnsafeCell<T>::get()`.
- Are there any other types commonly used as thread locals for which we should do something similar?
- Should `.set` skip the initializer, or not? We should consider this question again at stabilization time, and we should listen for anyone reporting concerns here (especially if it caused semantically unexpected behavior).
