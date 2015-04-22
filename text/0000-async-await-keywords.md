- Feature Name: `async_await_keywords`
- Start Date: 2015-04-21
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

This RFC proposes to reserve `async` and `await` as keywords for future compatibility.

# Motivation

Many mainstream languages are gaining language support for cooperative asychronous programing. We'd like to make sure that we can also introduce similar features in the future without backward incompatibilities.

.NET languages such as C# and Visual Basic are using `async` and `await` for their native asyncronous features, and other languages including ECMAScript 7 and Python are looking to use the keywords as well.

# Detailed design

Reserve `async` and `await` as keywords.

# Drawbacks

`async` and `await` can no longer be used as identifiers.

# Alternatives

## Do nothing

Don't add any new keywords. If we decide in the future to design features using `async` and `await`, it will require Rust version 2.0, since it will be a backward incompatible change.

There are a couple of clever solutions that also could help us avoid this backward incompatibility even if we choose this path:

1. **Contextual keywords**
   The `async` and `await` keywords could only be keywords in specific contexts where their use is possible. This would limit the possibilities for where we could have these keywords to places where it was not valid syntax before, and would likely require more complexity in the implementation, so it's probably a design decision best left for later.
2. **Explicit opt-in**
   The `async` and `await` keywords could only be keywords if the developer has explicitly opted in to their use. This is similar in concept to Python's `from __future__ import print_function`, etc, where the list of keywords is different based on this marker. This might be done at the crate or module level, with syntax something like `#![feature(async_syntax)]`. A new major version would still be required to deprecate the opt-in and make it the default, but that major version could happen at a later time. This is a pretty good idea for things we can't predict, and we may end up needing it for other things, but we can avoid this overhead if we can reserve these keywords ahead of time.

## Reserve only one word

It may be possible to implement async features in a backward compatible way using only one keyword. Such a decision should probably deferred, leaving this as a less than ideal option.

# Unresolved questions

Should we also reserve `defer`, used by _IcedCoffeeScript_?
