- Feature Name: `expose-fn-type`
- Start Date: 2023-08-20
- RFC PR: [rust-lang/rfcs#3476](https://github.com/rust-lang/rfcs/pull/3476)
- Rust Issue: N/A

# Summary
[summary]: #summary

This exposes the function type of a function item to the user.

# Motivation
[motivation]: #motivation

### DasLixou

I was trying to make something similar to bevy's system functions. And for safety reasons, they check for conflicts between SystemParams, so that a function requiring `Res<A>` and `ResMut<A>` [panic](https://github.com/bevyengine/bevy/blob/main/crates/bevy_ecs/src/system/system_param.rs#L421).

Then after I heard about axum's [`#[debug_handler]`](https://docs.rs/axum/latest/axum/attr.debug_handler.html) I wanted to do something similar to my copy of bevy systems, so that I get compile time errors when there is a conflict. I wanted even more, I wanted to force the user to mark the function with a specific proc attribute macro in order to make it possible to pass it into my code and call itself a system.

For that, I would need to mark the type behind the function item, for example, with a trait.

### madsmtm

In Swift, some functions have an associated selector that you can access with [`#selector`](https://developer.apple.com/documentation/swift/using-objective-c-runtime-features-in-swift).

In my crate `objc2`, it would be immensely beautiful (and useful) to be able to do something similar, e.g. access a function's selector using something like `MyClass::my_function::Selector` or `selector(MyClass::my_function)`, instead of having to know the selector name (which might be something completely different than the function name).

# Terminology

I'll may shorten `function` to `fn` sometimes.

- **function pointer**: pointer type with the type syntax `fn(?) -> ?` directly pointing at a function, not the type implementing the `Fn[Once/Mut](?) -> ?` traits.
- **function item** (or just function): a declared function in code. free-standing or associated to a type.
- **function group**: many non-specific functions with the same signature (params, return type, etc.)
- **function trait(s)**: the `Fn[Once/Mut](?) -> ?` traits
- **function type**: the type behind a function, which also implements the function traits.
- **fixed type**: directly named type, no generic / `impl Trait`.
- **describe the function type**: write `fn(..) -> ? name` instead of just `fn name`.

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
When we want to target a specific function for a trait implementation, we somehow need to get to the type behind it. 
Refering to the hidden type is achieved via the following syntax
```rust
fn is_positive(a: i32) -> bool { /* ... */ }
impl MyTrait for fn(i32) -> bool is_positive {
    /* ... */
}
```
For function signatures, where every parameter/return-type is a fixed type and can be known just by refering to the function (so no generics or `impl Trait` parameters/return type), we can drop the redundant information:
```rust
fn is_positive(a: i32) -> bool { /* ... */ }
impl MyTrait for fn is_positive {
    /* ... */
}
```

> 💡 NOTE: Even when we need to describe the function type but the return type is `()`, we can (just as for function pointers and function traits) drop the `-> ()` from the type. (This should also be added as a lint).

---
A function with a more complex signature, like a function that specifies `const`, `unsafe` or `extern "ABI"`, we just ignore that when naming the type:
```rust
const fn my_fn(a: i32) -> (i16, i16) { .. }
impl MyTrait for fn my_fn {}
// or with explicit declaration
impl MyTrait for fn(i32) -> (i16, i16) my_fn { .. }
```

When having an async function, we in theory have a `impl Future<Output = ..>` as a return type, which should force us to explicitly declare the function type like so
```rust
async fn request_name(id: PersonID) -> String { .. }

impl<F: Future<Output = String>> Requestable for fn(PersonID) -> F request_name {
    /* ... */
}
```
We can take a shortcut and use the `async` keyword, as long as the `Output` assoc type in the Future is still fixed
```rust
async fn request_name(id: PersonID) -> String { .. }

impl Requestable for async fn request_name {
    /* ... */
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

As described in the **Guide-level explanation**, with the syntax `[async] fn[(..) -> ?] <fn_path>`, we can reference the type behind the named function.

When the function is for example in a different mod, it should be referenced by its path
```rust
mod sub {
    fn sub_mod_fn() { .. }
}
trait MyTrait {}
impl MyTrait for fn sub::sub_mod_fn {
    /* ... */
}
```

> ⚠️ NOTE: The same rules apply here as for normal types. Either the function item or the trait to implement mustn't be foreign for the impl. [Same as E0210](https://github.com/rust-lang/rust/blob/master/compiler/rustc_error_codes/src/error_codes/E0210.md)

---

It should be also possible to get the type of associated functions:

```rust
struct MyStruct;
impl MyStruct {
    fn new() -> Self { Self }
}
impl fn MyStruct::new {
    /* ... */
}
```

When the associated function comes from a trait, the same rules as for associated types apply here ([Ambiguous Associated Type, E0223](https://github.com/rust-lang/rust/blob/master/compiler/rustc_error_codes/src/error_codes/E0223.md)):

```rust
struct MyStruct;
type MyTrait {
    fn ambiguous();
}
impl MyTrait for MyStruct {
    fn ambiguous() {  }
}
impl fn MyStruct::ambiguous { } // ERROR: ambiguous associated function
// instead:
impl fn <MyStruct as MyTrait>::ambiguous { } // OK
```

When the type of the associated function has generics, they will be handles as follows

```rust
struct MyStruct<T>(T);
impl<T> MyStruct<T> {
    fn get() -> T { .. }
}

impl<T> fn MyStruct<T>::get { }
// or fully described:
impl<T> fn() -> T MyStruct<T>::get { }
```

---

When a function has generics, the function type is forced to be described, and the generic should be placed at it's desired position:
```rust
fn send<T: Send>(val: T, postal_code: u32) {}
impl<T: Send> ParcelStation for fn(T, u32) send {
    /* ... */
}
```

When we have an implicit generic, the same rule applies
```rust
fn implicit_generic(val: impl Clone) -> impl ToString {}
impl<T: Clone, U: ToString> for fn(T) -> U implicit_generic {
    /* ... */
}
```

---

When functions have lifetimes, they have to be included in the types
```rust
fn log(text: &str) { .. }
impl<'a> Logger for fn(&'a str) log {
    /* ... */
}
```

When the lifetime is explicitly defined on the function signature and there's no other rule forcing us to describe the function type, we can take a shortcut as follows
```rust
fn log<'a>(text: &'a str) { .. } // explicit lifetime 'a
impl<'a> Logger for fn<'a> log {
    /* ... */
}
```

---

Just as structs and enums have the possibility to derive traits to automatically generate code, function type have similar ways via attribute macros:

```rust
#[debug_signature]
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

