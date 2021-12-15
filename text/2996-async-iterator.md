- Feature Name: `async_iterator`
- Start Date: 2020-09-29
- RFC PR: [rust-lang/rfcs#2996](https://github.com/rust-lang/rfcs/pull/2996)
- Rust Issue: [rust-lang/rust#79024](https://github.com/rust-lang/rust/issues/79024)

# Summary
[summary]: #summary

Introduce the `AsyncIterator` trait into the standard library, using the
design from `futures`. Redirect the `Stream` trait definition in the 
`futures-core` crate (which is "pub-used" by the `futures` crate) to the
`AsyncIterator` trait in the standard library.

# Motivation
[motivation]: #motivation

Async iterators are a core async abstraction. These behave similarly to `Iterator`,
but rather than blocking between each item yield, it allows other
tasks to run while it waits.

People can do this currently using the `Stream` trait defined in the 
[futures](https://crates.io/crates/futures) crate. However, we would like
to add `Stream` to the standard library as `AsyncIterator`. 

Including `AsyncIterator` in the standard library would clarify the stability guarantees of the trait. For example, if [Tokio](https://tokio.rs/) 
wishes to declare a [5 year stability period](http://smallcultfollowing.com/babysteps/blog/2020/02/11/async-interview-6-eliza-weisman/#communicating-stability), 
having the `AsyncIterator` trait in the standard library means there are no concerns 
about the trait changing during that time ([citation](http://smallcultfollowing.com/babysteps/blog/2019/12/23/async-interview-3-carl-lerche/#what-should-we-do-next-stabilize-stream)).

## Examples of current crates that are consuming async iterators

### async-h1

* [async-h1](https://docs.rs/async-h1)'s server implementation takes `TcpStream` instances produced by a `TcpListener` in a loop.

### async-sse

* [async-sse](https://docs.rs/async-sse/) parses incoming buffers into an async iterator of messages.

## Why a shared trait?

We eventually want dedicated syntax for working with async iterators, which will require a shared trait. 
This includes a trait for producing async iterators and a trait for consuming async iterators.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

An "async iterator" is the async version of an [iterator].

The `Iterator` trait includes a `next` method, which computes and returns the next item in the sequence. The `AsyncIterator` trait includes the `poll_next` method to assist with defining a async iterator. In the future, we should add a `next` method for use when consuming and interacting with a async iterator (see the [Future possiblilities][future-possibilities] section later in this RFC).

## poll_next method

When implementing a `AsyncIterator`, users will define a `poll_next` method. 
The `poll_next` method asks if the next item is ready. If so, it returns
the item. Otherwise, `poll_next` will return [`Poll::Pending`]. 

Just as with a [`Future`], returning [`Poll::Pending`] 
implies that the async iterator has arranged for the current task to be re-awoken when the data is ready.

[iterator]: https://doc.rust-lang.org/std/iter/trait.Iterator.html
[`Future`]: https://doc.rust-lang.org/std/future/trait.Future.html
[`Poll::Pending`]: https://doc.rust-lang.org/std/task/enum.Poll.html#variant.Pending

```rust
// Defined in std::async_iter module
pub trait AsyncIterator {
    // Core items:
    type Item;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>>;
    
    // Optional optimization hint, just like with iterators:
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, None)
    }
}
```

The arguments to `poll_next` match that of the [`Future::poll`] method:

* The self must be a pinned reference, ensuring both unique access to
  the async iterator and that the async iterator value itself will not move. Pinning
  allows the async iterator to save pointers into itself when it suspends,
  which will be required to support generator syntax at some point.
* The [context] `cx` defines details of the current task. In particular,
  it gives access to the [`Waker`] for the task, which will allow the
  task to be re-awoken once data is ready.

[`Future::poll`]: https://doc.rust-lang.org/std/future/trait.Future.html#tymethod.poll
[pinned]: https://doc.rust-lang.org/std/pin/struct.Pin.html
[context]: https://doc.rust-lang.org/std/task/struct.Context.html
[`Waker`]: https://doc.rust-lang.org/std/task/struct.Waker.html

### Usage

A user could create an async iterator as follows (Example taken from @yoshuawuyts' [implementation pull request](https://github.com/rust-lang/rust/pull/79023)).

Creating an async iterator involves two steps: creating a `struct` to
 hold the async iterator's state, and then implementing `AsyncIterator` for that
 `struct`.

 Let's make an async iterator named `Counter` which counts from `1` to `5`:

```rust
#![feature(async_iterator)]
# use core::async_iter::AsyncIterator;
# use core::task::{Context, Poll};
# use core::pin::Pin;

// First, the struct:

/// An async iterator which counts from one to five
struct Counter {
    count: usize,
}

// we want our count to start at one, so let's add a new() method to help.
// This isn't strictly necessary, but is convenient. Note that we start
// `count` at zero, we'll see why in `poll_next()`'s implementation below.
impl Counter {
    fn new() -> Counter {
        Counter { count: 0 }
    }
}

// Then, we implement `AsyncIterator` for our `Counter`:

impl AsyncIterator for Counter {
    // we will be counting with usize
    type Item = usize;

    // poll_next() is the only required method
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Increment our count. This is why we started at zero.
        self.count += 1;

        // Check to see if we've finished counting or not.
        if self.count < 6 {
            Poll::Ready(Some(self.count))
        } else {
            Poll::Ready(None)
        }
    }
}
```

## Initial impls

There are a number of simple "bridge" impls that are also provided:

```rust
impl<S> AsyncIterator for Box<S>
where
    S: AsyncIterator + Unpin + ?Sized,
{
    type Item = <S as AsyncIterator>::Item
}

impl<S> AsyncIterator for &mut S
where
    S: AsyncIterator + Unpin + ?Sized,
{
    type Item = <S as AsyncIterator>::Item;
}

impl<S, T> AsyncIterator for Pin<P>
where
    P: DerefMut<Target=T> + Unpin,
    T: AsyncIterator,
{
    type Item = <T as AsyncIterator>::Item;
}

impl<S> AsyncIterator for AssertUnwindSafe<S>
where
    S: AsyncIterator, 
{
    type Item = <S as AsyncIterator>::Item;
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This section goes into details about various aspects of the design and
why they ended up the way they did.

## Where does `AsyncIterator` live in the std lib?

`AsyncIterator` will live in the `core::async_iter` module and be re-exported as `std::async_iter`.

It is possible that it could live in another area as well, though this follows
the pattern of `core::future`.

## Why use a `poll` method?

An alternative design for the async iterator trait would be to have a trait
that defines an async `next` method:

```rust
trait AsyncIterator {
    type Item;
    
    async fn next(&mut self) -> Option<Self::Item>;
}
```

Unfortunately, async methods in traits are not currently supported,
and there [are a number of challenges to be
resolved](https://rust-lang.github.io/wg-async-foundations/design_notes/async_fn_in_traits.html)
before they can be added. 

Moreover, it is not clear yet how to make traits that contain async
functions be `dyn` safe, and it is important to be able to pass around `dyn
AsyncIterator` values without the need to monomorphize the functions that work
with them.

Unfortunately, the use of poll does mean that it is harder to write
async iterator implementations. The long-term fix for this, discussed in the [Future possiblilities][future-possibilities] section, is dedicated [generator syntax].

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Where should async iterator live?

As mentioned above, `core::async_iter` is analogous to `core::future`. But, do we want to find 
some other naming scheme that can scale up to other future additions, such as io traits or channels?

## Naming

When considering what to name the trait and concepts, there were two options:

- __`Stream`:__ with prior art in `futures-rs`, runtimes, and much of the
  of the async ecosystem.
- __`AsyncIterator`:__ which follows the pattern established of prefixing
  the async version of another trait with `Async` in the ecosystem. For example
  [`AsyncRead`](https://docs.rs/futures-io/latest/futures_io/trait.AsyncRead.html)
  is an async version of [`Read`](https://doc.rust-lang.org/std/io/trait.Read.html).

We ended up choosing `AsyncIterator` over `Stream` for a number of reasons:

1. It provides consistency between async and non-async Rust. Prefixing the async
   version of an existing trait with `Async` helps with discoverability, and teaching
   how APIs relate to each other. For example in this RFC we describe
   `AsyncIterator` as "an async version of `Iterator`".
2. The word "stream" is fairly established terminology within computing: it
   commonly refers to a type which yields data repeatedly. Traits such as
   `Iterator`, `Read`, and `Write` are often referred to as "streams" or
   "streaming".  Naming a single trait `Stream` can lead to confusion, as it is not
   the only trait which streams.
3. `std::net::TcpStream` does not in fact implement `Stream`, despite the name
   suggesting it might. In the ecosystem async versions of `TcpStream` don't either: 
   `Async{Read,Write}` are used instead. This can be confusing.

Additionally, there is prior art in other languages for using an
"iterator"/"async iterator" naming scheme:

- JavaScript: [`Symbol.Iterator`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Symbol/iterator)
  and [`Symbol.AsyncIterator`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Symbol/asyncIterator)
- C#: [`IEnumerable`](https://docs.microsoft.com/en-us/dotnet/api/system.collections.generic.ienumerable-1?view=net-5.0)
  and [`IAsyncEnumerable`](https://docs.microsoft.com/en-us/archive/msdn-magazine/2019/november/csharp-iterating-with-async-enumerables-in-csharp-8)
- Python: [`__iter__`](https://wiki.python.org/moin/Iterator)
  and [`__aiter__`](https://www.python.org/dev/peps/pep-0525/)
- Swift: [`Sequence`](https://developer.apple.com/documentation/swift/sequence)
  and [`AsyncSequence`](https://github.com/apple/swift-evolution/blob/main/proposals/0298-asyncsequence.md)

Despite being a clearer in many regards, the name `AsyncIterator` loses to
`Stream` in terms of brevity. `AsyncIterator` / `async_iter` / "async iterator"
is longer to write than `stream` in every instance.

Additionally the Rust ecosystem has a multi-year history of using `Stream` to
describe the concept of "async iterators". But we expect that as
`AsyncIterator` becomes the agreed upon terminology to refer to "async iterators",
the historical benefit of using "stream" terminology will lessen over time.

Overall we found that despite having some downsides, the name `AsyncIterator`
is strongly preferable over `Stream`.

# Future possibilities
[future-possibilities]: #future-possibilities

## Next method

While users will be able to implement a `AsyncIterator` as defined in this RFC, they will not have a way to interact with it in the core library. As soon as we figure out a way to do it in an object safe manner, we should add a `next` method  either in the `AsyncIterator` trait or elsewhere.

The `Iterator` trait includes a `next` method, which computes and returns the next item in the sequence. We should also implement a `next` method for `AsyncIterator`, similar to [the implementation in the futures-util crate](https://docs.rs/futures-util/0.3.5/src/futures_util/stream/stream/next.rs.html#10-12).

The core `poll_next` method is unergonomic; it does not let you iterate 
over the items coming out of the async iterator. Therefore, we include a few minimal 
convenience methods that are not dependent on any unstable features, such as `next`.

As @yoshuawuyts states in their [pull request which adds `core::stream::Stream` to the standard library](https://github.com/rust-lang/rust/pull/79023):

Unlike `Iterator`, `AsyncIterator` makes a distinction between the `poll_next`
method which is used when implementing a `AsyncIterator`, and the `next` method
which is used when consuming an async iterator. Consumers of `AsyncIterator` only need to
consider `next`, which when called, returns a future which yields
`Option<Item>`.

The future returned by `next` will yield `Some(Item)` as long as there are
elements, and once they've all been exhausted, will yield `None` to indicate
that iteration is finished. If we're waiting on something asynchronous to
resolve, the future will wait until the async iterator is ready to yield again.

As defined in the [`Future` docs](https://doc.rust-lang.org/stable/std/future/trait.Future.html):

Once a future has completed (returned Ready from poll), calling its poll method again may panic, block forever, or cause other kinds of problems; the Future trait places no requirements on the effects of such a call. However, as the poll method is not marked unsafe, Rust's usual rules apply: calls must never cause undefined behavior (memory corruption, incorrect use of unsafe functions, or the like), regardless of the future's state.

This is similar to the `Future` trait. The `Future::poll` method is rarely called 
directly, it is almost always used to implement other Futures. Interacting
with futures is done through `async/await`.

We need something like the `next()` method in order to iterate over the async iterator directly in an `async` block or function. It is essentially an adapter from `AsyncIterator` to `Future`.

This would allow a user to await on a future:

```rust
while let Some(v) = async_iter.next().await {

}
```

We could also consider adding a `try_next` method, allowing
a user to write:

```rust
while let Some(x) = s.try_next().await?
```

But this could also be written as:

```rust
while let Some(x) = s.next().await.transpose()?
```

### More Usage Examples

Using the example of `AsyncIterator` implemented on a struct called `Counter`, the user would interact with the async iterator like so:

```rust
let mut counter = Counter::new();

let x = counter.next().await.unwrap();
println!("{}", x);

let x = counter.next().await.unwrap();
println!("{}", x);

let x = counter.next().await.unwrap();
println!("{}", x);

let x = counter.next().await.unwrap();
println!("{}", x);

let x = counter.next().await.unwrap();
println!("{}", x);
#
}
```

This would print `1` through `5`, each on their own line.

An earlier draft of the RFC prescribed an implementation of the `next` method on the `AsyncIterator` trait. Unfortunately, as detailed in [this comment](https://github.com/rust-lang/rust/pull/79023#discussion_r547425181), it made the async iterator non-object safe. More experimentation is required - and it may need to be an unstable language feature for more testing before it can be added to core.

## More Convenience methods

The `Iterator` trait also defines a number of useful combinators, like
`map`.  The `AsyncIterator` trait being proposed here does not include any
such conveniences.  Instead, they are available via extension traits,
such as the [`AsyncIteratorExt`] trait offered by the [`futures`] crate.

[`AsyncIteratorExt`]: https://docs.rs/futures/0.3.5/futures/stream/trait.AsyncIteratorExt.html
[`futures`]: https://crates.io/crates/futures

The reason that we have chosen to exclude combinators is that a number
of them would require access to async closures. As of this writing,
async closures are unstable and there are a number of [outstanding
design issues] to be resolved before they are added. Therefore, we've
decided to enable progress on the async iterator trait by stabilizing a core,
and to come back to the problem of extending it with combinators.

[outstanding design issues]: https://rust-lang.github.io/wg-async-foundations/design_docs/async_closures.html

This path does carry some risk. Adding combinator methods can cause
existing code to stop compiling due to the ambiguities in method
resolution. We have had problems in the past with attempting to migrate
iterator helper methods from `itertools` for this same reason.

While such breakage is technically permitted by our semver guidelines,
it would obviously be best to avoid it, or at least to go to great
lengths to mitigate its effects. One option would be to extend the
language to allow method resolution to "favor" the extension trait in
existing code, perhaps as part of an edition migration.

Designing such a migration feature is out of scope for this RFC.

## IntoAsyncIterator / FromAsyncIterator traits

### IntoAsyncIterator

**Iterators**

Iterators have an `IntoIterator` that is used with `for` loops to convert items of other types to an iterator.

```rust
pub trait IntoIterator where
    <Self::IntoIter as Iterator>::Item == Self::Item, 
{
    type Item;

    type IntoIter: Iterator;

    fn into_iter(self) -> Self::IntoIter;
}
```

Examples are taken from the Rust docs on [for loops and into_iter](https://doc.rust-lang.org/std/iter/index.html#for-loops-and-intoiterator)

* `for x in iter` uses `impl IntoIterator for T`

```rust
let values = vec![1, 2, 3, 4, 5];

for x in values {
    println!("{}", x);
}
```

Desugars to:

```rust
let values = vec![1, 2, 3, 4, 5];
{
    let result = match IntoIterator::into_iter(values) {
        mut iter => loop {
            let next;
            match iter.next() {
                Some(val) => next = val,
                None => break,
            };
            let x = next;
            let () = { println!("{}", x); };
        },
    };
    result
}
```
* `for x in &iter` uses `impl IntoIterator for &T`
* `for x in &mut iter` uses `impl IntoIterator for &mut T`

**AsyncIterators**

We may want a trait similar to this for `AsyncIterator`. The `IntoAsyncIterator` trait would provide a way to convert something into a `AsyncIterator`.

This trait could look like this:

```rust
pub trait IntoAsyncIterator
where 
    <Self::IntoAsyncIterator as AsyncIterator>::Item == Self::Item,
{
    type Item;

    type IntoAsyncIterator: AsyncIterator;

    fn into_async_iter(self) -> Self::IntoAsyncIterator;
}
```

This trait (as expressed by @taiki-e in [a comment on a draft of this RFC](https://github.com/rust-lang/wg-async-foundations/pull/15/files#r449880986)) makes it easy to write streams in combination with [async iterator](https://github.com/taiki-e/futures-async-stream). For example:

```rust
type S(usize);

impl IntoAsyncIterator for S {
    type Item = usize;
    type IntoAsyncIterator: impl AsyncIterator<Item = Self::Item>;

    fn into_async_iter(self) -> Self::IntoAsyncIterator {
        #[stream]
        async move {
            for i in 0..self.0 {
                yield i;
            }
        }
    }
}   
```

### FromAsyncIterator

**Iterators**

Iterators have an `FromIterator` that is used to convert iterators into another type.

```rust
pub trait FromIterator<A> {

    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = A>;
}
```

It should be noted that this trait is rarely used directly, instead used through Iterator's collect method ([source](https://doc.rust-lang.org/std/iter/trait.FromIterator.html)).

```rust
pub trait Iterator {
    fn collect<B>(self) -> B
    where
        B: FromIterator<Self::Item>,
    { ... }
}
```

Examples are taken from the Rust docs on [iter and collect](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.collect)


```rust
let a = [1, 2, 3];

let doubled: Vec<i32> = a.iter()
                         .map(|&x| x * 2)
                         .collect();

```

**Async Iterators**

We may want a trait similar to this for `AsyncIterator`. The `FromAsyncIterator` trait would provide a way to convert a `AsyncIterator` into another type.

This trait could look like this:

```rust
pub trait FromAsyncIterator<A> {
    async fn from_async_iter<T>(iter: T) -> Self
    where
        T: IntoAsyncIterator<Item = A>;
}
```

We could potentially include a collect method for AsyncIterator as well.

```rust
pub trait AsyncIterator {
    async fn collect<B>(self) -> B
    where
        B: FromAsyncIterator<Self::Item>,
    { ... }
}
```

When drafting this RFC, there was [discussion](https://github.com/rust-lang/wg-async-foundations/pull/15#discussion_r451182595) 
about whether to implement from_async_iter for all T where `T: FromIterator` as well.
`FromAsyncIterator` is perhaps more general than `FromIterator` because the await point is allowed to suspend execution of the 
current function, but doesn't have to. Therefore, many (if not all) existing impls of `FromIterator` would work
for `FromAsyncIterator` as well. While this would be a good point for a future discussion, it is not in the scope of this RFC.

## Converting an Iterator to a AsyncIterator

If a user wishes to convert an Iterator to a AsyncIterator, they may not be able to use IntoAsyncIterator because a blanked impl for Iterator would conflict with more specific impls they may wish to write. Having a function that takes an `impl Iterator<Item = T>` and returns an `impl AsyncIterator<Item = T>` would be quite helpful. 

The [async-std](https://github.com/async-rs/async-std) crate has [stream::from_iter](https://docs.rs/async-std/1.6.5/async_std/stream/fn.from_iter.html). The [futures-rs](https://github.com/rust-lang/futures-rs) crate has [stream::iter](https://docs.rs/futures/0.3.5/futures/stream/fn.iter.html). Either of these approaches could work once we expose `AsyncIterator` in the standard library.

Adding this functionality is out of the scope of this RFC, but is something we should revisit once `AsyncIterator` is in the standard library.

## Other Traits

Eventually, we may also want to add some (if not all) of the roster of traits we found useful for `Iterator`.

[async_std::stream](https://docs.rs/async-std/1.6.0/async_std/stream/index.html) has created several async counterparts to the traits in [std::iter](https://doc.rust-lang.org/std/iter/). These include:

* DoubleEndedAsyncIterator: An async iterator able to yield elements from both ends.
* ExactSizeAsyncIterator: An async iterator that knows its exact length.
* Extend: Extends a collection with the contents of an async iterator.
* FromAsyncIterator: Conversion from a AsyncIterator.
* FusedAsyncIterator: An async iterator that always continues to yield None when exhausted.
* IntoAsyncIterator: Conversion into a AsyncIterator.
* Product: Trait to represent types that can be created by multiplying the elements of an async iterator.
* AsyncIterator: An asynchronous stream of values.
* Sum: Trait to represent types that can be created by summing up an async iterator.

As detailed in previous sections, the migrations to add these traits are out of scope for this RFC.

## Async iteration syntax

Currently, if someone wishes to iterate over a `AsyncIterator` as defined in the `futures` crate,
they are not able to use  `for` loops, they must use `while let` and `next/try_next` instead.

We may wish to extend the `for` loop so that it works over async iterators as well. 

```rust
#[async]
for elem in iter { ... }
```

One of the complications of using `while let` syntax is the need to pin.
A `for` loop syntax that takes ownership of the async iterator would be able to
do the pinning for you. 

We may not want to make sequential processing "too easy" without also enabling
parallel/concurrent processing, which people frequently want. One challenge is
that parallel processing wouldn't naively permit early returns and other complex
control flow. We could add a `par_async_iter()` method, similar to 
[Rayon's](https://github.com/rayon-rs/rayon) `par_iter()`.

Designing this extension is out of scope for this RFC. However, it could be prototyped using procedural macros today.

## "Lending" async iterators

There has been much discussion around lending async iterators (also referred to as attached async iterators).

### Definitions

[Source](https://smallcultfollowing.com/babysteps/blog/2019/12/10/async-interview-2-cramertj-part-2/#the-need-for-streaming-streams-and-iterators)


In a **lending** async iterator (also known as an "attached" async iterator), the `Item` that gets 
returned by `AsyncIterator` may be borrowed from `self`. It can only be used as long as 
the `self` reference remains live.

In a **non-lending** async iterator (also known as a "detached" async iterator), the `Item` that 
gets returned by `AsyncIterator` is "detached" from self. This means it can be stored 
and moved about independently from `self`.

This RFC does not cover the addition of lending async iterators (async iterators as implemented through 
this RFC are all non-lending async iterators). Lending async iterators depend on [Generic Associated Types](https://rust-lang.github.io/rfcs/1598-generic_associated_types.html), which are not (at the time of this RFC) stable.

We can add the `AsyncIterator` trait to the standard library now and delay
adding in this distinction between the two types of async iterators - lending and
non-lending. The advantage of this is it would allow us to copy the `AsyncIterator`
trait from `futures` largely 'as is'. 

The disadvantage of this is functions that consume async iterators would 
first be written to work with `AsyncIterator`, and then potentially have 
to be rewritten later to work with `LendingAsyncIterator`s.

### Current AsyncIterator Trait

```rust
pub trait AsyncIterator {
    type Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>>;

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, None)
    }
}
```

This trait, like `Iterator`, always gives ownership of each item back to its caller. This offers flexibility - 
such as the ability to spawn off futures processing each item in parallel.

### Potential Lending AsyncIterator Trait

```rust
trait LendingAsyncIterator<'s> {
    type Item<'a> where 's: 'a;

    fn poll_next<'a>(
        self: Pin<&'a mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item<'a>>>;
}

impl<S> LendingAsyncIterator for S
where
    S: AsyncIterator,
{
    type Item<'_> = S::Item;
    
    fn poll_next<'s>(
        self: Pin<&'s mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item<'s>>> {
        AsyncIterator::poll_next(self, cx)
    }
}
```

This is a "conversion" trait such that anything which implements `AsyncIterator` can also implement 
`LendingAsyncIterator`.

This trait captures the case where we re-use internal buffers. This would be less flexible for 
consumers, but potentially more efficient. Types could implement the `LendingAsyncIterator` 
where they need to re-use an internal buffer and `AsyncIterator` if they do not. There is room for both.

We would also need to pursue the same design for iterators - whether through adding two traits
or one new trait with a "conversion" from the old trait.

This also brings up the question of whether we should allow conversion in the opposite way - if
every non-lending async iterator can become a lending one, should _some_ lending async iterators be able to 
become non-lending ones? 

**Coherence**

The impl above has a problem. As the Rust language stands today, we cannot cleanly convert 
impl AsyncIterator to impl LendingAsyncIterator due to a coherence conflict.

If you have other impls like:

```rust
impl<T> AsyncIterator for Box<T> where T: AsyncIterator
```

and

```rust
impl<T> LendingAsyncIterator for Box<T> where T: LendingAsyncIterator
```

There is a coherence conflict for `Box<impl AsyncIterator>`, so presumably it will fail the coherence rules. 

[More examples are available here](https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=a667a7560f8dc97ab82a780e27dfc9eb).

Resolving this would require either an explicit “wrapper” step or else some form of language extension.

It should be noted that the same applies to Iterator, it is not unique to AsyncIterator.

We may eventually want a super trait relationship available in the Rust language

```rust
trait AsyncIterator: LendingAsyncIterator
```

This would allow us to leverage `default impl`.

These use cases for lending/non-lending async iterators need more thought, which is part of the reason it 
is out of the scope of this particular RFC.

## Generator syntax
[generator syntax]: #generator-syntax

In the future, we may wish to introduce a new form of function - 
`gen fn` in iterators and `async gen fn` in async code that
can contain `yield` statements. Calling such a function would
yield a `impl Iterator` or `impl AsyncIterator`, for sync and async 
respectively. Given an "attached" or "borrowed" async iterator, the generator
could yield references to local variables. Given a "detached"
or "owned" async iterator, the generator could yield owned values
or things that were borrowed from its caller.

### In Iterators

```rust
gen fn foo() -> Value {
    yield value;
}
```

After desugaring, this would result in a function like:

```rust
fn foo() -> impl Iterator<Item = Value>
```

### In Async Code

```rust
async gen fn foo() -> Value
```

After desugaring would result in a function like:

```rust
fn foo() -> impl AsyncIterator<Item = Value>
```

If we introduce `-> impl AsyncIterator` first, we will have to permit `LendingAsyncIterator` in the future. 
Additionally, if we introduce `LendingAsyncIterator` later, we'll have to figure out how
to convert a `LendingAsyncIterator` into a `AsyncIterator` seamlessly.

### Differences between Iterator generators and Async generators

We want `AsyncIterator` and `Iterator` to work as analogously as possible, including when used with generators. However, in the current design, there are some crucial differences between the two. 

Consider Iterator's core `next` method:

```rust
pub trait Iterator {
    type Item;

    fn next(&mut self) -> Option<Self::Item>;
}
```

Iterator does not require pinning its core next method. In order for a `gen fn` to operate with the Iterator ecosystem, there must be some kind of initial pinning step that converts its result into an iterator. This will be tricky, since you can't return a pinned value except by boxing. 

The general shape will be:

```rust
gen_fn().pin_somehow().adapter1().adapter2()
```

With async iterators, the core interface _is_ pinned, so pinning occurs at the last moment.

The general shape would be 

```rust
async_gen_fn().adapter1().adapter2().pin_somehow()
```

Pinning at the end, like with an async iterator, lets you build and return those adapters and then apply pinning at the end. This may be the more efficient setup and implies that, in order to have a `gen fn` that produces iterators, we will need to potentially disallow borrowing yields or implement some kind of `PinnedIterator` trait that can be "adapted" into an iterator by pinning.

For example: 

```rust
trait PinIterator {
    type Item;
}
impl<I: PinIterator, P: Deref<Target = I> + DerefMut> Iterator for Pin<P> {
    fn next(&mut self) -> Self::Item { self.as_mut().next() }
}

// this would be nice.. but would lead to name resolution ambiguity for our combinators 😬 
default impl<T: Iterator> PinIterator for T { .. }
```

Pinning also applies to the design of AsyncRead/AsyncWrite, which currently uses Pin even through there is no clear plan to make them implemented with generator type syntax. The asyncification of a signature is currently understood as pinned receiver + context arg + return poll.

Another key difference between `Iterator`s and `AsyncIterator`s is that futures are ultimately passed to some executor API like spawn which expects a `'static` future. To achieve that, the futures contain all the state they need and references are internal to that state. Iterators are almost never required to be `'static` by the APIs that consume them.

It is, admittedly, somewhat confusing to have Async generators require Pinning and Iterator generators to not require pinning, users may feel they are creating code in an unnatural way when using the Async generators. This will need to be discussed more when generators are proposed in the future.

### Disallowing self-borrowing generators in `gen fn`

Another option is to make the generators returned by `gen fn` always be `Unpin` so that the user doesn't have to think about pinning unless they're already in an async context.

In the spirit of experimentation, boats has written the [propane] 
crate. This crate includes a `#[propane] fn` that changes the function signature
to return `impl Iterator` and lets you `yield`. The non-async version uses 
(nightly-only) generators which are non-`static`, disallowing self-borrowing.
In other words, you can't hold a reference to something on the stack across a `yield`.

This should still allow yielding from inside a for loop, as long as the for loop is
over a borrowed input and not something owned by the stack frame.

[propane]: https://github.com/withoutboats/propane

Further designing generator functions is out of the scope of this RFC.
