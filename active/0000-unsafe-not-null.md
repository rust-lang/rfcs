- Start Date: 2014-04-05
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Add an `#[unsafe_not_null]` attribute to struct fields to enable the
nullable-pointer optimization for certain types containing raw pointers.

# Motivation

Currently, we do an optimization for certain enums where we know that the
valid representation of a type does not include null. For example:

```rust
enum Option<T> {
    Some(T),
    None
}
```

`Option<~T>` will not include an explicit discriminator, it will re-use "null"
instead, because `~T` can never be null. Unfortunately this optimization does
not extend beyond owning pointers and references. This hurts smart pointer
types such as `Rc<T>`, which will never be null, but `Option<Rc<T>>` still has
overhead from the descriminator.

# Detailed design

Motivating example:

```rust
#[unsafe_not_null]
struct Rc<T> {
    inner: *RcBox<T>
}
```

The `#[unsafe_not_null]` attribute, when applied to a `struct` which contains
only a single field whose type is an unsafe pointer, marks this structure as
eligible for the "nullable pointer" optimization. If the attribute is used
on a struct with more than one field, or whose only field is not a raw
pointer, the program will be rejected.

# Alternatives

One alternative that may seem appealing is to use some sort of marker type,
`NotNull<T>`. This is unappealing because it will require a method to do the
dereferencing, and it cannot implement `Deref` or `DerefMut`. Why? Because
this is wrapping an unsafe pointer, and `Deref`/`DerefMut` are assumed to be
safe. Introducing another method call on all uses of smart pointers using
`NonNull<T>` is going to put even more pressure on the already-strained LLVM
optimizers. Not that it cannot handle this; it will just be even slower.

A different generalization would be adding some way to annotate that certain
values of a type's representation are not valid/used, and trans could use that
information to pack enum variants into that. This is very complex, and it is
not clear how to best implement this. One benefit this approach would have is
being able to have a wrapper around float types that uses a NaN for the None
in `Option<SomeFloatyName>`.

# References

- <https://github.com/mozilla/rust/issues/7576>
- <https://github.com/mozilla/rust/issues/13194>
