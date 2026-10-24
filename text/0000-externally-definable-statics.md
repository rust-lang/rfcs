- Feature Name: `extern_static`
- Start Date: 2024-05-13
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary

A mechanism for defining a static whose definition can be provided (or overridden) in another crate.

Example 1:

```rust
// core::panic:

extern static PANIC_HANDLER: fn(_: &PanicInfo) -> !;

// user:

impl static core::panic::PANIC_HANDLER = |panic_info| {
    eprintln!("panic: {panic_info:?}");
    loop {}
};
```

Example 2:

```rust
// log crate:

extern static LOGGER: Logger = Logger::default();

// user:

impl static log::LOGGER = Logger::to_stdout().with_colors();
```

# Motivation

We have several items in the standard library that are overridable/definable by the user crate.
For example, the (no_std) `panic_handler`, the global allocator for `alloc`, and so on.

Each of those is a special lang item with its own special handling.
Having a general mechanism simplifies the language and makes this functionality available for other crates, and potentially for more use cases in core/alloc/std.

# Explanation

A `static` can be defined as "externally definable" using the `extern` keyword as follows:

```rust
// In crate `x`:

// Without a default:
extern static A: fn();

// With a default:
extern static B: fn() = || {
    println!("default impl");
};
```

Another crate can then provide (or override) the definition of these statics using `impl static` syntax (using their path) as follows:

```rust
// In another crate:

impl static x::A = || {
    println!("my implementation of A");
};

impl static x::B = || {
    println!("my implementation of B");
};
```

# Details

## Signature

The type on the `impl static` item is optional, as it is taken from the `extern static` item referred to by the path.
If it is given, the types must match.

## One definition

It is an error to have no `impl static` item (in any crate) for an `extern static` item without a default value.

It is an error to have multiple `impl static` items (across all crates) for the same `extern static` item.

Note: This means that adding or removing an `impl static` item is a semver incompatible change.

## Safety

By marking the `extern static` as `unsafe`, defining it becomes unsafe and will also require the `unsafe` keyword.

Example:

```rust
// In crate `x`:

unsafe extern static A: fn();

// In another crate:

unsafe impl static A = || {
    println!("hello");
};
```

## Visibility

`extern static` items can have a visibility specifier (like `pub`), which determines who can access the static.

*Defining* the static can be done by any crate that can name the item.
(The `impl static` item will need to name the item to define, which could be directly or through an alias/re-export.)

# Implementation

The implementation will be based on the same mechanisms as used today for the `panic_handler` and `#[global_allocator]` features.

The compiler of the root crate will find the implementation of all externally definable statics and give an error
if more than one definition is found for any of them.
If none are found, the result is either an error, or—if the `extern static` has a default value—a definition is emitted with that value.

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
  - Add a new (contextual) keyword (e.g. `existential static`, `define static`, …).
  - Use an attribute (e.g. `#[externally_definable]`) instead.

## Functions or statics

This RFC only proposes externally definable *statics*.

An alternative is to only provide externally implementable *functions* instead ([RFC 3632](https://github.com/rust-lang/rfcs/pull/3632)).

That would be equivalent in power: one can store a function pointer in a static, and one can return a reference to a static from a function.

(Another alternative, of course is to provide both. See future possibilities.)

## Visibility

There are two kinds of visibilities to be considered for externally definable statics:
who can *define* the static, and who can *access* the static.

Not allowing the static to be defined by other crates nullifies the functionality, as the entire point of externally definable statics is that they can be defined in another crate. This visibility is therefore always (implicitly) "pub".

Allowing a more restricted (that is, not `pub`) visibility for *accessing* the static can be useful.
For example, today's `#[panic_handler]` can be defined by any crate, but can not be called directly. (Only indirectly through `panic!()` and friends.)

A downside is that it is not possible to allow this "only definable but not publicly accessible" visibility through an alias.

An alternative could be to use the same visibility for both defining an accessing, which would simply mean that the static (or an alias to it) will always have to be `pub`.

# Prior art

[RFC 2494 "Existential types with external definition"](https://github.com/rust-lang/rfcs/pull/2492)
has been proposed before, which basically does this for *types*. Doing this for statics (as a start) saves a lot of complexity.

# Unresolved questions

- Should we allow some form of subtyping, similarly to how traits allow trait impls to do subtyping?
- What should the syntax be once we stabilize this?
- How should this work in dynamic libraries?
- Should not having an implementation be an error when the static is never used (after dead code elimination)?

# Future possibilities

- Doing this for `fn` items too. (Perhaps all items that can appear in an `extern "Rust" { … }` block.)
- Using this for existing overridable global behavior in the standard library, like the panic handler, global allocator, etc.
- Using this mechanism in the standard library to make more parts overridable. For example:
  - Allowing custom implementations of `panic_out_of_bounds` and `panic_overflowing_add`, etc.
    (The Rust for Linux project would make use of this.)
  - Allowing overriding `STDOUT` and `STDERR` in std.
    (This enables custom testing frameworks to capture output. It is also extremely useful on targets like wasm.)
- This could possibly be extended to traits, by allowing a trait te globally implemented.
  (E.g. `extern impl AsyncRuntime`, to say that there must be a global implementation of that trait.)
