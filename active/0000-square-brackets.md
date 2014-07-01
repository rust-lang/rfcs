- Start Date: 2014-06-30
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Switching (back) the current type parameter syntax from `<>` to `[]`.

# Motivation

Recently there has been a lot of talks on simplifying the syntax. Starting from removing the sigils `@` and `~` and making lifetimes less syntax heavy (through various proposals). I think changing the current generic syntax from `<>` to `[]` will make it that much better and clearer (I think `[]` is much easier to read).

1. `[]` is easier to type than `<>` on *most* keyboards.

2. IMO `[]` composes **much** better than the more cryptic `<>` form. The `[]` syntax separates the different pieces more so than `<>`. The `<>` syntax elongates everything and it mashes it's contents. This is a common readability issue when working with any nested types (such as encoders and decoders).

```rust
// Current syntax
fn parse<'a, T: Encodable<Encoder<'a>, IoError>>(value: T) {
    // ...
}
```

vs

```rust
// New syntax
fn parse['a, T: Encodable[Encoder['a], IoError]](value: T) {
    // ...
}
```

3. At the time when Rust switched from `[]` to `<>`, there was no precedence in a C-style language for `[]` generics; this is no longer true: Scala is an example of a language that has become fairly popular recently and which uses `[]` for its generics syntax.

4. `[]` delimeters are always matching, where one can finally use motions like `%` in Vim (and alternatives in other editors.).

# Detailed design

Type parameters would be encapsulated with `[]` instead of `<>`.

```rust
struct Vec[T] {
    // ...
}

fn compile['a, T](input: &'a str, arg: T) -> CompiledArg[T] {
    // ...
}
```

Ambiguities with vector indices are avoided the same way the current syntax works.

```rust
foo::[int]();
```

# Downsides

* The syntax is used quite a bit. Automation could potentially do some, if not most of the changes (The tricky part is the ambiguities in the current syntax). However, of the changes we've had in the past, I think this syntax change is a whole lot easier to work with than semantic changes, or more complex syntax changes.

* One that I forgot about is the issue with the indexing syntax, so there might still be ambiguity.

# Alternatives

* Keep it like it currently is and end up with the current syntax forever.

# Unresolved questions

* Why was did Rust originally have `[]` but decided to switch to `<>`? I heard it was related to try and be consistent with C-class languages (C++, Java, etc...), is this correct?
