- Feature Name: transmute_trait
- Start Date: 2017-01-30
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

Add a built-in trait `Transmute<T>` for types that can be transmuted to `T`.

# Motivation
[motivation]: #motivation

The signature of `mem::transmute<T, U>` is a lie. Obstensibly, it can be called
with any two type arguments `T` and `U` but in reality there are extra
restrictions which will cause compilation to fail if `T` and `U` aren't
compatible. This violates the spirit and purpose of Rust's trait system which
is meant to encode all these sorts of restrictions in a principled way.

Suppose I want to make a generic function which can be called with any type
that can be transmuted to a `usize`. Currently, there is no way to write such a
function even though "transmutable to a `usize`" is (a) exactly the kind of
concept which could be represented by a trait and (b) a concept already known
to the compiler.

In this RFC I propose that the transmutablity rules be reflected in a trait and
that `transmute` should respect the trait system by requiring this trait bound.

# Detailed design
[design]: #detailed-design

Add the following trait as a lang item.

```rust
#[lang="transmute_trait"]
trait Transmute<T> {}
```

This trait is automatically implemented for types which can be transmuted to
`T`. The rules for transmutability are that the input and output types must
match in size and that the output type may not be visibly uninhabited unless
the input type is aswell.

Change the type of `mem::transmute` to:

```rust
unsafe extern "rust-intrinsic" fn transmute<T, U>(e: T) -> U
    where T: Transmute<U>
```

# How We Teach This
[how-we-teach-this]: #how-we-teach-this

As this is just encoding the transmutability rules into a trait, we should
teach this trait whereever we currently teach transmute.

# Drawbacks
[drawbacks]: #drawbacks

Adds one more lang item to the language.

# Alternatives
[alternatives]: #alternatives

* Not do this.
* Add `transmute` as an `unsafe` method to the `Transmute` trait.

# Unresolved questions
[unresolved]: #unresolved-questions

None.
