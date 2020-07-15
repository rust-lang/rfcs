- Feature Name: `partial_closure_args`
- Start Date: 2020-07-14
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Pattern matching has this nice mechanism _partial pattern matching_: you can pattern match what you are
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

The `..` syntax is also used to state “everything else” when constructing values (however, this is out
of scope of this RFC):

```rust
fn reset_foo(x: Foo) -> Foo {
  Foo { x: 0., y: 0., ..x }
}
```

This RFC suggests to provide the same `..` ergonomics syntax to ignore closures’ arguments in the
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

As with tuples, it’s possible to pattern match up to the _nth_ argument, then ignore the rest:

```rust
|a, b, c, ..|
```

It is also possible to find `..` in the middle of the argument list, given all arguments are
provided at the beginning and the end of the list:

```rust
|a, .., z|
```

# Motivation
[motivation]: #motivation

Several functions / methods expect as argument another function, often expressed as a closure.
Sometimes, we want to ignore arguments and return a constant object whatever the arguments, or
simply are not interested in some of them. We are not often annoyed by this because the standard
library uses, most of the time, high-order unary functions, which means we can ignore the argument
with the simple `|_|` syntax — example: `Result::map_err`.

However, as we start building more complex applications, it happens that we have to provide n-ary
functions. In that case, for instance for a function with arity three (three arguments), the syntax
to ignore the arguments is `|_, _, _|`, which seems like a lot of noise to just express “ignore
everything”. Pattern matching tuples and structs has this mechanism, so it seems natural to extend
it to closure arguments as well.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## Partially pattern match closure arguments

As with pattern matching tuples and structs, it is possible to _partially pattern match_ closure
arguments by using the `..` in the list of arguments (possibly none). Example:

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

It is possible to mix underscores `_` with partial pattern matching as well with pattern matching
arguments you want in the same way you do with tuples and structs:

```rust
// with five arguments
x.map(|a, _, b, _, _| a + b);
x.map(|a, _, b, ..| a + b); // same as above
```

You can also use it in the middle of the arguments list, but that requires that all the arguments
to the right of `..` are filled until the end (without another `..` in the list):

```rust
x.map(|a, .., d, e| a + d + e);
x.map(|a, b, .., e| a + b + e);
```

The following, however, is not allowed:

```rust
x.map(|a, .., c, .., e| a + c + e); // error: cannot have two .. in a closure argument list
```

It is important to notice that `..` is not completely isomorphic to using `_` for all arguments. If
the arity of the function changes (for instance, it loses or gains arguments), the `..` syntax will
still compile while the `_` will obviously fail to match, which makes using `..` in lambda
arguments subject to breaking-changes if the type of the closure changes at the calling site.

## How we teach this

