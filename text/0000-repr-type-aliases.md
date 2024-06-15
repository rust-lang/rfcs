- Feature Name: `repr_type_aliases`
- Start Date: 2024-06-14
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Primitive representations on enums now accept type aliases, meaning that in addition to primitives like `#[repr(u32)]`, `#[repr(core::ffi::c_int)]` and `#[repr(self::my_type)]` are now accepted.

# Motivation
[motivation]: #motivation

For the same reasons why type aliases are useful, having type aliases in `repr` attributes would also be useful. A few examples:

* Types depend on an external API whose exact size may be uncertain. (e.g. `core::ffi::c_int`, `gl::types::GLsizei`)
* An internal API might want to be able to easily change a type later.
* The intent behind a type alias may be clearer than simply using the primitive directly.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Enums allow `#[repr(type)]` attributes to offer an explicit discriminant type. (`type` can be any primitive integer type, like `u8`, `i32`, or `usize`, but not `char`.) If all variants of the enum are unit variants, this means that the enum will be easily castable to `type` using `as`. Otherwise, the discriminant will still be of the specified type, but unsafe code is required to actually access it.

In addition to the primitive types themselves, you can also use the path to a type alias in the `repr` attribute instead, and it will resolve the primitive type of the type alias. However, to ensure compatibility as new potential representations are added, the path to the alias must contain a double-colon: you can access an alias `Alias` defined in the same module by using `self::Alias`.

For example, `#[repr(core::ffi::c_int)]` is valid because it contains a double-colon, but a `use core::ffi::c_int` followed by `#[repr(c_int)]` is not. If you wanted to `use core::ffi::c_int` first, then you could still do `#[repr(self::c_int)]` to reference the type.

You can use any type alias in the `repr` attribute, but it *must* be an alias to an accepted primitive type like `u8` or `i32`, and cannot be a pointer, reference, struct, etc.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The `repr` attribute now accepts arguments containing double-colon tokens, which will be parsed as paths to type aliases to resolve. If those type aliases resolve to a valid primitive type which can be used in the `repr` attribute, that will be used as the actual discriminant representation.

An additional, automatically-applicable lint should be added if a user references a valid type alias in the current scope without including multiple components in the path, recommending to add `self::` to the beginning to ensure forward-compatibility.

# Drawbacks
[drawbacks]: #drawbacks

The requirement for `self::` on already-imported types is unfortunate, but it feels like the best way to ensure that adding new representations isn't a breaking change going forward. Even if we were to decide it weren't a "breaking change," it would still break things anyway, being de-facto breaking.

And, of course, this complicates the compiler. But that's about it.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

We could always not do this.

But more realistically, here's an alternative design that would avoid the `self::` change, whose complexity feels worse than the `self::` requirement:

* Until a future edition, the current set of valid representations is solidified as taking precedence over any shadowed identifiers. For example, if someone defines `type transparent = u32`, then `repr(transparent)` still means `repr(transparent)` and not `repr(u32)`.
* At said future edition, type aliases now shadow all valid representations. So, for example, defining `type transparent = u32` would truly mean `repr(u32)`, not `repr(transparent)`. The only way to actually reference `repr(transparent)` would be to not have a type inside the current scope named `transparent`. There could be a deny-by-default warning when people do this.

Alternatively, you could require that the types start with capital letters- oh, right, `repr(C)` is a thing. It feels like there's not a good way to solve the shadowing problem besides adding in something that will never be a part of future representation names, that is, a double-colon token representing a path.

# Prior art
[prior-art]: #prior-art

No known prior art exists.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None currently.

# Future possibilities
[future-possibilities]: #future-possibilities

Future RFCs like #3607 propose explicit methods of obtaining enum discriminants, and that further justifies the desire to include a change like this. There aren't many other extensions that could be added, however.
