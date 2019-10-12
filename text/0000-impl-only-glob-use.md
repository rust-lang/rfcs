- Feature Name: impl-only-glob-use
- Start Date: 2019-10-12
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Allow `use path::* as _` as a way to import implementations of all traits from `path`.

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

Like `use module::Trait as _` can be used to import an individual trait's implementations without bringing it into scope, `use module::* as _` can be used to import trait implementations for all public traits from `module` into scope.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

`use module::*` has to do the same thing `use module::Trait` does for every trait that `module` publicly defines or re-exports.

# Drawbacks
[drawbacks]: #drawbacks

The `* as _` syntax might be too strange and thus off-putting for some users, especially those new to the language. Just like with `use Thing as _`, it is not immediately obvious what it does. Additionally, it might be harder to guess or recall its functionality when reading code as it contains no hint that it is about trait implementations – in contrast to `use Thing as _`, where this could be more easily deduced when knowing `Thing` is a trait.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The proposed syntax is simply a combination of glob import's and `impl`-only import's syntax.

An alternative syntax for this feature would be `use module::trait * as _`, which makes its functionality more obvious in exchange for being a larger deviation from existing syntax.

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

A potential future import syntax might simplify trait implementation imports yet again by recursively importing them from a given crate or module. Following the globbing syntax used in other places, this syntax could be `use cratename::** as _`.
