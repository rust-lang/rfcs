- Feature Name:  `prelude_2024_future`
- Start Date:  2023-10-05
- RFC PR: [rust-lang/rfcs#3509](https://github.com/rust-lang/rfcs/pull/3509)
- Rust Issue: [rust-lang/rust#121042](https://github.com/rust-lang/rust/issues/121042)

# Summary
[summary]: #summary

This RFC describes the inclusion of the `Future` and `IntoFuture` traits in the 2024 edition prelude.

# Motivation
[motivation]: #motivation

When an `async fn` is desugared we obtain an anonymous type with a signature of `impl Future`. In order to use this type we must first be able to _name_ it, which can only be done if the  `Future` trait in scope. Currently this can be a little inconvenient since it requires `std::future::Future` to be manually imported.

`IntoFuture` comes up regularly when writing operations that accept a `Future`. Adding `IntoFuture` makes it easy to call `.into_future()` to bridge between code accepting only `Future` and code supplying an `IntoFuture` impl, as well as making it easy to write new code that accepts any `IntoFuture` impl.

Both of these traits are generally useful, come up regularly, don't conflict with anything, and will not produce any surprising behavior if added to the prelude. And most other reifications of control-flow effects have their respective types and traits included in the prelude header. To support iteration we include both the `Iterator` and `IntoIterator` traits in the prelude. To support fallibility we include both `Option` and `Result` in the prelude. This RFC proposes we include both `Future` and `IntoFuture` to match.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

In Rust 2024, you can name and make use of the `Future` and `IntoFuture` traits without explicitly importing them, since they appear in the Rust 2024 prelude. For instance, you can write a function that accepts or returns an `impl Future`, or one that accepts any `impl IntoFuture`. Let's start with an example which takes an `impl IntoFuture`:

```rust
use std::future::IntoFuture;

async fn meow(phrase: impl IntoFuture<Output = String>) {
    println!("{}, meow", phrase.await);
}
```

In the Rust 2021 edition this code would require an explicit import of the `IntoFuture` trait. When migrating to the 2024 edition the import of the `IntoFuture` trait would be taken care of by the prelude, and the code could remove the explicit import:

```rust
async fn meow(phrase: impl IntoFuture<Output = String>) {
    println!("{}, meow", phrase.await);
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This RFC proposes we include both `Future` and `IntoFuture` as part of the 2024 prelude. The prelude in `std` re-exports the prelude in `core`. So the only change we need to make is in the prelude in `core`, changing it to both include `Future` and `IntoFuture`:

```rust
// core::prelude::rust_2024
mod rust_2024 {
    pub use crate::future::Future;
    pub use crate::future::IntoFuture;
    pub use super::rust_2021::*; 
}
```

# Tradeoffs
[tradeoffs]: #tradeoffs

Both the `Future` and `IntoFuture` definitions in the standard library are considered _canonical_: there exist no widespread alternative definitions in the Rust ecosystem. Simply having both traits in scope is unlikely to lead to any issues, and the only likely noticeable outcome is that authoring async code will require slightly less effort.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Include `Future` in 2024, re-consider `IntoFuture` in 2027

This RFC takes what could be called a: _"systems-based perspective"_. We're arguing that the core property which qualifies the `Future` and `IntoFuture` traits for inclusion in the prelude is their fundamental relationship to the language. Similar to how `Iterator` and `Result` correspond to core control-flow effects, `Future` and `IntoFuture` do too.

However, it's possible to use different criteria for what should be included in the prelude. One example is what could be referred to as a: _"merit-based perspective"_. From this perspective, items would only qualify for inclusion once they've crossed some usage threshold. From that perspective the `Future` trait would likely qualify since it's in wide use. But `IntoFuture` likely would not since it's a newer trait which sees less use.

Both of these perspectives can be applied to other scenarios too. Let's say we're finally able to stabilize the `Try` trait; should this be included in the following prelude? From a systems-based perspective, the answer would be "yes", since it's a fundamental operation which enables types to be named. From the merit-based perspective the answer would likely be "no", since it will be a new trait with limited usage. But it might be re-considered once it sees enough usage.

We believe that taking a merit-based perspective makes sense if the upsides of a choice also carry notable downsides. But as covered in the "tradeoffs" section of this RFC, there don't appear to be any meaningful downsides. So instead it seems better to base our evaluation on how the traits relate to the language, rather than on how much usage they see.

# Prior art
[prior-art]: #prior-art

## New inclusions in the Rust 2021 edition

The Rust 2021 edition includes three new traits:

- `FromIterator` - conversion from an iterator to a type
- `TryFrom` - fallible conversion
- `TryInto` - fallible conversion (inverted)

All three of these traits represent fundamental operations present in Rust. This is a natural supplement to other fundamental operations present in earlier editions such as `Try`, `Into`, and `IntoIterator`. I'd argue that `Future` and `IntoFuture` have an equal, if not more fundamental relationship to the Rust language than `TryFrom` or `FromIterator` do.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None at this time.

# Future possibilities
[future-possibilities]: #future-possibilities

## Inclusion of `Try` in a future prelude

As  in the "alternatives and rationale" section of this RFC, if we apply the same reasoning we're using in this RFC to the `Try` trait. Then once stabilized the `Try` trait's fundamental relationship to the language would qualify it for inclusion in an future edition's prelude as well.
