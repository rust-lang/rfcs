- Start Date: 2014-12-19
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Commit to marking `std::mem::drop` as unstable until negative bounds are implemented in the language.

# Motivation

`std::mem::drop` (henceforth `drop`, [as confusing as that is](https://github.com/rust-lang/rfcs/pull/535)) is a delightful little function in the standard library whose purpose is to take ownership of a value and then do nothing whatsoever with that value, thus causing it to immediately go out of scope.

Here is the function's whole definition:

```rust
pub fn drop<T>(_x: T) { }
```

I use this `drop` surprisingly often when showing off Rust to people, as its extreme simplicity is a great way to demonstrate Rust's ownership semantics (steveklabnik can corroborate how impressed people generally are with this tiny but useful function).

There is a catch, though: `drop`, as defined, can accept a value of any type with no restrictions, *which includes implicitly copyable types*. In other words, code such as the following is valid:

```rust
let x: int = 2;  // implicitly copyable!
drop(x);  // x can still be used after this point!
```

This runs counter to the purpose of the function, which is to take sole ownership of the value. I propose that this be amended by changing the definition of the function to only permit types which can cede ownership, as follows:

```rust
pub fn drop<T: !Copy>(_x: T) { }
```

This requires negative bounds to be implemented. Fortunately, negative bounds are a feature which are all but certain to arrive in the imminent future. However, this may not be until post-1.0. In the meantime, I propose that `drop` be marked as unstable until such point in time as they are implemented.

This does mean that users will not be able to make use of this function for 1.0. I contend that this is not a problem in the slightest, because the function itself is almost literally the simplest Rust function imaginable should they decide to reimplement `drop` themselves.

# Detailed design

This is a policy decision and has no detailed design.

# Drawbacks

Might force stable users to define their own trivial one-line function for a while (or just use `let _ = foo;`).

# Alternatives

Leave it as is, and accept that `drop` may be called with arguments that make no sense, making it harder to teach about ownership.

# Unresolved questions

Is the real part of every non-trivial zero of the Riemann zeta function 1/2?
