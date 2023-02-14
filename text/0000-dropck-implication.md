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

TODO

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

unsafe impl<'a: '!> Drop for Foo<'a> {
    fn drop(&mut self) {
        // Use of `'a: '!` is sound, because destructor never accesses `self.1`.
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

[TODO explain, this is more of a construct to enable the semantics of &T etc]

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

TODO: nesting rules

The following types have the given dropck implications (based on existing usage
of `#[may_dangle]`):

```text
ManuallyDrop<T> where for<'a> &'a T: '!
PhantomData<T> where for<'a> &'a T: '!
*const T where for<'a> &'a T: '!
*mut T where for<'a> &'a T: '!
&'_ T where for<'a> &'a T: '!
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
```

# Drawbacks
[drawbacks]: #drawbacks

Why should we *not* do this?

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- Why is this design the best in the space of possible designs?
- What other designs have been considered and what is the rationale for not choosing them?
- What is the impact of not doing this?
- If this is a language proposal, could this be done in a library or macro instead? Does the proposed change make Rust code easier or harder to read, understand, and maintain?

# Prior art
[prior-art]: #prior-art

Discuss prior art, both the good and the bad, in relation to this proposal.
A few examples of what this can include are:

- For language, library, cargo, tools, and compiler proposals: Does this feature exist in other programming languages and what experience have their community had?
- For community proposals: Is this done by some other community and what were their experiences with it?
- For other teams: What lessons can we learn from what other communities have done here?
- Papers: Are there any published papers or great posts that discuss this? If you have some relevant papers to refer to, this can serve as a more detailed theoretical background.

This section is intended to encourage you as an author to think about the lessons from other languages, provide readers of your RFC with a fuller picture.
If there is no prior art, that is fine - your ideas are interesting to us whether they are brand new or if it is an adaptation from other languages.

Note that while precedent set by other languages is some motivation, it does not on its own motivate an RFC.
Please also take into consideration that rust sometimes intentionally diverges from common language features.

- Compiler MCP 563: This RFC was supposed to come after the implementation of MCP 563 but that didn't happen. This RFC is basically a refinement of the ideas in the MCP.
- Unsound dropck elaboration for `BTreeMap`: <https://github.com/rust-lang/rust/pull/99413>

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- What parts of the design do you expect to resolve through the RFC process before this gets merged?
- What parts of the design do you expect to resolve through the implementation of this feature before stabilization?
- What related issues do you consider out of scope for this RFC that could be addressed in the future independently of the solution that comes out of this RFC?

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
