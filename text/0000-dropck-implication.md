- Feature Name: `dropck_implication`
- Start Date: 2023-02-13
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Provide a flexible framework for understanding and resolving dropck obligations,
built upon implied bounds, simplify dropck elaboration, and effectively
stabilize (a refinement of) `may_dangle`.

# Motivation
[motivation]: #motivation

Rust's drop checker (dropck) is an invaluable tool for making sure `impl Drop`
is sound. However, it can be too strict sometimes. Consider a custom `Box`:

```rust
struct MyBox<T> {
    inner: *const T,
}

impl<T> MyBox<T> {
  fn new(t: T) -> Self { ... }
}

impl<T> Drop for MyBox<T> {
    fn drop(&mut self) { ... }
}
```

This is... fine. However, actual `Box`es have the following property:

```rust
let x = String::new();
let y = Box::new(&x);
drop(x);
// y implicitly dropped here
```

Meanwhile, using `MyBox` produces the following error:

```text
error[E0505]: cannot move out of `x` because it is borrowed
  --> src/main.rs:16:10
   |
15 |     let y = MyBox::new(&x);
   |                        -- borrow of `x` occurs here
16 |     drop(x);
   |          ^ move out of `x` occurs here
17 |     // y implicitly dropped here
18 | }
   | - borrow might be used here, when `y` is dropped and runs the `Drop` code for type `MyBox`
```

This is where `may_dangle` comes in: it allows the impl to say "I don't touch
this parameter in a way that may cause unsoundness", and in fact the real `Box`
does use it. However, `may_dangle` was introduced as a hack specifically to make
`Box` (and collections like `Vec`) work like this, and it was never intended to
be the final form.

