- Feature Name: remove_lifetime_elision_in_type_parameter_position
- Start Date: Mon Feb 16 11:16:28 CET 2015
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Remove lifetime elision in type parameter position.

# Motivation

Lifetime elision in type parameter position makes it impossible to see whether a
function's return value borrows any of its arguments. E.g.
```rust
fn f(&self) -> X;
```
might borrow `self` or not. One has to look at the statements inside the
function body or the definition of `X` to find out if `X` borrows `self`.

Note that this change does not affect the most common case of lifetime elision:
```rust
fn f(&self) -> &Y;
```

Note that this is not a rustdoc problem as rustdoc can be modified to always
show elided lifetimes.

Note that this was already listed as a drawback in the original RFC.

In general, rust favors explicit over implicit syntax unless the benefits
greatly outweigh the disadvantages:

- Type inference
- Implicit referencing (`f(x) => f(&x)`)

# Detailed design

Don't allow lifetime elision in type parameter position.

# Drawbacks

Lifetime elision in type parameter position is no longer allowed.

# Alternatives

Retain lifetime elision with a twist:
```rust
fn f(&self) -> X<'_>;
```
