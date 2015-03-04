- Feature Name: if_let_multiple_patterns
- Start Date: 2015-03-04
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Allow `if let` to match on multiple patterns with `|`.

# Motivation

The goal of this RFC is to enable `if-let` to match on multiple patterns, just
like match is able to.

Currently, the `if-let` notation is inconsistent with the `match` notation,
even though `if-let` is mostly sugaring for `match`. Specifically, `if-let` does not
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

This RFC proposes to extend the notation for `if-let` to also cover multiple
patterns separated by pipes (`|`).

# Detailed design

[if-let](text/0160-if-let.md)


This is the bulk of the RFC. Explain the design in enough detail for somebody familiar
with the language to understand, and for somebody familiar with the compiler to implement.
This should get into specifics and corner-cases, and include examples of how the feature is used.

# Drawbacks

Why should we *not* do this?

# Alternatives

What other designs have been considered? What is the impact of not doing this?

# Unresolved questions

What parts of the design are still TBD?
