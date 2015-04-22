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

## Reserve only one word

It may be possible to implement async features in a backward compatible way using only one keyword. Such a decision should probably deferred, leaving this as a less than ideal option.

# Unresolved questions

Should we also reserve `defer`, used by _IcedCoffeeScript_?
