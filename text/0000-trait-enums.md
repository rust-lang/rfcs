- Feature Name: `trait_enum`
- Start Date: 2020-03-04
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC proposes a new type of enum which is a union of types with common traits as an alternative to enums with associated types and trait objects.

# Motivation
[motivation]: #motivation

This feature reduces code duplication while using enums.

Consider the following example:

```rust
trait Animal {
    fn leg_count(&self) -> u16 {
        4
    }
}

struct Chicken;
impl Animal for Chicken {
    fn leg_count(&self) -> u16 {
        2
    }
}

struct Sheep;
impl Animal for Sheep {}

enum UnknownAnimal {
    Chicken(Chicken),
    Sheep(Sheep),
}

impl Animal for UnknownAnimal {
    fn leg_count(&self) -> u16 {
        match self {
            UnknownAnimal::Chicken(chicken) => chicken.leg_count(),
            UnknownAnimal::Sheep(sheep) => sheep.leg_count(),
        }
    }
}

fn main() {
    let animals = [UnknownAnimal::Chicken(Chicken), UnknownAnimal::Sheep(Sheep)];

    for animal in &animals {
        println!("This animal has {} legs!", animal.leg_count());
    }
}
```

It is tedious to pattern match every single variant of an enum.
So the goal of this RFC is to eliminate this need and encourage the use of enums instead of trait objects in some situations.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Enums are useful in many situations. One of those situations is when a collection of different types is needed.

... (Explanation of enums in the book)

Sometimes we want to store objects of different types but with a common interface. Trait enums are exactly for that.

Let's consider an example where we need to keep track of the animals in our farm:

```rust
trait Animal {
    fn leg_count(&self) -> u16 {
        4
    }
}

struct Chicken;
impl Animal for Chicken {
    fn leg_count(&self) -> u16 {
        2
    }
}

struct Sheep;
impl Animal for Sheep {}

enum UnknownAnimal: Animal {
    // List of the types that can be stored
    Chicken,
    Sheep,
}

fn main() {
    let animals: [UnknownAnimal; _ ] = [Chicken, Sheep, Chicken];

    for animal in &animals {
        println!("This animal has {} legs!", animal.leg_count());
    }
}
```

We must always make sure that the types in our enum implement the specified traits, otherwise the compiler will complain.

```rust
trait Animal {}

struct Chicken;
impl Animal for Chicken {}
struct Sheep;

// Error: Sheep does not implement trait Animal required by trait enum UnknownAnimal
enum UnknownAnimal: Animal {
    Chicken,
    Sheep,
}
```

This feature's usefulness might be hard to understand for new Rustaceans.
However, more experienced Rust developpers know that using enums instead of trait objects is more optimized.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Trait enums are a middle ground between normal enums and C-like unions.
They do have an active field like rust enums but the user does not need to know the contained type to use the trait enum.

Trait enums must have at least one variant.

## Initialization and assignment
```rust
enum Sword: Weapon {
    Excalibur,
    IronSword
}

let sword = Excalibur::new() as Sword; // Either like this
let mut sword: Sword = IronSword::new(); // or like this
sword = Excalibur::new(); // The type is already known so there's no need to specify it
```

## Downcasting
Trait enums can be downcasted to their concrete type.

```rust
enum Cheese: Food {
    BlueCheese,
    GoatCheese
}

let cheese: Cheese = BlueCheese::new();
if let Some(_) = cheese.downcast_ref::<GoatCheese>() {
    println!("This is definitely Goat cheese!");
}
```

## Conflicting methods in traits
```rust
trait Tool {
    fn sharpen(&self);
}

trait Weapon {
    fn sharpen(&self);
}

enum Sword: Tool + Weapon {
    ...
}

let sword: Sword = IronSword::new();

Tool::sharpen(&Sword);
<Sword as Tool>::sharpen(&Sword);
```

# Drawbacks
[drawbacks]: #drawbacks

This feature is only useful in very specific cases and is quite complicated to implement.

## Syntax
This feature could either be named a trait enum, a type enum, a trait union or a safe union.
Thus, it should either use the `enum` keyword or the `union` keyword.
However, those keywords are already used so this would add more context-specific parsing.

## Type inference
The easiest way to easily manipulate trait enums would be to need a keyword like `Union` for unions.
The keyword would be used to initialize and assign to trait enums.
However, this is really ugly and hard to read. So it would be better if casting was needed or type inference didn't work at all.
This makes the implementation more complicated.

## Concrete type
The concrete type is harder to get than with normal enums.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This design allows achieving performance like normal enums while being convenient like trait objects.

# Prior art
[prior-art]: #prior-art

This idea is brand new. There is no prior-art.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

## Implementing additional methods
Implementing methods specific to a trait enum would be complicated. The easiest thing to do is to disallow it.
However, this is can be useful to implement methods to change the underlying type based on the current type.

## Downcasting possibility
Is it reasonable to thing that the `Any` trait could be used to check the concrete type of the trait enum?

## The name is pretty lame
Do you have better suggestions?

# Future possibilities
[future-possibilities]: #future-possibilities

## Generic trait enums
```rust
enum Weapon<T>: Attack where T: Material {
    Excalibur,
    Sword<T>,
}
```