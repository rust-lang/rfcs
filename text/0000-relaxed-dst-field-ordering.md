- Feature Name: `relaxed_dst_field_ordering`
- Start Date: 2024-10-18
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Relax the requirements on struct field ordering for dynamically sized fields:

* for `repr(Rust)`, `?Sized` fields can be anywhere in the field list, as long as there is only one
* for `repr(C)`, `?Sized` fields only have to be the last non-ZST field, and can be followed by ZST fields
* for `repr(transparent)`, apply both rules, since only one non-ZST field is allowed anyway

# Motivation
[motivation]: #motivation

Rust allows creating structs with dynamically sized fields, but in a very limited way: since the size is dynamic, dynamically sized fields must be the last field in the struct, to avoid making the offsets of fields also dynamic.

However, `repr(Rust)` allows reordering fields, and this is not not reflected in this rule for dynamically sized fields: no matter what you do, the dynamically sized field must be at the end. This is inconsistent with the rest of the language and limits the ability of authors to reorder the fields in a more natural way, like they can for statically sized structs.

Additionally, Rust has fully committed to zero-sized fields being truly invisible to struct layout, encoding this in the definition of `repr(transparent)`. So, why are dynamically sized fields unable to be followed by zero-sized fields, or reordered among statically sized fields in a `repr(Rust)` struct.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Before, structs were allowed to have dynamically sized types (DSTs) in their last field only. Now, this restriction has been relaxed to allow exactly one DST field, although it can occur anywhere inside the struct.

For `repr(C)` structs specifically, an additional requirement is added that the DST must be the last field that is not a zero-sized type (ZST), which is still more permissive than the previous definition.

The dynamically sized field will always be physically located at the end of the struct, although because `repr(Rust)` can reorder fields and because ZST fields do not affect layout, this doesn't have to be reflected in the actual struct definition.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This feature should be relatively easy to implement: we already reorder fields in `repr(Rust)` structs explicitly, so, this is just ensuring that DST fields are always placed last without relying on the definition order.

The code for `repr(transparent)` effectively doesn't change, and the code for ensuring that the DST is the last field can be mostly reused for `repr(C)`, assuming that ZST fields are still ignored.

# Drawbacks
[drawbacks]: #drawbacks

It's work to change the status quo.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This arguably simplifies the language and makes the DST field more in line with the existing field ordering rules.

But we could always not do it, I guess.

# Prior art
[prior-art]: #prior-art

None currently.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None currently.

# Future possibilities
[future-possibilities]: #future-possibilities

In the future, we'll hopefully have the ability to define custom DSTs, but such extensions are very compatible with this RFC.
