- Feature Name: dyn-keyword
- Start Date: 2016-05-01
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Introduce a keyword, `dyn`, for denoting dynamic dispatch, with the motivation
of avoid "accidental overhead" and making the costs more explicit, along with
deprecation of the old syntax.

# Motivation
[motivation]: #motivation

The current syntax for dynamic dispatch, making use of traits as unsized types,
does not align very well with the Rust philosophy for multiple reasons:

## Hidden overhead

The current syntax hides the overhead: One would think that `Box<Trait>` simply
carried the overhead of the allocation and indirection, but, in fact, the major
overhead stems from dynamic dispatch. This expense is rather implicit, in that
there is denotation nor sigil to signify this overhead.

Dynamic dispatch is not bad per se, but the user must carefully evaluate the
choice.

## Reducing mistakes

It is prone to subtle mistakes and unnecessary dynamic dispatch due to
`Trait` appearing like a unsized type.

At first, it might not be obvious that `&Num` fundamentally behaves differently
from, say, `&u32`.

## Obviousness

Between new beginners, there are often questions and confusions about trait
objects and their syntax. A common misconception is that traits are types,
which is directly implied by the current syntax.

## Grep-ability

Being able to grep for some string in large codebases is critical to
maintainability. As it stands now, one cannot search for dynamically dispatched
types.

## `impl` syntax

Abstract unboxed types are, in a sense, "simpler" than dynamically dispatched
types. Thus, concerns have been raised about syntactically favoring dynamically
dispatched types over abstract unboxed return types.

One cannot use the syntax, `-> Trait`, for these, due to collision with the
trait object syntax, in a consistent manner. Using `dyn` allows this syntax to
stand free.

## Non-locality

An important property well-designed language features must hold is the ability
to reason locally. With the current syntax, one needs non-local information for
determining its behavior.

## Formal correctness

`Trait` is, type theoretically speaking, not a type. It might seem so because
they share same syntax. In particular, every dynamically dispatchable trait has
a sister type representing its dynamic counterpart.

This is rather confusing, since it is not obvious at first, that the meaning
depends on the context.

## Self-documenting

Adding `dyn` makes it largely self-documenting.

## Function pointers

The difference between

```rust
&Fn(A) -> B
```

and

```rust
fn(A) -> B
```

is a subtle one, despite there being a 10-20% performance improvement.

# Detailed design
[design]: #detailed-design

To overcome these hurdles, we introduce a new syntax: `dyn Trait`.

`dyn Trait` is an unsized type, which is the dynamically dispatched form of
`Trait`.

It is worth mentioning that `dyn` is not, and can not be, a type constructor.
Traits are classes of types, not types them self.

Secondly, we add a deprecation lint against the current syntax, where traits
are treated like types.

The parsing is done in a context-specific manner (alike #1444 and #243).

# Examples

```rust
let vec: Box<Any> = box 4;
```

will now be

```rust
let vec: Box<dyn Any> = box 4;
```

# Drawbacks
[drawbacks]: #drawbacks

This won't cause breakage, but deprecation is certainly a drawback.

Concerns have been raised about `&dyn` looking like a seperate pointer.

# Alternatives
[alternatives]: #alternatives

## Have a "magic" type constructor, `Object<Trait>`.

Acting as a dynamic dispatcher for `Trait`.

## Leave it as is.

# Unresolved questions
[unresolved]: #unresolved-questions

None.
