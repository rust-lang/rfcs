- Feature Name: simplify_visibility_grammer
- Start Date: 2019-02-20
- RFC PR: 
- Rust Issue: 

# Summary
[summary]: #summary

At the time of writing, the grammar for a visibility specifier is:
```
Visibility:
      pub
    | pub(crate)
    | pub(self)
    | pub(super)
    | pub(in SimplePath)
```

This RFC suggests changing it to:
```
Visibility:
      pub
    | pub(SimplePath)
    | pub(in SimplePath)
```

In a future edition, this RFC supports changing it to:
```
Visibility:
      pub
    | pub(SimplePath)
```

This does not break any code, as `SimplePath` already parses `self`, `crate`, and `super`. `pub(in SimplePath)` must be retained, at least until another edition, for backwards compatability.

# Motivation
[motivation]: #motivation

This change removes the distinction between the three keywords and paths. This is simpler to understand, as the visibility specifier can now always be thought of as the path to the scope in which something is
visible. The current grammar does provide this ability, but has the potential to trip users up because of the differences between what is accepted within a visibility specifier without `in`, and what is a
valid `SimplePath`.

Users that have only ever worked with the current path system are taught that `crate` is a valid path, and is also accepted as a visibility specifier. However, to make this path more
specific, they then need to write `pub(in crate::foo::bar)`, which creates friction and exposes new users to remnants of the previous path system for no reason.

In summary:
* This version is easier to teach
* This version has a simpler grammer
* This version requires less logic to parse

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

With this change, users could write `pub(crate::foo::bar)`, which would be equivalent to `pub(in crate::foo::bar)`. The latter version would continue to work for now.
`pub(crate)`, `pub(self)`, and `pub(super)` would continue to work as they do now, because they are valid `SimplePath`s that resolve to the same scopes as are currently hard-coded.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This change requires very contained changes to the compiler. Specifically, [libsyntax::parse::parser::parse_visibility](https://github.com/rust-lang/rust/blob/74e35d270067afff72034312065c48e6d8cfba67/src/libsyntax/parse/parser.rs#L6957-L7015) will need to be changed to accept the new syntax, as well as providing updating diagnostics about valid visibility specifiers.

Specifically, we need to retain the logic for parsing `pub ()`, so it should first look ahead for the closing parenthesis. If found and `pub ()` is valid in that location, `parse_visibility` should return
`VisibilityKind::Public`. If `pub ()` is invalid, a diagnostic should be emitted specifying that a path should be supplied.

Next the logic for explicitly parsing `crate`, `self`, and `super` can be removed. Instead, `parse_visibility` should just attempt to parse a `SimplePath`. All paths would result in a `VisibilityKind::Restricted`. The only time this result would differ from the current implementation is in the case of `pub(crate)`, which changes from `VisibilityKind::Crate(CrateSugar::PubCrate)` to `VisibilityKind::Restricted`. I
am unsure if this would affect anything in the language, with the implementation as it currently is.

The current logic for parsing `in SimplePath` can be kept unchanged. In a future edition, support for this syntax could be removed, as it would become redundant with this change.

The [relevant part of the reference](https://doc.rust-lang.org/reference/visibility-and-privacy.html) should also be changed to reflect the new grammar.

# Drawbacks
[drawbacks]: #drawbacks

I do not see any drawbacks to this change.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The alternative is to do nothing, and keep the current visibility grammar. This would not affect the language greatly, as this change does not allow users to express anything they could not before. However,
the proposed grammer aligns with what users expect, given other syntax concerning paths (e.g. `crate` and `crate::foo::bar` are not treated differently in `use` statements, but are here), and so I believe
this change is worth making.

# Prior art
[prior-art]: #prior-art
