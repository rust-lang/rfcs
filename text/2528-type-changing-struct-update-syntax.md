- Feature Name: `type_changing_struct_update_syntax`
- Start Date: 2018-08-22
- RFC PR: https://github.com/rust-lang/rfcs/pull/2528
- Rust Issue: https://github.com/rust-lang/rust/issues/86555

# Summary
[summary]: #summary

Extend struct update syntax (a.k.a. functional record update (FRU)) to support
instances of the *same* struct that have different types due to generic type or
lifetime parameters. Fields of different types must be explicitly listed in the
struct constructor, but fields of the same name and same type can be moved with
struct update syntax.

This will make the following possible. In this example, `base` and `updated`
are both instances of `Foo` but have different types because the generic
parameter `T` is different. Struct update syntax is supported for `field2`
because it has the same type `i32` in both `base` and `updated`:

```rust
struct Foo<T, U> {
    field1: T,
    field2: U,
}

let base: Foo<String, i32> = Foo {
    field1: String::from("hello"),
    field2: 1234,
};
let updated: Foo<f64, i32> = Foo {
    field1: 3.14,
    ..base
};
```

# Motivation
[motivation]: #motivation

In today's Rust, struct update syntax is a convenient way to change a small
number of fields from a base instance as long as the updated instance is a
subtype of the base (i.e. the *exact same* type except lifetimes). However,
this is unnecessarily restrictive. A common pattern for implementing
type-checked state machines in Rust is to handle the state as a generic type
parameter. For example:

```rust
struct Machine<S> {
    state: S,
    common_field1: &'static str,
    common_field2: i32,
}

struct State1;
struct State2;

impl Machine<State1> {
    fn into_state2(self) -> Machine<State2> {
        // do stuff
        Machine {
            state: State2,
            common_field1: self.common_field1,
            common_field2: self.common_field2,
        }
    }
}
```

It would be much more convenient to be able to write

```rust
Machine {
    state: State2,
    ..self
}
```

instead of

```rust
Machine {
    state: State2,
    common_field1: self.common_field1,
    common_field2: self.common_field2,
}
```

but this is not possible in current Rust because `Machine<State1>` and
`Machine<State2>` are different types even though they are both the `Machine`
struct.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

It's often useful to create a new instance of a struct that uses most of an old
instance's values but changes some. You can do this using struct update syntax.

Consider a `User` type that can be in either the `LoggedIn` state or the
`LoggedOut` state and has a few additional fields describing the properties of
the user.

```rust
struct User<S> {
    state: S,
    email: String,
    username: String,
}

struct LoggedIn;
struct LoggedOut;
```

Let's say we have a logged-out user:

```rust
let logged_out = User {
    state: LoggedOut,
    email: String::from("ferris@example.com"),
    username: String::from("ferris"),
};
```

This example shows how we create a new `User` instance named `logged_in`
without the update syntax. We set a new value for `state` but move the values
of the other fields from `logged_out`.

```rust
let logged_in = User {
    state: LoggedIn,
    email: logged_out.email,
    username: logged_out.username,
};
```

Using struct update syntax, we can achieve the same effect more concisely, as
shown below. The syntax `..` specifies that the remaining fields not explicitly
set should be moved from the fields of the base instance.

```rust
let logged_in = User {
    state: LoggedIn,
    ..logged_out
};
```

Note that the expression following the `..` is an *expression*; it doesn't have
to be just an identifier of an existing instance. For example, it's often
useful to use struct update syntax with `..Default::default()` to override a
few field values from their default.

Struct update syntax is permitted for instances of the *same* struct (`User` in
the examples), even if they have different types (`User<LoggedOut>` and
`User<LoggedIn>` in the examples) due to generic type or lifetime parameters.
However, the types of the fields in the updated instance that are not
explicitly listed (i.e. those that are moved with the `..` syntax) must be
subtypes of the corresponding fields in the base instance, and all of the
fields must be visible ([RFC 736]). In other words, the types of fields that
are explicitly listed can change, such as the `state` field in the examples,
but those that are not explicitly listed, such as the `email` and `username`
fields in the examples, must stay the same (modulo subtyping).

Existing Rust programmers can think of this RFC as extending struct update
syntax to cases where some of the fields change their type, as long as those
fields are explicitly listed in the struct constructor.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Struct update syntax is now allowed for instances of the *same* struct even if
the generic type parameters or lifetimes of the struct are different between
the base and updated instances. The following conditions must be met:

1. The base and updated instances are of the same struct.

