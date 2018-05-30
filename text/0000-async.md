- Feature Name: (fill me in with a unique ident, my_awesome_feature)
- Start Date: 2018-04-04
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

This RFC provides the library component for the first-class `async`/`await`
syntax proposed in a [companion RFC]. It is intentionally minimal, including the
smallest set of mechanisms needed to support async/await with borrowing and
interoperation with the futures crate. Those mechanisms are:

- The task system of the futures crate, which will be moved into `libcore`
- A new `Future` trait, which integrates the [`PinMut` APIs][pin] with the task system to
  provide async values that can interoperate with futures.

[pin]: https://github.com/rust-lang/rfcs/pull/2349
[companion RFC]: https://github.com/rust-lang/rfcs/pull/2394

The RFC also covers the intended ecosystem migration path.

# Motivation
[motivation]: #motivation

The basic motivation for this RFC is to provide a supporting mechanism for
`async`/`await` syntax:

```rust
async fn function(argument: &str) -> usize {
     // ...
}
```

The syntax itself is motivated in the [companion RFC], and there is
a [blog post](http://aturon.github.io/2018/04/24/async-borrowing/) that goes
through its importance in greater detail. As with closures, the syntax involves
producing an anonymous type, so that the above declaration is equivalent to:

```rust
fn function<'a>(argument: &'a str) -> _Anonymous<'a, usize> {
     // ...
}
```

Again like a closure the anonymous type is only usable through the trait it
implements: `Future`. The goal of this RFC is to provide a concrete proposal for
this `Future` trait, based on the work pioneered by the futures crate.

A secondary benefit of this RFC is that it enshrines the *task system* currently
defined by the futures crate into `libcore`, thereby standardizing and
ultimately stabilizing the async ecosystem around a single lightweight task
mechanism.

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
customize the way task wakeups are handled. As such, there is a
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
    fn wake(&self);
}
```

In general async values are not coupled to any particular executor, so we use a trait
object to handle waking:

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

// We will see how to handle the no_std case later in the RFC...
impl<T> From<T> for Waker where Arc<T>: Wake + 'static { ... }
```

Task execution always happens in the context of a `Waker` that can be used to
wake the task up; we'll see the full `core::task::Context` structure below.

### Executors

An executor is responsible for polling tasks to completion. We represent this
with the `core::task::Executor` trait:

```rust
/// A task executor.
///
/// A *task* is a `()`-producing async value that runs at the top level, and will
/// be `poll`ed until completion. It's also the unit at which wake-up
/// notifications occur. Executors, such as thread pools, allow tasks to be
/// spawned and are responsible for putting tasks onto ready queues when
/// they are woken up, and polling them when they are ready.
pub trait Executor {
    /// Spawn the given task, polling it until completion.
    ///
    /// # Errors
    ///
    /// The executor may be unable to spawn tasks, either because it has
    /// been shut down or is resource-constrained.
    fn spawn_obj(&mut self, task: TaskObj) -> Result<(), SpawnObjError>;

    /// Determine whether the executor is able to spawn new tasks.
    ///
    /// # Returns
    ///
    /// An `Ok` return means the executor is *likely* (but not guaranteed)
    /// to accept a subsequent spawn attempt. Likewise, an `Err` return
    /// means that `spawn` is likely, but not guaranteed, to yield an error.
    fn status(&self) -> Result<(), SpawnErrorKind> {
        Ok(())
    }
}

pub struct TaskObj { .. }

impl TaskObj {
    /// Create a new `TaskObj` by boxing the given future.
    pub fn new<A: Future<Output = ()> + Send + 'static>(f: A) -> TaskObj;
}

/// Provides the reason that an executor was unable to spawn.
pub struct SpawnErrorKind { .. }

impl SpawnErrorKind {
    /// Spawning is failing because the executor has been shut down.
    pub fn shutdown() -> SpawnErrorKind;

    /// Check whether this error is the `shutdown` error.
    pub fn is_shutdown(&self) -> bool;

    // additional error variants added over time...
}

/// The result of a failed spawn
pub struct SpawnObjError {
    /// The kind of error
    pub kind: SpawnErrorKind,

    /// The task for which spawning was attempted
    pub task: TaskObj,
}
```

