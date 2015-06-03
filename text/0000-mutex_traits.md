- Feature Name: mutex_Traits
- Start Date: 2015-06-02
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Introduce a conception distinction between `!Trait` and `?Trait` to enable mutually exclusive
traits within the constraint that implementing traits should be a backwards compatible change.
With these two type - trait relations distinguished from one another, allow negative impls of all
traits as well as negative bounds, introducing mutual exclusion to the trait system. This enables
users to encode additional logic in the type system & to provide multiple coherent blanket impls
for parameterized types with mutually exclusive trait bounds, avoiding the backward compatibility
problems that have hampered similar proposals in the past.

# Motivation

Trait coherence rules ensure that the compiler will predictably associate a particular impl with
any invocation of a method or item associated with that trait. Without coherence rules, the
behavior of a Rust program could easily become disastrously non-deterministic.

However, the current coherence rules are very conservative in what they allow to be implemented.
Other RFCs may introduce allowances for multiple overlapping implementations using an order of
precendence for specialized implementations. This RFC addresses the limitation from another angle.
By introducing mutually exclusive traits, it becomes possible tp dec;are tjat twp traits must not
both be implemented by a single type, causing parameters bound b each of those traits to be
non-overlapping.

Take this system, which defines `Consumable`, `Edible`, and `Poisonous`, traits.

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

Though logic of this sort can be implemented with ADTs, they can not be implemented using traits,
which are importantly extensible in a way that ADTs aren't. Rust gains a great deal of
expressiveness, composability, and power from the ability of two unrelated crates to extend the
capabilities of types from a third crate they both depend on. However, this kind of pivot cannot
be performed on traits by, for example, crate A implementing a trait from std for its type, and
crate B implementing a new trait for all types which implement that same trait.

In addition to the coherence benefits of this change, disallowing a single type to implement two
traits has benefits in itself. By encoding mutual exclusivity in the type system, programmers can
implement a greater portion of the program logic in a statically analyzable manner. For example,
the `num` crate currently defines two traits `Signed` and `Unsigned`. `Unsigned` is a marker trait
which exists only to indicate that a type is not `Signed`, but the type system will allow types to
implement both `Signed` and `Unsigned` without objection. Clients of `num` cannot actually rely on
an `Unsigned` bound to guarantee that a type is not `Signed`, even though that is the trait's only
purpose.

# Detailed design

## Trait, !Trait, and ?Trait

An earlier RFC attempted to codify negative reasoning ran aground on the problem of backward
compatibility. If it is possible to bound a type parameter or trait by the non-implementation of
another trait, then that non-implementation - the _absence_ of code - becomes a semantic
expression. As an example, consider this system, with traits `Consuamble`, `Edible`, and type
`Dirt`.

```rust
pub trait Consumable {
    fn consume(&self);
}

pub trait Edible {
    fn nourish(&self);
}

impl<T> Consumable for T where T: Edible {
    fn consume(&self) { self.nourish() }
}

impl<T> Consumable for T where T: !Edible {
    fn consume(&self) {  }
}

pub type Dirt;
```

Under the current definitions, consuming `Dirt` has no effect, but if we were to later to discover
a wayo to implement `Edible` for `Dirt`, that would change, and the behavior of any client relying
on `Dirt`'s implementation of `Consumable` wouild change.

A solution to this problem which would enable mutual exclusivity is to hold that the relation
between types and traits is in one of three states for every type and trait, rather than two:

* `T: Trait` - The type `T` impleemnts the trait `Trait`.
* `T: !Trait` - The type `T` does not implement the trait `Trait`.
* `T: ?Trait` - The type `T` may or may not implement the trait `Trait`.

Without a contrary expression, the relationship between any `T` and any `Trait` is  `T: ?Trait`.
This table documents the three relations and how they are described:

|               | ?Trait            | Trait                | !Trait             |
|---------------|-------------------|----------------------|--------------------|
| Specific impl | by default        | impl Trait for T     | impl !Trait for T  |
|               | impl ?Trait for T |                      |                    |
| Default impl  | by default        | impl Trait for ..    | impl !Trait for .. |
| Bounds        | by default        | where T: Trait       | where T: !Trait    |
|               | where T: ?Sized   | by default for Sized |                    |

## Definition of ?Trait and !Trait

`?Trait` and `!Trait` act as if they were marker traits. They define no methods and have no
associated items. They are defined with the definition of `Trait` and imported wherever `Trait` is
imported.

## Implementing ?Trait