2. The type of each moved field (i.e. each field not explicitly listed) in the
   updated instance is a subtype of the type of the corresponding field in the
   base instance.

3. All fields are visible at the location of the update ([RFC 736]).

The struct update syntax is the following:

```rust
$struct_name:path {
    $($field_name:ident: $field_value:expr,)*
    ..$base_instance:expr
}
```

Struct update syntax is directly equivalent to explicitly listing all of the
fields, with the possible exception of type inference. For example, the listing
from the previous section

```rust
let logged_in = User {
    state: LoggedIn,
    ..logged_out
};
```

is directly equivalent to

```rust
let logged_in = User {
    state: LoggedIn,
    email: logged_out.email,
    username: logged_out.username,
};
```

except, possibly, for type inference.

# Drawbacks
[drawbacks]: #drawbacks

There are trade-offs to be made when selecting the type inference strategy,
since the types of fields are no longer necessarily the same between the base
and updated instances in struct update syntax. See the *Type inference* section
under [Unresolved questions](#unresolved-questions).

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This proposal is a relatively small user-facing generalization that
significantly improves language ergonomics in some cases.

## Further generalization

This proposal maintains the restriction that the types of the base and updated
instance must be the same struct. Struct update syntax could be further
generalized by lifting this restriction, so that the only remaining restriction
would be that the moved field names and types must match. For example, the
following could be allowed:

```rust
struct Foo {
    field1: &'static str,
    field2: i32,
}

struct Bar {
    field1: f64,
    field2: i32,
}

let foo = Foo { field1: "hi", field2: 1 };
let bar = Bar { field1: 3.14, ..foo };
```

While this would be convenient in some cases, it makes field names a much more
important part of the crate's API. It could also be considered to be too
implicit.

The proposal in this RFC does not preclude this further generalization in the
future if desired. The further generalization could be applied in a manner that
is backwards-compatible with this RFC. As a result, the conservative approach
presented in this RFC is a good first step. After the community has experience
with this proposal, further generalization may be considered in the future.

## Keep the existing behavior

If we decide to keep the existing behavior, we are implicitly encouraging users
to handle more logic with runtime checks so that they can use the concise
struct update syntax instead of the verbose syntax required due to type
changes. By implementing this RFC, we improve the ergonomics of using the type
system to enforce constraints at compile time.

# Prior art
[prior-art]: #prior-art

OCaml and Haskell allow changing the type of generic parameters with functional
record update syntax, like this RFC.

* OCaml:

  ```ocaml
  # type 'a foo = { a: 'a; b: int };;
  type 'a foo = { a : 'a; b : int; }
  # let x: int foo = { a = 5; b = 6 };;
  val x : int foo = {a = 5; b = 6}
  # let y: float foo = { x with a = 3.14 };;
  val y : float foo = {a = 3.14; b = 6}
  ```

* Haskell:

  ```haskell
  Prelude> data Foo a = Foo { a :: a, b :: Int }
  Prelude> x = Foo { a = 5, b = 6 }
  Prelude> :type x
  x :: Num a => Foo a
  Prelude> y = x { a = 3.14 }
  Prelude> :type y
  y :: Fractional a => Foo a
  ```

Like this RFC, OCaml does not allow the alternative further generalization:

```ocaml
# type foo = { a: int; b: int };;
type foo = { a : int; b : int; }
# type bar = { a: int; b: int };;
type bar = { a : int; b : int; }
# let x: foo = { a = 5; b = 6 };;
val x : foo = {a = 5; b = 6}
# let y: bar = { x with a = 7 };;
File "", line 1, characters 15-16:
Error: This expression has type foo but an expression was expected of type
         bar
```

# Unresolved questions
[unresolved-questions]: #unresolved-questions

## Type inference

What is the best type inference strategy? In today's Rust, the types of the
explicitly listed fields are always the same in the base and updated instances.
With this RFC, the types of the explicitly listed fields can be different
between the base and updated instances. This removes some of the constraints on
type inference compared to today's Rust. There are choices to make regarding
backwards compatibility of inferred types, the `i32`/`f64` fallback in type
inference, and the conceptual simplicity of the chosen strategy.

## Further generalization

Should struct update syntax be further generalized to ignore the struct type
and just consider field names and field types? This question could be answered
later after users have experience with the changes this RFC. The further
generalization could be implemented in a backwards-compatible way.

[RFC 736]: https://github.com/rust-lang/rfcs/blob/master/text/0736-privacy-respecting-fru.md