We need the executor trait to be usable as a trait object, which is why `TaskObj`
is constructed here from a boxed future. (In the no_std section, we'll see
another constructor). In the long run, though, once we can take `dyn` by value,
we would deprecate `spawn_obj` and add a default `spawn` method:

```rust
trait Executor {
    fn spawn(&mut self, task: Future<Output = ()> + Send) -> Result<(), SpawnErrorKind> {
        self.spawn_obj(TaskObj::new(task))
    }
    // ...
}
```

At that point we would also deprecate `TaskObj`, which is the reason for using
the `Obj` suffix -- we want to keep the name `Task` available for potential
usage down the line.

In addition to the above, the `core::task` module will include the following API
for helping detect bugs:

```rust
/// Marks the current thread as being within the dynamic extent of an
/// executor.
///
/// Executor implementations should call this function before beginning to
/// execute a tasks, and drop the returned `Enter` value after completing
/// task execution:
///
/// ```rust
/// let enter = enter().expect("...");
/// /* run task */
/// drop(enter);
/// ```
///
/// Doing so ensures that executors aren't accidentally invoked in a nested fashion.
/// When that happens, the inner executor can block waiting for an event that can
/// only be triggered by the outer executor, leading to a deadlock.
///
/// # Error
///
/// Returns an error if the current thread is already marked, in which case the
/// caller should panic with a tailored error message.
pub fn enter() -> Result<Enter, EnterError>
```

As stated in the doc comment, the expectation is that all executors will wrap
their task execution within an `enter` to detect inadvertent nesting.

### Task contexts

All tasks are executed with two pieces of contextual information:

- A `Waker` for waking the task up later on.
- An executor, which is the "default" place to spawn further tasks.

Notably, this list does *not* include task-local data; that can be addressed
externally, as we'll see in a later section.

The `core::task::Context` type gathers the (stack-rooted) contextual information
together, and is passed by mutable reference to all polling functions:

```rust
/// Information about the currently-running task.
///
/// Contexts are always tied to the stack, since they are set up specifically
/// when performing a single `poll` step on a task.
pub struct Context<'a> { .. }

impl<'a> Context<'a> {
    pub fn new(waker: &'a Waker, executor: &'a mut Executor) -> Context<'a>

    /// Get the `Waker` associated with the current task.
    pub fn waker(&self) -> &Waker;

    /// Run an asynchronous computation to completion on the default executor.
    ///
    /// # Panics
    ///
    /// This method will panic if the default executor is unable to spawn.
    /// To handle executor errors, use the `executor` method instead.
    pub fn spawn(&mut self, f: impl Future<Output = ()> + 'static + Send);

    /// Get the default executor associated with this task.
    ///
    /// This method is useful primarily if you want to explicitly handle
    /// spawn failures.
    ///
    /// NB: this will remain unstable until the final `Executor` trait is ready.
    pub fn executor(&mut self) -> &mut BoxExecutor;
}
```

Note that the `spawn` method here will box until `Executor` is added.

## `core::future` module

With all of the above task infrastructure in place, defining `Future` is
straightforward:

```rust
pub trait Future {
    /// The type of value produced on completion.
    type Output;

