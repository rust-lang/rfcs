- Feature Name: `return_type_attrs`
- Start Date: 2021-11-25
- RFC PR: [rust-lang/rfcs#3201](https://github.com/rust-lang/rfcs/pull/3201)

# Summary
[summary]: #summary

Allow attributes in function return type position. For example:

```rust
fn sum(x: u8, y: u8) -> #[some_attr] u8 {
    x + y
}
```

# Motivation
[motivation]: #motivation

Currently the whole function can be annotated with a macro attribute:

```rust
#[some_attr]
fn example() { .. }
```

As well as individual function parameters (introduced in [RFC2565]):

[RFC2565]: 2565-formal-function-parameter-attributes.md

```rust
fn example(#[attr] input: String, #[attr] x: u8) { .. }
```

However function return types currently cannot be annotated, which forces
domain-specific languages (DSLs) to resort to function attributes, for example:

```rust
#[wasm_bindgen]
impl RustLayoutEngine {
    #[return_type = "MapNode[]"]
    pub fn layout(
        &self,
        #[type = "MapNode[]"] nodes: Vec<JsValue>,
        #[type = "MapEdge[]"] edges: Vec<JsValue>
    ) -> Vec<JsValue> {
        ..
    }
}
```

Return type attributes would allow the above example to be changed to:

```rs
#[wasm_bindgen]
impl RustLayoutEngine {
    pub fn layout(
        &self,
        #[type = "MapNode[]"] nodes: Vec<JsValue>,
        #[type = "MapEdge[]"] edges: Vec<JsValue>
    ) -> #[type = "MapNode[]"] Vec<JsValue> {
        ..
    }
}
```

The resulting DSL is clearer to read ... in particular in functions with many
parameters where the function attribute would be several lines apart from the
return type (because `rustfmt` by default puts each parameter on its own line
when the function signature gets too long).

Since function parameters already can be annotated this can be regarded as the
next logical step towards more expressive and intuitive DSLs.  The motivation
for the introduction of parameter attributes outlined in [RFC2565] largely
applies to return type attributes as well, since they would also be useful for
property based testing, interoperability with other languages and optimization
annotations.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Return types of `fn` definitions as well as closures may have attributes
attached to them. Thereby, additional information may be provided.

For the purposes of illustration, let's assume we have the attribute
`#[apple]` available to us.

## Basic examples

The syntax for attaching attributes to return types is shown in the snippet below:

```rust
// Free functions:
fn foo() -> #[apple] u32 { .. }

impl Alpha { // In inherent implementations.
    fn bar() -> #[apple] u8 { .. }

    ..
}

impl Beta for Alpha { // Also works in trait implementations.
    fn bar() -> #[apple] u8 { .. }

    ..
}

fn foo() {
    // Closures:
    let bar = |x| -> #[apple] u8 { .. };
}
```

## Trait definitions

An `fn` definition doesn't need to have a body to permit return type attributes.
Thus, in `trait` definitions, we may write:

```rust
trait Beta {
    fn bar(&self) -> #[apple] u8;
}
```

## `fn` types

You can also use attributes in function pointer types.
For example, you may write:

```rust
type Foo = fn() -> #[apple] u8;
```

## Unit return type

When annotating the unit return type `()` must be specified explicitly. For
example:

```rust
fn foo() -> #[apple] () {
    ..
}
```

Attempting the following:

```rust
fn foo() -> #[apple] {
    ..
}
```

will result in a compile error:

```
error: return type attributes require an explicit return type
fn foo() -> #[apple] {
                    ^ expected ()
```

## Built-in attributes

Attributes attached to return types do not have an inherent meaning in
the type system or in the language. Instead, the meaning is what your
procedural macros, the tools you use, or what the compiler interprets certain
specific attributes as.

As for the built-in attributes and their semantics, we will, for the time being,
only permit the following attributes on return types:

- Lint check attributes, that is:
  `#[allow(C)]`, `#[warn(C)]`, `#[deny(C)]`, `#[forbid(C)]`,
  and tool lint attributes such as `#[allow(clippy::foobar)]`.

All other built-in attributes will be rejected with a semantic check.
For example, you may not write:

```rust
fn foo() -> #[inline] u32 { .. }
```


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

TODO

<!--
This is the technical portion of the RFC. Explain the design in sufficient detail that:

- Its interaction with other features is clear.
- It is reasonably clear how the feature would be implemented.
- Corner cases are dissected by example.

The section should return to the examples given in the previous section, and
explain more fully how the detailed proposal makes those examples work.
-->

# Drawbacks
[drawbacks]: #drawbacks

All drawbacks for attributes in any location also count for this proposal.

Having attributes in many different places of the language complicates its
grammar.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

> Why is this design the best in the space of possible designs?

If function parameters can be annotated it is only natural that function return
types can be annotated as well.

> What other designs have been considered and what is the rationale for not choosing them?

Using a function attribute to annotate the return type can result in the
annotation being far apart from the type that it's annotating, as showcased in
the [Motivation](#motivation) section.

[RFC2602](https://github.com/rust-lang/rfcs/pull/2602) suggests permitting
attributes to be attached to any types, so implementing that RFC would also
permit return types to be annotated with attributes. The concern that has
however been raised with that approach is that permitting attributes nearly
everywhere would undesirably increase the cognitive load needed to read Rust
code and thus harm the readability of Rust.  Currently attributes are
restricted to specific positions. Allowing them anywhere (even in nested types)
would pose a more radical change, whereas this RFC is more of a continuation of
the status quo by just permitting attributes in one more specific position.

> What is the impact of not doing this?

DSLs cannot take advantage of the additional expressiveness.

# Prior art
[prior-art]: #prior-art

Parameter attributes were introduced to Rust with [RFC2565].

# Unresolved questions
[unresolved-questions]: #unresolved-questions

Are there more built-in attributes that should be permitted for return types?

Is there precedent of other programming languages permitting return type
annotations to be placed directly in front of the return type?

# Future possibilities
[future-possibilities]: #future-possibilities

If `rustdoc` one day supports documentation comments on parameters, it could
also support documentation comments on return types.
