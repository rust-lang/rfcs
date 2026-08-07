- Feature Name: fragment-specifiers-for-generic-arguments
- Start Date: 2023-05-31
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Right now, there is no support for parsing the syntax of generic
parameters/arguments in declarative macros. This makes it difficult to
impossible to write a declarative macro that handles arbitrary generics easily.
I propose adding fragment specifiers to parse a generic parameter definition as
a whole and also parts of the definition.

# Motivation
[motivation]: #motivation

I personally encountered this issue when attempting to write a declarative macro
to implement a trait on a type. I wanted to write something like this minimal
toy example:

```rust
macro_rules! implement_debug {
    { $params:generic $type:ty $( where $where:where_clause )? } => {
        impl $params Debug for $ty $( where $where )? {
            /* .. implementation goes here .. */
        }
    };
}

struct Container<'a, T>(&'a T);
implement_debug!(<'a, T: Debug + 'a> Container<'a, T>);

struct Container2<'a, T>(&'a T);
implement_debug!(<'a, T> Container<'a, T> where T: Debug + 'a);
```

However, with the current state of declarative macros, there's no way to parse
arbitrary generic parameters in the body of the macro, forcing this macro of
mine to be a procedural macro. However, declarative macros are easier to read
and write, and this could be a declarative macro if there was only a way to
parse generic parameters.

Additionally, more complicated macros want to be able to parse each parameters
and its bounds for use in various places, so I'd like this as well:

```rust
macro_rules! implement_debug {
    { < $( $param:generic_param $( : $bound:generic_bounds )? ),+ > $type:ty } => {
        impl < $( $param $( : $bound )? ),+ > Debug for $ty {
            /* .. implementation goes here .. */
        }
    };
}

struct Container<'a, T>(&'a T);

implement_debug!(<'a, T: Debug + 'a> Container<'a, T>);
```

Any toy example will be obviously redundant with `:generic`, but more involved
macros sometimes want it.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When explaining fragment specifiers, this can be explained by adding the new
fragment specifiers and their descriptions:

* `:generic`: A full set of generic parameters and their bounds (e.g. `<'a, T:
  'a + SomeTrait, const N: usize>`)
* `:generic_param`: A generic parameter (e.g. `'a`, `T`, or `const N`)
* `:generic_bounds`: Bounds on a generic parameter (e.g. `'lifetime + SomeTrait`
  on a type or `usize` on a const parameter).
* `:generic_default`: A default value for a generic type or lifetime
* `:where_clause`: A where clause providing constraints on generic parameters

These five parameters are designed to make it easier to write declarative macros
that take in generic arguments (e.g. to use with a type or function), and then
use them to be generic on e.g. type definitions or `impl` blocks.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Exact parsing behavior:
* `:generic` matches the
  [`GenericParams`](https://doc.rust-lang.org/reference/items/generics.html)
  grammar item.
* `:generic_param` matches any of a lifetime, an identifier, or `const` followed
  by an identifier.
* `:generic_bounds` matches the
  [`TypeParamBounds`](https://doc.rust-lang.org/reference/trait-bounds.html)
  (can be the bounds on a type parameter) or
  [`LifetimeBounds`](https://doc.rust-lang.org/reference/trait-bounds.html) (can
  be the bounds on a lifetime parameter) grammar items, or a type (can be the
  bounds on a const parameter). It may also match nothing.
* `:generic_default` matches a type (can be the default for a type parameter) or
  anything that can be default for a const parameter (a block, an identifier, or
  a literal).
* `:where_clause` matches a
  [`WhereClause`](https://doc.rust-lang.org/reference/items/generics.html#where-clauses)
  excluding the initial `where` token.

All of these can potentially pick up on multiple tokens, so the result of any of
these parses is undestructible in the declarative macro.

Following behavior:
* `:generic` can be followed by anything, as it unambiguously ends when the
  closing `>` appears.
* `:generic_param` is similarly bounded and so anything can follow it, as well.
* `:generic_bounds` can be followed by anything that follows `:path` and `:ty`,
  as it contains some repetition of lifetimes and paths separated by `+`, or a
  type, and `+` is already illegal following a path or type.
* `:generic_default` can be followed by anything that follows `:ty`, since the
  other options all have an unambiguous end.
* `:where_clause` can be followed by anything that follows `:generic_bounds`
  except not a `,`, as it contains a comma-separated repetition whose terms end
  in a generic bound, so we need to tell that the generic bound is ending, and
  we can't have a following comma (because then we don't know if there's another
  repetition).

# Drawbacks
[drawbacks]: #drawbacks

This provides more features which will need to be supported going forward. This
also provides another features which "macros 2.0" will need to implement for
parity with existing declarative macros.

As far as I can tell, this additional cost to implementing and maintaining extra
code is the only drawback associated with this feature.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

We can do nothing, which provides no additional features and avoids the time and
effort cost of implementing and maintaining this feature.

# Prior art
[prior-art]: #prior-art

I'm not personally aware of any other languages that have similar declarative
macros to Rust with an equivalent to fragment specifiers, nor any prior effort
to add fragment specifiers covering this usage into Rust, so I don't know of any
prior art on this topic.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

Are these the best fragment specifiers to use for parsing macros? I'd like
opinions from other people who write macros that would want something like this
about if breaking up generics into some other form might be more useful. I think
this set of fragment specifiers is the best for my use cases, but other people
might be interested in macros that parse differently and they might want other
things instead.

Also, should `:generic_bounds` include the preceding `:` in the match (e.g.
`: 'a + SomeTrait` in the example above)? And likewise with the `=` before
`:generic_default`? I personally think it looks nicer without, but other people
may disagree with my aesthetic preferences.

# Future possibilities
[future-possibilities]: #future-possibilities

This could be combined with metavariable expressions for doing something with
them. I don't know what expressions would be useful for this, but other people
might have ideas.
