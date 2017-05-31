- Start Date: 2015-06-29
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Define `Interior<T>` type and associated language item. `Interior<T>`
is structurally identical to `T` (as in, same memory layout, same
structure fields, same enum discriminant interpretation), but has no
traits implemented. Define a new `DropInterior` trait that takes an
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
**Alternatives** section, below.

# Detailed design

We define an `Interior<T>` type and associated `"interior"` language
item. The `T` parameter of `Interior<T>` will be restricted to types
that implement `Drop`, or other such traits (and this RFC defines one)
that require the `T` value to be fully populated on clean-up. (For
purposes of exposition, we will describe all such traits as
"`Drop`-like traits".) We enforce this restriction by defining a
`DropGlue` marker-type:

```rust
// in libcore::marker:

// marker trait indicates that an instance of this type must be
// fully-populated on clean-up. For example:
//     struct MyStruct1;
//     impl Drop for MyStruct1 { ... }
//     struct MyStruct2;
//     fn print_dropglue<T: DropGlue>() {
//         println!("has drop-glue!");
//     }
//     print_dropglue::<MyStruct1>(); // prints "has drop-glue!"
//     // fails to compile:
//     // print_dropglue::<MyStruct2>();
#[lang="drop_glue_marker"]
pub trait DropGlue;
```

With the `DropGlue` marker-type defined we can restrict `Interior<T>`
such that it is only defined against types that implement `Drop`-like
traits:

```rust
// in libcore::interior:

// Presents access to `T` type, as though it did NOT implement any
// drop-related clean-up functions.
#[lang="interior"]
struct Interior<T: DropGlue>(T);
```

Since `Interior<T>` cannot have drop-glue, this restriction prevents
construction of `Interior<Interior<T>>` types. Further, we've
prevented construction of `Interior<T>` types when the underlying `T`
does not implement a drop-related clean-up function.

An instance of `Interior<T>` is defined to be structurally identical
to `T`, but is an independent type. Since it is an independent type,
it can have a unique set of `impl`s defined against it: the `impl`s
defined against `T` are not inherited by `Interior<T>`. However, it
will always be valid to partially or fully deconstruct an
`Interior<T>` instance (though it would not be valid to deconstruct an
instance of `T`).

We also define a new `DropInterior` trait to replace the legacy `Drop`
trait, and which takes an instance of `Interior<Self>` by value:

```rust
#[lang="drop_interior"]
trait DropInterior {
    fn drop_interior(Interior<Self>);
}
```

Define the following functions to convert between `T` and
`Interior<T>`:

```rust
impl<T> Interior<T> {
    unsafe fn from_outer_unsafe(outer: T) -> Self;
    fn into_outer(this: Self) -> T;
    fn as_outer(this: &Self) -> &T;
    fn as_outer_mut(this: &mut Self) -> &mut T;
    fn of_outer(outer: &T) -> &Self;
    fn of_outer_mut(outer: &mut T) -> &mut Self;
}
```

(Note that `from_outer_unsafe` is made unsafe to avoid a
backwards-compatibility risk with current `Drop`. This design point
will also be discussed under **Alternatives**.)

These functions will be used to define conversion traits against
`Interior<T>`:

```rust
impl<T> AsRef<T> for Interior<T> { ... }
impl<T> AsMut<T> for Interior<T> { ... }
```

(With negative bounds, a safe variant of `from_outer` could be defined
for !Drop types, and `From<T>` could be implemented for `Interior<T:
!Drop>`. This is discussed under **Alternatives**.)

With these definitions in place, it is possible to define the legacy
`Drop` trait in terms of `DropInterior`:

```rust
impl<T: Drop> DropInterior for T {
    fn drop_interior(mut this: Interior<T>) {
        Interior::as_outer_mut(&mut this).drop();
    }
}
```

## Structurally identical

`Interior<T>` is defined to be structurally identical to `T`. There
are several things we mean by that:

