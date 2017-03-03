- Start Date: 2015-01-05
- RFC PR: 
- Rust Issue: 

# Summary

New unsafe methods are added to `Any` for unchecked downcasting, a small
performance and code-bloat-avoidance optimisation in cases where the dynamic
type is already known.

# Motivation

The checked downcasting methods are all very well, but there exist various
cases where the type being downcast to is known with certainty by dint of
careful use of the type system, for example in the HTTP header representation
schemes employed by the Hyper and Teepee libraries and the AnyMap data
structure. In such cases, the checked cast methods return an `Option` or
`Result` which is immediately unwrapped with certainty, simply leading to a
little bit of code bloat and some unnecessary runtime overhead.

This use case is so firm that the aforementioned libraries have all implemented
these unchecked methods on their own account, Hyper via the [unsafe-any] crate,
AnyMap in itself (the [anymap] crate) to select a couple of examples. This is
an opportunity to shift that maintenance burden into the core distribution,
where it is a trivial matter, and allow others to conveniently reap this
benefit when applicable.

# Detailed design

Currently there are three methods providing downcasting:

```rust
// In src/libcore/any.rs:
impl Any {
	pub fn downcast_ref<T: 'static>(&self) -> Option<&T>;
	pub fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T>;
}

// In src/liballoc/boxed.rs:
impl Box<Any> {
	pub fn downcast<T: 'static>(self) -> Result<Box<T>, Box<Any>>;
}
```

(Truth to tell the `impl Box<Any>` shown above is currently a `BoxAny` trait,
but this is noted as something that will be changed; it simply hasn’t been
done, that’s all. We can do it as part of this change.)

Each of these methods will be augmented by a new unsafe method according to the
following signatures:

```rust
// In src/libcore/any.rs:
impl Any {
	pub unsafe fn downcast_ref_unchecked<T: 'static>(&self) -> &T;
	pub unsafe fn downcast_mut_unchecked<T: 'static>(&mut self) -> &mut T;
}

// In src/liballoc/boxed.rs:
impl Box<Any> {
	pub unsafe fn downcast_unchecked<T: 'static>(self) -> Box<T>;
}
```

The checked downcast methods can then be changed to call the unchecked version,
also, simplifying their code just a tad.

The changes are simple enough that no further explanation is deemed necessary here.

An implementation of this RFC can be found at the time of writing at
[chris-morgan’s `unchecked-downcast-methods-for-any` branch].

# Drawbacks

The currently completely safe `Any` API gains some unsafe methods.

It is probable that at some point someone will use this inappropriately, in
cases where the type system is not being used to guarantee that `T` is correct,
and so at some point it will break for them and they will then blame Rust for
having eaten their laundry. Of course, we will have the moral high ground,
being able to point out the word “unsafe”, but I bet they still won’t be happy.

# Alternatives

The only alternative is not adding these methods. Leaving this as it is will
lead to various users maintaining approximately the same code themselves, as
detailed in the Motivation above.

# Unresolved questions

Can we conceive of a situation where inappropriate use of unchecked downcasting
would cause Rust to literally eat one’s laundry?

<!-- I guess it might take the deployment of Rust code on an embedded device, a
laundry appliance. That’s probably noteworthy enough that we should throw a
party and call it International Rust Ate My Laundry Day. -->

[unsafe-any]: https://crates.io/crates/unsafe-any
[anymap]: https://crates.io/crates/anymap
[chris-morgan’s `unchecked-downcast-methods-for-any` branch]: https://github.com/chris-morgan/rust/compare/master...unchecked-downcast-methods-for-any
