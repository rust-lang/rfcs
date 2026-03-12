- Feature Name: `deref_move_trait`
- Start Date: 2026-01-23
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/3921)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary
[summary]: #summary

This RFC introduces the `DerefMove` trait. This trait allows arbitrary types to implement the
special dereference behavior of the `Box` type. In particular, it allows an arbitrary type
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

This proposal introduces a new unsafe trait `DerefMove`:
```rust
unsafe trait DerefMove: DerefMut {
    fn place(&self) -> *const Self::Target;
    fn place_mut(&mut self) -> *mut Self::Target;
}
```

The `DerefMove` trait allows values of the type to be treated as a `Box` with a content. That
is, they behave like a variable of type `Deref::Target`, just (potentially) stored in a
different location than the stack. The contents can be (partially) moved in and out of
dereferences of the type, with the borrow checker ensuring soundness of the resulting
code. As an example, if `Foo` implements `DerefMove` for type `Bar`, the following would be
valid rust code:
```rust
fn baz(mut x: Foo) -> Foo {
    let y = *x;
    *x = y.destructive_update()
    x
}
```

In implementing the `DerefMove` trait, the type transfers responsibility for managing its
content to the compiler. In particular, the type should not assume the contents are
initialized when `DerefMove::place` and `DerefMove::place_mut` are called.

It also transfers responsibility for dropping the contents to the compiler. This will
be done in drop glue, and the `Drop::drop` function will therefore see the contents as
uninitialized.

The transfer of responsibility also puts requirements on the implementer of `DerefMove`. In
particular, instances of the type where the contents are uninitialized through a means
other than moving out of a dereference should not be created. Breaking this rule and
dereferencing the resulting instance is immediate undefined behavior.

There is one oddity in the behavior of types implementing `DerefMove` to be aware of.
Automatically elaborated dereferences of values of such types will always trigger an abort
on panic, instead of unwinding when that is enabled. However, generic types constrained to
only implement `Deref` or `DerefMut` but not `DerefMove` will always unwind on panics during
dereferencing, even if the underlying type also implements `DerefMove`.

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This proposal introduces one new main language item, the traits `DerefMove`. We also introduce
a number of secondary language items which are used to make implementation easier and more
robust, which we shall define as they come up below.

Instances of a type implementing the trait `DerefMove` shall provide a "contents" of type
`Deref::Target`, which shall act as a place for borrow checking. The implementer of the
`DerefMove` trait shall ensure that:
- The `DerefMove::place` and `DerefMove::place_mut` return pointers to the storage of the content
  valid for the lifetime of the self reference.
- The `DerefMove::place` and `DerefMove::place_mut` functions shall not assume the contents to
  be initialized
- The `Drop::drop` function shall not interact with the content, but only with its storage.
- With the exception of uninitialization through moving out of a dereference of the instance,
  the implementer shall ensure that the contents are always initialized when dereferencing
  or dropping instances of the type.

In return, the compiler shall guarantee that:
- When the contents are uninitialized through moving out of a dereference of an instance,
  references to that instance will only be available through invocations of `DerefMove::place`,
  `DerefMove::place_mut` and `Drop::drop` by the compiler itself.
- The contents shall be dropped or moved out of by the compiler before invoking `Drop::drop`
  of the instance.

Furthermore, as there seems to be no good reasons for `DerefMove::place` and `DerefMove::place_mut`
to panic, the compiler shall handle panics in non-explicit calls to these functions by
aborting. This allows the compiler to compile code more efficiently and provides more
avenues for optimizations.

Note that the above requirements explicitly allow changes in the storage location of the
contents. This is only restricted by the contract imposed by `DerefMove` during the lifetime
of self pointers passed to `DerefMove::place` and `DerefMove::place_mut`.

As a consequence, `Pin<Foo>` does not automatically satisfy all the requirements of `Pin`
when Foo implements `DerefMove`. Whether a function transforming `Foo` to `Pin<Foo>` is safe
will have to be checked manually by the implementer of `Foo` using the requirements imposed
by `Pin`.

### Implementation details

Given the above contract, dereferences of a type implementing `DerefMove` can be lowered
directly to MIR, only being elaborated after borrow checking. This allows the borrow
checker and drop elaboration logic to provide the guarantees above, and ensure that
dereferences are sound in the presence of moves out of the contents.

