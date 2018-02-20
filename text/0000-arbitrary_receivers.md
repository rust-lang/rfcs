- Feature Name: arbitrary_self_types
- Start Date: 2018-02-15
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Allow any type that implements `Deref` targeting `Self` to be the receiver of a
method.

This feature has existed, in an incomplete form since before version 1.0.
Because of its long history, this RFC is not to propose the feature, but to
document its behavior & stabilize it.

# Motivation
[motivation]: #motivation

Today, methods can only be received by value, by reference, by mutable
reference, or by a `Box<Self>`. This has always intended to be generalized to
support any kind of pointer, such as an `Rc<Self>` or an `Arc<Self>`. Since
late 2017, it has been available on nightly under the `arbitrary_self_types`
feature.

This feature is increasingly relevant because of the role of special pointer
types to constraint self-referential types, such as generators containing
internal references. Because different kinds of "smart pointers" can constrain
the semantics in non trivial ways, traits can rely on certain assumptions about
the receiver of their method, whereas just implementing the trait *for* a smart
pointer doesn't allow that kind of reliance.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When declaring a method, users can declare the type of the `self` receiver to
be any type `T` where `T: Deref<Target = Self>`. Shorthand exists, so that
`self` with no ascription is of type `Self`, and `&self` and `&mut self` are of
type `&Self` and `&mut Self`:

```rust
// All of these are valid:
trait Foo {
    fn by_value(self: Self);
    fn by_ref(self: &Self);
    fn by_ref_mut(self: &mut Self);
    fn by_box(self: Box<Self>);
    fn by_rc(self: Rc<Self>);
    fn by_arc(self: Arc<Self>);
}
```

## Object safety

Assuming a trait is otherwise object-safe, methods are object-safe is the
receiver implements `Deref`, and `Deref::Target` is `?Sized`. That is, if the
`Deref` impl requires that the target be a `Sized` type, this method is not
object safe.

If the receiver type must be `Sized`, then this receiver is not object safe.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Handling trait objects

To support object-safety, we must have a way of obtaining a vtable from the
reference type. To implement this, we will call `Deref::deref` on the receiver
type to obtain an `&Trait` object, and use the vtable pointer from that object.

For example, consider this type:

```rust
struct Foo<T: ?Sized> {
    inner: T,
}

impl<T: ?Sized> Deref for Foo<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.inner
    }
}

trait Bar {
    fn baz(self: Foo<Self>);
}
```

Here, we start with an `&Foo<Trait>`, which is represented as a wide pointer.
When we call `Deref::deref`, we receive a wide pointer, with the data pointer
pointing to `self.inner`, and the vtable pointing to that same vtable.

Rather than trying to figure out how to obtain the vtable by inspecting the
structure during compilation, we can rely on this Deref implementation to find
the vtable for us.

# Drawbacks
[drawbacks]: #drawbacks

This has the same drawbacks as the general `Deref` trait feature: users could
use this to create method receivers that are not really appropriate as method
receivers (such as types that are not really "smart pointers.") We will
continue to discourage these sorts of `Deref` impls as highly unidiomatic.

# Rationale and alternatives
[alternatives]: #alternatives

The primary alternative to this is not to extend support for other types of
method receivers (that is, to do nothing).

We could restrict method receivers with some additional trait beyond `Deref`,
so that the original author of the type must opt into being a receiver at all.
There seems to be little reason to do this, since `Deref` already allows
syntatic extensions because of the role it plays in method resolution. Users
who create a `Deref` type intend for it to be used in a manner analogous to
this RFC.

# Unresolved questions
[unresolved]: #unresolved-questions

The solution to object safety and resolving the vtable has not been
implemented yet.
