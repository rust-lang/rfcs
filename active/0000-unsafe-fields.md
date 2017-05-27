- Start Date: 10-7-2014
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Add `unsafe` fields. Unsafe fields are declared in the definition of a
struct by prefacing them with `unsafe`, and a public `unsafe` field is
prefixed with `pub unsafe`. Accessing an `unsafe` field is `unsafe`.

# Motivation

We are doing this because it is sometimes useful to expose certain details
of a struct, perhaps in a `raw` module, but to make it clear that accessing
those fields directly can lead to an inconsistent state. For instance writing
directly to the body of an HTTP Response before writing the headers can lead
to incorrect, possibly unsafe behavior, however, it is still useful to expose
the ability to do so for other libraries which want to build safer abstractions
above a raw request or response.

# Detailed design

Add the ability to declare fields `unsafe`, and make their access through
dot notation `unsafe` and require an `unsafe` block or function.

# Drawbacks

Adds complexity.

# Alternatives

Allow access to all private fields using `unsafe`. The drawbacks of this
approach are that there are some fields which truly *must* be private, and
allowing blanket `unsafe` access could encourage serious issues.

Don't do this, only allow the creation of `unsafe` getters and setters
rather than direct field access. The drawbacks here are that this is verbose
and annoying to both write and use.

# Unresolved questions

None.

