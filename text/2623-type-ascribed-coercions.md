- Feature Name: Type Ascribed Coercions
- Start Date: 2019-01-06
- RFC PR: [rust-lang/rfcs#2623](https://github.com/rust-lang/rfcs/pull/2623)
- Rust Issue: _

[RFC 803]: https://github.com/rust-lang/rfcs/blob/master/text/0803-type-ascription.md
[RCF 2522]: https://github.com/rust-lang/rfcs/pull/2522

# Summary
[summary]: #summary

The
[Rust reference defines *coercion sites*](https://doc.rust-lang.org/reference/type-coercions.html#coercion-sites)
which are contexts in which a coercion can occur.
For consistency, we change the specification as of [RFC 803] such that a type
ascribed expression that needs to be coerced can only occur at these coercion
sites.

# Motivation
[motivation]: #motivation

The
[subsection "Type ascription and temporaries"](https://github.com/rust-lang/rfcs/blob/master/text/0803-type-ascription.md#type-ascription-and-temporaries)
of the merged [RFC 803] defines certain contexts (so-called *reference
contexts*) in which a type ascription that needs coercion **can not occur**.
Meanwhile and in contrast, the
[Rust reference defines *coercion sites*](https://doc.rust-lang.org/reference/type-coercions.html#coercion-sites)
which are contexts in which a coercion **can occur**.

By applying the same rule to type ascribed expressions, we aim to reduce language complexity and increase consistency.

This change shouldn't in any way conflict with [RCF 2522]

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Coercions are implicit type conversions and they already in the language.
You can read more about them
[here](https://doc.rust-lang.org/nomicon/coercions.html).

The already merged (but not yet stabilized) [RFC 803] added type ascription for
expressions.
You should read it first.
It proposed that a type ascribed lvalue is still an lvalue and a type ascribed
rvalue is still an rvalue.
[An lvalues is a value that lives in a memory location.](https://stackoverflow.com/a/42313956/7350842)
The author of [RFC 803] correctly identified a problem that could arise with
the following code:

```
let mut foo: S = ...;
{
    let bar = &mut (foo: T);  // S <: T, coercion from S to T
    *bar = ... : T;
}
// Whoops, foo has type T, but the compiler thinks it has type S, where potentially T </: S
```

`S <: T` means that `S` is a subtype of `T`.
[`S` can therefore be coerced to `T`.](https://doc.rust-lang.org/reference/type-coercions.html#coercion-types)
The problem is that `foo` suddenly isn't bound to an instance of `S` anymore,
even though it's still a variable of type `S`.

There are several solutions to this problem:

* Always treat type ascribed expressions as rvalues.
  It was explained in [RFC 803] why this solution isn't desirable.
* The solution proposed in [RFC 803]:
  Disallow coercions in type ascribed expressions if they occur in so-called
  *reference contexts*.
* The solution proposed in this RFC:
  Only allow coercions at
  [*coercion sites* as defined by the reference](https://doc.rust-lang.org/reference/type-coercions.html#coercion-sites).
  This means that a coercion in a type ascribed expression is only possible
  if the type ascribed expression occurs at a location where a coercion
  would be possible anyway.
  The above example wouldn't compile, because `&mut e` is not a coercion site.

More examples of what's a coercion site (i. e. a context in which an expression
can be subject to coercion) can be found on
[the corresponding page of the nomicon](https://doc.rust-lang.org/nomicon/coercions.html).
The same examples will still compile if the resulting type of the coercion is
ascribed.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

We drop the following rule introduced in [RFC 803]:

> If the type ascription expression is
in reference context, then we require the ascribed type to exactly match the
type of the expression, i.e., neither subtyping nor coercion is allowed.

Instead, we use the
[Rust reference's definition of a *coercion site*](https://doc.rust-lang.org/reference/type-coercions.html#coercion-sites) and extend it by the new rule:
A type ascription expression `e : T` is a coercion site if and only if it
occurs at a coercion site itself.

If `e` is of type `S` and the expression `e : T` occurs at a coercion site that
expects the type `U`, then it shall be required that `S` is coercible to `T`
and `T` is coercible to `U`.
In that case, the semantics are that of coercing `S` to `U`, which is possible
because of the transitivity.

Remarks:

  * `e` in `e : T` can of course only be subject to coercion if `e : T` is a coercion site by that rule.
  * With `S <: T`, coercing `S` to `T` is also
  [considered a coercion](https://doc.rust-lang.org/reference/type-coercions.html#coercion-types).

With that rule, type ascription is still idempotent since applying the same
type ascription twice doesn't only preserve the type but also whether the
ascribed expression is considered to occur at a coercion site.

This change doesn't introduce any new semantics since *coercion sites* and
[RFC 803]'s *reference contexts* are disjoint.

# Drawbacks
[drawbacks]: #drawbacks

There's possibly code that would compile with [RFC 803] and that doesn't
compile with this change.
Such code would however rely on a coercion in a context that wasn't supposed to
introduce coercions.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Type ascription wasn't intended to introduce sites for conversions, but to help
the compiler infer the correct type.
Therefore type ascription isn't the correct feature in case a user wants a
conversion that wouldn't already happen without type ascription.

With this design, coercions can only occur where they're already possible
without type ascription, which has the additional benefit that it's easy to see
that no new unsoundness is introduced.

Having coercion consistent across language features also minimizes the increase
of language complexity, which is good for both, the implementors and the users.
For example, there's no need to define *reference contexts* anymore.

Coercion in type ascribed expressions is
[not yet implemented](https://play.rust-lang.org/?version=nightly&mode=debug&edition=2018&gist=825110d9744d502cf30544e3c86ed37c).
This change should make its implementation easier.

The only alternative that I currently see is to disallow coercions in all type
ascribed expressions, such that the ascribed type always has to match the type
of the expression exactly.
This would however possibly be cumbersome and break the invariant that those
two lines are interchangable:

```rust
let _ : T = e;
let _     = e : T:
```
