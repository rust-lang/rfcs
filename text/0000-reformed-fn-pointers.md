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

Make the current function pointer types unsized, and introduce function reference types of the form `&fn(arg_list) -> ret_type` and new (const) function pointer types of the form `&const fn(arg_list) -> ret_type`.

In the following section, `fn{f}`s denote function item types, `fn`s, `&fn`s and `*const fn`s denote current function pointer types, function reference types and new function pointer types, respectively. Those types are considered "compatible" if their `arg_list -> ret_type` parts match.

The following rules will apply:

1. `fn{f}`s are still the function item types.
2. `fn`s are no longer function pointer types, but unsized types representing the bodies of functions.
3. `&fn`s are function reference types, DST pointers with "auxiliary data" of type `()`.
4. `&fn`s and `*const fn`s work like normal references/pointers and casting between compatible `&fn`s and `*const fn`s is valid.
5. There are no `&mut fn`s, also no `*mut fn`s.
6. `fn{f}`s still implement the closure traits (`Fn`/`FnMut`/`FnOnce`).
7. `fn`s still implement the closure traits, for keeping `&fn`s coercible to closure trait objects.
8. `&fn`s will implement the closure traits.
9. The `fn{f} -> fn` coercions between compatible `fn{f}`s and `fn`s are no longer valid.
10. The `&fn{f} -> &fn` coercions between compatible `fn{f}`s and `fn`s are valid as unsizing coercions.

Notes:

1. Currently, both `&fn{f}`s and `&fn`s are coercible to closure trait objects, but are not closures themselves. After the changes, they will be closures (`&fn`s) or coercible to closures (`&fn{f}`s).
2. Source codes using `fn`s will have to use `&fn`s or `*const fn`s instead.

Optional changes that can be applied now or after Rust's final stabilization:

1. Make `fn{f}`s zero-sized.
2. Implement `Deref<Target=fn()>` on `fn{f}`s. This enables the `*` operator on `fn{f}` values, which doesn't seem to have practical uses. However, depending on how one interprets the nature of `fn{f}`s and `fn{f} -> fn` coercions, this can be a desirable change. For some, this change can stress the fact that `fn{f}`s are pointer-like (they are copyable handles to function bodies, not the function bodies themselves) and they see `&fn{f} -> &fn`s as deref coercions.

Examples:

```rust
fn foo() { ... }
fn unboxed_hof<F: Fn()>(f: F) { ... }
fn boxed_hof(f: &Fn()) { ... }

let bar = foo; // still valid
let old_ptr_to_foo: fn() = foo; // currently valid, but will be invalid
let ref_to_foo: &fn() = &foo;
let ptr_to_foo = ref_to_foo as *const fn();

unboxed_hof(foo); // still valid
unboxed_hof(&foo); // currently invalid, but will be valid, `&foo` coerced to `&fn()`, a closure
boxed_hof(&foo); // still valid, `&foo` coerced to `&Fn()`, a closure trait object

let nullable_ptr_to_value: *const ValueType = ...; // for comparison
let old_nullable_ptr_to_fn: Option<fn()> = ...; // currently valid, but a workaround, will be invalid
let nullable_ref_to_fn: Option<&fn()> = ...; // directly replaces the above after the changes
let nullable_ptr_to_fn: baz: *const fn() = ...; // consistent with nullable value pointers after the changes
                                                // (currently a nullable pointer to a non-null function pointer, not to a function)
```

# Drawbacks

This involves breaking changes.

However, currently function pointers are not used much. (See [this comment](https://github.com/rust-lang/rfcs/pull/883#issuecomment-76291284) for some statistics.)

# Alternatives

Please see [RFC 883](https://github.com/rust-lang/rfcs/pull/883) and related discussions for the various alternatives.

# Unresolved questions

None.
