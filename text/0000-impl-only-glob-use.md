- Feature Name: impl-only-glob-use
- Start Date: 2019-10-12
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Allow `use path::trait * as _` as a way to import implementations of all traits from `path`.

# Motivation
[motivation]: #motivation

There are a number of crates that export `prelude` modules, meant to be used via `use cratename::prelude::*`.
Sometimes, these glob imports are only used to bring trait methods into scope.
Some well-known crates with which this pattern is often used are `rayon` and `gtk`.
(see [rayon::prelude], [gtk::prelude])

[rayon::prelude]: https://docs.rs/rayon/1.2.0/rayon/prelude/index.html
[gtk::prelude]: https://docs.rs/gtk/0.7.1/gtk/prelude/index.html

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Like `use module::Tr as _` can be used to import an individual trait's implementations without bringing it into scope, `use module::trait * as _` can be used to import trait implementations for all public traits from `module` into scope.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

`use module::trait * as _` is equivalent to one `use module::Tr as _` statement for each trait `Tr` that `module` publicly defines or re-exports.

# Drawbacks
[drawbacks]: #drawbacks

Just like `use module::Tr as _`, it is not immediately obvious what `use module::trait * as _` does. It is likely more confusing for newcomers to read than `use module::*`.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The most obvious alternative syntax-wise would be `use module::* as _`, a combination of glob imports and `impl`-only trait imports. This was also the syntax proposed in the initial version of this RFC; however it was deemed too obscure by many.

The impact of not implementing this RFC would be very low: A small amount of people will get symbol name clashes that could have been avoided with this feature, and another small amount of people will discover that the syntax described in it is not supported.

# Prior art
[prior-art]: #prior-art

The possibility of this feature was previously mentioned by @nikomatsakis in [a comment on the tracking issue for RFC 2166][comment] (which is the RFC that introduced the `use Trait as _` syntax).

The author of this RFC is not aware of comparable syntax and / or import semantics in other programming languages.

[comment]: https://github.com/rust-lang/rust/issues/48216#issuecomment-372642913

# Unresolved questions
[unresolved-questions]: #unresolved-questions

# Future possibilities
[future-possibilities]: #future-possibilities

A potential future import syntax might simplify trait implementation imports yet again by recursively importing them from a given crate or module. The syntax for this could for example be `use cratename::trait ** as _`.
