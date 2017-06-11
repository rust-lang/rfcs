- Feature Name: `more_visibilities`, or `variant_visibilities` and
`trait_item_visibilities` separately
- Start Date: 2017-06-12
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Support `pub`/`pub(restricted)` on enum variants and trait items.

# Motivation
[motivation]: #motivation

Enum variant and trait items already have visibilities internally in the
compiler, but they are always inherited from their enums and traits
respectively, this RFC supports specifying them explicitly in source code.
This can be useful for several reasons.

If variant is marked with smaller visibility than enum itself, for example
```rust
pub enum E {
    // This variant can be named only in the current crate.
    pub(crate) V1,
    // This variant can be named only in the current module.
    pub(self) V2,
}
```
, then it can restrict exhaustiveness of `match`ing performed on values of
enum `E`. If enum has a `pub(crate)` variant, it can be exhaustively matched
only in the current crate because the restricted variant cannot be named from
other crates. If enum has a `pub(self)` variant, it can be exhaustively matched
only in the current module, i.e. the exhaustiveness control is fine-grained.

If trait item is marked with smaller visibility than trait itself, for example
```rust
pub trait Trait1 {
    fn method1();
    pub(crate) fn method_without_default();
}
pub trait Trait2 {
    fn method2();
    pub(crate) fn method_with_default() {}
}
```
, then `Trait1` can be used in other crates (e.g. it can be imported, the
method `method1` can be called), but it cannot be implemented, because
`method_without_default` cannot be accessed from other crates.
`Trait2` can be used and implemented in other crates, it just have a private
helper method that can be used only locally (possibly for default
implementations of other methods).

# Detailed design
[design]: #detailed-design

Visibilities can be specified for enum variants and trait items at the beginning
of their declaration, but after attributes.

If visibility of a variant or trait item is not specified explicitly it's
inherited from its enum or trait respectively.

Variants / trait items with visibilities larger than visibilities of their
enums / traits are permitted (similarly to private structs with private fields,
or public inherent methods of private structs) but don't make much sense,
they won't be usable anyway due to type privacy.

Visibilities on trait impl items (`impl Tr for Ty { pub fn .... }`) are still
prohibited like before.

# How We Teach This
[how-we-teach-this]: #how-we-teach-this

`pub` should be already familiar from existing teaching materials and
`pub(restricted)` was documented during stabilization as well. Reference should
mention that visibilities can be applied to variants and trait items.

The book or rust-by-example should show that visibilities can be used for
controlling enum exhaustiveness and trait implementability, because
these idioms may not be immediately obvious.

# Drawbacks
[drawbacks]: #drawbacks

None known.

# Alternatives
[alternatives]: #alternatives

Enum exhaustiveness and trait implementability can be controlled by specialized
language additions instead, new attributes and `..` were previously proposed
(see for example
https://github.com/rust-lang/rfcs/pull/2008 or
https://github.com/rust-lang/rfcs/pull/757). These alternatives spend more
"language budget" than reusing visibilities, and are
more coarse-grained (e.g. distinction `pub(crate)` vs `pub(self)` is not
supported), but may be more concize / naturally looking.

Visibilities can also be supported on variant fields, because why not.
```rust
pub enum E {
    V1 { pub(self) a: u8, b: u16 },
    V2(pub(self) u8, b: u16),
}
```

# Unresolved questions
[unresolved]: #unresolved-questions

None known.
