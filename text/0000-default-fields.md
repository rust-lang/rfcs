- Feature Name: Default Fields
- Start Date: 2016-12-03
- RFC PR: 
- Rust Issue: 

# Summary
[summary]: #summary

Allow `struct` definitions to supply default values for individual fields, and then allow those fields to be omitted from `struct` initialisation:

```rust
struct Foo {
    a: &'static str,
    b: bool = true,
    c: i32,
}

let foo = Foo {
    a: "Hello",
    c: 42,
    ..
};
```

# Motivation
[motivation]: #motivation

Rust allows you to create an instance of a `struct` using a literal syntax. This requires that all fields in the `struct` are assigned a value, so it can be inconvenient for large `struct`s whose fields usually receive the same values. Literals also can't be used to initialise `struct`s with innaccessible private fields. Functional record updates can reduce noise when a `struct` derives `Default`, but are also invalid when the `struct` has private fields.

To work around these shortcomings, you can create constructor functions:

```rust
struct Foo {
    a: &'static str,
    b: bool,
    c: i32,
}

impl Foo {
    // Constructor function.
    fn new(a: &'static str, c: i32) -> Self {
        Foo {
            a: a,
            b: true,
            c: c
        }
    }
}

let foo = Foo::new("Hello", 42);
```

The problem with a constructor is that you need one for each combination of fields a caller can supply. To work around this, you can use builders, like [`process::Command`](https://doc.rust-lang.org/stable/std/process/struct.Command.html) in the standard library. Builders enable more advanced initialisation, but need additional boilerplate.

This RFC proposes a solution to improve `struct` literal ergonomics, so they can be used for `struct`s with private fields, and to reduce initialisation boilerplate for simple scenarios. This is achieved by letting callers omit fields from initialisation when a default is specified for that field. This syntax also allows allows fields to be added to `struct`s in a backwards compatible way, by providing defaults for new fields.

Field defaults allow a caller to initialise a `struct` with default values without needing builders or a constructor function:

```rust
struct Foo {
    a: &'static str = "Hello",
    b: bool = true,
    c: i32 = 42,
}

// Overriding a single field default
let foo = Foo {
    b: false,
    ..
};

// Override multiple field defaults
let foo = Foo {
    a: "Overriden",
    c: 1,
    ..
};

// Override no field defaults
let foo = Foo { .. };
```

# Detailed design
[design]: #detailed-design

## Grammar

In the definition of a `struct`, a default value expression can be optionally supplied for a field:

```
struct_field : vis? ident ':' type_path |
               vis? ident ':' type_path '=' expr
```

Initialisers can then opt-in to use field defaults for missing fields by adding `..` to the end of the initialiser:

```
struct_init_fields : struct_field_init ? [ ',' struct_field_init ] *

struct_init : '{' 
    struct_init_fields [ ".." | ".." expr ] ?
'}'
```

The syntax is modeled after constant expressions. Field defaults for tuple structs are not supported.

## Interpretation

The value of a field default must be a compile-time expression. So any expression that's valid as a `const` can be used as a field default. This ensures values that aren't specified by the caller are deterministic and cheap to produce. A type doesn't need to derive `Default` to be valid as a field default.

Valid:

```rust
struct Foo {
    a: &'static str,
    b: bool = true,
    c: i32,
}
```

Invalid:

```rust
struct Foo {
    a: &'static str,
    b: Vec<bool> = Vec::new(),
                   ^^^^^^^^^^
                   // error: calls in field defaults are limited to struct and enum constructors
    c: i32,
}
```

The above error is based on `E0015` for trying to initialise a constant with a non-constant expression. As the scope of constant expressions changes this message will change too.

Field defaults are like a shorthand for the 'real' initialiser, where values for missing fields are added with the supplied default expression:

```rust
let foo = Foo {
    a: "Hello",
    c: 42,
    ..
};
```

is equivalent to:

```rust
let foo = Foo {
    a: "Hello",
    b: true,
    c: 42,
};
```

The mechanism isn't exactly a shorthand because the `struct` can still be initialised using field defaults even if `b` is private. The caller still can't interact with private fields directly so privacy isn't violated.

When a caller doesn't supply a field value during initialisation and there is no default available then the `E0063` missing field error applies.

Field defaults are only considered for missing fields when the caller supplies a `..` at the end of the initialiser. Otherwise the standard `E0063` error applies with additional help when the field has a default value available:

```rust
let foo = Foo {
    a: "Hello",
    c: 42,
};

// error: missing field `b` in initializer of `Foo`.
// help: `b` has a default value. Try adding `..` so its default value will be used:
// `let foo = Foo { a: "Hello", c: 42, .. }`
```

## Order of precedence

Supplied field values take precedence over field defaults:

```rust
// `b` is `false`, even though the field default is `true`
let foo = Foo {
    a: "Hello",
    b: false,
    c: 42,
};
```

Supplied field values in functional updates take precedence over field defaults:

```rust
// `b` is `false`, even though the field default is `true`
let foo = Foo {
    a: "Hello",
    c: 0,
    ..Foo {
        a: "Hello",
        b: false,
        c: 0
    }
};
```

## Deriving `Default`

When deriving `Default`, supplied field defaults are used instead of the type default. This is a feature of `#[derive(Default)]`.

```rust
#[derive(Default)]
struct Foo {
    a: &'static str,
    b: bool = true,
    c: i32,
}

// `b` is `true`, even though `bool::default()` is `false`
let foo = Foo::default();
```

Field defaults allow `#[derive(Default)]` to be used more widely because the types of fields with default values don't need to implement `Default`.

## Enabling backwards compatibility

With no special syntax, additional fields can be added to a struct in a non-breaking fashion. Say we have the following API and consumer:

```rust
mod data {
    pub struct Foo {
        pub a: &'static str,
        pub c: i32,
        _marker: () = ()
    }
}

let foo = data::Foo {
    a: "Hello",
    c: 42,
    ..
}
```

Using a private marker field with a default value forces callers to opt-in to field defaults. We can now add a new field `b` to this `struct` with a default value, and the calling code doesn't change:

```rust
mod data {
    pub struct Foo {
        pub a: &'static str,
        pub b: bool = true,
        pub c: i32,
        _marker: () = ()
    }
}

let foo = data::Foo {
    a: "Hello",
    c: 42,
    ..
}
```

By using field defaults, callers can use `struct` literals without having to know about any private fields:

```rust
mod data {
    pub struct Foo {
        pub a: &'static str,
        pub c: i32,
        private_field: bool = true
    }
}

let foo = data::Foo {
    a: "Hello",
    c: 42,
    ..
}
```

## Field Privacy

Default values for fields are opted into by the `struct` definition, rather than the caller initialising the `struct`. Field privacy doesn't need to be violated to initialise a `struct`.

# How We Teach This
[how-we-teach-this]: #how-we-teach-this

Field defaults look similar to functional record updates, but solve different problems. New users could be confused by the similarity and be unsure when to use either feature. We can easily distinguish the two for new users:

- If you're in control of the `struct` definition, you can use field defaults.
- If you're only initialising `struct`s that don't have defaults or private fields, you can use functional record updates.

Field defaults are a tool for producers and functional record updates are a tool for consumers.

We should document field defaults in the Rust Reference sections 6.1.5 "Structs" and 7.2.4 "Struct expressions", with a one-sentence mention in 6.1.6 "Enumerations" that struct-like enum variants can have default fields just like structs.

# Drawbacks
[drawbacks]: #drawbacks

Field defaults are limited to constant expressions. This means there are values that can't be used as defaults, so any value that requires allocation, including common collections like `Vec::new()`. It's expected that users will use a constructor function or builder for initialisers that require allocations.

# Alternatives
[alternatives]: #alternatives

## Allow arbitrary expressions instead of just constant expressions

Allowing arbitrary expressions as field defaults would make this feature more powerful. However, limiting field defaults to constant expressions maintains the expectation that struct literals are cheap and deterministic. The same isn't true when arbitrary expressions that could reasonably panic or block on io are allowed.

For complex initialisation logic, builders are the preferred option because they don't carry this same expectation.

## Allow `Default::default()` instead of just constant expressions

An alternative to allowing any expression as a default value is allowing `Default::default()`, which is expected to be cheap and deterministic:

```rust
struct Foo {
    a: &'static str,
    b: Vec<i32> = Vec::default(),
    c: i32,
}

let foo = Foo {
    a: "Hello",
    c: 42,
}
```

It could be argued that supporting `default()` is an artificial constraint that doesn't prevent arbitrary expressions. The difference is that `Default` has an expectation of being cheap, so using it to inject logic into field initialisation is an obvious code smell.

Allowing functionality to be injected into data initialisation through `default()` means struct literals may have runtime costs that aren't ever surfaced to the caller. This goes against the expectation that literal expressions have small, predictable runtime cost and are totally deterministic (and trivially predictable).

## Type inference for fields with defaults

Reduce the amount of code needed to define a `struct` by allowing type inference for field defaults:

```rust
struct Foo {
    a: &'static str,
    b = Bar,
    c: i32,
}
```

This has the effect of simplifying the definition, but also requiring readers to manually work out what the type of the right-hand-side is. This could be less of an issue with IDE support.

## Implicit syntax for opting into field defaults

Invoke field defaults implicitely instead of requiring a `..` in the initialiser. This would be more in line with how other languages handle default values, but is less explicit. It would also be different from pattern matching for `struct`s, that require a `..` to ignore unnamed fields.

A future RFC could propose making the `..` optional for all places it's required when dealing with `struct`s.

# Unresolved questions
[unresolved]: #unresolved-questions
