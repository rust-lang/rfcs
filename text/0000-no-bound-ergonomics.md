- Feature Name: auto-bound
- Start Date: 2018-06-24
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

This document adds a new ergonomics syntax to Rust to express type variables which *bounds* or
*bounds combination* – are infered from the implementation – in order to *mirror* the already
existing `impl Trait` in return position that infers the type on the implementation.

```rust
fn foo(x: &impl _) { // x’s bounds will be deduced from the implementation
  let y = x.clone(); // we want Clone here
}
```

In the case of an ambiguity, the bound can be added the regular way:

```rust
fn foo(x: &impl _ + SomeTrait) { // x bounds will be deduced from the implementation
  let y = x.clone(); // we want Clone here
  x.some_trait_method();
}
```

# Motivation
[motivation]: #motivation

The main motivation is that we want to have more people coming from C++ and Java / JavaScript easily
get their feet wet with Rust. A new initiative has started to do this: the *ergonomics initiative*.
The idea is that the community really likes the *safety* of the language, its low *overhead* and its
*explicitness*, but it also seems to love *ergonomics* and *implicit* features to catch and factor
away typical patterns.

C++ programmers are used to what is called [duck typing](https://en.wikipedia.org/wiki/Duck_typing),
a *very good* idea introduced decades ago. This feature enables them to write something like this:

```cpp
template <T> // T can be anything
void add(T a, T b) {
  return a + b;
}
```

This code compiles and will adapt to any kind of input’s type.

> We took a C++ example, but this is also the case in JavaScript, for instance.

This is called duck typing and it’s very likely that programmers used to that will want to use the
same feature in Rust – and we also want that very much, right!

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This RFC adds the new syntax `impl _`. Several good points:

  - No new keyword added, so no breakage.
  - The feature has no known overlapping syntax, so we can add the feature as a minor, non-breaking
    change.
  - It uses two keywords used everywhere in Rust: `impl` and `_`. The syntax even looks like the
    *awesome* `impl Trait` in argument position (universal). Replacing the `Trait` with `_` is akin
    to say “Just give me the trait by looking at my implementation”.

Duck typing is really powerful and is obviously well suited for the new *ergonomics initiative* that
people tend to love nowadays.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This design interacts with the `impl Trait` in argument position and the bound mechanism. `rustc`
will be more permissive on which types it accepts by completely removing bounds, enabling people to
*just use what they are used to from other languages like C++ and Javascript*.

# Drawbacks
[drawbacks]: #drawbacks

This RFC *might* induce *confusion* and some people might have to *yell* because they don’t like
duck typing (the main argument is that it would break the type system in terrible ways, but people
have been using duck typing for decades and most of the softwares you use are likely written in C++,
so is that a *real issue*? Maybe some *work and articles* are needed there?).

# Rationale and alternatives
[alternatives]: #alternatives

As this is an ad hoc feature, no alternative are possible (one might want to change the syntax
though).

# Prior art
[prior-art]: #prior-art

Most of the prior art here can be found from C++, JavaScript, Python, etc. that make a *heavy use*
of duck typing.

# Unresolved questions
[unresolved]: #unresolved-questions

None yet.
