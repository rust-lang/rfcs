- Feature Name: `partial_lambda_args`
- Start Date: 2020-07-14
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Pattern matching has this nice mechanism to be _partial_: you can pattern match what you are
interested in, and then use the `..` syntax to _ignore everything else_. Example:

```rust
let (a, b, ..) = (1, true, "Hello, world!"); // here

struct Foo {
  x: u32,
  y: u32,
  id: String,
}

let foo = Foo { x: 0, y: 0, id: "Hello, world!".into() };
let Foo { id, .. } = foo; // here
```

The `..` syntax is also used to state “everything else” when constructing values:

```rust
fn reset_foo(x: Foo) -> Foo {
  Foo { x: 0., y: 0., ..x }
```

This RFC suggests to provide the same `..` ergonomics syntax to ignore  lambda’s argument in the
same way it works with tuples:

```rust
struct Point2D {
  x: f32,
  y: f32,
}

impl Point2D {
  fn map(self, f: impl FnOnce(f32, f32) -> (f32, f32)) -> Self {
    let (x, y) = f(self.x, self.y);
    Point2D { x, y }
  }
}

let x = Point2D { x: 123., y: -7. };
let a = x.map(|..| (0., 0.)); // here
```

As with tuples, it’s possible to pattern-match up to the _nth_ argument, then ignore the rest:

```rust
|a, b, c, ..|
```

# Motivation
[motivation]: #motivation

Several functions / methods expect as argument another function, often expressed as a lambda.
Sometimes, we want to ignore arguments and return a constant object whatever the arguments. We are
not often annoyed by this because the standard library uses, most of the time, high-order unary
functions, which means we can ignore the argument with the simple `|_|` syntax — example:
`Result::map_err`.

However, as we start building more complex applications, it happens that we have to provide n-ary
functions. In that case, for instance for a function with arity three (three arguments), the syntax
to ignore the arguments is `|_, _, _|`, which seems like a lot of noise to just express “ignore
everything”. Pattern-matching tuples and structs has this mechanism, so it seems natural to extend
it to lambda arguments as well.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

As with pattern-matching tuples and structs, it is possible to _partially pattern match_ lambda
arguments by using the `..` after the last pattern-matched argument (possibly none). Example:

```rust
// without argument
x.map(|| 123);
x.map(|..| 123); // same as above

// unary
x.map(|_| 123);
x.map(|..| 123); // same as above

// binary
x.map(|_, _| 123);
x.map(|..| 123); // same as above

// with five arguments
x.map(|_, _, _, _, _| 123);
x.map(|..| 123); // same as above
```

It is possible to mix underscore `_` with partial patter-matching as well with pattern-matching
arguments you want in the same way you do with tuples and structs:

```rust
// with five arguments
x.map(|a, _, b, _, _| a + b);
x.map(|a, _, b, ..| a + b); // same as above
```

It is important to notice that `..` is not completely isomorphic to using `_` for all arguments. If
the arity of the function changes (for instance, it loses or gains arguments), the `..` syntax will
still compile while the `_` will obviously fail to match.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Using `..` when all arguments have been matched or ignored (`_`) is authorized as in tuple / structs:

```rust
let t = (1, 2);
let (a, b, ..) = t; // okay

Some(123).map(|x, ..| x + 1); // okay too
```

The corollary authorizes to use `..` with functions taking no arguments:

```rust
Some(32).unwrap_or_else(|..| 0); // okay
```

However, `..` must always be found at the end of the list of arguments, if any others are specified:

```rust
// with two arguments
x.map(|.., b| b); // compiler error: .. cannot partially pattern-match while .. is not at the end
                  // of the arguments list
```

# Drawbacks
[drawbacks]: #drawbacks

Some people might be confused with the difference between `|_|` and `|..|`, or even with `||` and
`|..|`. However, this is the same problem as with pattern-matching regular types, so it shouldn’t
be too surprising.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This is en ergonomics change that is not needed to solve any technical problems but only adds up
more comfort to how to use Rust. More specifically, it makes the syntax coherent with how
pattern-matching is done everywhere else in the language, closing the gap a bit more between
pattern matching a tuple or a string and pattern-matching a lambda arguments.

# Prior art
[prior-art]: #prior-art

N/A

# Unresolved questions
[unresolved-questions]: #unresolved-questions

1. Constructing values with `..` allows to perform a diff-like update by explicitly setting fields and
leaving the `..` figure out the rest and forward them. The application of this syntax has not been
explored in the context of lambda arguments.

# Future possibilities
[future-possibilities]: #future-possibilities

About the [unresolved-questions]’s first point, one can imagine creating a lambda “pack” arguments
to capture arguments and forward them in a syntaxic way:

```rust
// current code
x.map(|x, y, z| foo(123, x, y, z));

// possible future code
x.map(|..| foo(123, ..));
```

If this syntax is too obscure, one could imagine naming the arguments (by prefixing `..` with the
name) and unpacking them (by suffixing `..` with the name):

```rust
x.map(|args..| foo(123, ..args));
```

This idea is not part of this RFC and should be explored in another RFC.
