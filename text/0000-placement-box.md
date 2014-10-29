- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[Summary]: #summary

Add user-defined placement `in` expression (more succinctly, "an `in`
expression"), an operator analogous to "placement new" in C++.  This
provides a way for a user to specify (1.) how the backing storage for
some datum should be allocated, (2.) that the allocation should be
ordered before the evaluation of the datum, and (3.) that the datum
should preferably be stored directly into the backing storage (rather
than allocating temporary storage on the stack and then copying the
datum from the stack into the backing storage).

This RFC does not suggest any change to the non-placement `box`
expression special form (`box <value-expr>`); a future RFC is planned
that will suggest changes to that form based on preliminary feedback
to this RFC, but that is orthogonal to this RFC's goals.

# Table of Contents
[Table of Contents]: #table-of-contents
* [Summary]
* [Table of Contents]
* [Motivation]
  * [Why ptr::write is not sufficient]
  * [Doing allocation prior to value construction]
  * [Placement `in` as an (overloaded) operator]
  * [Failure handling]
  * [Summary of motivations]
* [Detailed design]
  * [Section 1: Syntax]
  * [Section 2: Semantics]
  * [Section 3: API]
    * [API Example: Box]
    * [API Example: Vec emplace_back]
* [Drawbacks]
* [Alternatives]
  * [Same semantics, but different surface syntax]
  * [Just desugar into once-functions, not implementations of traits]
  * [Get rid of `Placer` and just use `PlacementAgent` trait]
* [Unresolved questions]

# Motivation
[Motivation]: #motivation

As put by Bjarne Stroustrup, "The Design and Evolution of C++":

> Two related problems were solved by a common mechanism:
>
> 1. We needed a mechanism for placing an object at a specific address,
>    for example, placing an object representing a process at the address
>    required by special-purpose hardware.
>
> 2. We needed a mechanism for allocating objects from a specific arena,
>    for example, for allocating an object in the shared memory of a
>    multi-processor or from an arena controlled by a persistent object
>    manager.

In C++, the solution was overload the pre-existing `new` operator with
an additional "`new (buf) X`" form known as "the placement syntax",
where the static type of the `buf` input dictates which overloaded
variant is used.

We also want to solve the above two problems in Rust. Moreover, we
want to do so in an efficient manner (i.e., we want our generated code
to be competitive with that of C++).

## Why ptr write is not sufficient
[Why ptr::write is not sufficient]: #why-ptr-write-is-not-sufficient

Today, one can emulate both goals (1.)  and (2.) above by using unsafe
native pointers and copying already constructed values into the target
addresses via unsafe code. In particular, one can write code like
this, which takes an input value, allocates a place for it, and writes
it into place.


```rust
fn allocate_t(t: T) -> MyBoxOfTee {
    unsafe {
        let place = heap::allocate(mem::size_of::<T>(), mem::align_of::<T>());
        let place : *mut T = mem::transmute(place);

        // `place` is uninitialized; don't run dtor (if any) for T at place.
        ptr::write(place, t);

        MyBoxOfTee { my_t: place }
    }
}
```

However, this is inefficient; using this API requires that one write
code like this `allocate_t(make_value())`, which to the `rustc`
compiler means something like this:

```rust
    let value : T = make_value();
    let my_box = allocate_t(value);
```

This is not ideal: it is creating a temporary value for `T` on the
stack, and then copies it into the final target location once it has
been allocated within the body of `allocate_t`.  Even if the compiler
manages to inline the body of `allocate_t` into the callsite above, it
will still be quite difficult for it to get rid of the intermdiate
stack-allocation and the copy, because to do so requires moving the
`heap::allocate` call up above the `make_value()` call, which usually
will not be considered a semantics-preserving transformation by the
`rustc` compiler and its LLVM backend.

A general solution to this problem requires finding some way to ensure
that the evaluation of the `make_value()` call will occur *after* the
`heap::allocate` call made by the implementation of `allocate_t`.

## Doing allocation prior to value construction
[Doing allocation prior to value construction]: #doing-allocation-prior-to-value-construction

The Rust development team has known about the above problem for a long
time; long enough that it established the `box` expression syntax
ahead of time to deal with this exact problem (though with much debate
about what exact syntax to use):

* [rust meeting October 2013]

* [rust-dev November 2013]

* [rust-dev December 2013]

* [rust-dev July 2014]

[rust meeting October 2013]: https://github.com/rust-lang/meeting-minutes/blob/master/weekly-meetings/2013-10-29.md#placement-new
[rust-dev November 2013]: https://mail.mozilla.org/pipermail/rust-dev/2013-November/006997.html
[rust-dev December 2013]: https://mail.mozilla.org/pipermail/rust-dev/2013-December/007084.html
[rust-dev July 2014]: https://mail.mozilla.org/pipermail/rust-dev/2014-July/010677.html

The point is to provide *some* syntactic special form that takes two
expressions: a `<place-expr>` and `<value-expr>`.  (For context, in
`rustc` today one writes the special form as `box (<place-expr>)
<value-expr>`.)  The `<place-expr>` dicates the allocation of the
backing storage (as well as the type used as a handle to the allocated
value, if any); the `<value-expr>` dicates the actual datum to store
into the backing storage.

We need to provide *both* expressions to the special form because we
need to do the allocation of the backing storage before we evalute
`<value-expr>`, but (for safety in the general case) we want to ensure
that the resulting value is written into the backing storage
(initializing it) before the address of the backing storage leaks to
any code that assumes it to be initialized.

## Placement `in` as an (overloaded) operator
[Placement `in` as an (overloaded) operator]: #placement-in-as-an-overloaded-operator

While the original motivation for adding placement new in C++ was for
placing objects at specific memory addresses, the C++ design was also
made general enough so that the operation could be overloaded:
statically dispatched at compile-time based on the type of the target
place. This allows for distinct allocation protocols to be expressed
via a single syntax, where the static type of `<place-expr>` indicates
a particular allocation method.  Rust can also benefit from such a
generalization, as illustrated by some examples adapted from
[rust meeting October 2013]:

```rust
// (used below)
let mut arena : Arena = ...;
let mut my_vector : Vec<Foo> = ...;

// Allocates an `RcBox<Thing>` (includes ref-count word on backing
// storage), initializes associated payload to result of evaluating
// `Thing::init("hello")` and returns an `Rc<Thing>`. Uses singleton
// (static) constant `RC` to signal this allocation protocol.
let my_thing : Rc<Thing> = box(in RC) Thing::init("hello");

// Allocates an entry in a locally-allocated arena, then stores result
// of `Foo::new()` into it, returning shared pointer into the arena.
let a_foo : &Foo = box(in arena) Foo::new()

// Adds an uninitialized entry to the end of the `Vec`, then stores
// result of `Foo::new()` into it, returning `()`, or perhaps the
// `uint` of the associated index. (It is a library design detail;
// this system should support either.)
box(my_vector.emplace_back()) Foo::new()
```

In addition, one can imagine a further generalization of the arena
example to support full-blown [reaps], a form of arena that supports
both owned references (where the associated memory can be given back
to the reap and reallocated) and shared references.

[reaps]: http://people.cs.umass.edu/~emery/pubs/berger-oopsla2002.pdf

```
let reap : reap::Reap = ...;

let a_foo : &Foo = {
    // both of afoo_{1,2} own their respective handles into the reap
    let afoo_1 : reap::OwningRef<Foo> = box(reap) Foo::new();
    let afoo_2 : reap::OwningRef<Foo> = box(reap) Foo::new();

    let shared_foo : &Foo = afoo_1.to_shared();
    shared_foo // here storage of afoo_2 is returned to the reap
};

...
```

## Failure handling
[Failure handling]: #failure-handling

One final detail: Rust supports task failure, and thus the placement `in` syntax
needs to make sure it has a sane story for properly unwinding its intermediate
state if the `<value-expr>` fails.

To be concrete: we established in
[Doing allocation prior to value construction] that we *must* do the
allocation of the backing storage before we start evaluating
`<value-expr>`.  But this implies that if `<value-expr>` causes a
failure, then we are also responsible for properly deallocating that
backing storage, or otherwise tearing down any state that was set up
as part of that allocation.

## Summary of motivations
[Summary of motivations]: #summary-of-motivations

So the goals are:

1. Support evaluating a value into a specific address, without an
   intermediate copy,

2. Provide user-extensible access to the syntax, so that one can
   control the types and executed code for both (2a.) the backing
   storage of the value, and (2b.) the resulting wrapper type
   representing the handle (this "handle" may be either owned or
   shared, depending on the design of library), and

3. Still handle task failure/panic properly.

# Detailed design
[Detailed design]: #detailed-design

The presentation of the design is broken into the following sections.

* Section 1, Syntax: A placement `box` syntax `box (<place-expr>)
  <value-expr>` has been in place within `rustc` for a while, but
  remains a point of contention for some, and thus is worth teasing
  out from the other parts.

  This RFC proposes using the slightly different syntax
  `in (<place-expr>) <value-expr>`, to further distinguish the
  placement form from the non-placement `box <value-expr>` form.

  (Note that the [Alternatives] section does discuss some of the
  alternative high-level syntaxes that have been proposed.)

* Section 2, Semantics: The placement `in` semantics, as observed by
  a client of the syntax using it with whatever standard library types
  support it. This section should be entirely uncontroversial, as it
  follows essentially from the goals given in the motivation section.

* Section 3, API: The method for library types to provide their own
  overloads of the placement `in` syntax.  In keeping with how other
  operators in Rust are handled today, this RFC specifies a trait that
  one implements to provide an operator overload.  However, due to the
  special needs of the placement `in` protocol as described in
  "Section 2, Semantics", this trait is a little more complicated to
  implement than most of the others in `core::ops`.

## Section 1: Syntax
[Section 1: Syntax]: #section-1-syntax

As stated in [Doing allocation prior to value construction], we need a
special form that takes a `<place-expr>` and a `<value-expr>`.

This RFC suggests using the form:

`in (<place-expr>) <value-expr>`

with the special case:

  * if you want the default owned box `Box<T>` (which allocates a `T`
    on the inter-task exchange heap), you can use the shorthand
    `in () <value-expr>`, omitting the `<place-expr>`.

----

Note: The form that was merged in [Rust PR 10929] takes the following form:

[Rust PR 10929]: https://github.com/rust-lang/rust/pull/10929

`box (<place-expr>) <value-expr>`

with the following special cases:

  * if you want the default owned box `Box<T>` (which allocates a `T`
    on the inter-task exchange heap), you can use the shorthand
    `box () <value-expr>`, omiting the `<place-expr>`.

  * as an additional special case for `Box<T>`: if `<value-expr>` has no
    surrounding parentheses (i.e. if `<value-expr>` starts with a token
    other than `(`), then you can use the shorthand `box <value-expr>`.

(The combination of the above two shorthands do imply that if for some
strange reason you want to create a boxed instance of the unit value
`()`, i.e. a value of type `Box<()>`, you need to write either `box ()
()`, or `box (HEAP) ()` where `HEAP` is an identifier that evaluates
to the inter-task exchange heap's placer value.)

The [semantics][Section 2: Semantics] and [API][Section 3: API]
described below are entirely compatible with the present day
`box (<place-expr>) <value-expr>` syntax; it should be trivial to
adopt any of the
[alternative syntaxes][Same semantics, but different surface syntax].
The bulk of this RFC is dedicated to the semantics and API, because
that is of foremost importance.

## Section 2: Semantics
[Section 2: Semantics]: #section-2-semantics

The type of the `<place-expr>` indicates what kind of box should be
constructed to hold the result of evaluating `<value-expr>`.

From the viewpoint of a end programmer, the semantics of `box
(<place-expr>) <value-expr>` is meant to be:

  1. Evaluate `<place-expr>` first (call the resulting value `PLACE`).
  2. Perform an operation on `PLACE` that allocates the backing storage
     for the `<value-expr>` and extracts the associated native pointer
     `ADDR` to the storage.
  3. Evaluate the `<value-expr>` and write the result directly into the
     backing storage.
  4. Convert the `PLACE` and `ADDR` to a final value of appropriate type;
     this final associated type is derived from the static types of
     `<place-expr>` and `<value-expr>`.

(Note that it is possible that in some instances of the protocol that
`PLACE` and `ADDR` may be the same value, or at least in a one-to-one
mapping.)

In addition, if a failure occurs during step 3, then instead of
proceeding with step 4, we instead deallocate the backing storage as
part of unwinding the continuation (i.e. stack) associated with the
placement `box` expression.

Examples of valid `<place-expr>` that will be provided by the standard
library:

 * The global constant `std::boxed::HEAP` allocates backing storage
   from the inter-task exchange heap, yielding `Box<T>`.

 * The global constant `std::rc::RC` allocates task-local backing
   storage from some heap and adds reference count meta-data to the
   payload, yielding `Rc<T>`

 * It seems likely we would also provide a `Vec::emplace_back(&mut
   self)` method (illustrated in the example code in
   [Placement `box` as an (overloaded) operator]), which allocates
   backing storage from the receiver vector, and returns `()`.  (Or
   perhaps `uint`; the point is that it does *not* return an owning
   reference.)

In addition, the `libarena` crate will probably provide support for
writing `in (arena_ref) <value-expr>`, (where `arena_ref: &Arena` or
`arena_ref: &TypedArena<T>` and `<value-expr>: T`), which will first
allocate backing storage from the referenced arena (which is likely to
have lifetime bounded by a stack frame) and then evaluate
`<value-expr>` into it.

## Section 3: API
[Section 3: API]: #section-3-api

How a library provides its own overload of placement `in`.

The standard library provides two new traits in `core::ops`:


```rust
/// Interface to user-implementations of `in (<placer_expr>) <value_expr>`.
///
/// `in (P) V` effectively expands into:
/// `{ let b = P.make_place();
///    let raw_place = b.pointer();
///    let v = V;
///    unsafe { ptr::write(raw_place, v); b.finalize() }
///  }`
///
/// An instance of `Interim` is transient mutable value; an instance
/// of `Placer` may *also* be some transient mutable value, but the
/// placer could also be an immutable constant that implements `Copy`.
pub trait Placer<Sized? Data, Owner, Interim: PlacementAgent<Data, Owner>> {
    /// Allocates a place for the data to live, returning an
    /// intermediate agent to negotiation build-up or tear-down.
    fn make_place(self) -> Interim;
}

/// Helper trait for expansion of `in (P) V`.
///
/// A placement agent can be thought of as a special representation
/// for a hypothetical `&uninit` reference (which Rust cannot
/// currently express directly). That is, it represents a pointer to
/// uninitialized storage.
///
/// The client is responsible for two steps: First, initializing the
/// payload (it can access its address via the `pointer()`
/// method). Second, converting the agent to an instance of the owning
/// pointer, via the `finalize()` method.
///
/// See also `Placer`.
pub trait PlacementAgent<Sized? Data, Owner> {
    /// Returns a pointer to the offset in the place where the data lives.
    fn pointer(&mut self) -> *mut Data;

    /// Converts this intermediate agent into owning pointer for the data.
    ///
    /// Note: successful result evaluation and conversion implies that the
    /// agent no longer owns the value's backing storage, and therefore
    /// should not deallocate the backing storage when `self` is dropped.
    /// Implementations of the `finalize` method can accomplish this,
    /// e.g. by calling `std::mem::forget` on self.
    unsafe fn finalize(self) -> Owner;
}
```

The necessity of a `Placer` trait of some form to hook into the
operator syntax should be unsuprising. The `Placer` trait has a single
`make_place` method, which is the first thing invoked by the placement
`in` operator. The interesting aspects of `Placer` are:

 * It has three type parameters.  The first two are the `Data` being
   stored and the `Owner` that will be returned in the end by the box
   expression; the need for these two follows from
   [Placement `box` as an (overloaded) operator].
   The third type parameter is used for the return value of `make_place`,
   which we explain next.

 * The return value of `make_place` is a so-called "interim agent"
   with two methods.  It is effectively an `&uninit` reference:
   i.e. it represents a pointer to uninitialized storage.

The library designer is responsible for implementing the two traits
above in a manner compatible with the hypothetical (hygienic) syntax
expansion:

```rust
            in (P) V
               ==>
    { let b = P.make_place(); let v = V; unsafe { b.pointer() = v; b.finalize() } }
```

In addition, any type implementing `PlacementAgent` is likely to want
also to implement `Drop`. The way that this design provides [Failure
handling] is to couple any necessary cleanup code with the `drop` for
the interim agent.  Of course, when doing this, one will also want to
ensure that such `drop` code does *not* run at the end of a call to
`PlacementAgent::finalize(self)`; that is why the documentation for
that method states that one might choose to forget self (as in
`mem::forget(self);`), in order to ensure that its destructor is not
run. (Of course one can use other methods to avoid the failure-cleanup
path, such as by setting a boolean flag within the agent.)

The following two examples are taken from a concrete prototype
implementation of the above API in [Rust PR 18233].

[Rust PR 18233]: https://github.com/rust-lang/rust/pull/18233


### API Example: Box
[API Example: Box]: #api-example-box

Here is an adaptation of code to go into `alloc::boxed` to work with this API.

```rust
/// This the singleton type used solely for `boxed::HEAP`.
pub struct ExchangeHeapSingleton { _force_singleton: () }

pub const HEAP: ExchangeHeapSingleton =
    ExchangeHeapSingleton { _force_singleton: () };

pub struct Box<Sized? T>(*mut T);

pub struct IntermediateBox<Sized? T>{
    ptr: *mut u8,
    size: uint,
    align: uint,
}

impl<T> Placer<T, Box<T>, IntermediateBox<T>> for ExchangeHeapSingleton {
    fn make_place(self) -> IntermediateBox<T> {
        let size = mem::size_of::<T>();
        let align = mem::align_of::<T>();

        let p = if size == 0 {
            heap::EMPTY as *mut u8
        } else {
            let p = unsafe {
                heap::allocate(size, align)
            };
            if p.is_null() {
                panic!("Box make_place allocation failure.");
            }
            p
        };

        IntermediateBox { ptr: p, size: size, align: align }
    }
}

impl<Sized? T> PlacementAgent<T, Box<T>> for IntermediateBox<T> {
    fn pointer(&mut self) -> *mut T {
        self.ptr as *mut T
    }
    unsafe fn finalize(self) -> Box<T> {
        let p = self.ptr as *mut T;
        mem::forget(self);
        mem::transmute(p)
    }
}

#[unsafe_destructor]
impl<Sized? T> Drop for IntermediateBox<T> {
    fn drop(&mut self) {
        if self.size > 0 {
            unsafe {
                heap::deallocate(self.ptr, self.size, self.align)
            }
        }
    }
}
```

Of interest is the use of `const HEAP: ExchangeHeapSingleton = ...` to
define the `HEAP` item: since `ExchangeHeapSingleton` is `Copy` and
the `HEAP` item is defined via `const`, we can use `HEAP` as an
r-value and thus call methods like `Placer::make_place` that take
`self` by-value.

### API Example: Vec emplace_back
[API Example: Vec emplace_back]: #api-example-vec-emplace_back

On the flip side, a `Vec::emplace_back` placer shows how to use the
Placer API to directly evaluate an expression into a target cell
within a vector object; it follows the example established by
[Placement `in` as an (overloaded) operator], where the
`emplace_back()` method returns a temporary value that 

```rust
struct EmplaceBack<'a, T:'a> {
    vec: &'a mut Vec<T>,
}

// The requirement to make this `pub` may serve as a new motivation
// for priv trait methods...
pub struct EmplaceBackAgent<'a, T:'a> {
    vec_ptr: &'a mut Vec<T>,
    offset: uint,
}

pub trait EmplaceBack<'a, T> {
    fn emplace_back(&'a mut self) -> EmplaceBackPlacer<'a, T>;
}

impl<'a, T:'a> EmplaceBack<'a, T> for Vec<T> {
    fn emplace_back(&'a mut self) -> EmplaceBackPlacer<'a, T> {
        EmplaceBackPlacer { vec: self }
    }
}

impl<'a, T:'a> Placer<T, (), EmplaceBackAgent<'a, T>> for EmplaceBackPlacer<'a, T> {
    fn make_place(self) -> EmplaceBackAgent<'a, T> {
        let len = self.vec.len();
        EmplaceBackAgent { vec_ptr: self.vec, offset: len }
    }
}

impl<'a, T> PlacementAgent<T, ()> for EmplaceBackAgent<'a, T> {
    fn pointer(&mut self) -> *mut T {
        let len = self.vec_ptr.len();
        self.vec_ptr.reserve_additional(1);
        assert_eq!(self.vec_ptr.len(), self.offset);
        assert!(self.offset < self.vec_ptr.capacity());
        unsafe {
            self.vec_ptr.as_mut_ptr().offset(self.offset.to_int().unwrap())
        }
    }

    unsafe fn finalize(self) -> () {
        assert_eq!((*self.vec_ptr).len(), self.offset);
        assert!(self.offset < (*self.vec_ptr).capacity());
        self.vec_ptr.set_len(self.offset + 1);
    }
}

#[unsafe_destructor]
impl<'a, T> Drop for EmplaceBackAgent<'a, T> {
    fn drop(&mut self) {
        // Do not need to do anything; all `make_place` did was ensure
        // we had some space reserved, it did not touch the state of
        // the vector itself.
    }
}
```

# Drawbacks
[Drawbacks]: #drawbacks

We have been getting by without user-defined box so far, so one might
argue that we do not need it now.  My suspicion is that people do want
support for the features listed in the motivation section (and are
just waiting patitently under the assumption that we are going to add
it given our past discussions).

In fact, as the [rust-dev December 2013] thread was winding down,
pcwalton [pointed out](https://mail.mozilla.org/pipermail/rust-dev/2013-December/007142.html):

>  Rust and C++ are different. You don't use placement `new` for
> `shared_ptr` in C++; however, you will use placement `new` (or `box`)
> for `RC` in Rust (the equivalent). For this reason I suspect that
> placement `new` will be much commoner in Rust than in C++.

# Alternatives
[Alternatives]: #alternatives

## Same semantics, but different surface syntax
[Same semantics, but different surface syntax]: #same-semantics-but-different-surface-syntax

There were a variety of suggestions listed on
[rust-dev November 2013]
[rust-dev December 2013]
[rust-dev July 2014].

In addition, [RFC Issue 405] provides yet another list of alternatives.

[RFC Issue 405]: https://github.com/rust-lang/rfcs/issues/405

The complaints listed on [RFC Issue 405] are largely about the use
of parentheses in the syntax especially given its special cases
(as described in [Section 1: Syntax]).

Some example alternatives, (many taken from [RFC Issue 405]):

 * `box (<place-expr>) <value-expr>` ("i.e. present day form")
 * `box (in <place-expr>) <value-expr>` ("box-in placement")
 * `box <value-expr> in <place-expr>` ("paren-free placement, `in` on rhs")
 * `box::(<place-expr>) <value-expr>` ("box colon colon")
 * `in <place-expr> box <value-expr>` ("paren-free placement, `box` on rhs")
 * `in (<place-expr>) <value-expr>` ("just `in` placement"; this RFC)

The author (Felix) personally wants to keep the `<place-expr>`
lexically to the left of `<value-expr>`, to remind the reader about
the evaluation order.  But perhaps `in <place-expr> box <value-expr>`
would be alternative acceptable to the author of [RFC Issue 405]?

As a reminder: the form `in <place-expr> <value-expr>` would not work,
because of parsing ambiguities such as whether `in a - b ...` is
parsed as `in (a - b) ...` (and the parser state is "waiting for a
`<value-expr>` in the `...`") or as `in (a) (-b) ...` (and the parser
state is "have now read a complete placement `in` expression").

Also, [nikomatsakis on discuss] seems to promote the "just `in`
placement" form (`in (<place-expr>) <value-expr>`). That is the main
reason I wrote the RFC using the "just `in` placement" form (rather
than "box-in placement" `box (in <place-expr>) <value-expr>` form). I
am not terribly attached to any particular syntax for the placement
`in` expression.

[nikomatsakis on discuss]: http://discuss.rust-lang.org/t/pre-rfc-placement-box-with-placer-trait/729/7

## Just desugar into once-functions, not implementations of traits
[Just desugar into once-functions, not implementations of traits]: #just-desugar-into-once-functions-not-implementations-of-traits

At the [rust meeting August 2014], the team thought we could do a
simpler desugaring that would wrap the `<value-expr>` in a
once-function closure; this way, you would still force the expected
order-of-evaluation (do the allocation first, then run the closure,
letting return value optimization handle writing the result into the
backing storage).

[rust meeting August 2014]: https://github.com/rust-lang/meeting-minutes/blob/master/workweek-2014-08-18/box-and-allocators.md#2014-08-21-box

The most obvious place where this completely falls down is that it does not
do the right thing for something like this:

```rust
let b: PBox<T> = box (place) try!(run_code())
```

because under the once-function desugaring, that gets turned into something
like:

```rust
place.make_box(|once: | -> T { try!(run_code()) })
```

which will not do the right thing when `run_code` returns an `Err`
variant, and in fact will not even type-check.

So the only way to make the once-function desugaring work would be to
either severely restrict the kinds of expressions that "work" as the
`<value-expr>`, or to make the desugaring a much more invasive
transformation of the `<value-expr>`; neither of these options is
really palatable.

## Get rid of `Placer` and just use `PlacementAgent` trait
[Get rid of `Placer` and just use `PlacementAgent` trait]: #get-rid-of-placer-and-just-use-placementagent-trait

Hypothetically, if we are willing to revise our API's for interacting
with placement `box`, then we could get rid of the `Placer` trait.

In particular, if we change the protocol so that `<place-expr>` itself
is responsible for returning an instance of `PlacementAgent`, then we
would not need `Placer` at all.

This would mean we could no longer write `in (rc::RC) <value-expr>` or
`in (boxed::HEAP) <value-expr>`; it would have to be something like
`in (rc::RC()) <value-expr>` and `in (boxed::HEAP()) <value-expr>`,
at best.  Many usages of the placement `in` expression would be more
complicated.

The [API Example: Vec emplace_back] would be slightly simplified
(since the body of `fn make_place` will just be folded into the body
of `fn emplace_back`).

In short, I am not sure this would be much of a real win. It seems it
would provide a small boon to library designers but a burden to many
library users.

Nota bene: you *cannot* take the alternative tack of removing the
`PlacementAgent` and just using a `Placer` trait -- you need the agent
because it carries the destructor code to run if you encounter failure
while evaluating the `<value-expr>`.

# Unresolved questions
[Unresolved questions]: #unresolved-questions

* Is there a significant benefit to building in linguistic support for `&uninit` rather
  than having the protocol rely on the unsafe methods in `PlacementAgent`?
