- Feature Name: `postfix-match`
- Start Date: 2022-07-10
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

An alternative postfix syntax for match expressions that allows for interspercing match statements with function chains

```rust
foo.bar().baz.match {
    _ => {}
}
```

as syntax sugar for

```rust
match foo.bar().baz {
    _ => {}
}
```

# Motivation
[motivation]: #motivation

Forever we hear the complaints of fresh rustaceans learning about option/result
method chaining being [very surprised by the ordering](https://github.com/rust-lang/rfcs/issues/1025) of the methods like
[`map_or_else`](https://doc.rust-lang.org/std/result/enum.Result.html#method.map_or_else).

This RFC proposes deprecating some of these methods for a slightly more verbose but likely
more readable versions using match statements.

In order to keep the spirit of method chaining, the core of this proposal is allowing
match to be written in a postfix form.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

`match expressions` are how one would normally deal with the values
of an enum type (eg `Option`, `Result`, etc.) by pattern matching on each
variant. Some powerful techniques like pattern binding, nested patterns and or patterns
allow for some versatile and concise, but still fairly readable syntax for dealing
with these types.

Rust often features functional approachs to lots of problems. For instance,
it's very common to have chains of `map`, `and_then`, `ok_or`, `unwrap`
to process some `Option` or `Result` type in a pipeline, rather than continously reassigning to new variable bindings.

```rust
let x = Some(42);
let magic_number = x.map(|x| x * 5)
    .and_then(NonZeroI32::new)
    .ok_or_else("5x was zero")
    .unwrap();
```

Some of these provided method chains are fairly readable, like the ones presented above,
but sometimes the desire to use long method chains is met with unweildy hacks or awkward function arguments.

```rust
let x = Some("crustaceans");
x.and_then(|x| (!x.is_empty()).then(x)) // None if x is an empty string
    .map_or_else(
        || "Ferris", // default
        |x| &x[1..], // remove first letter
    );
```

These can be re-written using postfix match to be much more self-documenting

```rust
x.match {
    Some("") | None => None
    x @ Some(_) => x
}.match {
    Some(x) => &x[1..],
    None => "Ferris",
};

// or even just

x.match {
    Some("") | None => "Ferris"
    x @ Some(_) => &x[1..]
};
```

While this example ended up being a single match, and is near-equivalent to the `match x {}` form, often these option chains can get quite long, especially when interspersed with `?`, `.await` and other forms of postfix syntax we already make heavy use of.

you could imagine that this `x` would be replaced with

```rust
context.client
    .post("https://example.com/crabs")
    .body("favourite crab?")
    .send()
    .await?
    .json::<Option<String>>()
    .await?
    .as_ref()
```

```rust
// prefix match
match context.client
    .post("https://example.com/crabs")
    .body("favourite crab?")
    .send()
    .await?
    .json::<Option<String>>()
    .await?
    .as_ref()
{
    Some("") | None => "Ferris"
    x @ Some(_) => &x[1..]
};

// postfix match
context.client
    .post("https://example.com/crabs")
    .body("favourite crab?")
    .send()
    .await?
    .json::<Option<String>>()
    .await?
    .as_ref()
    .match {
        Some("") | None => "Ferris"
        x @ Some(_) => &x[1..]
    };
```

While it's a matter of taste, the postfix form is much easier to parse
in terms of the flow of data. Having to jump back out to the beginning of
the expression to see the `match` keyword to know what the new expression context is
can be difficult to read.

While I am not at liberty to share the codebase, the former is equivalent to something
I see on some of our codebases at work.

## tap (pipelines)

While not the intended use of this proposal, [tap](https://crates.io/crates/tap) is a popular crate to allow more advanced pipeline operations.

```rust
let val = original_value
  .pipe(first)
  .pipe(|v| second(v, another_arg))
  .pipe(third)
  .pipe(|v| last(v, another_arg));
```

This can be written similarly using our postfix match:

```rust
let val = original_value
  .match { v => first(v) }
  .match { v => second(v, another_arg) }
  .match { v => third(v) }
  .match { v => last(v, another_arg) };
```

This avoids the need for a new dedicated pipeline operator or syntax.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This is the technical portion of the RFC. Explain the design in sufficient detail that:

- Its interaction with other features is clear.
- It is reasonably clear how the feature would be implemented.
- Corner cases are dissected by example.

The section should return to the examples given in the previous section, and explain more fully how the detailed proposal makes those examples work.

# Drawbacks
[drawbacks]: #drawbacks

Having multiple forms of match could be confusing. However, I believe
that most issues could be resolved if clippy would detect uses of

```rust
x.match {}
```

when the desugared

```rust
match x {}
```

would fit on a single line after being rustfmt. The opposite could
also be true - if the scrutinee spans multiple lines, it should be made into
a postfix form.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Rationale

The core rationale is that a lot of these method chain functions are designed to
avoid using bulky match statements that ruin the flow.

Rather than keep adding more of these methods to suit the needs, we should
make the language more flexible such that match statements aren't a hindrance.

## Alternatives

[postfix macros](https://github.com/rust-lang/rfcs/pull/2442) have been an idea for many years now. If they were to land, this feature
could easily be implemented as a macro (bikeshedding on postfix macro syntax):

```rust
macro match_! (
    postfix { $(
        $arm:pat => $body:expr,
    )+ } => {
        match $self { $(
            $arm => $body,
        )+ }
    }
)
```

However, after years of discussion and hundreds of thumbs-ups, it feels like we're
still not close to agreeing on syntax or behaviour.

# Prior art
[prior-art]: #prior-art

Discuss prior art, both the good and the bad, in relation to this proposal.
A few examples of what this can include are:

- For language, library, cargo, tools, and compiler proposals: Does this feature exist in other programming languages and what experience have their community had?
- For community proposals: Is this done by some other community and what were their experiences with it?
- For other teams: What lessons can we learn from what other communities have done here?
- Papers: Are there any published papers or great posts that discuss this? If you have some relevant papers to refer to, this can serve as a more detailed theoretical background.

This section is intended to encourage you as an author to think about the lessons from other languages, provide readers of your RFC with a fuller picture.
If there is no prior art, that is fine - your ideas are interesting to us whether they are brand new or if it is an adaptation from other languages.

Note that while precedent set by other languages is some motivation, it does not on its own motivate an RFC.
Please also take into consideration that rust sometimes intentionally diverges from common language features.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

## Lifetime extension

Method call chains will not lifetime extend their arguments. Match statements, however,
are notorious for having lifetime extension. It is currently unclear if promoting these
usecases of match would cause more [subtle bugs](https://fasterthanli.me/articles/a-rust-match-made-in-hell#my-actual-bug), or if it's negligable

# Future possibilities
[future-possibilities]: #future-possibilities

Think about what the natural extension and evolution of your proposal would
be and how it would affect the language and project as a whole in a holistic
way. Try to use this section as a tool to more fully consider all possible
interactions with the project and language in your proposal.
Also consider how this all fits into the roadmap for the project
and of the relevant sub-team.

This is also a good place to "dump ideas", if they are out of scope for the
RFC you are writing but otherwise related.

If you have tried and cannot think of any future possibilities,
you may simply state that you cannot think of anything.

Note that having something written down in the future-possibilities section
is not a reason to accept the current or a future RFC; such notes should be
in the section on motivation or rationale in this or subsequent RFCs.
The section merely provides additional information.
