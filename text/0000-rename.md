- Feature Name: rename_attr
- Start Date: 2015-06-03
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Add a `#[renamed(from="old_name", since="...")]` attribute for renaming APIs in a forwards
*and* backwards compatible way that requires no effort or trouble to downstream consumers.

# Motivation

Naming things is hard, and APIs churned enough near the release of 1.0 that we stabilized on
some suboptimal names. We'll probably do it again in the future. Therefore it would be
desirable to deprecate these in favour of a rename. However to avoid breaking everyone
there needs to be some trick to make impls still work. There are a few ways to do this today,
but all of them cause janky behaviour. For instance consumers of one name might get different
behaviour. However if we could alias functions/methods under new names as in a first-class
way, we could avoid any problems.

In addition, the mythical rustfix could identify these attributes and automatically fix the
source of anything using the old names.

# Detailed design

Add an attribute for functions `#[renamed(from="old_name", since="...")]`. This should have the following behaviour:

* Any reference to the current or old_name should resolve to the current
* The old name can be silently shadowed, to avoid namespace pollution
* The function can only be implemented under one name (deny duplicates)
* Users of the old name should receive a `warn(renamed)`
* Multiple renames should be able to co-exist (though this would be really unfortunate)

# Drawbacks

There are none beyond the usual "adding stuff"

# Alternatives

Enable full `fn` aliases via a `fn foo = bar;` syntax. This would allow renaming
without any warning in the case that signatures match exactly but both names should
be exposed. For instance `fn new = Default::default` is a plausible thing to want.

Then renames could theoretically be resolved simply by deprecating the alias.

This design is not considered simply because it would be considerably more complex
and require substantially more debate to hammer out all the details (would we want
to support some kind of "currying" like `type` does with `type<T> = Foo<Concrete, T>`?)

The RFC author would like to have a tool for this in the short-term, as there are
several outstanding rename requests.

# Unresolved questions

* Should this be blocked on a more general design?
* Should this be enabled on traits and structs? They can just use `pub use`, but
they don't get all the other "niceness". Regardless this can be "upgraded" to later
support structs and traits.