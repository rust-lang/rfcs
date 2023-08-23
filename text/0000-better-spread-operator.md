- Feature Name: better-spread-operator
- Start Date: 2023-07-23
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)

# Summary
[summary]: #summary

- Allow a value to be spread into another value of a different type
- Allow partial field spreading

# Motivation
[motivation]: #motivation

We currently have the spread operator, which allows a struct to be constructed with default values provided by other instance of the struct:

```rust
struct A {
  foo: u8,
  bar: u8
}
let a = A {foo: 1, bar: 2};
let b = A {foo: 5, ..a};
```

But this has a caveat, we can't spread another value of a different type, even if it has exactly the same fields with the same types.

```rust
struct A {
  foo: u8,
  bar: u8
}
struct B {
  bar: u8
}

struct C {
  foo: u8,
  bar: u8
}
let b = B {bar: 2};
let c = C {foo: 1, bar: 2};

let d = A {foo: 5, ..c}; // Compiler: Sorry you can't do that, they need to have the same type
let a = A {foo: 5, ..a}; // Compiler: Can't do that either even if all the fields are filled
```

Especially in builder patterns with lots of options, you can end up with a lot of boilerplate just on the `.finish(self)` function
# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation
When you have 2 instances of 2 different structs, and need to convert between one and the other easily by initializing the fields with the same values. You'd usually do this:
```rust
struct A {
  foo: u8,
  bar: u8
}
struct B {
  foo: u8,
  bar: u8
}
let a = A { foo: 1, bar: 2 };
let b = B { foo: a.foo, bar: a.bar };
```

This example is small, but can get quite annoying if the struct is big.

We can do better! Introducing the (better?) spread operator:

Assuming we have 2 structs, and one is a partial of the other
```rust
struct A {
  foo: u8,
  bar: u8
}
struct B {
  foo: u8
}
```

We can now spread struct `B` inside struct `A` easily, and the compiler will tell us if we miss a field, or if some type is mismatched:

```rust
let b = B { foo: 1 };
let a = A { bar: 0, ..a }; // Works!
let c = A { ..a }; // Error: The field `bar` is uninitialized because struct `B` does not fill that field
```

We can now easily initialize fields with fields from other structs easily using the spread operator.
# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation
- The spread operator will not be constrained to values of the same type
- The 2 structs fields types must be the same
- Partial field spreading is allowed
	- Which will involve checking if all fields are filled and throwing an error if some field was not initialized
- The spread operator will be more flexible and ergonomic and basically feel like the spread operator in javascript (but more type-safe)
# Drawbacks
[drawbacks]: #drawbacks

This may cause some problems if the programmer forgets to override a field that the spread operator is filling.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives
- Don't add this on top of the spread operator, and just make a macro instead
- Use a different keyword for the better spread operator like maybe use a @ instead of ... so it's a little more explicit that that is happening
# Prior art
[prior-art]: #prior-art
- Spread operator in javascript, which spreads all the fields of an object inside of another object