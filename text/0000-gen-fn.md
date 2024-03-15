- Feature Name: `gen-fn`
- Start Date: 2023-10-10
- RFC PR: [rust-lang/rfcs#3513](https://github.com/rust-lang/rfcs/pull/3513)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC reserves the `gen` keyword in the Rust 2024 edition for generators and adds `gen { .. }` blocks to the language.  Similar to how `async` blocks produce values that can be awaited with `.await`, `gen` blocks produce values that can be iterated over using `for` loops.

# Motivation
[motivation]: #motivation

The main motivation of this RFC is to reserve a new keyword in the 2024
edition. We will discuss the semantic questions of generators in this
document, but we do not have to settle them with this RFC. We'll describe
current thinking on the semantics, but some questions will be left open to be
answered at a later time after we gain more experience with the
implementation.

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
    gen {
        for value in values {
            if value.is_odd() {
                yield value * 2;
            }
        }
    }.into()
}
```

Iterators created with `gen` return `None` once they `return` (implicitly at the end of the scope or explicitly with `return`).
`gen` iterators are fused, so after returning `None` once, they will keep returning `None` forever.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## New keyword

Starting in the 2024 edition, `gen` is a keyword that cannot be used for naming any items or bindings.
This means during the migration to the 2024 edition, all variables, functions, modules, types, etc. named `gen` must be renamed
or be referred to via `k#gen`.

## Returning/finishing an iterator

`gen` blocks must diverge or return the unit type.
Specifically, the trailing expression must be of the unit or `!` type, and any `return` statements in the block must either be given no argument at all or given an argument of the unit or `!` type.

### Diverging iterators

For example, a `gen` block that produces the infinite sequence `0, 1, 0, 1, 0, 1, ...`, will never return `None`
from `next`, and only drop its captured data when the iterator is dropped:

```rust
gen {
    loop {
        yield 0;
        yield 1;
    }
}
```

If a `gen` block panics, the behavior is very similar to `return`, except that `next` unwinds instead of returning `None`.

## Error handling

Within `gen` blocks, the `?` operator desugars as follows.  When its
argument returns a value indicating "do not short circuit"
(e.g. `Option::Some(..)`, `Result::Ok(..)`, `ControlFlow::Continue(..)`), that
value becomes the result of the expression as usual.  When its argument
returns a value indicating that short-circuiting is desired
(e.g. `Option::None`, `Result::Err(..)`, `ControlFlow::Break(..)`), the value
is first yielded (after being converted by `From::from` as usual), then the
block returns immediately.

Even when `?` is used within a `gen` block, the block must return a
value of type unit or `!`.  That is, it does not return a value of `Some(..)`,
`Ok(..)`, or `Continue(..)` as other such blocks might.

However, note that when `?` is used within a `gen` block, all `yield`
statements will need to be given an argument of a compatible type.  For
example, if `None?` is used in an expression, then all `yield` statements will
need to be given arguments of type `Option`.

## Fusing

Iterators produced by `gen` keep returning `None` when invoked again after they have returned `None` once.
They do not implement `FusedIterator`, as that is not a language item, but may implement it in the future.

## Holding borrows across yields

Since the `Iterator::next` method takes `&mut self` instead of `Pin<&mut self>`, we cannot create self-referential
`gen` blocks (but see the open questions). Self-referential `gen` blocks occur when you hold a borrow to a local variable across a yield point:

```rust
gen {
    let x = vec![1, 2, 3, 4];
    let mut y = x.iter();
    yield y.next();
    yield Some(42);
    yield y.next();
}
```

or as a more common example:

```rust
gen {
    let x = vec![1, 2, 3, 4];
    for z in x.iter() {
        yield z * 2;
    }
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation
## New keyword

In the 2024 edition we reserve `gen` as a keyword. Previous editions will use `r#gen` to get the same features.

## Error handling

`foo?` in `gen` blocks will stop iteration after the first error by desugaring to:

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

This feature is mostly implemented via existing coroutines, though there are some special cases.

### `gen` blocks

`gen` blocks are the same as an unstable coroutine...

* ...without arguments,
* ...with an additional check forbidding holding borrows across `yield` points,
* ...and with an automatic implementation of a trait allowing the type to be used in `for` loops (see the open questions).
* ...do not panic if invoked again after returning

# Drawbacks
[drawbacks]: #drawbacks

It's another language feature for something that can already be written entirely in user code.

In contrast to `Coroutine`s (currently unstable), `gen` blocks that produce iterators cannot hold references across `yield` points.
See [`from_generator`][] which has an `Unpin` bound on the generator it takes to produce an `Iterator`.

