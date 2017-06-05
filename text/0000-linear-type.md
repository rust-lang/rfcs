- Start Date: 2015-01-28
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Add an `droppable-trait` language item, that will be applied to a
`Droppable` trait, that can be used to determine if variables of a
type can be implicitly dropped. Add a `linear-type` language item,
that will be applied to a `MakeLinear` unit struct, which will be used
to virally infect container-types with linear behavior. The compiler
will refuse to compile a source file in which a `linear` variable
would be implicitly dropped. A `linear` variable can be explicitly
dropped by either making it non-linear (by moving contained linear
fields out), or by using the `std::mem::forget` intrinsic. Add a
`DropPtr` pointer type, to allow partial moves out of a container as
it is being dropped. Add a `Finalize` trait, that behaves identically
to `Drop`, but can be applied to linear types to clean up during
unwinding. Add an `explicit_bounds` lint that will require that
generic type parameters for an `impl` have their bounds specified.
Compile the standard libraries with the `explicit_bounds` lint on, and
update as many APIs as make sense to be Linear-aware.

# Motivation

Scope-based drop is an implicit mechanism for ensuring that Rust
programs do not leak resources. For the most part, this implicit
mechanism works very well. However:

* Drop is an extremely limited interface, and may not capture all the
requirements of resource clean-up in some circumstances (such as when
failures can only be detected on a clean-up attempt, and failure
recovery is necessary).

* Fixed-memory system design will often require moves of data
structures between owners, while drops would yield resource leaks.
When operating with a fixed-memory constraint, *any* drop might be a
programmer error, of a type that could be prevented at compile time
with a linear type facility.

