- Feature Name: `place_traits`
- Start Date: 2026-01-23
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/3921)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary
[summary]: #summary

This RFC introduces the `Place` trait. This trait allows arbitrary types to implement the
special derefence behavior of the `Box` type. In particular, it allows an arbitrary type
to act as an owned place allowing values to be (partially) moved out and moved back in
again.

## Motivation
[motivation]: #motivation

Currently the Box type is uniquely special in the rust ecosystem. It is unique in acting
like an owned variable, and allows a number of special optimizations for direct
instantiation of other types in the storage allocated by it.

This special status comes with two challenges. First of all, Box gets its special status
by being deeply interwoven with the compiler. This is somewhat problematic as it requires
exactly matched definitions of how the type looks between various parts of the compiler
and the standard library. Moving box over to a place trait would provide a more pleasant
and straightforward interface between the compiler and the box type, at least in regards
to the move behavior.

Second, it is currently impossible to provide a safe interface for user-defined smart
pointer types which provide the option of moving data in and out of it. There have been
identified a number of places where such functionality could be interesting, such as when
removing values from containers, or when building custom smart pointer types for example
in the context of an implementation of garbage collection.

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This proposal introduces a new unsafe trait `Place`:
```rust
unsafe trait Place: DerefMut {
    fn place(&self) -> *const Self::Target;
    fn place_mut(&mut self) -> *mut Self::Target;
}
```

The `Place` trait essentially allows values of the type to be treated as an already-
existing box. That is, they behave like a variable of the type `Deref::Target`, just
stored in a different location than the stack. This means that values of type
`Deref::Target` can be (partially) moved in and out of dereferences of the type, with the
borrow checker ensuring soundness of the resulting code. As an example, if `Foo`
implements `Place` for type `Bar`, the following would become valid rust code:
```rust
fn baz(mut x: Foo) -> Foo {
    let y = *x;
    *x = y.destructive_update()
    x
}
```

When implementing this trait, the type itself effectively transfers some of the responsibilities for managing the value behind the pointer returned by `Place::place`, also called the content, to the compiler. In particular, the type itself should no longer count on the ccontent being properly initialized and dropable when its `Drop` implementation or `Place::place` implementation is called. However, the compiler still guarantees that, as long as the type implementing the place is always created with a value in it, and that value is never removed through a different mechanism than dereferencing the type, all other calls to member functions can assume the value to be implemented.

In general, the compilers requirements are met when
- The pointer returned by `place` should be safe to mutate through, and should be live
  for the lifetime of the mutable reference to `self` passed to `Place::place`.
- On consecutive calls to `Place::place`, the status of whether the content is initialized should not be changed.
- Drop must not drop the contents, only the storage for it.
- Newly initialized values of the type implementing `Place` must have their content initialized.

There is one oddity in the behavior of types implementing `Place` to be aware of.
Automatically elaborated dereferences of values of such types will always trigger an abort
on panic, instead of unwinding when that is enabled. However, generic types constrained to
only implement Deref or DerefMut but not Place will always unwind on panics during
dereferencing, even if the underlying type also implements Place.

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This proposal introduces one new main language item, the traits `Place`. We also introduce a number of secondary language items which are used to make implementation easier and more robust, which we shall define as they come up below.

A type implementing the trait Place is required to act as a place for borrow checking. Throughout the rest of this text, the contents of the memory pointed at by the pointer returned by the `Place::place` function shall be refered to as the content of the place. For a type to satisfy the above requirement, its implementation must in particular guarantee that
- Safe code shall not modify the initialization status of the contents.
- Unsafe code shall preserve the initialization status of the contents between two derefences of teh type's values.
- Values of the place type for which the content is uninitialized shall not be able to be created in safe code.
In the above context, the contents is also considered uniitialized if the whole or parts of the value of the contents has been moved out, or a destructor has been called upon them.

Dereferences of a type implementing `Place` can therefore be lowered directly to MIR, only
being elaborated in a pass after borrow checking. This allows the borrow checker to fully
check that the moves of data into and out of the type are valid.

The dereferences and drops of the contained value can then be elaborated in the passes
after borrow checking. This process will be somewhat similar to what is already done for
Box, with the difference that dereferences of types implementing `Place` may panic. We
propose to handle these panics by aborting to avoid introducing interactions with drop
elaboration and new execution paths not checked by the borrow checker.

In order to generate the function calls to the `Place::place` and `Deref::deref` during
the dereference elaboration we propose making these functions additional language items.

## Drawbacks
[drawbacks]: #drawbacks

There are three main drawbacks to the design as outlined above. First, the traits are
unsafe and come with quite an extensive list of requirements on the implementing type.
This makes them relatively tricky and risky to implement, as breaking the requirements
could result in undefined behavior that is difficult to find.

Second, with the current design the underlying type is no longer aware of whether or not
the space it has allocated for the value is populated or not. This inhibits functionality
which would use this information on drop to automate removal from a container. Note
however that such usecases can use a workaround with the user explicitly requesting
removal before being able to move out of a smart-pointer like type.

Finally, the type does not have runtime-awareness of when the value is exactly added.
This means that the proposed traits are not suitable for providing transparent locking of
shared variables to end user code.

In past proposals for similar traits it has also been noted that AutoDeref is complicated
and poorly understood by most users. It could therefore be considered problematic that
AutoDeref behavior is extended. However the behavior here is identical to what Box already
has, which is considered acceptable in its current state.

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Ideas for something like the `Place` trait design here can be found in past discussions of
DerefMove traits and move references. The desire for some way of doing move derefences
goes back to at least https://github.com/rust-lang/rfcs/issues/997.

