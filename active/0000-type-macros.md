- Start Date: 2014-07-22
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

This adds the ability to have macros in type signatures.

# Motivation

* Using a common set of type parameters. `A<int, CommonParameters!()>` and `B<CommonParameters!()>`

* Use procedural macros to generate values at the type level. For example `PackStr!("hello")` and `PackInt!(43)` with corresponding `UnpackStr!(Type)` and `UnpackInt!(Type)`

* Emulate field offsets passed as type parameters. Using `declare_field!(Type, field_name)` as an item and later `OffsetOf!(Type, field_name)` to get a type representing the field.

* Completeness. It's currently suprising that macros won't work in type signatures.

# Detailed design

Anywhere a type is allowed, an type macro is also allowed using the existing macro infrastructure.

# Drawbacks

None.

# Alternatives

None.

# Unresolved questions

None.