In the _Pattern Syntax_ section of the Rust book ([here](https://doc.rust-lang.org/book/ch18-03-pattern-syntax.html#ignoring-remaining-parts-of-a-value-with-),
some paragraphs should be added regarding pattern matching the arguments list by
using the `..` syntax.

The meaning of this syntax is equivalent to the tuple syntax:

```rust
let (a, b, ..) = (1, 2, 3, 4);
x.map(|a, b, ..| a + b);
```

The feature is not about [variadic functions](https://en.wikipedia.org/wiki/Variadic_function): the
arity is well-defined, so are the types of all of its arguments. Consider `..` as syntactic sugar for
ignoring zero to several arguments.

People should think of `..` as _“ignore everything else”_. So:

```rust
|a, ..|
```

is a closure that captures its first argument and ignore all the rest, whatever its arity. The
following:

```rust
|..|
```

Is a closure that will never read any of its arguments (if any). This:

```rust
|s, .., e|
```

Is a closure that captures its first `s` argument and its last `e` argument, and ignore all the
rest.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Interaction with type inference

A closure containing a `..` – as in `|a, b, .., x|` – is not type-inferable (there is no way to
detect which trait it implements). Closures with `..` can only be used in places where a type is
already constrained by one of the `FnOnce`, `Fn` or `FnMut` traits. The following is, thus, not
authorized:

```rust
let f = |a, b, ..| a + b;

println!("{}", f(1, 2)); // error: cannot deduce type of f
```

## Emptiness meaning

Using `..` when all arguments have been matched or ignored (`_`) is authorized as in tuple / structs:

```rust
let t = (1, 2);
let (a, b, ..) = t; // okay
let (a, .., b) = t; // still okay

Some(123).map(|x, ..| x + 1); // okay too
Some(123).map(|.., x| x + 1); // okay; same
```

The corollary authorizes to use `..` with functions taking no arguments:

```rust
Some(32).unwrap_or_else(|..| 0); // okay
```

This property is only there for convenience, as it might be useful when writing macro code.

## Variadic functions and arity

Variadic functions currently don’t exist in Rust. However, the
[RFC-2137](https://github.com/rust-lang/rfcs/pull/2137) introduces them for the FFI (C). The
syntax that seemed to have been accepted is the triple dot `...` syntax.

In the case of our RFC, we use the “ignore everything else” syntax; the double dot `..`. This
doesn’t a closure using `..` variadic. When reading this:

```rust
|a, b, ..|
```

one could be tempted to think it’s a closure taking two parameters and then is a variadic in the
rest, but this is not what it means. The arity is well fixed, even though we don’t know it / we
do not care while writing the closure.

## Errors, warnings

### Resolving ambiguities

Using `..` more than once result in ambiguities and prevent rustc from compiling the closure.
Example:

```rust
let f = |.., a, ..| a;
```

There is no way to know which arguments `a` binds to, so this is ambiguous. The error message
should include information similar to the error message with tuples:

```
error: `..` can only be used once per closure argument pattern
```

### Typing

Using `..` always makes a closure non-type-inferrable via its arguments only — i.e. it is not
possible to know which type the closure has based only on the arguments and the captured
environment in this case: it is ambiguous. Consider:

```rust
let f = |a, b, ..| a + b;
```

`f` could be a `Fn(u32, u32) -> u32` closure, or even a
`Fn(u32, u32, String, bool, Option<f32>) -> u32`. This ambiguity should be resolved by inferring the
type at call site based on explicit type ascriptions:

```rust
fn call<F>(f: F) where F: Fn(u32, u32, String, bool) -> u32;

let r = call(|a, b, ..| a + b);
```

In this case, the closure is passed in a place where the constraint is already known.

When using a closure with `..` without the explicit constraints, rustc should error out that
`..`-based closures cannot infer their types based solely on the arguments and captured
environment:

```
error: closures wich argument list contains `..` can only be used at a constrained call-site (
FnOnce, Fn, FnMut)
```

# Drawbacks
[drawbacks]: #drawbacks

## Weirdness of empty ..

Some people might be confused with the difference between `|_|` and `|..|`, or even with `||` and
`|..|`. However, this is the same problem as with pattern matching regular types, so it shouldn’t
be too surprising.

On the same level, these two seem surprising at first:

```rust
Some(3).map(|a, ..| a + 1);
Some(3).map(|.., a| a + 1);
```

However, they have to be read the exact same way the following is read:

```rust
let (a, .., b) = (1, 2);
let (.., a, b) = (1, 2);
let (a, b, ..) = (1, 2);
```

This is an odd syntax and permission but is allowed for code generation, so we also want to replicate it.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This is an ergonomics change that is not needed to solve any technical problems but only adds up
more comfort to how to use Rust. More specifically, it makes the syntax coherent with how
pattern matching is done everywhere else in the language, closing the gap a bit more between
pattern matching a tuple or a string and pattern matching a closure arguments.

The typical alternative to use `..` in closure arguments is to use a tuple. Indeed, the syntax is
almost the same:

```rust
|a, ..|
|(a, ..)|
```

However, this has a huge impact on the arity of the function, while this proposal doesn’t change
the arity of the function and simply adds syntactic sugar.

# Prior art
[prior-art]: #prior-art

- The `..` syntax in the current Rust language for structs and tuples. See
  [this section](https://doc.rust-lang.org/book/ch18-03-pattern-syntax.html#ignoring-remaining-parts-of-a-value-with-).

# Unresolved questions
[unresolved-questions]: #unresolved-questions

1. Constructing values with `..` allows to perform a diff-like update by explicitly setting fields and
leaving the `..` figure out the rest and forward them. The application of this syntax has not been
explored in the context of closure arguments.

# Future possibilities
[future-possibilities]: #future-possibilities

About the [unresolved-questions]’s first point, one can imagine creating a closure “pack” arguments
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
