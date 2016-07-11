- Feature Name: disjoint_associated_types
- Start Date: 2016-07-10
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

During coherence checking, when determining if the receivers of two impls are
disjoint, treat bounds with disjoint associated types as mutually exclusive
bounds.

# Motivation
[motivation]: #motivation

Consider this set of impls:

```rust
impl<T> Foo for T where T: Iterator<Item=u64> { }
impl<T> Foo for T where T: Iterator<Item=i64> { }
```

Both of these are "blanket impls" - they implement the trait `Foo` for an open
set of types - any type which implements `Iterator`, with the item `u64` in
the first, or the item `i64` in the second.

Blanket impls are a tricky beast, because often blanket impls will overlap with
other impls of the same trait, because the type being implemented for also
falls into the blanket impl (or could).

However, these two blanket impls, intuitively, do not overlap. Because you can
only implement `Iterator` once for each type, you cannot have a type which
implements `Iterator` with both of these types.

Another example of how this could be useful are the return types of functions.
Consider this code:

```rust
trait Applicator {
    fn apply(&mut self, Event) -> io::Result<()>;
}

fn apply_it<T>(T) -> io::Result<()> where T: Applicator { ... }

impl<F> Applicator for F where F: Fn(Event) -> io::Result<()> { ... }

impl<F> Applicator for F where F: Fn(Event) -> () { ... }
```

The return type here is an associated type, a function cannot return both
`io::Result<()>` and `()`. I had code a lot like this in a library I was
writing; the idea was that quick and dirty stateless implementations of the
analog trait to `Applicator` could be written as closures, returning either
an `io::Result` or unit. However, these impls are regarded as overlapping by
rustc today.

# Detailed design
[design]: #detailed-design

When considering whether two type variables are disjoint, these (informal)
rules prove that they are disjoint:

1. If they are both concrete types, and they are not the same type (this rule
already exists).
2. If they are both bound by the same trait, and both specify the same
associated type for that trait, and the types they specify are disjoint.

Additional rules could be added in separate RFCs, such as rules based on a
syntax for mutual exclusion.

Note that the second rule is recursive.


# How will we teach this?
[teach]: #teach

This will need to be documented in the reference or some other detailed
document, along with the general description of how Rust coherence works.
Otherwise, this doesn't particularly need to be called out separate from other
aspects of the coherence system.

# Drawbacks
[drawbacks]: #drawbacks

This adds more rules to coherence, making it more complicated. However, they
are intuitive rules, which align with the expected behavior of coherence, so
hopefully they will not add to the learning burden.

# Alternatives
[alternatives]: #alternatives

Explicit mutual exclusion of bounds ("negative bound syntax") could provide
some, but not all, of the benefits of this feature.

We could always do nothing and leave coherence as it is.

# Unresolved questions
[unresolved]: #unresolved-questions

None.
