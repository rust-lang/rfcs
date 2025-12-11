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
It was decided that all public API and top level items must be "obviously semver stable" rather than "quick to type".


In simple cases, explicitly writing out
the type of the const seems trivial. However, this isn't always the case:

- Sometimes the constant's value is complex, making the explicit type overly verbose.
- In some cases, the type may be unnamable.
- When creating macros, the precise type might not be known to the macro author.
- Code generators may not have enough information to easily determine the type.

This change aims to make Rust more expressive, concise and maintainable, especially in scenarios where the types of
const items are complicated or not easily expressible.

Inferring constant types also improves the ergonomics and consistency of the language, particularly for new users. Types are already being inferred for `let` bindings, and not allowing inference for obvious `const` or `static` items creates a mismatch of expectations, especially when `const`/`static` items may be defined inside a function, directly next to a `let` binding.


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

You may declare constants and static variables without specifying their types when the type can be inferred
from the initial value, subjecting to the following constraints:
- The types of all literals must be fully constrained, which generally means numeric literals must either
  have a type suffix, or the type must specify their type
- When declaring a top-level item, the typing may not be entirely omitted. At the very least, a `_` placeholder must be used, but the `_` placeholder
  may also appear anywhere in a nested type.

For example:

```rs
const NO = false; //  missing type for `const` item; hint: provide a type or add `_` placeholder
const PI: _ = 3.1415; // Ambiguous numeric type
const PI: _ = 3.1415_f32; // Ok
const WRAPPED_PI: MyStruct<_> = MyStruct(3.1415_f32); // Ok


static MESSAGE: _ = "Hello, World!"; // inferred as &'static str
static ARR: [u8; _] = [12, 23, 34, 45]; // inferred as [u8; 4]
const FN: _ = std::string::String::default; // inferred as the unnamable type of ZST closure associated  with this item. Its type is reported by `type_name_of_val` as ::std::string::String::default
```

In summary, globals should have sandboxed inference context, where their type would be fully known after all constraints in const expr block has been applied; i.e. no default types for literals, nor implicit casts should be allowed.

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
  especially to newcomers or when explicit types are needed to understanding the purpose of the item.
  It is my belief that this is a choice better left for the developers as in the case of `let` bindings.
- Semver compatibilty: The API surface of the type is implicit, changing the right-hand side in subtle ways can change the type in a way that can be hard to notice, for example between different integer types. This goes against the rule that "all top-level items must be fully type-annotated".

However, this philosophy needs to be carefully weighted against
language expressiveness and usability.

Not all `const` or `static` items are public, and in many cases the type is obvious enough that semver isn't a concern. Requiring explicit typing for this reason seems a bit heavy handed.

It is for this reason that we require "opt-in" where type inference is desired
by requiring at least a "_" placeholder for top level items. A clippy lint will also be added
when such top level item may in fact be named.


# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Impact of Not Doing This:

Rust code remains more verbose than necessary, especially in complex scenarios, and macro authors face challenges with type specifications.

## Alternatives

Allowing the naming of function types as in [#3476](https://github.com/rust-lang/rfcs/pull/3476) may help resolve some of the cases where type inference is needed.

`type_alias_impl_trait` may also partially address the problem. In particular, it helps with unnamable types
and macro / code generator output, without the drawbacks of loss of clarity and semvar trouble.
However, it cannot fully replace inference because
- There are cases where we do not want the type to be hidden behind an `impl Trait`.
- Defining a trait for the const item might be difficult - for example, when the
  const item is a function pointer with a variable number of arguments.
- The const item might be generated from a macro, and the macro might not
  want to require a separate trait to be defined.
- This also won't help with array lengths or types that do not implement a particular trait.

# Prior art
[prior-art]: #prior-art

In [RFC#1623](https://github.com/rust-lang/rfcs/pull/1623) we added `'static` lifetimes to every reference or generics lifetime value in `static` or `const` declarations.

In [RFC#2010](https://github.com/rust-lang/rfcs/pull/2010) const/static type inference
was proposed, but the RFC was **postponed**. The [reason](https://github.com/rust-lang/rfcs/pull/2010#issuecomment-325827854) can be summarized as follows:

- Things we can do with const/static was quite limited at the time.
Const/static type inference fails to provide value for simple cases such as `const SOMETHING = 32;`
- The team wanted to move forward in other areas (e.g. impl Trait) before moving on to solve this problem.

However, after 7 years it is now time to revisit this topic:

- The things that can be done in a const expression has been greatly expanded since
  2017, which means that it is more likely to use a complicated type or an unnamable type on const/static items.
- It is now possible to use impl types in function returns (although having impl type aliases could provide a similar solution for const and statics)
- Const and static aren't necessarily at the top level. It feels weird that the type can be elided on a let statement inside a function, but not on a const or static inside a function
- The original RFC resolution of **postpone** was made at least partially based on
  statistics done by @schuster. This RFC was proposed with the motivation of enabling the use
  of unnamable types in const/static items. Because this RFC enables new behaviors,
  data on current usage isn't very useful for determining how much it might improve language
  expressiveness.



# Unresolved questions
[unresolved-questions]: #unresolved-questions

None.
