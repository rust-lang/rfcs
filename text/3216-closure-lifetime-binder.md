- Feature Name: `closure_lifetime_binder`
- Start Date: 2022-01-06
- RFC PR: [rust-lang/rfcs#3216](https://github.com/rust-lang/rfcs/pull/3216)
- Rust Issue: [rust-lang/rust#97362](https://github.com/rust-lang/rust/issues/97362)


This RFC went through a pre-RFC phase at <https://internals.rust-lang.org/t/pre-rfc-allow-for-a-syntax-with-closures-for-explicit-higher-ranked-lifetimes/15888>

# Summary

Allow explicitly specifying lifetimes on closures via `for<'a> |arg: &'a u8| { ... }`. This will always result in a higher-ranked closure which can accept *any* lifetime (as in `fn bar<'a>(val: &'a u8) {}`). Closures defined without the `for<'a>` syntax retain their current behavior: lifetimes will be inferred as either some local region (via an inference variable), or a higher-ranked lifetime.

# Motivation

There are several open issues around closure lifetimes (https://github.com/rust-lang/rust/issues/91966 and https://github.com/rust-lang/rust/issues/41078), all of which stem from type inference incorrectly choosing either a higher-ranked lifetime, or a local lifetime.

This can be illustrated in the following cases:

1. We infer a higher-ranked region ( `for<'a> fn(&'a u8)` ) when we really want some specific (local) region. This occurs in the following code:

```rust
fn main () {
    let mut fields: Vec<&str> = Vec::new();
    let pusher = |a: &str| fields.push(a);
}
```

which gives the error:

```
error[E0521]: borrowed data escapes outside of closure
 --> src/main.rs:3:28
  |
2 |     let mut fields: Vec<&str> = Vec::new();
  |         ---------- `fields` declared here, outside of the closure body
3 |     let pusher = |a: &str| fields.push(a);
  |                   -        ^^^^^^^^^^^^^^ `a` escapes the closure body here
  |                   |
  |                   `a` is a reference that is only valid in the closure body
```

The issue is that `Vec<&str>` is not higher-ranked, so we can only push an `&'0 str` for some specific lifetime `'0` . The `pusher` closure signature requires that it accept *any* lifetime, which leads to a compiler error.

2. We infer some specific region when we really want a higher-ranked region. This occurs in the following code:

```rust
use std::cell::Cell;

fn main() {
    let static_cell: Cell<&'static u8> = Cell::new(&25);
    let closure = |s| {};
    closure(static_cell);
    let val = 30;
    let short_cell: Cell<&u8> = Cell::new(&val);
    closure(short_cell);
}
```

The above code uses `Cell` to force invariance, since otherwise, region subtyping will make this example work even without a higher-ranked region. The above code produces the following error:

```
error[E0597]: `val` does not live long enough
  --> src/main.rs:8:43
   |
4  |     let static_cell: Cell<&'static u8> = Cell::new(&25);
   |                      ----------------- type annotation requires that `val` is borrowed for `'static`
...
8  |     let short_cell: Cell<&u8> = Cell::new(&val);
   |                                           ^^^^ borrowed value does not live long enough
9  |     closure(short_cell);
10 | }
   | - `val` dropped here while still borrowed
```

Here, the closure gets inferred to `|s: Cell<&'static u8>|` , so it cannot accept a `Cell<&'0 u8>` for some shorter lifetime `&'0` . What we really want is `for<'a> |s: Cell<&'a u8>|` , so that the closure can accept both `Cell` s.

It might be possible to create an 'ideal' closure lifetime inference algorithm, which always correctly decides between either a higher-ranked lifetime, or some local lifetime. Even if we were to implement this, however, the behavior of closure lifetimes would likely remain opaque to the majority of users. By allowing users to explicitly 'desugar' a closure, we can make it easier to teach how closures work. Users can also take advantage of the `for<>` syntax to explicitly indicate that a particular closure is higher-ranked - just as they can explicitly provide a type annotation for the parameters and return type - to improve the readability of their code.

Additionally, the Rust compiler currently accepts the following trait impls (and may eventually do so without any warnings):

```rust
trait Trait {}
impl<T> Trait for fn(&T) { }
impl<T> Trait for fn(T) { }
```

See https://github.com/rust-lang/rust/pull/72493#issuecomment-633307151

These impls are accepted because `for<'a> fn(&'a T)` and `fn(T)` are distinct types. While this not does *directly* apply to closures, closures *can* be cast to function pointers, which will have a different impl of `Trait` apply depending on whether they contain a higher-ranked lifetime parameter. Thus, the closure lifetimes inferred by the compiler can end up influencing what code is executed at runtime (provided that the user inserts the necessary cast to the correct function pointer type). While this is definitely an unusual case, it highlights the subtlety of lifetimes. Allowing greater control over how closure lifetimes are determined will allow users to better understand and control the behavior of their code in unusual situations like this one.

# Guide-level explanation

When writing a closure, you will often take advantage of type inference to avoid the need to explicitly specify types. For example:

```rust
fn func(_: impl Fn(&i32) -> &i32) {}

fn main() {
    func(|arg| { arg });
}
```

Here, the type of `arg` will be inferred to `&i32`, and the return type will also be `&i32`. We can write this explicitly:

```rust
fn func(_: impl Fn(&i32) -> &i32) {}

fn main() {
    func(|arg: &i32| -> &i32 { arg });
}
```

Notice that we've *elided* the lifetime in `&i32`. When a lifetime is written this way, Rust will infer its value based on how it's used.

In this case, our closure needs to be able to accept an `&i32` with *any* lifetime. This is because our closure needs to implement `Fn(&i32) -> &i32` - this is syntactic sugar for `for<'a> Fn(&'a i32) -> &'a i32`.

We can make this explicit by writing our closure in the following way:

```rust
fn func(_: impl Fn(&i32) -> &i32) {}

fn main() {
    func(for<'a> |arg: &'a i32| -> &'a i32 { arg });
}
```

This indicates to both the compiler and the user that this closure can accept an `&i32` with *any* lifetime, and returns an `&i32` with the same lifetime.

However, there are cases where a closure *cannot* accept any lifetime - it can only accept some particular lifetime. Consider the following code:

```rust
fn main() {
    let mut values: Vec<&bool> = Vec::new();
    let first = true;
    values.push(&first);

    let mut closure = |value| values.push(value);
    let second = false;
    closure(&second);
}
```

In this code, `closure` takes in an `&bool`, and pushes it to `values`. However, `closure` *cannot* accept an `&bool` with *any* lifetime - it can only work with some specific lifetime. To see this, consider this slight modification of the program:

```rust
fn main() {
    let mut values: Vec<&bool> = Vec::new();
    let first = true;
    values.push(&first);

    let mut closure = |value| values.push(value);
    { // This new scope was added
        let second = false;
        closure(&second);
    } // The scope ends here, causing `second` to be dropped
    println!("Values: {:?}", values);
}
```

This program fails to compile:

```
error[E0597]: `second` does not live long enough
  --> src/main.rs:9:17
   |
9  |         closure(&second);
   |                 ^^^^^^^ borrowed value does not live long enough
10 |     }
   |     - `second` dropped here while still borrowed
11 |     println!("Values: {:?}", values);
   |                              ------ borrow later used here
```

This is because `closure` can only accept an `&bool` with a lifetime that lives at least as long as `values`. If this code were to compile (that is, if `closure` could accept a `&bool` with the shorter lifetime associated with `&second`), then `values` would end up containing a reference to the freed stack variable `second`.

Since `closure` cannot accept *any* lifetime, it cannot be written as `for<'a> |value: &'a bool| values.push(value)`. It's natural to ask - how *can* we write down an explicit lifetime for `value: &bool`?

Unfortunately, Rust does not currently allow the signature of such a closure to be written explicitly. Instead, you must rely on type inference to choose the correct lifetime for you.

# Reference-level explanation

We now allow closures to be written with a `for<'a .. 'z>` prefix, where `'a .. 'z` is a comma-separated sequence of zero or more lifetimes. The syntax is parsed identically to the `for<'a .. 'z>` in the function pointer type `for<'a .. 'z> fn(&'a u8, &'b u8) -> &'a u8`.
This can be use with or without the `move` keyword:

`for<'a .. 'z> |arg1, arg2, ..., argN| { ... }`
`for<'a .. 'z> move |arg1, arg2, ..., argN| { ... }`

When this syntax is used, any lifetimes specified with the `for<>` binder are always treated as higher-ranked, regardless of any other hints we discover during type inference. That is, a closure of the form `for<'a, 'b> |first: &'a u8, second: &'b bool| -> &'b bool`  will have a compiler-generated impl of the form:

```rust
impl<'a, 'b> FnOnce(&'a u8, &'b bool) -> &'b bool for [closure type] { ... }
```

Using this syntax requires that the closure signature be fully specified, without any elided lifetimes or implicit type inference variables. For example, all of the following closures do **not** compile:

```rust
for<'a> |elided: &u8, specified: &'a bool| -> () {}; // Compiler error: lifetime in &u8 not specified
for<'b> || {}; // Compiler error: return type not specified
for<'c> |elided_type| -> &'c bool { elided_type }; // Compiler error: type of `elided_type` not specified
for<> || {}; // Compiler error: return type not specified
```

This restriction allows us to avoid specifying how elided lifetime should be treated inside a closure with an explicit `for<>`. We may decide to lift this restriction in the future.

Additionally, this syntax is currently incompatible with async closures:

```rust
for<'a> async |arg: &'a u8| -> () {}; // Compare error: `for<>` syntax cannot be used with async closures
for<'a> async move |arg: &'a u8| -> () {}; // Compare error: `for<>` syntax cannot be used with async closures
```

This restriction may be lifted in the future, but the interactions between this feature and the `async` desugaring will need to be considered.

# Drawbacks

This slightly increases the complexity of the language and the compiler implementation. However, the syntax introduced (`for<'a>`) can already be used in both trait bounds and function pointer types, so we are not introducing any new concepts in the languages.

Previously, we only allowed the `for<>` syntax in a 'type' position: function pointers (`for<'a> fn(&'a u8)`) and higher-ranked trait bounds (`where for<'a> T: MyTrait<'a>`). This RFC requires supporting the `for<>` syntax in an 'expression' position as well (`for<'a> |&'a u8| { ... }`).
Crates that handle parsing Rust syntax (e.g. `syn`) will need to be updated to support this.

There is an ambiguity when parsing `for <` in expression position: it can either be:
1. The start of a `for` loop with a fully qualified path used as a pattern: `for <MyType as MyTrait>::Assoc { field1, field2 } in my_iter { }`
2. The start of the generics for a higher-ranked closure: `for<'a> |my_arg: &'a u8| { .. }`

However, the same kind of ambiguity exists when parsing `impl <`: it can either be:
1. A fully-qualified path: `impl <MyType as MyTrait>::Assoc { ... }`
1. The start of the generics for an `impl` item: `impl<T> MyTrait for T { ... }`

We will handle disambiguation in the same way that we handle disambiguation for `impl <` (performing additional lookahead to determine which case we are in).

In its initial form, this feature may be of limited usefulness - it can only be used with closures that have all higher-ranked lifetimes, prevents type elision from being used, and does not provide a way of explicitly indicating *non*-higher-ranked lifetimes. However, this proposal has been explicitly designed to be forwards-compatible with such additions. It represents a small, (hopefully) uncontroversial step towards better control over closure signatures.

# Rationale and alternatives

* We could use a syntax other than `for<>` for binding lifetimes - however, this syntax is already used, and has the same meaning here as it does in the other positions where it is allowed.
* We could allow mixing elided and explicit lifetimes in a closure signature - for example, `for<'a> |first: &'a u8, second: &bool|`. However, this would force us to commit to one of several options for the interpretation of `second: &bool`

1. The lifetime in `&bool` continues to be inferred as it would be without the `for<'a>`, and may or may not end up being higher-ranked.
2. The lifetime in `&bool` is always *non*-higher-ranked (we create a region inference variable). This would allow for solving the closure inference problem in the opposite direction (a region is inferred to be higher-ranked when it really shouldn't be).
3. Treat the signature exactly how it would be treated if it appeared in a function definition (e.g. `fn my_fn<'a>(first: &'a u8, second: &bool) { ... }`). This would provide consistently between closure and function signatures, but would inhibit the region inference variable behavior that's unique to closures.

We can choose at most one of these options. By banning this ambiguous case altogether, we can allow users to begin experimenting with the (limited) `for<>` closure syntax, and later reach a decision about how (or not) to explicitly indicate non-higher-ranked regions.

* We could try to design a 'perfect' or 'ideal' closure region inference algorithm that always correctly chooses between a higher-ranked and non-higher-ranked region, eliminating the need for users to explicitly specify their choice. Even if this is possible and easy to implement, there's still value in allowing closures to be explicitly desugared for teaching purposes. Currently: function definitions, function pointers, and higher-ranked trait bounds (e.g. `Fn(&u8)`) can all have their lifetimes (mostly) manually desugared - however, closures do not support this.
* We could do nothing, and accept the status quo for closure region inference. Given the number of users that have run into issues in practice, this would mean keeping a fairly significant wart in the Rust language.

# Prior Art

I previously discussed this topic in Zulip: https://rust-lang.zulipchat.com/#narrow/stream/213817-t-lang/topic/Explicit.20closure.20lifetimes

The `for<>` syntax is used with function pointers (`for<'a> fn(&'a u8)`) and higher-ranked trait bounds (`fn bar<T>() where for<'a> T: Foo<'a> {}`)

I'm not aware of any languages that have anything analogous to Rust's distinction between higher-ranked and non-higher-ranked lifetimes, let alone an interaction with closure/lambda type inference.

# Unresolved questions

None at this time

# Future possibilities

We could allow a lifetime to be explicitly indicated to be *non*-higher-ranked. The `'_` lifetime could be given special meaning in closures - for example, `for<'a> |first: &'a u8, second: &'_ bool| {}` could be used to indicate a closure that takes in a `&u8` with any lifetime, and an `&bool` with some specific lifetime. However, we already accept `|second: &'_ bool| {}` as a closure, so this would require changing the behavior of `&'_` when a `for<>` binder is present.

## Appendix: Late-bound regions, early-bound regions, and region variables


There are three 'kinds' of lifetimes we need to consider for closures:

1. Late-bound lifetimes (also referred to as higher-ranked lifetimes). These lifetimes
   can be written in function pointers using the `for<>` syntax (e.g. `for<'a> fn(&'a u8) -> &'a u8`).
   When a lifetime is used in a function argument without any other 'restrictions' (see below), then the corresponding function pointer type will have a late-bound lifetime. For example, the function `fn bar<'a>(val: &'a u8) {}` can be cast to the function pointer type `for<'a> fn(&'a u8)`
2. Early-bound lifetimes. A lifetime becomes early-bound when it is 'constrained' in some way that prevents us from writing down the necessary bounds with a `for<>` binder. For example, the function `fn foo<'a>(&'a u8) where &'a u8: MyTrait<'a> {}` will have an early-bound lifetime `'a`, since we cannot write function pointer with a 'higher-ranked bound' like `for<'a> fn(&'a u8) where &'a u8: MyTrait<'a>`
3. Region variables. This corresponds to some particular region in the enclosing function body, and cannot be explicitly named by the user. This exact region is inferred by the compiler based on the closure usage. For example:

```rust
fn main() {
    let mut values: Vec<&bool> = Vec::new();
    let first = true;
    values.push(&first);

    let mut closure = |value| values.push(value);
    let second = false;
    closure(&second);
}
```


Here, the closure stored in variable `closure` takes in an argument of type `&'0 bool`, where `'0` is some region variable. The closure *cannot* accept a `&bool` with an *any* lifetime - only lifetimes that live at least as long as `'0`.

This RFC is only concerned with higher-ranked (late-bound) lifetimes and region variables.

See https://rustc-dev-guide.rust-lang.org/early-late-bound.html#early-and-late-bound-variables and https://rust-lang.github.io/rfcs/0387-higher-ranked-trait-bounds.html#distinguishing-early-vs-late-bound-lifetimes-in-impls for more discussion about early-bound vs late-bound regions.
