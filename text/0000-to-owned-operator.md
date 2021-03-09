- Feature Name: to_owned_operator
- Start Date: 2021-03-09
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

We want to add the `@` unary prefix operator to function as an alias to `std::borrow::ToOwned`, along with an associated new trait in `std::ops::ToOwned` that is implemented for all types implementing `std::borrow::ToOwned`.

# Motivation
[motivation]: #motivation

New and experienced users of Rust alike may find the following patterns clunky:
```
let x = "test".to_owned(); // more commonly expressed as `to_string`
my_function("test".to_owned());
let y: Vec<u32> = [1, 2, 3].to_vec(); // more commonly expressed with the `vec!` macro, but can also be expressed as `to_owned`
let z = x.clone();
let z2 = t[0..1].to_vec(); // not uncommon to be explicitly referenced as (&t[0..1]).to_vec()
```
along with usage associated user defined `ToOwned` implementators.

These kinds of patterns could instead be expressed as:
```
let x = @"test";
my_function(@"test");
let y: Vec<u32> = @[1, 2, 3];
let z = @x;
let z2 = @t[0..1]; // or @&t[0..1]
```
This new syntax is more concise, which usage of the `@` character as an operator retains high visibility that a non-zero-cost operation is being performed (heap allocated generally).

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The `@` prefix operator is equivalent to calling the `to_owned` function of types implementing `std::borrow::ToOwned`.

Practically, this allows concise calling of `clone`, `str::to_string`, and `[T]::to_vec`. These functions are equivalent to their `ToOwned` counterpart, and all have the same operation of cloning/copying a value to the heap in the general case.

Any calls to `clone`, `str::to_string`, and `to_vec` may be replaced by usage of the `@` operator. This includes implementations on user defined types, provided the type implemented `ToOwned`.

Example:
```
let x = "test".to_string();
// would become
let x = @"test";
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

A new trait would be added to `std`: `std::ops::ToOwned`, which would be an alias trait, or a trait with no member definitions. All implementations of `std::borrow::ToOwned` would implement this trait through a blanket implementation.
```
// in std::ops module
pub trait ToOwned: std::borrow::ToOwned {}

impl<T: std::borrow::ToOwned> std::ops::ToOwned for T {}
```

A `ToOwned` corresponding unary operator would be added, mapped from `@`.

# Drawbacks
[drawbacks]: #drawbacks

* A very clear drawback of this proposal is the possibility for users not being aware of extraenous allocations being performed. The `@` operator was chosen due to it's high visiblity in order to mitigate, but not completely solve this issue.
* Due to usage of a blanket implementation, custom implementations of `std::ops::ToOwned` would not be possible. This is nominally ideal to avoid confusing misimplementation by a user, but a limitation that no other operator has.
* Usage of multiple unary operators is dense and potentially hard to decipher. I.e. `@&t`. Nominally usage of `@` followed by `&` is unnecessary -- a warning could be made, but other scenarios exist.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Why is this design the best in the space of possible designs?
* The `@` is maximally visible due to the shape/rareness of the character.
* Adding `std::ops::ToOwned` as an alias to `std::borrow::ToOwned` maintains convention of module for `ToOwned` implementors, while not being a breaking change.

## Other Design Considerations
* Having the `@` operator map directly to `std::borrow::ToOwned` is an option, but breaks convention.

## Impact of Not Doing
This RFC can not be implemented, and users can continue the current patterns established above.

# Prior art
[prior-art]: #prior-art

This feature or analogies to it do not exist in other languages.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

* Will this break any existing macros?
* Are there any conflicts with `@`'s use as a binary operator for named destructuring? (i.e. `if let x @ Some(_) = y`)
* Does having a generally relatively expensive and type-restricted operator with always-user-defined semantics work with Rustacean philosophy?
