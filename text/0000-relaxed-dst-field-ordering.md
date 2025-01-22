- Feature Name: `relaxed_dst_field_ordering`
- Start Date: 2024-10-18
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Relax the requirements on struct field ordering for dynamically sized fields for `repr(Rust)` and `repr(transparent)`, such that `?Sized` fields can be anywhere in the field list, as long as there is only one.

# Motivation
[motivation]: #motivation

Rust allows creating structs with dynamically sized fields, but in a very limited way: since the size is dynamic, dynamically sized fields must be the last field in the struct, to avoid making the offsets of fields also dynamic.

However, `repr(Rust)` allows reordering fields, and this is not not reflected in this rule for dynamically sized fields: no matter what you do, the dynamically sized field must be at the end. This is inconsistent with the rest of the language and limits the ability of authors to reorder the fields in a more natural way, like they can for statically sized structs.

Additionally, Rust has fully committed to zero-sized fields being truly invisible to struct layout, encoding this in the definition of `repr(transparent)`. So, why are dynamically sized fields unable to be followed by zero-sized fields, or reordered among statically sized fields in a `repr(Rust)` struct?

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Before, structs were allowed to have dynamically sized types (DSTs) in their last field only. Now, this restriction has been relaxed to allow at most one DST field, although it can occur anywhere inside the struct.

For `repr(C)` structs specifically, the old requirement that DSTs be at the end remains.

The dynamically sized field will always be physically located at the end of the struct, although because `repr(Rust)` can reorder fields, this doesn't have to be reflected in the actual struct definition.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

For `repr(transparent)` structs, the layout is altered so that ZST fields are always reordered to the beginning of the struct, so that their offsets are always zero. This is technically different than the status quo, since you could have the offset be the size of the struct, but this has never been specified or guaranteed. This ensures that the offsets are still static.

Per the current requirements, `repr(transparent)` structs still can only have ZSTs with trivial alignment, and types like `[T; 0]` will still be rejected.

# Drawbacks
[drawbacks]: #drawbacks

It's work to change the status quo.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

We could extend the rules on ZST field ordering to `repr(C)` too, to allow ZSTs past the DST field. However, since `repr(C)` does imply strict ordering, it's more likely that people have been relying on the offsets for ZSTs in structs, and it's better to avoid breaking this.

# Prior art
[prior-art]: #prior-art

None currently.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None currently.

# Future possibilities
[future-possibilities]: #future-possibilities

In the future, we'll hopefully have the ability to define custom DSTs, but such extensions are very compatible with this RFC.
