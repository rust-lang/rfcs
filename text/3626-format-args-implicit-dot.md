- Feature Name: `format_args_implicit_dot`
- Start Date: 2023-10-01
- RFC PR: [rust-lang/rfcs#3626](https://github.com/rust-lang/rfcs/pull/3626)
- Rust Issue: [rust-lang/rust#00000](https://github.com/rust-lang/rust/issues/00000)

# Summary
[summary]: #summary

This RFC extends the "implicit named arguments" mechanism to allow accessing
field names with `var.field` syntax: `format!("{self.x} {var.another_field}")`.

# Motivation
[motivation]: #motivation

[RFC 2795](https://github.com/rust-lang/rfcs/pull/2795) added "implicit named
arguments" to `std::format_args!` (and other macros based on it such as
`format!` and `println!` and `panic!`), allowing the format string to reference
variables in scope using identifiers. For instance, `println!("Hello {name}")`
is now equivalent to `println!("Hello {name}", name=name)`.

The original implicit named arguments mechanism only permitted single
identifiers, to avoid the complexity of embedding arbitrary expressions into
format strings. The implicit named arguments mechanism is widely used, and one
of the most common requests and most common reasons people cannot use that
syntax is when they need to access a struct field. Adding struct field syntax
does not conflict with any other format syntax, and unlike allowing *arbitrary*
expressions, allowing struct field syntax does not substantially increase
complexity or decrease readability.

This proposal has the same advantages as the original implicit named arguments
proposal: making more formatting expressions easy to read from left-to-right
without having to jump back and forth between the format string and the
arguments.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

With this proposal accepted, the following (currently invalid) macro
invocation:

```rust
format_args!("hello {person.name}")
```

would become a valid macro invocation, and would be equivalent to a shorthand
for the already valid:

```rust
format_args!("hello {unique_ident}", unique_ident=person.name)
```

The identifier at the beginning of the chain (`person` in this case) must be an
identifier which existed in the scope in which the macro is invoked, and must
have a field of the appropriate name (`name` in this case).

This syntax works for fields within fields as well:

```rust
format_args!("{obj.field.nested_field.another_field}")
```

As a result of this change, downstream macros based on `format_args!` would
also be able to accept implicit named arguments in the same way. This would
provide ergonomic benefit to many macros across the ecosystem, including:

 - `format!`
 - `print!` and `println!`
 - `eprint!` and `eprintln!`
 - `write!` and `writeln!`
 - `panic!`, `unreachable!`, `unimplemented!`, and `todo!`
 - `assert!`, `assert_eq!`, and similar
 - macros in the `log` and `tracing` crates

(This is not an exhaustive list of the many macros this would affect.)

## Additional formatting parameters

As a result of this RFC, formatting parameters can also use implicit named
argument capture:

```rust
println!("{self.value:self.width$.self.precision$}");
```

This is slightly complex to read, but unambiguous thanks to the `$`s.

## `await`

Formatting can use `.await`, as well:

```rust
println!("{future1.await} {future2.await}");
```

## Compatibility

This syntax is not currently accepted, and results in a compiler error. Thus,
adding this syntax should not cause any breaking changes in any existing Rust
code.

## No field access from named arguments

This syntax only permits referencing fields from identifiers in scope. It does
not permit referencing fields from named arguments passed into the macro. For
instance, the following syntax is not valid, and results in an error:

```rust
println!("{x.field}", x=expr()); // Error
```

If there is an ambiguity between an identifier in scope and an identifier used
for a named argument, the compiler emits an error.

```rust
let x = SomeStruct::new();
println!("{x.field}", x=expr()); // Error
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The implementation captures the first identifier in the chain using the same
mechanism as implicit format arguments, and then uses normal field accesses to
obtain the value, just as if the field were accessed within a named argument.
Thus, the following two expressions are semantically equivalent:

```rust
format_args!("{name.field1.field2}")

format_args!("{unique_identifier}", unique_identifier=name.field1.field2)
```

Any `Deref` operations or `.await` operations associated with the `.` in each
format argument are evaluated exactly once, from left-to-right as they appear
in the format string, at the point where the format string argument is
evaluated, before the positional or named arguments are evaluated. No
deduplication occurs: if `name.field` or `name.await` is mentioned multiple
times, it will be evaluated multiple times.

If the identifier at the start of the chain does not exist in the scope, the
usual error E0425 would be emitted by the compiler, with the span of that
identifier:

```
error[E0425]: cannot find value `person` in this scope
 --> src/main.rs:X:Y
  |
X |     format_args!("hello {person.name}");
  |                          ^^^^^^ not found in this scope
```

If one of the field references refers to a field not contained in the
structure, the usual error E0609 would be emitted by the compiler, with the
span of the field identifier:

```
error[E0609]: no field `name` on type `person`
 --> src/main.rs:X:Y
  |
5 |     format_args!("hello {person.name}");
  |                                 ^^^^ unknown field
```

# Drawbacks
[drawbacks]: #drawbacks

This adds incremental additional complexity to format strings.

Having `x.y` available may make people assume other types of expressions work
as well.

This introduces an additional mechanism to allow side-effects while evaluating
a format string. However, format strings could already cause side effects while
evaluating, if a `Display` or `Debug` implementation has side effects.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The null alternative is to avoid adding this syntax, and let users continue to
pass named arguments or bind local temporary names rather than performing
inline field accesses within format strings. This would continue to be
inconvenient but functional.

This functionality could theoretically be implemented in a third-party crate,
but would then not be automatically and consistently available within all of
Rust's formatting macros, including those in the standard library and those
throughout the ecosystem.

We could omit support for other formatting parameters (width, precision).
However, this would introduce an inconsistency that people have to remember;
people would *expect* this to work.

We could omit support for `.await`. However, to users this may seem like an
arbitrary restriction. The rationale for this RFC is purely *syntactic*, on the
basis that we can allow expressions using `.` without requiring delimiters.

# Prior art
[prior-art]: #prior-art

Rust's existing implicit format arguments serve as prior art, and discussion
around that proposal considered the possibility of future (cautious) extension
to additional types of expressions.

The equivalent mechanisms in some other programming languages (e.g. Python
f-strings, Javascript backticks, C#, and various other languages) allow
arbitrary expressions. This RFC does *not* propose adding arbitrary
expressions, nor should this RFC serve as precedent for arbitrary expressions,
but nonetheless these other languages provide precedent for permitting more
than just single identifiers.
