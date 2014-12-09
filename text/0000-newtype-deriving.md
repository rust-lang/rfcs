- Start Date: 2014-12-09
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Introduce a `#[newtype_deriving(...)]` syntax extension that allows a newtype
to derive traits of the wrapped type.

# Motivation

Newtypes are very useful to avoid mixing similar types with different
semantics. The typical example is a `struct Centimeters(f64)` and a
`struct Inches(f64)`.

A big problem with the usage of newtypes is that you need to manually implement
the traits of the wrapped type in order to use them. In the previous example,
you cannot add two `Centimeters` unless you implement the `Add` trait for the
struct (which is trivial, but verbose). This causes a lot of unnecessary
boilerplate which discourages using newtypes and makes it very painful when you
have no alternative.

# Detailed design

Introduce a new syntax extension similar to `#[deriving(...)]` but for
newtypes. A possible name is `#[newtype_deriving(...)]`.

Example:

```rust
#[newtype_deriving(Add, Sub, Mul, Div)]
struct Centimeters(f64);

fn do_something(cm: Centimeters) -> Centimeters {
    cm + cm * cm / cm - cm
}
```

Deriving a trait is trivial once you know the required functions and their
signatures. A first approach would be to hardcode this information like it
is done in the `#[deriving(...)]` syntax extension. However, if the first
unresolved question is solved (see below), we could derive any trait including
user-defined ones.

In case the first approach is chosen, the traits available to be derived would be:
* All traits in `std::ops`
* Show

Note that the comparison operators, serialization, Clone, Hash, Rand, Default
and FromPrimitive don't need to be included since they can be derived using
`#[deriving(...)]`.

# Drawbacks

It adds complexity to the language.

If a trait is modified, this syntax extension would need to be updated as well.
In case we can implement *arbitrary trait deriving* this would not be an issue,
since the syntax extension could look at the trait definition (see unresolved
questions).

# Alternatives

We could extend the current `#[deriving(...)]` syntax extension to handle the
case of newtypes. This could cause ambiguity (e.g. `#[deriving(Show)`]
could produce a `"Centimeter(5.0)"` as output or just `"5.0"`).

Do nothing and let future IDEs generate the boilerplate for us.

# Unresolved questions

1. According to [this comment]
(https://github.com/rust-lang/rust/issues/19597#issuecomment-65909118),
a syntax extension has no access to the function signatures of a trait. If
this changes, we could derive any trait (also user-defined ones) for a newtype.
Is it possible to allow syntax extensions to see the (required) function
signatures of a trait? How difficult would it be?

2. In case 1 cannot be solved, which traits should be available to be derived?
