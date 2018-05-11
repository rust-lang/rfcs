- Feature Name: deny_integer_literal_overflow_lint
- Start Date: 2018-05-10
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Turn the `overflowing_literals` lint from warn to deny for the 2018 edition.

# Motivation
[motivation]: #motivation

Rust has a strong focus on providing compile-time protection against common programmer errors. In early versions of Rust (circa 2012), integer literals were statically prevented from exceeding the range of their underlying fixed-size integral type. This was enforced syntactically, as at the time all integer literals required a suffix to denote their intended type, e.g. `let x: u8 = 0u8;`, so the parser itself was capable of rejecting e.g. `let x = 256u8;`. Eventually [integer literal type inference](https://mail.mozilla.org/pipermail/rust-dev/2012-July/002002.html) was implemented to improve ergonomics, allowing `let x: u8 = 0;`, but the property that the parser could enforce integer range checking [was lost](https://mail.mozilla.org/pipermail/rust-dev/2012-December/002734.html). It was [re-added](https://github.com/rust-lang/rust/issues/4220) as a warn-by-default lint for the following reasons:

1. Ancient Rust was perpetually uncertain regarding the proper policy towards integer overflow

2. Some vocal users of ancient Rust were insistent that code like `let x: u8 = -1;` should be allowed to work

3. With the aforementioned decision to permit literal underflow, it would be asymmetric to forbid integer overflow

However, since 2012 each of the above reasons has been obviated:

1. Modern Rust considers typical integer overflow and underflow a "program error" (albeit an error with well-defined semantics), thereby taking a stance against implicit wrapping semantics

2. The philosophy of supporting negative literals for unsigned integer literals [was reversed](https://internals.rust-lang.org/t/forbid-unsigned-integer/752) shortly prior to 1.0

3. Now that integer literal underflow is forbidden, the fact that integer literal overflow is allowed is now philosophically asymmetric

Neither I nor anyone else that I have polled can come up with any useful purpose for allowing integer literals to overflow. The only potential objection that has been raised is that we *wouldn't* catch something like `let x: u8 = 255 + 1;`, but that doesn't change the fact that denying integer literals from overflow would prevent strictly more bugs than Rust does today, at no additional cost.

Given that the upcoming 2018 edition allows us to change existing lints to deny-by-default, now is the ideal time to rectify this accident of history.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Integer literals may not exceed the numeric upper or lower bounds of the underlying integral type. For example, for the `u8` type, which is an unsigned 8-bit integer, the lowest number it can represent is 0 and the highest number is 255; therefore an assignment such as `let x: u8 = -1;` or `let x: u8 = 256;` will be rejected by the compiler.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Since this feature is already implemented, no implementation guidance is necessary.

To document what is already implemented: any assignment operation that would result in an integral type being assigned a literal value that is outside of that integral type's range will be rejected by the compiler. This encompasses straightforward assignment: `let x: u8 = 256;`; as well as transitive assignment: `let x = 256; let y: u8 = x;`; as well as function calls: `fn foo(x: u8) {} foo(256)`. This does not encompass arithmetic operations that would result in arithmetic overflow; `let x: u8 = 255 + 1;` is outside the scope of this analysis.

# Drawbacks
[drawbacks]: #drawbacks

No drawbacks that anyone can think of. Even the risk of breakage is remote, since the lint has existed since 2012 and we can think of no code that would bother relying on deliberately overflowing integer literals.

# Rationale and Alternatives
[alternatives]: #alternatives

The impact of not doing this will be that it is slightly harder to learn and use Rust, and users will be grumpy when they make obvious bugs that the compiler could have prevented but bafflingly chose not to.