The rationale behind the current design is that it explicitly sticks very closely to what
is already implemented for Boxes, which in turn closely mirror what can be done with stack
variables directly. This provides a relatively straightforward mental model for the user,
and significantly reduces the risk that the proposed design runs into issues in the
implementation phase.

### DerefMove trait

Designs based on a simpler DerefMove trait have been previously proposed in the unmerged
[RFC2439](https://github.com/rust-lang/rfcs/pull/2439) and an [internals forum thread](https://internals.rust-lang.org/t/derefmove-without-move-why-dont-we-have-it/19701).
These come down to a trait of the form
```
trait DerefMove : DerefMut {
    fn deref_move(self) -> Self::Target
}
```

The disadvantage of an approach like this is that it is somewhat unclear how to deal with
partial moves. This has in the past stopped such proposals in their tracks.

Furthermore, such a trait does not by itself cover the entirety of the functionality
offered by Box, and given its consuming nature it is unclear how to extend it. This also
leads to the potential for backwards incompatible changes to the current behavior of Box,
as has previously been identified.

### &move based solutions

A separate class of solutions has been proposed based on the idea of adding &move
references to the type system, where the reference owns the value, but not the allocation
behind the value. These were discussed in an [unsubmitted RFC by arielb1](https://github.com/arielb1/rfcs/blob/missing-derefs/text/0000-missing-derefs.md),
and an [internals forum thread](https://internals.rust-lang.org/t/pre-rfc-move-references/14511)

Drawbacks of this approach have been indicated as the significant extra complexity which
is added to the type system with the extra type of reference. There further seems to be a
need for subtypes of move references to ensure values are moved in or out before dropping
to properly keep the allocation initialized or deinitialized after use as needed.

This additional complexity leads to a lot more moving parts in this approach, which
although the result has the potential to allow a bit more flexibility makes them less
attractive on the whole.

### More complicated place traits

Several more complicated `Place` traits have been proposed by tema2 in two threads on the
internals forum:
- [DerefMove without `&move` refs](https://internals.rust-lang.org/t/derefmove-without-move-refs/17575)
- [`DerefMove` as two separate traits](https://internals.rust-lang.org/t/derefmove-as-two-separate-traits/16031)

These traits aimed at providing more feedback with regards to the length of use of the
pointer returned by the `Place::place` method, and the status of the value in that
location after use. Such a design would open up more possible use cases, but at the cost
of significantly more complicated desugarings.

Furthermore, allowing actions based on whether a value is present or not in the place
would add additional complexity in understanding the control flow of the resulting binary.
This could make understanding uses of these traits significantly more difficult for end
users of types implement these traits.

### Limited macro based trait

Going the other way in terms of complexity, a `Place` trait with constraints on how the
projection to the actual location to be dereferenced was proposed in [another internals forum thread](https://internals.rust-lang.org/t/derefmove-without-move-references-aka-box-magic-for-user-types/19910).

This proposal effectively constrains the `Place::deref` method to only doing field
projections and other dereferences. The advantage of this is that such a trait has far
less severe safety implications, and by its nature cannot panic making its use more
predictable.

However, the restrictions require additional custom syntax for specifying the precise
process, which adds complexity to the language and makes the trait a bit of an outlier
compared to the `Deref` and `DerefMut` traits.

### Existing library based solutions

The [moveit library](https://docs.rs/moveit/latest/moveit/index.html) provides similar
functionality in its `DerefMove` trait. However, this requires unsafe code on the part of
the end user of the trait, which makes them unattractive for developers wanting the
memory safety guarantees rust provides.

## Prior art
[prior-art]: #prior-art

The behavior enabled by the trait proposed here is already implemented for the Box type,
which can be considered prime prior art. Experience with the Box type has shown that its
special behaviors have applications.

Beyond rust, there aren't really comparable features that map directly onto the proposal
here. C++ has the option for smart pointers through overloading dereferencing operators,
and implements move semantics through Move constructors and Move assignment operators.
However, moves in C++ require the moved-out of place to always remain containing a valid
value as there is no intrinsic language-level way of dealing with moved-out of places in
a special way.

In terms of implementability, a small experiment has been done implementing the deref
elaboration for an earlier version of this trait at
https://github.com/davidv1992/rust/tree/place-experiment. That implementation is
sufficiently far along to support running code using the Place trait, but does not yet
properly drop the internal value, instead leaking it.

## Unresolved questions
[unresolved-questions]: #unresolved-questions

Right now, the design states that panic in calls to `Place::place` or `Place::place_mut` can
cause an abort when the call was generated in the MIR. This is done as it is at this point
somewhat unclear how to handle proper unwinding at the call sites for these functions.
However, it may turn out to be possible to implement this with proper unwinding, in which
case we may want to consider handling panics at these call sites the same as for ordinary
code.

## Future possibilities
[future-possibilities]: #future-possibilities

Should the trait become stabilized, it may become interesting to implement non-copying
variants of the various pop functions on containers within the standard library. Such
functions could allow significant optimizations when used in combination with large
elements in the container.

It may also be interesting at a future point to reconsider whether the unsized_fn_params
trait should remain internal, in particular once Unsized coercions become usable with user
defined types. However, this decision can be delayed to a later date as sufficiently many
interesting use cases are already available without it.

Finally, there is potential for the trait as presented here to become useful in the in
place initialization project. It could be a building block for generalizing things like
partial initialization to smart pointers. This would require future design around an api
for telling the borrow checker about new empty values implementing Place, but that seems
orthogonal to the design here.
