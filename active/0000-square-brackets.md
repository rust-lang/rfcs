- Start Date: 2014-06-30
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Switching (back) the current type parameter syntax from `<>` to `[]`.

# Motivation

Recently there has been a lot of talks on simplifying the syntax. Starting from removing the sigils `@` and `~` and making lifetimes less syntax heavy (through various proposals). I think changing the current generic syntax to `[]` will make it that much better and clearer (I think `[]` is much easier to read).

1. We would remove the current ambiguities surround the current syntax `<>`. That means, we could be able to have:

```rust
struct Vec[T] {
    // ...
}

fn foo[T]() -> Vec[T] {
    // ...
}

fn main() {
    let something = foo[int]();
}
```

2. `[]` composes **much** better than the more cryptic `<>` form. This is a common readability issue when working with any nested types (such as encoders and decoders).

```rust
fn parse['a, T: Encodable[Encoder['a], IoError]](value: T) {
    // ...
}
```

3. It would bring the ability to have much nicer syntax when dealing with HKTs (there are a few different proposals I have in mind in terms of syntax, but it's mostly inferred.).

```rust
// Possible syntax for HKTs.
pub trait Monad[M[T]] {
    // ...
}
```

4. There's precendence for it. Scala's syntax for generics is awesome. It imposes very little effort (I think) when reading and understanding.

6. Because it's consistent and has no ambiguities, one can finally use motions like `%` in Vim (and alternatives in other editors.).

# Detailed design

This is a very easy change to make.

## Downsides

* The syntax is used quite a bit. Automation could potentially do some, if not most of the changes (The tricky part is the ambiguities in the current syntax). However, of the changes we've had in the past, I think this syntax change is a whole lot easier to work with than semantic changes, or more complex syntax changes.

* One that I forgot about is the issue with the indexing syntax.

# Alternatives

* Keep it like it currently is and end up with the current syntax forever.

# Unresolved questions

* Why was did Rust originally have `[]` but decided to switch to `<>`? I heard it was related to try and be consistent with C-class languages (C++, Java, etc...), is this correct?