    /// Attempt to resolve the computation to a final value, registering
    /// the current task for wakeup if the value is not yet available.
    ///
    /// # Return value
    ///
    /// This function returns:
    ///
    /// - `Poll::Pending` if the value is not ready yet.
    /// - `Poll::Ready(val)` with the result `val` upon completion.
    ///
    /// Once a future has completed, clients should not `poll` it again.
    ///
    /// When a future is not ready yet, `poll` returns `Poll::Pending`.
    /// The future will *also* register the interest of the current task in the
    /// value being produced. For example, if the future represents the availability
    /// of data on a socket, then the task is recorded so that when data arrives,
    /// it is woken up (via `cx.waker()`). Once a task has been woken up,
    /// it should attempt to `poll` the future again, which may or may not
    /// produce a final value at that time.
    ///
    /// Note that if `Pending` is returned it only means that the *current* task
    /// (represented by the argument `cx`) will receive a notification. Tasks
    /// from previous calls to `poll` will *not* receive notifications.
    ///
    /// # Runtime characteristics
    ///
    /// `Future` values are *inert*; they must be *actively* `poll`ed to make
    /// progress, meaning that each time the current task is woken up, it should
    /// actively re-`poll` pending computations that it still has an interest in.
    /// Usually this is handled automatically by `async`/`await` notation or
    /// via adapter methods. Executors ensure that each task is `poll`ed every
    /// time a future internal to that task is ready to make progress.
    ///
    /// The `poll` function is not called repeatedly in a tight loop, but only
    /// whenever the computation itself is ready to make progress, as signaled via
    /// `cx.waker()`. If you're familiar with the `poll(2)` or `select(2)`
    /// syscalls on Unix it's worth noting that async values typically do *not*
    /// suffer the same problems of "all wakeups must poll all events"; they
    /// are more like `epoll(4)`.
    ///
    /// An implementation of `poll` should strive to return quickly, and should
    /// *never* block. Returning quickly prevents unnecessarily clogging up
    /// threads or event loops. If it is known ahead of time that a call to
    /// `poll` may end up taking awhile, the work should be offloaded to a
    /// thread pool (or something similar) to ensure that `poll` can return
    /// quickly.
    ///
    /// # Panics
    ///
    /// Once a future has completed (returned `Ready` from `poll`), subsequent
    /// calls to `poll` may panic, block forever, or otherwise cause bad behavior.
    /// The `Future` trait itself provides no guarantees about the behavior of
    /// `poll` after a future has completed.
    fn poll(self: PinMut<Self>, cx: &mut task::Context) -> Poll<Self::Output>;
}
```

Most of the explanation here follows what we've already said about the task
system. The one twist is the use of `PinMut`, which makes it possible to keep data
borrowed across separate calls to `poll` (i.e., "borrowing over yield
points"). The mechanics of pinning are explained
in [the RFC that introduced it](https://github.com/rust-lang/rfcs/pull/2349),
and the interoperation with traditional futures is described next.

## Relation to the futures crate

In many respects this RFC simply imports a minimal slice of
the [futures 0.2 API](http://aturon.github.io/2018/02/27/futures-0-2-RC/) into
libcore, and indeed
an [earlier iteration](https://github.com/rust-lang/rfcs/pull/2395) was
explicitly just that. This incarnation, however, focuses squarely on the
absolute minimal footprint needed to provide async/await support.

It's thus out of scope for this RFC to say *precisely* what the futures 0.3 APIs
will look like, but there are a few key aspects worth calling out. (Note that
over time, many of these APIs will be stabilized into `std`.)

### Error handling

It's very common to work with `Result`-producing futures, so the crate will
provide the following alias:

```rust
/// A convenience for futures that return `Result` values that includes
/// a variety of adapters tailored to such futures.
pub trait TryFuture {
    /// The type of successful values yielded by this future
    type Item;

    /// The type of failures yielded by this future
    type Error;

    /// Poll this `TryFuture` as if it were a `Future`.
    ///
    /// This method is a stopgap for a compiler limitation that prevents us from
    /// directly inheriting from the `Future` trait; in the future it won't be
    /// needed.
    fn try_poll(self: PinMut<Self>, cx: &mut task::Context) -> Poll<Result<Self::Item, Self::Error>>;
}

impl<F, T, E> TryFuture for F
    where F: Future<Output = Result<T, E>>
{
    type Item = T;
    type Error = E;

    fn try_poll(self: PinMut<Self>, cx: &mut task::Context) -> Poll<F::Output> {
        self.poll(cx)
    }
}
```

This alias makes it easy to require that a future return a `Result` (by bounding
by `TryFuture`), and to obtain the success/error types (by using the `Item` and
`Error` associated types).

Similarly, `PollResult<T, E>` is a type alias for `Poll<Result<T, E>>`.

### Combinators

The crate will provide extension traits, `FutureExt` and `TryFutureExt`, with a
full complement of combinators like those provided in futures 0.2.

### Conveniences for unpinned futures

The `FutureExt` trait will, in particular, provide a convenience for working
with `Unpin` types without having to use the pin APIs:

```rust
trait FutureExt: Future {
    fn poll_unpin(&mut self, cx: &mut task::Context) -> Poll<Self::Output>
        where Self: Unpin { ... }

