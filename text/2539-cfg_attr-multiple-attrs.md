- Feature Name: `cfg_attr_multi`
- Start Date: 2018-09-10
- RFC PR: [rust-lang/rfcs#2539](https://github.com/rust-lang/rfcs/pull/2539)
- Rust Issue: [rust-lang/rust#54881](https://github.com/rust-lang/rust/issues/54881)

# Summary
[summary]: #summary

Change `cfg_attr` to allow multiple attributes after the configuration
predicate, instead of just one. When the configuration predicate is true,
replace the attribute with all following attributes.

# Motivation
[motivation]: #motivation

Simply put, ergonomics and intent. When you have multiple attributes you
configure away behind the same predicate today, you need to duplicate the entire
predicate. And then when you read code that does this, you have to check the
entire predicates with each other to make sure they're the same. By allowing
multiple attributes it removes that duplication and shows explicitly that the
author wanted those attributes configured behind the same predicate.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The `cfg_attr` attribute takes a configuration predicate and then a list of
attributes that will be in effect when the predicate is true.

For an example of multiple attributes, say we want to have two attribute macros
(`sparkles` and `crackles`), but only when `feature = "magic"` is enabled. We
can write this as:

```rust,ignore
#[cfg_attr(feature = "magic", sparkles, crackles)]
fn bewitched() {}
```

When the feature flag is enabled, it expands to:

```rust,ignore
#[sparkles]
#[crackles]
fn bewitche() {}
```

The list of attributes may be empty, but will warn if the actual source code
contains an empty list.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The next section replaces what's in the Conditional Compilation Chapter for the
`cfg_attr` attribute. It explains both current and new behavior, mainly because
the current reference material needs improvement.

## `cfg_attr` Attribute

The `cfg_attr` attribute conditionally includes attributes based on a
configuration predicate. 

It is written as `cfg_attr` followed by `(`, a comma separated metaitem
sequence, and then `)` The metaitem sequence contains one or more metaitems.
The first is a conditional predicate. The rest are metaitems that are also
attributes. Trailing commas after attributes are permitted. The following list
are all allowed:

* `cfg_attr(predicate, attr)`
* `cfg_attr(predicate, attr_1, attr_2)`
* `cfg_attr(predicate, attr,)`
* `cfg_attr(predicate, attr_1, attr_2,)`
* `cfg_attr(predicate,)`

> Note: `cfg_attr(predicate)` is not allowed. That comma is semantically
> distinct from the commas following attributes, so we require it.

When the configuration predicate is true, this attribute expands out to be an
attribute for each attribute metaitem. For example, the following module will
either be found at `linux.rs` or `windows.rs` based on the target.

```rust,ignore
#[cfg_attr(linux, path = "linux.rs")]
#[cfg_attr(windows, path = "windows.rs")]
mod os;
```

For an example of multiple attributes, say we want to have two attribute macros,
but only when `feature = "magic"` is enabled. We can write this as:

```rust,ignore
#[cfg_attr(feature = "magic", sparkles, crackles)]
fn bewitched() {}
```

When the feature flag is enabled, the attribute expands to:

```rust,ignore
#[sparkles]
#[crackles]
fn bewitche() {}
```

Note: The `cfg_attr` can expand to another `cfg_attr`. For example,
`#[cfg_attr(linux, cfg_attr(feature = "multithreaded", some_other_attribute))`
is valid. This example would be equivalent to
`#[cfg_attr(all(linux, feaure ="multithreaded"), some_other_attribute)]`.

## Warning When Zero Attributes

This RFC allows `#[cfg_attr(predicate,)]`. This is so that macros can generate
it. Having it in the source text emits an `unused_attributes` warning.

## Attribute Syntax Opportunity Cost

This would be the first place attributes would be allowed in a comma-separated
list. As such, it adds a restriction that attributes cannot have a non-delimited
comma.

Today, an attribute can look like:

* `name`,
* ``name(`TokenStream`)``
* ``name = `TokenTree` ``

where `TokenStream` is a sequence of tokens that only has the restriction that
delimiters match and `TokenTree` is a single identifier, literal, punctuation
mark, or a delimited `TokenStream`.

With this RFC accepted, the following cannot ever be parsed as attributes:

* `name, option`
* `name = some, options`

Arguably, we could allow `(name, option)`, but we shouldn't.

This restriction is also useful if we want to put multiple attributes in a
single `#[]` container, which has been suggested, but this RFC will not tackle.

# Drawbacks
[drawbacks]: #drawbacks

It's another thing that has to be learned. Though even there, it's just learning
that the attribute takes 1+, and not just 1 attribute.

It restricts the future allowable syntaxes for attributes.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

We could require that multiple attributes must be within in a delimiter to make
it so that it's always two arguments at the top level. E.g.,
`#[cfg_attr(predicate, [attr, attr])]`. While this could increase clarity, it
mostly seems like it would just add noise. In the multiline case, it already
reads pretty clear with the predicate on the first line and each attribute
indented.

The default alternative of not doing this is a possibility. It would just mean
that conditionally including attributes is slightly less ergonomic than it
could be.

We could change attribute container syntax to allow multiple attributes and then
state that `cfg_attr` takes the attribute container syntax without the `#[]`
part. While this could be a good final state, it's a more ambitious change that
has more drawbacks. There are legitimate reasons we'd want `cfg_attr` to take
multiple attributes but not the attribute container. As such, I would like to
go with the conservative change first.

The original draft of this RFC only allowed one or more attributes and did not
allow the trailing comma. Because it helps macros and fits the rest of the
language, it now allows those.

# Prior art
[prior-art]: #prior-art

I cannot think of any prior art specifically, but changing something from taking
one of something to one or more of something is pretty common.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None.
