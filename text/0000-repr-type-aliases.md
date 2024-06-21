- Feature Name: `repr_type_aliases`
- Start Date: 2024-06-14
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Primitive representations on enums now accept type aliases, meaning that in addition to primitives like `#[repr(u32)]`, `#[repr(discriminant = core::ffi::c_int)]` and `#[repr(discriminant = my_type)]` are now accepted.

# Motivation
[motivation]: #motivation

For the same reasons why type aliases are useful, having type aliases in `repr` attributes would also be useful. A few examples:

* Types depend on an external API whose exact size may be uncertain. (e.g. `core::ffi::c_int`, `gl::types::GLsizei`)
* An internal API might want to be able to easily change a type later.
* The intent behind a type alias may be clearer than simply using the primitive directly.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Enums allow `#[repr(discriminant = type)]` attributes to offer an explicit discriminant type. (`type` can be any primitive integer type, like `u8`, `i32`, or `usize`, but not `char`.) If all variants of the enum are unit variants, this means that the enum will be easily castable to `type` using `as`. Otherwise, the discriminant will still be of the specified type, but unsafe code is required to actually access it. (Future RFCs like [#3607] might change this.)

[#3607]: https://github.com/rust-lang/rfcs/pull/3607

To ensure compatibility, the `#[repr(discriminant = type)]` form is required if the type is a type alias instead of a known primitive type. Depending on circumstances, it might be more beneficial to use the old `#[repr(type)]` syntax instead of `#[repr(discriminant = type)]` syntax, explained below:

> Since `type` is resolved as a path instead of being a hard-coded value, it's possible to shadow existing primitive types and cause weird results. For example, if a scope contains `type u32 = u8`, then `#[repr(discriminant = u32)` means `#[repr(u8)]`, not `#[repr(u32)]`. You can always refer to these types directly, however, and `#[repr(discriminant = ::std::u32)]` will always mean `#[repr(u32)]`.
>
> It's also worth noting that this syntax allows using type aliases with names of other `repr` arguments, like `C`. For example, `type C = u8` in the scope means that `#[repr(discriminant = C)]` means `#[repr(u8)]`, not `#[repr(C)]`.

You can use any type alias in the `repr` attribute, but it *must* be an alias to an accepted primitive type like `u8` or `i32`, and cannot be a pointer, reference, struct, etc. See the [future possibilities] section for some potential alternatives that may be allowed in the future.

[future possibilities]: #Future_possibilities

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The `repr` attribute now accepts a `discriminant = ...` argument to indicate a resolved path instead of a well-known primitive type. If those the path resolves to a type alias to a valid primitive type which can be used in the `repr` attribute, that will be used as the actual discriminant representation.

An automatically-applicable lint should be added that warns a user if a `repr` argument is invalid but references in an-scope type alias without the `discriminant = ` prefix. (For example, `#[repr(MyType)]` becomes `#[repr(discriminant = MyType)]`, but `#[repr(C)]` does not warn even if an alias `C` is in scope.)

To aid macro authors, an allow-by-default lint should be added that warns a user if they use `discriminant = ...` with a relative path instead of an absolute one, since the resolved type depends on the existing scope. For example, `discriminant = u32` or `discriminant = my_type` could potentially be shadowed by another type in the scope, but `discriminant = ::std::u32` and `discriminant = $crate::my_type` will always work as expected. This could potentially be relegated to a `clippy::pedantic` lint instead of a `rustc` lint.

# Drawbacks
[drawbacks]: #drawbacks

The requirement for `discriminant =` is unfortunate, but it feels like the best way to ensure that adding new representations isn't a breaking change going forward. Even if we were to decide it weren't a "breaking change," it would still break things anyway, being de-facto breaking.

And, of course, this complicates the compiler. But that's about it.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

We could always not do this.

But more realistically, here are some alternative designs that were rejected.

## `type`

The second proposal for this RFC used `type = ...` instead of `discriminant = ...`. Initially, this decision was chosen because `discriminant` was long to type, but it was ultimately decided that `discriminant` is more clear and that the extra typing is worth it. Additionally, RFC [#3607] proposes using `discriminant` in its proposed syntax, so, this would be further in line with that as well.

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

In the future, more potential discriminant types could be added which might be useful:

* `repr(transparent)` structs over valid types
* Floating-point primitives
* `char` or other restricted primitives like `NonZeroU*` and `NonZeroI*`

Of course, all of these would require larger changes to the compiler than the ones proposed, and are currently left as future extensions.

[#3607]: https://github.com/rust-lang/rfcs/pull/3607
