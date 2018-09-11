- Feature Name: cfg_attr_multi
- Start Date: 2018-09-10
- RFC PR: 
- Rust Issue: 

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

```rust,igore
#[cfg_attr(feature = "magic", sparkles, crackles)]
fn bewitched() {}
```

When the feature flag is enabled, it is equivalent to:

```rust,ignore
#[sparkles]
#[crackles]
fn bewitche() {}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This replaces what's in the Conditional Compilation Chapter for the `cfg_attr`
attribute. It explains both current and new behavior, mainly because the
current reference material needs improvement.

## `cfg_attr` Attribute

The `cfg_attr` attribute conditionally includes attributes based on a
configuration predicate. 

It is written as `cfg_attr` followed by `(`, a comma separated metaitem
sequence, and then `)` The metaitem sequence contains two or more metaitems.
The first is a conditional predicate. The rest are metaitems that are also
attributes.

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

```rust,igore
#[cfg_attr(feature = "magic", sparkles, crackles)]
fn bewitched() {}
```

When the feature flag is enable, it is equivalent to:

```rust,ignore
#[sparkles]
#[crackles]
fn bewitche() {}
```

Note: The `cfg_attr` can expand to another `cfg_attr`. For example,
`#[cfg_attr(linux, cfg_attr(feature = "multithreaded", some_other_attribute))`
is valid. This example would be equivalent to
`#[cfg_attr(and(linux, feaure ="multithreaded"), some_other_attribute)]`.

# Drawbacks
[drawbacks]: #drawbacks

It's another thing that has to be learned. Though even there, it's just learning
that the attribute takes 1+, and not just 1 attribute.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

There are no other alternatives.

By not doing this, conditionally including attributes is slightly less
ergonomic than it can be.

# Prior art
[prior-art]: #prior-art

I cannot think of any prior art specifically, but changing something from taking
one of something to one or more of something is pretty common.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None.
