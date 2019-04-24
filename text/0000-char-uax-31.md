- Feature Name: `char_uax_31`
- Start Date: 2019-24-04
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add functionc to the standard library for testing a `char` against [UAX TR31](https://unicode.org/reports/tr31/) ("Unicode Annex 31")
`Pattern_White_Space`, `Pattern_Syntax`, `XID_Start`, `ID_Nonstart`, and `XID_Continue` (the XID ones are already in the standard
library, but are unstable; this RFC proposes to stablize them).

# Motivation
[motivation]: #motivation

As a systems language, Rust is heavily used for parsing.
As a progressive, forward-thinking language that accepts anyone,
Rust supports Unicode and makes the definitive string types UTF-8.
At the intersection of these needs sits *UAX #31: Unicode Identifier and Pattern Syntax* ("Annex 31"),
a standardized set of code point categories for defining computer language syntax.

This is being used in production Rust code already.
Rust's own compiler already has functions to check against Annex 31 code point categories in the lexer,
[but not everyone who works on the compiler knows about them](https://internals.rust-lang.org/t/for-await-loops/9819/16),
and since they're not in the standard library,
not everyone who works on Rust-related tooling has access to them.
I'm not asserting that putting these in libstd would've avoided that bug,
but if it was in the standard library,
it would resolve the questions about whether third-party tooling can be expected to support the full range of Unicode whitespace.

[Other languages](https://rosettacode.org/wiki/Unicode_variable_names#C) also follow Annex 31, such as C# and Elixir.
Other common grammars, even ones that aren't actually for programming languages, can also be found or defined in Annex 31,
such as hashtags and XML.

It's also pretty clear what the "right" API is for this,
since `is_whitespace` and `is_ascii_whitespace` already set the precedent here,
so there's little need to experiment with API design.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

In addition to functions for checking "ASCII white space" and "Unicode white space,"
some languages, such as Rust and C#, use Unicode Annex 31 to define their syntax.
These functions are also exposed as methods on the `char` type.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## `fn char::is_id_nonstart(self) -> bool`

Check if `self` is a member of Unicode Annex 31's `ID_Nonstart` code point category.
This function is defined as `self.is_xid_continue() && !self.is_xid_start()`.

## `fn char::is_pattern_syntax(self) -> bool`

Check if `self` is a member of Unicode Annex 31's `Pattern_Syntax` code point category.

## `fn char::is_pattern_white_space(self) -> bool`

Check if `self` is a member of Unicode Annex 31's `Pattern_White_Space` code point category.

# Drawbacks
[drawbacks]: #drawbacks

The big problem, that has always made designing the text APIs hard,
is that it's not clear how much of Unicode we want to include in libstd.
The standard library certainly doesn't want a hashtag parser, even though Annex 31 describes one in section 6,
and libstd certainly doesn't want a character shaping algorithm,
even though Unicode places plenty of requirements on that process, too.

The other problem is that a lot of languages aren't defined in terms of Annex 31 anyway,
like Swift and HTML, which simply spell out the set of allowed code points themselves,
so this isn't necessarily useful to allw of the language implementers.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The design was chosen to line up with how character classification is already being done (like `is_whitespace`).
The alternative, of providing a more generic classification API,
seems to have enough room for debate that it would be better served in crates that provide purpose-built frameworks.
In particular, proposal is made for the benefit of parsers, not text layout engines.
Those will still need to use things like `rust-unic`.

# Prior art
[prior-art]: #prior-art

There's already a crate that mostly provides this API, [unicode-xid](https://lib.rs/crates/unicode-xid),
but it's actually less comprehensive than this proposal (it only provides XID_Start and XID_Continue).

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- What about ID_Start and ID_Continue? They're deprecated by the Unicode Consortium, but probably still useful for parsing some languages.
- `is_pattern_white_space`, like UAX 31 spells it? Or `is_pattern_whitespace`, for consistency with the rest of libstd?

# Future possibilities
[future-possibilities]: #future-possibilities

What does [Mosh](https://mosh.org/) use need to know for its UTF-8 handling?
Anything that's necessary to implement a correct UTF-8 enabled VT100 state machine seems applicable to Rust,
since that state machine is separate from the text shaping itself, but still has to know things like combining marks,
and what's necessary there is probably necessary for other, similar state machines like HTML and PDF,
where you have to pick out weird combining-mark corner cases.
