- Start Date: 2014-07-04
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Turn our existing conventions for capitalization of identifiers into language
rules for 1.0.

The goal of this proposal is to enshrine existing practice with minimal
disruption, and to break as few programs as possible.


# Motivation

We can remove the restriction again after 1.0, but not add it.

The purpose of our existing conventions is not just consistency of style. They
are also important for aspects of semantic correctness.

Identifiers in pattern matches are interpreted as referring to an existing
`enum` variant or `static` declaration if one is in scope, and as creating a
local binding otherwise. This context-dependent meaning presents a significant
vulnerability to mistakes and accidental breakage. The convention of writing
`enum` variants and `static`s uppercase and local bindings lowercase, if it is
adhered to, eliminates this vulnerability, but it is only a convention.

A lint encouraging use of the conventions, as we have, would be appropriate if
they were only of aesthetic significance, but if they're also important for
ensuring correctness of programs, then it should be more than a lint.

We're already paying the "cost" of reduced identifier freedom with our current
quasi-enforced conventions, but aren't extracting a corresponding benefit from
being able to rely on them as a guarantee.

Being able to distinguish classes of identifiers based on their capitalization
opens up significant room for adding new features to the language in a
backwards-compatible way. To substantiate this claim, below are a few ideas I've
thought of since I began writing this proposal. The worthiness of these ideas is
not important: they're only intended as examples of how significant
capitalization can solve problems and avoid ambiguities.

 * Suppose we want to add functions with named, rather than positional,
   arguments. One idea would be to play off the existing struct vs. tuple
   duality  and, whereas a function call with positional arguments looks like
   `f(1, 2)`, function calls with named arguments would look like
   `f { a: 1, b: 2 }`. Currently this would be impossible to distinguish from
   a struct literal. However, if structs are always uppercase and functions are
   always lowercase, then distinguishing them is possible.

 * Along the same lines, with respect to current language features, it would
   enable *syntactically* distinguishing between function calls and tuple struct
   or enum variant literals. (I don't know where this might be useful.)

 * Rather than a function call with named arguments, we could also use
   `variable_name { a: foo, b: bar }` to denote functional struct update,
   instead of the current more unwieldy
   `StructName { a: foo, b: bar, ..variable_name }`, as seen in some functional
   languages.

 * To close the features gap with structural structs, suppose we want to allow
   accessing the fields of tuples and tuple structs as `.0`, `.1`, `.2`, etc.
   Pretty soon someone will ask: can I use a `static` `uint` declaration instead
   of a literal? If struct fields are always lowercase and `static` declarations
   are always uppercase, this is unambiguous, so the answer may be "yes, why
   not".

 * There was a recent proposal to remove the `'` sigil for lifetimes. If types
   are always uppercase and lifetimes are always lowercase, they are
   syntactically unambiguous even without the sigil.

Again, I'm not suggesting these are good ideas. (I think a couple of them are,
but I won't say which.) The purpose is only to illustrate how significant
capitalization can make our jobs as language designers easier.


# Detailed design

Separate identifiers into two classes: ones which must start with an uppercase
character, and ones which must start with a lowercase character.

Uppercase:

 * Type parameters
 * `struct` declarations
 * `enum` declarations
 * `type` declarations
 * `trait` declarations
 * `static` declarations
 * `enum` variants

Lowercase:

 * `mod` declarations
 * `extern crate` declarations
 * `fn` declarations
 * `macro_rules!` declarations
 * `struct` and structural `enum` variant fields
 * Local bindings: `fn` and closure arguments, `let`, `match`, and `for`..`in`
   bindings
 * Lifetime parameters
 * Lifetimes, including loop labels
 * Attributes

## Grandfather clause

To avoid disruption to existing code, the names of the built-in types should
remain unchanged and remain legal. In exchange, to maintain the full separation
between type identifiers and lowercase-class identifiers, the names of the
built-in types should be disallowed as lowercase-class identifiers.

There are multiple ways to accomplish this:

 1. Make a special exception such that the names of the built-in types are legal
    as type names, but not as lowercase-class identifiers. (So, in effect, they
    are legal only as type names.)

 2. Declare that the names of the built-in types are considered to be upperclass
    identifiers. Thus, they are legal as identifiers for anything in the
    uppercase-class, not just types, and not for anything in the lowercase-class.

 3. Turn them into keywords.

My preference is for 1 or 3.

As with the rule itself, this restriction could be loosened in a
backwards-compatible way in the future.

The names of the built-in types are:

 * `int`, `uint`
 * `i8`..`i64`
 * `u8`..`u64`
 * `f32`, `f64`
 * `bool`
 * `char`
 * `str`


# Drawbacks

The most common objection I've encountered to this idea is that we may want to
allow non-ascii identifiers in the future, and that not all alphabets have a
distinction between uppercase and lowercase characters.

I have three counterarguments, of which I consider either the first by itself or
the combination of the second and third to be sufficient, so that altogether,
they should be sufficient to counter it twice.

 1. If we impose the restriction for 1.0, and later decide that we want to allow
    non-ascii identifiers, we can then remove the restriction. We can't do it
    the other way around.

 2. We can figure out ways to accomodate such alphabets while maintaining the
    distinction. For instance, we could say that characters from alphabets
    without an uppercase-lowercase distinction are always considered to be
    uppercase, and `_` is lowercase. Therefore lowercase-class identifiers
    written in such alphabets should start with `_`. We also have a few unused
    sigils now; we can get creative.

 3. As mentioned in the Motivation, we *already* rely on capitalization for
    aspects of program correctness, namely dealing with the `static` and `enum`
    variants versus pattern bindings issue. Under these conditions, even if we
    enable support for non-ascii identifiers, it's plausible that style guides
    and coding conventions would disallow the use of
    neither-uppercase-nor-lowercase identifiers, in order to prevent bugs. In
    other words, writing identifiers in certain foreign alphabets would be
    legal, but *technically inferior*. I don't think this would be a positive
    situation.


# Alternatives

The status quo, obviously. Refer to *Motivation*.

We could regulate the capitalization of not only the first character, but the
rest of the identifier as well, i.e. `ELEPHANT_CASE` and `CamelCase` and
`snake_case`. However, this would not provide any additional guarantees unless
we also impose further restrictions, such as requiring `ELEPHANT_CASE`
identifiers to be at least two characters long, while banning underscores and
consecutive uppercase characters in `CamelCase` identifiers. Either way, this
doesn't seem like it would pass the cost-benefit test.


# Unresolved questions

The precise formulation of the grandfather clause.