* `Interior<T>` will have the same size and alignment as `T`.
* The fields of `Interior<T>` will have the same names as the fields of
`T` and will live at the same memory offsets within the structure.
* If `T` is an enum, then `Interior<T>` will have the same enum
discriminant and variants.
* The accessability of the fields of `Interior<T>` will be identical
to the accessability of the equivalent fields of `T`.

This definition is intended to support having the conversion from `T`
to `Interior<T>` (and vice-versa) be a no-op at run-time: the
introduction of the `Interior<T>` type should introduce *no* run-time
overhead.

## Drop-glue functions

In this design, we have eliminated the need for the drop-glue function
for a type to be compiler-generated. Rather, we make the drop-glue
function (called on scope-based clean-up for a `DropGlue`-value)
explicit, with an associated lang-item:

```rust
#[lang="drop_glue_fn"]
fn drop_glue<T: DropInterior>(arg: T) {
    // using "unsafe" variant to allow working with `Drop` types (as
    // opposed to `DropInterior` types that do NOT implement `Drop`).
    DropInterior::drop_interior(unsafe { Interior::from_outer_unsafe(arg) });
}
```

(Note that, under this proposal, all drop-hooks are invoked via
`DropInterior`, so restricting the `T` argument to implementors of
`DropInterior` will cover all types that require `DropGlue`.) This
drop-glue function is also used by trait vtables (to support running
drop-based clean-up when ownership is attached to a trait pointer, as
in `Box<Trait>`).

## Example clean-up process

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
    // we allow partial moves from `this`:
    mem::drop(this.0);
    println!("drop_bar");
    // the remainder of "this" is consumed at function exit, so the compiler
    // will generate the following code:
    // drop_glue::<Foo>(this.1);
    // and the following will be displayed as this function is called:
    // drop_foo
    // drop_bar
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
    //   AmFoo(val) => drop_glue::<Foo>(val),
    //   AmBar(val) => drop_glue::<Bar>(val),
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

## `Deref` and `DerefMut`

I also suggest implementing `Deref` and `DerefMut` for `Interior<T>`
(where `Deref::Target` would be `T`). This will allow the drop-hook to
call functions defined against `T`, without needing to go through
`Interior::as_outer` or `Interior::as_outer_mut` (which would be a
significant ergonomic problem).

```rust
impl<T> Deref for Interior<T> {
  type Target = T;
  fn deref(&self) -> &T {
    Interior::as_outer(self)
  }
}
impl<T> DerefMut for Interior<T> {
  fn deref_mut(&mut self) -> &mut T {
    Interior::as_outer_mut(self)
  }
}
```

# Drawbacks

## Expanding "unsafe"

