- Feature Name: `type_changing_struct_update_syntax`
- Start Date: 2018-08-22
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

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
fields, with the exception of type inference and type checking, as described in
the following section. For example, the listing from the previous section

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

except for type inference and type checking.

## Type inference with struct update syntax

Given an instance of struct update syntax:

```rust
let updated = Struct {
    field1: expr1,
    field2: expr2,
    ..base
};
```

it is known that `updated` and `base` must be instances of `Struct`, but the
generic type parameters and lifetimes of `Struct` may need to be inferred for
`updated` and `base`. Note that in current Rust, the outer struct type
(`Struct` in this example) is always known, even if the identifier `Struct` is
replaced with a type alias or `Self`, but this may change if [RFC 2515] gets
merged.

Type inference of the generic type parameters and lifetimes will follow these
rules:

1. If the type of `updated` can be a subtype of the type of `base` without
   violating any constraints, then the type of `updated` is inferred to be a
   subtype of the type of `base`. This rule is evaluated simultaneously for all
   instances of struct update syntax within the inference context, and
   conflicts between applications of this rule should result in compilation
   errors.

2. If the type of `updated` cannot be a subtype of the type of `base` without
   violating any constraints, then the explicitly listed fields (`field1` and
   `field2` in the example) are inferred independently between `updated` and
   `base`. In other words, in this case, the example can be equivalently
   expanded into the following:

   ```rust
   let updated = Struct {
       field1: expr1,
       field2: expr2,
       kept_field1: (result of base).kept_field1,
       kept_field2: (result of base).kept_field2,
       kept_field3: (result of base).kept_field3,
   };
   ```

These rules preserve the inferred types of existing Rust code while minimizing
the assumptions type inference makes for the type-changing case.

For example, the inferred type of `updated` is `Foo<u8>` in the following
example in both current Rust and with this RFC:

```rust
struct Foo<A> {
    a: A,
    b: &'static str,
}

let base: Foo<u8> = Foo {
    a: 1,
    b: "hello",
};
let updated = Foo {
    a: 2,
    ..base
};
```

Since the type of `updated` can be a subtype of the type of `base` without
violating any constraints, it is inferred to be so. Note, in particular, that
this rule takes precedence over the Rust fallback integer type `i32` for the
`2` literal.

If the type of `updated` cannot be a subtype of the type of `base` without
violating any constraints, then inference for the explicitly listed fields is
handled independently between the base and updated instances. For example:

```rust
struct Foo<A, B> {
    a: A,
    b: B,
    c: i32,
}

let base = Foo {
    a: 1u8,
    b: 2u8,
    c: 3i32,
};
let updated = Foo {
    a: "hello",
    b: 2,
    ..base
}
```

In this case `base` has type `Foo<u8, u8>`. Since the type of `updated` cannot
be a subtype of the type of `base` due to the change in the type of the `a`
field, the types of `base.b` and `updated.b` are inferred independently. Since
`updated.b` is an unconstrained integer literal, it has the Rust integer
fallback type `i32`, and so `updated` is inferred to have type `Foo<&'static
str, i32>`.

The same behavior can also be caused by a broadening in a lifetime, since if
`updated` needs a broader lifetime than `base`, its type cannot be a subtype of
the type of `base. For example:

```rust
struct Foo<'a, B> {
    a: &'a (),
    b: B,
    c: i32,
}

let tup_stack = ();
let base = Foo {
    a: &tup_stack,
    b: 2u8,
    c: 3i32,
};
let tup_static: &'static () = &();
let updated: Foo<'static, _> = Foo {
    a: tup_static,
    b: 2,
    ..base
};
```

Since the lifetime of `&tup_stack` is shorter than the `'static` lifetime, the
type of `updated` cannot be a subtype of the type of `base`. As a result, the
types of `base.b` and `updated.b` are inferred independently. Since `updated.b`
is an unconstrained integer literal, it has the Rust integer fallback type
`i32`, and so `updated` is inferred to have type `Foo<'static, i32>`.

There is an edge case to rule 1 when there are multiple instances of struct
update syntax within a single inference context. For example:

```rust
struct Foo<A> {
    a: A,
    b: &'static str,
}

let base_u8: Foo<u8> = Foo {
    a: 1,
    b: "hello",
};
let base_u16: Foo<u16> = Foo {
    a: 1,
    b: "hello",
};

let a_unknown_int = 2;
let updated1 = Foo {
    a: a_unknown_int,
    ..base_u8
};
let updated2 = Foo {
    a: a_unknown_int,
    ..base_u16
};
```

Individually, the types of `updated1` and `updated2` can be subtypes of the
types of `base_u8` and `base_u16`, respectively. However, `updated1.a` and
`updated2.a` must have the same type because they are copies of
`a_unknown_int`. As a result, there is a conflict with both `updated1` and
`updated2` being subtypes of `base_u8` and `base_u16`, respectively, so a
compilation error should be generated.

# Drawbacks
[drawbacks]: #drawbacks

If the user does not know the type inference rules described earlier, type
inference may result in slightly surprising results for struct update syntax
with numeric literals. For example:

```rust
struct Foo<A, B> {
    a: A,
    b: B,
    c: i32,
}

let base = Foo {
    a: 1u8,
    b: 2u8,
    c: 3i32,
};
let updated = Foo {
    a: "hello",
    b: 2,
    ..base
}
```

