- Feature Name: `derefered_composite_types`
- Start Date: 2023-04-06
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)


# Summary
[summary]: #summary

The proposal of Derefered Composite Types is a Rust unification of Refered Types and Derefered Types.

Derefered Composite Types could help also to get rid of constructor boilerplate.

Symbol `*` before the type name is a marker to Dereferd Composite Type. 

The new operator `=&` (and `=&&`, `=&&&`, ...) as a compound of borrow and assignment (`x = &2` same as `x =& 2`) is also required.


# Motivation
[motivation]: #motivation

Currently, Rust has 
- (1) derefered primitive types (like `i32'`)
- (2) refered primitive types (like `&i32'`) 
- (3) refered composite types (like `Box<i32>`)

But there are no any derefered composite types.

This is not universal. We wish to improve this. So we propose types like `*Box<i32>`.


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Currently we could write:
```rust
let foo : u32 = 5;  // derefered primitive type ~ *Stack<u32>
let baz : &u32 = &5;  // refered primitive type ~ Stack<u32> == &*Stack<u32>
let bar : Box<u32> = Box::new(5);  // refered composite type

let foo2 : u32 = 5 + *bar;
```

It is possible to write derefered composite type with new syntax:
```rust
let bar2 : *Box<u32> = 5;  // derefered composite type
let foo3 : u32 = 5 + bar2;
```

We expect, that `Box<T> == &*Box<T>`.

Sure, to use composite dereferd types, those types must implement 2 traits: `Deref` and new `Construct`
```rust
impl<T> Deref .... fn deref(&self)
impl<T> Construct .... fn construct(&self)
```

That allow also to get rid of constructor boilerplate
```rust
let foo : *String = "some string";  // we get rid of String::from("some string")
let bar : *Box<u32> = 5;    // we get rid of Box::new(5)
let foo : *Box<*String> = "some string";  // we get rid of Box::new(String::from("some string"))
```

The `&=` operator is already in use and it has meaning bitwise and assignment.

It is required to include a compound borrow and assignment operator `=&` (and `=&&`, `=&&&`, ...) for use with derefed composite types.

We could also use derefered types for refered types with new operator (`x = &2` same as `x =& 2`):
```rust
let foo : String =& "some string";  // free transmute &*String to String
let bar : Box<u32> =& 5;    // free transmute &*Box<u32> to Box<u32>
let foo : Box<String> =&& "some string";  // free transmute &*Box<&*String> to Box<String>
```


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation


# Drawbacks
[drawbacks]: #drawbacks

Rust has a hack of `&str` type, which technically is a `str == *Str` type in terms of Dereferd Composite Types.

With this proposal we must admit additional type hack `&str == str`.


# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

None known.


# Future possibilities
[future-possibilities]: #future-possibilities

This feature allows to use more universal General Types such as `*T`.


# Prior art
[prior-art]: #prior-art

None known.


# Unresolved questions
[unresolved-questions]: #unresolved-questions

- None so far.
