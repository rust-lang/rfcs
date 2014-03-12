# Summary

Rust currently has an attribute usage lint but it does not work particularly
well. This RFC proposes a new implementation strategy that should make it
significantly more useful.

# Motivation

The current implementation has two major issues:

1. There are very limited warnings for valid attributes that end up in the
wrong place. Something like this will be silently ignored:
```rust
#[deriving(Clone)]; // Shouldn't have put a ; here
struct Foo;

#[ignore(attribute-usage)] // Should have used #[allow(attribute-usage)] instead!
mod bar {
    //...
}
```

2. `ItemDecorators` can now be defined outside of the compiler, and there's no
way to tag them and associated attributes as valid. Something like this
requires an `#[allow(attribute-usage)]`:
```rust
#[feature(phase)];
#[phase(syntax, link)]
extern crate some_orm;

#[ormify]
pub struct Foo {
    #[column(foo_)]
    #[primary_key]
    foo: int
}
```

# Detailed design

The current implementation is implemented as a simple fold over the AST,
comparing attributes against a whitelist. Crate-level attributes use a separate
whitelist, but no other distinctions are made.

This RFC would change the implementation to actually track which attributes are
used during the compilation process. `syntax::ast::Attribute_` would be
modified to add a new `used` field:
```rust
pub struct Attribute_ {
    style: AttrStyle,
    value: @MetaItem,
    is_sugared_doc: bool,
    used: bool,
}
```

It will be initialized to `false`, and code reading an attribute should set it
to `true`. The utility methods in `ast::attr` can be modified to automatically
set it to handle the simple cases automatically.

The `attribute-usage` lint would run at the end of compilation and warn on all
attributes whose `used` field hasn't been set.

One interesting edge case is attributes like `doc` that are used, but not in
the normal compilation process. There could either be a separate fold pass to
mark all `doc` attributes as used or `doc` could simply be whitelisted in the
`attribute-usage` lint.

Attributes in code that has been eliminated with `#[cfg()]` will not be linted,
but I feel that this is consistent with the way `#[cfg()]` works in general
(e.g. the code won't be type-checked either).

# Alternatives

An alternative would be to rewrite `rustc::middle::lint` to robustly check
that attributes are used where they're supposed to be. This will be fairly
complex and be prone to failure if/when more nodes are added to the AST. This
also doesn't solve motivation #2, which would require externally loaded lint
support.

# Unresolved questions

+ This implementation doesn't allow for a distinction between "unused" and
"unknown" attributes. The `#[phase(syntax)]` crate loading infrastructure could
be extended to pull a list of attributes from crates to use in the lint pass,
but I'm not sure if the extra complexity is worth it.