`?Trait` can only be implemented for types for which a default impl has been explicitly defined
(e.g. `Send` and `Sync`). Explicit default impls of `?Trait` are not allowed. `?Trait` is
implemented by default anyway, and it would not make sense to implement it except in the cases
where an explicit default impl exists. As a rule, the syntax `?Trait` will be very uncommon.
`?Trait` follows the same orphan rules as `Trait`.


## Implementing !Trait

Implementing `!Trait` for `T` is a forward-compatible guarantee that `T` does not and will not
implement `Trait`. This makes negative reasoning explicit and avoids backwards compatibility
hazards. It goes without saying that it would be a coherence violation for a single type to
implement both `Trait` and `!Trait`.

`!Trait` follows the same orphan rules as `Trait`.

## Bounding by !Trait

Bounding by `!Trait` requires that types _explicitly implement_ `!Trait` in order to meet that
bound. As mentioned prior, this avoids the hazard that implicit negative reasoning introduces.

## Syntactic sugar: Implicit `!Trait` inference

If a type `T` implements a trait `Foo` which is bounded `!Bar`, an implementation of `!Bar` is
inferred for `T` (unless `T` explicitly implements `!Bar`, of course). This avoids boilerplate
negative impls which are inferrable from other impls for the type.

## Clarification of default impl rules

If a default impl of `Trait` exists, these rules are used to determine the relation between `T` and
`Trait`:

* If `Trait`, `?Trait` or `!Trait` is implemented for `T`, that impl defines the relation
* If one of the members of `T` impls a trait which conflicts with the default impl, `T` is `?Trait`
* Otherwise, `T` implements the defualt impl.

Note that this definition is sound if we suppose that every trait has an implicit default impl of
`?Trait`.

## Orphan rule warbles

The rules above all apply to a Rust system as a whole, composed of multiple crates associated as a
directed acyclic graph. Within crates and modules, orphan rules allow silence to have a semantic
expressions that is slightly different from these rules. Unfortunately, eliminating this warble
would be backwards incompatible.

Specifically, when both a trait and a type are defined within a single crate, that type and trait
have the relationship `T: !Trait` by default, rather than `?Trait`, only within that crate. This
allows a certain degree of implicit negative reasoning which cannot be performed outside of that
local context. It does not present a contradiction for the proposal.

# Drawbacks

This adds rules to Rust's trait coherence system. Adding rules to the language makes it less
accessible, and is always a drawback. There is a trade off here between easiness and
expressiveness.

It may be difficult to grok the difference between `!Trait` and `?Trait`. The reason for this
difference only becomes clear with an understanding of all the factors at play in the coherence
system. Inferred `!Trait` impls and the rarity of `?Trait` impls should make this an unlikely
corner of the trait system for a new user to accidentally happen upon, however.

The `impl !Trait for T` syntax overlaps with the syntax of existing negative impls for types with
default impls, and has slightly greater semantic content under this RFC tahn before. For each
existing negative impl, it will need to be determined whether that type should impl `!Trait` or
`?Trait` (that is, whether or not the non-implementation is a guarantee). That said, this change is
not backwards incompatible and will not cause any regressions, and existing negative impls are an
unstable feature outside of std.

# Alternatives

## Sibling proposal: !Trait by default

There is an alternative scheme which has some advantages and disadvantages when compared to that
proposed in the main RFC. I am mostly certain that the main proposal is the better one, but I have
included this for a complete consideration.

Under this alternative, types would impl `!Trait` by default, and a default implementation of
`?Trait` would be necessary to make that not the case. The table for such a proposal would look
like this:

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

Under this alternative proposal, types would be implicitly non-overlapping with traits they do not
implement, but it would also be backwards incompatible to implement new traits for types unless
the trait's author has specified that it should be. Because the author is unlikely to know if
anyone will want to add new implementations in a backwards compatible way, I have preferred that
traits by default be implemented `?Trait`.

## Other alternatives

Allowing negative bounds without distinguishing `!Trait` and `?Trait` remains an alternative, but
it presents a backward compatibility hazard as discussed above.

Doing nothing is also an alternative; this would mean that traits cannot be declared to be
mutually exclusive.

## Not an alternative: specialization

As an aside, this RFC does not overlap with proposals for trait specialization. Mutual exclusion is
useful for situations in which specialization would not be possible, and the same is true of the
reverse. Put in terms of sets, types which implement mutually exclusive traits are disjoint sets,
whereas specialization allows a distinct implementation for a subset of the types which implement a
given trait.

Conceptually, they are connected in that they expand what is allowed by Rust's coherence system,
but their use cases are separate and distinct.

# Unresolved questions

This RFC does not attempt to address how mutual exclusion would be applied to the types and traits
in std and other Rust-lang sponsored crates. This should be the subject of one or more separate
RFCs.