`may_dangle` is itself a refinement of "unguarded escape hatch" (or UGEH for
short), because UGEH was found to be too much of a footgun even for an internal
compiler feature. UGEH effectively applied `may_dangle` to *all* parameters. But
even `may_dangle` sometimes comes back to bite, for example when dropck was
simplified, causing a broken BTreeMap implementation to become unsound. (see
rust-lang/rust#99413)

This RFC proposes a *safe* refinement of `may_dangle`, while also making it
resistant to the pitfalls observed with the existing `may_dangle` mechanism.
This refined mechanism can be called "Liveness obligations" or "Dropck bounds".
In particular, it tries to encode the soundness obligations of `may_dangle` in
the type system, directly, so that they can be checked by the compiler.

## Custom Box and Custom Collections

The perhaps main use-case for a stable `may_dangle` is custom collections. With
the `MyBox` above, we can have a `Drop` impl as below:

```rust
impl<T: '!> Drop for MyBox<T> {
  fn drop(&mut self) {
    unsafe {
      drop_in_place(self.inner);
      free(self.inner);
    }
  }
}
```

(N.B. this still uses `unsafe`! however, the unsafety is about upholding the
contract of `drop_in_place` and `free`, *not* the `: '!` mechanism.)

## Self-referential types

The second use-case for a stable `may_dangle` is the ability to have `Drop` for
self-referential types. This doesn't come up too often, but:

```rust
struct Foo<'a> {
  this: Cell<Option<&'a Foo<'a>>>
}

impl<'a: '!> Drop for Foo<'a> {
  fn drop(&mut self) {
    ...
  }
}
```

Without this proposal, such a struct cannot be dropped. (It is, in fact,
possible to write such a `Drop` impl. It just errors at site-of-use.)

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## Liveness obligations

Liveness obligations are effectively bounds related to the liveness of a
type/lifetime. Lifetime bounds are themselves a form of liveness obligations:
a lifetime bound defines an outlives relationship between a type/lifetime and
another lifetime. However, there are also 3 kinds of liveness obligations which
are specially relevant to dropck. These can also be called dropck obligations,
and they are named as such:

1. You must not touch the type/value in question.
2. You may only drop the type/value in question.
3. You may freely access the type/value in question.

For a type which does not itself implement `Drop`, these are implied by what's
in the type. These implications are akin to variance or traits like `Send` and
`Sync`. Unlike `Send`/`Sync` these cannot be overriden by non-`Drop` types.

For a type which implements `Drop`, these obligations can be added to the type
parameters, in which case they must be explicitly specified on both the type
and the `Drop` impl.

### You may only drop the type/value

The most interesting of these obligations is probably obligation type 2: you may
only drop the type/value. This obligation is particularly interesting for
collection types.

For example, consider a custom `Box`:

```rust
struct MyBox<T> {
    inner: *const T,
}

impl<T> MyBox<T> {
  fn new(t: T) -> Self { ... }
}

impl<T> Drop for MyBox<T> {
    fn drop(&mut self) { ... }
}
```

This is... fine. However, actual `Box`es have the following property:

```rust
let x = String::new();
let y = Box::new(&x);
drop(x);
// y implicitly dropped here
```

Meanwhile, using `MyBox` produces the following error:

```text
error[E0505]: cannot move out of `x` because it is borrowed
  --> src/main.rs:16:10
   |
15 |     let y = MyBox::new(&x);
   |                        -- borrow of `x` occurs here
16 |     drop(x);
   |          ^ move out of `x` occurs here
17 |     // y implicitly dropped here
18 | }
   | - borrow might be used here, when `y` is dropped and runs the `Drop` code for type `MyBox`
```

As the error says, "borrow might be used here, when `y` is dropped and runs the
`Drop` code for type `MyBox`". To allow `MyBox` to have this property, simply
put a "You may only drop the type/value in question." bound on `T`, like so:

```rust
struct MyBox<T: '!> {
    inner: *const T,
}

impl<T> MyBox<T> {
  fn new(t: T) -> Self { ... }
}

impl<T: '!> Drop for MyBox<T: '!> {
    fn drop(&mut self) { ... }
}
```

This "You may only drop the type/value in question." bound, represented by
`T: '!`, is only (directly) compatible with exactly one function:
`core::ptr::drop_in_place`, which has a `T: '!` bound. (N.B. it's also perfectly
fine to wrap `drop_in_place` and call it indirectly, as long as the dropck
bound is carried around.) Trying to use `T` in any other way causes an error.
For our example, this bound prevents the `Drop` impl from using the borrow,
tho it can still be dropped. (Obviously, dropping a borrow is a no-op.)

### You may freely access the type/value in question

This obligation is the default obligation (requires no additional syntax), for
both backwards compatibility reasons and because it puts no restrictions on the
`Drop` impl: the `Drop` impl can do whatever it wants with the given type/value.

For example, it could print something out on `Drop`:

```rust
use core::fmt::Display;

struct PrintOnDrop<T: Display>(T);

impl<T: Display> Drop for PrintOnDrop<T> {
    fn drop(&mut self) {
        println!("{}", self.0);
    }
}
```

This is only possible while the type parameter is still live. As such, the
compiler rejects the following code:

```rust
let x = String::new();
let y = PrintOnDrop(&x);
drop(x);
// y dropped here
```

```text
error[E0505]: cannot move out of `x` because it is borrowed
  --> src/main.rs:14:10
   |
13 |     let y = PrintOnDrop(&x);
   |                         -- borrow of `x` occurs here
14 |     drop(x);
   |          ^ move out of `x` occurs here
15 |     // y implicitly dropped here
16 | }
   | - borrow might be used here, when `y` is dropped and runs the `Drop` code for type `PrintOnDrop`
```

And it also rejects an `T: '!` bound:

```rust
struct PrintOnDrop<T: Display + '!>(T);

impl<T: Display + '!> Drop for PrintOnDrop<T> {
    fn drop(&mut self) {
        println!("{}", self.0);
    }
}
```

```text
TODO
```

### You must not touch the type/value in question

This obligation prevents you from touching the type/value in your `Drop` impl
entirely. This is also the most flexible for the *user* of your type.

So far, the other 2 dropck obligations have applied to type parameters. This
dropck obligation instead applies to lifetimes.

Actually, there are 2 forms of it: As applied to one or more lifetimes, it
prevents accessing those lifetimes. As applied to *all possible* lifetimes, it
prevents accessing the *type* entirely. A bound can be applied to all possible
lifetimes with the use of HRTB syntax, i.e. `for<'a>`.

#### As applied to one (or more) lifetimes

[modified from tests/ui/dropck/issue-28498-ugeh-with-lifetime-param.rs]

[TODO explain]

```rust
// run-pass

// Demonstrate the use of the unguarded escape hatch with a lifetime param
// to assert that destructor will not access any dead data.
//
// Compare with ui/span/issue28498-reject-lifetime-param.rs

#![feature(dropck_implication)]

#[derive(Debug)]
struct ScribbleOnDrop(String);

impl Drop for ScribbleOnDrop {
    fn drop(&mut self) {
        self.0 = format!("DROPPED");
    }
}

struct Foo<'a: '!>(u32, &'a ScribbleOnDrop);

impl<'a: '!> Drop for Foo<'a> {
    fn drop(&mut self) {
        // Use of `'a: '!` means destructor cannot access `self.1`.
        println!("Dropping Foo({}, _)", self.0);
    }
}

fn main() {
    let (last_dropped, foo0);
    let (foo1, first_dropped);

    last_dropped = ScribbleOnDrop(format!("last"));
    first_dropped = ScribbleOnDrop(format!("first"));
    foo0 = Foo(0, &last_dropped);
    foo1 = Foo(1, &first_dropped);

    println!("foo0.1: {:?} foo1.1: {:?}", foo0.1, foo1.1);
}
```

#### As applied to all possible lifetimes

When given a type and a lifetime:

```rust
struct Bar<'a: '!, T: '!>(&'a T);

impl<'a: '!, T: '!> Drop for Bar<'a, T> {
  ...
}
```

This treats `T` like it needs to be safe to drop, which is overly restrictive.
(After all, a type with `'a` and `T: 'a` could have both `&'a T` and an owned
`T` in it.) So we need a way to convey that we don't want that.

[FIXME: `for<'a> &'a T: '!` doesn't make sense, what we want is more akin to
`for<'a> 'a: T + '!`, or "for all `'a`, `'a` outlives `T` and may dangle", which
is just cursed.]

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Syntax

The core syntax for dropck obligations is `'!` in bound position. This is
further restricted to only being allowed in functions (including trait
functions), data types (structs, enums), and impls.

## Extent of bound-ness

Dropck obligations aren't true bounds. If anything, they're syntactic sugar for
an implicit impl. They also work backwards from how most bounds work. This was
chosen for both ergonomics and backwards compatibility reasons.

A type is considered "safe to drop" if all of its fields are considered "safe to
drop".

For example, given a struct like:

```rust
struct Foo<'a>(&'a str);
```

The struct `Foo` is considered safe to drop if `&'a str` is safe to drop. Since
`&'a str` is always safe to drop, then `Foo` is always safe to drop.

For another example, given a struct like:

```rust
struct Bar<T: '!>(T);

impl<T: '!> Drop for Bar<T> {
  ...
}
```

Then `Bar` is safe to drop if `T` is safe to drop. (Note that `Bar` implicitly
drops `T`.)

Effectively, it's as if these were automatically generated by the compiler:

```rust
impl<'a> SafeToDrop for Foo<'a> where &'a str: SafeToDrop {}
impl<T> SafeToDrop for Bar<T> where T: SafeToDrop {}
```

But they are only evaluated where `Drop` needs to be called.

Meanwhile, `ManuallyDrop` is always safe to drop. As a lang item, it behaves as
if:

```rust
impl<T> SafeToDrop for ManuallyDrop<T> {}
```

Additionally, where a parameter is used in the `Drop` impl, the type is only
safe to drop while the parameter is alive:

```rust
struct Baz<T>(T);

impl<T> Drop for Baz<T> {
  ...
}

// effectively:
impl<T> SafeToDrop for Baz<T> where T: Alive {}
```

(A type which is `Alive` is also `SafeToDrop`.)

These examples also show that this kind of bound is primarily about types, not
lifetimes. But `may_dangle` also works on lifetimes, so they must be supported
too. Lifetimes are "easier" to support, since we just need to forbid using their
scope entirely.

## Parametricity (or lack thereof)

Dropck bounds are non-parametric. They effectively assert properties about the
concrete type `T`: either that it is safe to use, or that it is safe to drop.

If it is safe to use, then the `Drop` impl is allowed to use it. If it's safe to
drop, then the `Drop` impl is only allowed to drop it.

Given the classic example of parametric dropck unsoundness: (rust-lang/rust#26656)

```rust
// Using this instead of Fn etc. to take HRTB out of the equation.
trait Trigger<B> { fn fire(&self, b: &mut B); }
impl<B: Button> Trigger<B> for () {
    fn fire(&self, b: &mut B) {
        b.push();
    }
}

// Still unsound Zook
trait Button { fn push(&self); }
struct Zook<B: '!> { button: B, trigger: Box<Trigger<B>+'static> }

impl<B: '!> Drop for Zook<B> {
    fn drop(&mut self) {
        self.trigger.fire(&mut self.button);
    }
}

// AND
struct Bomb { usable: bool }
impl Drop for Bomb { fn drop(&mut self) { self.usable = false; } }
impl Bomb { fn activate(&self) { assert!(self.usable) } }

enum B<'a> { HarmlessButton, BigRedButton(&'a Bomb) }
impl<'a> Button for B<'a> {
    fn push(&self) {
        if let B::BigRedButton(borrowed) = *self {
            borrowed.activate();
        }
    }
}

fn main() {
    let (mut zook, ticking);
    zook = Zook { button: B::HarmlessButton,
                  trigger: Box::new(()) };
    ticking = Bomb { usable: true };
    zook.button = B::BigRedButton(&ticking);
}
```

This errors directly in `Zook<B>::drop`, since it attempts to call an
inappropriate function.

## Interactions with `ManuallyDrop`

`may_dangle` interacts badly with `ManuallyDrop`. This is unsound:

```rust
struct Foo<T>(ManuallyDrop<T>);

unsafe impl<#[may_dangle] T> Drop for Foo<T> {
  fn drop(&mut self) {
    unsafe { ManuallyDrop::drop(&mut self.0) }
  }
}
```

Because `may_dangle` just defers to the drop obligations of the fields, and
`ManuallyDrop` does not have drop obligations regardless of its contents, you
can give it a type that *does* have (invalid) drop obligations and cause UB:

```rust
struct Bomb<'a>(&'a str);
impl<'a> Drop for Bomb<'a> {
  fn drop(&mut self) { println!("{}", self.0); }
}

let s = String::from("hello");
let bomb = Foo(Bomb(&s));
drop(s);
```

Meanwhile, this RFC requires the `Drop for Foo<T>` to be annotated with "safe to
drop" obligations on `T` (i.e. `T: '!`), preventing this mistake. With
`may_dangle`, one needs to remember to add a `PhantomData<T>` for the dropck
obligations instead. Likewise for pointer types.

While the compiler can't prevent double-frees here without real typestate, it
can at least prevent regressions like rust-lang/rust#99413.

## Dropck elaboration

After processing all implied bounds for the types, dropck becomes a matter of
checking those bounds when dropping. This is a weakened form of typestate,
which only applies to dropck, but is not too dissimilar to existing dropck. The
main difference is that the type is already fully annotated by the time we get
here, so no recursing into the type's fields is necessary.

In other words, we treat dropck obligations as both bounds (when writing code
and running typeck) and annotations (when running dropck).

Given a type `T`:

- If `T` has no (type or lifetime) parameters, then it can be dropped.
- If `T` has lifetime parameters:
    - For each lifetime parameter that is not annotated with a `'!` bound, check
        that it's still live.
- If `T` has type parameters:
    - For each type parameter that is not annotated with a `'!` bound, check
        that it's still live.
    - For each type parameter that is annotated with a `'!` bound, check that it
        can be dropped.

## Tweaks to Typeck

Lifetimes tagged as `'!` cannot be used. Types tagged as `'!` cannot be used
except where `drop_in_place` is concerned. `unsafe` does not allow sidestepping
these restrictions. If all possible lifetimes for a type are tagged as `'!`
(i.e. `for<'a> &'a T: '!`), then said type cannot be dropped either (since that
would require creating a reference to it, for `fn drop(&mut self)`).

## Dropck obligations for built-in types

The following types have the given dropck implications (based on existing usage
of `#[may_dangle]`):

```text
ManuallyDrop<T> where for<'a> &'a T: '!
PhantomData<T> where for<'a> &'a T: '! // see below for unresolved questions
[T; 0] where for<'a> &'a T: '! // see below for unresolved questions
*const T where for<'a> &'a T: '!
*mut T where for<'a> &'a T: '!
&'_ T where for<'a> &'a T: '! // N.B. this is special
&'_ mut T where for<'a> &'a T: '!

OnceLock<T: '!>
RawVec<T: '!, A>
Rc<T: '!>
rc::Weak<T: '!>
VecDeque<T: '!, A>
BTreeMap<K: '!, V: '!, A>
LinkedList<T: '!>
Box<T: '!, A>
Vec<T: '!, A>
vec::IntoIter<T: '!, A>
Arc<T: '!>
sync::Weak<T: '!>
HashMap<K: '!, V: '!, A>
// FIXME: other types which currently use may_dangle but were not found by grep
```

# Drawbacks
[drawbacks]: #drawbacks

Why should we *not* do this?

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This design is a further refinement of RFC 1327 with intent to stabilize. It
tries to avoid introducing too many new concepts, but does attempt to integrate
the lessons learned from RFC 1327 into the type system - specifically, allowing
them to be checked by the compiler.

In particular, this design explicitly prevents the user from doing unsound
operations in safe code, while also allowing `impl Drop` to be *safe* even in
the presence of liveness obligations/dropck bounds.

The explicit goals are:

- Safe `impl Drop` for self-referential structs.
- Preventing dropck mistakes, both with first-party and third-party collections.

Explicit **non**-goals include:

- Preventing double frees, use-after-free, etc in unsafe code.

Leveraging typeck is good. The fact that `: '!` acts akin to `?Sized` can be a
bit confusing, but is necessary for backwards compatibility, and further, if we
treat `may_dangle` as a bound, it's basically in the name: *may* dangle.

As a consequence of `'!` being akin to `?Sized`, something like
`<'a: '!, 'b: 'a>` does not imply `'b: '!`, while `<'a: 'b + '!, 'b>` does imply
`'b: '!`. (In the first, `'b` outlives `'a`, which may dangle. if `'a` dangles,
`'b` does not also need to dangle. In the second, `'a` outlives `'b`, and so if
`'a` dangles, then so must `'b`.)

# Prior art
[prior-art]: #prior-art

- Compiler MCP 563: This RFC was supposed to come after the implementation of MCP 563 but that didn't happen. This RFC is basically a refinement of the ideas in the MCP.
- Unsound dropck elaboration for `BTreeMap`: <https://github.com/rust-lang/rust/pull/99413>
- `may_dangle`: RFC 1238, RFC 1327

# Unresolved questions
[unresolved-questions]: #unresolved-questions

## Syntax for "no dropck obligations, cannot use or drop type"

This RFC proposes the syntax `for<'a> &'a T: '!` to discharge all dropck
obligations and restrict `impl Drop` the most. However, something feels off
about this syntax, but we can't quite put a finger on what. Meanwhile, using
`T: '!` and `'a: '!` for the rest of the dropck bounds feels fine.

## Spooky-dropck-at-a-distance

Currently, the following code is not accepted by the compiler:

```rust
use core::cell::Cell;
use core::marker::PhantomData;

struct Dropper<T>(T);

impl<T> Drop for Dropper<T> {
    fn drop(&mut self) {}
}

struct Foo<'a, T>(PhantomData<Dropper<Cell<&'a Foo<'a, T>>>>, T);

fn main() {
  fn make_selfref<'a, T>(x: &'a Foo<'a, T>){}
  let x = Foo(PhantomData, String::new());
  make_selfref(&x);
}
```

At the same time, replacing `String::new()` with `()` makes it compile, which
is really surprising: after all, the error is seemingly unrelated to the
`String`. This is an interaction between at least 4 factors: `T` having drop
glue, the presence of `Dropper`, the invariance of `'a`, and the use of
`make_selfref`.

This RFC recommends removing this spooky-dropck-at-a-distance behaviour
entirely, to make the language less surprising. Since this RFC ties
user-defined dropck obligations with a `Drop` impl, it automatically prevents
users from defining similar spooky-dropck behaviour. If removed, the above
example would be accepted by the compiler.

However, this spooky-dropck behaviour can also be used in no-alloc crates to
detect potentially-unsound `Drop` impls in current stable. For example, the
`selfref` crate *could* do something like this:

```rust
type UBCheck...;

// paste selfref::opaque! ub_check here.
```

But this RFC provides an alternative way which does not require spooky-dropck,
and `selfref` explicitly does not rely on spooky-dropck as the behaviour of
spooky-dropck has changed in the past: namely, between Rust 1.35 and Rust 1.36,
in the 2015 edition: <https://github.com/rust-lang/rust/issues/102810#issuecomment-1275106549>.
(Arguably, this is when it *became* "spooky".)

## Behaviour of `[T; 0]`

Should `[T; 0]` have dropck obligations? As above, this also basically falls
under spooky-dropck-at-a-distance, since `[T; 0]` lacks drop glue. This RFC
proposes treating `[T; 0]` the same as `PhantomData`, as above.

# Future possibilities
[future-possibilities]: #future-possibilities

## `dyn Trait` dropck obligations

Currently, if you attempt to make a self-referential (e.g.) `dyn Iterator`, you
get an error:

```rust
use core::cell::RefCell;

#[derive(Default)]
struct Foo<'a> {
    vec: RefCell<Option<Vec<&'a Foo<'a>>>>,
    iter: RefCell<Option<Box<dyn Iterator<Item=&'a Foo<'a>>>>>,
}

fn main() {
    let x = Foo::default();
    x.vec.borrow_mut().insert(vec![]).push(&x);
}
```

```text
error[E0597]: `x` does not live long enough
  --> src/main.rs:11:44
   |
11 |     x.vec.borrow_mut().insert(vec![]).push(&x);
   |                                            ^^ borrowed value does not live long enough
12 | }
   | -
   | |
   | `x` dropped here while still borrowed
   | borrow might be used here, when `x` is dropped and runs the destructor for type `Foo<'_>`
```

A future RFC could build on this RFC to allow traits to demand dropck
obligations from their implementers. Adding these bounds to existing traits
would be semver-breaking, so it can't be done with `Iterator`, but it could
be useful for other traits.

## Generalize to "bounds generics" or "associated bounds"

This proposal limits itself to dropck semantics, but a future proposal could
generalize these kinds of bounds to some sort of "bounds generics" or
"associated bounds" kind of system.
