- Feature Name: `union_initialization_and_drop`
- Start Date: 2018-08-03
- RFC PR: [rust-lang/rfcs#2514](https://github.com/rust-lang/rfcs/pull/2514)
- Rust Issue: [rust-lang/rust#55149](https://github.com/rust-lang/rust/issues/55149)

# Summary
[summary]: #summary

Unions do not allow fields of types that require drop glue (the code that is
automatically run when a variables goes out of scope: recursively dropping the
variable and all its fields), but they may still `impl Drop` themselves.  We
specify when one may move out of a union field and when the union's `drop` is
called.  To avoid undesired implicit calls of drop, we also restrict the use of
`DerefMut` when unions are involved.

# Motivation
[motivation]: #motivation

Currently, it is unstable to have a non-`Copy` field in the union.  The main
reason for this is that having fields which need drop glue raises some hard
questions about whether to call that drop glue when assigning a union field, and
how to make programming with such unions less of a time bomb (triggered by
accidentally dropping data one meant to just overwrite).  Not much progress has
been made on stabilizing the unstable union features.  This RFC proposes a route
forwards that side-steps the time bomb: Do not allow fields with drop glue.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## Union Definition

When defining a union, it is a hard error to use a field type that requires drop glue.
Examples:
```rust
// Accepted
union Example1<T> {
    // `ManuallyDrop<T>` never has drop glue, even if `T` does.
    f1: ManuallyDrop<T>,
    // `RefCell<i32>` is a fully known type, and does not have drop glue.
    f2: RefCell<i32>,
}
union Example2<T: Copy> {
    // `Copy` types never have drop glue.
    f1: T,
}
trait Trait3 { type Assoc: Copy; }
union Example3<T: Trait3> {
    // `T::Assoc` is `Copy` and hence cannot have drop glue.
    f1: T::Assoc,
}

// Rejected
union Example4<T> {
    // `T` might have drop glue, and then `RefCell<T>` would as well.
    f1: RefCell<T>,
}
trait Trait5 { type Assoc; }
union Example5<T: Trait5> {
    // `T::Assoc` might have drop glue.
    f1: T::Assoc,
}
```

Ruling out possibly dropping types may seem restrictive, but thanks to
`ManuallyDrop` it in fact is not: If the compiler rejects a union definition,
you can always wrap field types in `ManuallyDrop` to obtain a working
definition.  This means you have to manually take care of when to drop the data,
but that is already something to be concerned with when working on unions.

As a consequence, it is quite obvious that writing to a union field will never
implicitly call `drop`.  Such a write is hence always a safe operation.  This
removes a whole class of pitfalls related to `drop` being called in tricky
unsafe code when you might not expect that to happen.  (However, see below for
some pitfalls that remain.)

Reading from a union field and creating a reference remain unsafe: We cannot
guarantee that the field contains valid data.

## Union initialization and `Drop`

In two cases, the compiler cares about whether a (field of a) variable is
initialized: When deciding whether a move from the field/variable is allowed
(for cases where the type is not `Copy`), and when deciding whether or not the
variable has to be dropped when it goes out of scope.

A union just does very simple initialization tracking: There is a single boolean
state for the entire union and all of its fields.  Nested inner fields are
tracked just like they are for structs; however, when the union becomes
(un)initialized, then all nested inner fields of all union fields are
(un)initialized at once.  So, (un)initializing a union field also
(un)initializes its siblings.  For example:

```rust
// This code creates bad references and transmutes to `Vec` in incorrect ways.
// This is just to demonstrate what the compiler would accept in terms of
// tracking initialization.

struct S(i32); // not `Copy`, no drop glue
union U { f1: ManuallyDrop<Vec<i32>>, f2: (S, S), f3: i32 }

let mut u: U;
// Now `u` is not initialized: `&u`, `&u.f2` and `&u.f2.0` are all rejected.

// We can write into uninitialized inner fields:
u.f2.1 = S(42);
{ let _x = &u.f2.1; } // This field is initialized now.
// But this does not change the initialization state of the union itself,
// or any other (inner) field.

// We can initialize by assigning an entire field:
u.f1 = ManuallyDrop::new(Vec::new());
// Now *all (nested) fields* of `u` are initialized, including the siblings of `f1`:
{ let _x = &u.f2; }
{ let _x = &u.f2.0; }

// Equivalently, we can assign the entire union:
u = U { f2: (S(42), S(23) };
// Now `u` is still initialized.

// Copying does not change anything:
let _x = u.f3;
// Now `u` is still initialized.

// We can move out of an initialized union:
let v = u.f1;
// Now `f1` *and its siblings* are no longer initialized (they got "moved out of"):
// `let _x = u.f2;` would hence get rejected, as would `&u.f1` and `foo(u)`.
u.f1 = v;
// Now `u` and all of its fields are initialized again ("moving back in").

// When we move out of an inner field, the other union fields become uninitialized
// even if they are `Copy`.
let s = u.f2.1;
// Now `u.f1` and `u.f3` are no longer initialized.  But `u.f2.0` is:
let s = u.f2.0;
```

If the union implements `Drop`, the same restrictions as for structs apply: It
is not possible to initialize a field before initializing the entire variable,
and it is not possible to move out of a field.  For example:

```rust
// This code creates bad references and transmutes to `Vec` in incorrect ways.
// This is just to demonstrate what the compiler would accept in terms of
// tracking initialization.

struct S(i32); // not `Copy`, no drop glue

union U { f1: ManuallyDrop<Vec<i32>>, f2: (S, S), f3: u32 }
impl Drop for U {
    fn drop(&mut self) {
        println!("Goodbye!");
    }
}

let mut u: U;
// `u.f1 = ...;` gets rejected: Cannot initialize a union with `Drop` by assigning a field.
u = U { f2: (S(42), S(1)) };
// Now `u` is initialized.

// `let v = u.f1;` gets rejected: Cannot move out of union that implements `Drop`.
let v_ref = &mut u.f1; // creating a reference is allowed
let _x = u.f3; // copying out is allowed
```

When a union implementing `Drop` goes out of scope, its destructor gets called if and only if the union is currently considered initialized:
(Continuing the example from above.)

```rust
{
    let u = U { f2: (S(0), S(1)) };
    // drop gets called
}
{
    let u = U { f1: ManuallyDrop::new(Vec::new()) };
    foo(u);
    // drop does NOT get called
}
```

## Potential pitfalls around `DerefMut`

There is still a potential pitfall left around assigning to union fields: If the
assignment implicitly happens through a `DerefMut`, it may call drop glue.  For
example:

```rust
#![feature(untagged_unions)]

use std::mem::ManuallyDrop;

union U<T> { x:(), f: ManuallyDrop<T> }

fn main() {
    let mut u : U<(Vec<i32>,)> = U { x: () };
    unsafe { u.f.0 = Vec::new() }; // uninitialized `Vec` being dropped
}
```
This requires `unsafe` because it desugars to `ManuallyDrop::deref_mut(&mut u.f).0`,
and while writing to a union field is safe, taking a reference is not.

For this reason, `DerefMut` auto-deref is not applied when working on a union or
its fields.  However, note that manually dereferencing is still possible, so
`(*u.f).0 = Vec::new()` is still a way to drop an uninitialized field!  But this
can never happen when no `*` is involved, and hopefully dereferencing an element
of a union is a clear enough signal that the union better be initialized
properly for this to make sense.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Union definition

When defining a union, it is a hard error to use a field type that requires drop glue.
This is checked as follows:

* Proceed recursively down the given type, insofar as the type involved is known
  at compile-time.  For example, `u32`, `&mut T` and `ManuallyDrop<T>` are known
  to not have drop glue no matter the choice of `T`.
* When hitting a type variable where no progress can be made, check that `T:
  Copy` as a proxy for `T` not requiring drop glue.

Note: Currently, union fields with drop glue are allowed on nightly with an
unstable feature.  This RFC proposes to remove support for that entirely; code using
nightly might have to be changed.

## Writing to union fields

Writing to union fields is currently unsafe when the field has drop glue.  This
check is no longer needed, because union fields will never have drop glue.
Moreover, writing to a nested field (e.g., `u.f1.x = 0;`) is currently unsafe as
well, this should also become a safe operation as long as the path (expanded,
i.e., after auto-derefs are inserted) consists *only of field projections, not
deref's*.  Note that this is sound only because `ManuallyDrop`'s only field is
private (so, in fact, this is *not* sound inside the module that defines
`ManuallyDrop`).

## Union initialization tracking

A "fragment" is a place of the form `local_var.field.field.field`, without any
implicit derefs.  A fragment can be either *initialized* or *uninitialized*.
This state is approximated statically: The type system will only allow accesses
to definitely initialized fragments.  Drop elaboration needs to know the precise
state of a fragment, for which purpose it adds run-time drop flags as needed.

If a fragment has some uninitialized nested fragments then it is still
uninitialized and accesses to this fragment as a whole are prevented. This
applies even if it also has a nested initialized fragment (in which case we speak
of a *partially initialized* fragment).  If a fragment has only initialized
nested fragments then it is initialized as a whole and can be accessed.

A fragment becomes initialized when it is assigned to, or created using an
initializer, or it is a union field and a sibling becomes initialized, or all
its nested fragments become initialized.  A fragment becomes uninitialized when
it doesn't implement `Copy` and is moved out from, or it is a union field
(possibly `Copy`) and its sibling becomes uninitialized, or some of its nested
fragments becomes uninitialized.

In other words, union fields behave a lot like struct fields except that if one
field changes initialization state, the others follow suit.  In particular, if
one union field becomes partially initialized (because one of its nested
fragments got uninitialized), all its siblings become *entirely* uninitialized,
including their nested fragments.

If a fragment is of a type which has an `impl Drop`, then its nested fragments
cannot be separately (un)initialized.  Only the entire fragment can be
initialized by assignment, and the entire fragment can be uninitialized by
moving out.

NOTE: To my knowledge, this already mostly matches the current
implementation. The only exception is that "fragment becomes initialized when
all its nested fragments become initialized" rule is not currently implemented
for neither structs nor unions, so the compiler accepts less code than it
should.  However, `impl Drop for Union` and non-`Copy` union fields are behind a
feature gate, so the effects of this on unions cannot currently be observed on
stable compilers.

(This closely follows a
[previously proposed RFC by @petrochenkov](https://github.com/petrochenkov/rfcs/blob/e5266bd105f592f7408b8592c5c3deaccba7f1ec/text/1444-union.md#initialization-state).)

## Potential pitfalls around `DerefMut`

When adding auto-derefs on the left-hand side of an assignment, as we traverse
the path, once we hit a `union`, we stop adding further auto-derefs.  So with
`s: Struct` and `u: Union`, when encountering `s.u.f.x`, auto-deref *does*
happen on `s`, but not on `s.u` or any of the later components.

Notice that this relies crucially on the only field of `ManuallyDrop` being
private!  If we could project directly through that field, no `DerefMut` would
be needed to reproduce the problematic example from the "guide" section.

# Drawbacks
[drawbacks]: #drawbacks

This makes working with unions involving types that may have drop glue slightly
more verbose than today: One has to write `ManuallyDrop` more often than one may
want to.

The restriction placed on `DerefMut` is not fully backwards compatible: A type
could implement `Copy + DerefMut` and actually rely on the deref coercion inside
a union.  That seems very unlikely, but should be tested with a crater run.

The initialization tracking rules are somewhat surprising, and one might prefer
the compiler to just not track anything when it comes to unions.  After all, the
compiler fundamentally cannot know what part of the union is properly
initialized.  Unfortunately, not having any initialization tracking is not an
option when non-`Copy` fields are involved: We have to decide if moving out of a
union field is allowed.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Ruling out fields with drop glue does not, in fact, reduce the expressiveness of
unions because one can use `ManuallyDrop<T>` to obtain a drop-glue-free version
of `T`.  If anything, having the `ManuallyDrop` in the union definition should
help to drive home the point that no automatic dropping is happening, ever.
(Before this RFC, automatic dropping is happening when assigning to a union
field but not when the union goes out of scope.  That seems to be the result of
necessity, not of a coherent design.)

An alternative approach to proceed with unions has been
[previously proposed by @petrochenkov](https://github.com/petrochenkov/rfcs/blob/e5266bd105f592f7408b8592c5c3deaccba7f1ec/text/1444-union.md#initialization-state).
That proposal replaces RFC 1444 and goes into a lot more points than this much more
limited proposal.  In particular, it allows fields with drop glue.  However, it
can be pretty hard for the programmer to predict when drop glue will be
automatically invoked on assignment or not, because the initialization tracking
(which this RFC adapts from @petrochenkov's proposal) can sometimes be a little
surprising when looking at individual fields: Whether `u.f2 = ...;` drops
depends on whether `u.f1` has been previously initialized.  We hence
have a lint to warn people that unions with drop-glue fields are not always
very well-behaved.  This RFC, on the other hand, side-steps the entire question
by not allowing fields with drop glue.  Initialization tracking thus has no
effect on the code executed during an assignment of a union field.  For unions
that `impl Drop`, it still has an effect on what happens when the union goes out
of scope, but in that case initialization is so restricted that I cannot think
of any surprises.  Together with the `DerefMut` restriction, that should make it
very unlikely to accidentally call `drop` when it was not intended.

We could significantly simplify the initialization tracking by always applying
the rules that are currently only applied to unions that `impl Drop`.  However,
that does not actually help with the pitfall described above.  The more complex
rules allow more code that many will reasonably expect to work, and do not seem
to introduce any additional pitfalls.

We could reduce the relevance of state tracking further by not to allowing `impl
Drop for Union`.  It is still possible to add a wrapper struct around the union
which has drop glue, so this does not restrict expressiveness.  However, this
seems unnecessarily cumbersome, and it does not seem to help avoid any
surprises.  State tracking around unions that `impl Drop` is pretty much as
simple as it gets.

# Prior art
[prior-art]: #prior-art

I do not know of any language combining initialization tracking and destructors
with unions: C++ [never runs destructors for fields of unions][cpp_union_drop],
and it does not track whether fields of a data structures are initialized to
(dis)allow references or moves.

[cpp_union_drop]: https://en.cppreference.com/w/cpp/language/union

# Unresolved questions
[unresolved-questions]: #unresolved-questions

Should we even try to avoid the `DerefMut`-related pitfall?  And if yes, should
we maybe try harder, e.g. lint against using `*` below a union type when
describing a place?  That would make people write `let v = &mut u.f; *v =
Vec::new();`.  It is not clear that this helps in terms of pointing out that an
automatic drop may be happening.

We could allow moving out of a union field even if it implements `Drop`.  That
would have the effect of making the union considered uninitialized, i.e., it
would not be dropped implicitly when it goes out of scope.  However, it might be
useful to not let people do this accidentally.  The same effect can always be
achieved by having a dropless union wrapped in a newtype `struct` with the
desired `Drop`.
