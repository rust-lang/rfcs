- Start Date: 2015-01-28
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Add a `linear-trait` language item, that will be applied to a `Linear`
trait, that can be used to determine if a type can be treated as
Linear. Add a `linear-type` language item, that will be applied to a
`MakeLinear` unit struct, which will be used to virally infect
container-types with the `Linear` trait. The compiler will refuse to
compile a source file in which a `linear` variable would be implicitly
dropped. A `linear` variable can be explicitly dropped by either
making it non-linear (by moving contained linear fields out), or by
using the `std::mem::forget` intrinsic. Add a `&move` pointer type, to
allow partial moves out of a container as it is being dropped. Add a
`Finalize` trait, that behaves identically to `Drop`, but can be
applied to linear types to clean up during unwinding. Add an
`explicit_bounds` lint that will require that generic type parameters
for an `impl` have their bounds specified. Compile the standard
libraries with the `explicit_bounds` lint on, and update as many APIs
as make sense to be Linear-aware.

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
while the `&move` pointer type was originally described by
[@nikomatsakis and
@glaebhoerl](https://github.com/rust-lang/rust/issues/10672#issuecomment-29939937).
Credit goes to these authors for the original ideas, while of course
any blame for misunderstanding or misusing these ideas is mine alone.

## The `Linear` bound on types.

A linear type is represented in the compiler as a type that has the
`BoundLinear` bound associated with it. This bound is applied to a
type in one of two ways:

1. Either the type is attached to the `linear_type` language item, OR
2. The type is a compound type, and at least one of the members of the
type has the `linear` bound.

The `Linear` bound can be removed from a container-type by having the
container implement `Drop`. (This will be described in more detail
below.)

To allow checking if a type is linear, we also define a `Linear`
trait, associated with a new `linear_trait` language item. The
definition of these items in the `std::markers` crate will look like
this:

```rust
#[lang="linear_trait"]
trait Linear;
#[lang="linear_type"];
struct MakeLinear;
impl MakeLinear { ... }
```

(Note that I have not added `#[derive(Linear)]`, or `impl Linear for
MakeLinear` to explicitly annotate that the `MakeLinear` structure
will be of `Linear` type. It is considered important in this design
that the compiler decide which types are to be treated as linear by
construction, so that an explicit annotation from a user that a type
should be linear is unnecessary and undesired.)

Then defining a new linear type would look something like:

```rust
// has linear bound because it embeds the `MakeLinear` marker.
struct Foo {
    linear: std::markers::MakeLinear,
}
// has linear bound because it embeds `struct Foo` that has a linear
// bound.
struct Bar {
    foo: Foo,
}
```

Removing the linear bound would look something like:

```rust
struct Baz {
    linear: std::markers::MakeLinear,
}
impl Drop for Baz {
    fn drop(&move self) { ... }
}

// does not have linear bound, because Baz does not have linear bound.
struct Xyzzy {
    baz: Baz,
}
```

(Removing the linear bound will be described in more detail, below.)

And checking if a type-parameter is linear would look like:

```rust
fn drop<T: !Linear>(_x: T) {}
fn id<T: ?Linear>(x: T) -> T { x }
// same as `id` function, but only works on linear types.
fn linear_id<T: Linear>(x: T) -> T { x }
```

This, of course, depends on negative trait bounds working. An
alternative design would be to use a positive bounds check on a
NonLinear trait. I'll discuss this possibility under the
**Alternatives** heading, below.

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
impl<T: ?Linear> Option<T> {
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

## Removing a linear bound from a type.

As alluded to earlier, a linear bound can be removed from a type by
having the type implement the `Drop` trait. Unfortunately, this won't
work in current Rust, and (as far as I can tell) a fix involves
changing the signature of the `drop` method. (I will have more to say
about this below, under **Alternatives**.) The problem is, in this
design, a linear variable is made non-linear by a partial move: moving
a linear field out of a container suffices to make the container
non-linear. Since partial moves are disallowed for `Drop` types --
even during the call to `drop` -- this means that any linear clean-up
function (which involves a move of the linear container) cannot be
called from the `drop` function body, so linear resource clean-up
would be impossible.

We get around this limitation by using the `&move` pointer type
[originally described by @nikomatsakis and
@glaebhoerl](https://github.com/rust-lang/rust/issues/10672#issuecomment-29939937),
and changing the signature of `Drop::drop` to take `&move self`,
instead of `&mut self`. The original discussion of `&move` pointers
can be found at the link, the discussion here attempts to cover the
design of how these pointers could be introduced to the language. (I
apologize if there is another design document describing these
pointers, I could not easily find it.) `&move` pointers act like
`&mut` pointers, with the additional behavior that partial moves are
allowed from the `&move` pointer referent, and that the referent's
memory will be reclaimed some time after the `&move` pointer goes out
of scope. In this design, we also add the constraint that the referent
be made non-linear by the time the `&move` pointer goes out of scope.
For example:

```rust
struct Foo(MakeLinear);
impl Drop for Foo {
    fn drop(&move self) {
        // make `self` non-linear by consuming the `Linear` field.
        self.0.consume();
    }
}
```

When a `&move` pointer goes out of scope, the referent's memory can be
reclaimed. Since consuming the `&move` pointer will not invoke the
`drop` callback, creating a `&move` pointer is an `unsafe` operation
(in the same way that `std::mem::forget` is unsafe):

```rust
// given the following:
struct Foo;
struct Bar(Foo);
fn drop_forget<T>(_x: &move T) { }

// the following are legal:
let x = Foo;
drop_forget(unsafe { &move x });
// create an instance variable, but refer to it only through an
// `&move` pointer:
let x = unsafe { &move Foo };
drop_forget(x);
let x = Bar(Foo, Foo);
drop_forget(unsafe { &move x });

let x = Bar(Foo);
drop_forget(unsafe { &move x.0 });
// the following line is illegal, since `x` is partially moved.
drop_forget(unsafe { &move x });
```

In fact, it would *almost* be possible to implement `std::mem::forget`
with a `&move` pointer (except that then this routine could not be
invoked against a `MakeLinear` variable):

```rust
// in std::mem:
unsafe fn forget<T>(x: T) { unsafe { let _ = &move x; } }
```

And, for completeness, we add a `std::mem::dropptr` method, which can
allow the `drop` hook to be called when invoked with a `&move`
pointer:

```rust
// in std::mem:
fn dropptr<T: !Linear>(x: &move T) { let _ = *x; }
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
    fn finalize(&move self) {
        self.0.consume();
    }
}
```

For `Drop` types, the default implementation of the `Finalize` trait
will be to use the `drop` method:

```rust
trait Finalize {
    fn finalize(&move self) {
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
`!Linear`, so that this definition of `std::mem::drop`:

```rust
fn drop<T>(_x: T) { }
```

would continue to work as before. However, a function akin to
Haskell's `id` function should be allowable with any type:

```rust
fn id<T: ?Sized + ?Linear>(x: T) -> T { x }
```

Many parts of the standard library (such as `Option<T>` and `Vec<T>`)
should be updated to work with linear types, however manually updating
the APIs would likely lead to error. Fortunately, it should be
possible for the compiler to assist maintainers in determining what
the bounds of a given type-parameter should be: in the case of a
function like `drop`, that a variable of type-parameter `T` is
implicitly dropped in the function implementation can be understood by
the compiler to mean that `T` cannot be `Linear`.

In order to make this sort of information available to maintainers,
we'll define an `explicit_bounds` lint, which can be used to inform
maintainers when the bounds on a type-parameter to a function are more
restrictive than necessary:

```rust
#[warn(explicit_bounds)]
fn id<T>(_x: T) -> T { x }
// will generate a compiler warning:
// Type parameter `T` to function `id` is more restrictive than
// necessary. Consider using `T: ?Sized + ?Linear` instead?
```

## Update the standard library to be `Linear`-aware.

There are many facilities in the standard library that could be used
with Linear types, except that they currently have assumptions that
implicit drop is always allowable. For example, the `Option<T>` type
has routines like `unwrap_or`, which will result in an implicit drop
if the `Option` is `Some(x)` (in which case, the `or` parameter to the
function will be dropped). This can be addressed by splitting the
implementation of `Option` based on its type parameters:

```rust
impl<T: ?Linear> Option<T> {
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

impl<T: !Linear> Option<T> {
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
working with linear types (`&move` references in particular), I've
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
explicitly. Other types that would benefit from having a `Linear`
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

## `NonLinear` instead of `Linear`.

As noted above, this full design requires negative trait bounds, since
it will often be the case that a user wants to write code with the
bounds that a type-parameter is non-linear. (For example,
`std::mem::drop` must enforce that its type parameter is non-linear.)
This design requires that negative trait bounds work. An alternative
design could use a bound named `NonLinear` instead of `Linear`, so
that the drop routine could be written as:

```rust
fn drop<T: NonLinear>(_x: T) { }
```

As a human reader, this means that the `drop` function is making a
positive assertion that the type is not linear. This isn't that hard
to read in this case, but if one considers `?NonLinear` (type may or
may not be linear) or `!NonLinear` (type is linear), then confusion
quickly becomes possible: a reader will always spend some cognitive
effort to translate from the negative "NonLinear" to a positive
"Linear" form. This is accidental complexity that should be avoided.

On the other hand, perhaps there is a better name than "NonLinear" for
the trait we are describing? Perhaps "Droppable"? I still personally
prefer "Linear", even though it means some of the implementation must
wait for negative traits to land, but perhaps others see things
differently.

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
both `Linear` and `Drop`.

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
4. Allow coercion between `&mut` and `&move` on the `drop` function
signature.

Of all of these approaches, I preferred the first listed, since it
feels less "bolted-on" to the language than the others described:
every current `Drop` implementor could mechanically replace `&mut`
with `&move` in the `drop` function signature, and would continue to
work, while the `&move` pointer itself adds significant extra utility
to the language, in a way that seems (to me) to fit with Rust's
philosophy. But of course, backwards-compatibility on this scale is a
strong counter-argument to this proposal, so that perhaps even a more
"bolted-on" facility may be considered preferable.

## Use inference for the `Linear` bound on a function type-parameter.

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
the basic linear types mechanism, and @glaebhoerl for his help in
understanding `&move` pointers.
