- Feature Name: `phantomdata_dropck`
- Start Date: 2022-10-19
- RFC PR: [rust-lang/rfcs#3331](https://github.com/rust-lang/rfcs/pull/3331)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC deprecates and eventually removes `PhantomData` participation in dropck
entirely, as it has been broken since the introduction of NLL with Rust 2018
and backported to Rust 2015 with rustc 1.36.

# Motivation
[motivation]: #motivation

dropck and NLL currently disagree about how `PhantomData` are dropped, as
can be demonstrated:

For one, this didn't compile in Rust 2015 before 1.36:

```rust
use std::cell::Cell;
use std::marker::PhantomData;

struct Foo<'a> {
  selfref: Cell<Option<&'a Foo<'a>>>,
}
impl<'a> Drop for Foo<'a> {
  fn drop(&mut self) {
  }
}

fn make_selfref<'a>(x: &'a PhantomData<Foo<'a>>) {}
fn make_pd<'a>() -> PhantomData<Foo<'a>> {
    unimplemented!()
}
fn main() {
  let x = make_pd();
  make_selfref(&x);
}
```

The same behaviour can be observed by wrapping the `PhantomData` in a struct:

```rust
use std::cell::Cell;
use std::marker::PhantomData;

struct Wrapper<T: ?Sized>(PhantomData<T>);

struct Foo<'a> {
  selfref: Cell<Option<&'a Foo<'a>>>,
}
impl<'a> Drop for Foo<'a> {
  fn drop(&mut self) {
  }
}

fn make_selfref<'a>(x: &'a Wrapper<Foo<'a>>) {}
fn make_wrapper<'a>() -> Wrapper<Foo<'a>> {
    unimplemented!()
}
fn main() {
  let x = make_wrapper();
  make_selfref(&x);
}
```

However, by causing dropck to run, it still fails to compile today. A simple
way to make dropck run is to use a drop sibling:

```rust
use std::cell::Cell;
use std::marker::PhantomData;

struct DropSibling;
impl Drop for DropSibling {
  fn drop(&mut self) {}
}

struct Wrapper<T: ?Sized>(DropSibling, PhantomData<T>);

struct Foo<'a> {
  selfref: Cell<Option<&'a Foo<'a>>>,
}
impl<'a> Drop for Foo<'a> {
  fn drop(&mut self) {
  }
}

fn make_selfref<'a>(x: &'a Wrapper<Foo<'a>>) {}
fn make_wrapper<'a>() -> Wrapper<Foo<'a>> {
    unimplemented!()
}
fn main() {
  let x = make_wrapper();
  make_selfref(&x);
}
```

This means dropck is overly restrictive in comparison with NLL, and NLL and
dropck are separate compiler subsystems.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This change brings dropck in line with NLL, by completely deprecating any and
all dropck-related functionality from `PhantomData`. In other words,
`PhantomData` becomes a purely variance-related construct (and a few other
things related to auto traits like `Send`/`Sync` and whatnot).

The following section must be removed from `PhantomData` documentation:

```
/// ## Ownership and the drop check
///
/// Adding a field of type `PhantomData<T>` indicates that your
/// type owns data of type `T`. This in turn implies that when your
/// type is dropped, it may drop one or more instances of the type
/// `T`. This has bearing on the Rust compiler's [drop check]
/// analysis.
///
/// If your struct does not in fact *own* the data of type `T`, it is
/// better to use a reference type, like `PhantomData<&'a T>`
/// (ideally) or `PhantomData<*const T>` (if no lifetime applies), so
/// as not to indicate ownership.
///
/// [drop check]: ../../nomicon/dropck.html
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

As with NLL, any `Copy` type must be completely ignored by dropck, and as such
if a type parameter only ever gets used in a `Copy` type, it too must be
ignored. Note that this only applies to *drop glue*. Any type which explicitly
implements `Drop` experiences the same behaviour as today, as defined by [RFC
1238].

# Drawbacks
[drawbacks]: #drawbacks

This change allows emulating `#[may_dangle]` on stable, with some restrictions
and runtime overhead. However, it could already be done with creative use of
`ManuallyDrop` - which does not participate in drop glue.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This change brings dropck in line with NLL. By doing so, it eliminates major
surprises in the edge cases between compiler subsystems.

It is important to note that this change does not/cannot break safe code, and
unsafe code today does not rely on these edge cases. Indeed, existing unsafe
code can get away with not using `PhantomData` at all due to the changes
brought by [RFC 1238], tho it still needs to be aware of variance and/or auto
traits - which are best handled by just using `PhantomData`. In fact, despite
the outdated comment (which hasn't changed since rustc 1.2, a couple of months
before [RFC 1238]), this is what the compiler-internal `Unique<T>` pointer uses
`PhantomData` for.

It's also pointing out that these changes are sound - `ManuallyDrop` has them,
and while `ManuallyDrop` is unsound in the presence of parametric drop, we do
not have parametric drop since [RFC 1238]. In fact, this change is basically
implicitly wrapping every `Copy` type in `ManuallyDrop`.

The main alternative would be to special case `PhantomData` in NLL. It is yet
unclear how to accomplish that.

The second main alternative would be to do nothing and/or officially
stabilize/fossilize the existing "drop sibling" behaviour.

# Prior art
[prior-art]: #prior-art

[RFC 1238] is kinda prior art, but there really isn't much in terms of prior
art for this change.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

The only question is whether this is how we wanna do it. See also alternatives
above.

# Future possibilities
[future-possibilities]: #future-possibilities

This change makes it particularly impossible to check dropck behaviour of
`?Sized` types. A future proposal could bring a `PhantomDrop` marker type
which is simply

```rust
struct PhantomDrop<T: ?Sized>(PhantomData<T>);
unsafe impl<#[may_dangle] T: ?Sized> Drop for PhantomDrop<T> {
  fn drop(&mut self) {}
}
```

for any cases where checking the dropck behaviour of a potentially-`?Sized`
type is necessary for soundness, without relying on `alloc` or nightly. The
`selfref` crate is an example of where such checks are necessary for soundness.
(Note that this RFC does not break `selfref`.)

[RFC 1238]: https://github.com/rust-lang/rfcs/blob/master/text/1238-nonparametric-dropck.md