# Additional ToDo's

## Change the fn type syntax for consistency

When we try to compile the current [code snippet](https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=cba2a1391c3e431a7499c6bf427b350d) 
```rust
fn cool<'a, T: Clone>(val: &'a T) -> (i32, bool) {
    todo!()
}

fn main() {
    let _a: () = cool;
}
```
we get following error:
```
error[E0308]: mismatched types
 --> src/main.rs:6:18
  |
6 |     let _a: () = cool;
  |             --   ^^^^ expected `()`, found fn item
  |             |
  |             expected due to this
  |
  = note: expected unit type `()`
               found fn item `for<'a> fn(&'a _) -> (i32, bool) {cool::<_>}`

For more information about this error, try `rustc --explain E0308`.
```
For consistency, we should change the syntax to `for<'a, T: Clone> fn(&'a T) -> (i32, bool) cool` (I'm not sure if we should put generics in the for)

# Drawbacks
[drawbacks]: #drawbacks

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The type behind functions already exists, we just need to expose it to the user.

# Prior art
[prior-art]: #prior-art

i dont know any

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Is the syntax good? It could create confusion between a function pointer.
- What about closures? They don't even have names so targetting them would be quite difficult. I wouldn't want to use the compiler generated mess of a name like `[closure@src/main.rs:13:18: 13:20]`. It would also contain line numbers which would be changing quite often so thats not ideal.

# Future possibilities
[future-possibilities]: #future-possibilities

- Also expose the type of closures
