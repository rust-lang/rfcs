- Feature Name: if_while_let_multiple_patterns
- Start Date: 2015-03-04
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Allow `if let` and `while let` to match on multiple patterns with `|`.

# Motivation

The goal of this RFC is to enable `if let` and `while let` statements to match on multiple patterns, just
like `match` is able to.

Currently, the `if let` and `while let` notation is inconsistent with the `match` notation,
even though they are is just sugaring for `match`. Specifically, `if let` and `while let` do not
allow matching on multiple patterns.

For instance, the following piece of code is allowed:

```rust
enum Foo {
  One(u8),
  Two(u8),
  Three
}
use Foo::*;

fn main () {
    let x = One(42);
    match x {
        One(n) | Two(n) => {
            println!("Got one or two with val: {}", n);
        }
        _ => {}
    }
}
```

but this isn't:


```rust
enum Foo {
  One(u8),
  Two(u8),
  Three
}
use Foo::*;

fn main () {
    let x = One(42);
    if let One(n) | Two(n) = x {
        println!("Got one or two with val: {}", n);
    }
}
```

This RFC proposes to extend the notation for `if let` and `while let` to also cover multiple
patterns separated by pipes (`|`).

# Detailed design

Basically take the design proposed in the original RFC for [if-let](text/0160-if-let.md) and extend the notation to allow multiple patters

# Drawbacks

It's an additional feature in the language.

# Alternatives

Not doing anything and require multiple `if let` statements or an actual `match`.

# Unresolved questions

We could consider extending `if let` and `while let` to also include guards, but that has opens up the question about syntax (`if let Some(n) = x if n > 0` is not extremely pretty), so we should probably save that for an RFC of its own.
