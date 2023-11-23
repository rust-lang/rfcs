- Feature Name: `cfg_attribute_in_tuple_type`
- Start Date: 2023-11-23
- RFC PR: [rust-lang/rfcs#3532](https://github.com/rust-lang/rfcs/pull/3532)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Let's make it more elegant to conditionally compile tuple type declarations by allowing cfg-attributes directly on their element types.

# Motivation
[motivation]: #motivation

Currently, there is limited support for conditionally compiling tuple type declarations:

```rust
type ConditionalTuple = (u32, i32, #[cfg(feature = "foo")] u8);
```

```
error: expected type, found `#`
 --> <source>:1:36
  |
1 | type ConditionalTuple = (u32, i32, #[cfg(feature = "foo")] u8);
  |                                    ^ expected type
```

As with [RFC #3399](https://rust-lang.github.io/rfcs/3399-cfg-attribute-in-where.html), some workarounds exist, but they can result in combinatorial boilerplate:

```rust
// GOAL: 
// type ConditionalTuple = (
//     u32, 
//     i32, 
//     #[cfg(feature = "foo")] u8,
//     #[cfg(feature = "bar")] i8,
// );

// CURRENT:
#[cfg(all(feature = "foo", feature = "bar"))]
type ConditionalTuple = (u32, i32, u8, i8);
#[cfg(all(feature = "foo", not(feature = "bar")))]
type ConditionalTuple = (u32, i32, u8);
#[cfg(all(not(feature = "foo"), feature = "bar"))]
type ConditionalTuple = (u32, i32, i8);
#[cfg(all(not(feature = "foo"), not(feature = "bar")))]
type ConditionalTuple = (u32, i32);
```

One could also use a struct and attach cfg-attributes to its members, but this loses the compositional and syntactical advantages of using a tuple. For example, there are situations (e.g. with generics) where tuples are used for type communication rather than actual structure or storage. Structs can't easily serve this use case.

Importantly, Rust already supports per-element cfg-attributes in tuple *initialization*. The following is legal Rust code and functions as expected, even though the resulting type of `x` can't be expressed very easily:

```rust
pub fn main() {
    let x = (1u32, 4i32, #[cfg(all())] 23u8);
    println!("{}", x.2) // Output: 23
}
```

So it makes sense to support it in tuple type declaration as well.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Tuple type declarations can use cfg-attributes on individual elements, like so:

```rust
type MyTuple = (
    SomeTypeA,
    #[cfg(something_a)] SomeTypeB,
    #[cfg(something_b)] SomeTypeC,
)
```

and in other situations where tuple types are declared, such as in function arguments. These will conditionally include or exclude the type in that tuple (affecting the tuple's length) based on the compile-time evaluation result of each `#[cfg]` predicate.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This RFC proposes changing the syntax of the `TupleType` (see 10.1.5 in the Rust reference) to include `OuterAttribute*` before each occurrence of `Type`. These attributes can decorate each individual type (up to the comma or closing paren). In practice, at least within the scope of this RFC, only cfg-attributes need to be supported in this position.

# Drawbacks
[drawbacks]: #drawbacks

As with any feature, this adds complication to the language and grammar. Conditionally compiling tuple type elements can be a semver breaking change, but not any more than with the already existing workarounds.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

(See [RFC #3399](https://rust-lang.github.io/rfcs/3399-cfg-attribute-in-where.html) for a similar write-up.)

The need for conditionally compiling tuple types can arise in applications with different deployment targets or that want to release 
builds with different sets of functionality (e.g. client, server, editor, demo, etc.). It would be useful to support cfg-attributes 
directly here without requiring workarounds to achieve this functionality. Macros, proc macros, and so on are also ways to conditionally 
compile tuple types, but these also introduce at least one level of obfuscation from the core goal. Finally, tuples can be wholly 
duplicated under different cfg-attributes, but this scales poorly with both the size and intricacy of the tuple and the number of 
interacting attributes (which may grow combinatorically), and can introduce a maintenance burden from repeated code.

It also makes sense in this instance to support cfg-attributes here because they are already supported in this manner for tuple initialization.

# Prior art
[prior-art]: #prior-art

I'm not aware of any prior work in adding this to the language.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

I don't have any unresolved questions for this RFC.

# Future possibilities
[future-possibilities]: #future-possibilities

I believe this change is relatively self-contained, though I also think it's worth continuing to look for additional places where support for cfg-attributes makes sense to add. Conditional compilation is very important, especially in some domains, and requiring workarounds and additional boilerplate to support it is not ideal.
