- Feature Name: formal_function_param_attrs
- Start Date: 2018-10-14
- RFC PR: 
- Rust Issue: 

# Summary
[summary]: #summary

This RFC proposes to allow attributes in formal function parameter position.

# Motivation
[motivation]: #motivation

Having attributes on formal function parameters allows for certain different use cases.

## Example: Handling of unused parameter

In today's Rust it is possible to prefix the name of an identifier to silence the compiler about it being unused.
With attributes in formal function parameter position we could have an attribute like `#[unused]` that explicitely states this for a given parameter.

```rust
fn foo(#[unused] bar: u32) -> bool;
```

Instead of

```rust
fn foo(_bar: u32) -> bool
```

This would better reflect the explicit nature of Rust compared to the underscore prefix as of today.

## Example: Low-level code

For raw pointers that are oftentimes used when operating with C code one could provide the compiler with additional information about the set of parameters.
You could for example mirror C's restrict keyword or even be more explicit by stating what pointer argument might overlap.

```rust
fn foo(
 #[overlaps_with(in_b)] in_a: *const u8,
 #[overlaps_with(in_a)] in_b: *const u8,
 #[restrict] out: *mut u8
);
```

Which might state that the pointers `in_a` and `in_b` might overlap but `out` is non overlapping.
Please note that I am *not* proposing to actually add this to the language!

## Example: Procedural Macros

Also procedural macros could greatly benefit from having their own defined custom attributes on formal parameters.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Formal parameters of functions, methods, closures and functions in trait definitions may have attributes attached to them.
This allows to provide additional information to a given formal parameter.

For the next examples the hypothetical `#[unused]` attribute means that the attributed parameter is unused in the associated implementation.

## Examples

The syntax for this is demonstrated by the code below:

```rust
// Function
fn foo(#[unused] bar: u32) { .. }

// Methods & trait & definitions:
// - `self` can also be attributed
fn foo(#[unused] self, ..) { .. }
fn foo(#[unused] &self, ..) { .. }
fn foo(#[unused] &mut self, ..) { .. }

// Closures & Lamdas
|#[unused] x| { .. }
```

### Trait declarations

```rust
fn foo(#[unused] self);
```

Note that while the `#[unused]` attribute is syntactically
possible to put here it doesn't actually make sense semantically
since method declarations have no implementation.
Other attributes might be very useful as for formal parameters in a method declaration.

## Errors & Warnings

### Warning: Unused attribute

When using an non-defined attribute that is not used by either the language or a custom defined procedural macro.

```
warning: unused attribute
 --> src/main.rs:2
  |
2 | #[foo] bar: u32
  | ^^^^^^^^^
  |
  = note: #[warn(unused_attributes)] on by default
```

### Error: Malformed attribute

When using a known attribute that is not defined for formal parameters such as when attributing `fn main` with `#[allow]`.

Example shows the usage of the known attribute `#[inline]` used on a formal parameter without being defined for it.

```
error[E0452]: malformed lint attribute
 --> src/main.rs:2
  |
2 | #[inline] bar: u32
  | ^^^^^^^^
```

The same applies for attributes with an incorrect format such as `#[inline(key = value)]` that is handled as its done in other contexts.


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Description

In accordance to the RFC for [attributes for generic params](https://github.com/frol/rust-rfcs/blob/master/text/1327-dropck-param-eyepatch.md)
this feature is guarded by the `formal_function_param_attrs` feature guard.

The grammar of the following language items has to be adjusted to allow that
constructions like the following will become legal.

- Function definitions
- Method definitions
- Trait function declarations and defnitions (with or without default impl)
- Lambda & closure definitions

### Example: A single attributed parameter for function decl or definition

```rust
fn foo(#[bar] baz: bool);
fn bar(#[bar] qux: bool) { println!("hi"); }
```

### Example: For methods or trait function definitions

```rust
fn into_foo(#[bar] self);
fn foo(#[bar] &self);
fn foo_mut(#[bar] &mut self);
```

### Example: Multiple attributed parameters

```rust
// Twice the same attribute
fn fst_foo(#[bar] baz: bool, #[bar] qiz: u32);

// Different attributes
fn snd_foo(#[bar] baz: bool, #[qux] qiz: u32);
```

### Example: Any structured attribute

```rust
fn foo(#[bar(Hello)] baz: bool);
fn bar(#[qux(qiz = World)] baz: bool);
```

### Example: Lambdas & closures

```rust
let mut v = [5, 4, 1, 3, 2];
v.sort_by(|#[bar] a, #[baz] b| a.cmp(b));
```

## Errors

Users can encounter two different errorneous situations.

### Unknown attribute used

When a user is using an attribute that is not known at the point of its invokation
a warning is generated similar to other usages of unknown attributes in the language.

This may for example happen in the context of a procedural macros.

### Malformed attribute

When a user is using a known or language defined attribute at a non supported location
an error is generated like in other usages of malformed attributes in the language.
An example can be seen in the previous section.


# Drawbacks
[drawbacks]: #drawbacks

All drawbacks for attributes in any location also count for this proposal.

Having attributes in many different places of the language complicates its grammar.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Why is this proposal considered the best in the space of available ideas?

This proposal clearly goes the path of having attributes in more places of the language.
It nicely plays together with the advance of procedural macros and macros 2.0 where users
can define their own attributes for their special purposes.

## Alternatives

An alternative to having attributes for formal parameters might be to just use the current
set of available attributable items to store meta information about formal parameters like
in the following example:

```rust
#[ignore(param = bar)]
fn foo(bar: bool);
```

Note that this does not work in all situations (for example closures) and might involve even
more complexity in user's code than simply allowing formal function parameter attributes.

## Impact

The impact will most certainly be that users might create custom attributes when
designing procedural macros involving formal function parameters.

There should be no breakage of existing code.

# Prior art
[prior-art]: #prior-art

Some example languages that allows for attributes in formal function parameter positions are C# and C++.

Also note that attributes in other parts of the Rust language could be considered prior art to this proposal.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

We might want to introduce new attributes for the language like the mentioned `#[unused]` attribute.
However, this RFC proposes to decide upon this in another RFC.