    // ...
}
```

## Writing manual futures

Not all async code will be written using `async`/`await`, and it's important to
retain a solid experience for manually implementing futures.

Compared to the futures 0.2 experience, the main changes in this RFC are (1) the
use of `PinMut` and (2) no longer baking in an error type. In both cases, it's
straightforward, if a bit tedious to recover the previous programming model.

Here's an example drawn from [tower-grpc]:

[tower-grpc]: https://github.com/tower-rs/tower-grpc/blob/master/src/client/server_streaming.rs#L21-L32

```rust
// code written in futures 0.2 style

impl<T, U, B> Future for ResponseFuture<T, U>
where T: Message + Default,
      U: Future<Item = Response<B>>,
      B: Body<Data = Data>,
{
    type Item = ::Response<Streaming<T, B>>;
    type Error = ::Error<U::Error>;

    fn poll(&mut self, cx: &mut task::Context) -> Poll<Self::Item, Self::Error> {
        self.inner.poll(cx)
    }
}
```

To port this code, we systematically employ a few steps:

- Implementing `Unpin` for the manual future, which opts out of interior
  borrowing.
- Use `TryFuture` for all bounds over futures, and return `PollResult` in `poll`.
- Call `poll_unpin` rather than `poll` when polling inner futures.

All told, the ported code looks as follows:

```rust
// now ported to futures 0.3

impl<T, U> Unpin for ResponseFuture<T, U> {}

impl<T, U, B> Future for ResponseFuture<T, U>
where T: Message + Default,
      U: TryFuture<Item = Response<B>> + Unpin,
      B: Body<Data = Data>,
{
    type Output = Result<::Response<Streaming<T, B>>, ::Error<U::Error>>;

    fn poll(self: PinMut<Self>, cx: &mut task::Context) -> PollResult<Self::Item, Self::Error> {
        self.inner.poll_unpin(cx)
    }
}
```

These changes are mechanical enough that it's likely possible to write a script
to perform them with high accuracy.

To be clear, however, there is a definite ergonomic hit here, which is discussed
further in the drawbacks section.

## Stabilization plan

The ultimate goal is to ship async/await as part of Rust 2018 (roughly by
mid-September).

Much of the design has been vetted over quite a long period (futures 0.1), and
the 0.2 changes have gotten substantial use in Google's Fuchsia OS. The task
system in particular has existed in roughly the proposed shape for quite a long
time in the futures crate, and has thus already been quite thoroughly vetted.

Thus the major new elements are, once more, the removal of built-in errors and
the use of `PinMut`.

- For built-in errors, the situation is straightforward: we can provide
  equivalent functionality at mild ergonomic cost (an extra import).

- For `PinMut`, there's a clear way to "opt out" and recover the previous
  semantics (again at some ergonomic cost), so the primary questions come down
  to the `PinMut` API itself. The core types have already received significant
  vetting within the RustBelt formal model. In addition to the opt-out, there's
  ongoing work in making `PinMut` ergonomic and safe to use directly (rather
  than only through `async` notation).

Tactically, the proposal is to produce a 0.3-beta release, which will be
nightly-only, and to work through support in as large a chunk of the ecosystem
as we can manage, by adding feature flags to crates to opt in to the new
version.

## Details for `no_std` compatibility

The APIs proposed above are almost entirely compatible with `core`, except for a
couple of constructors that require `std` objects:

- Constructing a `Waker` from an `Arc<T>: Wake`
- Constructing a `TaskObj` from a future

These both have a similar shape: we have a concrete but opaque type (`Waker`,
`TaskObj`) that represents a trait object, but does *not* force a particular
*representation* for the trait object. In `std` environments, you can largely
gloss over this point and just use `Arc` or `Box` respectively. But internally,
the `Waker` and `TaskObj` types are more abstract.

We'll look at the `Waker` case in detail. The idea is to provide an `UnsafeWake`
trait which represents "an arbitrary `Wake`-like trait object":

```rust
/// An unsafe trait for implementing custom memory management for a
/// `Waker`.
///
/// A `Waker` conceptually is a cloneable trait object for `Wake`, and is
/// most often essentially just `Arc<T>: Wake`. However, in some contexts
/// (particularly `no_std`), it's desirable to avoid `Arc` in favor of some
/// custom memory management strategy. This trait is designed to allow for such
/// customization.
///
/// A default implementation of the `UnsafeWake` trait is provided for the
/// `Arc` type in the standard library.
pub unsafe trait UnsafeWake {
    /// Creates a new `Waker` from this instance of `UnsafeWake`.
    ///
    /// This function will create a new uniquely owned handle that under the
    /// hood references the same notification instance. In other words calls
    /// to `wake` on the returned handle should be equivalent to calls to
    /// `wake` on this handle.
    ///
    /// # Unsafety
    ///
    /// This function is unsafe to call because it's asserting the `UnsafeWake`
    /// value is in a consistent state, i.e. hasn't been dropped.
    unsafe fn clone_raw(self: *mut Self) -> Waker;

