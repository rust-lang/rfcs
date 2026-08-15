- Feature Name: `async_gen_types`
- Start Date: 2024-05-06
- RFC PR: [rust-lang/rfcs#3628](https://github.com/rust-lang/rfcs/pull/3628)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Allow the syntax `async T` and `gen T` as types, equivalent to
`impl Future<Output = T>` and `impl Iterator<Item = T>` respectively. Accept
them anywhere `impl Trait` can appear.

# Motivation
[motivation]: #motivation

Users working with asynchronous code may encounter `impl Future<Output = T>`
types.  Users working with iterators may encounter `impl Iterator<Item = T>`
types.

These types are long and cumbersome to work with. They may be the first time
a user will encounter an associated type, and they add verbosity that
obfuscates the `Output`/`Item` types that people care more about. In
particular, a function that combines multiple futures or iterators with other
types requires reading past a lot of syntactic overhead.

Users do not encounter these types when consuming iterators with loops or
combinators (or in the future producing them with `gen` blocks), or when
producing or consuming futures using async/await syntax.

The syntax proposed by this RFC provides the same benefits that the current
`async fn` syntax does (highlighting the future output type), but usable in any
type rather than only in function return values.

# Explanation
[explanation]: #explanation

In any context where you can write an `impl Trait` type, you can write
`async T`, which desugars to `impl Future<Output = T>`:

```rust
fn future_seq<T, U>(f1: async T, f2: async U) -> async (T, U) {
    async {
        (f1.await, f2.await)
    }
}
```

Similarly, in any context where you can write an `impl Trait` type, you can
write `gen T`, which desugars to `impl Iterator<Item = T>`:

```rust
fn iter_seq<T>(g1: gen T, g2: gen T) -> gen T {
    gen {
        yield from g1;
        yield from g2;
    }
}
```

These syntaxes work exactly as their desugarings suggest, and can appear
anywhere their desugarings can appear.

Compare these to the longhand versions of these two functions:

```rust
fn future_seq<T, U>(
    f1: impl Future<Output = T>,
    f2: impl Future<Output = U>,
) -> impl Future<Output = (T, U)> {
    async {
        (f1.await, f2.await)
    }
}

fn iter_seq<T>(
    g1: impl Iterator<Item = T>,
    g2: impl Iterator<Item = T>)
-> impl Iterator<Item = T> {
    gen {
        yield from g1;
        yield from g2;
    }
}
```

Notice how much longer these are, and how much more syntax the user needs to
wade through to observe the types they care about.

# Drawbacks
[drawbacks]: #drawbacks

This adds an additional case to Rust type syntax.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

We could introduce a mechanism to abbreviate `impl Future<Item = T>` as
`impl Future<T>` or `impl Fut<T>` or similar. However, this still leaves much
of the syntactic "weight" in place. In addition, this may confuse users by
obfuscating the difference between associated types and generic parameters.

# Prior art
[prior-art]: #prior-art

We have special syntaxes for arrays in types, `[T]` and `[T; N]`, which are
evocative of the corresponding value syntax for arrays. Similarly, the syntax
for tuple types `(A, B)` is evocative of the syntax for tuple values `(a, b)`.

The use of `async fn` to hide the asynchronous type serves as a partial
precedent for this: the case made at the time was that users cared about the
output type of the future more than they cared about the `Future` trait. This
RFC extends that benefit to any place a type can appear.

Similarly, `async` blocks do not require specifying the Future trait, and
neither do the proposed `gen` blocks.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

The `gen T` syntax can be added as unstable right away, but should not be
stabilized until we stabilize the rest of `gen` support.

Introducing `async T` as a type meaning `impl Future<Output = T>` would close
off the use of `async T` as a syntax for "asynchronous versions" of existing
types (e.g. `async File`).

# Future possibilities
[future-possibilities]: #future-possibilities

Once we add `async gen` support, we can add the corresponding type
`async gen T`, mapping to whatever type we use for async iterators.

These syntaxes would work very well together with a syntax to abbreviate
functions consisting of a single block.

For example:

```rust
fn countup(limit: usize) -> gen usize
gen {
    for x in 0..limit {
        yield i;
    }
}

fn do_something_asynchronously() -> async ()
async {
    do_something().await;
}
```

Together, these mechanisms would provide a general solution for what might
otherwise motivate a `gen fn` feature. Using `gen T` as a type makes the return
type simple enough to not need to hide the type.
