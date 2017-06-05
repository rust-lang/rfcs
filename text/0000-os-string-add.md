- Feature Name: `os_string_add`
- Start Date: 2017-06-04
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Implement `Add` for `OsString` analogous to `Add` for `String`.

# Motivation
[motivation]: #motivation

Make concatenating `OsString`s (e.g. in paths and/or command-line arguments) a tiny bit more convenient, which in turn makes writing small command-line utilities easier.

# Detailed design
[design]: #detailed-design

Basically this:

```rust
impl Add<&OsStr> for OsString { type Output = OsString; ... }
```

It is a thin wrapper over `.push`.  The main advantage of this over `push` is that `push` necessitates having a mutable (in/out) variable and is hard to nest.  A typical usage of `push` would require at least 2 extra lines: one for the variable definition, and one for the `push`, neither of which are necessary for `+`.

# How We Teach This
[how-we-teach-this]: #how-we-teach-this

Users already familiar with `String` shouldn't be surprised by the presence of this implementation.

# Drawbacks
[drawbacks]: #drawbacks

Not aware of any besides adding a tiny bit of bloat to the std API surface.

# Alternatives
[alternatives]: #alternatives

Since `.push` already exists, there isn't any other sensible way to define `Add`.

# Unresolved questions
[unresolved]: #unresolved-questions

None.