In this case `base` has type `Foo<u8, u8>`, while `updated` is inferred to have
type `Foo<&'static str, i32>`. If users are not aware that changing the type of
`a` results in inferring `b` independently for `base` and `updated`, they may
be surprised that the type of `b` has changed from `u8` to `i32`. The same
behavior can also result from broadening lifetimes between `base` and
`updated`, which may be especially surprising because lifetimes are usually
implicit in Rust.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This proposal is a relatively small user-facing generalization that
significantly improves language ergonomics in some cases.

## Alternative type inference rules

A variety of alternative type inference rules are possible.

### Initially assume that individual fields do not change type

The inference rule proposed in this RFC means that if the type of *any* of the
fields changes, the types of *all* other explicitly listed fields are inferred
independently between the base and updated instances. Alternatively, type
inference could treat fields individually, assuming that the type of each
individual field is the same between the base and updated instances unless it
violates a constraint, regardless of the types of the other fields. Given this
example:

```rust
struct Foo<A, B> {
    a: A,
    b: B,
    c: i32,
}

let base = Foo {
    a: 1u8,
    b: 2u8,
    c: 3i32,
};
let updated = Foo {
    a: "hello",
    b: 2,
    ..base
}
```

the approach proposed in the RFC infers the type of `updated` to be
`Foo<&'static str, i32>`, while this alternative type inference rule infers the
type of `updated` to be `Foo<&'static str, u8>` because the type of `b` can be
the same in both `base` and `updated` without violating any constraints.

This alternative rule reduces the need for explicit type annotations in some
cases, but it may also be surprising to users who expect explicitly listed
fields to be inferred independently of the base instance.

### Disable `i32`/`f64` fallback for explicitly listed fields

When an integer or floating point literal is unconstrained in Rust, its type is
inferred to be the fallback of `i32` or `f64`. This could be slightly
surprising in combination with the type inference rules proposed in this RFC,
as described in the [Drawbacks][drawbacks] section. One possibility is to
disable the `i32`/`f64` fallback for explicitly listed fields in struct update
syntax, and instead throw a compilation error if the specific type of literal
cannot be inferred. This may avoid confusion caused by the recommended proposal
or the previously described alternative inference rule by throwing a
compilation error whenever the type is not clear.

### Always independently infer types of explicitly listed fields

The type inference rules proposed in this RFC assume that the type of the
updated struct matches the type of the base struct unless this assumption
violates any constraints. This preserves backwards compatibility with existing
code but is a special case that the user needs to be aware of. An alternative
approach that is more consistent with the rest of the language and doesn't
require any special cases is to always infer the types of explicitly listed
fields independently between the base and updated instances. This alternative
approach can break existing code in two ways:

1. It can require additional explicit type annotations in some cases. For
   example:

   ```rust
   struct Foo<T> {
       a: Vec<T>,
       b: i32,
   }

   let base: Foo<u8> = Foo {
       a: Vec::new(),
       b: 5,
   };
   let updated = Foo {
       a: Vec::new(),
       ..base
   };
   ```

   In current `Rust`, `updated` always has the same type as `base`, so no
   additional type annotations are necessary. With this alternative inference
   rule, the type of `a` in `updated` is inferred independently of the type of
   `a` in `base`, so it is ambiguous and an explicit type annotation is
   necessary.

2. It can change the inferred type of a struct instance in existing Rust code.
   For example,

   ```rust
   struct Foo<A> {
       a: A,
       b: i32,
   }

   let base: Foo<u8> = Foo {
       a: 1,
       b: 2,
   };
   let updated = Foo {
       a: 3,
       ..base
   };
   ```

   In current Rust, `updated` has type `Foo<u8>`. With this alternative
   inference rule, the type of `a` in `updated` is inferred independently of
   the type of `a` in `base`. Since it is an integer literal without any
   constraints, the type is inferred to be the Rust integer fallback `i32`. So,
   with the alternative inference rule, the type of `updated` would be
   `Foo<i32>` instead of `Foo<u8>` as it is in current Rust.

### Combination of always independently inferring types of explicitly listed fields and disabling `i32`/`f64` fallback

A combination of the previous two alternatives would still be a breaking change
by requiring additional type annotations in some cases, but it would not
silently change inferred types in existing code. All breakage would result in
easy-to-fix compile-time errors.

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

Should struct update syntax be further generalized to ignore the struct type
and just consider field names and field types? This question could be answered
later after users have experience with the changes this RFC. The further
generalization could be implemented in a backwards-compatible way.

What is the best inference rule? The proposal tries to strike a balance between
consistency, simplicity, and backwards compatibility, but one of the
alternative inference rules may be preferable.

Is the proposed inference rule practical to implement? With this rule, region
constraints can affect the inferred type, since checking if the type of
`updated` can be a subtype of the type of `base` requires checking region
constraints. This may be problematic since type inference is currently
implemented in separate phases for non-region and region constraints. One
possible workaround for this is to always infer lifetimes in explicitly listed
fields independently between `updated` and `base`, but would this be backwards
compatible?

[RFC 736]: https://github.com/rust-lang/rfcs/blob/master/text/0736-privacy-respecting-fru.md
[RFC 2515]: https://github.com/rust-lang/rfcs/pull/2515
