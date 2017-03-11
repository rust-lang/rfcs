- Feature Name: `inferred_variant_constructor`
- Start Date: 2017-03-11
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Allow the name (qualifier) of an enum variant to be elided in expressions and
patterns whenever it can be inferred.

# Motivation
[motivation]: #motivation

The main goal of this proposal is to improve ergonomics of enums in Rust by
reducing verbosity.  Currently, there are two convenient ways to use enums.
One would either:

  - bring the enum type into scope (`use foolib::Foo;`), or
  - bring the enum variants into scope (`use foolib::Foo::*;`).

The former is less convenient than the latter, but the latter pollutes the
namespace more.  One could always write enum variants fully qualified,
`foolib::Foo::Alpha`, but that is even more tedious.

This proposal intends to make variant expressions as convenient and
pollution-free as field expressions today.  Whereas field expressions allow
the user to extract fields of a struct without ever bringing the struct into
scope, inferred variant constructors allow the user to construct/match
variants of an enum without ever bringing the enum or its variants into scope.

This could lower the usability barrier for enums, particularly for library
users.  It would reduce the verbosity of enums to a level similar to
dynamically typed languages, where strings (e.g. Python) or symbols
(e.g. Lisps, Ruby) designate ad hoc variants

The feature could encourage library writers to use domain-specific enums more
often, rather than to re-purpose generic enums like `Option` or `Result`,
leading to more readable code.  It would also allow library writers to use
longer names for enums without significantly compromising their usability.

The feature can also provide a slight benefit for refactoring: if the name of
an enum is changed, code that uses inferred variant constructors would not
need to be renamed.

# Detailed design
[design]: #detailed-design

Consider this enum:

~~~rust
mod foolib {
    enum Foo<T, U, V> {
        Alpha(T),
        Beta { gamma: U, delta: V },
    }
}
~~~

In today's Rust, there are only two ways to write a specific variant of an
enum.  Either qualified,

~~~rust
use foolib::Foo;
Foo::Alpha(t)
Foo::Beta { gamma: u, delta: v }
~~~

or as unqualified,

~~~rust
use foolib::Foo::*;
Alpha(t)
Beta { gamma: u, delta: v }
~~~

The proposal intends to add a third intermediate form:

~~~rust
_::Alpha(t)
_::Beta { gamma: u, delta: v }
~~~

They shall be tentatively called **inferred variant constructors**.  The
two-token prefix `_::` serves to disambiguate inferred variant constructors
from ordinary variables or functions.

In an expression, an inferred variant constructor is permissible only in
contexts where the receiving type of the expression is inferred to be a known
visible enum, irrespective of whether the type parameters of the enum are
known.

Dually, in a pattern, an inferred variant constructor is permissible only in
contexts where the type of the scrutinee is inferred to be a known visible
enum.

If the inferred type is a private enum from another module, or a struct, or is
unknown, then the program is rejected with a type error.  If the enum type is
known, but the specified variant does not exist within that enum or the
argument list does not match that variant, then the program is also rejected.

Inferred variant constructors do not take the namespace into consideration at
all, nor do they affect the namespace.  It is purely type-directed, albeit
subject to visibility rules.

There should be no change in the behavior of existing programs, as the feature
is entirely opt-in.

## Example

~~~rust
fn use_foo<V>(foo: Foo<&str, bool, V>) { … }

fn main() {
    use_foo(_::Alpha("hi"));
    use_foo(_::Beta { gamma: true, delta: '~' });
}
~~~

In this example, the type of `use_foo` is known, so the receiving type of the
argument would be inferred as `Foo<_, _, _>`.  In both cases, they are
unambiguously inferred as variants of `Foo`.

# How We Teach This
[how-we-teach-this]: #how-we-teach-this

Potential names for the proposed syntax in order of increasing verbosity:

  - Inferred variant constructors
  - Semi-qualified variant constructors
  - Context-dependent variant constructors
  - Variant constructors with placeholders
  - Variant constructors with elided enum names
  - Variant constructors that simply do the Obvious Thing™

As noted earlier, this is a bit like the dual of field expressions, so in some
sense it is a natural development to bring the ergonomics of enums on par with
that of structs, if not a bit *more* since currently struct names cannot be
elided in the record syntax (an idea for another future proposal).

The acceptance of this proposal should not negatively impact new users
significantly, as the feature is quite straightforward to explain.  It would
result in the reduction of boilerplate such as `use somelib::SomeEnum` in
examples and tutorials, which would appeal users coming from less verbose
dynamic languages such as Python.  It is possible that the `_::` may confuse
newcomers, but hopefully the Syntax Index can help here.

The Rust Reference would need to be amended to explain the syntax as well as
the mechanism.  The feature itself is small enough that it would not require a
dedicated section in any of the books.  A brief mention in passing along with
an example would suffice.  A note for `_::` in the Syntax Index would also be
useful.

# Drawbacks
[drawbacks]: #drawbacks

This would make the type inference implementation more complex, and thus the
compiler as well.  (But it would not negatively affect language users who do
not use the feature.)

The feature would occupy the `_::` syntax, which could be used for,
e.g. inferred static methods of a trait or struct.

This might draw attention away from [more ambitious features][294].  (But it
would not conflict them.)

It is possible that over-use of this feature may lead to less readable code in
some contexts.

# Alternatives
[alternatives]: #alternatives

The two-token prefix `_::` is not a particularly crucial part of the proposal
and could be substituted for anything, but it does appear to be one of the
most natural syntaxes for it given the current use of `_` as a placeholder for
type inference.  It is somewhat unsightly and long, but at least it can be
typed without lifting the shift key on standard QWERTY layouts.

It is possible to only admit either the expression or the pattern subset of
this feature, but so far there is no compelling reason for why this would be
necessary or desirable.

A [similar idea][4935] was proposed by *hgrecco*, which is very similar in
semantics but does not include any syntactic sigil to disambiguate from
regular identifiers.

[4935]: https://internals.rust-lang.org/t/elliding-type-in-matching-an-enum/4935

Although the motivations are similar to those of [anonymous sum types][294],
this proposal aims to build on top of what already exists in Rust rather than
to add something new.  It brings in some of the benefits of anonymous sum
types, but does not intend to subsume anonymous sum types entirely.

[294]: https://github.com/rust-lang/rfcs/issues/294

The impact of not doing this is that enums would remain a bit less convenient
to use in comparison to structs.  This would discourage the use of more
readable enums, leading to the use of generic enums more frequently.

# Unresolved questions
[unresolved]: #unresolved-questions

  - Would the syntax introduce major complications to the parsing?
  - To what extent would it affect the existing type inference algorithm?
