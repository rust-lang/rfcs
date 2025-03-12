- Feature Name: `deny_integer_literal_overflow_lint`
- Start Date: 2018-05-10
- RFC PR: [rust-lang/rfcs#2438](https://github.com/rust-lang/rfcs/pull/2438)
- Rust Issue: [rust-lang/rust#54502](https://github.com/rust-lang/rust/issues/54502)

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

One further note: our intent here is primarily to deny overflowing integer literals, though the `overflowing_literals` lint has one other function: to warn when a floating-point literal exceeds the largest or smallest finite number that is representable by the chosen precision. However, this isn't "overflow" per se, because in this case Rust will assign the value of positive or negative infinity to the variable in question. Because this wouldn't clash with our general stance against implicit overflow, it would not be inconsistent to continue allowing this; however, we adopt the stance that it is both desirable to force someone who wants a value of infinity to explicitly use e.g. `std::f32::INFINITY`, and that it is unlikely that code in the wild would break because of this (and any potential breakage would be precisely noted by the compiler, and could be fixed quickly and trivially). Therefore we are content with the additional strictness that denying this lint would imply.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Integer literals may not exceed the numeric upper or lower bounds of the underlying integral type. For example, for the unsigned eight-bit integer type `u8`, the lowest number it can represent is 0 and the highest number is 255; therefore an assignment such as `let x: u8 = -1;` or `let x: u8 = 256;` will be rejected by the compiler.

Floating-point literals may not exceed the largest or smallest finite number that is precisely representable by the underlying floating-point type, after floating-point rounding is applied. If a floating-point literal is of a sufficient size that it would round to positive or negative infinity, such as `let x: f32 = 3.5e38;`, it will be rejected by the compiler.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Since this feature is already implemented, no implementation guidance is necessary.

To document what is already implemented: any assignment operation that would result in an integral type being assigned a literal value that is outside of that integral type's range will be rejected by the compiler. This encompasses straightforward assignment: `let x: u8 = 256;`; as well as transitive assignment: `let x = 256; let y: u8 = x;`; as well as function calls: `fn foo(x: u8){} foo(256)`. This does not encompass arithmetic operations that would result in arithmetic overflow; `let x: u8 = 255 + 1;` is outside the scope of this analysis. Likewise, this analysis does not attempt to limit the actions of the `as` operator; `let x: i8 = 0xFFu8 as i8;` remains legal.

Similarly, any assignment operation that would result in a floating-point type being assigned a literal value that rounds to positive or negative infinity will be rejected by the compiler.

# Drawbacks
[drawbacks]: #drawbacks

No drawbacks that anyone can think of. Even the risk of breakage is remote, since the lint has existed since 2012 and we can think of no code that would bother relying on deliberately overflowing integer literals. Similarly, we do not anticipate that any code is relying upon overlarge floating-point literals as aliases for `std::f32::INFINITY`.

# Rationale and Alternatives
[alternatives]: #alternatives

The impact of not doing this will be that it is slightly harder to learn and use Rust, and users will be grumpy when they make obvious bugs that the compiler could have prevented but perplexingly chose not to.

An alternative to this proposal would be to deny the ability to write overflowing integer literals while still allowing one to write overlarge floating-point literals. This would involve splitting the `overflowing_literals` lint into two separate lints, one for ints and one for floats, and denying only the former.

Another alternative would be to turn these warnings into hard errors rather than merely denying them; the difference being that in this case nobody would be able to re-enable this behavior. The use case that would suffer from this would be automatic code generation from C programs that make use of C's implicit literal overflow; transition for these users would be easier if this was not a hard error and thus `#![allow(overflowing_literals)]` would suffice.
