- Feature Name: `gen-fn`
- Start Date: 2023-10-10
- RFC PR: [rust-lang/rfcs#3513](https://github.com/rust-lang/rfcs/pull/3513)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Reserve the `gen` keyword in the 2024 edition and start experimenting on
implementing generator functions and blocks that allow implementing `Iterator`s
without writing `Iterator::next` impls under placeholder syntax.

Add `#[rustc_gen] {}` blocks to the language. These implement `Iterator` by `yield`ing
elements. This is simpler and more intuitive than creating a custom type and
manually implementing `Iterator` for that type, which requires writing an
explicit `Iterator::next` method body. This is a change similar to adding `async
{}` blocks that implement `Future` instead of having to manually write futures
and their state machines.

Furthermore, add `#[rustc_gen] fn` to the language. `#[rustc_gen] fn foo(arg: X) -> Y` desugars to
`fn foo(arg: X) -> impl Iterator<Item = Y>`.

# Motivation
[motivation]: #motivation

The main motivation of this RFC is to reserve a new keyword in the 2024 edition.
The feature used by the keyword described here should be treated as an e-RFC for
experimentation on nightly. I would like to avoid discussion of the semantics
provided here, deferring that discussion until during the experimental
implementation work.

Writing iterators manually can be very painful. Many iterators can be written by
chaining `Iterator` methods, but some need to be written as a `struct` and have
`Iterator` implemented for them. Some of the code that is written this way
pushes people to avoid iterators and instead execute a `for` loop that eagerly
writes values to mutable state. With this RFC, one can write the `for` loop
and still get a lazy iterator of values.

As an example, here are multiple ways to write an iterator over something that contains integers
while only keeping the odd integers and multiplying each by 2:

```rust
// `Iterator` methods
fn odd_dup(values: impl Iterator<Item = u32>) -> impl Iterator<Item = u32> {
    values.filter(|value| value.is_odd()).map(|value| value * 2)
}

// `std::iter::from_fn`
fn odd_dup(mut values: impl Iterator<Item = u32>) -> impl Iterator<Item = u32> {
    std::iter::from_fn(move || {
        loop {
            let value = values.next()?;
            if value % 2 == 1 {
                return Some(value * 2);
            }
        }
    })
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
    #[rustc_gen] {
        for value in values {
            if value.is_odd() {
                yield value * 2;
            }
        }
    }
}

// `gen fn`
#[rustc_gen]
fn odd_dup(values: impl Iterator<Item = u32>) -> u32 {
    for value in values {
        if value.is_odd() {
            yield value * 2;
        }
    }
}
```

Iterators created with `#[rustc_gen]` return `None` once they `return` (implicitly at the end of the scope or explicitly with `return`).
See [the unresolved questions][unresolved-questions] for whether `#[rustc_gen]` iterators are fused or may behave strangely after having returned `None` once.
Under no circumstances will it be undefined behavior if `next` is invoked again after having gotten a `None`.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## New keyword

Starting in the 2024 edition, `gen` is a keyword that cannot be used for naming any items or bindings. This means during the migration to the 2024 edition, all variables, functions, modules, types, etc. named `gen` must be renamed.

## Returning/finishing an iterator

`#[rustc_gen]` block's trailing expression must be of the unit type or the block must diverge before reaching its end.

### Diverging iterators

For example, a `#[rustc_gen]` block that produces the infinite sequence `0, 1, 0, 1, 0, 1, ...`, will never return `None`
from `next`, and only drop its captured data when the iterator is dropped:

```rust
#[rustc_gen] {
    loop {
        yield 0;
        yield 1;
    }
}
```

If a `#[rustc_gen]` block panics, the behavior is very similar to `return`, except that `next` unwinds instead of returning `None`.

## Error handling

Within `#[rustc_gen]` blocks, the `?` operator desugars differently from how it desugars outside of `#[rustc_gen]` blocks.
Instead of returning the `Err` variant, `foo?` yields the `Err` variant and then `return`s immediately afterwards.
This creates an iterator with `Iterator::Item`'s type being `Result<T, E>`.
Once a `Some(Err(e))` is produced via `?`, the iterator returns `None` on the subsequent call to `Iterator::next`.

In contrast to other code where you can use `?`, `#[rustc_gen]` blocks do not need to have a trailing `Ok(x)` or `x` expression.
Returning from a `#[rustc_gen]` block will make the `Iterator` return `None`, which needs no value.
Instead, all `yield` operations must be given a `Result`.

The `?` operator on `Option`s will `yield None` if it is `None`, and require passing an `Option` to all `yield` operations.
As an example:

```rust
let x = some_option?;
yield Some(x + 1)
```

will yield `None` if `some_option` is `None`, but `Some(x + 1)` otherwise.

## Fusing

Like `Generators`, `Iterator`s produced by `#[rustc_gen]` panic when invoked again after they have returned `None` once.
This will probably be fixed by special casing the generator impl if `Generator::Return = ()`, as we can trivially
produce infinite values of the unit type.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation
## New keyword

In the 2024 edition we reserve `gen` as a keyword. Previous editions will use `k#gen` to get the same features.

## Error handling

`foo?` in `#[rustc_gen]` blocks will stop iteration after the first error by desugaring to:

```rust
match foo.branch() {
    ControlFlow::Break(err) => {
        yield R::from_residual(err);
        return;
    },
    ControlFlow::Continue(val) => val,
}
```

This is the same behaviour that `collect::<Result<_, _>>()` performs
on iterators over `Result`s.

## Implementation

This feature is mostly implemented via existing generators.
We'll need additional desugarings and lots of work to get good diagnostics.

### `#[rustc_gen] fn`

`#[rustc_gen] fn` desugars to the function itself with the return type replaced by `impl Iterator<Item = $ret>` and its body wrapped in a `#[rustc_gen]` block.
A `#[rustc_gen] fn`'s "return type" is its iterator's `yield` type.

A `#[rustc_gen] fn` captures all lifetimes and #[rustc_gen]eric parameters into the `impl Iterator` return type (just like `async fn`).
If more control over captures is needed, type alias impl trait can be used when it is stabilized.

Like other uses of `impl Trait`, auto traits are revealed without being specified.

### `#[rustc_gen]` blocks

`#[rustc_gen]` blocks are the same as an unstable generator...

* ...without arguments,
* ...with an additional check forbidding holding borrows across `yield` points,
* ...and with an automatic `Iterator` implementation.

We'll probably be able to modularize the generator implementation and make it more robust on the implementation and diagnostics side for the `#[rustc_gen]` block case, but I believe the initial implementation should be a HIR lowering to a generator and wrapping that generator in [`from_generator`][].

## Fusing

Special case the generator implementation if `Generator::Return = ()` to not panic, but
repeatedly produce values of the unit type.

# Drawbacks
[drawbacks]: #drawbacks

It's another language feature for something that can already be written entirely in user code.

In contrast to `Generator`, `#[rustc_gen]` blocks that produce `Iterator`s cannot hold references across `yield` points.
See [`from_generator`][] which has an `Unpin` bound on the generator it takes to produce an `Iterator`.

[`from_generator`]: https://doc.rust-lang.org/std/iter/fn.from_generator.html

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives
## Keyword

We could use `iter` as the keyword.
I prefer `iter` because I connect generators with a more powerful scheme than plain `Iterator`s.
The `Generator` trait can do everything that `iter` blocks and `async` blocks can do and more.
I believe connecting the `Iterator` trait with `iter` blocks is the right choice,
but that would require us to carve out many exceptions for this keyword as `iter` is used for module names and method names everywhere (including libstd/libcore).
It may not be much worse than `gen` (see also [the unresolved questions][unresolved-questions]).
We may want to use `gen` for full on generators in the future.

## Do not do this

One alternative is to keep adding more helper methods to `Iterator`.
It is already hard for new Rustaceans to be aware of all the capabilities of `Iterator`.
Some of these new methods would need to be very generic.
While it's not an `Iterator` example, [`array::try_map`][] is something that has very complex diagnostics that are hard to improve, even if it's nice once it works.

Users can use crates like [`genawaiter`](https://crates.io/crates/genawaiter) or [`propane`](https://crates.io/crates/propane) instead.
`genawaiter` works on stable and provides `gen!` macro blocks that behave like `#[rustc_gen]` blocks, but don't have compiler support for nice diagnostics or language support for the `?` operator. The `propane` crate uses the `Generator` trait from nightly and works mostly
like `#[rustc_gen]` would.

The standard library includes [`std::iter::from_fn`][], which can be used in
some cases, but as we saw in the example [above][motivation], often the
improvement over writing out a manual implementation of `Iterator` is limited.

[`std::iter::from_fn`]: https://doc.rust-lang.org/std/array/fn.from_fn.html
[`array::try_map`]: https://doc.rust-lang.org/std/primitive.array.html#method.try_map

## `return` statements `yield` one last element

Similarly to `try` blocks, trailing expressions could yield their element.

There would then be no way to terminate iteration as `return` statements would have to have a
value that is `yield`ed before terminating iteration.

We could do something magical where returning `()` terminates the iteration, so this code...

```rust
#[rustc_gen] fn foo() -> i32 {
    42
}
```

...could be a way to specify `std::iter::once(42)`. The issue I see with this is that this...

```rust
#[rustc_gen] fn foo() -> i32 {
    42; // note the semicolon
}
```

...would then not return a value.

Furthermore this would make it unclear what the behaviour of this...

```rust
#[rustc_gen] fn foo() {}
```

...is supposed to be, as it could be either `std::iter::once(())` or `std::iter::empty::<()>()`.

## Different syntax for `#[rustc_gen] fn`:

This RFC explicitly picks an attribute, as that has no conflicts with any other syntax, even within macros, and
does not pick any option that may influence how experimental users think about syntax.
There are many options to choose from, and we'll have to decide on one before stabilization.
Some options are:

```rust
fn foo(args) yield item
fn foo(args) yields item
fn foo(args) => item
fn* foo(args) -> item // or any of the `fn foo` variants for the item type
gen fn foo(args) // or any of the above variants for the item type
gen foo(args) // or any of the above variants for the item type
generator fn foo(args) // or any of the above variants for the item type
```

The design space here is very large, but either way, I propose to reserve the `gen` keyword.

# Prior art
[prior-art]: #prior-art

## Python

Python has equivalent functionality to `#[rustc_gen] fn`: any function that uses `yield` internally.
The main difference is that raising an exception automatically passes the exception outwards, instead of yielding an `Err()` element.

```python
def odd_dup(values):
    for value in values:
        if is_odd(value):
            yield value * 2
```

## C# ##

In C#, within an [`iterator`][c-sharp-iterators], the [`yield`][c-sharp-yield]
statement is used to either yield the next value or to stop iteration.  E.g.:

```csharp
IEnumerable<int> OddDupUntilNegative(IEnumerable<int> numbers)
{
    foreach (int n in numbers)
    {
        if (n < 0)
        {
            yield break;
        }
        else if (n % 2 == 1)
        {
            yield return n * 2;
        }
    }
}
```

Analogously with this RFC and with `async` blocks in Rust (but unlike `async
Task` in C#), execution of C# iterators does not start until they are
iterated.

[c-sharp-iterators]: https://learn.microsoft.com/en-us/dotnet/csharp/iterators
[c-sharp-yield]: https://learn.microsoft.com/en-us/dotnet/csharp/language-reference/statements/yield

# Unresolved questions
[unresolved-questions]: #unresolved-questions

## Keyword

Should we use `iter` as the keyword, as we're producing `Iterator`s?
We could use `gen` as proposed in this RFC and later extend its abilities to more powerful generators.

[playground](https://play.rust-lang.org/?version=nightly&mode=debug&edition=2021&gist=efeacb803158c2ebd57d43b4e606c0b5)

```rust
#![feature(generators)]
#![feature(iter_from_generator)]

fn main() {
    let mut it = std::iter::from_generator(|| {
        yield 1
    });

    assert_eq!(it.next(), Some(1));
    assert_eq!(it.next(), None);
    it.next(); // panics
}
```

## Panicking

What happens when `Iterator::next` is called again on a `#[rustc_gen]` block that panicked? Do we need to poison the iterator?

## Contextual keyword

Popular crates (like `rand`) have methods called [`gen`][Rng::gen]. If we forbid those, we are forcing those crates to make a major version bump when they update their edition, and we are requiring any users of those crates to use `r#gen` instead of `gen` when calling that method.

We could choose to use a contextual keyword and only forbid `gen` in:

* bindings
* field names (due to destructuring bindings)
* enum variants
* type names

This should avoid any parsing issues around `gen` followed by `{` in expressions.

[Rng::gen]: https://docs.rs/rand/latest/rand/trait.Rng.html#method.gen

## `Iterator::size_hint`

Should we try to compute a conservative `size_hint`? This will reveal information from the body of a generator,
but at least for simple cases users will likely expect `size_hint` to not just be the default.

## Implement other `Iterator` traits.

Is there a possibility for implementing traits like `DoubleEndedIterator`, `ExactSizeIterator` at all?

# Future possibilities
[future-possibilities]: #future-possibilities

## `yield from` (forwarding operation)

Python has the ability to `yield from` an iterator.
Effectively this is syntax sugar for looping over all elements of the iterator and yielding them individually.
There are infinite options to choose from if we want such a feature, so I'm listing general ideas:

### Do nothing, just use loops

```rust
for x in iter {
    yield x
}
```

### Language support

We could do something like postfix `yield`:

```rust
iter.yield
```

Or we could use an entirely new keyword.

### stdlib macro

We could add a macro to the standard library and prelude.
The macro would expand to a `for` loop + `yield`.

```rust
yield_all!(iter)
```

## Complete `Generator` support

We already have a `Generator` trait on nightly that is more powerful than the `Iterator`
API could possibly be:

1. It uses `Pin<&mut Self>`, allowing self-references in the generator across yield points.
2. It has arguments (`yield` returns the arguments passed to it in the subsequent invocations).

Similar to the ideas around `async` closures,
I think we could argue for `Generators` to be `#[rustc_gen]` closures while `#[rustc_gen]` blocks are a simpler concept that has no arguments and only captures variables.

Either way, support for full `Generator`s should be discussed and implemented separately,
as there are many more open questions around them beyond a simpler way to write `Iterator`s.

## `async` interactions

We could support using `await` in `#[rustc_gen] async` blocks, similar to how we support `?` being used within `#[rustc_gen]` blocks.
We'd have similar limitations holding references held across `await` points as we do have with `yield` points.
The solution space for `#[rustc_gen] async` is large enough that I will not explore it here.
This RFC's design is forward compatible with anything we decide on.

At present it is only possible to have a `#[rustc_gen]` block yield futures, but not `await` within it, similar to how
you cannot write iterators that `await`, but that return futures from `next`.

## Self-referential `#[rustc_gen]` blocks

We can allow `#[rustc_gen]` blocks to hold borrows across `yield` points in the future.

There are a few options forward (though this list is probably not complete):

* Add a separate trait for pinned iteration that is also usable with `#[rustc_gen]` and `for`.
    * *Downside*: We would have very similar traits for the same thing.
* Backward-compatibly add a way to change the argument type of `Iterator::next`.
    * *Downside*: It's unclear whether this is possible.
* Implement `Iterator` for `Pin<&mut G>` instead of for `G` directly (whatever `G` is here, but it could be a `#[rustc_gen]` block).
    * *Downside*: The thing being iterated over must now be pinned for the entire iteration, instead of for each invocation of `next`.

This RFC is forward compatible with any such designs, so I will not explore it here.

## `try` interactions

We could allow `#[rustc_gen] try fn foo() -> i32` to mean something akin to `#[rustc_gen] fn foo() -> Result<i32, E>`.
Whatever we do here, it should mirror whatever `try fn` means in the future.
