- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

This is a resurrection/expansion of [an older RFC](https://github.com/rust-lang/rust/issues/8277).

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

Disjoins fill an analogous role to tuples. They're useful where the programmer needs a single-use type who's usage will be localised to a small area of code.

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

Like with tuples, disjoins should only be used where the meaning of the
variants will be obvious. For things like argument/return types on public
methods, named enums should be used instead.

Another motivation for disjoins is that they require a syntax for the empty
disjoin type. This makes them the perfect vehicle for bringing `!` into the
type system.

## The `!` type

I've prosthelytised before that `!` should be treated as a type but have been met
with skepticism. So I'll start with what I think is an accurate analogy. In
C/C++, `void` is in essence a type like any other. It is the trivial type with
one value, equivalent to a struct type with no members and can be trivially
cast from any other type (by simply throwing away the value).  However it can't
be used in all the normal positions where a type can be used. This breaks
generic code (eg. `T foo(); T val = foo()` where `T == void`) and forces one to
use workarounds such as defining `struct Void {}` and wrapping `void`-returning
functions:
  
```c
Void foo_wrap() {
  foo();
  return {};
};
```

If in the 1960s, when PL theory was young, someone had suggested to Dennis
Ritchie to allow `void` to be used a function argument, as a struct member and
everywhere else a type could be used, he may well have said something like
"That doesn't make sense. `void` isn't a type, it's just a special syntax for
declaring functions with no return value, why would you want a `void` function
argument anyway?". So instead, the `void` type gets treated as a second class
citizen, adding extra hassle and complexity to the language for no benefit.
  
Fast-forward fifty years...

Rust, building on decades of experience, decides to fix C's shortsightedness
and bring `void` into the type system in the form of the empty tuple `()`.
Rust also introduces a new composite data kind, dual to the notion of structs,
in the form of enums. These are a somewhat innovative feature, most major
languages don't have them (eg. C), and those that do often don't permit empty
enums like Rust does (eg. Haskell). However Rust also introduces a syntax for
declaring functions that never return: `fn() -> !`. Here, `!` is in essence a
type like any other. It is the trivial type with no values, equivalent to an
enum type with no variants, and can be trivially cast to any other type
(because it has no values).  However it can't be used in all the normal
positions where a type can be used. This breaks generic code (eg. `fn foo() ->
T; let val: T = foo()` where `T == !`) and forces one to use workarounds such
as defining `enum Void {}` and wrapping `!`-returning functions.

```rust
fn foo_wrap() -> Void {
  foo()
};
```

However when it's suggested to allow `!` to be used as a function argument, as
a struct member and everywhere else a type can be used people often respond
with something like "That doesn't make sense. `!` isn't a type, it's a special
syntax for declaring functions that don't return, why would you want a `!`
function argument anyway?". So instead, the `!` type gets treated as a second
class citizen, adding extra hassle and complexity to the language for no
benefit.

`!` has a meaning in any situation that any other type does. A `!` function
argument makes a function uncallable, a `Vec<!>` is a vector that can never
contain an element, a `!` enum variant makes the variant guaranteed never to
occur and so forth. It might seem pointless to use a `!` function argument or a
`Vec<!>` (just as it would be pointless to use a `()` function argument or a
`Vec<()>`), but that's no reason to disallow it. And generic code sometimes
requires it. There's at least two cases of this:
  * Functions of the type `fn() -> !` cannot satisfy the `Fn() -> T` trait. As
    such diverging functions can't be passed to generic higher-order functions.
  * There is no standard way to express types like `Result<T, !>` causing 
    library authors to implement their own, incompatible implementations of
    `enum Void {}`. (`Result<T, !>` is useful when implementing a trait method
    of type `Result<T, E>` and you know the implementation will never return
    `Err`)

Promoting `!` to a type and allowing it to unify with all other types would
give it the same behavior as the current `!` syntax, except generalized,
allowing more kinds of correct code to exist.

It's worth making clear the difference between `struct Void {}` (ie `void`, ie.
`()`) and `enum Void {}` (ie. `!`). We can think of `struct` and `enum` as
being dual type operators where `struct {A, B, C}` is `A AND B AND C` and `enum
{A, B, C}` is `A OR B OR C`. `()` and `!` are then the identities of these
operators in the sense that adding a `()` member to a struct does not change
the overall structure of the type (because the member is always instantiated
with `()` and thus does nothing) and adding a `!` variant to an enum does not
change the overall structure of the type (because the member can never be
instantiated and thus does nothing). The reason `()` and `!` have historically
been neglected as types is that they're so trivial that it doesn't occur to
people that they even *are* types. `()` only has one value and so it's not
interesting. Something that can only be in one state cannot carry information
and a type with that doesn't carry any information has no obvious use in a
language. `!` has no values and so it's not interesting. A type with no values
can never exist and a type that can never exist has no obvious use in a
language.

However they are types, they have their use cases and an algebraic type system
is not complete without both of them.

# Detailed design

Disjoins have the exact same semantics as named enums and behave the same in
terms of code generation, representation etc. Code that handles the empty
disjoin type should be marked unreachable and eliminated where possible.
Functions that return it should be marked with the llvm `NoReturn` attribute.
The above should also apply to empty named enums.

The typechecker should allow `!` to unify with all other types.

The existing compiler support for diverging functions (eg. `FnDiverging`)
should be removed/replaced.

# Drawbacks

Adds it's own complexity to the type system and compiler.

# Alternatives

* Do Nothing.
* Some have suggested non-positional disjoint union types (ie. where (T | T) is
  isomorphic to T). However these aren't enums, aren't algebraic, and would add
  enormous complexity to Rust's type system compared to positional disjoint
  unions.
* Change the syntax? This might be necessary if the suggested syntax turns out
  to be ambiguous (see: Unresolved Questions). The `!` in disjoin expressions
  and patterns could be changed to another character although `!` already
  has connotations of "no value".
* Just promote `!` to a type. Although `!` fits naturally into a scheme
  of anonymous enums this would still be a worthwhile change if done
  independently. This would be a smaller change in the sense that it would
  involve removing restrictions on an existing feature rather than adding a
  whole new feature.

# Unresolved questions

Is the suggested syntax unambiguous? The '|' character is already used for
closures, bitwise OR and disjunctive match patterns. The '!' character is used
for the not-operator and negative trait bounds. I can't see any of these being
a problem but I'm not sure.

Could `!` be treated as a subtype of all other types? Theoretically, yes, but
the implementation might be a headache. Allowing `!` to unify with all other
types would be simpler and good enough for most cases. And we don't treat `()`
as a supertype of all other types either.

