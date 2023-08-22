- Feature Name: `expose-fn-type`
- Start Date: 2023-08-20
- RFC PR: [rust-lang/rfcs#3476](https://github.com/rust-lang/rfcs/pull/3476)
- Rust Issue: N/A

# Summary
[summary]: #summary

This exposes the ghost-/inner-/localtype of a function to the user.

# Motivation
[motivation]: #motivation

I was trying to make something similar to bevy's system functions. And for safety reasons, they check for conflicts between SystemParams, so that a function requiring `Res<A>` and `ResMut<A>` [panic](https://github.com/bevyengine/bevy/blob/main/crates/bevy_ecs/src/system/system_param.rs#L421).

Then after I heard about axum's [`#[debug_handler]`](https://docs.rs/axum/latest/axum/attr.debug_handler.html) I wanted to do something similar to my copy of bevy systems, so that I get compile time errors when there is a conflict. I wanted even more, I wanted to force the user to mark the function with a specific proc attribute macro in order to make it possible to pass it into my code and call itself a system.

For that, I would need to mark the type behind the function, for example, with a trait.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

As we all know, you can refer to a struct by its name and for example implement a trait
```rust
struct Timmy;
impl Person for Timmy {
    fn greet() {
        println!("Hey it's me, Timmy!");
    }
}
```
When we want to target a specific function for a trait implementation, we somehow need to get to the type behind it. That is being done with the `fn` keyword as follows
```rust
fn my_function() {}
impl MyTrait for fn my_function {
    /* ... */
}
```
---
For a better understanding, imagine you have a struct like this:
```rust
struct FnContainer<F: Fn()> {
    inner: F,
}
fn goods() { }

let contained_goods = FnContainer {
    inner: goods
};
```
Here, we make a `FnContainer` which can hold every function with the signature `() -> ()` via generics.
But what about explicitly designing the `FnContainer` for a specific function, just like the compiler does when resolving the generics. This will work the same as with the trait impl from above:
```rust
struct GoodsContainer {
    inner: fn goods,
}
fn goods() {}

let contained_goods = GoodsContainer {
    inner: goods,
}
```
---
A function with a more complex signature, like with parameters, modifiers or a return type, is still just referenced by its name, because it's already unique
```rust
async fn request_name(id: PersonID) -> String { .. }

impl Requestable for fn request_name {
    /* ... */
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

As described in the **Guide-level explanation**, with the syntax `fn <fn_path>`, we can reference the type behind the named function.

When the function is for example in a different mod, it should be referenced by its path
```rust
mod sub {
    fn sub_mod_fn() { .. }
}
impl fn sub::sub_mod_fn {
    /* ... */
}
```

It should be also possible to get the type of functions inside impl blocks:

```rust
struct MyStruct;
impl MyStruct {
    fn new() -> Self { Self }
}
impl fn MyStruct::new {
    /* ... */
}
```

When a function has generics, they will be handled as follows, just like we know it from normal types
```rust
fn send<T: Send>(val: T) {}
impl<T: Send> ParcelStation for fn send<T> {
    /* ... */
}
```

When we have an implicit generic, they will be appended in order to the generic list:
```rust
fn implicit_generic(val: impl Clone) -> impl ToString {}
impl<T: Send, U: ToString> for fn implicit_generic<T, U> {
    /* ... */
}
```

Just as structs and enums have the possibility to derive traits to automatically generate code, function type do too

```rust
#[derive(DbgSignature)]
fn signature_test(val: i32) -> bool {
    /* ... */
}

// Expands to

fn signature_test(val: i32) -> bool {
    /* ... */
}
impl DbgSignature for fn signature_test {
    fn dbg_signature() -> &'static str {
        "fn signature_test(val: i32) -> bool"
    }
}
```

Other than that, it should behave like every other type does.

# Drawbacks
[drawbacks]: #drawbacks

- When introducing the derive feature, it could lead to parsing problems with proc macros having an older `syn` crate version.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The type behind functions already exists, we just need to expose it to the user.
The hard part would be allowing derives, because that may break some things.

# Prior art
[prior-art]: #prior-art

i dont know any

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Is the syntax good? It could create confusion between a function pointer.
- What about closures? They don't even have names so targetting them would be quite difficult. I wouldn't want to use the compiler generated mess of a name like `[closure@src/main.rs:13:18: 13:20]`. It would also contain line numbers which would be changing quite often so thats not ideal.
- I provided a possible solution for a `fn implicit_generic(val: impl Clone) -> impl ToString` function, but because we currently don't have a defined syntax for those generics in types, thus we can't use `impl Trait` as types for fields in structs, we should think about this more, maybe don't implement exposed types of function for such `fn`s and wait for another RFC?

# Future possibilities
[future-possibilities]: #future-possibilities

- Also expose the type of closures