The `gen` keyword causes some fallout in the community, mostly around the `rand` crate, which has `gen` methods on its traits.

[`from_generator`]: https://doc.rust-lang.org/std/iter/fn.from_generator.html

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives
## Keyword

We could use `iter` as the keyword.
I prefer `iter` because I connect generators with a more powerful scheme than plain `Iterator`s.
The unstable `Coroutine` trait (which was previously called `Generator`) can do everything that `iter` blocks and `async` blocks can do and more.
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
`genawaiter` works on stable and provides `gen!` macro blocks that behave like `gen` blocks, but don't have compiler support for nice diagnostics or language support for the `?` operator. The `propane` crate uses the `Coroutine` trait from nightly and works mostly
like `gen` would.

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
fn foo() -> impl Iterator<Item = i32> {
    gen { 42 }
}
```

...could be a way to specify `std::iter::once(42)`. The issue I see with this is that this...

```rust
fn foo() -> impl Iterator<Item = i32> {
    gen { 42; } // note the semicolon
}
```

...would then not return a value.

Furthermore this would make it unclear what the behaviour of this...

```rust
fn foo() -> impl Iterator<Item = ()> { gen {} }
```

...is supposed to be, as it could be either `std::iter::once(())` or `std::iter::empty::<()>()`.

# Prior art
[prior-art]: #prior-art

## CLU, Alphard

The idea of generators that yield their values goes back at least as far as
the Alphard language from circa 1975 (see ["Alphard: Form and
Content"][alphard], Mary Shaw, 1981). This was later refined into the idea of
iterators in the CLU language (see ["A History of CLU"][clu-history], Barbara
Liskov, 1992 and ["CLU Reference Manual"][clu-ref], Liskov et al., 1979).

The CLU language opened an iterator context with the `iter` keyword and
produced values with `yield` statements. E.g.:

```
odds = iter () yields (int)
  x: int := 1
  while x <= 20 do
    yield x
    x := x + 2
  end
end odds
```

[alphard]: https://web.archive.org/web/20150926014020/http://repository.cmu.edu/cgi/viewcontent.cgi?article=1868&context=isr
[clu-history]: https://web.archive.org/web/20030917041834/http://www.lcs.mit.edu/publications/pubs/pdf/MIT-LCS-TR-561.pdf
[clu-ref]: https://web.archive.org/web/20211105171453/https://pmg.csail.mit.edu/ftp.lcs.mit.edu/pub/pclu/CLU/3.Documents/MIT-LCS-TR-225.pdf

## Icon

In [Icon][icon-language] (introduced circa 1977), generators are woven deeply
into the language, and any function can return a sequence of values. When done
explicitly, the `suspend` keyword is used. E.g.:

```
procedure range(i, j)
  while i < j do {
    suspend i
    i +:= 1
  }
  fail
end
```

[icon-language]: https://web.archive.org/web/20230721102710/https://www2.cs.arizona.edu/icon/ftp/doc/lb1up.pdf

## Python

In Python, any function that contains a `yield` statement returns a
generator. E.g.:

```python
def odd_dup(xs):
  for x in xs:
    if x % 2 == 1:
      yield x * 2
```

## ECMAScript / JavaScript

In JavaScript, `yield` can be used within [`function*`][javascript-function*]
generator functions. E.g.:

```javascript
function* oddDupUntilNegative(xs) {
  for (const x of xs) {
    if (x < 0) {
      return;
    } else if (x % 2 == 1) {
      yield x * 2;
    }
  }
}
```

These generator functions are general coroutines. `yield` forms an expression
that returns the value passed to `next`. E.g.:

```javascript
function* dup(x) {
  while (true) {
    x = yield x * 2;
  }
}

const g = dup(2);
console.assert(g.next().value === 4);
console.assert(g.next(3).value === 6);
```

[javascript-function*]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Statements/function*

## Ruby

In Ruby, `yield` can be used with the [`Enumerator`][ruby-enumerator] class to
implement an iterator. E.g.:

```ruby
def odd_dup_until_negative xs
  Enumerator.new do |y|
    xs.each do |x|
      if x < 0
        return
      elsif x % 2 == 1
        y.yield x * 2
      end
    end
  end
end
```

Ruby also uses `yield` for a general coroutine mechanism with the
[`Fiber`][ruby-fiber] class. E.g.:

```ruby
def dup
  Fiber.new do |x|
    while true
      x = Fiber.yield x * 2
    end
  end
end

