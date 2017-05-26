- Feature Name: mutex_Traits
- Start Date: 2015-06-02
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Introduce a distinction between `!Trait` and `?Trait` to enable mutually exclusive traits within the constraint that
implementing a trait for a type should be a backwards compatible change. With these two type - trait relations
distinguished from one another, allow negative impls of all traits as well as negative bounds, introducing mutual
exclusion to the trait system. This enables users to encode additional logic in types & to provide multiple coherent
blanket impls for parameterized types with mutually exclusive trait bounds, avoiding the backward compatibility
problems that have hampered similar proposals in the past.

# Motivation

Trait coherence rules ensure that the compiler will predictably associate a particular impl with any invocation of a method or item associated with that trait. Without coherence rules, the behavior of a Rust program could easily become disastrously non-deterministic.

However, the current coherence rules are very conservative in what they allow to be implemented. Other RFCs may
introduce allowances for multiple overlapping implementations using an order of precendence for specialized
implementations. This RFC addresses the limitation from another angle. By introducing mutually exclusive traits, it
becomes possible to declare that two traits must not both be implemented by a single type, causing parameters bound by
each of those traits to be non-overlapping.

Take this system, which defines `Consumable`, `Edible` and `Poisonous` traits.

```rust
pub trait Consumable {
    fn consume(&self);
}

pub trait Edible: !Poisonous {
    fn nourish(&self);
}

pub trait Poisonous: !Edible {
    fn sicken(&self);
}

impl<T> Consumable for T where T: Edible {
     fn consume(&self) { self.nourish() }
}

impl<T> Consumable for T where T: Poisonous {
     fn consume(&self) { self.sicken() }
}
```

Though logic of this sort can be implemented with ADTs, they can not be implemented using traits, which are 
importantly extensible in a way that ADTs aren't. Rust gains a great deal of expressiveness, composability, and power 
from the ability of two unrelated crates to extend the capabilities of types from a third crate they both depend on. 
For example, two crates which depend on std can both extend `Vec<T>`, and a client of both those crates can compose 
the behaviors they implement. However, this kind of 'pivot' cannot be performed on traits by, for example, crate A 
implementing a trait from std for its new type, and crate B implementing a new trait for all types which implement 
that same trait.

In addition to the coherence benefits of this change, disallowing a single type to implement two traits has benefits 
in itself. By encoding mutual exclusivity in the type system, programmers can implement a greater portion of the 
program logic in a statically analyzable manner. For example, the `num` crate currently defines two traits `Signed` 
and `Unsigned`. `Unsigned` is a marker trait which exists only to indicate that a type is not `Signed`, but the type 
system will allow types to implement both `Signed` and `Unsigned` without objection. Clients of `num` cannot actually 
rely on an `Unsigned` bound to guarantee that a type is not `Signed`, even though that is the trait's only purpose.

# Detailed design

## Trait, !Trait, and ?Trait

An earlier RFC attempted to codify negative reasoning ran aground on the problem of backward compatibility. If it is 
possible to bound a type parameter or trait by the non-implementation of another trait, then that non-implementation -
the _absence_ of code - becomes a semantic expression. As an example, consider this system, with traits `Consuamble`, 
`Edible`, and type `Dirt`.

```rust
pub trait Consumable {
    fn consume(&self);
}

pub trait Edible {
    fn nourish(&self);
}

pub type Dirt;

impl<T> Consumable for T where T: Edible {
    fn consume(&self) { self.nourish() }
}

impl<T> Consumable for T where T: !Edible {
    fn consume(&self) {  }
}
```

Under the current definitions, consuming `Dirt` has no effect, but if we were to later to discover a way to implement 
`Edible` for `Dirt`, that would change, and the behavior of any client relying on `Dirt`'s implementation of 
`Consumable` wouild change.

A solution to this problem which would enable mutual exclusivity is to hold that the relation between types and traits
is in one of three states for every type and trait, rather than two:

* `T: Trait` - The type `T` implements the trait `Trait`.
* `T: !Trait` - The type `T` does not implement the trait `Trait`.
* `T: ?Trait` - The type `T` may or may not implement the trait `Trait`.

Without a contrary expression, the relationship between any `T` and any `Trait` is  `T: ?Trait`. This table documents 
the three relations and how they are described:

|               | ?Trait            | Trait                | !Trait             |
|---------------|-------------------|----------------------|--------------------|
| Specific impl | by default        | impl Trait for T     | impl !Trait for T  |
|               | impl ?Trait for T |                      |                    |
| Default impl  | by default        | impl Trait for ..    | impl !Trait for .. |
| Bounds        | by default        | where T: Trait       | where T: !Trait    |
|               | where T: ?Sized   | by default for Sized |                    |

## Defining ?Trait and !Trait

`?Trait` and `!Trait` act as if they were marker traits. They define no methods and have no associated items. They are
defined implicitly in the same scope as the definition of `Trait` and imported wherever `Trait` is imported.

## Implementing ?Trait

`?Trait` is a relation which means that `T` does not meet either the bounds `Trait` or `!Trait`; that is, whether or 
not it implements `Trait` is undefined. `?Trait` can only be implemented for types for which a default impl has been 
explicitly defined (e.g. `Send` and `Sync`), and explicit default impls of `?Trait` are not allowed. `?Trait` is 
implemented by default anyway, and it would not make sense to implement it except in the cases where an explicit 
default impl exists. As a rule, the syntax `?Trait` will be very uncommon.