* Sometimes, a `drop` has side-effects whose timing can be important
for program correctness. In these cases, a developer may wish to
signal that the timing must be explicitly considered by preventing
implicit drop, and requiring explicit drop. If the timing of `drop`
events changes, or is allowed to change (for example, if [eager
drop](https://github.com/rust-lang/rfcs/pull/239) is adopted), then
linear types will greatly help developers to control the timing of
drop events.

I have seen some resistance to the idea that scope-based clean-up may
be inadequate, so I'll try to address that here.

## When is scope-based drop inappropriate?

Scope-based drop is inappropriate in scenarios where resource clean-up
has side-effects whose timing can affect program correctness. For
example, a `MutexGuard` will release a shared mutex when it is
dropped. The scope of time in which a shared mutex is locked is a
highly important consideration in writing correct multi-threaded code,
and highly important considerations such as this should be explicitly
reflected in code.

### Example: Force explicit mutex drop.

To take an example from [RFC #210](https://github.com/rust-lang/rfcs/pull/210):

```rust
        let (guard, state) = self.lock(); // (`guard` is mutex `LockGuard`)
        ...
        if state.disconnected {
            ...
        } else {
            ...
            match f() {
                Variant1(payload) => g(payload, guard),
                Variant2          => {}
            }

            ... // (***)

            Ok(())
        }
```
([source](https://github.com/rust-lang/rfcs/blob/a773ba113ba135ddb7f481c4829882733eaa5355/active/0000-remove-drop-flag-and-zeroing.md#the-early_noisy_drop-lint))

In this code, it is impossible to discern whether the author intended
or did not intend for the `MutexGuard` to be held in the `... //
(***)` code region. Developer intent could be properly signalled in
two ways:

1. If the developer intended that the lock possibly be held for the
`(***)` code, he could wrap the guard in an `Option`. This solution is
well understood, I don't feel I need to spend more time on it here.
2. If the developer intended that the lock *not* be held, he should
enforce that each branch of the `match` above cause a drop.

There is currently no way for rust to help the programmer to enforce
case 2. With `linear` types, this could be handled as follows:

```rust
        let (guard, state) = self.lock(); // (`guard` is mutex `LockGuard`)
        ...
        if state.disconnected {
            ...
        } else {
            ...
            let linear_guard = LinearOf(guard); // (`guard` moved into linear_guard)
            match f() {
                Variant1(payload) => g(payload, linear_guard),
                Variant2          => {
                    // Unless the `drop` is uncommented, compilation will
                    // fail with:
                    // ERROR: linear type `linear_guard` not fully consumed by block.
                    //drop(linear_guard.release())
                }
            }

            ... // (***)

            Ok(())
        }
        // existing drop rules enforce that `guard` would be dropped
        // as it leaves scope.
```

This signals developer intention much more clearly, and allows the
compiler to help the developer prevent a bug in the old code.

### Example: Force explicit variable lifetime for FFI.

Consider this example:

```rust
extern {
  fn set_callback(cb:|c_int|, state:*const c_void);
  fn check(a:c_int, b:c_int);
}

fn main() {
  let r = |x:c_int, data:*const c_void| {
    let foo:&mut Foo = transmute(data);
    foo.add(x as int);
    println!("{} was the output", x as int);
  };
  let data = Foo::new();
  unsafe { set_callback(r, &data as *const Foo as *const c_void); }
  for i in range(0, 10) {
    unsafe {
      check(10, i);
    }
  }
  // Now we must manually drop(data); and drop(r) here, othewise check will segfault.      
  // because data will already be dropped. 
}
```
([source](https://github.com/rust-lang/rfcs/pull/239#issuecomment-56261758))

Having the C FFI interact with rust structures requires an explicit
model of how the lifetime of rust structures that may cross the FFI
boundary interact with the lifetime of the C representations of those
structures. (In other words, both C and Rust need to have some
agreement about the lifetimes of shared data structures.) At present,
there is no way to explicitly enforce the relationship between the
lifetimes of two representations of the same data structure, so that
code like the above must rely on a deep understanding of Rust's and
C's allocation semantics in order to work correctly. A `linear` type
provides a means of documenting that variable lifetime has been
explicitly considered:

```rust
extern {
  fn set_callback(cb:|c_int|, state:*const c_void);
  fn check(a:c_int, b:c_int);
}

fn main() {
  let r = |x:c_int, data:*const c_void| {
    let foo:&mut Foo = transmute(data);
    foo.add(x as int);
    println!("{} was the output", x as int);
  };
  let r = LinearOf(r);
  let data = LinearOf(Foo::new());
  unsafe { set_callback(r.read(), data.read() as *const Foo as *const c_void); }
  for i in range(0, 10) {
    unsafe {
      check(10, i);
    }
  }
  // compilation will fail unless we manually drop(data); and drop(r) here.
  // using linear types prevented a segfault.
  //drop(r.release());
  //drop(data.release());
}
```

## Isn't this just like sprinkling `free()` calls through the code?

Sort of, but it's much safer than C's `free()`. There are two major
problems with explicit resource clean-up in C-like languages:

1. Failure to free.

2. Use after free.

This proposal continues to prevent both issues in rust:

1. The obligation that data be moved out of a `linear` type means that
it is impossible to fail to free resources (compilation will fail if
the `linear` value is not explicitly destructured for drop); AND

2. Rust's borrow-checker continues to enforce that use-after-free
is prevented.

This design is intended to bring back some *benefits* of explicit
resource management, without inflicting their costs.

## But linear types don't interact well with unwinding?

First, the `linear` attribute as described here does not create true
linear types: when unwinding past a `linear` type, the `linear`
attribute will be ignored, and a `Finalize` trait could be invoked.
Supporting unwinding means that Rust's `linear` types would in effect
still be affine. However, if we ever allow a post-1.0 subset of rust
without unwinding, Rust's `linear` types would become *true* linear
types.

Second, and probably more importantly, unwinding should be extremely
infrequent in rust code of any reasonable quality. As such, the linear
types as presented in this proposal, while not truely linear, are
probably within an epsilon of acting like true linear types in
practice.

# Detailed design

Note that the `linear bound` design was largely adapted from a [design
by
@eddyb](http://internals.rust-lang.org/t/pre-rfc-linear-type-modifier/1225/9),
while the `DropPtr` pointer type was inspired by [@nikomatsakis and
@glaebhoerl](https://github.com/rust-lang/rust/issues/10672#issuecomment-29939937).
Credit goes to these authors for the original ideas, while of course
any blame for misunderstanding or misusing these ideas is mine.

## The `Droppable` type-kind.

A linear type is represented in the compiler as a type that does NOT
have the `Droppable` property. `Droppable` is applied to all types,
except for types defined in one of the following ways:

1. Either the type is attached to the `linear_type` language item, OR
2. The type is a compound type, and at least one of the members of the
type has the `linear` bound.

The `Droppable` bound can be added from a container-type by having the
container implement `Drop`. (This will be described in more detail
below.)

To allow checking if a type is droppable, we also define a `Linear`
trait, associated with a new `linear_trait` language item. The
definition of these items in the `std::markers` crate will look like
this:

```rust
#[lang="droppable_trait"]
trait Droppable;
#[lang="linear_type"];
struct MakeLinear;
impl MakeLinear { ... }
```

Then defining a new linear type would look something like:

```rust
// is not droppable because it embeds the `MakeLinear` marker.
struct Foo {
    linear: std::markers::MakeLinear,
}
// is not droppable because it embeds `struct Foo`, which is not
// droppable.
struct Bar {
    foo: Foo,
}
```

Making a type droppable would look something like:

```rust
struct Baz {
    linear: std::markers::MakeLinear,
}
impl Drop for Baz {
    fn drop(DropPtr self) { ... }
}

// is droppable, because Baz is droppable.
struct Xyzzy {
    baz: Baz,
}
```

(Adding the droppable bound will be described in more detail, below.)

And checking if a type-parameter is linear would look like:

```rust
fn drop<T: Droppable>(_x: T) {}
fn id<T: ?Droppable>(x: T) -> T { x }
// same as `id` function, but only works on linear types.
fn linear_id<T: !Droppable>(x: T) -> T { x }
```

## The `linear` bound on variables.

In this design, any user-defined linear type must be a compound-type.
(The `linear_type` language item will apply to at most one type,
called MakeLinear above, so that any user-defined linear type must be
a compound-type including either `MakeLinear` itself, or a field that
ultimately includes `MakeLinear`.) A variable is considered linear if
either:

* The variable is of `MakeLinear` type, OR
* The variable is a compound type, and ultimately owns (or may own, in
the case of enums) a field of `MakeLinear` type.

So a compound variable is made linear by moving a linear field into
the variable, and is made non-linear by moving the linear field out.
The restriction against partial moves of container structures means
that receivers of the linear container can assume that the moved-in
variable will be linear on receive (since all owned fields, including
the linear fields, if any, must be populated).

The `MakeLinear` type is the "base" case, so we'll consider that
first. The `impl` for `MakeLinear` will be as follows:

```rust
// in std::markers:
impl MakeLinear {
    pub fn consume(self) {
        // the `linear_type` can only be dropped via the
        // `std::mem::forget` intrinsic.
        unsafe { std::mem::forget(self) }
    }
}
```

With this `impl` for `MakeLinear`, we can demonstrate how it would be
used in a linear fashion:

```rust
fn test_make_linear() {
    // after declaring an uninitialized MakeLinear variable:
    let x: MakeLinear;
    // `x` is not yet "linear", since it is uninitialized.

    // after initializing the variable:
    x = MakeLinear;
    // `x` is now linear, so that an attempt to implicitly drop x
    // would cause a compilation failure.

    // to drop a variable of `MakeLinear` type:
    x.consume();
    // `x` has been dropped, so compilation can succeed.
}
```

Compound types work similarly. Consider a `LinearOf` struct, which
wraps a variable to enforce that clean-up be explicit:

```rust
struct LinearOf<T> {
    el: T,
    linear: MakeLinear,
}
impl<T> LinearOf<T> {
    pub fn new(el: T) -> Self {
        // make a new instance of this structure. The new instance
        // will be linear because it will own a linear field.
        LinearOf { el: el, linear: MakeLinear }
    }
    pub fn release(self) -> T {
        // dispose of the `linear` field, to stop the compiler from
        // treating `self` as linear.
        self.linear.consume();
        // now that the `linear` field has moved out, self can be
        // implicitly dropped.
        self.el
    }
}
impl<T> Deref for LinearOf<T> {
    type Target = T;
    fn deref<'a>(&'a self) -> &'a T {
        &self.el
    }
}
impl<T> DerefMut for LinearOf<T> {
    fn deref_mut<'a>(&mut 'a self) -> &mut 'a T {
        &mut self.el
    }
}
```

This is a compound type, with one user-defined field, and a linear
field that makes the compiler treat fully-populated variables of this
type as linear.

Enums (such as `Option<T>`) may or may not own a variable, based on
the value of their discriminant. In the case that an enum *may* hold a
linear value, the compiler will require users to deconstruct the enum
in order to dispose of the value. For example:

```rust
impl<T: ?Droppable> Option<T> {
    // new function: behaves like `unwrap`, but for the None case.
    // Consumes self, panicking if the value is `Some`.
    pub fn unwrap_empty(self) {
        match self {
            None => (),
            Some(_) => panic!("Attempted to unwrap_empty full value"),
        }
    }
}
```

## Making a linear-type droppable.

As alluded to earlier, a linear type can be made droppable by having
the type implement the `Drop` trait. Unfortunately, this won't work in
current Rust, and (as far as I can tell) a fix involves changing the
signature of the `drop` method. (I will have more to say about this
below, under **Alternatives**.) The problem is, in this design, a
linear variable is made non-linear by a partial move: moving a linear
field out of a container suffices to make the container non-linear.
Since partial moves are disallowed for `Drop` types -- even during the
call to `drop` -- this means that any linear clean-up function (which
involves a move of the linear container) cannot be called from the
`drop` function body, so linear resource clean-up would be impossible.

We get around this limitation by defining a new `DropPtr` pointer
type, and changing the signature of `Drop::drop` to take
`DropPtr<Self>`, instead of `&mut self`. `DropPtr<T>` pointers act
like `&mut` pointers, with the additional behavior that partial moves
are allowed from the `DropPtr` pointer referent, that unmoved fields
will have their destructors called, and that the referent's memory
will be reclaimed some time after the `DropPtr` pointer goes out of
scope. For this design, we also must have the constraint that the
referent be made non-linear by the time the `DropPtr` pointer goes out
of scope. For example:

```rust
struct Foo(MakeLinear);
impl Drop for Foo {
    fn drop(self: DropPtr<Foo>) {
        // make `self` non-linear by consuming the `Linear` field.
        self.0.consume();
    }
}
```

When a `DropPtr` pointer goes out of scope, the referent's memory can
be reclaimed. Since consuming the `DropPtr` pointer will not invoke
the `drop` callback, creating a `DropPtr` pointer is an `unsafe`
operation (in the same way that `std::mem::forget` is unsafe):

```rust
// given the following:
struct Foo;
struct Bar(Foo);
fn drop_forget<T>(_x: DropPtr<T>) { }

// the following are legal:
let x = Foo;
drop_forget(unsafe { &x as DropPtr<_> });
// create an instance variable, but refer to it only through an
// `DropPtr` pointer:
let x = unsafe { &Foo as DropPtr<Foo> };
drop_forget(x);
let x = Bar(Foo, Foo);
drop_forget(unsafe { &x as DropPtr<Foo> });

let x = Bar(Foo);
drop_forget(unsafe { &x.0 as DropPtr<Foo> });
// the following line is illegal, since `x` is partially moved.
drop_forget(unsafe { &x as DropPtr<Bar> });
```

And, for completeness, we add a `std::mem::dropptr` method, which can
allow the `drop` hook to be called when invoked with a `DropPtr`
pointer:

```rust
// in std::mem:
fn dropptr<T: Droppable>(x: DropPtr<T>) { let _ = *x; }
```

## The `Finalize` trait.

It is possible to implicitly clean up a "Linear" type (as defined in
this RFC) through unwinding. In case some clean-up is necessary during
an unwind, we define a new `Finalize` trait with a `finalize` method,
which acts identically to the `drop` method of the `Drop` trait, but
which can will be reached via unwinding.

```rust
struct Foo(MakeLinear);
impl Finalize for Foo {
    fn finalize(self: DropPtr<Foo>) {
        self.0.consume();
    }
}
```

For `Drop` types, the default implementation of the `Finalize` trait
will be to use the `drop` method:

```rust
trait Finalize {
    fn finalize(self: DropPtr<Self>) {
        std::mem::dropptr(self);
    }
}
```

In this way, we've also allowed the `Drop` and `Finalize` traits to
reflect the two possible ways that a variable may go out of scope (the
"normal" return path is associated with `Drop`, and the "exceptional"
return path with `Finalize`), in case users want to use different code
in either case. (A reasonable application may be to have `Finalize`
trigger a process abort, or to allow `Drop` to perform clean-up that
would be inappropriate during unwinding, such as blocking on joining a
thread.)

## The `explicit_bounds` lint.

Since linear types cannot be implicitly dropped, any generic function
which includes implicit drops on an arbitrarily-typed variable must
fail to compile when parametrized with a variable of linear type. (In
other words, functions such as `std::mem::drop` should not accept
variables of linear type.) In order not to disturb backwards
compatibility too much, a type-parameter should default to assuming
`Droppable`, so that this definition of `std::mem::drop`:

```rust
fn drop<T>(_x: T) { }
```

would continue to work as before. However, a function akin to
Haskell's `id` function should be allowable with any type:

```rust
fn id<T: ?Sized + ?Droppable>(x: T) -> T { x }
```

Many parts of the standard library (such as `Option<T>` and `Vec<T>`)
should be updated to work with linear types, however manually updating
the APIs would likely lead to error. Fortunately, it should be
possible for the compiler to assist maintainers in determining what
the bounds of a given type-parameter should be: in the case of a
function like `drop`, that a variable of type-parameter `T` is
implicitly dropped in the function implementation can be understood by
the compiler to mean that `T` must be `Droppable`.

In order to make this sort of information available to maintainers,
we'll define an `explicit_bounds` lint, which can be used to inform
maintainers when the bounds on a type-parameter to a function are more
restrictive than necessary:

```rust
#[warn(explicit_bounds)]
fn id<T>(_x: T) -> T { x }
// will generate a compiler warning:
// Type parameter `T` to function `id` is more restrictive than
// necessary. Consider using `T: ?Sized + ?Droppable` instead?
```

## Update the standard library to be `Droppable`-aware.

There are many facilities in the standard library that could be used
with linear types, except that they currently have assumptions that
implicit drop is always allowable. For example, the `Option<T>` type
has routines like `unwrap_or`, which will result in an implicit drop
if the `Option` is `Some(x)` (in which case, the `or` parameter to the
function will be dropped). This can be addressed by splitting the
implementation of `Option` based on its type parameters:

```rust
impl<T: ?Droppable> Option<T> {
  pub fn is_some ...
  pub fn is_none ...
  pub fn as_ref ...
  pub fn as_mut ...
  pub fn as_mut_slice ...
  pub fn expect ...
  pub fn unwrap ...
  pub fn map ...
  ...etc...
}

impl<T: Droppable> Option<T> {
  pub fn unwrap_or ...
  pub fn unwrap_or_else ...
  pub fn map_or ...
  ...etc...
}
```

By compiling the standard library with the `explicit_bounds` lint
enabled, it will be possible to modify entities such as `Vec` and
`Option` so that different APIs will be available depending on the
bounds of their type parameters.

# Drawbacks

Overall, this proposes a significant change to the language, and there
are several pieces required to make the result usable and ergonomic.
Where new facilities felt necessary to improve the ergonomics of
working with linear types (`DropPtr` references in particular), I've
attempted to make those facilities more broadly useful, so that it
would be useful and meaningful to fold aspects of this proposal into
the language as parts. Most aspects of this proposal are intended to
be backwards compatible, so they would not need to be adopted before
the 1.0 release. (On the other hand, the `Drop::drop` API change may
not be backwards compatible, and therefore would want to be folded in
before 1.0, unless some alternative approach can be identified.)

Some aspects of this proposal push for change in some APIs. All such
changes, with the exception of the `Drop::drop` function signature,
would be backwards compatible, at least at a source level (I am not
familiar with rust's `.rlib` binary representation), however the
utility of many parts of the standard library (and other libraries)
would be improved under this proposal by separating those interfaces
which can apply to *all* type-parameter kinds, from those that can
only apply to non-linear kinds. (The text mentioned `Option<T>`
explicitly. Other types that would benefit from having a `?Droppable`
subset include `Vec<T>`, `Result<T>`, [T], etc.) This would cause some
churn in library implementations, but the modifications would
generally be highly mechanical. (Refactoring the `impl<T> Option<T>`
was perhaps the easiest part of this RFC to write.)

# Alternatives

## Do nothing.

We could not do this, and live with the status quo. I tried to show
why this is disadvantageous (and, in my opinion, highly
disadvantageous in some domains) in the Motivation above, but it is
certainly possible to live without linear types.

## `Linear` instead of `Droppable`.

This was the trait naming in an earlier draft of the proposal. In the
earlier draft, I had only considered `NonLinear` as a name for the
"can-be-implicitly-dropped" trait, which would have resulted in
double-negatives as developers read code like `fn something<T:
!NonLinear>(x: T)`. `Droppable` seems a better name, and better
reflects that allowing implicit drops is a facility that is added to a
linear type (a positive trait) than that implicit drops are removed
from an affine type in the old design.

## `Drop` and backwards compatibility.

As discussed above, this proposal makes a potentially breaking change
to the `Drop` method. This is, obviously, a widely-used method, so
that breaking this method seems to be a very expensive change, and
should be justified explicitly.

The most natural way to signal programmer intent that a linear bound
be removed seems (to me) to be by adding a `Drop` trait to a compound
structure that might contain a linear type:

* Linear bounds are characterized by disallowing implicit drops, while
the `Drop` trait is defined to execute some code when a variable is
implicitly dropped, so that it doesn't make sense for a type to be
both `!Droppable` and `Drop`.

* Since `Linear` types require explicit action in order to reclaim
linear resources, changing a type from `Linear` to `!Linear` will
require code execution to clean up the linear resource -- which is to
say, we'd need a `drop` routine to execute the linear-specific code to
clean up the linear resource. Which, of course, the `Drop` trait
provides.

So it seems like the `Drop` trait is a good way for a user to signify
that a type should be implicitly droppable. But, since partial moves
are currently impossible during the `drop` routine, and because
cleaning up `linear` variables requires moving the `linear` fields out
of the container (to make the container non-linear), a `Linear` type
cannot be cleaned up using the current `Drop::drop` function
signature. There are a few alternative approaches that occur to me,
and I'm actively interested in feedback here.

1. Modify the `Drop::drop` function signature, such that it can be
made to allow partial moves during the function invocation. (This is
the approach described above.)
2. Add a new trait, with identical meaning as `Drop`, but with a
better function signature.
3. Modify the `Drop` trait, to export a different routine that would
allow partial moves during the `drop` invocation. The default
implementation of this routine would invoke the current `drop` routine.
4. Allow coercion between `&mut` and `DropPtr` on the `drop` function
signature.

Of all of these approaches, I preferred the first listed, since it
feels less "bolted-on" to the language than the others described:
every current `Drop` implementor could mechanically replace `&mut`
with `DropPtr` in the `drop` function signature, and would continue to
work, while the `DropPtr` pointer itself adds significant extra
utility to the language, in a way that seems (to me) to fit with
Rust's philosophy. But of course, backwards-compatibility on this
scale is a strong counter-argument to this proposal, so that perhaps
even a more "bolted-on" facility may be considered preferable.

## Drop::drop argument type.

Once it was determined that no existing pointer type would satisfy the
requirements for this proposal, the obvious alternative was to use a
new pointer type. I tried to analyze ones that were already described,
but I could not find one that worked easily:

* The `&move` pointers described by @glaebhoerl and @nikomatsakis have
the behavior that the Drop routine will be invoked when the `&move`
pointer goes out of scope. This makes this pointer type impossible to
use inside the drop callback, since it goes out of scope with the drop
routine, which would imply that the routine would be invoked
recursively as its `&move` argument goes out of scope. So that
wouldn't work.

* @eddyb pointed me to his proposal for a new pointer facility
(tentatively called OpenPointer) that could likely be made to work for
my purposes. I personally like the proposal, but it is another design
dimension that I would prefer to avoid including in this proposal. On
the other hand, I've tried to make this proposal forward-compatible
with his.

* The syntax for creating and using `DropPtr` is definitely more
awkward than the `&mut self` currently used by the `Drop::drop`
callback. This syntax could be cleaned up with a new `&drop` pointer
type. I am not sure what the implications of this would be: obviously,
it would be inappropriate to make `drop` a keyword in the syntax.
Still, it would be possible in the future to change the syntax so that
`&drop T` could desugar to `DropPtr<T>`, or this syntax could be
neatened by allowing `fn drop(DropPtr<self>) {}` as a self-argument to
a method. I believe a `DropPtr` type would be forward compatible with
future language evolution to simplify this syntax.

## Use inference for the `Droppable` bound on function type-parameters.

In [an earlier draft of this
proposal](http://internals.rust-lang.org/t/pre-rfc-linear-types-take-2/1323),
I suggested that the default `Linear` bound on generic type parameters
to a function could be inferred. It was later pointed out to me that
this would make the externally-visible function signature fragile, in
that a type-parameter that had been inferred to be linear may end up
changing to non-linear (or vice-versa) as a result of routine
maintenance. This seems undesirable, and goes against the Rust spirit
of making function signatures completely explicit in terms of what
types of argument they can take. Inference is still useful, so this
draft moves the bounds-inference logic to a lint.

# Unresolved questions

None that I can think of.

# Acknowledgments

I'd like to thank the commentors on internals.rust-lang.org for their
patience in helping me identify and work through some of the corner
cases in this design, and for helping me understand more of the Rust
philosophy. In particular, I'd like to thank @eddyb for his design of
the basic linear types mechanism, and for his feedback while iterating
this design. I believe this design would have been much weaker without
his help. I'd also like to thank @glaebhoerl for his help in
understanding some of the new proposed pointer types.
