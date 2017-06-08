- Feature Name: derive-deref
- Start Date: 2016-08-01
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Allow deriving `Deref` and `DerefMut`.

# Motivation
[motivation]: #motivation

Newtypes are a common pattern in Rust, and dereferencing is a convenient way to
work with these. Implementing `Deref` and `DerefMut` is quite mildly annoying,
compared to the triviality.

# Detailed design
[design]: #detailed-design

Add a `derive_Deref` and `derive_DerefMut` attribute (that is, `derive(Deref)`
and `derive(DerefMut)`), which derives the respective implementation for
single-field `struct`s to deref to their only field.

```rust
#[derive(Deref, DerefMut)]
struct MyType<T> {
    inner: T,
}

// Now MyType<T> implements Deref<Target = T> as well as DerefMut.
```

# Drawbacks
[drawbacks]: #drawbacks

This encourages using `Deref` for non-pointer types, which could be considered
an anti-pattern.

# Alternatives
[alternatives]: #alternatives

Newtype derive, where every implementation of the inner type is reimplemented
for the newtype.

# Unresolved questions
[unresolved]: #unresolved-questions

None.
