- Feature Name: safe-intrinsics
- Start Date: 2015-08-12
- RFC PR:
- Rust Issue:

# Summary

Allow intrinsics to be marked as "safe", allowing them to be called without the use of `unsafe`.

# Motivation

While many intrinsics are inherently unsafe, e.g. `transmute`, `init`, `copy`, many are not. Often
this is not a significant problem as they can be wrapped in a safe function however this
indirection is not always desirable. Notably, such indirection causes the `likely` and `unlikely`
intrinsics from RFC 1131 to be non-functional.

While the intrinsics can be called directly in unsafe contexts, such use of unsafe is misleading
for intrinsic functions like `size_of`.

The ability to mark intrinsics as safe also opens the possibility of exposing said intrinsics
directly via re-export instead of using wrapper functions. This RFC does not address re-exporting
intrinsics, but the ability to do so for safe functions is an important motivation for the feature.

# Detailed design

Introduce an attribute `#[safe]` that can be applied to the intrinsic definitions that overrides
the implicit `unsafe` applied to foreign functions. This attribute would only be valid on
intrinsics, the justification being that compiler *does* know that they are safe. The attribute
would be behind a feature gate with no planned stabilisation.

All obviously-safe intrinsics can then be marked as safe:

* `size_of`
* `min_align_of`, `pref_align_of`
* `size_of_val`, `min_align_of_val`
* `type_name`
* `type_id`
* `needs_drop`
* Math intrinsics
* Bit manipulation intrinsics
* `bswap16`, `bswap32`, `bswap64`
* Checked arithmetic
* `overflowing_add`, `overflowing_sub`, `overflowing_mul`

# Drawbacks

None.

# Alternatives

* Remove the restriction that the attribute is only valid on intrinsics. This runs counter to the
  idea that functions we don't know the bodies of are presumed unsafe.
* Hard-code the safety for each intrinsic. This ensures consistency since you can declare the
  intrinsics wherever you like, but makes the fact invisible until you use the intrinsic. It also
  requires more changes to the compiler.

# Unresolved questions

* The name of the attribute is not that important, it can be changed.
* Should we mark all the intrinsics noted as safe?
* Should the attribute get it's own feature gate, or should it reuse an existing feature such as
  `core_intrinsics`?
