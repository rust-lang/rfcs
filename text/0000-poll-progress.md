- Feature Name: `poll_progress`
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: [rust-lang/rfcs#4020](https://github.com/rust-lang/rfcs/pull/4020)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary
[summary]: #summary

The current [`AsyncIterator`] trait ([RFC 2996]) is incompatible with the
"strict" [`Future`] contract ([RFC 4019][strict_future_rfc]), which requires us
to poll futures promptly when they request a wakeup. The only way to poll an
`AsyncIterator` today is `poll_next`, but we do not and generally cannot call
that method while control is suspended in e.g. the body of a `for await` loop
([#118898][for_await]).

To fix this, add a required `poll_progress` method to the `AsyncIterator`
trait. Call it automatically whenever the body of a `for await` loop is
pending. In `async gen` blocks and functions ([#117078][gen_blocks]), also
forward `poll_progress` while control is suspended at a `yield` in a `for
await` body. Document a "strict" `AsyncIterator` contract requiring other
consumers and adapters to behave analogously. To emphasize the new contract
requirements, define a `PollNext<_>` enum for `poll_next` to return, replacing
`Poll<Option<_>>`.

[`AsyncIterator`]: https://doc.rust-lang.org/std/async_iter/trait.AsyncIterator.html
[RFC 2996]: https://rust-lang.github.io/rfcs/2996-async-iterator.html
[for_await]: https://github.com/rust-lang/rust/issues/118898
[gen_blocks]: https://github.com/rust-lang/rust/issues/117078

To avoid burying the lede: Effectively deprecating [`StreamExt::next`] will
probably be the most controversial part of this RFC. See ["What about
`.next()`?"](#what-about-next) in the Drawbacks section.

## Motivation
[motivation]: #motivation

Since the `AsyncIterator` trait is still unstable, iteration in the async
ecosystem today is organized around the [`Stream`] trait from the `futures`
crate. Apart from the name, the two traits currently have the same shape:

[`Stream`]: https://docs.rs/futures/latest/futures/stream/trait.Stream.html

```rs
pub trait AsyncIterator { // or `pub trait Stream`
    type Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>>;

    fn size_hint(&self) -> (usize, Option<usize>) { (0, None) }
}
```

In this interface, everything an async iterator does happens in `poll_next`.
Consider how that works in the context of a `for await` loop:

```rs
for await _ in my_iter {
    do_work().await;
}
```

When control is at the top, the `for await` loop calls `poll_next` on
`my_iter`[^into] until it either yields an item (currently `Ready(Some(_))`) or
indicates that it's done (currently `Ready(None)`). Then, when control moves
into `do_work`, the loop stops driving `my_iter`. That applies necessary
"backpressure" to async iterators, and it's mostly by design. But it can be a
problem if `my_iter` wraps multiple concurrent futures or other iterators
internally, because pausing them at arbitrary await points isn't generally
correct. Here's an example where this causes a deadlock that looks like it
should be impossible in a straight-line reading of the code:

[^into]: For completeness, the `for await` loop calls
    [`IntoAsyncIterator::into_async_iter`][into_async_iter] first, the same way
    a regular `for` loop calls [`IntoIterator::into_iter`][into_iter], but
    there are no nontrivial implementations of `IntoAsyncIterator` today.

[into_async_iter]: https://doc.rust-lang.org/std/async_iter/trait.IntoAsyncIterator.html
[into_iter]: https://doc.rust-lang.org/std/iter/trait.IntoIterator.html

```rs
// `do_work` takes a private lock, sleeps briefly, and releases it. A deadlock here shouldn't be possible.
async fn do_work() {
    static LOCK: Mutex<()> = Mutex::const_new(());
    let _guard = LOCK.lock().await;
    sleep(Duration::from_millis(10)).await;
}

#[tokio::main]
async fn main() {
    // `my_iter` merges two child iterators, each of which wraps a `do_work` future.
    let my_iter = once(do_work()).merge(once(do_work()));
    for await _ in my_iter {
        // This deadlocks! The second `do_work` future above is holding `LOCK`, but we've stopped polling it.
        do_work().await;
    }
}
```

[`Merge`] doesn't implement `AsyncIterator` today, so this example doesn't
compile as written, but you can run it on stable by replacing `for await` with
[`for_each`] ([playground link][for_each_playground]). When control enters the
loop body, the right side of the `Merge` still holds a `do_work` future, which
has tried to acquire `LOCK` and taken a spot in its waiters queue. The call to
`do_work` in the body tries to acquire `LOCK` again, but the waiter at the
front of the queue never wakes up, so it's deadlocked. This problem often comes
up with ["fancy" async iterators like `FuturesUnordered` or `buffered`
streams][barbara], but the example above uses `merge` to emphasize that simple
concurrent combinators have the same problem.

[for_each_playground]: <https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&code=use+futures%3A%3Astream%3A%3A%7BStreamExt+as+_%2C+once%7D%3B%0Ause+tokio%3A%3Async%3A%3AMutex%3B%0Ause+tokio%3A%3Atime%3A%3A%7BDuration%2C+sleep%7D%3B%0Ause+tokio_stream%3A%3AStreamExt+as+_%3B%0A%0A%2F%2F+%60do_work%60+takes+a+private+lock%2C+sleeps+briefly%2C+and+releases+it.%0A%2F%2F+A+deadlock+here+shouldn%27t+be+possible.%0Aasync+fn+do_work%28%29+%7B%0A++++static+LOCK%3A+Mutex%3C%28%29%3E+%3D+Mutex%3A%3Aconst_new%28%28%29%29%3B%0A++++let+_guard+%3D+LOCK.lock%28%29.await%3B%0A++++sleep%28Duration%3A%3Afrom_millis%2810%29%29.await%3B%0A%7D%0A%0A%23%5Btokio%3A%3Amain%5D%0Aasync+fn+main%28%29+%7B%0A++++%2F%2F+%60my_iter%60+merges+two+child+iterators%2C+each+of+which+wraps+a%0A++++%2F%2F+%60do_work%60+future.%0A++++let+my_iter+%3D+once%28do_work%28%29%29.merge%28once%28do_work%28%29%29%29%3B%0A++++my_iter%0A++++++++.for_each%28%7C_%7C+async+%7B%0A++++++++++++%2F%2F+This+deadlocks%21+One+of+the+%60do_work%60+futures+above%0A++++++++++++%2F%2F+is+holding+%60LOCK%60%2C+but+we%27ve+stopped+polling+it.%0A++++++++++++println%21%28%22We+make+it+here...%22%29%3B%0A++++++++++++do_work%28%29.await%3B%0A++++++++++++println%21%28%22...but+not+here%21%22%29%3B%0A++++++++%7D%29%0A++++++++.await%3B%0A%7D>

To avoid these sorts of deadlocks, and other hard-to-diagnose hangs and
latencies, futures and async iterators need to continuously drive any other
futures or async iterators they contain. That's why the "strict" `Future`
contract ([RFC 4019][strict_future_rfc]) requires us to poll or drop a future
promptly when it requests a wakeup. At a high level, that guarantees steady
control flow through `async` blocks and functions until they return or get
cancelled. We need a "strict" `AsyncIterator` contract that requires the same,
that we poll or drop an async iterator promptly when it requests a wakeup. At a
high level, that would guarantee steady control flow through `async gen` blocks
and functions until they return, get cancelled, or _yield an item_.
Backpressure is important for async iterators, but we should only apply it at
yield points, not at await points.[^yield_points]

[^yield_points]: Sometimes (including in [the original `async`/`await`
    RFC][rfc2394]) the term "yield point" refers to anywhere we yield _control_
    of the current thread, including `.await` expressions. But here in the
    context of `AsyncIterator` and `async gen fn`, we'll use "yield point" to
    refer specifically to returning an item from `poll_next`, which is what the
    `yield` keyword does. An "await point" is anywhere `poll` or `poll_next`
    could return `Pending`, including `.await` expressions and `for await`
    items. The general term that covers both yield points and await points is
    "suspension points".

[rfc2394]: https://rust-lang.github.io/rfcs/2394-async_await.html

In the example above, the loop has driven `Merge` to a yield point, and `Merge`
has driven its first [`Once`] child to a yield point, but its second `Once`
child is suspended at an await point. The `do_work` future that the second
`Once` owns is also suspended at an await point, and it has arranged a wakeup
for itself. We need `Merge` to re-poll its children when that wakeup
fires.[^bad_merge] That means we also need `for await` to re-poll `Merge`. How?

[`Once`]: https://docs.rs/futures/latest/futures/stream/fn.once.html

[^bad_merge]: Technically another option here is for `Merge` to delay yielding
    its first item until both its children have yielded. We could generalize
    that to every `AsyncIterator` with a rule like "Don't yield an item while
    any of your child iterators/futures are pending." In theory that would fix
    all the deadlocks in this RFC just as well as `poll_progress` does. The
    problem is that no one actually wants `Merge` to work like that. That would
    be more like [`Zip`], in its timing if not its item type. That rule would
    also make e.g. [`FuturesUnordered`] behave more like [`JoinAll`].

[`Zip`]: https://docs.rs/futures/latest/futures/stream/trait.StreamExt.html#method.zip
[`JoinAll`]: https://docs.rs/futures/latest/futures/future/fn.join_all.html

This is where `poll_progress` comes in. It lets us say, "We're not ready for
another item, but here's your chance to poll any concurrent futures or async
iterators you're responsible for." `for await` loops will call `poll_progress`
whenever their body is pending. How does that break our deadlock? There's a lot
of async machinery involved, most of which already exists today, but here's the
sequence of events ([playground link][poll_progress_playground]):

[poll_progress_playground]: https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&gist=7f761d927209833bc25c3cb1564236ba

1. When `do_work` in the loop body sees that `LOCK` is already taken, it adds
   its `Waker` to the lock's waiters queue before reporting `Pending`.
2. **New:** Because control in `main` is in the body of a `for await` loop,
   `main` calls `poll_progress` on the loop's iterator before reporting
   `Pending` itself.
3. **New:** `Merge::poll_progress` polls[^rule] its children, and its second
   child finishes acquiring `LOCK`, registers its own `Waker` with the
   runtime's sleep/timer system, and returns `Pending`. `Merge::poll_progress`
   then returns `Pending` itself, and `main`'s `poll` function also returns
   `Pending`.
4. After 10 ms, the runtime invokes the sleeping `Waker` from step 3 and polls
   `main` again.
5. `do_work` in the loop body tries to acquire the lock again, fails again, and
   reports `Pending` again.
6. **New:** As in step 2, `main` calls `Merge::poll_progress` again. It polls
   its children, and this time the second child drops its lock guard and yields
   `()`. `Merge` stores that item internally.
7. Dropping that guard invokes the `Waker` that the body registered in step 1
   (and reregistered in step 5), so the runtime re-polls `main` immediately.
8. This time `do_work` in the loop body successfully acquires `LOCK` and starts
   its own sleep. There's no further lock contention, and after a couple
   iterations the loop finishes, and `main` returns.

[^rule]: Here we're glossing over whether `Merge::poll_progress` should call
    `poll_next` or `poll_progress` on its children. It could conceivably work
    either way, but we're going to require `poll_next` for the second child in
    this case. See the "`PollNext::Pending` rule" in the [following
    section](#implementing-asynciterator) and also the [discussion in the
    rationales](#why-not-allow-poll_progress-at-any-time).

Expanding on steps 2 and 6 above: When control resumes in the body of a `for
await` loop, we don't know if it was the body or the iterator (or both) that
requested the current wakeup, so we need to poll both of them. If control
reaches the end of the body immediately, then we'll proceed to the next loop
iteration and call `poll_next` immediately, which is good enough. But if some
future in the body returns `Pending`, then we need to call `poll_progress` "on
the way out" before returning `Pending` to our own caller, in case it was the
iterator that requested the current wakeup.

At a high level, `poll_progress` satisfies the "strict" `Future` contract and
provides a steady control flow guarantee for code running inside an
`AsyncIterator`. This makes reasoning about async locking in these cases "only"
as difficult as regular locking plus cancellation, instead of even more
difficult than that.

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

### `AsyncIterator` intro

As its name suggests, `AsyncIterator` is the async version of [`Iterator`].
`Iterator` is the trait at the heart of `for` loops and other forms of
synchronous iteration, and `AsyncIterator` is the trait at the heart of `for
await` loops and other forms of asynchronous iteration.

[`Iterator`]: https://doc.rust-lang.org/std/iter/trait.Iterator.html

Async code can and often does use regular iterators and `for` loops too, as
long as their next item is always immediately available, like loops over a
range or a collection. But async functions shouldn't use iterators that can
block waiting for input, like [`std::io::Lines`] or [`std::sync::mpsc::Iter`].
The main purpose of `AsyncIterator` is to support iteration that waits for
input in a way that's compatible with async code.

[`std::io::Lines`]: https://doc.rust-lang.org/std/io/struct.Lines.html
[`std::sync::mpsc::Iter`]: https://doc.rust-lang.org/std/sync/mpsc/struct.Iter.html

Unlike regular iterators, an `AsyncIterator` can also do concurrent work while
its caller is processing an item. For example:

```rs
for await jpeg in fetch_images() {
    save_image(jpeg).await;
}
```

Depending on how it's implemented, `fetch_images` could be downloading the next
`jpeg` concurrently while `save_image` is running. A regular iterator could
potentially do something like that by spawning a thread, but threads complicate
borrowing and short-circuiting and usually require heap allocation. Async
iterators can do concurrent work without threads or allocations, and with full
support for local borrowing and straightforward behavior for `break` and
`return` (cancellation).

### Implementing `AsyncIterator`

> This section describes the proposed `poll_progress` method and the proposed
> new return type of `poll_next`.

The core of the `AsyncIterator` trait is the `poll_next` and `poll_progress`
methods, which look like this:

```rs
use core::task::Poll;

enum PollNext<Item> {
    Item(Item),
    Pending,
    Done,
}

trait AsyncIterator {
    type Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> PollNext<Self::Item>;

    fn poll_progress(self: Pin<&mut Self>, cx: &mut Context) -> Poll<()>;
}
```

`poll_next` is a cross between `Iterator::next` and `Future::poll`. Like
`Iterator::next`, it can return an item or indicate that the iterator is done.
And like `Future::poll`, it can also return `Pending`, meaning that no item is
currently ready and a wakeup has been registered. When a `for await` loop wants
the next item from an async iterator, it calls `poll_next`.

`poll_progress` is what allows an async iterator to do background work when the
caller isn't ready for another item. It returns `Pending` when more progress
might be possible in the future, and it also registers a wakeup in that case.
When no more internal progress is possible without calling `poll_next` again,
`poll_progress` returns `Ready`. When an `.await` in the body of a `for await`
loop is pending, the loop calls `poll_progress` on its iterator before
reporting pending itself.

Like `Future::poll`, the return values of these methods impose certain
requirements on their callers. But while `Future` only has two possible return
states to contend with, `AsyncIterator` has _five_:

| Previous return | Requirements |
| --- | --- |
| `poll_next` returned `PollNext::Item` | You must <ins><strong>poll again promptly,</strong></ins> either `poll_next` if you want another item or `poll_progress` if not. |
| `poll_next` returned `PollNext::Pending` | You must call `poll_next` again at the next wakeup. <ins><strong>`poll_progress` is not allowed,</strong></ins> and it might panic or otherwise misbehave (within the bounds of safe code). |
| `poll_next` returned `PollNext::Done` | You must not poll again. Either `poll_next` or `poll_progress` might panic or misbehave. |
| `poll_progress` returned `Poll::Pending` | You can call `poll_next` whenever you want another item, otherwise you must call `poll_progress` again at the next wakeup. |
| `poll_progress` returned `Poll::Ready` | No requirements. You can call `poll_next` whenever you want another item, or you can stop polling. Further calls to `poll_progress` are allowed but usually have no effect. |

When an `AsyncIterator` is first created, either `poll_next` or `poll_progress`
is valid. Although a `for await` loop will always start by calling `poll_next`,
other consumers (e.g. [`Chain`], on its right side) may start with
`poll_progress`. Also, cancelling an iterator by dropping it is allowed at any
time.

Expanding on the first two requirements (underlined) in the table above:

- **The `PollNext::Item` rule:** When `poll_next` returns an item, it's not
  required to register any wakeups. That's important for performance, because
  there might be many items ready to return, and we don't want to trigger
  redundant wakeups for each of them or cause an extra round trip through the
  executor. (This much is also true of `Stream` today.) On the other hand,
  if/when we don't want to call `poll_next` anymore, we need to call
  `poll_progress` before returning pending ourselves, to give the iterator a
  chance to finish its own polling responsibilities and register wakeups.
- **The `PollNext::Pending` rule:** When `poll_next` is pending, we have to
  call `poll_next` again at the next wakeup; we can't change our minds about
  wanting an item and switch to `poll_progress`. (The only valid way to change
  our minds is to cancel the whole iterator by dropping it.) This rule is aimed
  at concurrent combinators that "merge" multiple async iterators together.
  After a child iterator yields an item, those combinators should keep driving
  their other children with `poll_next` internally until each of them yields an
  item. That gives us consistent control flow through chains of adapters and
  combinators, and it means that most async iterators don't need to allocate
  buffer space for an item. The rationale section [discusses this rule
  further](#why-not-allow-poll_progress-at-any-time).

Let's look at the implementation of an async iterator adapter, `Buffer1`, which
wraps another `AsyncIterator` and pre-fetches its next item in `poll_progress`.
This turns any `AsyncIterator` into a concurrent one that does work in the
background:

```rs
struct Buffer1<Iter: AsyncIterator> {
    #[pin] // TODO: a standard way to do structural pinning in examples?
    inner: Fuse<Iter>,
    item: Option<Iter::Item>,
}

impl<Iter: AsyncIterator> AsyncIterator for Buffer1<Iter> {
    type Item = Iter::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> PollNext<Iter::Item> {
        let this = self.project();
        if let Some(item) = this.item.take() { // If we have a buffered `item`, yield it.
            PollNext::Item(item)
        } else { // Otherwise poll the `inner` async iterator.
            this.inner.poll_next(cx)
        }
    }

    fn poll_progress(self: Pin<&mut Self>, cx: &mut Context) -> Poll<()> {
        let mut this = self.project();
        if this.item.is_none() { // If we don't have a buffered `item`, try to get one.
            match this.inner.as_mut().poll_next(cx) {
                PollNext::Item(item) => {
                    *this.item = Some(item);
                    this.inner.poll_progress(cx) // `poll_progress` is required before returning.
                }
                PollNext::Pending => Poll::Pending, // `poll_progress` is forbidden.
                PollNext::Done => Poll::Ready(()), // `inner` is fused.
            }
        } else { // If we already have a buffered `item`, forward `poll_progress`.
            this.inner.poll_progress(cx)
        }
    }
}
```

Look at how `Buffer1::poll_next` returns the result of `inner.poll_next`
directly, without doing any further polling. That might seem like it violates
the "`PollNext::Item` rule" about polling again promptly, however, the same
rule applies _to the caller_. The caller must poll again, and when they do,
`inner` will get polled again. However, `Buffer1::poll_progress` doesn't impose
the same requirements on its caller, so its `Item` branch needs to call
`inner.poll_progress` explicitly. (It would also need to handle `Done`, but
[`Fuse`] takes care of that.)

[`Fuse`]: https://docs.rs/futures/latest/futures/stream/struct.Fuse.html

Note also that `Buffer1::poll_progress` follows the "`PollNext::Pending` rule"
and doesn't call `inner.poll_progress` after `inner.poll_next` returns
`Pending`. The next poll will be either `Buffer1::poll_next` if the caller
wants another item, or else `Buffer1::poll_progress` calling `inner.poll_next`
again after a wakeup. When we do have a buffered item, we know the last call to
`inner.poll_next` returned `Item`, and we forward `poll_progress`.

### [`FuturesUnordered`] docs

`FuturesUnordered` is an async iterator over the results of the futures it
contains. Looping over it gives you each of the results as they arrive, and
pending futures keep making progress concurrently with the loop body. For
example:

```rs
use futures::stream::FuturesUnordered;
use tokio::time::{Duration, sleep};

async fn foo(secs: u64, message: &str) -> &str {
    sleep(Duration::from_secs(secs)).await;
    println!("foo {message} woke up");
    message
}

#[tokio::main]
async fn main() {
    let futures = FuturesUnordered::new();
    futures.push(Box::pin(foo(0, "A")));
    futures.push(Box::pin(foo(1, "B")));
    for await message in futures {
        println!("got {message}");
        sleep(Duration::from_secs(2)).await;
    }
}
```

The first `foo` future does a zero-length sleep, so we see `foo A woke up` and
`got A` immediately. The second `foo` future runs concurrently, so we see `foo
B woke up` after one second, while the loop body is in the middle of its own
two-second sleep. The output of the second `foo` future is buffered, and once
the two-second sleep is finished, we loop around and print `got B` immediately.

(See "What about `.next()`?" below for a more complicated case.)

### [`tokio_stream::StreamExt::merge`] docs

[`tokio_stream::StreamExt::merge`]: https://docs.rs/tokio-stream/latest/tokio_stream/trait.StreamExt.html#method.merge

Combine two streams into one by interleaving the outputs of both as they're
produced. When one side of the merge yields an item, the other side continues
making progress in the background until it also yields an item. For example:

```rs
use futures::stream::once;
use tokio::time::{Duration, sleep};
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() {
    let stream_a = once(async { "A" });
    let stream_b = once(async {
        sleep(Duration::from_secs(1)).await;
        println!("stream_b woke up");
        "B"
    });
    for await item in stream_a.merge(stream_b) {
        println!("got {item}");
        sleep(Duration::from_secs(2)).await;
    }
}
```

Here we see `got A` immediately, and then the loop body starts its two-second
sleep. The other stream continues in the meantime, so we see `stream_b woke up`
after one second, and the item `"B"` is buffered. When the two-second sleep is
finished, we loop around and print `got B` immediately.

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

### `for await`

We can't desugar this RFC's version of `for await` syntax into a loop ([like
the reference does for the `for` keyword][for]), because we'd need to treat the
body as a future (e.g. `async { /* loop body */ }`) to intercept `Pending` and
call `poll_progress` in the right place, and that's incompatible with `return`,
`?`, `break`, or `continue` in the body.

[for]: https://doc.rust-lang.org/reference/expressions/loop-expr.html#r-expr.loop.for.desugar

Instead we can describe the steps `for await` takes internally, [the same way
`.await` is described][await]. To evaluate the expression:

[await]: https://doc.rust-lang.org/reference/expressions/await-expr.html#r-expr.await.effects

```
'label: for await PATTERN in ITER_EXPR {
    /* loop body */
}
```

We execute the following steps:

1. Create an async iterator with
   `IntoAsyncIterator::into_async_iter(ITER_EXPR)`.
2. Pin it using `Pin::new_unchecked`.
3. Poll it by calling the `AsyncIterator::poll_next` method and passing it the
   current task context.
4. If the call to `poll_next` returns `PollNext::Pending`, then the surrounding
   async context returns `Poll::Pending` (or in an `async gen fn`,
   `PollNext::Pending`), suspending its state so that, when it's re-polled,
   execution returns to step 3.
5. If the call to `poll_next` returns `PollNext::Done`, then the loop drops the
   async iterator and evaluates to `()`. Skip the remaining steps.
6. If the call to `poll_next` returns `PollNext::Item(item)`, then `item` is
   matched against the irrefutable `PATTERN`.
7. Control proceeds through the loop body, with the bindings from `PATTERN` in
   scope. When any `.await` expression or `for await` item (i.e. step 3 above,
   if another `for await` loop is nested within this one) in the body is
   pending, call `poll_progress` on the async iterator before reporting pending
   from the surrounding async context.
8. \[`break`, `continue`, and looping and diverging control flow as usual,
   dropping the iterator on exit\]

Note that if `for await` loops are nested, a pending expression in an inner
loop triggers step 7 above for _all_ the containing loops, starting with the
innermost.

### `async gen fn`

The `AsyncIterator` contract limits the cases where it's valid to call
`poll_progress`. There are only two:

1. `poll_next` has never been called.
2. The previous call to `poll_next` returned `Item`.

In an `async gen fn` -- that is, in the anonymous `AsyncIterator`
implementation returned by an `async gen fn` -- those cases correspond to the
initial state where control hasn't yet entered the body, and to the states
where control is suspended at a `yield` expression. Importantly, in the states
where control is suspended at a pending `.await` or `for await`, we know that
the previous call to `poll_next` returned `Pending`, so calling `poll_progress`
is not valid. (The implementation should probably panic if it receives such a
call, though that isn't required by the contract. See the [unresolved questions
section](#should-invalid-calls-to-poll_progress-always-panic).) This gives us
an important simplification: **Only `poll_next` advances control through the
body of an `async gen fn`.**

In contrast, `poll_progress` has a limited responsibility: If control is
suspended at a `yield` in the body of a `for await` loop, then the `async gen
fn`'s implementation of `poll_progress` calls ("forwards") `poll_progress` on
that loop's iterator. If `for await` loops are nested, there may be multiple
such iterators, and `poll_progress` gets forwarded to all of them, starting
with the innermost. If any of the forwarded calls returns `Pending`, then
`poll_progress` returns `Pending`, otherwise it returns `Ready`.

Note that when control reaches a `yield`, that by itself does not necessarily
imply a call to `poll_progress` on any active iterators. If the caller doesn't
encounter some pending `.await` before requesting the next item, then nothing
will call `poll_progress`, and control will resume from the `yield` promptly
instead (unless cancelled). In terms of the `AsyncIterator` contract, that's
the case where a call to `poll_next` that returns `Item` is followed by another
prompt call to `poll_next`, rather than a call to `poll_progress`.

For example:

```rs
async gen fn foo() {
    for await _ in bar() {
        for await _ in baz() {
            // While this expression is pending, `foo`'s `poll_next` function calls `poll_progress` on the
            // `baz` and `bar` async iterators before reporting pending itself. The `AsyncIterator` contract
            // forbids calling `foo`'s `poll_progress` function in this state, and `foo` will panic if you do.
            do_work().await;

            // While control is suspended at this `yield`, `foo`'s `poll_progress` function calls
            // `poll_progress` on the `baz` and `bar` async iterators, and it reports pending if either of
            // those calls is pending. Note that reaching this `yield` does *not* automatically call
            // `poll_progress` on `baz` or `bar`. Those calls only come if/when the caller encounters a
            // pending await in their own loop and calls `poll_progress` on `foo`.
            yield;
        }
    }
}
```

## Drawbacks
[drawbacks]: #drawbacks

### What about `.next()`?

The main way users interact with `Stream` in the async ecosystem today is the
[`StreamExt::next`] method, which returns a future representing the next item
in the stream. But the `AsyncIterator` contract proposed in this RFC isn't
compatible with `next`, because the `Next` future is short-lived, and it can't
drive the iterator after it's gone. **`AsyncIterator` won't have a `next`
method,** and we'll need a migration plan for existing callers.

First, the problem. Here's a version of [the `merge` deadlock
above][motivation] that `poll_progress` can't fix ([playground link][next1]):

[next1]: <https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&code=use+futures%3A%3Astream%3A%3Aonce%3B%0Ause+std%3A%3Apin%3A%3Apin%3B%0Ause+tokio%3A%3Async%3A%3AMutex%3B%0Ause+tokio%3A%3Atime%3A%3A%7BDuration%2C+sleep%7D%3B%0Ause+tokio_stream%3A%3AStreamExt+as+_%3B%0A%0A%2F%2F+%60do_work%60+takes+a+private+lock%2C+sleeps+briefly%2C+and+releases+it.%0A%2F%2F+A+deadlock+here+shouldn%27t+be+possible.%0Aasync+fn+do_work%28%29+%7B%0A++++static+LOCK%3A+Mutex%3C%28%29%3E+%3D+Mutex%3A%3Aconst_new%28%28%29%29%3B%0A++++let+_guard+%3D+LOCK.lock%28%29.await%3B%0A++++sleep%28Duration%3A%3Afrom_millis%2810%29%29.await%3B%0A%7D%0A%0A%23%5Btokio%3A%3Amain%5D%0Aasync+fn+main%28%29+%7B%0A++++let+mut+my_iter+%3D+pin%21%28once%28do_work%28%29%29.merge%28once%28do_work%28%29%29%29%29%3B%0A++++_+%3D+my_iter.next%28%29.await%3B%0A++++println%21%28%22We+make+it+here...%22%29%3B%0A++++do_work%28%29.await%3B%0A++++println%21%28%22...but+not+here%21%22%29%3B%0A%7D>

```rs
let mut my_iter = pin!(once(do_work()).merge(once(do_work())));
_ = my_iter.next().await;
do_work().await; // Deadlock!
```

In theory we're supposed to call `my_iter.poll_progress` while
`do_work().await` is pending, but there isn't a `for await` loop doing that for
us, and the `Next` future is gone. Also, here's another deadlock without the
`merge`, where the problem is cancellation rather than concurrency ([playground
link][next2]):

[next2]: <https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&code=use+futures%3A%3Astream%3A%3Aonce%3B%0Ause+std%3A%3Apin%3A%3Apin%3B%0Ause+tokio%3A%3Async%3A%3AMutex%3B%0Ause+tokio%3A%3Atime%3A%3A%7BDuration%2C+sleep%2C+timeout%7D%3B%0Ause+tokio_stream%3A%3AStreamExt+as+_%3B%0A%0A%2F%2F+%60do_work%60+takes+a+private+lock%2C+sleeps+briefly%2C+and+releases+it.%0A%2F%2F+A+deadlock+here+shouldn%27t+be+possible.%0Aasync+fn+do_work%28%29+%7B%0A++++static+LOCK%3A+Mutex%3C%28%29%3E+%3D+Mutex%3A%3Aconst_new%28%28%29%29%3B%0A++++let+_guard+%3D+LOCK.lock%28%29.await%3B%0A++++sleep%28Duration%3A%3Afrom_millis%2810%29%29.await%3B%0A%7D%0A%0A%23%5Btokio%3A%3Amain%5D%0Aasync+fn+main%28%29+%7B%0A++++let+mut+my_iter+%3D+pin%21%28once%28do_work%28%29%29%29%3B%0A++++_+%3D+timeout%28Duration%3A%3Afrom_millis%281%29%2C+my_iter.next%28%29%29.await%3B%0A++++println%21%28%22We+make+it+here...%22%29%3B%0A++++do_work%28%29.await%3B%0A++++println%21%28%22...but+not+here%21%22%29%3B%0A%7D>

```rs
let mut my_iter = pin!(once(do_work()));
_ = tokio::time::timeout(Duration::from_millis(1), my_iter.next()).await;
do_work().await; // Deadlock!
```

In both cases, there's an `AsyncIterator` that's ready to make progress -- in
the first case `LOCK` has already invoked a `Waker`, and in the second case
`sleep` eventually does -- and somebody needs to poll it or drop it promptly.
But who? The caller? What we're seeing is that, to satisfy the proposed
contract, whatever's driving an `AsyncIterator` generally needs to _own_ it.
That's the case with `for await` loops, and with terminal consumers like
[`for_each`] and [`collect`], but it's a problem for `next`.

[`for_each`]: https://docs.rs/futures/latest/futures/stream/trait.StreamExt.html#method.for_each
[`collect`]: https://docs.rs/futures/latest/futures/stream/trait.StreamExt.html#method.collect

Now, the alternatives. The most common use case for `next` today is loops like
this:

```rs
let mut stream = pin!(...);
while let Some(item) = stream.next().await { ... }
```

These can be replaced with `for await` loops:

```rs
for await item in stream { ... }
```

The `for await` version is more concise and doesn't require pinning, so it's a
nice improvement where it works. Unfortunately, it doesn't work everywhere.
Here's an example of a `next` caller that can't switch to `for await`
([playground link][loop_select]):

[loop_select]: <https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&code=use+futures%3A%3AStreamExt%3B%0Ause+futures%3A%3Astream%3A%3AFuturesUnordered%3B%0Ause+tokio%3A%3Aselect%3B%0Ause+tokio%3A%3Atime%3A%3A%7BDuration%2C+sleep%7D%3B%0A%0Aasync+fn+work%28%29+%7B%0A++++sleep%28Duration%3A%3Afrom_secs%28rand%3A%3Arandom_range%280..5%29%29%29.await%3B%0A%7D%0A%0Aasync+fn+more_work%28%29+-%3E+impl+Future%3COutput+%3D+%28%29%3E+%7B%0A++++sleep%28Duration%3A%3Afrom_secs%281%29%29.await%3B%0A++++work%28%29%0A%7D%0A%0A%23%5Btokio%3A%3Amain%5D%0Aasync+fn+main%28%29+%7B%0A++++let+mut+futures+%3D+FuturesUnordered%3A%3Anew%28%29%3B%0A++++loop+%7B%0A++++++++select%21+%7B%0A++++++++++++%2F%2F+Add+more+jobs+as+they+come+in.%0A++++++++++++job+%3D+more_work%28%29+%3D%3E+%7B%0A++++++++++++++++println%21%28%22got+a+job%22%29%3B%0A++++++++++++++++futures.push%28job%29%3B%0A++++++++++++%7D%0A%0A++++++++++++%2F%2F+Handle+the+outputs+of+running+jobs.%0A++++++++++++Some%28_%29+%3D+futures.next%28%29+%3D%3E+%7B%0A++++++++++++++++println%21%28%22finished+a+job%22%29%3B%0A++++++++++++%7D%0A++++++++%7D%0A++++%7D%0A%7D>

```rs
let mut futures = FuturesUnordered::new();
loop {
    select! {
        // Add more jobs as they come in.
        job = more_work() => futures.push(job),

        // Handle the outputs of running jobs.
        Some(output) = futures.next() => ...
    }
}
```

This caller is iterating over a `FuturesUnordered` and also adding more work to
it in the loop body. We can't reorganize this around a `for await`, because
that would take ownership of `futures`, and `futures.push(job)` wouldn't
compile. Sometimes we can replace things like this with task spawning, but
there are cases where the borrowing and mutability details make that [very
difficult][mini_redis].

In the difficult cases, we can recreate the `next` method in a
`poll_progress`-compatible way using a helper function that takes a closure, in
the style of [`std::thread::scope`]. Here's a [working
proof-of-concept][drive_next], which takes ownership of an async iterator and
provides a handle with `next` and `with_mut` methods on it. This helper fixes
both of the deadlocks above plus [another that the `select!` loop is prone
to][loop_select_deadlock]. It's `no_std`-compatible, but it's also somewhat
complicated. We could consider adding something like it to `core` someday, but
this RFC doesn't propose doing that at first.

[`std::thread::scope`]: https://doc.rust-lang.org/std/thread/fn.scope.html
[drive_next]: https://github.com/oconnor663/drive_async_iterator
[loop_select_deadlock]: <https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&code=use+futures%3A%3AStreamExt%3B%0Ause+futures%3A%3Astream%3A%3AFuturesUnordered%3B%0Ause+tokio%3A%3Aselect%3B%0Ause+tokio%3A%3Async%3A%3AMutex%3B%0Ause+tokio%3A%3Atime%3A%3A%7BDuration%2C+sleep%7D%3B%0A%0Aasync+fn+work%28%29+%7B%0A++++static+LOCK%3A+Mutex%3C%28%29%3E+%3D+Mutex%3A%3Aconst_new%28%28%29%29%3B%0A++++let+_guard+%3D+LOCK.lock%28%29.await%3B%0A++++sleep%28Duration%3A%3Afrom_millis%28rand%3A%3Arandom_range%280..5%29%29%29.await%3B%0A%7D%0A%0Aasync+fn+more_work%28%29+-%3E+impl+Future%3COutput+%3D+%28%29%3E+%7B%0A++++sleep%28Duration%3A%3Afrom_millis%281%29%29.await%3B%0A++++work%28%29%0A%7D%0A%0A%23%5Btokio%3A%3Amain%5D%0Aasync+fn+main%28%29+%7B%0A++++let+mut+futures+%3D+FuturesUnordered%3A%3Anew%28%29%3B%0A++++loop+%7B%0A++++++++select%21+%7B%0A++++++++++++%2F%2F+Add+more+jobs+as+they+come+in.%0A++++++++++++job+%3D+more_work%28%29+%3D%3E+%7B%0A++++++++++++++++println%21%28%22got+a+job%22%29%3B%0A++++++++++++++++futures.push%28job%29%3B%0A++++++++++++%7D%0A%0A++++++++++++%2F%2F+Handle+the+outputs+of+running+jobs.%0A++++++++++++Some%28_%29+%3D+futures.next%28%29+%3D%3E+%7B%0A++++++++++++++++println%21%28%22finished+a+job%22%29%3B%0A++++++++++++++++work%28%29.await%3B+%2F%2F+Deadlock%21%0A++++++++++++%7D%0A++++++++%7D%0A++++%7D%0A%7D>

If we use a helper function like this inside an `async gen fn`, we won't be
able to `yield` in the closure body. The "Future possibilities" section
discusses some [hypothetical syntax](#concurrency-syntax) that would let us
write a helper macro that does support `yield` (and `break` and `continue`).

### What about the blanket impls?

`AsyncIterator` currently has blanket impls that include [`&mut
_`][async_iter_blanket_mut] and [`Pin<&mut _>`][async_iter_blanket_pin]. **We
should remove these impls,** because driving an `AsyncIterator` by reference is
deadlock-prone. (We'll keep `Box<_>` and `Pin<Box<_>>`.) The problem is similar
to what we just saw with `next` above ([playground link][blanket_deadlock]):

[async_iter_blanket_mut]: https://doc.rust-lang.org/std/async_iter/trait.AsyncIterator.html#impl-AsyncIterator-for-%26mut+S
[async_iter_blanket_pin]: https://doc.rust-lang.org/std/async_iter/trait.AsyncIterator.html#impl-AsyncIterator-for-Pin%3CP%3E
[blanket_deadlock]: <https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&code=use+futures%3A%3Astream%3A%3A%7BStreamExt%2C+once%7D%3B%0Ause+std%3A%3Apin%3A%3Apin%3B%0Ause+tokio%3A%3Async%3A%3AMutex%3B%0Ause+tokio%3A%3Atime%3A%3A%7BDuration%2C+sleep%7D%3B%0Ause+tokio_stream%3A%3AStreamExt+as+_%3B%0A%0A%2F%2F+%60do_work%60+takes+a+private+lock%2C+sleeps+briefly%2C+and+releases+it.%0A%2F%2F+A+deadlock+here+shouldn%27t+be+possible.%0Aasync+fn+do_work%28%29+%7B%0A++++static+LOCK%3A+Mutex%3C%28%29%3E+%3D+Mutex%3A%3Aconst_new%28%28%29%29%3B%0A++++let+_guard+%3D+LOCK.lock%28%29.await%3B%0A++++sleep%28Duration%3A%3Afrom_millis%2810%29%29.await%3B%0A%7D%0A%0A%23%5Btokio%3A%3Amain%5D%0Aasync+fn+main%28%29+%7B%0A++++let+my_iter+%3D+pin%21%28once%28do_work%28%29%29.merge%28once%28do_work%28%29%29%29%29%3B%0A++++%2F%2F+The+following+take%2Ffor_each+chain+is+equivalent+to+this+loop%3A%0A++++%2F%2F%0A++++%2F%2F+for+await+_+in+my_iter+%7B%0A++++%2F%2F+++++break%3B%0A++++%2F%2F+%7D%0A++++StreamExt%3A%3Atake%28my_iter%2C+1%29.for_each%28async+%7C_%7C+%7B%7D%29.await%3B%0A++++println%21%28%22We+make+it+here...%22%29%3B%0A++++do_work%28%29.await%3B%0A++++println%21%28%22...but+not+here%21%22%29%3B%0A%7D>

```rs
let mut my_iter = pin!(once(do_work()).merge(once(do_work())));
for await _ in my_iter {
    break;
}
do_work().await; // Deadlock!
```

`poll_progress` is no help here either, because the deadlock is happening after
our `for await` loop is finished. Normally `for await` would drop the iterator
when it short-circuits, but the `pin!` here means we're actually looping over a
`Pin<&mut _>` reference, and dropping that has no effect. This is another
example of how driving an `AsyncIterator` correctly requires _ownership_.
`Pin<&mut _>` should not implement `AsyncIterator`, and this loop should not
compile.

See also the discussion of [the blanket `Future` impl for `Pin<&mut _>`
references][blanket] in [RFC 4019][strict_future_rfc].

[blanket]: https://doc.rust-lang.org/std/future/trait.Future.html#impl-Future-for-Pin%3CP%3E

### What about "lending" async iterators?

A hypothetical `LendingAsyncIterator` would yield items that borrow the
iterator itself. That could let us share a mutable buffer between items, for
example, in exchange for not being allowed to `collect` those items. RFC 2996
[discussed this possibility][lending].

[lending]: https://rust-lang.github.io/rfcs/2996-async-iterator.html#lending-async-iterators

However, `poll_progress` isn't compatible with lending. If the loop body
borrows the iterator, but we also want to call `poll_progress` whenever the
body is pending, that's an unavoidable borrowck conflict. Probably any approach
to supporting "background work" would have the same problem. (Unless both
`poll_progress` and lending iterators used shared references and interior
mutability? Unlikely.)

The need for `poll_progress` came from the following assumptions:

1. We want async iterators like `Merge` and `FuturesUnordered` to wrap multiple
   concurrent child iterators or futures and yield their results as they come.
2. We can't pause those child iterators or futures at random await points.

`LendingAsyncIterator` can't do much about the second requirement, which comes
from the "strict" `Future` contract in [RFC 4019][strict_future_rfc]. But it
might be able to attack the first, either by not wrapping multiple futures, or
by waiting for all the futures it's running to finish before yielding control.
That could be a useful abstraction in many cases, but giving up on `Merge`
seems like too much of a sacrifice for the standard trait that will power `for
await` and `async gen fn`.

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

### Why not allow `poll_progress` at any time?

The "`PollNext::Pending` rule" says that `poll_progress` shouldn't be called
when `poll_next` is pending. That seems kind of arbitrary. What's the point of
that rule? Consider the `poll_progress` implementation of a combinator like
[`Merge`]. _Without_ the `PollNext::Pending` rule, it could be as short as
this:

```rs
fn poll_progress(self: Pin<&mut Self>, cx: &mut Context) -> Poll<()> {
    let this = self.project();
    // XXX: This version doesn't keep track of whether each child's `poll_next` is pending.
    let poll1 = this.left.poll_progress(cx);
    let poll2 = this.right.poll_progress(cx);
    any_pending([poll1, poll2])
}
```

That would be nice and simple (we'll call it a "lazy" `Merge`), but the
downside is that _every other_ async iterator would need to handle the case
where their caller abruptly switches from `poll_next` to `poll_progress` before
they yield an item. For example, think about this hypothetical `async gen fn`:

```rs
async gen fn foo() {
    do_work().await;
    yield;
}
```

If control is in the middle of `do_work`, we can't let it get stuck there, or
else we'll violate the `Future` contract and cause the same deadlocks we saw
above. If the caller could switch to `poll_progress` at any time, then
`poll_progress` would need to keep driving control through the body up to the
next `yield`. However, `poll_progress` shouldn't _always_ drive control to the
next `yield`. If it did, then `for await` would drive _every_ async iterator
concurrently with its loop body, which isn't how it's supposed to work.
Instead, `foo` would need to track whether `poll_next` has been called _at
least once_ before allowing control to enter the body, and before allowing
control to proceed after a `yield`. That's doable, but then would we want
equivalent behavior from adapters like [`Map`] and [`Then`]? Those would need
to add a state flag that they don't have today -- call it `next_item_wanted` --
and they'd also need a rarely used buffer slot for an item. (See the following
section for detailed examples of all of this.)

[`Map`]: https://docs.rs/futures/latest/futures/stream/trait.StreamExt.html#method.map
[`Then`]: https://docs.rs/futures/latest/futures/stream/trait.StreamExt.html#method.then

In practice, the bookkeeping for "if `poll_next` has been called at least once,
`poll_progress` advances control to the next yield and buffers an item" would
look awfully similar to the bookkeeping for "don't call `poll_progress` when
`poll_next` is pending", except moved down a level from the caller to the
callee. Rather than adding complexity and buffer slots to concurrent async
iterators like `Merge` and [`StreamMap`], we'd add complexity and buffer slots
to _every_ async iterator.

With the `PollNext::Pending` rule, we don't get to write the "lazy" version of
`Merge::poll_progress` above, and instead we have to do state tracking and
conditional buffering there. That's a downside. The upside is that
`Then::poll_progress` looks like this:

```rs
fn poll_progress(self: Pin<&mut Self>, cx: &mut Context) -> Poll<()> {
    assert!(self.future.is_none());
    self.project().stream.poll_progress(cx)
}
```

`Then::poll_progress` doesn't need to worry about what to do with
[`self.future`] or its output, because it can only be called when `self.future`
is `None`. Similarly, it never needs to call `stream.poll_next`, because it
will never be called after `poll_next` returns `Pending`. Simple adapters like
`Map` and `Then` are more common than concurrent ones like `Merge` and
`Buffer1`, both in terms of how many implementations we need to write and also
how many instances appear in chains of iterators, and we want to keep them
simple.

[`self.future`]: https://docs.rs/futures-util/0.3.33/src/futures_util/stream/stream/then.rs.html#14-20

Another upside of the `PollNext::Pending` rule compared to the alternative is
that it's clear when it's been violated, and we can write asserts like the one
above. An `async gen fn` should panic if we call `poll_progress` while it's
suspended at an `.await`, just like an `async fn` panics today if we poll it
again after it's returned.

### Implementing [`Map`] with and without the "`PollNext::Pending` rule"

The previous section claimed that adapters like `Map` get more complicated if
callers can switch from `poll_next` to `poll_progress` at any time. This
section illustrates that in detail. (If the previous section already makes
perfect sense, then this section will be a bit repetitive.) Consider the
following example, which doesn't use `Map` at first:

```rs
async gen fn slow_numbers() -> u32 {
    for i in 0..10 {
        sleep(Duration::from_millis(1)).await;
        yield i;
    }
}

async gen fn print_numbers() -> u32 {
    for await i in slow_numbers() {
        println!("NUMBER {i}");
        yield i;
    }
}

// prints "NUMBER 0" once, does *not* print "NUMBER 1"
for await _ in print_numbers() {
    sleep(Duration::from_millis(10)).await;
    break;
}

// prints "NUMBER 0" *twice*
for await _ in print_numbers().merge(print_numbers()) {
    sleep(Duration::from_millis(10)).await;
    break;
}
```

This program does one iteration in each of a couple of `for await` loops. The
first loop should print "NUMBER 0" once. (That's "obvious", but we'll come back
to it below.) The second loop should print "NUMBER 0" twice, initially when it
receives an item, and then again immediately when it starts its
sleep.[^print_1] We should expect this much _regardless of the details of the
`AsyncIterator` contract,_ because we're not allowed to pause an async function
at an await point (or in this case, pause an `async gen fn` at a `for await`
point). The sleep in `slow_numbers` is there to make sure that `Merge` calls
`poll_next` on both sides; we don't need to ask subtle questions about what
`Merge` does when one side is immediately ready.

[^print_1]: Could `Merge` in the second loop continue calling `poll_next` on
    its _left_ child during the sleep, so that it prints "NUMBER 1" also?
    That's an implementation choice, and either `poll_next` or `poll_progress`
    is valid after yielding an item, depending on the behavior we want. The
    `poll_progress` version, which does _not_ print "NUMBER 1" here, is
    arguably more "natural" for a few reasons: \[1\] Merging an async iterator
    with [`empty()`] should have no effect on its behavior, and merging with
    [`pending()`] should also have no effect until the first iterator is
    exhausted. \[2\] The `poll_progress` version has cleaner buffering behavior
    if we chain several `merge` calls together, buffering at most one item from
    each iterator no matter where we put the parentheses. \[3\] We can
    replicate the `poll_next` version by composing the `poll_progress` version
    with the `Buffer1` adapter from the "Implementing `AsyncIterator`" section.

[`empty()`]: https://docs.rs/futures/latest/futures/stream/fn.empty.html
[`pending()`]: https://docs.rs/futures/latest/futures/stream/fn.pending.html

With all that in mind, let's think about what would happen if we instead
implemented `print_numbers` like this, using `Map`:

```rs
fn print_numbers() -> impl AsyncIterator<Item = u32> {
    slow_numbers().map(|i| {
        println!("NUMBER {i}");
        i
    })
}
```

This is debatable, but let's take it for granted that we want the observable
behavior of that function to be *identical* to the `async gen fn` above.
Consider the following implementation of `Map` ([playground
link][playground_map]):

[playground_map]: https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&gist=2a8abf235ae90b4661872bdd0d1c47f4

```rs
struct Map<Iter, F> {
    #[pin] // TODO: a standard way to do structural pinning in examples?
    iter: Iter,
    f: F,
}

impl<Iter, F, T> AsyncIterator for Map<Iter, F>
where
    Iter: AsyncIterator,
    F: FnMut(Iter::Item) -> T,
{
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> PollNext<T> {
        let this = self.project();
        this.iter.poll_next(cx).map(this.f) // `PollNext::map` is analogous to `Poll::map`.
    }

    fn poll_progress(self: Pin<&mut Self>, cx: &mut Context) -> Poll<()> {
        self.project().iter.poll_progress(cx)
    }
}
```

That's nice and simple, but is it correct? Namely, does it print "NUMBER 0"
twice in our `Merge` loop? If we can assume the "`PollNext::Pending` rule",
then it is, and it does. Once `Merge::poll_next` has called `Map::poll_next`,
`Merge::poll_progress` must keep calling it until it gets an item. That means
this version of `Map` will print as we expect, without any other assumptions
about how `Merge` is implemented.

On the other hand, what if `Merge::poll_progress` only called `poll_progress`
on both its children? (I.e. the "lazy" version in the previous section.) Now
our simple `Map` implementation isn't equivalent to the `async gen fn` anymore.
We won't see the print where we expect it, because `Map::poll_progress` never
calls `f`. If we care about equivalent behavior here, then we'll need to fix
that, and we'll also need a buffer slot for the resulting item. However,
there's a footgun: `Map::poll_progress` shouldn't *always* try to fill that
buffer slot, or else the *first loop* above will start printing "NUMBER 1"
while its body is sleeping. If we want `Map` to behave exactly like the `async
gen fn`, but we have to tolerate a "lazy" `Merge`, then `Map` also needs a
`next_item_wanted` flag like this ([playground link][playground_map_lazy]):

[playground_map_lazy]: https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&gist=491a04c7812478322bf7d08f0036cb4c

```rs
struct Map<Iter, F, T> {
    #[pin]
    iter: Fuse<Iter>,
    next_item_wanted: bool,
    f: F,
    item: Option<T>,
}

impl<Iter, F, T> AsyncIterator for Map<Iter, F, T>
where
    Iter: AsyncIterator,
    F: FnMut(Iter::Item) -> T,
{
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> PollNext<T> {
        let this = self.project();
        if let Some(item) = this.item.take() {
            return PollNext::Item(item);
        }
        match this.iter.poll_next(cx) {
            PollNext::Item(item) => {
                *this.next_item_wanted = false;
                PollNext::Item((this.f)(item))
            }
            PollNext::Pending => {
                *this.next_item_wanted = true;
                PollNext::Pending
            }
            PollNext::Done => PollNext::Done, // fused
        }
    }

    fn poll_progress(self: Pin<&mut Self>, cx: &mut Context) -> Poll<()> {
        let mut this = self.project();
        if this.item.is_none() && *this.next_item_wanted {
            match this.iter.as_mut().poll_next(cx) {
                PollNext::Item(item) => {
                    *this.item = Some((this.f)(item));
                    *this.next_item_wanted = false;
                    this.iter.poll_progress(cx)
                }
                PollNext::Pending => Poll::Pending,
                PollNext::Done => Poll::Ready(()), // fused
            }
        } else {
            this.iter.poll_progress(cx)
        }
    }
}
```

This is complicated. And any other `AsyncIterator` that runs caller code that
might have observable side effects -- so maybe not e.g. `Take` or `Skip`, but
including e.g. `Then`, `Filter`, `TakeWhile`, and `SkipWhile` -- would also be
_at least_ this complicated. It's not clear whether we'd actually want the
`AsyncIterator` contract to require this sort of `async gen fn`-equivalent
behavior, because those requirements would be both hard to explain and also
hard to `assert!`. Realistically, we'd probably give up on the idea, and we'd
accept that control flow through chains of async iterators is inconsistent and
unpredictable, at least when callers care about exactly when side effects
happen. At the same time, async iterators containing futures would still need
to be careful to avoid violating the `Future` contract.

The "`PollNext::Pending` rule" fixes this whole mess. It gives us a consistent,
high-level picture of how control flow in any `AsyncIterator` works: Control
"enters" the iterator when `poll_next` is initially called, and it "leaves" the
iterator when `poll_next` returns `Item` or `Done`. Once an iterator starts
"running", it continues uninterrupted (unless the whole thing is cancelled) to
its next yield point (if any). This works the same way whether it's an `async
gen fn` or a chain of combinators, and we can document it and teach it. For
non-concurrent async iterators, all `poll_progress` needs to do is forward
itself to any children. The cost of enforcing this rule falls on concurrent
combinators like `Merge` and `StreamMap`, which must track which of their
children are "running" and keep running them until they give up control.

### Is it worth having a whole new `PollNext` enum?

Today's `Poll<Option<_>>` return type captures the three possible return states
(item, pending, and done), but it doesn't represent anything about the
`poll_next`/`poll_progress` contract. Compare that to the `Future::poll`
method. Rust could've defined `poll` to return `Option<Output>`, but the
`Future` contract is important and subtle enough that it was worth adding a
separate type to represent it.

The same is true of `poll_next` in the new contract. The "`PollNext::Item`
rule" and the "`PollNext::Pending` rule" are important and subtle enough that
it's worth defining a distinct return type to represent them.

### Could we weaken "`poll_progress` is not allowed" to "`poll_progress` is not sufficient"?

By forbidding `poll_progress` after `poll_next` returns `Pending`, we give
implementations permission to panic in that case. But if not for explicit
asserts to enforce the rule, the `poll_progress` call itself would probably be
harmless in practice. Some child futures might get polled before asking for a
wakeup, but that's fine. Why not allow it?

The main reason not to allow it is that (like the `Future` contract) the
`AsyncIterator` contract is subtle and not enforced by the compiler. Folks
writing low-level async iterators for the first time are unlikely to read all
the docs, and a panic that says "there's a rule you didn't know about" is
better than confusing control flow bugs. We can't do this for the
`PollNext::Item` rule, and callers who overlook that one will miss wakeups, but
for the `PollNext::Pending` rule we can do it.

### Internal iteration

There's been [some discussion over the years][discussion] about whether
[`Iterator::try_fold`] would've been a better basic building block for
iteration than [`Iterator::next`]. It tends to make things easier on the
optimizer. As Scott McMurray once said, "The purpose of an `Iterator` is to
iterate, not to stop." Could the same be true of `AsyncIterator`?

[discussion]: https://internals.rust-lang.org/t/brainstorming-allowing-implementing-just-try-fold-and-getting-next-free/9417
[`Iterator::try_fold`]: https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.try_fold
[`Iterator::next`]: https://doc.rust-lang.org/std/iter/trait.Iterator.html#tymethod.next

An important detail here is that `Iterator::try_fold` takes `&mut self`. In
fact even if it didn't, there's a [blanket `Iterator` impl for `&mut _`
references][iterator_blanket], which lets us call `Iterator`'s `self` methods
from a mutable reference. But for all the reasons discussed above,
`AsyncIterator` won't support iteration by reference. It won't be possible for
`AsyncIterator` to implement `next` or `poll_next` in terms of `try_fold`.

[iterator_blanket]: https://doc.rust-lang.org/std/iter/trait.Iterator.html#impl-Iterator-for-%26mut+I

Perhaps `AsyncIterator` could define some sort of hybrid `poll_try_fold`? That
could be an interesting optimization, but it's not clear whether anybody wants
that to be the fundamental building block of async iteration.

## Prior art
[prior-art]: #prior-art

Various approaches to this feature, and the hangs and deadlocks that motivate
it, have been discussed for many years. Some points of reference:

- ["Footgun lurking in `FuturesUnordered` and other concurrency-enabling streams"](https://github.com/rust-lang/futures-rs/issues/2387)
- ["Barbara battles buffered streams"][barbara]
- ["`for await` and the battle of buffered streams"](https://tmandry.gitlab.io/blog/posts/for-await-buffered-streams/)
- https://without.boats/blog/poll-progress
- ["A discussion prototype for `AsyncIterator`"](https://hackmd.io/EplPcmBCTCSQ9LPOU6IZDw) and related [meeting notes](https://hackmd.io/bYaPiCdqR3WIyjKhFWzdtQ)
- [`futures_concurrency::concurrent_stream::Consumer::progress`](https://docs.rs/futures-concurrency/latest/futures_concurrency/concurrent_stream/trait.Consumer.html#tymethod.progress)
- ["Future's liveness problem"](https://skepfyr.me/blog/futures-liveness-problem/)
- ["Futurelock"][futurelock]
- ["Never snooze a future"][snooze]

## Unresolved questions
[unresolved-questions]: #unresolved-questions

### What other names should we consider for `poll_progress`?

`poll_progress` is the most common name folks use to refer to this feature, but
there have been other suggestions, including [`poll_bg`] and [`poll_pending`].
We might want to bikeshed this a bit.

[`poll_bg`]: https://www.reddit.com/r/rust/comments/18czxf1/comment/kchm9c9/
[`poll_pending`]: https://hackmd.io/wy0xQU8eRyuwhki5zkpoCg

### How should the `Stream` ecosystem migrate?

It would be nice if the `futures-core` crate could add a blanket `impl<Iter:
AsyncIterator> Stream for Iter`. Unfortunately, that would overlap with
`Stream`'s existing blanket impls.

We might need to ask `Stream` implementations to also, separately, implement
`AsyncIterator`. That's a lot of boilerplate, but it should be straightforward
for non-concurrent streams, which only need to forward `poll_progress` to their
children. Implementing `AsyncIterator` will be necessary for compatibility with
`for await`. Consumers with `Stream` bounds will be in a tricky position,
because they can't migrate to `AsyncIterator` until all of their dependencies
have added impls, but also `async gen fn` iterators will not implement
`Stream`.

It's an open question whether `Stream` should add a `poll_progress` method with
a no-op default implementation. That could help some of the ecosystem get the
benefits of `poll_progress` before migrating. On the other hand, since `Stream`
callers frequently use the `next` method, the benefit might be small, and it
could be more confusing than helpful. Feedback needed from the `futures-rs`
maintainers.

### Should `for await` loops track some sort of "progress pending" flag?

If a `for await` loop calls `poll_progress` on its iterator and receives
`Ready`, then it isn't necessary to call `poll_progress` again until the next
loop iteration. Should the loop track a per-iteration flag for this and only
conditionally call `poll_progress`? Should adapters like `Then` include a
similar flag? On the one hand it's good to skip unnecessary work, but on the
other hand we'd rather not have every adapter in a long chain redundantly track
the same bit of state.

In any case, `AsyncIterator` implementations should be able to tolerate either
answer to this question, so we could also leave this implementation detail
unspecified.

### Should invalid calls to `poll_progress` always panic?

The `AsyncIterator` contract doesn't require any particular behavior for
invalid calls to `poll_progress`, but we do need to decide what the
compiler-generated implementations of `async gen` blocks and functions will do.
It might be difficult to change this after stabilization.

As described in the [reference-level explanation](#async-gen-fn) and in the
[rationales](#could-we-weaken-poll_progress-is-not-allowed-to-poll_progress-is-not-sufficient),
we'd like invalid calls to these `poll_progress` functions to panic. On the
other hand, for `async gen` functions that don't loop over any other async
iterators, `poll_progress` might not have any real work to do besides these
panics. In that (common?) case, it would be nice if `poll_progress` was a
no-op, so that the optimizer could elide it entirely. Maybe trivial
`poll_progress` implementations should omit these panics? Or maybe
`poll_progress` should only panic in debug mode, like integer overflows? One
factor here could be that a generic `AsyncIterator` impl (like the two-line
`Then::poll_progress` example [in the rationales
section](#why-not-allow-poll_progress-at-any-time)) can't know whether its
child iterator's `poll_progress` function is trivial, so a debug-mode-only
approach might be better for consistency.

### Should we allow an indefinite delay between creation and polling?

See the corresponding question in [RFC 4019][strict_future_rfc]. `Future` and
`AsyncIterator` should presumably answer this question the same way.

### Should we require callers to drop async iterators promptly after they're `Done`?

Again see the corresponding question in [RFC 4019][strict_future_rfc].

Note that the `Map` playground examples above ([\[1\]][playground_map],
[\[2\]][playground_map_lazy]) include implementations of `Take` that drop their
inner iterators internally to avoid relying on the caller to drop them promptly
after `Done`. This will be easy for implementations to get wrong in practice.

## Future possibilities
[future-possibilities]: #future-possibilities

### Concurrency syntax

[RFC 4019][strict_future_rfc] mentions dedicated concurrency syntax in its
"future possibilities" section. It's interesting to consider how yielding from
concurrent blocks might work in that syntax:

```rs
async gen fn my_concurrent_generator() -> u32 {
    await all {
        yield do_stuff().await;
    } and {
        yield do_other_stuff().await;
    }
}
```

After one block yields, `poll_progress` would need to keep driving the other
block(s) to a yield point (if any), and each block would need stack space to
buffer an item.

Could we implement `merge` like this?

```rs
async gen fn merge<Other>(self, other: Other) -> Self::Item
where
    Other: IntoAsyncIterator<Item = Self::Item>,
{
    await all {
        for await item in self {
            yield item;
        }
    } and {
        for await item in other {
            yield item;
        }
    }
}
```

Almost! One difference is that the `Merge` struct will (probably) always
forward `poll_progress` to both its children, even in the case where
`poll_next` has never been called (e.g. on the right side of a [`Chain`]). But
in this `async gen fn` version, if `poll_next` has never been called, then
control hasn't entered either of the `for await` loops. We haven't done the
`IntoAsyncIterator::into_async_iter` conversion yet, and we won't/can't forward
`poll_progress`. That might be an unfortunate footgun.

### Generalized coroutines

We could imagine a more general "coroutine" version of `Iterator` and
`AsyncIterator`, which (as in e.g. Python) takes inputs in addition to yielding
items, and which also has a final return value. There are a couple of unused
value spots in the `for` and `for await` syntax:

```rs
for await item in iter {
    do_stuff(item).await;
    // The value of the body is always `()`.
} // The value of the loop itself is always `()`.
```

Similarly, there are a couple of unused value spots in the `gen` and `async
gen` syntax:

```rs
async gen fn foo() -> u32 {
    yield 1; // The value of the `yield` expression is always `()`.
    // The return value of the function is always `()`.
}
```

These spots are a surprisingly good fit for coroutine inputs and return values.
In the `gen fn` / `async gen fn` syntax, an input item would become the value
of the currently suspended `yield` expression, and the return value of the body
would be the final value of the coroutine. In the `for` / `for await` syntax,
the final value could become the value of the loop itself, and the input items
could come from the value of the body. (This would suggest that `break` would
need a value of the same type as the final value, and `continue` would need a
value of the same type as the inputs.)

The async coroutine equivalent of `PollNext` might have a second type parameter
for the final value:

```rs
enum CoroutinePollNext<Item, Return> {
    Item(Item),
    Pending,
    Done(Return),
}
```

An async coroutine trait might look very similar to the version of
`AsyncIterator` proposed here. It would need three type parameters and a third
method called something like `send`. Perhaps the contract would be that you
must call `send` once at some point after each `Item`, before calling
`poll_next` again. We might also define a "`send` rule" similar to the
"`PollNext::Pending` rule": once you `send` an input, the iterator is
"running", and you must call `poll_next` promptly. It wouldn't be as easy for a
`for await coroutine` loop or whatever it might be called to enable concurrency
between the coroutine and the body -- a wrapper type like `Buffer1` above would
need a way to come up with input values, which might be possible in some cases
-- but macros might be able to do interesting things.

[`Future`]: https://doc.rust-lang.org/std/future/trait.Future.html
[barbara]: https://rust-lang.github.io/wg-async/vision/submitted_stories/status_quo/barbara_battles_buffered_streams.html
[futurelock]: https://rfd.shared.oxide.computer/rfd/0609
[`FuturesUnordered`]: https://docs.rs/futures/latest/futures/stream/struct.FuturesUnordered.html
[`Merge`]: https://docs.rs/tokio-stream/latest/tokio_stream/trait.StreamExt.html#method.merge
[`Chain`]: https://docs.rs/futures/latest/futures/stream/trait.StreamExt.html#method.chain
[`StreamExt::next`]: https://docs.rs/futures/latest/futures/stream/trait.StreamExt.html#method.next
[`StreamMap`]: https://docs.rs/tokio-stream/latest/tokio_stream/struct.StreamMap.html
[mini_redis]: https://smallcultfollowing.com/babysteps/blog/2022/06/13/async-cancellation-a-case-study-of-pub-sub-in-mini-redis/
[snooze]: https://jacko.io/snooze.html
[strict_future_rfc]: https://github.com/rust-lang/rfcs/pull/4019
