- Feature Name: `impl-trait-for-fn`
- Start Date: 2023-08-20
- RFC PR: [rust-lang/rfcs#3476](https://github.com/rust-lang/rfcs/pull/3476)
- Rust Issue: N/A

# Summary
[summary]: #summary

Support for implementing traits on functions

# Motivation
[motivation]: #motivation

I was trying to make something similar to bevy's system functions. And for safety reasons, they check for conflicts between SystemParams, so that a function requiring `Res<A>` and `ResMut<A>` [panic](https://github.com/bevyengine/bevy/blob/main/crates/bevy_ecs/src/system/system_param.rs#L421).

Then after I heard about axum's [`#[debug_handler]`](https://docs.rs/axum/latest/axum/attr.debug_handler.html) I wanted to do something similar to my copy of bevy systems, so that I get compile time errors when there is a conflict. I wanted even more, I wanted to force the user to mark the function with a specific proc attribute macro in order to make it possible to pass it into my code and call itself a system.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

As we all know, you can implement a trait for a struct with the following syntax
```rust
struct Timmy;
impl Person for Timmy {
    fn greet() {
        println!("Hey it's me, Timmy!");
    }
}
```
And we can also implement a trait for all functions with a specific signature like this
```rust
impl<F> ValidSignature for F
    where F: Fn(i32) -> bool
{
    /* ... */
}
```
Now we can also implement traits for specific functions only
```rust
fn valid() {}
impl ValidFunction for fn valid {
    /* ... */
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

When the function has parameters, modifiers or a return type, it should not be included in the impl block, because the path is already unique
```rust
async fn request_name(id: PersonID) -> String { .. }

impl Requestable for fn request_name {
    /* ... */
}
```
It gives the possibility to also implement a trait directly scoped to a function instead of generic implementation of multiple. Other than that, it basically behaves the same. It should also be possible to implement them via proc attribute macros:
```rust
#[impl_debug_name = "Greeting"]
fn greet() {
    /* ... */
}
// should expand to something like
fn greet() {
    /* ... */
}
impl FnDebugName for fn greet() {
    fn debug_name() -> &'static str {
        "Greeting"
    }
}
```

# Drawbacks
[drawbacks]: #drawbacks

i dont know any

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

I think it's a easy task because we can already implement traits for a group of functions with the same signature, so why shouldn't we also implement single scoped impls?

# Prior art
[prior-art]: #prior-art

i dont know any

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Is the syntax good? I feel like we could drop the `fn` to from `impl Trait for fn function` to `impl Trait for function`.
- What about closures? They don't even have names so targetting them would be quite difficult. I wouldn't want to use the compiler generated mess of a name like `[closure@src/main.rs:13:18: 13:20]`. It would also contain line numbers which would be changing quite often so thats not ideal.

# Future possibilities
[future-possibilities]: #future-possibilities

- also make it possible to implement traits for closures directly.