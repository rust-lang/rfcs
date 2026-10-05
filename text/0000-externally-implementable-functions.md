- Feature Name: `extern_impl_fn`
- Start Date: 2024-05-10
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary

A mechanism for defining a function whose implementation can be defined (or overridden) in another crate.

Example 1:

```rust
// core::panic:

extern impl fn panic_handler(_: &PanicInfo) -> !;

// user:

impl fn core::panic::panic_handler(panic_info: &PanicInfo) -> ! {
    eprintln!("panic: {panic_info:?}");
    loop {}
}
```

Example 2:

```rust
// log crate:

extern impl fn logger() -> Logger {
    Logger::default()
}

// user:

impl fn log::logger() -> Logger {
    Logger::to_stdout().with_colors()
}
```

# Motivation

We have several items in the standard library that are overridable/definable by the user crate.
For example, the (no_std) `panic_handler`, the global allocator for `alloc`, and so on.

Each of those is a special lang item with its own special handling.
Having a general mechanism simplifies the language and makes this functionality available for other crates, and potentially for more use cases in core/alloc/std.

# Explanation

A function can be defined as "externally implementable" using `extern impl` as follows:

```rust
// In crate `x`:

// Without a body:
extern impl fn a();

// With a body:
extern impl fn b() {
    println!("default impl");
}
```

Another crate can then provide (or override) the implementation of these functions using `impl fn` syntax (using their path) as follows:

```rust
// In another crate:

impl fn x::a() {
    println!("my implementation of a");
}

impl fn x::b() {
    println!("my implementation of b");
}
```

# Details

## Signature

It is an error to have a different signature for the `impl fn` item.

(Whether `#[track_caller]` is used or not is considered part of the signature here.)

## One impl

It is an error to have no `impl fn` item (in any crate) for an `extern impl fn` item without a body.

It is an error to have multiple `impl fn` items (across all crates) for the same `extern impl fn` item.

Note: This means that adding or removing an `impl fn` item is a semver incompatible change.

## Visibility

`extern impl fn` items can have a visibility specifier (like `pub`), which determines who can *call* the function (or create pointers to it, etc.).

*Implementing* the function can be done by any crate that can name the item.
(The `impl fn` item will need to name the item to implement, which could be directly or through an alias/re-export.)

# Implementation

The implementation will be based on the same mechanisms as used today for the `panic_handler` and `#[global_allocator]` features.

The compiler of the root crate will find the implementation of all externally implementable functions and give an error
if more than one implementation is found for any of them.
If none are found, the result is either an error, or—if the `extern impl fn` has a default body—an implementation
is generated that calls that default body.

# Drawbacks

- It encourages globally defined behaviour.
  - Counterargument: We are already doing this anyway, both inside the standard library (e.g. panic_handler, allocator)
    and outside (e.g. global logger). This just makes it much easier (and safer) to get right.
- This will invite the addition of many hooks to the standard library to modify existing behavior.
  While we should consider such possibilities, this RFC does not propose that every piece of standard library behavior should be replaceable.

# Rationale and alternatives

## Syntax

The syntax re-uses existing keywords. Alternatively, we could:
  - Use the `override` reserved keyword.
  - Add a new (contextual) keyword (e.g. `existential fn`).
  - Use an attribute (e.g. `#[extern_impl]`) instead.

## Functions or statics

This RFC only proposes externally implementable *functions*.

An alternative is to only provide externally definable *statics* instead.

That would be equivalent in power: one can store a function pointer in a static, and one can return a reference to a static from a function ([RFC 3635](https://github.com/rust-lang/rfcs/pull/3635)).

(Another alternative, of course is to provide both. See future possibilities.)

## Visibility

There are two kinds of visibilities to be considered for externally implementable functions:
who can *implement* the function, and who can *call* the function.

Not allowing the function to be implemented by other crates nullifies the functionality, as the entire point of externally implementable functions is that they can be implemented in another crate. This visibility is therefore always (implicitly) "pub".

Allowing a more restricted (that is, not `pub`) visibility for *calling* the function can be useful. For example, today's `#[panic_handler]` can be defined by any crate, but can not be called directly. (Only indirectly through `panic!()` and friends.)

A downside is that it is not possible to allow this "only implementable but not publicly callable" visibility through an alias.

An alternative could be to use the same visibility for both implementing an calling, which would simply mean that the function (or an alias to it) will always have to be `pub`.

## Configuration

An `extern impl fn` may have `#[cfg(...)]` attributes applied to it as usual. For instance, a crate may only provide an `extern impl fn` with a given feature flag enabled, and might then use the same feature flag to conditionally provide make other functions depending on that `extern impl fn`. This is a useful pattern for crates that don't want to provide a default implementation but want to avoid producing a compilation error unless the function is needed.

# Prior art

[RFC 2494 "Existential types with external definition"](https://github.com/rust-lang/rfcs/pull/2492)
has been proposed before, which basically does this for *types*. Doing this for functions (as a start) saves a lot of complexity.

# Unresolved questions

- Should we provide a mechanism to set an `extern impl fn` using `=` from an existing `fn` value, rather than writing a body? For instance, `impl fn x::y = a::b;`
- Should we allow some form of subtyping, similarly to how traits allow trait impls to do subtyping?
- What should the syntax be once we stabilize this?
- How should this work in dynamic libraries?
- Should there be a way to specify that implementing the function is unsafe, separately from whether the function itself is unsafe?
- Should not having an implementation be an error when the function is never called?
- If we do end up designing and providing an `extern impl Trait` feature in addition to `extern impl fn`, should we *only* provide `extern impl Trait`, or is there value in still providing `extern impl fn` as well? This RFC proposes that we should still have `extern impl fn`, for the simpler case, rather than forcing such functions to be wrapped in traits.
- An `extern impl fn` that's marked as `pub(crate)` but is nonetheless pub to *implement* could surprise people. Is there some way we can make this less surprising? Should we require that all `extern impl fn` have `pub` visibility?

# Future possibilities

- Adding a syntax to specify an existing function as the impl. E.g. `impl core::panic_handler = my_panic_handler;`
- Doing this for `static` items too. (Perhaps all items that can appear in an `extern "Rust" { … }` block.)
- Using this for existing overridable global behavior in the standard library, like the panic handler, global allocator, etc.
- We could add a mechanism for arbitrating between multiple provided implementations. For instance, if a crate A depended on B and C, and both B and C provide implementations of an `extern impl fn`, rather than an error, A could provide its own implementation overriding both.
- Using this mechanism in the standard library to make more parts overridable. For example:
  - Allowing custom implementations of `panic_out_of_bounds` and `panic_overflowing_add`, etc.
    (The Rust for Linux project would make use of this.)
  - Allowing overriding `write_to_stdout` and `write_to_stderr`.
    (This enables custom testing frameworks to capture output. It is also extremely useful on targets like wasm.)
- This could possibly be extended to groups of functions in the form of a `trait` that can be globally implemented.
  (E.g. `extern impl AsyncRuntime`, to say that there must be a global implementation of that trait.)
  - Given an `extern impl Trait` feature, could we provide a compatibility mechanism so that a crate providing an `extern impl fn` can migrate to an `extern impl Trait` in a compatible way, such that crates doing `impl fn` will still be compatible with the new `extern impl Trait`?