g = dup
4 == (g.resume 2)
6 == (g.resume 3)
```

[ruby-enumerator]: https://ruby-doc.org/3.2.2/Enumerator.html
[ruby-fiber]: https://ruby-doc.org/3.2.2/Fiber.html

## Kotlin

In Kotlin, a lazy [`Sequence`][kotlin-sequences] can be built using `sequence`
expressions and `yield`. E.g.:

```kotlin
fun oddDup(xs: Iterable<Int>): Sequence<Int> {
    return sequence {
        for (x in xs) {
            if (x % 2 == 1) {
                yield(x * 2);
            }
        }
    };
}

fun main() {
    for (x in oddDup(listOf(1, 2, 3, 4, 5))) {
        println(x);
    }
}
```

[kotlin-sequences]: https://kotlinlang.org/docs/sequences.html#from-elements

## Swift

In Swift, [`AsyncStream`][swift-asyncstream] is used with `yield` to produce
asynchronous generators. E.g.:

```swift
import Foundation

let sequence = AsyncStream { k in
    for x in 0..<20 {
        if x % 2 == 1 {
            k.yield(x * 2)
        }
    }
    k.finish()
}

let semaphore = DispatchSemaphore(value: 0)
Task {
    for await elem in sequence {
        print(elem)
    }
    semaphore.signal()
}
semaphore.wait()
```

Synchronous generators are not yet available in Swift, but [may
be][swift-sync-gen] something they are planning.

[swift-asyncstream]: https://developer.apple.com/documentation/swift/asyncstream
[swift-sync-gen]: https://forums.swift.org/t/is-it-possible-to-make-an-iterator-that-yelds/53995/7

## C# ##

In C#, within an [`iterator`][csharp-iterators], the [`yield`][csharp-yield]
statement is used to either yield the next value or to stop iteration. E.g.:

```csharp
IEnumerable<int> OddDupUntilNegative(IEnumerable<int> xs)
{
    foreach (int x in xs)
    {
        if (x < 0)
        {
            yield break;
        }
        else if (x % 2 == 1)
        {
            yield return x * 2;
        }
    }
}
```

Analogously with this RFC and with `async` blocks in Rust (but unlike `async
Task` in C#), execution of C# iterators does not start until they are
iterated.

[csharp-iterators]: https://learn.microsoft.com/en-us/dotnet/csharp/iterators
[csharp-yield]: https://learn.microsoft.com/en-us/dotnet/csharp/language-reference/statements/yield

## D

In D, `yield` is used when constructing a
[`Generator`][dlang-generators]. E.g.:

```dlang
import std.concurrency;
import std.stdio: writefln;

auto odd_dup(int[] xs) {
    return new Generator!int({
        foreach(x; xs) {
            if (x % 2 == 1) {
                yield(x * 2);
            }
        }
    });
}

void main() {
    auto xs = odd_dup([1, 2, 3, 4, 5]);
    foreach (x; xs) {
        writefln("%d", x);
    }
}
```

As in Ruby, generators in D are built on top of a more general
[`Fiber`][dlang-fibers] class that also uses `yield`.

[dlang-generators]: https://dlang.org/library/std/concurrency/generator.html
[dlang-fibers]: https://dlang.org/library/core/thread/fiber/fiber.html

## Dart

In Dart, there are both synchronous and asynchronous [generator
functions][dart-generators].  Synchronous generator functions return an
`Iteratable`. E.g.:

```dart
Iterable<int> oddDup(Iterable<int> xs) sync* {
    for (final x in xs) {
        if (x % 2 == 1) {
            yield x * 2;
        }
    }
}

void main() {
    oddDup(List<int>.generate(20, (x) => x + 1)).forEach(print);
}
```

Asynchronous generator functions return a `Stream` object. E.g.:

```dart
Stream<int> oddDup(Iterable<int> xs) async* {
    for (final x in xs) {
        if (x % 2 == 1) {
            yield x * 2;
        }
    }
}

void main() {
  oddDup(List<int>.generate(20, (x) => x + 1)).forEach(print);
}
```

[dart-generators]: https://dart.dev/language/functions#generators

## F# ##

In F#, generators can be expressed with [sequence
expressions][fsharp-sequences] using `yield`. E.g.:

```fsharp
let oddDup xs = seq {
  for x in xs do
    if x % 2 = 1 then
      yield x * 2 }

for x in oddDup (seq { 1 .. 20 }) do
  printfn "%d" x
```

[fsharp-sequences]: https://learn.microsoft.com/en-us/dotnet/fsharp/language-reference/sequences

## Racket

In Racket, generators can be built using [`generator`][racket-generators] and
`yield`. E.g.:

```racket
#lang racket
(require racket/generator)

