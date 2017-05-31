- Start Date: 2014-03-31
- RFC PR #:
- Rust Issue #:

# Summary

The new `Deref` and `DerefMut` traits can enable more expressive APIs but
create some design caveats that should be thought out sooner rather than later.
This RFC will both change the APIs of current first-party types implementing
these traits and serve as a set of conventions for future implementations.

This was discussed previously in mozilla/rust#13126.

# Motivation

`Deref` and `DerefMut` are implemented by the various smart pointers in the
standard library. Some, like `std::cell::Ref` and `sync::Arc` consist of
nothing but `Deref`, `DerefMut` and `Drop` impls, and act just like a normal
copy of the wrapped type would. Others, like `std::rc::Rc` and
`sync::MutexGuard` have implementations of `Deref` and `DerefMut` as well
as other methods or public fields. This poses a couple of problems:

* It makes the types confusing to work with. When writing code that uses an
  `Rc<MyType>` or `MutexGuard<MyType>`, you end up with code that looks almost
  like code that uses a `MyType` directly except that a new field like `cond`
  or a new method like `downgrade()` is usable. If `MyType` already has a field
  or method with the same name as one of these, the author will get mysterious
  type errors until the real reason is discovered and the code is changed to
  the slightly strange looking `(*foo).cond`.

* A type that implements `DerefMut` and/or `Deref` with a generic parameter can
  never have public fields or methods added to it without breaking backwards
  compatibility.

# Detailed design

In general, types that implement `Deref` or `DerefMut` should not implement any
other methods or have any public fields. Methods like `downgrade` will move to
associated functions: `Rc::downgrade(foo)` instead of `foo.downgrade()`. Fields
like `cond` will also be accessed via associated functions:
`MutexGuard::cond(foo)` instead of `foo.cond`.

There are some exceptions to this rule. For example, the entire *point* of `Rc`
is to change the semantics of `clone`, so it would be silly for `Rc` to not
implement `Clone` directly. The same goes for `Arc`. Implementations of
operator traits that simply forward to the wrapped types are also probably
fine. These exceptions should be uncommon - I'd imagine it would apply almost
exclusively to core traits like `Clone`, `Eq`, `Ord`, etc.

Specifically, the following methods and fields will need to be altered:
```
std::rc::Rc::downgrade
sync::Arc::downgrade
sync::Arc::make_unique
sync::MutexGuard::cond
sync::RWLockWriteGuard::cond
sync::RWLockWriteGuard::downgrade
```

In addition, documentation should be added to `Deref` and `DerefMut` discussing
these conventions.

# Alternatives

C++ avoids this issue by having separate `.` and `->` operators. `foo.bar()`
calls the `bar` function on the smart pointer type and `foo->bar()` calls the
`bar` function on the wrapped type. Rust could change to emulate this setup.
This would be a truly enormous change. `->` could be limited to use with
`Deref` and `DerefMut`, but that ghettoizes smart pointers and would also
probably be a bit confusing.

Leaving the standard library's smart pointers as is will both limit our ability
to add to their APIs in the future and add unnecessary "gotcha's" to the
standard library.

# Unresolved questions

I'm a bit undecided on where some of the associated functions should end up.
For example, it may make more sense to have `Mutex::cond(foo)` than
`MutexGuard::cond(foo)` since `MutexGuard` is more of an implementation detail
than anything.
