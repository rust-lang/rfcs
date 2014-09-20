- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

The current `fn()` types represent something close to `&'static Code`, which means that safely representing a function pointer into code with a non-static lifetime safely is impossible. This RFC describes the addition of the ability to assign a non-static lifetime to the `fn()` type, which would allow `fn():'a` to represent a type similar to `&'a Code`.

# Motivation

This is useful in safely handling plugins, as any function pointer into the plugin could be guaranteed not to outlive the plugin itself. It's also necessary to ensure safety in instances where dynamic code generation is taking place, such as a JIT (just-in-time compilation) engine.

# Detailed design
The syntax for an `fn()` type with the lifetime 'a and return type R would be:
```
fn():'a -> R
```
This grammar would accept any lifetime. A function pointer is always contravariant with regard to its lifetime.
This makes the `fn():'a` type roughly equivalent to the `||:'a` type without an environment.
To maintain backwards compability and code cleanness, the current syntax would imply a 'static lifetime bound.

# Drawbacks

This may introduce further complexity into borrowck.

# Alternatives

Turn `fn()` into a quasi-unsized type, in such a way that the current `fn()` would represent `&'static fn()`.

# Unresolved questions

Should eliding the lifetime lead to it being inferred (as per the lifetime elision RFC) or default to 'static?
One proposal is to have `fn():` represent an elided lifetime, and `fn()` a static one.
