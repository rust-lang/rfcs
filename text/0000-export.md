- Feature Name: `export`
- Start Date: 2023-04-19
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Dynamically Linked Crates

This is a proposal for a new `#[export]` attribute to greatly simplify
the creation and use of dynamic libraries.

This proposal complements the ["crabi" ABI](https://github.com/rust-lang/rust/pull/105586) proposal.

## Problem statement

Imagine a simple library crate with just one simple function, and an application that uses it:

```rust
//! library crate

pub fn hello() {
    println!("Hello!");
}
```

```rust
//! application crate

fn main() {
    library::hello();
}
```

By default, Cargo will automatically build both crates and **statically** link them into a single binary.

However, there are many reasons why one might want to **dynamically** link the library instead.
The use cases for dynamic linking can be roughly split into two:

1. Cases where both the dynamic library and application are compiled with the exact same compiler (on the same platform, with the same settings) and shipped together.
2. Cases where dynamic library and application can be compiled and shipped separately from each other.

The first situation is currently relatively well supported by Rust.
The Rust compiler itself falls into this category, where we ship a single `librustc_driver.so` (or .dll or equivalent)
file that is used by `rustc`, `rustfmt`, `rustdoc`, and `clippy`.
The motivation is simply to reduce the binary size of the overall package containing all these tools.

The second situation has far more use cases and currently not supported well by Rust.
A common use case is a library that is shipped as part of the system (e.g. `libz.so` or `kernel32.dll`),
in which case you want to use the version provided by the system the program is run on,
and not from the system it was compiled on.
In these cases, dynamically linking is important to make sure the library can be independently updated.
(And it also helps to not blow up binary sizes.)

We need a good solution for this second category of use cases.

### Solution today

Currently, a way to implement this would make use of a combination of `extern "C"`, `#[no_mangle]` and `unsafe`,
each of which has major downsides.

It'd look something like this:

```rust
//! library crate

pub fn hello() {
    println!("Hello!");
}

#[no_mangle]
pub extern "C" fn some_unique_name_for_hello() {
    hello();
}
```

```rust
//! library bindings crate

#[link(name = "library")]
extern "C" {
    fn some_unique_name_for_hello();
}

#[inline]
pub fn hello() {
    unsafe { some_unique_name_for_hello() };
}
```

```rust
//! application crate

fn main() {
    library_bindings::hello();
}
```

This is bad. It's very verbose and error prone. More specifically:

- `#[no_mangle]` is needed to export a symbol under a stable name, but it requires manually picking a good unique name that won't collide with other items from other crates.
- A stable ABI is necessary to allow linking code from a different compiler (version),
  but `extern "C"` puts severe limitations on the function signatures,
  as most Rust types can't directly pass through the C ABI.
- `unsafe` code is required, because the compiler cannot validate the imported symbol matches the expected function signature.
  Importing the wrong library (with the same symbol name) could result in unsoundness.
- There are now two library crates: one that will be compiled into the dynamic library (the .dll/.so/.dylib file),
  and one that provides the bindings to that dynamic library.
  The second library likely fully inlined into the final application,
  as it only has wrappers, just to bring back the original (safe) function signatures.

Much of this solution could be automated by a procedural macro,
but splitting a library crate in two falls outside of what a procedural macro can reasonably do.

### Proposed solution sketch

Instead of all the manual usage of `#[no_mangle]`, `extern`, and `unsafe`,
a much better solution would look as closely as possible to the original code.

With the proposal below, one only needs to add an `#[export]` attribute, and give the function a stable ABI
(e.g. `extern "C"` or (in the future) `extern "crabi"`):

```rust
//! library crate

#[export]
pub extern "C" fn hello() {
    println!("Hello!");
}
```

```rust
//! application crate

fn main() {
    library::hello();
}
```

The library can then be either linked statically or dynamically, by informing cargo of the choice:

```diff
  [dependencies]
- library = { path = "..." }
+ library = { path = "...", dynamic = true }
```

## Proposal

Creating and using dynamic libraries involves three things:

1. A stable ABI that can be used for the items that are exported/imported.
2. A way to export and import items.
3. A way to create and use dynamic libraries.

For (1) we currently only have `extern "C"`, which only suffices for very simple cases.
This proposal does not include any improvements for (1),
but the ["crabi" proposal](https://github.com/rust-lang/rust/pull/105586) proposes the creation
of a new `extern "…"` ABI that is more flexible, which perfectly complements this proposal.

This proposal provides solutions for (2) and (3).
Exporting (and importing) items is done through a new language feature: the `#[export]` attribute.
Creating and using dynamic libraries is made easy through a new Cargo feature: `dynamic` dependencies.

### The `#[export]` Attribute

The `#[export]` attribute is used to mark items which are "stable" (in ABI/layout/signature)
such that they can be used across the border between (separately compiled) dynamically linked libraries/binaries.

The `#[export]` attribute can be applied to any public item that is *exportable*.
Which items are *exportable* is something that can increase over time with future proposals.
Initially, only the following items are *exportable*:

- Non-generic functions with a stable ABI (e.g. `extern "C"`)
  for which every user defined type used in the signature is also marked as `#[export]`.
  - This includes type associated functions ("methods").
- Structs/enums/unions with a stable representation (e.g. `repr(i32)` or `repr(C)`).
- Re-exports of those items (`use` statements, `type` aliases).

An `#[export]` attribute can also be applied to a crate, module, and non-generic type `impl` block,
which is simply equivalent to applying the attribute to every public item within it.

For types, the `#[export]` attribute represents the commitment to keep the representation of the type stable.
(To differentiate from, for example, a `#[repr(i32)]` that only exists as an optimization rather than as a stable promise.)

For functions, the `#[export]` attribute will make the function available from the dynamic library
under a stable "mangled" symbol that uniquely represents its crate and module path *and full signature*.
(More on that below.)

For aliases of functions, an `#[export]` attribute on the `use` statement will use the
path (and name) of the alias, not of the original function.
(So it won't 'leak' the name of any (possibly private/unstable) module it was re-exported from.)

### Privacy

It is an error to export an item that is not public, or is part of a non-public module.
The set of exported items of a crate will always be a subset of the crate's public interface.

It's fine to `#[export]` a public alias of a public type from a private module:

```rust
mod a {
    pub extern "C" fn f() { … }
}

#[export]
pub mod b {
    pub use super::a::f;
}
```

(This will export the function f as `b::f`.)

### Importing Exported Items

Normally, when using a crate as a dependency, any `#[export]` attributes of that crate have no effect
and the dependency is statically linked into the resulting binary.

When explicitly specifying `dynamic = true` for the dependency with `Cargo.toml`,
or when using a `extern dyn crate …;` statement in the source code,
only the items marked as `#[export]` will be available and the dependency will be linked dynamically.

### Building Dynamic Dependencies

When using `dynamic = true` for a dependency, there is no need to build that full crate:
only the signatures of its exported items are necessary.
Cargo will pass a flag to the Rust compiler which will stop it from generating
code for non-exported items and function bodies.

A clear separation between "public dependencies" (which used in the interface)
and "private dependencies" (which are only used in the implementation) is required
to avoid building unnecessary indirect dependencies.
A system for that has been proposed in [RFC 1977](https://rust-lang.github.io/rfcs/1977-public-private-dependencies.html).

### Name Mangling and Safety

Because a dynamic dependency and the crate that uses it are compiled separately
and only combined at runtime,
it is impossible for the compiler to perform any (safety, borrow, signature, …) checks.
However, making a (perhaps accidental) change to a function signature or type
should not lead to undefined behavior at runtime.

There are two ways to solve this problem:

1. Make it the responsibility of the user.
2. Make it the responsibility of the loader/linker.

Option (1) simply means making everything `unsafe`, which isn't very helpful to the user.
Option (2) means the loader (the part that loads the dynamic library at runtime) needs to perform the checks.

Unless we ship our own loader as part of Rust binaries,
we can only make use of the one functionality available in the loaders of all operating systems:
looking up symbols by their name.

So, in order to be able to provide safety, the symbol name has to be unique for the full signature,
including all relevant type descriptions.

To avoid extremely long symbol names that contain a full (encoded) version of the function signature
and all relevant type descriptions, we use a 128-bit hash based on all this information.

For example, an exported item in `foo::bar` in the crate `mycrate` would be exported with a symbol name such as:

```
_RNvNtC_7mycrate3foo3bar_f8771d213159376fafbff1d3b93bb212
```

Where the first part is the (mangled) path and name of the item,
and the second part is the hexadecimal representation of a 128-bit hash of all relevant signature and type information.
The hash algorithm is still to be determined.

(See also the "alternatives" section below.)

### Type Information

As mentioned above, the hash in a symbol name needs to cover _all_ relevant type information.
However, exactly which information is and isn't relevant for safety is a complicated question.

#### Types with Public Fields

For a simple user defined type where all fields are both public, like the `Point` struct below,
the relevant parts are the size, alignment, and recursively all field information.

```rust
#[export]
#[repr(C)]
pub struct Point {
    pub x: f32,
    pub y: f32,
    pub name: &str,
}
```

The `#[export]` attribute is the user's commitment to keep the type stable, but without `unsafe`,
any mistakes should _not_ result in unsoundness.
Accidentally changing the struct to swap the `x` and `name` fields should result in a different hash,
such that the `f32` won't get interpreted as a `&str`, for example.

Note that, technically, the names of the type and the fields are not relevant, at least _not for memory safety_.
Swapping the `x` and `y` fields result in surprises and bugs and shouldn't be done,
but it won't result in undefined behaviour, since any Rust code can swap the fields without using `unsafe`.

However, for public fields, the field names are already part of the stable API, so we include them in the hash as well.

It is an error to use a plain `#[export]` attribute on a type with out stable `#[repr(…)]`,
if it has any private fields,
or if any of the fields are not of an `#[export]`ed or builtin type.

#### Types with Private Fields

For types where not all fields are public, the situation is much more complicated.

Private fields usually come with certain *invariants*, and come with `unsafe` code that makes assumptions about them.
For example, the private fields of a `Vec` are assumed to represent a valid "owned" pointer to an allocation together with its capacity and initialized size.

If it would be possible to define a identically named type with the same fields but different (or no) invariants/assumptions,
or just change the invariants in an existing library,
it'd be possible to cause undefined behavior by loading the "wrong" dynamic library.

Therefore, we can't allow a regular `#[export]` attribute on a type with private fields,
since we have no way of automatically determining the invariants / unsafe assumptions about private fields.

Instead, for these types, we must require the user to *unsafely* commit to
ABI stability if they want to make the type available to exported functions.

Using `#[export(unsafe_stable_abi = «hash»)]`, one can make the (unsafe) promise
that the type will remain ABI compatible as long as the provided hash remains the same.
The hash must have been randomly generated to ensure uniqueness (which is part of the unsafe promise).

```rust
#[export(unsafe_stable_abi = "ca83050b302bf0644a1417ac3fa6982a")]
#[repr(C)]
pub struct ListNode {
    next: *const ListNode,
    value: i32,
}
```

In this case, using the type as part of a function signature will not result in a hash based on the full (recursive) type definition,
but will instead be based on the user provided hash (and the size and alignment of the type).

### Standard Library

Once the ["crabi"](https://github.com/rust-lang/rust/pull/105586) feature has progressed far enough,
we should consider adding `#[export]` attributes to some standard library types, effectively committing to a stable ABI for those.
For example, `Option`, `NonZero`, `File`,
and many others are good candidates for `#[export(unsafe_stable_abi)]`
(if the "crabi" ABI doesn't already handle them specially).

## Future Possibilities

- A `#[no_export]` attribute, which can be useful when marking an entire crate or module as `#![export]`.
- Options within the `#[export(…)]` attribute for e.g. a future name mangling scheme version.
- `#[export(by_path)]` to export a symbol only based on the path, without the hash of relevant safety/type information.
  This is useful in situations where safety is not a primary concern, to simplify cases like using Rust code from another language (with a simple symbol name).
  (Importing (or using) such a symbol from Rust will be `unsafe`.)
- `#[export(opaque)]` (or `#[export(indirect)]`) for opaque types that can only be used indirectly (e.g. through a pointer or reference) in exported items,
  such that their size is not a stable ABI promise.
- Exportable `static`s.
- Exportable `trait`s, for e.g. dynamic trait objects. (See also https://github.com/rust-lang/rust/pull/105586.)
- A tool to create a stripped, 'dynamic import only' version of the crate source code,
  with only the exported items, without the function bodies. (Essentially a "header file" or "bindings-only crate".)
  - Alternatively or in addition to that, a way to optionally include such a "header file" or "bindings"
    (or the same metadata in some other (perhaps Rust-agnostic) format)
    inside the resulting dynamic library file.
- Allow exporting two identically named items to create a dynamic library that is backwards compatible with an older interface,
  including both a symbol for the old and new interface.
- Next to the hash of the type information,
  additionally and optionally include the full type information in an extra section,
  to allow for (debug) tools to accurately diagnose mismatched symbol errors.
- Some kind of `#[export_inline]` feature to allow for functions that will be inlined into the calling crate,
  rather than being part of the dynamic library, which will only be able to call exported items.

## Alternatives

- Alternatives for using a hash of all relevant type information:
  - Don't include type information in the symbols,
    but make using a dynamic dependency `unsafe` by requiring e.g. `unsafe extern dyn crate …;`.
  - Include the full (encoded) type information in the symbols, without hashing it.
    This results in extremely long symbol names, and all the type information will be recoverable
    (which might be useful or might be undesirable, depending on the use case).
    This can result in significantly larger binary sizes.
  - Don't include type information in the symbols, but include the information in another way (e.g. an extra section).
    If we do this, we can't make use of the loader/linker for the safety checks,
    so we'll have to include extra code in Rust binaries that will perform the checks separately
    before using any dynamic dependency.

## What this Proposal is not

Questions like

- How do panics propagate across dynamically linked crates or FFI boundaries?
- How can allocated types can cross an export boundary and be dropped/deallocated on the other side?

are **not** solved by `#[export]`, but instead are the responsibility of the ABI.

The existing `extern "C"` ABI 'solves' these by simply not having any such features.

The [`extern "crabi"` ABI](https://github.com/rust-lang/rust/pull/105586)
will attempt to solve these (but perhaps not in the first version),
but that falls outside the scope of this RFC.

(A separate RFC for the first version of "crabi" might very well appear soon. ^^)

Separately, the question of how this will be (optionally) used for the standard library is another question entirely,
which is left for a later proposal. (Although the hope is that this RFC gives at least a rough idea of how that might work.)