    /// Drops this instance of `UnsafeWake`, deallocating resources
    /// associated with it.
    ///
    /// # Unsafety
    ///
    /// This function is unsafe to call because it's asserting the `UnsafeWake`
    /// value is in a consistent state, i.e. hasn't been dropped
    unsafe fn drop_raw(self: *mut Self);

    /// Indicates that the associated task is ready to make progress and should
    /// be `poll`ed.
    ///
    /// Executors generally maintain a queue of "ready" tasks; `wake` should place
    /// the associated task onto this queue.
    ///
    /// # Panics
    ///
    /// Implementations should avoid panicking, but clients should also be prepared
    /// for panics.
    ///
    /// # Unsafety
    ///
    /// This function is unsafe to call because it's asserting the `UnsafeWake`
    /// value is in a consistent state, i.e. hasn't been dropped
    unsafe fn wake(self: *mut self);
}
```

We then provide the following constructor for `Waker`:

```rust
impl Waker {
    pub unsafe fn new(inner: *const dyn UnsafeWake) -> Waker;
}
```

and a `From<Arc<T>>` (where `Arc<T>: Wake`) impl that uses it.

## Task-local storage

This RFC does not propose any implicit, built-in task-local storage. (Explicit
storage is always possible).

Task-local storage is implementable on top of the proposed APIs by wrapping a
task in a *scoped* use of *thread*-local storage. When polling the task, a
thread-local value is established and hence usable implicitly within the call
chain. But when returning -- which also happens when the task is blocked -- the
thread-local is moved back out and stored with the task.

In the future, we anticipate adding "spawn hooks" for the `Context::spawn`
method, essentially allowing you to guarantee that tasks spawned within some
scope are wrapped in some way. That's a separately useful feature, but it can in
particular be used to implement inheritance of task-local data.

It may be that eventually we do want to build in some task-local data scheme, but:

- The no_std story is unclear.
- There are a lot of possible designs around things like typemaps and
  inheritance, and so it seems best for this work to begin in the ecosystem
  first.

# Rationale, drawbacks, and alternatives

This RFC is one of the most substantial additions to `std` proposed since
1.0. It commits us to including a particular task and polling model in the
standard library, and ties us to `PinMut`.

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

## Rationale, drawbacks and alternatives to the core trait design (wrt `PinMut`)

Putting aside error handling, which is orthogonal and discussed above, the
primary other big item in this RFC is the move to `PinMut` for the core polling
method, and how it relates to `Unpin`/manually-written futures. Over the course
of RFC discussions, we've identified essentially three main approaches to this
question:

- **One core trait**. That's the approach taken in the main RFC text: there's
  just a single core `Future` trait, which works on `PinMut<Self>`. Separately
  there's a `poll_unpin` helper for working with `Unpin` futures in manual
  implementations.

- **Two core traits**. We can provide two traits, for example `MoveFuture` and
  `Future`, where one operates on `&mut self` and the other on `PinMut<Self>`.
  This makes it possible to continue writing code in the futures 0.2 style,
  i.e. without importing `PinMut`/`Unpin` or otherwise talking about pins. A
  critical requirement is the need for interoperation, so that a `MoveFuture`
  can be used anywhere a `Future` is required. There are at least two ways to
  achieve such interop:

  - Via a blanket impl of `Future` for `T: MoveFuture`. This approach currently
    blocks some *other* desired impls (around `Box` and `&mut` specifically),
    but the problem doesn't appear to be fundamental.

  - Via a subtrait relationship, so that `T: Future` is defined essentially as
    an alias for `for<'a> PinMut<'a, T>: MoveFuture`. Unfortunately, such
    "higher ranked" trait relationships don't currently work well in the trait
    system, and this approach also makes things more convoluted when
    implementing `Future` by hand, for relatively little gain.

The drawback of the "one core trait" approach taken by this RFC is its ergonomic
hit when writing moveable futures by hand: you now need to import `PinMut` and
`Unpin`, invoke `poll_unpin`, and impl `Unpin` for your types. This is all
pretty mechanical, but it's a pain. It's possible that improvements in `PinMut`
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
conservative, forward-compatible option**. It's possible to add `MoveFuture`,
together with a blanket impl, at any point in the future. Thus, starting with
just the single `Future` trait as proposed in this RFC keeps our options
maximally open while we gain experience.

## Alternative no_std handling

Rather than using the `UnsafeWake` trait, we could factor "abstract `Arc`-like
trait objects" out as a first-class concept, `ArcObj`. We would also define an
`ArcLike` trait to determine what concrete types can fit into it:

```rust
// An `Arc`-like trait object for a trait `T`
//
// Implements `Send`, `Sync` and `Clone`
struct ArcObj<T: ?Sized> {
    inner: *mut T,

