- Feature Name: (fill me in with a unique ident, my_awesome_feature)
- Start Date: 2018-11-09
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

This RFC proposes to stabilize the library component for the [first-class `async`/`await`
syntax][companion RFC]. In particular, it would stabilize:

- All APIs of the `std`-level task system, i.e. `std::task::*`.
- The core `Future` API, i.e. `core::future::Future` and `std::future::Future`.

It does *not* propose to stabilize any of the `async`/`await` syntax itself, which will be proposed in a separate step. It also does not cover stabilization of the `Pin` APIs, which has [already been proposed elsewhere](https://github.com/rust-lang/rust/issues/55766).

This is a revised and slimmed down version of the [earlier futures RFC](https://github.com/rust-lang/rfcs/pull/2418), which was postponed until more experience was gained on nightly.

[pin]: https://github.com/rust-lang/rfcs/pull/2349
[companion RFC]: https://github.com/rust-lang/rfcs/pull/2394

# Motivation
[motivation]: #motivation

## Why `Future`s in `std`?

The core motivation for this RFC is to stabilize the supporting mechanisms for
`async`/`await` syntax.  The syntax itself is motivated in the (already merged)
[companion RFC], and there is a [blog post](http://aturon.github.io/2018/04/24/async-borrowing/)
that goes through its importance in greater detail.

As with closures, `async` syntax involves producing an anonymous type that implements
a key trait: `Future`. Because `async`/`await` requires language-level support,
the underlying trait must also be part of the standard library. Thus, the goal
of this RFC is to stabilize this this `Future` trait and the types it depends on.
This is the last step needed before we are in a position to stabilize `async`/`await`
itself.

## How does this step fit into the bigger picture?

The `async`/`await` syntax is one of the most eagerly desired features in Rust, and
will have a major impact on the ecosystem. It, and the APIs described here, have been
available on nightly and put into major use since late May 2018.

Stabilizing the futures API portion of this design makes it easier for libraries to
both work on stable Rust *and* to seamlessly support use of `async`/`await` on nightly.
It also allows us to finalize design debate on the API portion, and focus on the few
remaining questions about `async` syntax before it, too, is stabilized.

# Historical context

The APIs proposed for stabilization have a lengthy history:

- The `Future` trait began with the futures crate; [0.1 was released](http://aturon.github.io/2016/08/11/futures/)
in August of 2016. That release established the core ideas of the task/polling model,
as well as many other aspects of the API that are retained here. The 0.1 series
continues to be heavily used throughout the Rust ecosystem and in production systems.

- In early 2018, as work began toward `async`/`await`, the futures team set up
an RFC process and wrote [several RFCs](https://github.com/rust-lang-nursery/futures-rfcs/pulls?q=is%3Apr+is%3Aclosed) to make revisions to the core APIs based
on longstanding community feedback. These RFCs ultimately resulted in a [0.2le release](http://aturon.github.io/2018/02/27/futures-0-2-RC/), which [shipped](http://aturon.github.io/2018/04/06/futures2/) in April.

- During the same period, @withoutboats's work on the pinning APIs supporting borrowing
within `async` blocks [came to completion](https://boats.gitlab.io/blog/post/2018-04-06-async-await-final/).
The [pinning APIs](https://github.com/rust-lang/rfcs/pull/2349) were a game-changer, making it possible to support borrowing-across-yield *without* making the core future APIs unsafe.

- In April 2018, a pair of RFCs formally proposed the `async`/`await` syntax as well as further revision of the futures API (to take advantage of the pinning APIs); the latter went through many revisions, including a [fresh RFC](https://github.com/rust-lang/rfcs/pull/2418). Ultimately, the [syntax RFC was merged](https://github.com/rust-lang/rfcs/pull/2394#issuecomment-387550523) in May, while the API RFC was closed, with [the understanding](https://github.com/rust-lang/rfcs/pull/2418#issuecomment-415841459) that further design iteration would occur on nightly, to be followed up by a stabilization RFC: this one!

- The APIs [landed in `std`](https://github.com/rust-lang/rust/pull/51263) at the end of May.

- Since then, the syntax, the `std` APIs, and the futures 0.3 crate have all evolved in tandem as we've gained experience with the APIs. A major driver in this experience has been Google's Fuchsia project, which is using *all* of these features at large scale in an operating system setting.

- The most recent revisions were in August, and involved [some insights](https://boats.gitlab.io/blog/post/rethinking-pin/) into how to make the `Pin` APIs even cleaner. These APIs have been [proposed for stabilization](https://github.com/rust-lang/rust/issues/55766), as has [their use as `self` types](https://github.com/rust-lang/rust/issues/55786).

- There are multiple compatibility layers available for using futures 0.1 and 0.3 simultaneously. That's important, because it allows for *incremental* migration of existing production code.

Since the initial futures 0.3 release, relatively little has changed about the core `Future` trait and task system, other than the refinements mentioned above. The actual `Future` trait has stayed essentially as it was back in April.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The `Future` trait represents an *asynchronous* and lazy computation that may
eventually produce a final value, but doesn't have to block the current thread
to do so.

Futures can be constructed through `async` blocks or `async` functions, e.g.,

```rust
async fn read_frame(socket: &TcpStream) -> Result<Frame, io::Error> { ... }
```

This `async` function, when invoked, produces a future that represents the
completion of reading a frame from the given socket. The function signature
is equivalent to:

```rust
fn read_frame<'sock>(socket: &'sock TcpStream)
    -> impl Future<Output = Result<Frame, io::Error>> + 'sock;
```

Other async functions can *await* this future; see the [companion
RFC] for full details.

In addition to `async fn` definitions, futures can be built using adapters, much
like with `Iterator`s. Initially these adapters will be provided entirely "out
of tree", but eventually they will make their way into the standard library.

Ultimately asynchronous computations are executed by *tasks*, which are
lightweight threads. In particular, an *executor* is able to "spawn" a
`()`-producing `Future` as an independent task; these tasks are then
cooperatively scheduled onto one or more operating system threads. The
`Executor` trait defines this interface, and the `task` module provides a host
of related definitions needed when manually implementing `Future`s or
executors.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## `core::task` module

The fundamental mechanism for asynchronous computation in Rust is *tasks*, which
are lightweight threads of execution; many tasks can be cooperatively scheduled
onto a single operating system thread.

To perform this cooperative scheduling we use a technique sometimes referred to
as a "trampoline". When a task would otherwise need to block waiting for some
event, instead it schedules itself for later wakeup and *returns* to the
executor running it, which can then run another task. Subsequent wakeups place
the task back on the executors queue of ready tasks, much like a thread
scheduler in an operating system.

Attempting to complete a task (or async value within it) is called *polling*,
and always yields a `Poll` value back:

```rust
/// Indicates whether a value is available, or if the current task has been
/// scheduled for later wake-up instead.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Poll<T> {
    /// Represents that a value is immediately ready.
    Ready(T),

    /// Represents that a value is not ready yet.
    ///
    /// When a function returns `Pending`, the function *must* also
    /// ensure that the current task is scheduled to be awoken when
    /// progress can be made.
    Pending,
}
```

When a task returns `Poll::Ready`, the executor knows the task has completed and
can be dropped.

### Waking up

Each task executor provides its own scheduling facilities, and hence needs to
customize the way task wakeups are handled. Most of the time, you should use the
`std::task::Wake` trait defining wakeup behavior:

```rust
/// A way of waking up a specific task.
///
/// Any task executor must provide a way of signaling that a task it owns
/// is ready to be `poll`ed again. Executors do so by providing a wakeup handle
/// type that implements this trait.
pub trait Wake: Send + Sync {
    /// Indicates that the associated task is ready to make progress and should
    /// be `poll`ed.
    ///
    /// Executors generally maintain a queue of "ready" tasks; `wake` should place
    /// the associated task onto this queue.
    fn wake(self: &Arc<Self>);

    /// Indicates that the associated task is ready to make progress and should be polled.
    /// This function is like wake, but can only be called from the thread on which this
    /// `Wake` was created.
    ///
    /// Executors generally maintain a queue of "ready" tasks; `wake_local` should place
    /// the associated task onto this queue.
    unsafe fn wake_local(self: &Arc<Self>)
}
```

To see how this might be used in practice, here's a simple example sketch:

```rust
struct ExecutorInner {
    sync_ready_queue: SynchronousQueue,
    optimized_queue: UnsafeCell<Vec<Task>>,
}

struct Task {
    future: ...,
    executor: Arc<ExecutorInner>,
}

impl Wake for Task {
    fn wake(self: &Arc<Self>) {
        self.executor.sync_ready_queue.push(self.clone());
    }
    unsafe fn wake_local(self: &Arc<Self>) {
        (&mut *self.executor.optimized_queue.get()).push(self.clone())
    }
}
```

The use of `&Arc<Self>` rather than just `&self` makes it possible to work directly with
the trait object for `Wake`, including cloning it. With `UnsafeWake` below, we'll see
an API with greater flexibility for the cases where `Arc` is problematic.

In general async values are not coupled to any particular executor, so we use trait
objects to handle waking. These come in two forms: `Waker` for the general case, and
`LocalWaker` to provide more effenciency when the wakeup is guaranteed to occur on the
executor thread:

```rust
/// A `Waker` is a handle for waking up a task by notifying its executor that it
/// is ready to be run.
///
/// This handle contains a trait object pointing to an instance of the `UnsafeWake`
/// trait, allowing notifications to get routed through it.
///
/// Implements `Clone`, `Send`, and `Sync`.
pub struct Waker { ... }

impl Waker {
    /// Wake up the task associated with this `Waker`.
    pub fn wake(&self);
}

/// A `LocalWaker` is a handle for waking up a task by notifying its executor that it is ready to be run.
///
/// This is similar to the `Waker` type, but cannot be sent across threads. Task executors can use this type to implement more optimized singlethreaded wakeup behavior.
impl LocalWaker {
    /// Wake up the task associated with this `LocalWaker`.
    pub fn wake(&self);
}

/// You can upgrade to a sendable `Waker` at zero cost, but waking through a `Waker` is more expensive
/// due to synchronization.
impl From<LocalWaker> for Waker  { .. }
```

Task execution always happens in the context of a `LocalWaker` that can be used to
wake the task up locally, or converted into a `Waker` that can be sent to other threads.

It's possible to construct a `Waker` using `From<Arc<dyn Wake>>`.

### `UnsafeWake` and `no_std` compatibility

The [`UnsafeWake` trait](https://doc.rust-lang.org/nightly/std/task/trait.UnsafeWake.html)
in `core::task` is designed to support task wakeup in  `no_std` environments, where
we cannot use `Arc`. It is *not* proposed for stabilization at this time, because
its APIs are awaiting revision based on object safety for `*mut self` methods.

## `core::future` module

With all of the above task infrastructure in place, defining `Future` is
straightforward:

```rust
pub trait Future {
    /// The type of value produced on completion.
    type Output;

    /// Attempt to resolve the future to a final value, registering
    /// the current task for wakeup if the value is not yet available.
    ///
    /// # Return value
    ///
    /// This function returns:
    ///
    /// - [`Poll::Pending`] if the future is not ready yet
    /// - [`Poll::Ready(val)`] with the result `val` of this future if it
    ///   finished successfully.
    ///
    /// Once a future has finished, clients should not `poll` it again.
    ///
    /// When a future is not ready yet, `poll` returns `Poll::Pending` and
    /// stores a clone of the [`LocalWaker`] to be woken once the future can
    /// make progress. For example, a future waiting for a socket to become
    /// readable would call `.clone()` on the [`LocalWaker`] and store it.
    /// When a signal arrives elsewhere indicating that the socket is readable,
    /// `[LocalWaker::wake]` is called and the socket future's task is awoken.
    /// Once a task has been woken up, it should attempt to `poll` the future
    /// again, which may or may not produce a final value.
    ///
    /// Note that on multiple calls to `poll`, only the most recent
    /// [`LocalWaker`] passed to `poll` should be scheduled to receive a
    /// wakeup.
    ///
    /// # Runtime characteristics
    ///
    /// Futures alone are *inert*; they must be *actively* `poll`ed to make
    /// progress, meaning that each time the current task is woken up, it should
    /// actively re-`poll` pending futures that it still has an interest in.
    ///
    /// The `poll` function is not called repeatedly in a tight loop-- instead,
    /// it should only be called when the future indicates that it is ready to
    /// make progress (by calling `wake()`). If you're familiar with the
    /// `poll(2)` or `select(2)` syscalls on Unix it's worth noting that futures
    /// typically do *not* suffer the same problems of "all wakeups must poll
    /// all events"; they are more like `epoll(4)`.
    ///
    /// An implementation of `poll` should strive to return quickly, and must
    /// *never* block. Returning quickly prevents unnecessarily clogging up
    /// threads or event loops. If it is known ahead of time that a call to
    /// `poll` may end up taking awhile, the work should be offloaded to a
    /// thread pool (or something similar) to ensure that `poll` can return
    /// quickly.
    ///
    /// # [`LocalWaker`], [`Waker`] and thread-safety
    ///
    /// The `poll` function takes a [`LocalWaker`], an object which knows how to
    /// awaken the current task. [`LocalWaker`] is not `Send` nor `Sync`, so in
    /// order to make thread-safe futures the [`LocalWaker::into_waker`] method
    /// should be used to convert the [`LocalWaker`] into a thread-safe version.
    /// [`LocalWaker::wake`] implementations have the ability to be more
    /// efficient, however, so when thread safety is not necessary,
    /// [`LocalWaker`] should be preferred.
    ///
    /// # Panics
    ///
    /// Once a future has completed (returned `Ready` from `poll`),
    /// then any future calls to `poll` may panic, block forever, or otherwise
    /// cause bad behavior. The `Future` trait itself provides no guarantees
    /// about the behavior of `poll` after a future has completed.
    ///
    /// [`Poll::Pending`]: ../task/enum.Poll.html#variant.Pending
    /// [`Poll::Ready(val)`]: ../task/enum.Poll.html#variant.Ready
    /// [`LocalWaker`]: ../task/struct.LocalWaker.html
    /// [`LocalWaker::into_waker`]: ../task/struct.LocalWaker.html#method.into_waker
    /// [`LocalWaker::wake`]: ../task/struct.LocalWaker.html#method.wake
    /// [`Waker`]: ../task/struct.Waker.html
    fn poll(self: Pin<&mut Self>, lw: &LocalWaker) -> Poll<Self::Output>;
}
```

Most of the explanation here follows what we've already said about the task
system. The one twist is the use of `Pin`, which makes it possible to keep data
borrowed across separate calls to `poll` (i.e., "borrowing over yield
points"). The mechanics of pinning are explained
in [the RFC that introduced it](https://github.com/rust-lang/rfcs/pull/2349)
and the [blog post about t he latest revisions](https://boats.gitlab.io/blog/post/rethinking-pin/).

## Relation to futures 0.1

The various discussions outlined in the historical context section above cover the
path to these APIs from futures 0.1. But, in a nutshell, there are three major shifts:

- The use of `Pin<&mut self>` rather than just `&mut self`, which is necessary
to support borrowing withing `async` blocks. The `Unpin` marker trait can be used
to restore ergonomics and safety similar to futures 0.1 when writing futures by hand.

- Dropping *built in* errors from `Future`, in favor of futures returning a `Result`
when they can fail. The futures 0.3 crate provides a `TryFuture` trait that bakes
in the `Result` to provide better ergonomics when working with `Result`-producing futures.
Dropping the error type has been discussed in previous threads, but the most
important rationale is to provide an orthogonal, compositional semantics for `async fn`
that mirrors normal `fn`, rather than *also* baking in a particular style of
error handling.

- Passing a `LocalWaker` explicitly, rather than stashing it in thread-local storage.
This has been a hotly debated issue since futures 0.1 was released, and this
RFC does not seek to relitigate it, but to summarize, the major advantages are (1)
when working with manual futures (as opposed to `async` blocks) it's much easier to
tell where an ambient task is required, and (2) `no_std` compatibility is
significantly smoother.

To bridge the gap between futures 0.1 and 0.3, there are several compatibility shims,
including one built into the futures crate itself, where you can shift between the two
simply by using a `.compat()` combinator. These compatibility layers make it possible
to use the existing ecosystem smoothly with the new futures APIs, and make it possible
to transition large code bases incrementally.

# Rationale, drawbacks, and alternatives

This RFC is one of the most substantial additions to `std` proposed since
1.0. It commits us to including a particular task and polling model in the
standard library, and ties us to `Pin`.

So far we've been able to push the task/polling model into virtually every niche
Rust wishes to occupy, and the main downside has been, in essence, the lack of
async/await syntax (and
the
[borrowing it supports](http://aturon.github.io/2018/04/24/async-borrowing/)).

This RFC does not attempt to provide a complete introduction to the task model
that originated with the futures crate. A fuller account of the design rationale
and alternatives can be found in the following two blog posts:

- [Zero-cost futures in Rust](http://aturon.github.io/2016/08/11/futures/)
- [Designing futures for Rust](http://aturon.github.io/2016/09/07/futures-design/)

To summarize, the main alternative model for futures is a callback-based approach,
which was attempted for several months before the current approach was discovered.
In our experience, the callback approach suffered from several drawbacks in Rust:

- It forced allocation almost everywhere, and hence was not compatible with no_std.
- It made cancellation *extremely* difficult to get right, whereas with the
  proposed model it's just "drop".
- Subjectively, the combinator code was quite hairy, while with the task-based model
  things fell into place quickly and easily.

Some additional context and rationale for the overall async/await project is
available in the [companion RFC].

For the remainder of this section, we'll dive into specific API design questions
where this RFC differs from futures 0.2.

## Rationale, drawbacks and alternatives for removing built-in errors

There are an assortment of reasons to drop the built-in error type in the main
trait:

- **Improved type checking and inference**. The error type is one of the biggest
  pain points when working with futures combinators today, both in trying to get
  different types to match up, and in inference failures that result when a
  piece of code cannot produce an error. To be clear, many of these problems
  will become less pronounced when `async` syntax is available.

- **Async functions**. If we retain a built-in error type, it's much less clear
  how `async fn` should work: should it always require the return type to be a
  `Result`? If not, what happens when a non-`Result` type is returned?

- **Combinator clarity**. Splitting up the combinators by whether they rely on
  errors or not clarifies the semantics. This is *especially* true for streams,
  where error handling is a common source of confusion.

- **Orthogonality**. In general, producing and handling errors is separable from
  the core polling mechanism, so all things being equal, it seems good to follow
  Rust's general design principles and treat errors by *composing* with `Result`.

All of that said, there are real downsides for error-heavy code, even with
`TryFuture`:

- An extra import is needed (obviated if code imports the futures prelude, which
  we could perhaps more vocally encourage).

- It can be confusing for code to *bound* by one trait but *implement* another.

The error handling piece of this RFC is separable from the other pieces, so the
main alternative would be to retain the built-in error type.

## Rationale, drawbacks and alternatives to the core trait design (wrt `Pi`)

Putting aside error handling, which is orthogonal and discussed above, the
primary other big item in this RFC is the move to `Pin` for the core polling
method, and how it relates to `Unpin`/manually-written futures. Over the course
of RFC discussions, we've identified essentially three main approaches to this
question:

- **One core trait**. That's the approach taken in the main RFC text: there's
  just a single core `Future` trait, which works on `Pin<&mut Self>`. Separately
  there's a `poll_unpin` helper for working with `Unpin` futures in manual
  implementations.

- **Two core traits**. We can provide two traits, for example `MoveFuture` and
  `Future`, where one operates on `&mut self` and the other on `Pin<&mut Self>`.
  This makes it possible to continue writing code in the futures 0.2 style,
  i.e. without importing `Pin`/`Unpin` or otherwise talking about pins. A
  critical requirement is the need for interoperation, so that a `MoveFuture`
  can be used anywhere a `Future` is required. There are at least two ways to
  achieve such interop:

  - Via a blanket impl of `Future` for `T: MoveFuture`. This approach currently
    blocks some *other* desired impls (around `Box` and `&mut` specifically),
    but the problem doesn't appear to be fundamental.

  - Via a subtrait relationship, so that `T: Future` is defined essentially as
    an alias for `for<'a> Pin<&mut 'a T>: MoveFuture`. Unfortunately, such
    "higher ranked" trait relationships don't currently work well in the trait
    system, and this approach also makes things more convoluted when
    implementing `Future` by hand, for relatively little gain.

The drawback of the "one core trait" approach taken by this RFC is its ergonomic
hit when writing moveable futures by hand: you now need to import `Pin` and
`Unpin`, invoke `poll_unpin`, and impl `Unpin` for your types. This is all
pretty mechanical, but it's a pain. It's possible that improvements in `Pin`
ergonomics will obviate some of these issues, but there are a lot of open
questions there still.

On the other hand, a two-trait approach has downsides as well. If we *also*
remove the error type, there's a combinatorial explosion, since we end up
needing `Try` variants of each trait (and this extends to related traits, like
`Stream`, as well). More broadly, with the one-trait approach, `Unpin` acts as a
kind of "independent knob" that can be applied orthogonally from other concerns;
with the two-trait approach, it's "mixed in". And both of the two-trait
approaches run up against compiler limitations at the moment, though of course
that shouldn't be taken as a deciding factor.

**The primary reason this RFC opts for the one-trait approach is that it's the
conservative, forward-compatible option, and has proven itself in practice**.
It's possible to add `MoveFuture`, together with a blanket impl, at any point in the future.
Thus, starting with just the single `Future` trait as proposed in this RFC keeps our options
maximally open while we gain experience.

# Prior art
[prior-art]: #prior-art

There is substantial prior art both with async/await notation and with futures
(aka promises) as a basis. The proposed futures API was influenced by Scala's
futures in particular, and is broadly similar to APIs in a variety of other
languages (in terms of the adapters provided).

What's more unique about the model in this RFC is the use of tasks, rather than
callbacks. The RFC author is not aware of other *futures* libraries using this
technique, but it is a fairly well-known technique more generally in functional
programming. For a recent example,
see
[this paper](https://www.microsoft.com/en-us/research/wp-content/uploads/2011/01/monad-par.pdf) on
parallelism in Haskell. What seems to be perhaps new with this RFC is the idea
of melding the "trampoline" technique with an explicit, open-ended task/wakeup
model.

# Unresolved questions
[unresolved]: #unresolved-questions

None at the moment.
