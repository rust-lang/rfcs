- Feature Name: `repr_type_aliases`
- Start Date: 2024-06-14
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Primitive representations on enums now accept type aliases, meaning that in addition to primitives like `#[repr(u32)]`, `#[repr(tag = core::ffi::c_int)]` and `#[repr(tag = my_type)]` are now accepted.

# Motivation
[motivation]: #motivation

For the same reasons why type aliases are useful, having type aliases in `repr` attributes would also be useful. A few examples:

* Types depend on an external API whose exact size may be uncertain. (e.g. `core::ffi::c_int`, `gl::types::GLsizei`)
* An internal API might want to be able to easily change a type later.
* The intent behind a type alias may be clearer than simply using the primitive directly.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Enums allow `#[repr(tag = type)]` attributes to explicitly tag variants using the given type. (`type` can be any primitive integer type, like `u8`, `i32`, or `usize`, but not `char`.) If all variants of the enum are unit variants, this means that the enum will be easily castable to `type` using `as`. Otherwise, the discriminant will still be a tag of the specified type, but unsafe code is required to actually access it. (Future RFCs like [#3607] might change this.)

[#3607]: https://github.com/rust-lang/rfcs/pull/3607

To ensure compatibility, the `#[repr(tag = type)]` form is required if the type is a type alias instead of a known primitive type. While this may cause additional issues compared to the native `#[repr(type)]` syntax, it is possible to work around these issues as explained below:

> Since `type` is resolved as a path instead of being a hard-coded value, it's possible to shadow existing primitive types and cause weird results. For example, if a scope contains `type u32 = u8`, then `#[repr(tag = u32)` means `#[repr(u8)]`, not `#[repr(u32)]`. You can always refer to these types directly, however, and `#[repr(tag = ::std::u32)]` will always mean `#[repr(u32)]`.
>
> It's also worth noting that this syntax allows using type aliases with names of other `repr` arguments, like `C`. For example, `type C = u8` in the scope means that `#[repr(tag = C)]` means `#[repr(u8)]`, not `#[repr(C)]`.

You can use any type alias in the `repr` attribute, but it *must* be an alias to an accepted primitive type like `u8` or `i32`, and cannot be a pointer, reference, struct, etc. See the [future possibilities] section for some potential alternatives that may be allowed in the future.

[future possibilities]: #Future_possibilities

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The `repr` attribute now accepts a `tag = ...` argument to indicate an arbitrary type instead of a well-known primitive type. It accepts both fully-resolved types (including all necessary generics) as well as macros that expand to types.

If the resulting type resolves to a type alias to a valid primitive type which can be used in the `repr` attribute, that will be used as a tag for the discriminant representation. Otherwise, the result is an error which states that only primitive integer types and type aliases to them are allowed.

An automatically-applicable lint should be added that warns a user if a `repr` argument is invalid but references in an-scope type alias without the `tag = ` prefix. (For example, `#[repr(MyType)]` becomes `#[repr(tag = MyType)]`, but `#[repr(C)]` does not warn even if an alias `C` is in scope.)

To aid macro authors, an allow-by-default lint should be added that warns a user if they use `tag = ...` with a relative path instead of an absolute one, since the resolved type depends on the existing scope. For example, `tag = u32` or `tag = my_type` could potentially be shadowed by another type in the scope, but `tag = ::std::u32` and `tag = $crate::my_type` will always work as expected. This could potentially be relegated to a `clippy::pedantic` lint instead of a `rustc` lint.

# Drawbacks
[drawbacks]: #drawbacks

The requirement for `tag =` is unfortunate, but it feels like the best way to ensure that adding new representations isn't a breaking change going forward. Even if we were to decide it weren't a "breaking change," it would still break things anyway, being de-facto breaking. Some people have expressed desire for a dedicated syntax instead, which would fix this problem, but this can still be added after the fact.

Some proc macros like [`zerocopy::FromBytes`] may also have to complicate their logic if they depend on checking the existing `#[repr]` attribute for validity. In particular, since they can no longer exactly know which primitive type is being used for the representation, they would have to instead depend on associated constants on the types like `BITS` to verify which type is being used. Instead of being able to emit an error at macro expansion time, this error will have to be triggered at constant evaluation time or runtime instead, which is unfortunate.

Complicating proc macros in this way is considered acceptable for the following reasons:

* Since errors can still be included in constants now that `const` panics are stable, errors can still be caught at compile time.
* Users of special `repr` attributes are more likely to have more knowledge about specifics of how these proc macros work, and are thus more equipped to deal with weird errors like these.
* Ultimately, the benefits of adding the feature outweigh this negative.

[`zerocopy::FromBytes`]: https://docs.rs/zerocopy/latest/zerocopy/derive.FromBytes.html

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

We could always not do this.

But more realistically, here are some alternative designs that were rejected.

## `type`, `discriminant`

The previous proposals for this RFC used `type = ...` and `discriminant = ...` instead of `tag = ...`. Initially, this decision was chosen because `discriminant` was long to type, but it was ultimately decided that `discriminant` is more clear and that the extra typing is worth it. However, the lang team was swayed by the brevity of `tag` and, because this representation is explicitly a tagged discriminant, the shorter `tag` seems most fitting.

RFC [#3607] is considering `discriminant` but may be moving to `tag` also, so, it's worth considering the outcome of that RFC as well.

Note that `tag` also works because alternative representations could be considered. For example, an enum of two other enums with disjoint values could have a representation that avoids tags altogether, while still having the same "discriminant type" of the two individual enums.

## `self::`

We could, instead of using `type =`, require that all types contain a double-colon to indicate they're a path, effectively preventing collisions with arguments that aren't paths. This would require using `self::` for types that are imported in the local scope, and was actually the first proposal of this RFC, but wasn't very well-received.

## Shadowing attributes

Until a future edition, the current set of valid representations could be solidified as taking precedence over any shadowed identifiers. For example, if someone defines `type transparent = u32`, then `repr(transparent)` still means `repr(transparent)` and not `repr(u32)`.

In future editions, we could either:

* Let type aliases shadow all valid representations. This isn't ideal since there is no way to override the shadowing besides nesting your code in a new module and then re-exporting it outside that module, which is very messy.
* Expand the list of unshadowable representations every edition where necessary.

# Prior art
[prior-art]: #prior-art

This was previously suggested in [#1605] and was rejected at the time. The primary reasoning at the time was due to the lack of advanced syntax in attributes which were able to support this feature, which is no longer the case; the compiler has improved a lot in 8 years!

[#1605]: https://github.com/rust-lang/rfcs/pull/1605

# Unresolved questions
[unresolved-questions]: #unresolved-questions

* Is `tag` really the best name, or should it be `discriminant` instead?

# Future possibilities
[future-possibilities]: #future-possibilities

In the future, more potential discriminant types could be added which might be useful:

* `repr(transparent)` structs over valid types
* Floating-point primitives
* `char` or other restricted primitives like `NonZeroU*` and `NonZeroI*`
* Arbitrary `StructuralEq` types

Of course, all of these would require larger changes to the compiler than the ones proposed, and are currently left as future extensions.

[#3607]: https://github.com/rust-lang/rfcs/pull/3607
