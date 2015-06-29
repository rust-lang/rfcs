- Start Date: 2015-06-29
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Define `Interior<T>` type and associated language item. `Interior<T>`
is structurally identical to `T` (as in, same memory layout, same
structure fields, same enum discriminant interpretation), but has no
traits implemented. Define a new `DropValue` trait that takes an
`Interior<T>` argument (instead of `&mut T`). This will allow fields
to be moved out of a compound structure during the drop glue. The
drop-glue will change to directly invoke the `DropValue` hook with the
`Interior<T>` structure of the `T` structure being dropped.

# Motivation

Currently, the `Drop::drop` method does not take its argument by
value. This is unfortunate, as there exist cases where it may be
important for a type to control whether and how its embedded fields
are destructed. Consider the `SmallVec` example from the
[`ManuallyDrop` RFC][manuallydrop]:

```rust
struct SmallVec<T> {
    length: uint,
    capacity: uint,
    pointer: *mut T,
    inline: [T, .. 8]
}
```

This type is defined to preferentially use the `inline` field for
value storage, to avoid unnecessary interactions with the heap when
only a vew values must be stored. However, whether or not the `inline`
field is used for value storage, the destructors for all 8 elements of
the array *will* be called, unless counter-measures are taken.
Currently, counter-measures are taken: the `SmallVec` implementation
explicitly sets the drop-flag for each unfilled element of the array,
such that the drop-glue will not be invoked for that element. But as
the drop-flag is going away this is not a viable long-term solution.

As another example, consider a `MessageForwarder` type, which promises
to send an attached message to a recipient when the forwarder is
dropped:

```rust
struct Message;
unsafe impl marker::Send for Message;
struct MessageForwarder<'a> {
  c: &'a mpsc::Sender<Message>,
  m: Message,
}
impl<'a> Drop for MessageForwarder<'a> {
  fn drop(&mut self) {
    // currently generates compile-error
    self.c.send(self.m);
  }
}
```

There is nothing memory-unsafe in what `MessageForwarder` is trying to
do. The language ought to support the desired behavior.

My personal motivation for this RFC is that it can be used by a
[proposed linear-types facility][lineartypes] to allow wrapping a
linear-type in an affine type (which can be useful to make linear
types more ergonomic in some circumstances). In the linear types
proposal, implementing `Drop` against a compound type containing a
linear field will allow the compound container type to be treated as
affine. In that case, the linear field must be moved out during the
drop-hook to satisfy linearity constraints, but the current drop-hook
API does not allow partial moves from the dropped value. The
drop-behavior defined in this RFC *would* be forward-compatible with
the proposed linear-types mechanism.

[lineartypes]: https://github.com/rust-lang/rfcs/issues/814

This RFC proposes that the drop-hook take a form of `self` by value,
instead of by reference. I say "a form of", because we want the
drop-hook to allow moves out of the `self` argument, but partial moves
are disallowed for types implementing `Drop`. Also, if the argument
were the direct `self` value, then it would usually imply that new
drop-glue would be inserted at the end of the drop-hook, resulting in
an infinite recursive loop at the end of the drop-hook. What this
approach needs is a different view of the `self` argument to `drop`
which does *not* implement `Drop`, so that partial moves would be
allowed, and so that the drop-hook would not re-invoke itself
recursively by default.

Other solutions to this issue in current-Rust require circumlocutions
for a `Drop`-implementing compound type to control whether or how its
embedded fields get dropped. These approaches will be discussed in the
`Alternatives` section, below.

# Detailed design

We define an `Interior<T>` type and associated `"interior"` language
item:

```rust
#[lang="interior"]
struct Interior<T>(T);
```

An instance of `Interior<T>` is defined to be structurally identical
to `T`, but is an independent type. Since it is an independent type,
it can have a unique set of `impl`s defined against it: the `impl`s
defined against `T` are not inherited by `Interior<T>`. In particular,
this means that `Interior<T>` does not implement `Drop`, which means
it is (by default) valid to partially or fully deconstruct an
`Interior<T>` instance (even where it would not be valid to
deconstruct an instance of `T`).

We also define a new `DropValue` trait to replace the legacy `Drop`
trait, and which takes an instance of `Interior<Self>` by value:

```rust
#[lang="dropvalue"]
trait DropValue {
    fn drop_value(Interior<Self>);
}
```

Define the following bare functions to convert between `T` and
`Interior<T>`:

```rust
fn into_interior<T>(t: T) -> Interior<T>;
fn from_interior<T>(t: Interior<T>) -> T;
fn as_interior<T>(t: &T) -> &Interior<T>;
fn as_interior_mut<T>(t: &mut T) -> &mut Interior<T>;
fn of_interior<T>(t: &Interior<T>) -> &T;
fn of_interior_mut<T>(t: &mut Interior<T>) -> &mut T;
```

And then it is possible to define a compatibility `Drop` trait in
terms of `DropValue`:

```rust
impl<T: Drop> DropValue for T {
  fn drop_value(this: Interior<Self>) {
    of_interior_mut(this).drop();
  }
}
```

## Compiler-generated drop-glue.

In this design, the compiler-generated drop-glue for a type gets so
simplified as to become nearly, or possibly entirely, non-existent.
Consider the following types:

```rust
struct Foo;
impl Drop for Foo { ... }

struct Bar(Foo, Foo);
impl Drop for Bar { ... }

enum Baz {
  Empty,
  AmFoo(Foo),
  AmBar(Bar),
}
impl Drop for Baz { ... }
```

In current Rust, the drop-glue for these types will look something
like this:

```rust
fn drop_glue_Foo(v: Foo) {
  v.drop();
  mem::forget(v);
}
fn drop_glue_Bar(v: Bar) {
  v.drop();
  drop_glue_Foo(v.0);
  drop_glue_Foo(v.1);
  mem::forget(v);
}
fn drop_glue_Baz(v: Baz) {
  v.drop();
  match v {
    Empty => (),
    AmFoo(val) => drop_glue_Foo(val),
    AmBar(val) => drop_glue_Bar(val),
  }
  mem::forget(v);
}
```

Under this proposal, the drop-glue for member fields is called at
the point `this` is consumed:

```rust
impl DropValue for Foo {
  fn drop_value(this: Interior<Foo>) {
    println!("drop_foo");
    // no compiler-inserted clean-up necessary
  }
}
impl DropValue for Bar {
  fn drop_value(this: Interior<Bar>) {
    println!("drop_bar");
    // "this" is consumed at function exit, so the compiler
    // will generate the following code:
    // DropValue::<Foo>::drop_value(into_interior(this.0));
    // DropValue::<Foo>::drop_value(into_interior(this.1));
    // and the following will be displayed as this function is called:
    // drop_bar
    // drop_foo
    // drop_foo
  }
}
impl DropValue for Baz {
  fn drop_value(this: Interior<Baz>) {
    // mem::drop() is #[inline], so the compiler will replace this
    // call:
    mem::drop(this);
    // with something like the following code:
    // match this.discriminant {
    //   Empty => (),
    //   AmFoo(val) => DropValue::<Foo>::drop_value(into_interior(val)),
    //   AmBar(val) => DropValue::<Bar>::drop_value(into_interior(val)),
    // }
    println!("drop_baz");
    // note that the println! should be evaluated *after* `this` is
    // dropped. So if `this` were `AmFoo(Foo)`, then the following
    // would be displayed:
    // drop_foo
    // drop_baz
  }
}
```

The drop-glue function no longer appears to be necessary: all clean-up
now occurs in the drop-hook itself, in the same way it does for types
that do not have drop-glue defined.

## `impl DropValue for Interior<T>`

We do not prohibit `impl DropValue for Interior<T>`: it should just
work in the natural way.

```rust
struct Foo;
impl DropValue for Interior<Foo> {
  fn drop_value(this: Interior<Interior<Foo>>) {
    println!("drop_interior_foo");
  }
}
impl DropValue for Foo {
  fn drop_value(this: Interior<Foo>) {
    println!("drop_foo");
    // `this` is cleaned up as it falls out of scope. the compiler
    // will generate something like the following code at this point:
    // DropValue::<Interior<Foo>>::drop_value(into_interior(this));
  }
}
fn main() {
  mem::drop(Foo);
  // the compiler will generate something like the following code
  // at this point:
  // DropValue::<Foo>::drop_value(into_interior(Foo));
}
// the following text should be displayed on running this program:
// drop_foo
// drop_interior_foo
```

## Interaction with `impl Drop`

The blanket implementation of `DropValue` for `Drop` types means that
it will be impossible for a user to simultaneously define `DropValue`
and `Drop` for a given type. On the other hand, it will be possible to
implement `Drop` for `T` and `DropValue` for `Interior<T>`, or
vice-versa. The drop-hooks will then be called in the natural way.

## `Deref` and `DerefMut`

I also suggest implementing `Deref` and `DerefMut` for `Interior<T>`
(where `Deref::Target` would be `T`). This will allow the drop-hook to
call functions defined against `T`, without needing to go through
`of_interior` or `of_interior_mut` (which would be a significant
ergonomic problem).

```rust
impl<T> Deref for Interior<T> {
  type Target = T;
  fn deref(&self) -> &T {
    of_interior(self)
  }
}
impl<T> DerefMut for Interior<T> {
  fn deref_mut(&mut self) -> &mut T {
    of_interior_mut(self)
  }
}
```

# Drawbacks

Additional compiler complexity. Potentially awkward concept to
document and explain. Major use is for the drop-hook, though such a
broad-purpose concept seems like it should have other important uses.
Drop-hook function declaration is unfortunately verbose.

# Alternatives

* A field of type `U` can be replaced with an `Option<U>`, which can
be populated with `Some()` for the lifetime of the container, and
updated with `None` by the `drop` call. This introduces unnecessary
overhead, and is not self-documenting. It's not clear in the structure
definition whether it would be valid for the field to be `None` during
the active lifetime of the structure without comments, and the
constraint that the value be populated during the container's active
lifetime cannot be enforced at the language-level.

* A [`ManuallyDrop`][manuallydrop] or [`NoDrop`][nodrop] wrapper could
be placed around the field of type `U`. This approach requires unsafe
code to explicitly enable dropping a member field (using `ptr::read`
to move droppable data out of a `NoDrop` type). Since it is not
actually memory unsafe to control whether or not a field's destructor
will run (at least when the field is considered valid), it seems
undesirable to force clients to use `unsafe` for this purpose. While
unsafe code will always be necessary for the `SmallVec` motivating
example (since `SmallVec` explicitly allows uninitialized fields, and
these are necessarily unsafe to deal with), the `MessageForwarder`
example *should* be implementable with strictly safe code.

[manuallydrop]: https://github.com/rust-lang/rfcs/pull/197
[nodrop]: https://crates.io/crates/nodrop

* The `*_interior` functions above could be made `impl` functions for
`struct Interior<T>`. I chose not to do so in this RFC to avoid any
potential for conflict with functions defined against `T`.

# Unresolved questions

The name can be bike-shedded. Alternatives include `Synonym<T>`,
`Mirror<T>`, `Structure<T>`, `Inner<T>`, `Inside<T>`, `Bare<T>`,
`Naked<T>`, `Contents<T>`, perhaps there are others?

# Acknowledgments

This is based on a design by @eddyb.