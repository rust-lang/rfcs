- Feature Name: `async_stream`
- Start Date: 2020-09-29
- RFC PR: [rust-lang/rfcs#2996](https://github.com/rust-lang/rfcs/pull/2996)
- Rust Issue: [rust-lang/rust#79024](https://github.com/rust-lang/rust/issues/79024)

# Summary
[summary]: #summary

Introduce the `Stream` trait into the standard library, using the
design from `futures`. Redirect the `Stream` trait definition in the 
`futures-core` crate (which is "pub-used" by the `futures` crate) to the standard library.

# Motivation
[motivation]: #motivation

Streams are a core async abstraction. These behave similarly to `Iterator`,
but rather than blocking between each item yield, it allows other
tasks to run while it waits.

People can do this currently using the `Stream` trait defined in the 
[futures](https://crates.io/crates/futures) crate. However, we would like
to add `Stream` to the standard library. 

Including `Stream` in the standard library would clarify the stability guarantees of the trait. For example, if [Tokio](https://tokio.rs/) 
wishes to declare a [5 year stability period](http://smallcultfollowing.com/babysteps/blog/2020/02/11/async-interview-6-eliza-weisman/#communicating-stability), 
having the `Stream` trait in the standard library means there are no concerns 
about the trait changing during that time ([citation](http://smallcultfollowing.com/babysteps/blog/2019/12/23/async-interview-3-carl-lerche/#what-should-we-do-next-stabilize-stream)).

## Examples of current crates that are consuming streams

### async-h1

* [async-h1](https://docs.rs/async-h1)'s server implementation takes `TcpStream` instances produced by a `TcpListener` in a loop.

### async-sse

* [async-sse](https://docs.rs/async-sse/) parses incoming buffers into a stream of messages.

## Why a shared trait?

We eventually want dedicated syntax for working with streams, which will require a shared trait. 
This includes a trait for producing streams and a trait for consuming streams.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

A "stream" is the async version of an [iterator].

The `Iterator` trait includes a `next` method, which computes and returns the next item in the sequence. The `Stream` trait includes the `poll_next` method to assist with defining a stream. In the future, we should add a `next` method for use when consuming and interacting with a stream (see the [Future possiblilities][future-possibilities] section later in this RFC).

## poll_next method

When implementing a `Stream`, users will define a `poll_next` method. 
The `poll_next` method asks if the next item is ready. If so, it returns
the item. Otherwise, `poll_next` will return [`Poll::Pending`]. 

Just as with a [`Future`], returning [`Poll::Pending`] 
implies that the stream has arranged for the current task to be re-awoken when the data is ready.

[iterator]: https://doc.rust-lang.org/std/iter/trait.Iterator.html
[`Future`]: https://doc.rust-lang.org/std/future/trait.Future.html
[`Poll::Pending`]: https://doc.rust-lang.org/std/task/enum.Poll.html#variant.Pending

```rust
// Defined in std::stream module
pub trait Stream {
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
  the stream and that the stream value itself will not move. Pinning
  allows the stream to save pointers into itself when it suspends,
  which will be required to support generator syntax at some point.
* The [context] `cx` defines details of the current task. In particular,
  it gives access to the [`Waker`] for the task, which will allow the
  task to be re-awoken once data is ready.

[`Future::poll`]: https://doc.rust-lang.org/std/future/trait.Future.html#tymethod.poll
[pinned]: https://doc.rust-lang.org/std/pin/struct.Pin.html
[context]: https://doc.rust-lang.org/std/task/struct.Context.html
[`Waker`]: https://doc.rust-lang.org/std/task/struct.Waker.html

### Usage

A user could create a stream as follows (Example taken from @yoshuawuyt's [implementation pull request](https://github.com/rust-lang/rust/pull/79023)).

Creating a stream involves two steps: creating a `struct` to
 hold the stream's state, and then implementing `Stream` for that
 `struct`.

 Let's make a stream named `Counter` which counts from `1` to `5`:

```rust
#![feature(async_stream)]
# use core::stream::Stream;
# use core::task::{Context, Poll};
# use core::pin::Pin;

// First, the struct:

/// A stream which counts from one to five
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

// Then, we implement `Stream` for our `Counter`:

impl Stream for Counter {
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
impl<S> Stream for Box<S>
where
    S: Stream + Unpin + ?Sized,
{
    type Item = <S as Stream>::Item
}

impl<S> Stream for &mut S
where
    S: Stream + Unpin + ?Sized,
{
    type Item = <S as Stream>::Item;
}

impl<S, T> Stream for Pin<P>
where
    P: DerefMut<Target=T> + Unpin,
    T: Stream,
{
    type Item = <T as Stream>::Item;
}

impl<S> Stream for AssertUnwindSafe<S>
where
    S: Stream, 
{
    type Item = <S as Stream>::Item;
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This section goes into details about various aspects of the design and
why they ended up the way they did.

## Where does `Stream` live in the std lib?

`Stream` will live in the `core::stream` module and be re-exported as `std::stream`.

It is possible that it could live in another area as well, though this follows
the pattern of `core::future`.

## Why use a `poll` method?

An alternative design for the stream trait would be to have a trait
that defines an async `next` method:

```rust
trait Stream {
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
Stream` values without the need to monomorphize the functions that work
with them.

Unfortunately, the use of poll does mean that it is harder to write
stream implementations. The long-term fix for this, discussed in the [Future possiblilities][future-possibilities] section, is dedicated [generator syntax].

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Where should stream live?

As mentioned above, `core::stream` is analogous to `core::future`. But, do we want to find 
some other naming scheme that can scale up to other future additions, such as io traits or channels?

# Future possibilities
[future-possibilities]: #future-possibilities

## Next method

While users will be able to implement a `Stream` as defined in this RFC, they will not have a way to interact with it in the core library. As soon as we figure out a way to do it in an object safe manner, we should add a `next` method  either in the `Stream` trait or elsewhere.

The `Iterator` trait includes a `next` method, which computes and returns the next item in the sequence. We should also implement a `next` method for `Stream`, similar to [the implementation in the futures-util crate](https://docs.rs/futures-util/0.3.5/src/futures_util/stream/stream/next.rs.html#10-12).

The core `poll_next` method is unergonomic; it does not let you iterate 
over the items coming out of the stream. Therefore, we include a few minimal 
convenience methods that are not dependent on any unstable features, such as `next`.

As @yoshuawuyts states in their [pull request which adds `core::stream::Stream` to the standard library](https://github.com/rust-lang/rust/pull/79023):

Unlike `Iterator`, `Stream` makes a distinction between the `poll_next`
method which is used when implementing a `Stream`, and the `next` method
which is used when consuming a stream. Consumers of `Stream` only need to
consider `next`, which when called, returns a future which yields
`Option<Item>`.

The future returned by `next` will yield `Some(Item)` as long as there are
elements, and once they've all been exhausted, will yield `None` to indicate
that iteration is finished. If we're waiting on something asynchronous to
resolve, the future will wait until the stream is ready to yield again.

As defined in the [`Future` docs](https://doc.rust-lang.org/stable/std/future/trait.Future.html):

Once a future has completed (returned Ready from poll), calling its poll method again may panic, block forever, or cause other kinds of problems; the Future trait places no requirements on the effects of such a call. However, as the poll method is not marked unsafe, Rust's usual rules apply: calls must never cause undefined behavior (memory corruption, incorrect use of unsafe functions, or the like), regardless of the future's state.

This is similar to the `Future` trait. The `Future::poll` method is rarely called 
directly, it is almost always used to implement other Futures. Interacting
with futures is done through `async/await`.

We need something like the `next()` method in order to iterate over the stream directly in an `async` block or function. It is essentially an adapter from `Stream` to `Future`.

This would allow a user to await on a future:

```rust
while let Some(v) = stream.next().await {

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

Using the example of `Stream` implemented on a struct called `Counter`, the user would interact with the stream like so:

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

An earlier draft of the RFC prescribed an implementation of the `next` method on the `Stream` trait. Unfortunately, as detailed in [this comment](https://github.com/rust-lang/rust/pull/79023#discussion_r547425181), it made the stream non-object safe. More experimentation is required - and it may need to be an unstable language feature for more testing before it can be added to core.

## More Convenience methods

The `Iterator` trait also defines a number of useful combinators, like
`map`.  The `Stream` trait being proposed here does not include any
such conveniences.  Instead, they are available via extension traits,
such as the [`StreamExt`] trait offered by the [`futures`] crate.

[`StreamExt`]: https://docs.rs/futures/0.3.5/futures/stream/trait.StreamExt.html
[`futures`]: https://crates.io/crates/futures

The reason that we have chosen to exclude combinators is that a number
of them would require access to async closures. As of this writing,
async closures are unstable and there are a number of [outstanding
design issues] to be resolved before they are added. Therefore, we've
decided to enable progress on the stream trait by stabilizing a core,
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

## IntoStream / FromStream traits

### IntoStream

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

**Streams**

We may want a trait similar to this for `Stream`. The `IntoStream` trait would provide a way to convert something into a `Stream`.

This trait could look like this:

```rust
pub trait IntoStream
where 
    <Self::IntoStream as Stream>::Item == Self::Item,
{
    type Item;

    type IntoStream: Stream;

    fn into_stream(self) -> Self::IntoStream;
}
```

This trait (as expressed by @taiki-e in [a comment on a draft of this RFC](https://github.com/rust-lang/wg-async-foundations/pull/15/files#r449880986)) makes it easy to write streams in combination with [async stream](https://github.com/taiki-e/futures-async-stream). For example:

```rust
type S(usize);

impl IntoStream for S {
    type Item = usize;
    type IntoStream: impl Stream<Item = Self::Item>;

    fn into_stream(self) -> Self::IntoStream {
        #[stream]
        async move {
            for i in 0..self.0 {
                yield i;
            }
        }
    }
}   
```

### FromStream

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

**Streams**

We may want a trait similar to this for `Stream`. The `FromStream` trait would provide a way to convert a `Stream` into another type.

This trait could look like this:

```rust
pub trait FromStream<A> {
    async fn from_stream<T>(stream: T) -> Self
    where
        T: IntoStream<Item = A>;
}
```

We could potentially include a collect method for Stream as well.

```rust
pub trait Stream {
    async fn collect<B>(self) -> B
    where
        B: FromStream<Self::Item>,
    { ... }
}
```

When drafting this RFC, there was [discussion](https://github.com/rust-lang/wg-async-foundations/pull/15#discussion_r451182595) 
about whether to implement from_stream for all T where `T: FromIterator` as well.
`FromStream` is perhaps more general than `FromIterator` because the await point is allowed to suspend execution of the 
current function, but doesn't have to. Therefore, many (if not all) existing impls of `FromIterator` would work
for `FromStream` as well. While this would be a good point for a future discussion, it is not in the scope of this RFC.

## Converting an Iterator to a Stream

If a user wishes to convert an Iterator to a Stream, they may not be able to use IntoStream because a blanked impl for Iterator would conflict with more specific impls they may wish to write. Having a function that takes an `impl Iterator<Item = T>` and returns an `impl Stream<Item = T>` would be quite helpful. 

The [async-std](https://github.com/async-rs/async-std) crate has [stream::from_iter](https://docs.rs/async-std/1.6.5/async_std/stream/fn.from_iter.html). The [futures-rs](https://github.com/rust-lang/futures-rs) crate has [stream::iter](https://docs.rs/futures/0.3.5/futures/stream/fn.iter.html). Either of these approaches could work once we expose `Stream` in the standard library.

Adding this functionality is out of the scope of this RFC, but is something we should revisit once `Stream` is in the standard library.

## Other Traits

Eventually, we may also want to add some (if not all) of the roster of traits we found useful for `Iterator`.

[async_std::stream](https://docs.rs/async-std/1.6.0/async_std/stream/index.html) has created several async counterparts to the traits in [std::iter](https://doc.rust-lang.org/std/iter/). These include:

* DoubleEndedStream: A stream able to yield elements from both ends.
* ExactSizeStream: A stream that knows its exact length.
* Extend: Extends a collection with the contents of a stream.
* FromStream: Conversion from a Stream.
* FusedStream: A stream that always continues to yield None when exhausted.
* IntoStream: Conversion into a Stream.
* Product: Trait to represent types that can be created by multiplying the elements of a stream.
* Stream: An asynchronous stream of values.
* Sum: Trait to represent types that can be created by summing up a stream.

As detailed in previous sections, the migrations to add these traits are out of scope for this RFC.

## Async iteration syntax

Currently, if someone wishes to iterate over a `Stream` as defined in the `futures` crate,
they are not able to use  `for` loops, they must use `while let` and `next/try_next` instead.

We may wish to extend the `for` loop so that it works over streams as well. 

```rust
#[async]
for elem in stream { ... }
```

One of the complications of using `while let` syntax is the need to pin.
A `for` loop syntax that takes ownership of the stream would be able to
do the pinning for you. 

We may not want to make sequential processing "too easy" without also enabling
parallel/concurrent processing, which people frequently want. One challenge is
that parallel processing wouldn't naively permit early returns and other complex
control flow. We could add a `par_stream()` method, similar to 
[Rayon's](https://github.com/rayon-rs/rayon) `par_iter()`.

Designing this extension is out of scope for this RFC. However, it could be prototyped using procedural macros today.

## "Lending" streams

There has been much discussion around lending streams (also referred to as attached streams).

### Definitions

[Source](https://smallcultfollowing.com/babysteps/blog/2019/12/10/async-interview-2-cramertj-part-2/#the-need-for-streaming-streams-and-iterators)


In a **lending** stream (also known as an "attached" stream), the `Item` that gets 
returned by `Stream` may be borrowed from `self`. It can only be used as long as 
the `self` reference remains live.

In a **non-lending** stream (also known as a "detached" stream), the `Item` that 
gets returned by `Stream` is "detached" from self. This means it can be stored 
and moved about independently from `self`.

This RFC does not cover the addition of lending streams (streams as implemented through 
this RFC are all non-lending streams). Lending streams depend on [Generic Associated Types](https://rust-lang.github.io/rfcs/1598-generic_associated_types.html), which are not (at the time of this RFC) stable.

We can add the `Stream` trait to the standard library now and delay
adding in this distinction between the two types of streams - lending and
non-lending. The advantage of this is it would allow us to copy the `Stream`
trait from `futures` largely 'as is'. 

The disadvantage of this is functions that consume streams would 
first be written to work with `Stream`, and then potentially have 
to be rewritten later to work with `LendingStream`s.

### Current Stream Trait

```rust
pub trait Stream {
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

### Potential Lending Stream Trait

```rust
trait LendingStream<'s> {
    type Item<'a> where 's: 'a;

    fn poll_next<'a>(
        self: Pin<&'a mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item<'a>>>;
}

impl<S> LendingStream for S
where
    S: Stream,
{
    type Item<'_> = S::Item;
    
    fn poll_next<'s>(
        self: Pin<&'s mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item<'s>>> {
        Stream::poll_next(self, cx)
    }
}
```

This is a "conversion" trait such that anything which implements `Stream` can also implement 
`LendingStream`.

This trait captures the case where we re-use internal buffers. This would be less flexible for 
consumers, but potentially more efficient. Types could implement the `LendingStream` 
where they need to re-use an internal buffer and `Stream` if they do not. There is room for both.

We would also need to pursue the same design for iterators - whether through adding two traits
or one new trait with a "conversion" from the old trait.

This also brings up the question of whether we should allow conversion in the opposite way - if
every non-lending stream can become a lending one, should _some_ lending streams be able to 
become non-lending ones? 

**Coherence**

The impl above has a problem. As the Rust language stands today, we cannot cleanly convert 
impl Stream to impl LendingStream due to a coherence conflict.

If you have other impls like:

```rust
impl<T> Stream for Box<T> where T: Stream
```

and

```rust
impl<T> LendingStream for Box<T> where T: LendingStream
```

There is a coherence conflict for `Box<impl Stream>`, so presumably it will fail the coherence rules. 

[More examples are available here](https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=a667a7560f8dc97ab82a780e27dfc9eb).

Resolving this would require either an explicit “wrapper” step or else some form of language extension.

It should be noted that the same applies to Iterator, it is not unique to Stream.

We may eventually want a super trait relationship available in the Rust language

```rust
trait Stream: LendingStream
```

This would allow us to leverage `default impl`.

These use cases for lending/non-lending streams need more thought, which is part of the reason it 
is out of the scope of this particular RFC.

## Generator syntax
[generator syntax]: #generator-syntax

In the future, we may wish to introduce a new form of function - 
`gen fn` in iterators and `async gen fn` in async code that
can contain `yield` statements. Calling such a function would
yield a `impl Iterator` or `impl Stream`, for sync and async 
respectively. Given an "attached" or "borrowed" stream, the generator
could yield references to local variables. Given a "detached"
or "owned" stream, the generator could yield owned values
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
fn foo() -> impl Stream<Item = Value>
```

If we introduce `-> impl Stream` first, we will have to permit `LendingStream` in the future. 
Additionally, if we introduce `LendingStream` later, we'll have to figure out how
to convert a `LendingStream` into a `Stream` seamlessly.

### Differences between Iterator generators and Async generators

We want `Stream` and `Iterator` to work as analogously as possible, including when used with generators. However, in the current design, there are some crucial differences between the two. 

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

With streams, the core interface _is_ pinned, so pinning occurs at the last moment.

The general shape would be 

```rust
async_gen_fn().adapter1().adapter2().pin_somehow()
```

Pinning at the end, like with a stream, lets you build and return those adapters and then apply pinning at the end. This may be the more efficient setup and implies that, in order to have a `gen fn` that produces iterators, we will need to potentially disallow borrowing yields or implement some kind of `PinnedIterator` trait that can be "adapted" into an iterator by pinning.

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

Another key difference between `Iterators` and `Streams` is that futures are ultimately passed to some executor API like spawn which expects a `'static` future. To achieve that, the futures contain all the state they need and references are internal to that state. Iterators are almost never required to be `'static` by the APIs that consume them.

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
