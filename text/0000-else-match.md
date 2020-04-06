- Feature Name: `else-match`
- Start Date: 2020-04-06
- RFC PR: -
- Rust Issue: -

# Summary
[summary]: #summary

The goal would be to allow a trailing `match` expression following an `else` branch,
permitting an `else match` construct.  
Similar in style to an an `else if` branch, this would provide most, if not all,
the same benefits an `else if` branch would have.  
A large portion of branching expressions already have a unified
form with `if` (`if let`, `while let`, `else if`).  

# Motivation
[motivation]: #motivation

Currently, a `match` expression cannot trail an else, and must be inserted within the `else`'s scope.  
Much for the same reason `else if` exist, allowing a match expression to take the place of a trailing
`else`, would simplify code, and unify another branching expression.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Syntax:

```rust
let target = if ctx.close {
    None
} else match ctx.target {
    PerfTarget::None => None,
    PerfTarget::FrameTime(ft) => Some(ft),
    PerfTarget::Fps(fps) => Some(1000 / fps as usize),
};
```

Match expressions can be chained together with an `if-else`.  
Similar to the standard `if-else` within Rust and other languages, it allows conditions to remain readable.  
They are still expressions, and must match the return type of the other branches.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

On the surface, should be relatively harmless.
It should only have a couple of interactions between the various parts of the language.

Macros _might_ accept more tokens, but if the developer is using the `expr` fragment, then there should be no risk, as an `else` can't trail an expression.

The `else-match` should be semantically equivalent to a simple `if`else`, but with a `match` within the body.

The scope of this RFC will need to be decided, as it's entirely possible that the matches can be placed anywhere within an `if-else` chain.
If that's the case, it'll probably interact with a lot of semantic analysis.
Including exhaustive checking.

For the most part, we could start with a simple "match expressions can be the last part in an `if-else`".
This is very similar to how an `else` works.

# Drawbacks
[drawbacks]: #drawbacks

It's not too different from just writing the explicit else scope, and writing the match expression from within.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- Most branching expressions have a unified form. `if let`, `while let`, `else if`.

# Prior art
[prior-art]: #prior-art

As far as I'm aware, no language does this intentionally.
I'm not too familiar with the other branches of languages that have matches,
but I would imagine they support this simply because they don't require `else` to have an explicit scope.  
I also think most of the languages that support matches, don't have the same block scoping via braces.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Which positions should be supported?

Should it just be a trailing expression, or can it be used inline?
Eg:

```rust
let target = if ctx.close {
    None
} else match ctx.target {
    PerfTarget::FrameTime(ft) => Some(ft),
    PerfTarget::Fps(fps) => Some(1000 / fps as usize),
} else {
    None
};
```

Is that else branch the same as a `_ => ...` arm?

- There's also the question of a potential ambiguity if/when postfix keywords are introduced:

```rust
let target = if ctx.close {
    None
} else ctx.target.match {
    PerfTarget::None => None,
    PerfTarget::FrameTime(ft) => Some(ft),
    PerfTarget::Fps(fps) => Some(1000 / fps as usize),
}
```

# Future possibilities
[future-possibilities]: #future-possibilities

I can't think of any further extensions at the moment, but considering it's similar to `if-else`, it should be fairly composable.
