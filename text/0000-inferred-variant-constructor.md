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

Currently, there are two convenient ways to use enums in Rust.  One would
either:

  - (a) bring the enum type into scope (`use foolib::Foo;` or
    `use foolib::Foo as F;`), or
  - (b) bring the enum variants into scope (`use foolib::Foo::*;`).

The former is less convenient than the latter, but the latter pollutes the
namespace more.  One could always write enum variants fully qualified,
`foolib::Foo::Alpha`, but that is even more tedious.

The main goal of this proposal is to improve ergonomics of enums in Rust by
providing an intermediate syntax that:

  - has less boilerplate than either (a) or (b),
  - is less verbose than (a) or equally so if `use foolib::Foo as F;` was
    used, and
  - avoids the polluting the namespace, in contrast to (a) or (b).

The semantics of the feature are inspired by those of
[field expressions][field] (`some_struct.some_field` or
`some_struct.some_method(…)`) and could be considered their [dual][dual] in
some sense.  Its benefits are therefore analogous to field expressions:
whereas field expressions allow the user to extract fields of a struct without
ever bringing the struct into scope, inferred variant constructors allow the
user to construct/match variants of an enum without ever bringing the enum or
its variants into scope.

[field]: https://doc.rust-lang.org/reference.html#field-expressions
[dual]: https://en.wikipedia.org/wiki/Dual_(category_theory)

The proposed feature avoids the `use` boilerplate while also avoiding the
namespace pollution caused by bringing enums or their variants into scope.
This lowers the the usability barrier for enums, particularly for library
users.  It would allow enums to serve a role similar to strings (e.g. Python)
or symbols (e.g. Lisps, Ruby) in dynamically typed languages where they
designate ad hoc variants.

This could encourage library writers to use domain-specific enums more often,
rather than to re-purpose generic enums like `Option` or `Result`.  It would
also allow library writers to use longer names for enums without significantly
compromising their usability.

The feature is most beneficial for situations where a large number of enums is
being used but the number of uses of each enum type is small, which tend to
occur when using multiple disparate libraries or libraries with a large number
of features.

The proposed feature can replace certain uses of the “builder pattern”:

~~~rust
Pizza::new().thin_crust().pineapple().pepperoni()
~~~

This could be replaced with the more readable:

~~~rust
Pizza { crust: _::Thin, toppings: &[_::Pineapple, _::Pepperoni] }
~~~

The record syntax makes the relationship between each field self-evident,
whereas from the builder pattern it may be unclear whether `pineapple` and
`pepperoni` are mutually exclusive without reading the documentation of each
method.  Moreover, builder patterns require a significant amount of
boilerplate from the library author, therefore it would ideal to avoid it
where possible.

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

  - In an expression, an inferred variant constructor is permissible only in
    contexts where the receiving type of the expression is inferred to be a
    known visible enum, irrespective of whether the type parameters of the
    enum are known.

  - Dually, in a pattern, an inferred variant constructor is permissible only
    in contexts where the type of the scrutinee is inferred to be a known
    visible enum.

If the inferred type is a private enum from another module, or a struct, or is
an abstract type parameter, then the program is rejected with a type error.
If the enum type is known, but the specified variant does not exist within
that enum or the argument list does not match that variant, then the program
is also rejected.

Inferred variant constructors do not consider the availability of the enum or
its variants within the current scope.  They also do not introduce any
identifiers into the local scope.  The inference is purely type-directed,
albeit subject to privacy rules as usual.

There should be no change in the behavior of existing programs, as the feature
is entirely opt-in.

## Example

~~~rust
mod foolib {
    pub enum Foo<T, U, V> {
        Alpha(T),
        Beta { gamma: U, delta: V },
    }
    pub struct Bar {
        foo: Foo<&'static str, bool, char>,
    }
    pub fn use_foo<V>(foo: Foo<&str, bool, V>) { … }
    pub fn get_foo<V: Default>() -> Foo<&str, bool, V>) { … }
}

fn get_alpha<T>() -> T {
    // return type is T
    // ⇒ t must be T
    // ⇒ _::Alpha must be a variant of T (?)
    // ✘ Error: insufficient information to infer enum type
    let t = _::Alpha(42);
    t
}

fn get_beta<T>() -> Foo<T, bool, char> {
    // return type is Foo<T, bool, char>
    // ⇒ _::Beta must be a variant of Foo
    // ⇒ arguments of _::Beta must be { gamma: bool, delta: char }
    // ✓ Type checks
    _::Beta { gamma: true, delta: '~' }
}

