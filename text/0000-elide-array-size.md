- Feature Name: elide_array_size
- Start Date: 2018-09-18
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

In arrays, allow to elide the size in the type and put an underscore there
instead if it can be deduced from the initializer. For example: `static
BLORP_NUMBERS: [u32; _] = [0, 8, 15];`.

# Motivation
[motivation]: #motivation

This will make it easier to set up static data. With the current syntax, one
needs to count the elements of an array manually to determine its size. Letting
the compiler find the size reduces the potential for error. It also allows us
to ascribe the array component type without requiring the size (which is
perhaps surprisingly not currently allowed).

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Requiring to write `[MyType; 42]` for a 42-element array gets tedious,
especially since the compiler already knows the size from the initializer. To
reduce the strain, Rust allows you to omit the `42` in those type ascriptions.

For example, you might write:

```rust
const CONST_CHARS: [u8; _] = b"This is really a byte array";
static STATIC_MASKS: [u8; _] = [0, 1, 3, 7, 15, 31, 63, 127, 255];

fn main() {
    let local_strs: [&'static str; _] = ["Hello", "Rust"];
    ..
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This feature is a simple extension of the parser plus AST transformation. In
its proposed form, there are no corner cases, because the only thing it enables
that is currently disallowed is to put a `_` wildcard in place of the number of
elements in array types in `let`s, `const`s and `static`s.

In the parser, at the array length position, we must allow underscores. The AST
may need to be extended to allow underscore; In principle `AnonConst` can
contain any expression, so `_` could be represented by a `Path`, or the whole
`AnonConst` could be made `Option`al.

At this point we should disallow a `None` value for `let` bindings without
initializers or with initializers that are not `ExprKind::Repeat` or
`ExprKind::Array`.

At the lowering stage, the initializer is checked for the actual array length,
and the `Ty` is constructed as if the size was given verbatim.

# Drawbacks
[drawbacks]: #drawbacks

There is a modicum of complexity to extend the parser and AST which needs to be
done for every Rust parser in existenct. However, the feature is minor, so the
cost should be acceptable.

Also for longer array declarations the actual size may no longer be obvious
from the code. However, putting any probable or improbable length in and
observing the compiler error (if any) is enough to find out; also the author
hopes that the programmers will put in the lengths if they are essential.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

We could extend this to full inference, which would require us to set the size
*after* lowering. This would make the feature more powerful, but reduce
locality. For example if the array isn't initialized in the next 100 lines or
is initialized from a `static`, or a `fn` that is far away in another module,
changes to said item would lead to errors down the line that are needlessly
hard to track down.

We could leave out the wildcard entirely, but `[u32; ]` looks strange and could
possibly indicate an error, so it's better to use the more balanced `[u32; _]`.

We could do nothing, and waste Rustacean's time by counting or parsing compiler
errors (unless their accuracy of estimating array length is 100%. I'm sure mine
isn't).

# Prior art
[prior-art]: #prior-art

We already allow wildcard lifetimes, which have been beneficial. I'll defer to
[RFC 2115](https://rust-lang.github.io/rfcs/2115-argument-lifetimes.html) for
more information.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None