(define (odd-dup xs)
  (generator ()
    (for ([x xs])
      (when (odd? x)
        (yield (* 2 x))))))

(define g (odd-dup '(1 2 3 4 5)))
(= (g) 2)
(= (g) 6)
(= (g) 10)
```

Note that because of the expressive power of [`call/cc`][racket-callcc] (and
continuations in general), generators can be written in Racket as a normal
library.

[racket-callcc]: https://docs.racket-lang.org/reference/cont.html
[racket-generators]: https://docs.racket-lang.org/reference/Generators.html

## Haskell, Idris, Clean, etc.

In [Haskell][] (and in similar languages such as [Idris][idris-lang],
[Clean][clean-lang], etc.), all functions are lazy unless specially annotated.
Consequently, Haskell does not need a special `yield` operator. Any function
can be a generator by recursively building a list of elements that will be
lazily returned one at a time. E.g.:

```haskell
oddDup :: (Integral x) => [x] -> [x]
oddDup [] = []
oddDup (x:xs)
  | odd x = x * 2 : oddDup xs
  | otherwise = oddDup xs

main :: IO ()
main = putStrLn $ show $ take 5 $ oddDup [1..20]
```

[haskell]: https://www.haskell.org/
[clean-lang]: https://wiki.clean.cs.ru.nl/Clean
[idris-lang]: https://www.idris-lang.org/

# Unresolved questions
[unresolved-questions]: #unresolved-questions

## Whether to implement `Iterator`

There may be benefits to having the type returned by `gen` blocks *not* implement `Iterator` directly.  Instead, these blocks would return a type that implements either `IntoIterator` or a new `IntoGenerator` trait.  Such a design could leave us more appealing options for supporting self-referential `gen` blocks.  We leave this as an open question.

## Self-referential `gen` blocks

We can allow `gen` blocks to hold borrows across `yield` points. Should this be part of the initial stabilization?

There are a few options for how to do this, either before or after stabilization (though this list is probably not complete):

* Add a separate trait for pinned iteration that is also usable with `gen` and `for`.
    * *Downside*: We would have very similar traits for the same thing.
* Backward-compatibly add a way to change the argument type of `Iterator::next`.
    * *Downside*: It's unclear whether this is possible.
* Implement `Iterator` for `Pin<&mut G>` instead of for `G` directly (whatever `G` is here, but it could be a `gen` block).
    * *Downside*: The thing being iterated over must now be pinned for the entire iteration, instead of for each invocation of `next`.
    * *Downside*: Now the `next` method takes a double-indirection as an argument `&mut Pin<&mut G>`, which may not optimize well sometimes.

This RFC is forward compatible with any such designs. However, if we were to stabilize `gen` blocks that could not hold borrows across `yield` points, this would be a serious usability limitation that users might find surprising. Consequently, whether we should choose to address this before stabilization is an open question.

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
It is backwards compatible to later add support for opportunistically implementing `size_hint`.

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

## Complete `Coroutine` support

We already have a `Coroutine` trait on nightly (previously called `Generator`) that is more powerful than the `Iterator`
API could possibly be:

1. It uses `Pin<&mut Self>`, allowing self-references across yield points.
2. It has arguments (`yield` returns the arguments passed to it in the subsequent invocations).

Similar to the ideas around `async` closures,
I think we could argue for coroutines to be `gen` closures while `gen` blocks are a simpler concept that has no arguments and only captures variables.

Either way, support for full coroutines should be discussed and implemented separately,
as there are many more open questions around them beyond a simpler way to write `Iterator`s.

## `async` interactions

We could support using `await` in `gen async` blocks, similar to how we support `?` being used within `gen` blocks.
We'd have similar limitations holding references held across `await` points as we do have with `yield` points.
The solution space for `gen async` is large enough that I will not explore it here.
This RFC's design is forward compatible with anything we decide on.

At present it is only possible to have a `gen` block yield futures, but not `await` within it, similar to how
you cannot write iterators that `await`, but that return futures from `next`.

## `try` interactions

We could allow `gen try fn foo() -> i32` to mean something akin to `gen fn foo() -> Result<i32, E>`.
Whatever we do here, it should mirror whatever `try fn` means in the future.

## `gen fn`:

This does not introduce `gen fn`. The syntax design for them is fairly large
and there are open questions around the difference between returning or yielding a type.

```rust
fn foo(args) yield item
fn foo(args) yields item
fn foo(args) => item
fn* foo(args) -> item // or any of the `fn foo` variants for the item type
gen fn foo(args) // or any of the above variants for the item type
gen foo(args) // or any of the above variants for the item type
generator fn foo(args) // or any of the above variants for the item type
```