fn main() {
    use foolib::use_foo;

    // definition of use_foo
    // ⇒ argument must be Foo<&'_ str, bool, _>
    // ⇒ _::Alpha must be a variant of Foo
    // ⇒ argument of _::Alpha must be &'_ str
    // ✓ Type checks
    use_foo(_::Alpha("hi"));

    // definition of use_foo
    // ⇒ argument must be Foo<&'_ str, bool, _>
    // ⇒ b must be Foo<&'_ str, bool, _>
    // ⇒ _::Beta must be a variant of Foo
    // ⇒ arguments of _::Beta must be { gamma: bool, delta: _ }
    // ✓ Type checks
    let b = _::Beta { gamma: true, delta: '~' };
    use_foo(b);

    // definition of Bar
    // ⇒ foo field must be Foo<&'static str, bool, char>
    // ⇒ _::Alpha must be a variant of Foo
    // ⇒ argument of _::Alpha must be &'static str
    // ✓ Type checks
    Bar { foo: _::Alpha("hi") };

    // definition of use_foo
    // ⇒ argument must be Foo<&'_ str, bool, _>
    // ⇒ _::Epsilon must be a variant of Foo (✘)
    // ✘ Error: Epsilon is not a variant of Foo
    use_foo(_::Epsilon(42));

    // return type of a discarded statement is _
    // ⇒ _::Alpha must be a variant of _ (?)
    // ✘ Error: insufficient information to infer enum type
    _::Alpha("hi");

    // definition of println!'s internal functions
    // ⇒ argument must be _: Debug
    // ⇒ _::Alpha must be a variant of _ (?)
    // ✘ Error: insufficient information to infer enum type
    println!("{:?}", _::Alpha("hi"));

    // definition of get_foo
    // ⇒ return type must be Foo<&'_ str, bool, _>
    // ⇒ _::Alpha and _::Beta must be all the variants of Foo
    // ✓ Type checks
    let mut s = String::new();
    match get_foo() {
        _::Alpha(_) => {}
        _::Beta(_, v) => { s.push(v); }
    }

    // definition of get_foo
    // ⇒ return type must be Foo<&'_ str, bool, _>
    // ⇒ _::Alpha and _::Beta must be all the variants of Foo (✘)
    // ✘ Error: Epsilon is not a variant of Foo
    // ✘ Error: match is missing Beta case
    match get_foo() {
        _::Alpha(_) => {}
        _::Epsilon(_) => {}
    }
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

*Like field expressions, this new feature would afflict enums with the same
problems that structs have today.*

It is possible that over-use of this feature could cause readability problems.
In some exceptional situations, the type could be inferred mechanically but
for a user it might require reading the documentation at several disparate
locations to manually infer the enum type.

This would increase the difficulty of grepping all uses of a particular enum
without using a tool that understands Rust code, much like how grepping for
all uses of a struct is nontrivial.  (On the other hand, if the name of an
enum is changed, code that uses inferred variant constructors would not need
to be renamed.)

This would make the type inference implementation more complex, and thus the
compiler as well.  It is not clear how much of an impact this would cause.
(But it would not negatively affect language users who do not use the
feature.)

*There are also some additional issues specific to this feature.*

Excessive use of domain-specific enums can add to the boilerplate when
multiple enums are required to interoperate, as domain-specific enums will
likely have very few helper methods simplify their use.

The feature would occupy the `_::` syntax, which could be used for,
e.g. inferred static methods of a trait or struct.

This might draw attention away from [more ambitious features][294].  (But it
would not conflict them.)

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

There are existing ways to work around the problem.  One could use short
abbreviations (`use foolib::Foo as F;`), or construct preludes (`use
foolib::prelude::*;`) at the cost of polluting the user's namespace, or use
builder patterns as noted earlier.

The impact of not doing this is that enums would lack their analog of field
expressions for structs.  Therefore, enums would remain slightly less
convenient to use in comparison to structs, where users can often use its
fields without ever importing the struct.  This would discourage the use of
more readable enums in APIs, leading to the use of generic enums or builder
patterns more frequently.

# Unresolved questions
[unresolved]: #unresolved-questions

  - Would the syntax introduce major complications to the parsing?
  - To what extent would it affect the existing type inference algorithm?
