- Feature Name: reformed_fn_pointers
- Start Date: 2015-03-21
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Repurpose current function pointer types to mean "function bodies". Introduce function reference types and new function pointer types, so that function references/pointers work more like value references/pointers.

This is based on [RFC 883](https://github.com/rust-lang/rfcs/pull/883) and related discussions, where the following design is already largely agreed upon, but the author of RFC 883 doesn't have time to revise it. Therefore, this RFC is created to push for the changes.

# Motivation

Currently in Rust, there are two kinds of types that represent functions:

1. Function item types, or function handle/proxy/value types: the types of function items, `fn(arg_list) -> ret_type {foo}`s.
2. Function pointer types: the types of pointers to functions, `fn(arg_list) -> ret_type`s.

Though called "function pointers", the function pointer types are considered *value* types (not reference/pointer types) by the language, which has the following problems:

1. Inconsistencies in the language, especially when it comes to FFI codes that deal with nullable foreign function pointers. 
2. There is currently no easy way to express the lifetime constraint on a function pointer to a non-`'static` function, which in particular makes it harder to implement type-safe JIT-ed functions/hot-loaded plugins in the future.

Thus, this RFC proposes the following solution.

# Detailed design

Repurpose current function pointer types to mean "function bodies". Introduce shared function reference types of the form `&fn(arg_list) -> ret_type`, and the variations: mutable function references, const function pointers, and mutable function pointers.

In the following section, `fn{f}`s denote function item types, `fn`s denote current function pointer types, `&fn`s, `&mut fn`s, `*const fn`s and `*mut fn`s are function references and their variations. Those types are considered "compatible" if their `(arg_list) -> ret_type` parts match.

The following rules will apply:

1. `fn{f}`s' representations and semantics remain unchanged.
2. `fn`s become unsized statically and (as an implementation detail) zero sized dynamically.
3. `&fn`s are DST pointers with auxiliary data of type `()`.
4. `*const fn`s, `&mut fn`s and `*mut fn`s work as expected, though `&mut fn`s and `*mut fn`s may not have practical uses.
5. `fn{f}`s still implement the closure traits (`Fn`/`FnMut`/`FnOnce`).
6. `fn`s still implement the closure traits, for keeping `&fn`s coercible to closure trait objects.
7. `&fn`s implement the closure traits, so they can be used in places expecting `fn`s currently. 
8. The `fn{f} -> fn` coercions between compatible `fn{f}`s and `fn`s are no longer valid.
9. The `&fn{f} -> &fn` coercions between compatible `fn{f}`s and `fn`s are valid as unsizing coercions.
10. Optional: `&fn{f}`s can implement the closure traits for better symmetry with `&fn`.
11. Optional: `&fn{f}`s can implement `Deref<Target=fn>` to stress the fact that `fn`s represent function bodies and `fn{f}`s are handles/proxies "to" `fn`s. 

Notes:

1. Currently, both `&fn{f}`s and `&fn`s are coercible to closure trait objects, but are not closures themselves. After the changes, they will be closures (`&fn`s) or coercible to closures (`&fn{f}`s). If the first optional change happens, then `&fn{f}`s will also be closures, without coercions.
2. Source codes using `fn`s will have to use `&fn`s or `*const fn`s instead. Due to the inference rules of the language, in practice, most uses of `&fn` would be `&'static fn`.
3. It is an *implementation detail* that `fn`s are zero sized dynamically. The actual intention is for `fn`s to be "truly unsized" when the necessary language support for those types are designed and implemented. (Please see [RFC 709](https://github.com/rust-lang/rfcs/pull/709) and [RFC Issue 813](https://github.com/rust-lang/rfcs/pull/813) for discussions about truly unsized types.)

Examples:

```rust
fn foo() { ... }
fn unboxed_hof<F: Fn()>(f: F) { ... }
fn boxed_hof(f: &Fn()) { ... }

let bar = foo; // valid and unchanged
let old_fn_ptr: fn() = foo; // currently valid, but will be invalid
let fn_ref: &'static fn() = &foo; // the new `&fn{f} -> &fn` coercion
let fn_ptr = fn_ref as *const fn(); // the new `&fn -> *const fn` cast

unboxed_hof(foo); // valid and unchanged
unboxed_hof(&foo); // currently invalid, but will be valid, `&foo` coerced to `&fn()`, a closure
boxed_hof(foo); // invalid both before and after the changes
boxed_hof(&foo); // valid and unchanged, `&foo` coerced to `&Fn()`, a closure trait object

let nullable_value_ptr: *const ValueType = ...; // for comparison
let old_nullable_fn_ptr: Option<fn()> = ...; // currently valid, but a workaround, will be invalid
let nullable_fn_ref: Option<&'static fn()> = ...; // directly replaces the above after the changes
let nullable_fn_ptr: *const fn() = ...; // consistent with nullable value pointers after the changes

// Note:
// Some of the lines above are valid currently and their semantics will not be changed,
// but some others, while still valid, will take on new meanings.
```

# Drawbacks

1. This involves breaking changes. However, currently function pointers are not used much. (See [this comment](https://github.com/rust-lang/rfcs/pull/883#issuecomment-76291284) for some statistics.)
2. This goes down a particular path in the type system that may have unforeseen interactions.
3. In order to hide the fact that `fn`s are zero-sized dynamically, functions like `size_of_val` would not be made usable on unsized types in the near future.

# Alternatives

#### A. Keep the status quo.

And stick with function pointers that aren't quite function pointers.

#### B. Make "`fn`s are dynamically zero sized" externally visible.

And interpret `fn`s as "incomplete function items", which aligns well with the plan to make `fn{f}`s (the "complete" function items) zero sized statically. (See [Rust Issue 19925](https://github.com/rust-lang/rust/issues/19925).)

However, it is likely that Rust will gain (other) truly unsized types one day, which is a more generally applicable solution. If possible, it is better to avoid special cases like "statically unsized but dynamically zero sized types". Thus it is better to avoid exposing the dynamic sizes of `fn`s for now.

#### C. Allow `fn{f} -> &'static fn`, not `&fn{f} -> &fn`.

Though `fn{f}`s are indeed pointer-like in a way, it is a bit strange that a value type can be coerced to a reference type. Also, the symmetry between `fn{f}`s and `fn`s will be lost.

#### D. Make function item types truly denote functions.

This alternative makes values of the type `fn{f}`s not copyable, and only `&fn{f}`s (and variations) can be passed around.

This has the advantage of having dedicated types for representing functions (new `fn{f}`s), and there will be no pointer-like handle types (current `fn{f}`s), only true function references/pointers (`&fn{f}`s and variations). This is theoretically purer.

However, function handles do have their advantages over function references/pointers:

1. Function handles can be zero-sized.
2. Function handles do not involve the `&` sigil which usually has something to do with indirections (not considering optimizations). The lack of `&` is a visual hint for the lack of indirection when calling functions through the handles.

Also, if `fn{f}`s are no longer `Copy`, more code will have to be changed to use `&fn{f}`, making this alternative a much larger-scale breaking change.

#### E. Make function item types `&'static fn{f}`s instead of `fn{f}`s.

This alternative eliminates the pointer-like handle types just like Alternative D does, without breaking every piece of code currently taking function handles as arguments.

This also has the pros and cons of having no function handles.

# Unresolved questions

What exactly are the "unforeseen interactions" in Drawback 2, if any?
