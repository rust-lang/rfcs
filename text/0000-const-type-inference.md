- Feature Name: const_type_inference
- Start Date: 2023-12-21
- RFC PR: [rust-lang/rfcs#3546](https://github.com/rust-lang/rfcs/pull/3546)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Allow type inference for `const` or `static` when the type of the initial value is known.

# Motivation
[motivation]: #motivation

Rust currently requires explicit type annotations for `const` and `static` items.


In simple cases, explicitly writing out
the type of the const seems trivial. However, this isn't always the case:

- Sometimes the constant's value is complex, making the explicit type overly verbose.
- In some cases, the type may be unnameable.
- When creating macros, the precise type might not be known to the macro author.
- Code generators may not have enough information to easily determine the type.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

You may declare constants and static variables without specifying their types when the type can be inferred
from the initial value. For example:

```rs
const PI = 3.1415; // inferred as f64
static MESSAGE = "Hello, World!"; // inferred as &'static str
const FN_PTR = std::string::String::default; // inferred as fn() -> String
```

This change aims to make Rust code more concise and maintainable, especially in scenarios where the types of
const items are complicated or not easily expressible.


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation


The type inference for `const` and `static` will leverage Rust's existing type inference mechanisms. The compiler will infer the type exclusively based on the RHS. If the type cannot be determined or if it leads to ambiguities, the compiler will emit an error, prompting the programmer to specify the type explicitly.


Today, the compiler already gives hint for most cases where the const or static item is missing a type:

```
802 | const A = 0;
    |        ^ help: provide a type for the constant: `: i32`
```


```
error: missing type for `const` item                                                     
  --> file.rs:27:26
   |
27 |     pub const update_blas = SystemStage { system: test_system, stage: vk::Pipeli... 
   |                          ^ help: provide a type for the constant: `: render_pass::SystemStage<for<'a> fn(ResMut<'a, AsyncQueues>)>`
```

The implementation should only need to carry over this information and set the type correspondingly
instead of emitting an error.


# Drawbacks
[drawbacks]: #drawbacks

- Potential Loss of Clarity: In some cases, omitting the type might make the code less clear,
  especially to newcomers or in codebases where explicit types are part of the documentation style.
  It is my belief that this is a choice better left for the developers as in the case of `let` bindings.
- Anything else?

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- Impact of Not Doing This: Rust code remains more verbose than necessary, especially in complex scenarios, and macro authors face challenges with type specifications.


# Prior art
[prior-art]: #prior-art

In [RFC#1623](https://github.com/rust-lang/rfcs/pull/1623) we added `'static` lifetimes to every reference or generics lifetime value in `static` or `const` declarations.


# Unresolved questions
[unresolved-questions]: #unresolved-questions

Should we allow assignment of unnameable types? For example,
```rs
let A = |a: u32| {
    123_i32
};

```

```
error: missing type for `const` item
   |
28 | const A = |a: u32| {
   |        ^
   |
note: however, the inferred type `[closure@render_pass.rs:28:11]` cannot be named        
   |
28 |   const A = |a: u32| {
   |  ___________^
29 | |     1_i32
30 | | };
   | |_^
```

If this significantly complicates the implementation, we can leave it outside the scope of thie RFC.
