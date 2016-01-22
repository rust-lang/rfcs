- Feature Name:
- Start Date: 2016-01-22
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add compiler support for generic atomic operations.

# Motivation
[motivation]: #motivation

It is sometimes necessary or useful to use atomic operations in a generic
context.

```rust
Atomic<T>
```

where `T` is any type. This is currently possible but fails to compile if `T` is
too large for the native atomic operations of the target architecture.

# Detailed design
[design]: #detailed-design

Four language items are added:

```rust
#[lang = "atomic_load"]
fn atomic_load<T>(ptr: *const T, ordering: ...) -> T { ... }

#[lang = "atomic_store"]
fn atomic_store<T>(ptr: *const T, val: T, ordering: ...) { ... }

#[lang = "atomic_exchange"]
fn atomic_exchange<T>(ptr: *const T, val: T, ordering: ...) -> T { ... }

#[lang = "atomic_compare_exchange"]
fn atomic_compare_exchange<T>(ptr: *const T, old: &T, new: T, ordering: ...) -> T { ... }
```

When a call to one of the corresponding atomic intrinsics cannot be translated
to a native operation, it is translated to a call to the corresponding language
item.

`atomic_compare_exchange` performs a bitwise comparison of the current and the
expected value.

One intrinsic is added:

```rust
fn has_native_atomic_ops<T>() -> bool;
```

This intrinsic returns `false` iff a call to one of the aforementioned atomic
operations is translated to a library call.

# Drawbacks
[drawbacks]: #drawbacks

`atomic_compare_exchange` performs a bitwise comparison of the current and the
expected value. This must be so because atomic operations for `T: PartialEq`
where `T` has a native operation available will be translated to an operation
that performs a bitwise comparison.

# Alternatives
[alternatives]: #alternatives

None.

# Unresolved questions
[unresolved]: #unresolved-questions

None.
