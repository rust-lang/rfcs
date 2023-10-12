- Feature Name: `gen-fn`
- Start Date: 2023-10-10
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add `gen {}` blocks to the language. These blocks implement `Iterator` and
enable writing iterators in regular code by `yield`ing elements instead of having
to implement `Iterator` for a custom struct and manually writing an `Iterator::next`
method body. This is a change similar to adding `async {}` blocks that implement
`Future` instead of having to manually write futures and their state machines.

Furthermore, add `gen fn` to the language. `gen fn foo(arg: X) -> Y` desugars to
`fn foo(arg: X) -> impl Iterator<Item = Y>`.

# Motivation
[motivation]: #motivation

Writing iterators manually can be very painful. Many iterators can be written by
chaining `Iterator` methods, but some need to be written as a `struct` and have
`Iterator` implemented for them. Some of the code that is written this way pushes
people to instead not use iterators, but just run a `for` loop and write to mutable
state. With this RFC, you could write the `for` loop, without mutable state, and get
an iterator out of it again.

As an example, here are three ways to write an iterator over something that contains integers,
only keep the odd integers, and multiply all of them by 2:

```rust
// `Iterator` methods
fn odd_dup(values: impl Iterator<Item = u32>) -> impl Iterator<Item = u32> {
    values.filter(|value| value.is_odd()).map(|value| value * 2)
}
// `struct` and manual `impl`
fn odd_dup(values: impl Iterator<Item = u32>) -> impl Iterator<Item = u32> {
    struct Foo<T>(T);
    impl<T: Iterator<Item = u32>> Iterator<Item = u32> for Foo<T> {
        type Item = u32;
        fn next(&mut self) -> Option<u32> {
            loop {
                let value = self.0.next()?;
                if value.is_odd() {
                    return Some(x * 2)
                }
            }
        }
    }
    Foo(values)
}
// `gen block`
fn odd_dup(values: impl Iterator<Item = u32>) -> impl Iterator<Item = u32> {
    gen {
        for value in values {
            if value.is_odd() {
                yield value * 2;
            }
        }
    }
}

// `gen fn`
gen fn odd_dup(values: impl Iterator<Item = u32>) -> u32 {
    for value in values {
        if value.is_odd() {
            yield value * 2;
        }
    }
}
```

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

- Introducing new named concepts.
- Explaining the feature largely in terms of examples.
- Explaining how Rust programmers should *think* about the feature, and how it should impact the way they use Rust. It should explain the impact as concretely as possible.
- If applicable, provide sample error messages, deprecation warnings, or migration guidance.
- If applicable, describe the differences between teaching this to existing Rust programmers and new Rust programmers.
- Discuss how this impacts the ability to read, understand, and maintain Rust code. Code is read and modified far more often than written; will the proposed feature make code easier to maintain?

For implementation-oriented RFCs (e.g. for compiler internals), this section should focus on how compiler contributors should think about the change, and give examples of its concrete impact. For policy RFCs, this section should provide an example-driven introduction to the policy, and explain its impact in concrete terms.

## New keyword

Starting in the 2024 edition, `gen` is a keyword that cannot be used for naming any items or bindings. This means during the migration to the 2024 edition, all variables, functions, modules, types, ... named `gen` must be renamed. 

## Returning/finishing an iterator

`gen` blocks' trailing expression must be of unit type or the block must diverge before reaching its end.

### Diverging iterators

For example, an `gen` block that produces the sequence `0, 1, 0, 1, 0, 1, ...`, will never return `None`
from `next`, and only drop its captured data when the iterator is dropped.

```rust
gen {
    loop {
        yield 0;
        yield 1;
    }
}
```

If an `gen` panics, the behavior is very similar to `return`, except that `next` doesn't return `None`, but unwinds.

## Error handling

Within `gen` blocks, the `?` operator desugars differently from how it desugars outside of `gen` blocks.
Instead of returning the `Err` variant, `foo?` yields the `Err` variant and then `return`s immediately afterwards.
This has the effect of it being an iterator with `Iterator::Item`'s type being  `Result<T, E>`, and once a `Some(Err(e))`
is produced via `?`, the iterator returns `None` next.

`gen` blocks do not need to have a trailing `Ok(x)` expression, because returning from an `gen` block will make the `Iterator` return `None` from now, which needs no value. Instead all `yield` operations must be given a `Result`.