`?Trait` follows the same orphan rules as `Trait`.

## Implementing !Trait

Implementing `!Trait` for `T` is a forward-compatible guarantee that `T` does not and will not implement `Trait`. This
makes negative reasoning explicit and avoids backwards compatibility hazards. It goes without saying that it would be 
a coherence violation for a single type to implement both `Trait` and `!Trait`.

`!Trait` follows the same orphan rules as `Trait`.

## Bounding by !Trait

Bounding by `!Trait` requires that types _explicitly implement_ `!Trait` in order to meet that bound. As mentioned 
prior, this avoids the hazard that implicit negative reasoning introduces.

## Clarification of default impl rules

If a default impl of `Trait` exists, these rules are used to determine the relation between `T` and `Trait`:

* If `Trait`, `?Trait` or `!Trait` is implemented for `T`, that impl defines the relation
* If one of the members of `T` impls a trait which conflicts with the default impl, `T` is `?Trait`
* Otherwise, `T` implements the default impl.

Note that this set of rules is sound if we suppose that every trait has an implicit default impl of `?Trait`.

## !Trait inference

Though the type - trait relation is `?Trait` by default, a `!Trait` relation can be inferred by providing
implementations which would be in conflict if the relation were `?Trait`. There are two categories of impls which
allow an inference:

* __Implementing a trait bound by `!Trait`:__ e.g. if trait `Edible` is bound `!Poisoonous`, and type `BirthdayCake` 
implements the trait `Edible`, `BirthdayCake` is inferred to be `!Poisonous`.
* __An implementation that would otherwise overlap:__ If trait `Consumable` is implemented for `T where T: Edible`,
and `Consumable` is implemented for `Dirt`, these would overlap if `Dirt` were `?Edible`, therefore these two impls
imply `Dirt: !Edible`. _Note_ that for backwards compatibility reasons, orphan rules currently restrict these impls
such that both `Consumable` and `Dirt` must be defined in the same crate in order for this impl to be allowable. This
orphan rule already exists, [see this code for an example](hhttps://play.rust-lang.org/?gist=cebbc637fa27a6c1640e&version=stable).

# Drawbacks

This adds rules to Rust's trait coherence system. Adding rules to the language makes it less accessible, and is always
a drawback. There is a trade off here between easiness and expressiveness.

It may be difficult to grok the difference between `!Trait` and `?Trait`. The reason for this difference only becomes 
clear with an understanding of all the factors at play in the coherence system. Inferred `!Trait` impls and the rarity
of `?Trait` impls should make this an unlikely corner of the trait system for a new user to accidentally happen upon, 
however.

The `impl !Trait for T` syntax overlaps with the syntax of existing negative impls for types with default impls, and 
has slightly greater semantic content under this RFC than before. For each existing negative impl, it will need to be 
determined whether that type should impl `!Trait` or `?Trait` (that is, whether or not the non-implementation is a 
guarantee). That said, this change is not backwards incompatible and will not cause any regressions, and existing 
negative impls are an unstable feature outside of std.

# Alternatives

## Sibling proposal: !Trait by default

There is an alternative scheme which has some advantages and disadvantages when compared to that proposed in the main 
RFC. I am mostly certain that the main proposal is the better one, but I have included this for a complete 
consideration.

Under this alternative, types would impl `!Trait` by default, and a default implementation of `?Trait` would be 
necessary to make that not the case. The table for such a proposal would look like this:

|               | ?Trait             | Trait                | !Trait            |
|---------------|--------------------|----------------------|-------------------|
| Specific impl | impl ?Trait for T  | impl Trait for T     | by default        |
|               |                    |                      | impl !Trait for T |
| Default impl  | impl ?Trait for .. | impl Trait for ..    | by default        |
| Bounds        | by default         | where T: Trait       | where T: !Trait   |
|               | except: ?Sized     | by default for Sized |                   |

The trade off at play here is between these two desirable and incompatible features:

* Adding new implementations should be backwards compatible.
* Implementations for `T: Trait` should not overlap with implementations for types that don't
implement Trait.

Under this alternative proposal, types would be implicitly non-overlapping with traits they do not implement, but it 
would also be backwards incompatible to implement new traits for types unless the trait's author has specified that it
should be. Because the author is unlikely to know if anyone will want to add new implementations in a backwards 
compatible way, traits implementing `?Trait` by default is preferred.

## Don't implement !Trait inference

The inferences that `T` is `!Trait` if it has certain impls defined is not necessary for the rest of the proposal to
be implemented; if this inference is considered an unnecessary or negative introduction, or conflicts in some way with
specification proposals, the rest of this proposal could be accepted without it.

## Other alternatives

Allowing negative bounds without distinguishing `!Trait` and `?Trait` remains an alternative, but it presents a 
backward compatibility hazard as discussed above.

Doing nothing is also an alternative; this would mean that traits cannot be declared to be mutually exclusive.

## Not an alternative: specialization

As an aside, this RFC does not overlap with proposals for trait specialization. Mutual exclusion is useful for 
situations in which specialization would not be possible, and the same is true of the reverse. Put in terms of sets, 
traits declare sets of types; mutually exclusive traits are disjoint sets, and specialized implementations are 
subsets.

Conceptually, they are connected in that they expand what is allowed by Rust's coherence system, but their use cases 
are separate and distinct.

# Unresolved questions

This RFC does not attempt to address how mutual exclusion would be applied to the types and traits in std and other 
Rust-lang sponsored crates. This should be the subject of one or more separate RFCs.
