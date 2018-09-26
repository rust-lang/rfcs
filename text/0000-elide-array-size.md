- Feature Name: elide_array_size
- Start Date: 2018-09-18
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

In arrays declared in `static`s, `const`s and `let` bindings with initializer, 
allow to elide the size in the type and put an underscore there instead if it 
can be inferred. For example: `static BLORP_NUMBERS: [u32; _] = [0, 8, 15];`.

# Motivation
[motivation]: #motivation

This will make it easier to set up static data. With the current syntax, one 
needs to count the elements of an array manually to determine its size. Letting 
the compiler find the size reduces the potential for error. In `let` bindings 
it also allows us to ascribe the array component type without requiring the 
size (which is – perhaps surprisingly – not currently allowed).

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Requiring to write `[MyType; 42]` for a 42-element array gets tedious,
especially since the compiler already knows the size from the initializer. To
reduce the strain, Rust allows you to omit the `42` in those type ascriptions.

For example, you can write:

```rust
const CONST_CHARS: [u8; _] = *b"This is really a byte array";
static STATIC_MASKS: [u8; _] = [0, 1, 3, 7, 15, 31, 63, 127, 255];

fn main() {
    let local_strs: [&'static str; _] = ["Hello", "Rust"];
    ..
}
```

In all other positions and on `let` bindings without initializer, the exact
number keeps being required.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This feature is a simple extension of the parser plus AST transformation. In
its proposed form, there are no corner cases, because the only thing it enables
that is currently disallowed is to put a `_` wildcard in place of the number of
elements in array types in `let`s, `const`s and `static`s.

In the parser, at the array length position, we allow underscores. The AST
may need to be extended to allow underscore; In principle `AnonConst` can
contain any expression, so `_` could be represented by a `Path`, or the whole
`AnonConst` could be made `Option`al.

To ensure locality of reasoning, this RFC proposes a lint for `const` or
`static` items with initializers other than `ExprKind::Repeat` or
`ExprKind::Array`.

The length can simply be inferred from the initializer. Care should be taken to 
keep the error messages useful.

# Drawbacks
[drawbacks]: #drawbacks

There is a modicum of complexity to extend the parser and AST which needs to be
done for every Rust parser in existence. However, the feature is minor, so the
cost should be acceptable.

Also for longer array declarations the actual size may no longer be obvious
from the code. However, putting any probable or improbable length in and
observing the compiler error (if any) is enough to find out; also the author
hopes that the programmers will put in the lengths if they are essential.

Finally, it's possible to add a clippy lint that suggests replacing the 
underscore with the actual length.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This feature is very useful and has been requested numerous times. A draft of
this RFC actually proposed a very resticted type of inference, which would
however clash with RFC [#2000](https://github.com/rust-lang/rfcs/pull/2000).
So the solution is to add the restriction as a lint (either in rustc or
clippy).

We could leave out the wildcard entirely, but `[u32; ]` looks strange and could
possibly indicate an error, so it's better to use the more balanced `[u32; _]`.

We could do nothing, and waste Rustacean's time by counting or parsing compiler
errors (unless their accuracy of estimating array length is 100%. I'm sure mine
isn't) or use a macro (see below).

# Prior art
[prior-art]: #prior-art

We already allow wildcard lifetimes, which have been beneficial. I'll defer to
[RFC 2115](https://rust-lang.github.io/rfcs/2115-argument-lifetimes.html) for
more information.

Following the tradition of using macros to prototype language features, Alex
Durka has built the [counted-array](https://crates.io/crates/counted-array)
crate which contains a macro to enable the proposed syntax.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

Bikeshedding the lint name