Similarly the `?` operator on `Option`s will `yield None` if it is `None`, and require passing an `Option` to all `yield` operations.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This is the technical portion of the RFC. Explain the design in sufficient detail that:

- Its interaction with other features is clear.
- It is reasonably clear how the feature would be implemented.
- Corner cases are dissected by example.

The section should return to the examples given in the previous section, and explain more fully how the detailed proposal makes those examples work.

## New keyword

In the 2024 edition we reserve `gen` as a keyword. Previous editions need to use `k#gen` to get the same features.

## Error handling

`foo?` in `gen` blocks desugars to

```rust
match foo.branch() {
    ControlFlow::Break(err) => {
        yield R::from_residual(err);
        return;
    },
    ControlFlow::Continue(val) => val,
}
```

which will stop iteration after the first error. This is the same behaviour that `collect::<Result<_, _>>()` performs
on any iterator over `Result`s

# Drawbacks
[drawbacks]: #drawbacks

It's another language feature for something that can already be written entirely in user code.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- Why is this design the best in the space of possible designs?
- What other designs have been considered and what is the rationale for not choosing them?
- What is the impact of not doing this?
- If this is a language proposal, could this be done in a library or macro instead? Does the proposed change make Rust code easier or harder to read, understand, and maintain?

## Keyword

We could also use `iter` as a keyword. I would prefer `iter` in because I connect generators with a more powerful
scheme than just plain `Iterator`s. The `Generator` trait can do everything that `iter` blocks and `async` blocks can do, and more. I believe connecting the `Iterator`
trait with `iter` blocks is the right choice, but that would require us to carve out many exceptions for this keyword,
as `iter` is used for module names and method names everywhere (including libstd/libcore).

## Contextual keyword

We allow `gen` as an identifier for function names and module names, without that conflicting with `gen` blocks, but that makes the syntax more complicated than necessary, for not too much gain.

## 2021 edition

We could allow `gen` blocks on the 2021 edition via `k#gen {}` syntax.
We can allow `gen fn` on all editions.

## `gen` identifiers on 2024 edition

We can allow `i#gen` identifiers in the 2024 edition in order to refer to items named `gen` in previous edition crates.

## Do not do this

The alternative is to keep adding more helper methods to `Iterator`. It is already rather hard for new Rustaceans to get a hold of all the options they have on `Iterator`.
Some such methods would also need to be very generic (not an `Iterator` example, but https://doc.rust-lang.org/std/primitive.array.html#method.try_map on arrays is something
that has very complex diagnostics that are hard to improve, even if it's nice once it works).

Users can use crates like [`genawagen`](https://crates.io/crates/genawagen) instead, which work on stable and give you `gen!` blocks that behave pretty mostly
like `gen` blocks, but don't have compiler support for nice diagnostics or language support for the `?` operator.

# Prior art
[prior-art]: #prior-art

## Python

Python has `gen fn`: any function that uses `yield` internally.
These work pretty much like the `gen` functions proposed in this PR. The main difference is that raising an
exception automatically passes the exception outwards, instead of yielding an `Err()` element.

```python
def odd_dup(values):
    for value in values:
        if is_odd(value):
            yield value * 2
```

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- What parts of the design do you expect to resolve through the RFC process before this gets merged?
- What parts of the design do you expect to resolve through the implementation of this feature before stabilization?
- What related issues do you consider out of scope for this RFC that could be addressed in the future independently of the solution that comes out of this RFC?

## Panicking

What happens when a `gen` block that panicked gets `next` called again? Do we need to poison the iterator?

## Fusing

Are `gen` blocks fused? Or may they behave eratically after returning `None` the first time?

# Future possibilities
[future-possibilities]: #future-possibilities

## `yield from` (forwarding operation)

Python has the ability to `yield from` an iterator.
Effectively this is syntax sugar for looping over all elements of the iterator and yielding them individually.
There are infinite options to choose from if we want such a feature, so I'm just going to list the general ideas below:

### Do nothing, just use loops

```rust
for x in iter {
    yield x
}
```

### language support

we could do something like postfix `yield` or an entirely new keyword, or...

```rust
iter.yield
```

### stlib macro

We could add a macro to the standard library and prelude, the macro would just expand to the for loop + yield.

```rust
yield_all!(iter)
```
