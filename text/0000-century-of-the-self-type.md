- Feature Name: arbitrary_self_types
- Start Date: 2018-02-15
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Allow types that implement `Deref` targeting `Self` to be the receiver of a
method. If the receiver type also implements the correct `CoerceUnsized` bound,
that method is object safe.

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

## Recursive arbitrary receivers

Like the rule for deref coercions, the rule for receivers is recursive. If type
`T` implements `Deref` targeting type `U`, and type `U` implements `Deref`
targeting `Self`, `T` is a valid receiver (and so on outward).

For example, this self type is valid:

```rust
impl MyType {
     fn by_ref_to_rc(self: &Rc<Self>) { ... }
}
```

## Object safety

In order for these receivers to be object safe, some additional traits need to
be implemented. Given a reference type `Ptr<dyn Trait>`, the compiler must be
able to prove that `T: Unsize<dyn Trait>` implies `Ptr<T>:
CoerceUnsized<Ptr<dyn Trait>>`. If the compiler can prove this, methods with
these receivers are object safe (how they object safe conversion is implemented
is discussed later in the detailed design).

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Handling trait objects

To support object-safety, we must have a way of obtaining a vtable from the
reference type, and then passing the correct receiver type to the function in 
the vtable.

### Step 1: Obtaining the vtable

First, we call `Deref::deref` on the receiver type to obtain an `&Trait`
object. This will return an `&dyn Trait`. Since an `&dyn Trait` is defined as 2
words, one a pointer to the object data and the other a pointer to the vtable,
we can obtain the vtable by dereferencing the second pointer of this object.

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

### Step 2: Obtaining the correct receiver type

Having obtained the vtable, we now need to obtain a value of the correct type
to pass to the function. For example, given a trait like this:

```rust
trait Foo {
    fn bar(self: Rc<Self>);
}
```

The function in the vtable expects a value of `Rc<Self>`, where `Self` is the
concrete type that was cast into the vtable. So if we have an `Rc<Foo>`, we
need to temporarily cast that *back* into an `Rc<i32>` or whatever the concrete
type is.

This is why the `CoerceUnsized` bound is necessary for object-safe receiver
types. Using the type ID stored in the vtable, we can downcast the `Self` type
to the correct concrete type to pass to the function pointer for the method
we're calling, effectively reversing the unsizing coercion.

## Stabilization plans

As soon as possible, we intend to stabilize *using* a method receiver defined
this way, so long as it is object safe. That stabilization will immediately
extend support to `Rc<T>` and `Arc<T>`.

However, we also feel that `CoerceUnsized` is not ready to stabilize without
further consideration of the trade offs. For that reason, defining your own
arbitrary method receivers may not be stabilized as quickly.

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
implemented yet (whereas the non-object safe version is already available on
nightly).
