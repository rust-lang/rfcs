- Feature Name: `daddy_borrowck_little_dropck`
- Start Date: 2023-02-13
- RFC PR: [rust-lang/rfcs#3390](https://github.com/rust-lang/rfcs/pull/3390)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Provide a flexible framework for understanding and resolving dropck obligations,
built upon the borrow checker; simplify dropck elaboration, and effectively
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
In particular, it tries to encode the soundness obligations of `may_dangle` and
dropck itself in the type/borrow system, directly.

## Custom Box and Custom Collections

The perhaps main use-case for a stable `may_dangle` is custom collections. With
the `MyBox` above, we can have a `Drop` impl as below:

```rust
impl<T: SafeToDrop + '!> Drop for MyBox<T> {
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

Rust functions generally have an implicit requirement that all lifetimes must
outlive the function. This can be desugared by treating:

```rust
fn foo<'a>(x: &'a ()) {}
```

as:

```rust
fn foo<'a: 'fn>(x: &'a ()) {}
```

Likewise, types also come with an implicit `'self` lifetime:

```rust
struct Foo<'a>(&'a ());
```

is really:

```rust
struct Foo<'a: 'self>(&'a ());
```

The ability to opt-out of this bound is called "non-local lifetimes". They're
lifetimes external or out-of-scope to a type or function.

This out-of-scope-ness can be represented using blocks:

```rust
fn foo<'a: '!, 'b: '!, 'c>(arg1: &'a (), arg2: &'b (), arg3: &'c ()) {
    function_body();
}
```

becomes:

```rust
let arg1;
let arg2;
let arg3;
{ // block of non-local lifetimes
    // need to make sure these are non-'static
    let temporary_arg1 = ();
    let temporary_arg2 = ();
    arg1 = &temporary_arg1;
    arg2 = &temporary_arg2;
}
let temporary_arg3 = ();
arg3 = &temporary_arg3;
function_body();
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Syntax

The core syntax for opting out of the implicit lifetime bounds is `'!` in bound
position. This is further restricted to only being allowed in functions
(including trait functions), data types (structs, enums), and impls.

This is an opt-out bound: it is similar to `?Sized`.

## Interaction with borrowck

The non-local lifetime interacts with borrowck the exact same way as the block
representation above shows, replicated below for contextualization:

```rust
fn foo<'a: '!, 'b: '!, 'c>(arg1: &'a (), arg2: &'b (), arg3: &'c ()) {
    function_body();
}
```

becomes:

```rust
let arg1;
let arg2;
let arg3;
{ // block of non-local lifetimes
    // need to make sure these are non-'static
    let temporary_arg1 = ();
    let temporary_arg2 = ();
    arg1 = &temporary_arg1;
    arg2 = &temporary_arg2;
}
let temporary_arg3 = ();
arg3 = &temporary_arg3;
function_body();
```

This effectively makes arg1 and arg2 unusable in the function body.

## Interactions with references

References have a few special properties:

1. They participate in implicit lifetime bounds.
2. They're generally required to be alive.

This RFC relaxes those properties. For the first one, we have that `&'a &'b T`
does no longer imply `'b: 'a` if `'b: '!`, and thus `&'b T: 'a` is also no
longer true.

The second one is a bit more complicated. However, there is an existing
mechanism which also breaks this assumption: `#[may_dangle]`. Putting
`#[may_dangle]` on a lifetime allows that lifetime to no longer be alive when
the code runs (but `#[may_dangle]` has its own set of issues, like allowing
the relevant lifetimes to be interacted with).

## Interactions with dropck

This opt-out would enable implementing `Drop` for self-referential structs:

```rust
struct Foo<'a: '!> {
    inner: Cell<Option<&'a Foo<'a>>>,
}

impl<'a: '!> Drop for Foo<'a> {
    fn drop(&mut self) {
        // can't actually use `self` here, as per the previously defined rules.
    }
}

let foo = Foo { inner: Default::default() };
foo.inner.set(Some(&foo));
// foo dropped here, compiles successfully.
```

## Interactions with Box/Vec, adapting for `may_dangle`

Traits can be implemented for lifetime relations which are stricter than the
lifetime relations of the parent types, today:

```rust
struct Foo<'a, 'b, T>(&'a T, &'b T);

impl<'a, 'b, T> Default for Foo<'a, 'b, T> where 'a: 'b {
  ...
}
```

However, the lifetime relations are unique to the trait impl - they cannot be
specialized (it's not possible to impl the same trait for two different sets of
lifetime relations for the same type).

Thus we simply have the compiler implicitly build a `SafeToDrop` trait impl for
every type, as such:

```rust
struct Foo<'a, T> {
  field1: &'a (),
  field2: T,
  ...
}

impl<'a: '!, T: '!> SafeToDrop for Foo<'a, T> where &'a (): SafeToDrop, T: SafeToDrop {}
```

In other words, a type impls `SafeToDrop` when all of its fields impl
`SafeToDrop`, but the `SafeToDrop` impl doesn't apply any additional lifetime
bounds except those required by such `SafeToDrop` impls.

When the type has a `Drop` impl, the `SafeToDrop` impl follows the `Drop` impl:

```rust
struct Foo<'a, T> {
  field1: &'a (),
  field2: T,
  ...
}
impl<'a, T> Drop for Foo<'a, T> { ... }
impl<'a, T> SafeToDrop for Foo<'a, T> where &'a (): SafeToDrop, T: SafeToDrop {}
```

(Note how the `SafeToDrop` impl now requires `'a: 'self` and `T: 'self`, due to
the requirements of the `Drop` impl.)

The lang items that are special for `SafeToDrop` are:

```rust
impl<'a: '!, T: '!> SafeToDrop for &'a T {}
impl<'a: '!, T: '!> SafeToDrop for &'a mut T {}
impl<T: '!> SafeToDrop for *const T {}
impl<T: '!> SafeToDrop for *mut T {}
impl<T: '!> SafeToDrop for ManuallyDrop<T> {}
impl<T: '!> SafeToDrop for PhantomData<T> {} // but see unresolved questions below
impl<T: '!, const N: usize> SafeToDrop for [T; N] where T: SafeToDrop {} // but see unresolved questions below
```

The following lang item gains a `SafeToDrop` bound, and loses its implied/`'fn`
bounds:

```rust
pub unsafe fn drop_in_place<T: '! + SafeToDrop + ?Sized>(to_drop: *mut T) {...}
```

# Drawbacks
[drawbacks]: #drawbacks

The Rust compiler pretty extensively assumes `&'a &'b T` implies `'b: 'a, T: 'b`
and this completely changes that.

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

Leveraging borrowck is good. The fact that `: '!` acts akin to `?Sized` can be a
bit confusing, but is necessary for backwards compatibility, and further, if we
treat `may_dangle` as a bound, it's basically in the name: *may* dangle.

For soundness reasons, `SafeToDrop` cannot be opted-out-of. In fact, due to
`drop_in_place` being `unsafe`, it's technically possible to hide it altogether,
make `PhantomData` require a `T: SafeToDrop`, and just add to the
`drop_in_place` contract that it must be "logically owned" (however one chooses
to define that). But simply exposing `SafeToDrop` is enough to avoid issues like
rust-lang/rust#99413. (Just because unsafe Rust is unsafe, doesn't mean we
should add known footguns.)

# Prior art
[prior-art]: #prior-art

- Compiler MCP 563: This RFC was supposed to come after the implementation of MCP 563 but that didn't happen. This RFC is basically a refinement of the ideas in the MCP.
- Unsound dropck elaboration for `BTreeMap`: <https://github.com/rust-lang/rust/pull/99413>
- `may_dangle`: RFC 1238, RFC 1327

# Unresolved questions
[unresolved-questions]: #unresolved-questions

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
detect potentially-unsound `Drop` impls in *current stable*. For example, the
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

## Generalize to "bounds generics" or "associated bounds" (or typestate?)

This proposal limits itself to dropck semantics, but a future proposal could
generalize these kinds of bounds to some sort of "bounds generics" or
"associated bounds" or "typestate" kind of system.
