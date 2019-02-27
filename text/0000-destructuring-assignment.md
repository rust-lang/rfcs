- Feature Name: Destructuring assignment
- Start Date: 2019-02-27
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

[summary]: #summary

Destructuring assignment for tuples: if a function returns a tuple, it can be destructured, with the individual values getting assigned to the provided variables.

# Motivation

[motivation]: #motivation

A destructuring declaration can already be made in rust with `let (a, b)`. Having destructuring assigment would complement this: variables can be both declared and assigned to.

# Guide-level explanation

[guide-level-explanation]: #guide-level-explanation

The `let` syntax allows you to bind the return value of a function to a new variable. When you declare a tuple in this way, the values are automatically destructured: in the following lines of your code, you can use the values individually.

```rust
fn tuple(a: i32, b: i32) -> (i32, i32) {
    (a, b)
}

fn main() {
    let (a, b) = tuple(1, 2);
    println!("{}", a);
    println!("{}", b);
}
```

The destructuring assignment can also be used to assign the values to previously declared mutable variables:

```rust
fn tuple(a: i32, b: i32) -> (i32, i32) {
    (a, b)
}

fn main() {
    let mut a: i32;
    let mut b: i32;

    (a, b) = tuple(1, 2);
    println!("{}", a);
    println!("{}", b);
}
```

# Reference-level explanation

[reference-level-explanation]: #reference-level-explanation

// TODO: help needed here.

# Drawbacks

[drawbacks]: #drawbacks

There are some concerns over whether this can be implemented in the Rust compiler while adhering to the LL(k) property.

# Rationale and alternatives

[rationale-and-alternatives]: #rationale-and-alternatives

In Rust, variables can be declared and assigned to. Tuples can be declared with destructuring, but currently destructuring assignment is not possible. This can feel like an inconsistency.

In addition, various other languages implement destructuring assignment, and the feature is commonly used. People coming to Rust from other languages have been surprised by the lack of this feature, as shown by the various issues and comments.

As an alternative, destructuring assignment could be left unimplemented. Workarounds to the usecases are possible, with additional destructuring and/or declaration steps, and don't necessarily require significantly more code.

# Prior art

[prior-art]: #prior-art

Relevant discussion:

- https://github.com/rust-lang/rfcs/issues/372
- https://github.com/rust-lang/rust/issues/10174
- https://github.com/rust-lang/rust/issues/12138

# Unresolved questions

[unresolved-questions]: #unresolved-questions

The implementation details for the compiler passes need to be investigated and precisely defined.

# Future possibilities

[future-possibilities]: #future-possibilities

Destructuring assignment would be a quality-of-life improvement: it would facilitate readable, clean code. Additionally, as similar features already exist and are widely used in various other languages, implementing this feature could reduce friction people coming from other languages experience while getting started with Rust.
