- Feature Name: reformed_fn_pointers
- Start Date: 2015-03-21
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Make Rust's current function pointer types unsized, and introduce function reference types and new function pointer types, in order to make function references/pointers work more like value references/pointers.

This is based on [RFC 883](https://github.com/rust-lang/rfcs/pull/883) and related discussions, where the following design is already largely agreed upon, but the author of RFC 883 doesn't have time to revise it. Therefore, this RFC is created to push for the changes.

# Motivation

Currently in Rust, there are two kinds of types that represent functions:

1. Function item types: the types of function items, `fn(arg_list) -> ret_type {foo}`s.
2. Function pointer types: the types of pointers to functions, `fn(arg_list) -> ret_type`s.

Though called "function pointers", the function pointer types are considered *value* types (not reference/pointer types) by the language, which has the following problems:

1. Inconsistencies in the language, especially when it comes to FFI codes that deal with nullable foreign function pointers. 
2. There is currently no easy way to express the lifetime constraint on a function pointer to a non-`'static` function, which in particular makes it harder to implement type-safe JIT-ed functions/hot-loaded plugins in the future.

Thus, this RFC proposes the following solution.

# Detailed design

Make the current function pointer types unsized, and introduce function reference types of the form `&fn(arg_list) -> ret_type` and new (const) function pointer types of the form `*const fn(arg_list) -> ret_type`.

In the following section, `fn{f}`s, `fn`s, `&fn`s and `*const fn`s denote function item types, current function pointer types, function reference types and new function pointer types, respectively. Those types are considered "compatible" if their `(arg_list) -> ret_type` parts match.

The following rules will apply:

1. `fn{f}`s are still the function item types, or "function handles/proxies/values".
2. `fn`s are no longer function pointer types, but types representing "incomplete function items", which are unsized statically and zero-sized dynamically.
3. `&fn`s are function reference types, DST pointers with "auxiliary data" of type `()`.
4. `&fn`s and `*const fn`s work like normal references/pointers and casting between compatible `&fn`s and `*const fn`s is valid.
5. There are no `&mut fn`s, also no `*mut fn`s.
6. `fn{f}`s still implement the closure traits (`Fn`/`FnMut`/`FnOnce`).
7. `fn`s still implement the closure traits, for keeping `&fn`s coercible to closure trait objects.
8. `&fn`s will implement the closure traits.
9. The `fn{f} -> fn` coercions between compatible `fn{f}`s and `fn`s are no longer valid.
10. The `&fn{f} -> &fn` coercions between compatible `fn{f}`s and `fn`s are valid as unsizing coercions.

Optional changes that can be applied now or after Rust's final stabilization:

1. Make `fn{f}`s zero-sized statically to save space and better align with the fact that `fn`s are zero-sized dynamically.
2. Make `&fn{f}`s implement closure traits for better symmetry between `fn{f}`s and `fn`s. 

Notes:

1. Currently, both `&fn{f}`s and `&fn`s are coercible to closure trait objects, but are not closures themselves. After the changes, they will be closures (`&fn`s) or coercible to closures (`&fn{f}`s). If the second optional change happens, then `&fn{f}`s will also be closures, without coercions.
2. Source codes using `fn`s will have to use `&fn`s or `*const fn`s instead.
3. In previous revisions of this RFC, `fn`s were going to be interpreted as types representing "function bodies", not "incomplete function items". But then it was pointed out that every type in Rust must have dynamically determinable sizes. This means, for `fn`s, the sizes must all be the same, because `&fn`s cannot actually carry any auxiliary data, or they will not be thin pointers. The obvious special size value here is zero. It would be weird for function bodies to be considered zero-sized, so `fn`s are reinterpreted as "incomplete function items".
4. In previous revisions of this RFC, there were another optional change: implementing `Deref<Target=fn>`s on `fn{f}`s. This were intended to stress the fact that `fn{f}`s were themselves pointer-like constructs "pointing to" function bodies. But now that `fn`s will not be interpreted as "function bodies", this optional change will make no sense. Therefore it is dropped.

Examples:

```rust
fn foo() { ... }
fn unboxed_hof<F: Fn()>(f: F) { ... }
fn boxed_hof(f: &Fn()) { ... }

let bar = foo; // valid and unchanged
let old_fn_ptr: fn() = foo; // currently valid, but will be invalid
let fn_ref: &fn() = &foo; // the new `&fn{f} -> &fn` coercion
let fn_ptr = fn_ref as *const fn(); // the new `&fn -> *const fn` cast

unboxed_hof(foo); // valid and unchanged
unboxed_hof(&foo); // currently invalid, but will be valid, `&foo` coerced to `&fn()`, a closure
boxed_hof(foo); // invalid both before and after the changes
boxed_hof(&foo); // valid and unchanged, `&foo` coerced to `&Fn()`, a closure trait object

let nullable_value_ptr: *const ValueType = ...; // for comparison
let old_nullable_fn_ptr: Option<fn()> = ...; // currently valid, but a workaround, will be invalid
let nullable_fn_ref: Option<&fn()> = ...; // directly replaces the above after the changes
let nullable_fn_ptr: *const fn() = ...; // consistent with nullable value pointers after the changes

// Note:
// Some of the lines above are valid currently and their semantics will not be changed,
// but some others, while still valid, will take on new meanings.
```

# Drawbacks

This involves breaking changes.

However, currently function pointers are not used much. (See [this comment](https://github.com/rust-lang/rfcs/pull/883#issuecomment-76291284) for some statistics.)

# Alternatives

Please see [RFC 883](https://github.com/rust-lang/rfcs/pull/883) and related discussions for the various alternatives.

# Unresolved questions

None.