The dereferences and drops of the contained value can be elaborated in the passes after
borrow checking. This process will be somewhat similar to what is already done for Box,
with the difference that dereferences of types implementing `DerefMove` may panic. These
panics will be handled by aborting, avoiding significant changes in the control flow graph.

In order to generate the function calls to the `DerefMove::place` and `DerefMove::place_mut`
during the dereference elaboration we propose making these functions additional language
items.

## Drawbacks
[drawbacks]: #drawbacks

There are three main drawbacks to the design as outlined above. First, the traits are
unsafe and come with quite an extensive list of requirements on the implementing type.
This makes them relatively tricky and risky to implement, as breaking the requirements
could result in undefined behavior that is difficult to find.

Second, with the current design the underlying type is no longer aware of whether or not
the space it has allocated for the value is populated or not. This inhibits functionality
which would use this information on drop to automate removal from a container. Note
however that such use cases can use a workaround with the user explicitly requesting
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

Ideas for something like the `DerefMove` trait design here can be found in past discussions of
`DerefMove` traits and move references. The desire for some way of doing move dereferences
goes back to at least https://github.com/rust-lang/rfcs/issues/997.

The rationale behind the current design is that it explicitly sticks very closely to what
is already implemented for Boxes, which in turn closely mirror what can be done with stack
variables directly. This provides a relatively straightforward mental model for the user,
and significantly reduces the risk that the proposed design runs into issues in the
implementation phase.

### DerefMove trait

Designs based on a simpler `DerefMove` trait have been previously proposed in the unmerged
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

Several more complicated `DerefMove` traits have been proposed by tema2 in two threads on the
internals forum:
- [DerefMove without `&move` refs](https://internals.rust-lang.org/t/derefmove-without-move-refs/17575)
- [`DerefMove` as two separate traits](https://internals.rust-lang.org/t/derefmove-as-two-separate-traits/16031)

These traits aimed at providing more feedback with regards to the length of use of the
pointer returned by the `DerefMove::place` method, and the status of the value in that
location after use. Such a design would open up more possible use cases, but at the cost
of significantly more complicated desugarings.

Furthermore, allowing actions based on whether a value is present or not in the place
would add additional complexity in understanding the control flow of the resulting binary.
This could make understanding uses of these traits significantly more difficult for end
users of types that implement these traits.

### Nadrieril custom_refs proposal

Similar functionality is also being discussed as part of the [custom reference proposal
originally created by Nadrieril](https://hackmd.io/N0sjLdl1S6C58UddR7Po5g). This is also
fits in the category of significantly more complicated traits. However, the very large
amount of additional affordances it could offer may make it more worth it in this
specific case.

This RFC still prefers the simpler approach, as pretty much all of the Nadrieril
proposal would need to be implemented to get `DerefMove`-like behavior. This would bring
significant implementation effort and risk.

If desired, the functionality of this proposal can at a latter time be reimplemented
through the trait in that proposal, meaning that this can be seen as an intermediate
step.

### Limited macro based trait

Going the other way in terms of complexity, a `DerefMove` trait with constraints on how the
projection to the actual location to be dereferenced was proposed in [another internals forum thread](https://internals.rust-lang.org/t/derefmove-without-move-references-aka-box-magic-for-user-types/19910).

This proposal effectively constrains the `DerefMove::place` method to only doing field
projections and other dereferences. The advantage of this is that such a trait has 
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
sufficiently far along to support running code using the `DerefMove` trait, but does not yet
properly drop the content, instead leaking it.

## Unresolved questions
[unresolved-questions]: #unresolved-questions

Right now, the design states that panics in calls to `DerefMove::place` or `DerefMove::place_mut`
can cause an abort when the call was generated in the MIR. This is done to make compilation
and the resulting code more performant. It is however possible to implement these with
proper unwinding, at the cost of generating a more complicated control flow graph before
borrow checking for code using the dereference behavior of `DerefMove`. This would likely
result in longer compile times and less optimized results, however that could be judged
to be a worthwhile tradeoff.

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
for telling the borrow checker about new empty values implementing `DerefMove`, but that seems
orthogonal to the design here.
