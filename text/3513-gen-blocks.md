- Feature Name: `gen_blocks`
- Start Date: 2023-10-10
- RFC PR: [rust-lang/rfcs#3513](https://github.com/rust-lang/rfcs/pull/3513)
- Tracking Issue: [rust-lang/rust#117078](https://github.com/rust-lang/rust/issues/117078)

# Summary
[summary]: #summary

This RFC reserves the `gen` keyword in the Rust 2024 edition for generators and adds `gen { .. }` blocks to the language.  Similar to how `async` blocks produce values that can be awaited with `.await`, `gen` blocks produce values that can be iterated over with `for`.

# Motivation
[motivation]: #motivation

Writing iterators manually can be painful.  Many iterators can be written by chaining together iterator combinators, but some need to be written with a manual implementation of `Iterator`.  This can push people to avoid iterators and do worse things such as writing loops that eagerly store values to mutable state.  With `gen` blocks, we can now write a simple `for` loop and still get a lazy iterator of values.

By way of example, consider these alternate ways of expressing [run-length encoding][]:

[run-length encoding]: https://en.wikipedia.org/wiki/Run-length_encoding

```rust
// This example uses `gen` blocks, introduced in this RFC.
fn rl_encode<I: IntoIterator<Item = u8>>(
    xs: I,
) -> impl Iterator<Item = u8> {
    gen {
        let mut xs = xs.into_iter();
        let (Some(mut cur), mut n) = (xs.next(), 0) else { return };
        for x in xs {
            if x == cur && n < u8::MAX {
                n += 1;
            } else {
                yield n; yield cur;
                (cur, n) = (x, 0);
            }
        }
        yield n; yield cur;
    }.into_iter()
}

// This example uses a manual implementation of `Iterator`.
fn rl_encode<I: IntoIterator<Item = u8>>(
    xs: I,
) -> impl Iterator<Item = u8> {
    struct RlEncode<I: IntoIterator<Item = u8>> {
        into_xs: Option<I>,
        xs: Option<<I as IntoIterator>::IntoIter>,
        cur: Option<<I as IntoIterator>::Item>,
        n: u8,
        yield_x: Option<<I as IntoIterator>::Item>,
    }
    impl<I: IntoIterator<Item = u8>> Iterator for RlEncode<I> {
        type Item = u8;
        fn next(&mut self) -> Option<Self::Item> {
            let xs = self.xs.get_or_insert_with(|| unsafe {
                self.into_xs.take().unwrap_unchecked().into_iter()
            });
            if let Some(x) = self.yield_x.take() {
                return Some(x);
            }
            loop {
                match (xs.next(), self.cur) {
                    (Some(x), Some(cx))
                        if x == cx && self.n < u8::MAX => self.n += 1,
                    (Some(x), Some(cx)) => {
                        let n_ = self.n;
                        (self.cur, self.n) = (Some(x), 0);
                        self.yield_x = Some(cx);
                        return Some(n_);
                    }
                    (Some(x), None) => {
                        (self.cur, self.n) = (Some(x), 0);
                    }
                    (None, Some(cx)) => {
                        self.cur = None;
                        self.yield_x = Some(cx);
                        return Some(self.n);
                    }
                    (None, None) => return None,
                }
            }
        }
    }
    RlEncode {
        into_xs: Some(xs), xs: None, cur: None, n: 0, yield_x: None,
    }
}

// This example uses `iter::from_fn`.
fn rl_encode<I: IntoIterator<Item = u8>>(
    xs: I,
) -> impl Iterator<Item = u8> {
    let (mut cur, mut n, mut yield_x) = (None, 0, None);
    let (mut into_xs, mut xs) = (Some(xs), None);
    core::iter::from_fn(move || loop {
        let xs = xs.get_or_insert_with(|| unsafe {
            into_xs.take().unwrap_unchecked().into_iter()
        });
        if let Some(x) = yield_x.take() {
            return Some(x);
        }
        match (xs.next(), cur) {
            (Some(x), Some(cx)) if x == cx && n < u8::MAX => n += 1,
            (Some(x), Some(cx)) => {
                let n_ = n;
                (cur, n) = (Some(x), 0);
                yield_x = Some(cx);
                return Some(n_);
            }
            (Some(x), None) => (cur, n) = (Some(x), 0),
            (None, Some(cx)) => {
                cur = None;
                yield_x = Some(cx);
                return Some(n);
            }
            (None, None) => return None,
        }
    })
}
```

Iterators created with `gen` blocks return `None` from `next` once the `gen` block has returned (either implicitly at the end of the scope or explicitly with the `return` statement) and are fused (after `next` returns `None` once, it will keep returning `None` forever).

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## New keyword

Starting in the 2024 edition, `gen` is a keyword that cannot be used for naming any items or bindings.  This means during the migration to the 2024 edition, all variables, functions, modules, types, etc. named `gen` must be renamed or be referred to via `r#gen`.

## Return value

`gen` blocks must diverge or return the unit type.  Specifically, the trailing expression must be of the unit or `!` type, and any `return` statements in the block must either be given no argument at all or given an argument of the unit or `!` type.

### Diverging

For example, a `gen` block that produces the infinite sequence `0, 1, 0, 1, 0, 1, ..` will never return `None` from `next` and will only drop its captured state when the iterator is dropped.  E.g.:

```rust
gen {
    loop {
        yield 0;
        yield 1;
    }
}
```

If a `gen` block panics, the behavior is similar to that of `return`, except that the call to `next` unwinds instead of returning `None`.

## Error handling

Within `gen` blocks, the `?` operator behaves as follows.  When its argument is a value indicating "do not short circuit" (e.g. `Option::Some(..)`, `Result::Ok(..)`, `ControlFlow::Continue(..)`), that value becomes the result of the expression as usual.  When its argument is a value indicating that short-circuiting is desired (e.g. `Option::None`, `Result::Err(..)`, `ControlFlow::Break(..)`), the value is first yielded (after being converted by `FromResidual::from_residual` as usual), then the block returns immediately.

Even when `?` is used within a `gen` block, the block must return a value of type unit or `!`.  That is, it does not return a value of `Some(..)`, `Ok(..)`, or `Continue(..)` as other such blocks might.

However, note that when `?` is used within a `gen` block, all `yield` statements will need to be given an argument of a compatible type.  For example, if `None?` is used in an expression, then all `yield` statements will need to be given arguments of some `Option` type (or of the `!` type) .

## Fusing

Iterators produced by `gen` return `None` from `next` repeatedly after having once returned `None` from `next`.  However, they do not implement `FusedIterator`, as that is not a language item, but may do so in the future (see the future possibilities).

## Holding borrows across yields

Since the `Iterator::next` method takes `&mut self` instead of `Pin<&mut Self>`, we cannot create self-referential `gen` blocks without taking other steps (see the open questions).  Self-referential `gen` blocks occur when holding a borrow to a local variable across a yield point.  E.g.:

```rust
gen {
    let xs = vec![1, 2, 3, 4];
    for x in xs.iter() {
        yield x * 2;
    }
    //~^ ERROR borrow may still be in use when `gen` block yields
}
```

This may in fact be a severe and surprising limitation, and whether we should take the steps necessary to address this before stabilization is left as an open question.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## New keyword

In the 2024 edition we reserve `gen` as a keyword.  Rust 2021 will use `k#gen` to access the same feature.  What to do about earlier editions is left as an open question.

## Error handling

`foo?` in `gen` blocks will stop iteration after the first error as if it desugared to:

```rust
match foo.branch() {
    ControlFlow::Break(err) => {
        yield <_ as FromResidual>::from_residual(err);
        return
    },
    ControlFlow::Continue(val) => val,
}
```

## Implementation

This feature is mostly implemented via existing coroutines, though there are some special cases.

We could say that `gen` blocks are the same as unstable coroutines...

- ...without arguments,
- ...with an additional check forbidding holding borrows across `yield` points,
- ...with an automatic implementation of a trait allowing the type to be used in `for` loops (see the open questions),
- ...that do not panic if invoked again after returning.

# Drawbacks
[drawbacks]: #drawbacks

The main drawback is that this adds a language feature for something that can already be written entirely (if more painfully) in user code.

In contrast to full coroutines (currently unstable), `gen` blocks cannot hold references across `yield` points (see the open questions, and see [`from_coroutine`][] which has an `Unpin` bound on the generator it takes to produce an `Iterator`).

Reserving  the `gen` keyword will require some adaptation from the ecosystem mostly due to the `rand` crate which has `gen` methods on its traits.

[`from_coroutine`]: https://doc.rust-lang.org/1.77.0/core/iter/fn.from_coroutine.html

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Keyword

We could use `iter` as the keyword.

Due to unstable coroutines having originally been named "generators" within `rustc` and nightly Rust, some of the authors connect "generators" with this more powerful control flow construct that can do everything that `gen` blocks and `async` blocks can do and more.

There is some appeal in syntactically connecting the `Iterator` trait with `iter` blocks, but that would require us to carve out many exceptions for this keyword as `iter` is widely used for module names and method names, not just in the ecosystem, but also in `libstd` and `libcore`.  To what degree this might be worse than the situation for the `gen` keyword we leave as an open question.

Not using the `gen` keyword now would leave open the possibility of using the `gen` keyword in the future for a kind of block that might produce types that implement a more powerful `Generator` trait (perhaps one that takes `self` by pinned reference) or that implement `Coroutine`.

## Do not do this

### Add more combinators

One alternative is to instead add more helper methods to `Iterator`.

However, it is already difficult for new users of Rust to become familiar with all of the many existing methods on `Iterator`.  Further, some of the new methods we might want would need to be quite generic (similar to [`array::try_map`][]).

[`array::try_map`]: https://doc.rust-lang.org/1.77.0/std/primitive.array.html#method.try_map

### Use crates

We could suggest that people use crates like [`genawaiter`][], [`propane`][], or [`iterator_item`][] instead.  `genawaiter` works on stable Rust and provides `gen!` macro blocks that behave like `gen` blocks, but it doesn't have compiler support for nice diagnostics or language support for the `?` operator.  The `propane` and `iterator_item` crates use the `Coroutine` trait from nightly and work mostly like `gen` would (but therefore require unstable Rust).

[`genawaiter`]: https://crates.io/crates/genawaiter
[`propane`]: https://crates.io/crates/propane
[`iterator_item`]: https://crates.io/crates/iterator_item

### Use `iter::from_fn`

The standard library includes [`std::iter::from_fn`][] which can be used in some cases, but as we saw in the [motivating example][motivation], often the improvement over writing out a manual implementation of `Iterator` is limited.

[`std::iter::from_fn`]: https://doc.rust-lang.org/1.77.0/std/array/fn.from_fn.html

## Have trailing expressions yield one last element

Trailing expressions could have a meaningful value that is yielded before terminating iteration.

However, if we were to do this, we would need to add some other way to immediately terminate iteration without yielding a value.  We could do something magical where returning `()` terminates the iteration, so that this code...

```rust
fn foo() -> impl Iterator<Item = i32> {
    gen { 42 }
}
```

...could be a way to specify `std::iter::once(42)`.  However, then logically this code...

```rust
fn foo() -> impl Iterator<Item = i32> {
    gen { 42; } // Note the semicolon.
}
```

...would then not return a value due to the semicolon.

Further, this would make it unclear what the behavior of this...

```rust
fn foo() -> impl Iterator<Item = ()> { gen {} }
```

...should be, as it could reasonably be either `std::iter::once(())` or `std::iter::empty::<()>()`.

Note that, under this RFC, because `return` within `gen` blocks accepts an argument of type `()` and `yield` within `gen` blocks returns the `()` type, it is possible to yield one last element concisely with `return yield EXPR`.

# Prior art
[prior-art]: #prior-art

## CLU, Alphard

The idea of generators that yield their values goes back at least as far as the Alphard language from circa 1975 (see ["Alphard: Form and Content"][alphard], Mary Shaw, 1981).  This was later refined into the idea of iterators in the CLU language (see ["A History of CLU"][clu-history], Barbara Liskov, 1992 and ["CLU Reference Manual"][clu-ref], Liskov et al., 1979).

The CLU language opened an iterator context with the `iter` keyword and produced values with `yield` statements.  E.g.:

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

In [Icon][icon-language] (introduced circa 1977), generators are woven deeply into the language, and any function can return a sequence of values.  When done explicitly, the `suspend` keyword is used.  E.g.:

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

In Python, any function that contains a `yield` statement returns a generator.  E.g.:

```python
def odd_dup(xs):
  for x in xs:
    if x % 2 == 1:
      yield x * 2
```

## ECMAScript / JavaScript

In JavaScript, `yield` can be used within [`function*`][javascript-function*] generator functions.  E.g.:

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

These generator functions are general coroutines.  `yield` forms an expression that returns the value passed to `next`.  E.g.:

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

In Ruby, `yield` can be used with the [`Enumerator`][ruby-enumerator] class to implement an iterator.  E.g.:

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

Ruby also uses `yield` for a general coroutine mechanism with the [`Fiber`][ruby-fiber] class.  E.g.:

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

In Kotlin, a lazy [`Sequence`][kotlin-sequences] can be built using `sequence` expressions and `yield`.  E.g.:

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

In Swift, [`AsyncStream`][swift-asyncstream] is used with `yield` to produce asynchronous generators.  E.g.:

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

Synchronous generators are not yet available in Swift, but [may be][swift-sync-gen] something they are planning.

[swift-asyncstream]: https://developer.apple.com/documentation/swift/asyncstream
[swift-sync-gen]: https://forums.swift.org/t/is-it-possible-to-make-an-iterator-that-yelds/53995/7

## C# ##

In C#, within an [`iterator`][csharp-iterators], the [`yield`][csharp-yield] statement is used to either yield the next value or to stop iteration.  E.g.:

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

Analogously with this RFC and with `async` blocks in Rust (but unlike `async Task` in C#), execution of C# iterators does not start until they are iterated.

[csharp-iterators]: https://learn.microsoft.com/en-us/dotnet/csharp/iterators
[csharp-yield]: https://learn.microsoft.com/en-us/dotnet/csharp/language-reference/statements/yield

## D

In D, `yield` is used when constructing a [`Generator`][dlang-generators].  E.g.:

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

As in Ruby, generators in D are built on top of a more general [`Fiber`][dlang-fibers] class that also uses `yield`.

[dlang-generators]: https://dlang.org/library/std/concurrency/generator.html
[dlang-fibers]: https://dlang.org/library/core/thread/fiber/fiber.html

## Dart

In Dart, there are both synchronous and asynchronous [generator functions][dart-generators].  Synchronous generator functions return an `Iteratable`.  E.g.:

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

Asynchronous generator functions return a `Stream` object.  E.g.:

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

In F#, generators can be expressed with [sequence expressions][fsharp-sequences] using `yield`.  E.g.:

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

In Racket, generators can be built using [`generator`][racket-generators] and `yield`.  E.g.:

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

Note that because of the expressive power of [`call/cc`][racket-callcc] (and continuations in general), generators can be written in Racket (and in other Scheme dialects) as a normal library.

[racket-callcc]: https://docs.racket-lang.org/reference/cont.html
[racket-generators]: https://docs.racket-lang.org/reference/Generators.html

## Haskell, Idris, Clean, etc.

In [Haskell][] (and in similar languages such as [Idris][idris-lang], [Clean][clean-lang], etc.), all functions are lazy unless specially annotated.  Consequently, Haskell does not need a special `yield` operator.  Any function can be a generator by recursively building a list of elements that will be lazily returned one at a time.  E.g.:

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

## Koka

The [Koka][] language, by contrast, does not lean on laziness.  Instead, like Scheme, Koka provides powerful general control flow constructs from which generators, async, coroutines, and other such things fall out naturally.  Unlike Scheme, these powerful control flow constructs are *typed* and are called effect handlers.  E.g.:

```koka
effect yield<a>
  fun yield(x : a) : ()

fun odd_dup(xs : list<int>) : yield<int> ()
  match xs
    Cons(x,xx) ->
      if x % 2 == 1 then
        yield(x * 2)
      odd_dup(xx)
    Nil -> ()

fun main() : console ()
  with fun yield(i : int)
    println(i.show)
  list(1,20).odd_dup
```

Note that there is no library being used here and that `yield` is not a keyword or feature of the language.  In Koka, the code above is all that is needed to express generators.

[koka]: https://koka-lang.github.io/

## Rust

In Rust, `async` blocks are built on top of the coroutine transformation.  Using a no-op `Waker`, it's possible to expose this transformation.  With that, we can build generators.  Without the assistance of macros, the result looks like this:

```rust
let odd_dup = |xs| {
    Gen::new(async move |mut y| {
        for x in xs {
            if x % 2 == 1 {
                y.r#yield(x * 2).await;
            }
        }
    })
};

let odd_dup = pin!(odd_dup(1u8..20));
let odd_dup = odd_dup.init();

for (i, x) in odd_dup.enumerate() {
    assert_eq!((i as u8 * 2 + 1) * 2, x);
}
```

Crates such as [`genawaiter`][] use this technique.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

## Whether to implement `Iterator`

There may be benefits to having the type returned by `gen` blocks *not* implement `Iterator` directly.  Instead, these blocks would return a type that implements either `IntoIterator` or a new `IntoGenerator` trait.  Such a design could leave us more appealing options for supporting self-referential `gen` blocks.  We leave this as an open question.

## Self-referential `gen` blocks

We can allow `gen` blocks to hold borrows across `yield` points.  Should this be part of the initial stabilization?

Here are some options for how we might do this, either before or after stabilization:

- Add a separate trait for pinned iteration that is also usable with `gen` and `for`.
    - *Downside*: We would have very similar traits for the same thing.
- Backward compatibly add a way to change the argument type of `Iterator::next`.
    - *Downside*: It's unclear whether this is possible.
- Implement `Iterator` for `Pin<&mut G>` instead of for `G` directly (for some type `G` that can be produced by `gen` blocks).
    - *Downside*: The `next` method would take a double indirected reference of the form `&mut Pin<&mut G>` which may present challenges for optimization.

If we were to stabilize `gen` blocks that could not hold borrows across `yield` points, this would be a serious usability limitation that users might find surprising.  Consequently, whether we should choose to address this before stabilization is an open question.

## Keyword

Should we use `iter` as the keyword since we're producing `Iterator`s?

Alternatively, we could use `gen` as proposed in this RFC and then later extend its abilities to include those of more powerful generators or coroutines, thereby justifying use of the more general name.

## Contextual keyword

Popular crates (like `rand`) have methods named [`gen`][Rng::gen].  If we reserve `gen` as a full keyword, users of Rust 2024 and later editions would need to call these methods as `r#gen` until these crates update to make some accommodation.

We could instead choose to make `gen` a contextual keyword and only forbid it in:

- bindings
- field names (due to destructuring bindings)
- enum variants
- type names

[Rng::gen]: https://docs.rs/rand/latest/rand/trait.Rng.html#method.gen

## `Iterator::size_hint`

Should we try to compute a conservative `size_hint`?

Doing this would reveal information from the body of a generator.  But, at least for simple cases, users would likely expect `size_hint` to not just be the default.

It is backward compatible to later add support for opportunistically implementing `size_hint`.

## Implement other `Iterator` traits

Might we later want to or be able to implement traits such as `DoubleEndedIterator`, `ExactSizeIterator`, etc.?

## What to do about Rust 2015 and Rust 2018

In [RFC 3101][] we reserved prefixed identifiers such as `prefix#ident`.  For this reason, we can make `gen` blocks available in Rust 2021 using `k#gen` as was anticipated in the (currently pending) [RFC 3098][].

Whether and how to make this feature available in Rust 2015 and Rust 2018, however, we leave as an open question.

[RFC 3098]: https://github.com/rust-lang/rfcs/pull/3098
[RFC 3101]: https://github.com/rust-lang/rfcs/pull/3101

# Future possibilities
[future-possibilities]: #future-possibilities

## `yield from` (forwarding operator)

Python has the ability to `yield from` an iterator.  Effectively this is syntactic sugar for looping over all elements of the iterator and yielding each individually.  There is a wide design space here, but some options are included in the following subsections.

### Do nothing, just use loops

Instead of adding special support for this, we could expect that users would write, e.g.:

```rust
for x in iter {
    yield x
}
```

### Language support

We could do something like postfix `yield`, e.g.:

```rust
iter.yield
```

Alternatively, we could use an entirely new keyword.

### stdlib macro

We could add a macro to the standard library and prelude.  The macro would expand to a `for` loop + `yield`.  E.g.:

```rust
yield_all!(iter)
```

## Complete `Coroutine` support

We have a `Coroutine` trait on nightly (previously called `Generator`) that is more powerful than the `Iterator` API could possibly be:

1. `resume` takes `Pin<&mut Self>`, allowing self-references across yield points.
2. `yield` returns the argument passed to `resume`.

We could perhaps argue for coroutines to be `gen` closures while leaving `gen` blocks as a simpler concept.

There are many open questions here, so we leave this to future work.

## `async` interactions

We could support using `await` in `gen async` blocks in a similar way to how we support `?` being used within `gen` blocks.  Without a solution for self-referential generators, we'd have the limitation that these blocks could not hold references across `await` points.

The solution space here is large.  This RFC is forward compatible with the solutions we can foresee, so we leave this to later work.

## `try` interactions

We could allow `gen try fn foo() -> i32` to mean something akin to `gen fn foo() -> Result<i32, E>`.  Whatever we do here, it should mirror whatever `try fn` means in the future.

## `gen fn`

This RFC does not introduce `gen fn`.  The syntax design space for this is large and there are open questions around the difference between returning or yielding a type.  The options currently known include, e.g.:

```rust
fn foo(..) yield .. { .. }
fn foo(..) yields .. { .. }
fn foo(..) => .. { .. }
// Each of the below may instead be combined
// with `yield`, `yields`, or `=>`.
fn* foo(..) -> .. { .. }
gen fn foo(..) -> .. { .. }
gen foo(..) -> .. { .. }
generator fn foo(..) -> .. { .. }
```

## Implement `FusedIterator`

The iterators produced by `gen` blocks are fused but do not implement `FusedIterator` because it is not a language item.  We may in the future want for these iterators to implement `FusedIterator`.
