- Feature Name: discriminant_bits
- Start Date: 2019-04-01
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add methods to `std::mem::Discriminant` which inform of the space necessary for a bitwise representation of an enums Dicriminant and the bits itself in an opaque fashion.

# Motivation
[motivation]: #motivation

Rust encourages using enums to encode data with multiple variants. And example of this can be found in the [game of life tutorial][game-of-life-tutorial].

```rust
enum Cell {
    Dead  = 0,
    Alive = 1
}
```

Using these enums in collections is wasteful, as each instance reserves at least 1 byte of space. Similarly, `std::mem::size_of<Discriminant<Cell>>()` is at least 1 byte. For that reason, the book later goes on and replaces `Vec<Cell>` by [`fixedbitset`][game-of-life-exercise].

If it were possible to read the exact necessary size and the bit representation the descriminant, we could have interface like this:

```rust
let x = PackedBits<Cell>;
```

Where `PackedBits` uses exactly as much space as necessary.

This allows for an efficient representation of Discriminant sets, which is both useful for simple enums, but also for crating an index of all Discriminant values present in collection.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## Disciminant data

`Discriminant#bit_size` and `Discriminant#data` are two methods to retrieve the structure of the discriminant.

```rust
const fn bit_size(&self) -> usize { }
```

The `bit_size` function returns the number of bits necessary to represent this discriminant. This number is not subject to optimisation, so e.g. `Option<&str>` reports a bitsize of `1`.

For example:

```rust
enum Cell {
    Dead = 0,
    Alive = 1,
}

enum RGB {
    Red,
    Green,
    Blue
}

std::mem::discriminant(Cell::Alive).bit_size == 1
std::mem::discriminant(Option::None as Option<&str>).bit_size == 1
std::mem::discriminant(RGB::Red).bit_size == 2
```

This information can be used to pack multiple discriminants easily for in bitfields for efficient storage and easy indexing.

```rust
fn data(&self) -> usize
```

Returns a bit representation of the discriminant. This data can be used to construct an efficient storage or index.

```rust
fn from_data<T>(data: usize) -> Discriminant<T>
```

Creates a Discriminant from emitted data usable for comparison.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The feature may interact with non-exaustive enums. In this case, still, the currently used discriminant size should be used.

Adding the proposed functions probably entails adding a new compiler intrinsic `discriminant_size`.

Empty enums are of 0 size.

# Drawbacks
[drawbacks]: #drawbacks

The added methods increase API surface in stdlib.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- Why is this design the best in the space of possible designs?
- What other designs have been considered and what is the rationale for not choosing them?
- What is the impact of not doing this?

# Prior art
[prior-art]: #prior-art

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Naming of the functions could be improved.
- A basic implementation of a bitfield should be created during the implementation phase
- Exact layout of the returned usize value is not clear, especially endianess.
- `std::mem::discriminant` isn't const, which makes `bit_size` unreachable as a const function. Can `std::mem::discriminant` be const?

# Future possibilities
[future-possibilities]: #future-possibilities

The feature is self-contained and I don't see direct extensions.

[game-of-life-tutorial]: https://rustwasm.github.io/docs/book/game-of-life/implementing.html
[game-of-life-exercise]: https://rustwasm.github.io/docs/book/game-of-life/implementing.html#exercises
