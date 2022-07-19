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

Method chaining is something rust users do a lot to provide a nice
flow of data from left-to-right/top-to-bottom which promotes a very natural reading order
of the code.

Sometimes, these method chains can become quite terse for the sake of composability

Forever we hear the complaints of fresh rustaceans learning about option/result
method chaining being [very surprised by the ordering](https://github.com/rust-lang/rfcs/issues/1025) of the methods like
[`map_or_else`](https://doc.rust-lang.org/std/result/enum.Result.html#method.map_or_else).

This RFC proposes promoting the use of match statements by supporting postfix-match, reducing the use of some of these methods terse and potentially confusing method chains.

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
    .ok_or("5x was zero")
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

### into

Occasionally I've bumped into the difficulty of using `Into` in a postfix setting.
The tap crate also provides a `Conv` trait to convert in a method chain using turbofish.

```rust
let upper = "hello, world"
    .conv::<String>()
    .tap_mut(|s| s.make_ascii_uppercase());
```

This can also be emulated with postfix-match

```rust
let upper = "hello, world"
    .match { s => String::from(s) }
    .match { mut s => { s.make_ascii_uppercase(); s } }
```

## async support

One thing of note is that in option chains you cannot use futures unless you use adapters
like [`OptionFuture`](https://docs.rs/futures/0.3.21/futures/future/struct.OptionFuture.html).

Using match, you can avoid that by supporting `.await` directly.

```rust
context.client
    .post("https://example.com/crabs")
    .body("favourite crab?")
    .send()
    .await
    .match {
        Err(_) => Ok("Ferris"),
        Ok(resp) => resp.json::<Option<String>>().await,
        //         this works in a postfix-match ^^^^^^
    }
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

`X.Y.match {}.Z`

Will be interpreted as

```rust
match X.Y {}.Z`
```

I believe this would be the same precedence as `await`.

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

## `await`

`await` was initially proposed to be a prefix keyword.

There was a [suggestion to make it postfix](https://github.com/rust-lang/rfcs/pull/2394#discussion_r179929778) for very similar reasons (not breaking up method chains).

This eventually became the favourite given the pain that await chains introduces in other languages.
I've heard many accounts from people that postfix-await is one of their favourite features of the language.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

## Lifetime extension

Method call chains will not lifetime extend their arguments. Match statements, however,
are notorious for having lifetime extension. It is currently unclear if promoting these
usecases of match would cause more [subtle bugs](https://fasterthanli.me/articles/a-rust-match-made-in-hell#my-actual-bug), or if it's negligable

# Future possibilities
[future-possibilities]: #future-possibilities

Eventually more operators could become postfix, such as `for_each`
