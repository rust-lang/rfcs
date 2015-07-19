- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

This is a resurrection/expansion of [an older RFC](https://github.com/rust-lang/rust/issues/8277).
It also requires [the `!` RFC](https://github.com/rust-lang/rfcs/pull/1216) as
a prerequisite.

Rust supports both kinds of composite algebraic data type: product types
(structs) and sum types (enums). Rust has anonymous structs (tuples) but no
anonymous enums, leaving a gap in it's type system.

                    named      anonymous
                 ------------------------
                 |           |          |
        products |  structs  |  tuples  |
                 |           |          |
                 ------------------------
                 |           |          |
            sums |   enums   |    ??    |
                 |           |          |
                 ------------------------
                 |           |          |
    exponentials | functions | closures |
                 |           |          |
                 ------------------------

This RFC proposes to add anonymous enums to Rust and suggests naming them
disjoins (as in disjoint unions).

```rust
let foo: (char | i32 | i32) = (!|!|123);
match foo {
  (c|!|!) => println!("char in position zero: {:?}", c),
  (!|i|!) => println!("i32 in position one: {}", i),
  (!|!|i) => println!("i32 in position two: {}", i),
};

let foo: (char|) = ('a'|);
match foo {
  (c|)  => println!("char in position zero: {:?}", c),
};

let foo: ! = panic!("no value");
match foo {
};

```

The syntax is chosen to look like tuples, but with pipes instead of commas to
signify OR instead of AND.

# Motivation

Disjoins are the natural extension of the anonymous empty enum type `!`.

Disjoins fill an analogous role to tuples. They're useful where the programmer
needs a single-use type who's usage will be localised to a small area of code.

For example, consider this code:

```rust
fn some_function() -> Result<i32, io::Error> {
  ...

  fn inner_helper_function() -> Result<char, (io::Error | Utf8Error)> {
    ...
  }

  match inner_helper_function() {
    Ok(c)  => ...,
    Err(e) => match e {
      (io_err|!)  => return Err(io_err),
      (!|u8_err)  => ...,
    },
  };
  ...
}
```

Here, defining a type `enum IoOrUtf8Error { ... }` would have been possible,
but would have been overkill because it would only have been used in one place.
The type would also had to have been defined somewhere outside of
`some_function` which would have spread out the relevant code and made it less
readable.

Disjoins are also useful in situations where having unnamed variants is the
natural choice. For example, with disjoins it would be possible to define a
one-hole-context type operator.

    ohc!(T, (i32, T, T)) ==> ((!, T, T) | (i32, (), T) | (i32, T, ()))

Disjoins will become especially useful if Rust ever adds a way to define
methods over generically-sized tuples. For example, it would also be possible
to write a function that selects an item from a tuple.

    match select((2i32, 'a', true)) {
        (i|!|!) => println!("got an i32: {:?}", i),
        (!|c|!) => println!("got a char: {:?}", c),
        (!|!|b) => println!("got a bool: {:?}", b),
    }

Disjoins would also be necessary if we ever wanted a way to extract the
representation of a type in the form of a type. Disjoins would be needed as the
anonymous form of enums:

    struct Foo { x: i32, y: char }
    enum Bar { X(i32), Y(char) }

    <Foo as Anonymous>::Anon == (i32, char)
    <Bar as Anonumous>::Anon == (i32 | char)

Like with tuples, disjoins should only be used where the meaning of the
variants will be obvious. For things like argument/return types on public
methods, named enums should be used instead.

# Detailed design

Disjoins have the exact same semantics as named enums and behave the same in
terms of code generation, representation etc.

# Drawbacks

Adds complexity to the type system and compiler.

# Alternatives

* Do Nothing.
* Some have suggested non-positional disjoint union types (ie. where (T | T) is
  isomorphic to T). However these aren't enums, aren't algebraic, and would add
  enormous complexity to Rust's type system compared to positional disjoint
  unions.
* Change the syntax? This might be necessary if the suggested syntax turns out
  to be ambiguous (see: Unresolved Questions). The `!` in disjoin expressions
  and patterns could be changed to another character although `!` already
  has connotations of "no value". Also the suggested syntax seems unpopular.

# Unresolved questions

Is the suggested syntax unambiguous? The '|' character is already used for
closures, bitwise OR and disjunctive match patterns. The '!' character is used
for the not-operator and negative trait bounds. I can't see any of these being
a problem but I'm not sure.