`Interior::from_outer_unsafe` is not actually "unsafe" (in my
understanding of Rust's current definition of unsafe), but it does
pose a backwards-compatibility issue: In current Rust, there is no way
that the destructor for a field of a container will run *unless* the
destructor for the container has also run, and it is possible that
there is Rust code in the wild that relies on this property being
true. `from_outer_unsafe` changes this: using this function, it's
possible to cause a field's destructors to be called *without* having
called the container's destructor. Rust explicitly allows that failing
to call a destructor is "safe" behavior, so the `from_outer_unsafe`
behavior should be considered "safe" from a technical perspective.
But, borrowing an example from @eefriedman:

```rust
struct MaybeCrashOnDrop { c: bool }
impl Drop for MaybeCrashOnDrop {
    fn drop(&mut self) {
        if self.c {
            unsafe { *(1 as *mut u8) = 0 }
        }
    }
}
pub struct InteriorUnsafe { m: MaybeCrashOnDrop }
impl InteriorUnsafe {
    pub fn new() -> InteriorUnsafe {
        InteriorUnsafe { m: MaybeCrashOnDrop{ c: true } }
    }
}
impl Drop for InteriorUnsafe {
    fn drop(&mut self) {
        self.m.c = false;
    }
}
```

The above program would *not* crash with safe code in current Rust,
but could be made to crash, *by safe code*, if `Interior::from_outer`
were made safe.

An alternative design is discussed below under **Alternatives**, but I
could not identify an alternative that did all of the following:

* Preserved the current definition of unsafe;
* Did not pose a backwards-compatibility risk;
* Integrated the implementations of `Drop` and `DropInterior`.

As such, this proposal requires expanding the definition of "unsafe"
to also include "backwards compatibility risk". Or, at least, to have
an exception made that allows the `unsafe` marker on
`from_outer_unsafe` to mean "backwards compatibility risk", rather
than "memory unsafe".

## Standard-library complexity

We've expanded the number of standard-library concepts involved in
`Drop`-based clean-up from one (the `Drop` trait) to four (the
`DropGlue` marker, `struct Interior<T: DropGlue>`, the `DropInterior`
trait, and the `drop_glue` function). On the other hand, we've also
removed some magic (the `drop_glue` functions are no longer
compiler-generated, and the `DropGlue` marker documents when
partial-moves are disallowed).

The `Interior<T>` type does not have an obvious parallel in other
languages, so we will have a higher documentation burden in
explanation.

## Usage complexity

The `DropInterior` hook function declaration is unfortunately verbose.

# Alternatives

## Variants of this design

### Make `from_outer` safe.

As discussed above (under **Drawbacks**), the `from_outer` behavior is
not technically unsafe, but it does pose a backwards-compatibility
risk. We *could* ignore the backwards-compatibility issue, and call
`from_outer` safe. Or, we could keep the current drop-trait in place,
unmodified, define two parallel drop mechanisms in the language,
implement negative bounds, and only define `from_outer` on `T: !Drop`.

### Rely on negative bounds.

There are some aspects of this design that would, in my opinion, be
better if negative bounds were already supported:

* We could define a safe `from_outer` function (in addition to the
current `from_outer_unsafe`) that was only implemented for `T: !Drop`
types.
* We could use the safe `from_outer` to `impl<T> convert::From<T>
for Interior<T>`.

I don't think this proposal should block on negative bounds, but I've
tried to make the design forward-looking, so that when/if negative
bounds are implemented we can put these improvements in place.

### Drop-glue extensions

This proposal moves the drop-glue function into the language itself,
which allows considerably more flexibility in how drop-glue can be
implemented. In the interest of keeping this proposal bounded, I'm not
pursuing such avenues here.

## Other approaches

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

* Many have argued for a [consuming destructuring][destructure]
operation (as in `let MyStruct { field1, field2 } = mystruct`).
Consuming destructuring, along with a `DropValue` trait which took
`self` by value, could address many of the same use-cases as the
mechanism proposed in this RFC. I prefer the solution proposed here
for a few reasons:

** Consuming destructuring would have the same backwards-compatibility
concern that the `Interior::from_outer` operation does (current code
might assume that the containing structure would be dropped before the
fields are dropped), so consuming-destructuring would probably need to
be limited to types that don't implement `Drop`. As a result, we would
probably still require compiler support for two types of `Drop` trait.

** Taking `self` by value during the drop-hook would *require*
implementors to deconstruct (or otherwise consume) the `self` argument
in the drop-hook. Failure to do so would result in infinite recursion,
as drop calls itself when the `self` argument goes out of scope at the
end of the function. This is fragile, and probably not something we
want to force on users of the language.

** Some types can implement `Drop`, but are not easily consumed when
destructured. In particular, enums where no variants have non-copy
data, or structs in which all fields are `Copy`, cannot be easily
destructured.

[destructure]: https://internals.rust-lang.org/t/pre-rfc-allow-by-value-drop/1845

# Unresolved questions

The name can be bike-shedded. Alternatives include `Synonym<T>`,
`Mirror<T>`, `Structure<T>`, `Inner<T>`, `Inside<T>`, `Bare<T>`,
`Naked<T>`, `Contents<T>`, perhaps there are others?

# Acknowledgments

This is based on a design by @eddyb. Thank you to commenters on the
RFC PR.
