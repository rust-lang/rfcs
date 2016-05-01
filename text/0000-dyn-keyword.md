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
does not align very well with the Rust philosophy for two reasons:

1. It hides the overhead: One would think that `Box<Trait>` simply carried the
   overhead of the allocation and indirection, but, in fact, the major overhead
   stems from dynamic dispatch. This expense is rather implicit, in that there
   is denotation nor sigil to signify this overhead.
2. It is prone to subtle mistakes and unnecessary dynamic dispatch due to
   `Trait` appearing like a unsized type.

Furthermore, it is worth noting that `Trait` is not a type, despite it may seem
so.

# Detailed design
[design]: #detailed-design

To overcome these hurdles, we introduce a new syntax: `dyn Trait`.

`dyn Trait` is an unsized type, which is the dynamically dispatched form of
`Trait`. Two things are essential to the semantics:

1. `∀T.[dyn T]∊T`: namely that `dyn Trait` satisfy the bound `: Trait`.
2. `∀t∊T.[c (dyn T) <: c t]`: meaning that `T` where `T: Trait` can coerce into
   `dyn Trait`. This rule is similar to the current trait object coercion rule.

It is worth mentioning that `dyn` is not, and can not be, a type constructor.
Traits are classes of types, not types them self.

Secondly, we add a deprecation lint against the current syntax, where traits
are treated like types.

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

# Alternatives
[alternatives]: #alternatives

## Have a "magic" type constructor, `Object<Trait>`.

Acting as a dynamic dispatcher for `Trait`.

## Leave it as is.

# Unresolved questions
[unresolved]: #unresolved-questions

None.
