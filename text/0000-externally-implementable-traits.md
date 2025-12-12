- Feature Name: `extern_impl_trait`
- Start Date: 2024-05-22
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary

A mechanism for defining a set of functions whose implementation can be defined (or overridden) in another crate.

Example 1:

```rust
// core::panic:

extern trait PanicHandler {
    fn panic_handler(_: &PanicInfo) -> !;
}

// user:

impl core::panic::PanicHandler {
    fn panic_handler(panic_info: &PanicInfo) -> ! {
        eprintln!("panic: {panic_info:?}");
        loop {}
    }
}
```

Example 2:

```rust
// alloc:

unsafe extern trait GlobalAllocator {
    fn allocate(layout: Layout) -> Result<NonNull<[u8]>, AllocError>;

    unsafe fn deallocate(ptr: NonNull<u8>, layout: Layout);
}

// alloc::alloc:

fn alloc(layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
    GlobalAllocator::allocate(layout)
}

// user:


// Provided by the user
static GLOBAL: MyGlobalAlloc = ...;

// In the expansion of `#[global_allocator]`
unsafe extern impl GlobalAllocator {
    fn allocate(layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        GLOBAL.allocate(layout)
    }

    unsafe fn deallocate(ptr: NonNull<u8>, layout: Layout) {
        GLOBAL.deallocate(ptr, layout)
    }
}
```

Example 3:

```rust
// log crate:

extern trait GlobalLogger {
    fn logger() -> Logger {
        Logger::default()
    }
}

let logger = GlobalLogger::logger();

// user:

extern impl log::GlobalLogger {
    fn logger() -> Logger {
        Logger::to_stdout().with_colors()
    }
}
```

# Motivation

We have several items in the standard library that are overridable/definable by the user crate.
For example, the (no_std) `panic_handler`, the global allocator for `alloc`, and so on.

Each of those is a special lang item with its own special handling.
Having a general mechanism simplifies the language and makes this functionality available for other crates, and potentially for more use cases in core/alloc/std.

# Explanation

A set of functions can be defined as "externally implementable" by placing them in an `extern trait` as follows:

```rust
// In crate `x`:

// Without a body:
extern trait Foo {
    fn a();
}

// With a body:
extern trait Bar {
    fn b() {
        println!("default impl");
    }
}
```

These functions can be called directly as items under the namespace of the `extern trait`:

```rust
// In crate `x`:

Foo::a();
Bar::b();
```

Another crate can then provide (or override) the implementation of these functions using `extern impl` syntax as follows:

```rust
// In another crate:

extern impl x::Foo {
    fn a() {
        println!("my implementation of a");
    }
}

extern impl x::Bar {
    fn x::b() {
        println!("my implementation of b");
    }
}
```

# Details

## Not actually a trait

Despite the use of the `trait` keyword, `extern trait` does not define a normal trait which can be used in generic bounds. It is instead a separate "kind" of item which can only be used with `extern impl`.

The rationale for using the `trait` keyword is that this feature is similar to traits: one crate defines a trait with an interface while another crate implements that inferface.

## Members

An `extern trait` may contain functions but not other associated items such as types or constants. Additionally, these functions may not refer to `self` or `Self` in their signature. Effectively, these functions follow the same rules as free functions outside an `impl` block.

## Signatures

As with a normal trait `impl`, the signatures of functions in the `extern impl` must match those defined in the `extern trait`.

(As with normal traits, whether `#[track_caller]` is used or not is considered part of the signature here.)

## Implementations

An `extern impl` must provide implementations for all functions defined in an `extern trait`, except those for which a default implementation is provided.

It is an error to have no `extern impl` item (in any crate) for an `extern trait` item, unless *all* functions in the `extern trait` have a default implementation.

It is an error to have multiple `extern impl` items (across all crates) for the same `extern trait` item.

Note: This means that adding or removing an `extern impl` item is a semver incompatible change.

## Visibility

