- Feature Name: `repr_type_aliases`
- Start Date: 2024-06-14
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Primitive representations on enums now accept type aliases, meaning that in addition to primitives like `#[repr(u32)]`, `#[repr(type = core::ffi::c_int)]` and `#[repr(type = my_type)]` are now accepted.

# Motivation
[motivation]: #motivation

For the same reasons why type aliases are useful, having type aliases in `repr` attributes would also be useful. A few examples:

* Types depend on an external API whose exact size may be uncertain. (e.g. `core::ffi::c_int`, `gl::types::GLsizei`)
* An internal API might want to be able to easily change a type later.
* The intent behind a type alias may be clearer than simply using the primitive directly.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Enums allow `#[repr(type = ...)]` attributes to offer an explicit discriminant type. (`...` can be any primitive integer type, like `u8`, `i32`, or `usize`, but not `char`.) If all variants of the enum are unit variants, this means that the enum will be easily castable to `type` using `as`. Otherwise, the discriminant will still be of the specified type, but unsafe code is required to actually access it.

To ensure compatibility, the `#[repr(type = ...)]` form is required if the type is not one of the known primitive types. Note that this form is not necessarily equivalent to using the primitive representations directly, since shadowing is possible; for example, if you did `type u32 = u8` and then `#[repr(type = u32)]`, this would be equivalent to `#[repr(u8)]`, not `#[repr(u32)]`.

You can use any type alias in the `repr` attribute, but it *must* be an alias to an accepted primitive type like `u8` or `i32`, and cannot be a pointer, reference, struct, etc.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The `repr` attribute now accepts a `type = ...` argument to indicate a resolved path instead of a well-known primitive type. If those the path resolves to a type alias to a valid primitive type which can be used in the `repr` attribute, that will be used as the actual discriminant representation.

An additional, automatically-applicable lint should be added that warns a user if they use `type = ...` for a well-known primitive type, since adding `type = ` instead of using the type directly introduces the possibility of shadowing. (For example, `#[repr(type = u32)]` becomes `#[repr(u32)]`.)

Similarly, an automatically-applicable lint should be added that warns a user if a `repr` argument references in an-scope type alias without the `type = ` prefix. (For example, `#[repr(MyType)]` becomes `#[repr(type = MyType)]`.)

# Drawbacks
[drawbacks]: #drawbacks

The requirement for `type =` is unfortunate, but it feels like the best way to ensure that adding new representations isn't a breaking change going forward. Even if we were to decide it weren't a "breaking change," it would still break things anyway, being de-facto breaking.

And, of course, this complicates the compiler. But that's about it.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

We could always not do this.

But more realistically, here are some alternative designs that were rejected.

## `self::`

We could, instead of using `type =`, require that all types contain a double-colon to indicate they're a path, effectively preventing collisions with arguments that aren't paths. This would require using `self::` for types that are imported in the local scope, and was actually the first proposal of this RFC, but wasn't very well-received.

## Shadowing attributes

Until a future edition, the current set of valid representations could be solidified as taking precedence over any shadowed identifiers. For example, if someone defines `type transparent = u32`, then `repr(transparent)` still means `repr(transparent)` and not `repr(u32)`.

In future editions, we could either:

* Let type aliases shadow all valid representations. This isn't ideal since there is no way to override the shadowing besides nesting your code in a new module and then re-exporting it outside that module, which is very messy.
* Expand the list of unshadowable representations every edition where necessary.

## Capital letters

You could require that the types start with capital letters- oh, right, `repr(C)` is a thing.

# Prior art
[prior-art]: #prior-art

No known prior art exists.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None currently.

# Future possibilities
[future-possibilities]: #future-possibilities

Future RFCs like [#3607] propose explicit methods of obtaining enum discriminants, and that further justifies the desire to include a change like this. There aren't many other extensions that could be added, however.

[#3607]: https://github.com/rust-lang/rfcs/pull/3607
