- Feature Name: pattern_synonyms
- Start Date: 2017-02-10
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

Pattern matching is very common in Rust and can be used in several places (such as match branches,
variable bindings and so on). It enables you to destructure a type and can even enables you to bind
internal variables. This RFC proposes an extension to the pattern matching mechanism with *pattern
synonyms*. Those are syntactic sugar and are intented to be ergonomic only. They can be used at
any place a regular pattern can be found.

# Motivation
[motivation]: #motivation

The main motivation is ergonomic code and convenience. In the same way that polymorphic types can be
partially applied, this RFC brings the possibility to partially apply a pattern.

```rust
struct Foo<A, B, C>(A, B, C)
type FooStringBool<C> = Foo<String, Bool, C>;
```

The `Foo` type has kind `* -> * -> * -> *` and `FooStringBool` has kind `* -> *`, because we
partially applied the `String` and `Bool` types. This type of convenience is often used in the `std`
lib with `Result`. For instance, have a look at [`std::io::Result`](https://doc.rust-lang.org/std/io/type.Result.html).

Now, consider:

```rust
enum Face<'a> {
  Point(&'a Point),
  Line(&'a Point, &'a Point),
  Polygon(&'a Point, &'a Point, &'a Point, &'a [&'a Point])
}

let face: Face = …;

match face {
  Face::Polygon(a, b, c, &[]) => println!("here’s a triangle!"),
  Face::Polygon(a, b, c, &[d]) => println!("here’s a quadrangle!"),
  _ => {}
}
```

The idea is to express the two branches of the match in a more readable way, like the following:

```rust
match face {
  Face::Triangle(a, b, c) =>  println!("here’s a triangle!"),
  Face::Quadrangle(a, b, c, d) => println!("here’s a quadrangle!"),
  _ => {}
}
```

# Detailed design
[design]: #detailed-design

In all the examples bellow, we’ll be refering to this code:

```rust
enum Face<'a> {
  Point(&'a Point),
  Line(&'a Point, &'a Point),
  Polygon(&'a Point, &'a Point, &'a Point, &'a [&'a Point])
}

let face: Face = …;
```

## Declaration

To declare a new pattern synonym, the keyword `pattern` is added to the language specification.

    pattern : "pattern" pat => pat;
    pat : Ident [ ident | [ ident , ] + ident ] ?

This is intented to be declared in an `impl` block only. This is required so that we know which type
the pattern refers to without having to explicitely write it down.

### Example

```rust
impl<'a> Face<'a> {
  /// A triangle.
  pattern Triangle(a, b, c) => Polygon(a, b, c, &[]);
  /// A quadrangle.
  pattern Quadrangle(a, b, c, d) => Polygon(a, b, c, &[d]);
}
```

## Use

The syntax to use a pattern is exactly the same as the one with regular patterns. You use them as
if they were real patterns.

### Example

```rust
match face {
  Face::Triangle(a, b, c) =>  println!("here’s a triangle!"),
  Face::Quadrangle(a, b, c, d) => println!("here’s a quadrangle!"),
  _ => {}
}
```

## Visibility from other modules

Pattern synonyms are not automatically imported when you `use` the item they refer to. You have to
explicitely ask for them, or use `::*`.

## Capturing

Patterns are generally connected to capturing. Pattern synonyms let you pass capture kind through
them down to the real pattern.

### Example

```rust
let p = ("hey!".to_owned(), "oh!".to_owned());

match &p {
  &(x, y) => …, // here, we cannot move out of borrow
  _ => {}
}
```

In order to compile this code, we have to tell rustc how we want to capture `x` and `y`. We can do
that immutably (`ref`) or mutably (`ref mut`):

```rust
match &p {
  &(ref x, ref y) => …,
  _ => {}
}
```

Because they’re just syntactic sugar, pattern synonyms enable you to specify which capture kind you
want:

```rust
match &face {
  &Face::Triangle(ref a, _, ref c) => …,
  _ => {}
}
```

# How We Teach This
[how-we-teach-this]: #how-we-teach-this

The prerequisites is obviously pattern matching. People should be comfortable with pattern matching.
Then, introducing pattern synonyms after the `type` keyword might be a good thing since they act a
bit like the same – aliases.

# Drawbacks
[drawbacks]: #drawbacks

I don’t see any yet.

# Alternatives
[alternatives]: #alternatives

The other alternative is to completely pattern match the pattern. In our case, instead of using
`Triangle` for instance, we’d use the following:

```rust
match face {
  Face::Polygon(a, b, c, &[]) => …,
  _ => {}
}
```

# Unresolved questions
[unresolved]: #unresolved-questions

## Accepted place to declare pattern synonyms

They are declared in an `impl` block. Still is to be determined whether we can declare ones in other
modules than the one the type is declared in.