`extern trait` items can have a visibility specifier (like `pub`), which determines who can *call* functions in the trait (or create pointers to it, etc.). Individual functions in the trait may not have a visibility modifer.

*Implementing* the trait can be done by any crate that can name the item.
(The `extern impl` item will need to name the item to implement, which could be directly or through an alias/re-export.)

# Implementation

The implementation will be based on the same mechanisms as used today for the `panic_handler` and `#[global_allocator]` features.

The compiler of the root crate will find the implementation of all externally implementable traits and give an error
if more than one implementation is found for any of them.
If none are found, the result is either an error, or—if the all functions in an `extern trait` have a default body—an implementation is generated that calls those default bodies.

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
  - Add a new (contextual) keyword (e.g. `extern existential`).
  - Use an attribute (e.g. `#[extern_impl]`) instead.

## Functions or statics

This RFC only proposes externally implementable *functions*.

An alternative is to only provide externally definable *statics* instead.

That would be equivalent in power: one can store a function pointer in a static, and one can return a reference to a static from a function ([RFC 3635](https://github.com/rust-lang/rfcs/pull/3635)).

(Another alternative, of course is to provide both. See future possibilities.)

## Visibility

There are two kinds of visibilities to be considered for externally implementable traits:
who can *implement* the trait, and who can *call* functions in the trait.

Not allowing the trait to be implemented by other crates nullifies the functionality, as the entire point of externally implementable trait is that they can be implemented in another crate. This visibility is therefore always (implicitly) "pub".

Allowing a more restricted (that is, not `pub`) visibility for *calling* functions in the trait can be useful. For example, today's `#[panic_handler]` can be defined by any crate, but can not be called directly. (Only indirectly through `panic!()` and friends.)

A downside is that it is not possible to allow this "only implementable but not publicly callable" visibility through an alias.

An alternative could be to use the same visibility for both implementing an calling, which would simply mean that the `extern trait` (or an alias to it) will always have to be `pub`.

## Configuration

An `extern trait` may have `#[cfg(...)]` attributes applied to it as usual. For instance, a crate may only provide an `extern trait` with a given feature flag enabled, and might then use the same feature flag to conditionally provide make other functions depending on that `extern trait`. This is a useful pattern for crates that don't want to provide a default implementation but want to avoid producing a compilation error unless the trait is needed.

# Prior art

[RFC 2494 "Existential types with external definition"](https://github.com/rust-lang/rfcs/pull/2492)
has been proposed before, which basically does this for *types*. Doing this for functions (as a start) saves a lot of complexity.

This RFC is effectively an alternate syntax for [RFC 3632 "externally implementable functions"](https://github.com/rust-lang/rfcs/pull/3632). The syntax proposed in this RFC is cleaner since it allows safety on the trait and functions to be specified separately. Additionally it allows cleaner grouping of related functions such as those used for the global allocator.

# Unresolved questions

- What should the syntax be once we stabilize this?
- Is this an appropriate use of the `trait` keyword?
- How should this work in dynamic libraries?
- An `extern trait` that's marked as `pub(crate)` but is nonetheless pub to *implement* could surprise people. Is there some way we can make this less surprising? Should we require that all `extern trait` have `pub` visibility?

# Future possibilities

- Doing this for `static` items too. (Perhaps all items that can appear in an `extern "Rust" { … }` block.)
- Using this for existing overridable global behavior in the standard library, like the panic handler, global allocator, etc.
- We could add a mechanism for arbitrating between multiple provided implementations. For instance, if a crate A depended on B and C, and both B and C provide implementations of an `extern impl fn`, rather than an error, A could provide its own implementation overriding both.
- Using this mechanism in the standard library to make more parts overridable. For example:
  - Allowing custom implementations of `panic_out_of_bounds` and `panic_overflowing_add`, etc.
    (The Rust for Linux project would make use of this.)
  - Allowing overriding `write_to_stdout` and `write_to_stderr`.
    (This enables custom testing frameworks to capture output. It is also extremely useful on targets like wasm.)