    // a manually-constructed vtable for the Arc-like methods
    drop_fn: unsafe fn(*mut T),
    clone_fn: unsafe fn(*mut T) -> ArcObj<T>,
}

unsafe impl<T: ?Sized + Send + Sync> Send for ArcObj<T> {}
unsafe impl<T: ?Sized + Send + Sync> Sync for ArcObj<T> {}

impl<T: ?Sized> Deref for ArcObj<T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.inner }
    }
}

// An object that can be used like `Arc<T>`
unsafe trait ArcLike<T: ?Sized>: Send + Sync {
    fn into_raw(self) -> *mut T;
    unsafe fn drop_fn(*mut T);
    unsafe fn clone_fn(*mut T) -> ArcObj<T>;
}

unsafe impl<T: ?Sized + Send + Sync> ArcLike<T> for Arc<T> {
    fn into_raw(self) -> *mut T {
        Arc::into_raw(self) as *mut T
    }

    unsafe fn drop_fn(t: *mut T) {
        drop(Arc::from_raw(t));
    }

    unsafe fn clone_fn(t: *mut T) -> ArcObj<T> {
        let val: Arc<T> = Arc::from_raw(t);
        let cloned = val.clone();
        mem::forget(val);
        ArcObj::new(cloned)
    }
}

impl<T: ?Sized> ArcObj<T> {
    fn new<U: ArcLike<T>>(u: U) -> ArcObj<T> {
        ArcObj {
            inner: u.into_raw(),
            drop_fn: U::drop_fn,
            clone_fn: U::clone_fn,
        }
    }
}

impl<T: ?Sized> Clone for ArcObj<T> {
    fn clone(&self) -> ArcObj<T> {
        unsafe {
            (self.clone_fn)(self.inner)
        }
    }
}

impl<T: ?Sized> Drop for ArcObj<T> {
    fn drop(&mut self) {
        unsafe {
            (self.drop_fn)(self.inner)
        }
    }
}
```

With this setup, we can define `Waker` as:

```rust
struct Waker {
    obj: ArcObj<dyn Wake>,
}
```

and allow construction from *any* `ArcObj<dyn Wake>`, rather than just
`Arc<dyn Wake>`, without using `UnsafeWake`.

However, this would involve `ArcObj` appearing in multiple places throughout the
API, rather than sequestering the niche case into just the `UnsafeWake` trait as
this RFC proposes.

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
