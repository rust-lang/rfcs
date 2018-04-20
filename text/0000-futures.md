- Feature Name: (fill me in with a unique ident, my_awesome_feature)
- Start Date: 2018-04-04
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

This RFC proposes to add futures to libcore, in order to support
the first-class `async`/`await` syntax proposed in a [companion RFC]. To start
with, we add the smallest fragment of the futures-rs library required, but we
anticipate follow-up RFCs ultimately bringing most of the library into libcore (to
provide a complete complement of APIs).

The proposed APIs are based on the futures crate, but with two major changes:

- The use of [pinned types] to enable borrowing within futures.
- Removing the associated `Error` type (and adjusting combinators accordingly),
  in favor of just using `Output = Result<T, E>` instead. The RFC includes an
  extension trait to provide conveniences for `Result`-producing futures as
  well.

[pinned types]: https://github.com/rust-lang/rfcs/pull/2349
[companion RFC]: https://github.com/rust-lang/rfcs/pull/2394

# Motivation
[motivation]: #motivation

There are two reasons to consider bringing futures into the standard library.

The first, and by far most important, is to provide a supporting mechanism for
`async`/`await` syntax:

```rust
async fn function(argument: &str) -> usize {
     // ...
}
```

The syntax itself is motivated in the [companion RFC]. As with closures, it
involves producing an anonymous type, so that the above declaration is
equivalent to:

```rust
fn function<'a>(argument: &'a str) -> _Anonymous<'a, usize> {
     // ...
}
```

and, again like a closure, the type is only usable through the trait it
implements: `Future`. Hence, to include this syntax in the language, we must
also introduce `Future` into `core` as a lang item.

The second reason to introduce futures is to establish them more formally as
*the* way to express composable, asynchronous computation in Rust. Over time,
the futures library has been refined to a core that bakes in almost no
assumptions and is usable in a wide variety of contexts, including operating
systems and embedded devices. As such, it has also become a *de facto* standard
in this space, and there is a strong desire for the core parts of the library to
reach the same level of stability as `std` itself. Bringing futures into libcore
is a final step in standardization.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The `Future` trait represents an *asynchronous* computation that may eventually
produce a final value, but don't have to block the current thread to do so.

Futures are usually constructed via *asynchronous functions*:

```rust
async fn read_frame(socket: &TcpStream) -> Result<Frame, io::Error> { ... }
```

This `async` function, when invoked, produces a *future* that represents the
completion of reading a frame from the given socket. The function signature
is equivalent to:

```rust
fn read_frame<'sock>(socket: &'sock TcpStream)
    -> impl Future<Output = Result<Frame, io::Error>> + 'sock;
```

Other async functions can *await* this asynchronous value; see the [companion
RFC] for details.

In addition to `async fn` definitions, futures can be built using adapters on
the `Future` trait, much like with `Iterator`s. The standard library includes a
number of basic adapters (described in the reference below), while some
particularly interesting variants are iterating in the crates.io ecosystem
first.

Ultimately asynchronous computations are executed by *tasks*, which are
lightweight threads. In particular, an *executor* is able to "spawn" a
`()`-producing future as an independent task; these tasks are then cooperatively
scheduled onto one or more operating system threads. The `Executor` trait
defines this interface, and the `task` module provides a host of related
definitions needed when manually implementing futures or executors.

*Note: additional guide-level documentation is available in
the [futures crate](https://docs.rs/futures/0.2.0-beta/futures/)*.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## `core::task` module

The fundamental mechanism of futures is *tasks*, which are lightweight
threads of execution; many tasks can be cooperatively scheduled onto a single
operating system thread.

To perform this cooperative scheduling we use a technique sometimes referred to
as a "trampoline". When a task would otherwise need to block waiting for some
event, instead it schedules itself for later wakeup and *returns* to the
executor running it, which can then run another task. Subsequent wakeups place
the task back on the executors queue of ready tasks, much like a thread
scheduler in an operating system.

Attempting to complete a task (or future within it) is called *polling*, and
always yields a `Poll` value back:

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
/// is ready to be `poll`ed again. Executors do so by implementing this trait.
pub trait Wake: Send + Sync {
    /// Indicates that the associated task is ready to make progress and should
    /// be `poll`ed.
    ///
    /// Executors generally maintain a queue of "ready" tasks; `wake` should place
    /// the associated task onto this queue.
    fn wake(&Arc<self>);
}
```

Note that this trait uses `Arc` and hence is only available in `std`; however,
it's a convenience on top of an `UnsafeWake` trait we'll see in the `no_std`
section.

In general futures are not coupled to any particular executor, so we use a trait
object to handle waking:

```rust
/// A `Waker` is a handle for waking up a task by notifying its executor that it
/// is ready to be run.
///
/// This handle contains a trait object pointing to an instance of the `UnsafeWake`
/// trait, allowing notifications to get routed through it.
pub struct Waker { ... }

impl Waker {
    /// Wake up the task associated with this `Waker`.
    pub fn wake(&self);
}

impl Clone for Waker { .. }

// We will see how to handle the no_std case later in the RFC...
impl<T> From<Arc<T>> for Waker where T: Wake + 'static { ... }
```

Task execution always happens in the context of a `Waker` that can be used to
wake the task up; we'll see the full `core::task::Context` structure below.

### Executors

An executor is responsible for polling tasks to completion. We represent this
with the `core::task::BoxExecutor` trait (more on the name below):

```rust
/// A task executor.
///
/// A *task* is a `()`-producing future that runs at the top level, and will
/// be `poll`ed until completion. It's also the unit at which wake-up
/// notifications occur. Executors, such as thread pools, allow tasks to be
/// spawned and are responsible for putting tasks onto ready queues when
/// they are woken up, and polling them when they are ready.
pub trait BoxExecutor {
    /// Spawn the given task, polling it until completion.
    ///
    /// # Errors
    ///
    /// The executor may be unable to spawn tasks, either because it has
    /// been shut down or is resource-constrained.
    fn spawn(&mut self, task: Task) -> Result<(), SpawnError>;

    /// Determine whether the executor is able to spawn new tasks.
    ///
    /// # Returns
    ///
    /// An `Ok` return means the executor is *likely* (but not guaranteed)
    /// to accept a subsequent spawn attempt. Likewise, an `Err` return
    /// means that `spawn` is likely, but not guaranteed, to yield an error.
    fn status(&self) -> Result<(), SpawnError> {
        Ok(())
    }
}

pub struct Task { .. }

// this impl is in `std` only:
impl From<Box<dyn Future<Output = ()> + Send>> for Task { .. }

/// Provides the reason that an executor was unable to spawn.
pub struct SpawnError { .. }

impl SpawnError {
    /// Spawning is failing because the executor has been shut down.
    pub fn shutdown() -> SpawnError;

    /// Check whether this error is the `shutdown` error.
    pub fn is_shutdown(&self) -> bool;

    // additional error variants added over time...
}
```

We need the executor trait to be usable as a trait object, which is why `Task`
is constructed here from a boxed future. (In the no_std section, we'll see
another constructor). In the long run, though, once we can take `dyn` by value,
we would deprecate `BoxExecutor` and have:

```rust
trait Executor {
    fn spawn(&mut self, task: Future<Output = ()> + Send) -> Result<(), SpawnError>;
    fn status(&self) -> Result<(), SpawnError> { .. }
}

impl<E: BoxExecutor> Executor for E {
    /* implement by boxing */
}
```

This is why the RFC proposes the name `BoxExecutor` for the trait.

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
    /// Note: this signature is future-proofed for `E: Executor` later.
    pub fn new<E>(waker: &'a Waker, executor: &'a mut E) -> Context<'a>
        where E: BoxExecutor;

    /// Get the `Waker` associated with the current task.
    pub fn waker(&self) -> &Waker;

    /// Spawn a future onto the default executor.
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
straightforward.  Like `Iterator`, the `Future` trait has a single required
method, `poll`, and a large number of provided methods (called "adapters" or
"combinators"). We'll look first at `poll`:

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
    /// - `Poll::Pending` if the future is not ready yet.
    /// - `Poll::Ready(val)` with the result `val` of this future if it completed.
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
    /// Futures alone are *inert*; they must be *actively* `poll`ed to make
    /// progress, meaning that each time the current task is woken up, it should
    /// actively re-`poll` pending futures that it still has an interest in.
    /// Usually this is handled automatically by `async`/`await` notation or
    /// via adapter methods. Executors ensure that each task is `poll`ed every
    /// time a future internal to that task is ready to make progress.
    ///
    /// The `poll` function is not called repeatedly in a tight loop for
    /// futures, but only whenever the future itself is ready, as signaled via
    /// `cx.waker()`. If you're familiar with the `poll(2)` or `select(2)`
    /// syscalls on Unix it's worth noting that futures typically do *not*
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
    fn poll(self: Pin<Self>, cx: &mut task::Context) -> Poll<Self::Item>;
}
```

Most of the explanation here follows what we've already said about the task
system.  The one new twist is the use of `Pin`, which makes it possible to keep
data borrowed across separate calls to `poll` (i.e., "borrowing over yield
points"). The mechanics of pinning are explained
in [the RFC that introduced it](https://github.com/rust-lang/rfcs/pull/2349),
but one key point is that when implementing `Future` by hand, if you don't wish
to use this borrowing feature, you can treat `Pin<Self>` just like `&mut self`.

### Universal adapters

There are several basic adapters, modeled on `Iterator`, that work for arbitrary
futures. Using `impl Trait` notation, we can define them as follows:

```rust
trait Future {
    // Transform the result of the future
    fn map<T>(self, f: impl FnOnce(Self::Item) -> T) -> impl Future<Output = T>
        { .. }

    // Chain a future onto this one
    fn then<F>(self, f: impl FnOnce(Self::Item) -> F) -> impl Future<Output = F::Item>
        where F: Future
        { .. }

    // Chain a closure for side effects
    fn inspect(self, f: impl FnOnce(&Self::Item)) -> impl Future<Output = Self::Item>
        { .. }

    // Translate unwinding within this future into a `Result`
    fn catch_unwind(self) -> impl Future<Output = Result<Self::Item, Box<Any + Send>>>
        where Self: UnwindSafe
        { .. }
}
```

These adapters are all straightforward and based on substantial prior art.

More interesting adapters like `shared`, `select` and `join` will be left to the
ecosystem to iterate on before being RFC'ed for `std`.

### `Result`-specific adapters

Futures are often enough used with `Result` values that we provide a distinct
subtrait for that case, equipped with some additional adapters:

```rust
trait FutureResult<T, E>: Future<Output = Result<T, E>> {
    // Transform the successful result of the future
    fn map_ok<U>(self, f: impl FnOnce(T) -> U) -> impl FutureResult<U, E>
        { .. }

    // Transform the error result of the future
    fn map_err<F>(self, f: impl FnOnce(E) -> F) -> impl FutureResult<T, F>
        { .. }

    // Chain a future onto this one on success
    fn and_then<F, U>(self, f: impl FnOnce(T) -> F) -> impl FutureResult<U, E>
        where F: FutureResult<U, E>
        { .. }

    // Chain a future onto this one on failure
    fn or_else<F, G>(self, f: impl FnOnce(E) -> F) -> impl FutureResult<T, G>
        where F: FutureResult<T, G>
        { .. }

    // Pass the error type through an arbitrary conversion
    fn err_into<F>(self) -> impl FutureResult<T, F>
        where E: Into<F>
        { .. }

    // Handle the error provided by this future
    fn recover<F>(self, f: impl FnOnce(E) -> F) -> impl Future<Output = T>
        where F: Future<Output = T>
        { .. }
}

// Automatically applied to all `Result`-returning futures
impl<T, E, F> FutureResult<T, E> for F where F: Future<Output = Result<T, E>> {}
```

## Stabilization plan

The holy grail would be to stabilize async/await for Rust 2018 (roughly by mid-September).

As of this writing, the futures crate is just about to release its 0.2 version;
some details are available [here](http://aturon.github.io/2018/02/27/futures-0-2-RC/).

The APIs proposed here roughly correspond to the `futures-core` part of the
crate, plus some adapters that are currently within `futures-util`. However,
there are two substantial changes in this RFC:

- The use of [pinned types] to enable borrowing within futures.
  - It's currently not possible to use this API due to rustc limitations; these
    are expected to be addressed very soon.

- Removing the associated `Error` type (and adjusting combinators accordingly).
  - This change has been long desired, but didn't make it for the 0.2 release.

Concurrent with this RFC, the futures team plans to do the following:

- Create a 0.3 branch that fully matches this RFC.

- Publish the 0.3 version, initially as nightly-only, as soon as the limitations
  around pinning are lifted.

- Publish a 0.3.x version that works on the stable channel, as soon as pinning
  is stable.

The idea is for futures 0.3 to be a "release candidate" for inclusion in `std`,
and to gain as much feedback as possible as early as possible. In particular,
the fact that the external crate will be usable on stable before this RFC is
stabilized allows us to gather a wider array of feedback.

Once the proposed APIs are available in `std`, a 0.3.x version can be published
that simply re-exports them. In other words, the 0.3 release will be
forward-compatible with the `std` version.

A lot of functionality beyond this RFC, e.g. streams, will remain available only
in the futures crate. This RFC proposes only a minimal core needed to support
async/await, allowing for further iteration in the rest of the stack. The
intent, however, is for most of what's in the futures crate to eventually make
its way into `std`.

## Details for `no_std` compatibility

The APIs proposed above are almost entirely compatible with `core`, except for a
couple of constructors that require `std` objects:

- Constructing a `Waker` from an `Arc<dyn Wake>`
- Constructing a `Task` from a `Box<dyn Future>`

These both have a similar shape: we have a concrete but opaque type (`Waker`,
`Task`) that represents a trait object, but does *not* force a particular
*representation* for the trait object. In `std` environments, you can largely
gloss over this point and just use `Arc` or `Box` respectively. But internally,
the `Waker` and `Task` types are more abstract.

We'll look at the `Waker` case in detail. The idea is to provide an `UnsafeWake`
trait which represents "an arbitrary `Wake`-like trait object":

```rust
/// An unsafe trait for implementing custom memory management for a
/// `Waker`.
///
/// A `Waker` conceptually is a cloneable trait object for `Wake`, and is
/// most often essentially just `Arc<dyn Wake>`. However, in some contexts
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

and a `From<Arc<dyn Wake>>` impl that uses it.

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

# Drawbacks
[drawbacks]: #drawbacks

This RFC is one of the most substantial additions to `std` proposed since
1.0. It commits us not just to futures, but to a specific futures model. The
stakes are rather high.

So, however, are the benefits. The borrow checker integration that's possible
with async/await notation is a incredible boon to asynchronous programming that
will solve myriad problems with the async ecosystem today, including error
messages and learnability. In particular, integrating borrowing means it's
possible to follow synchronous patterns for things like the `read` function
(which want to hang on to a buffer reference). It's hard to overstate the
impact. Given the importance of async programming to Rust in general (and
in [2018 in particular](https://github.com/rust-lang/rfcs/pull/2314)), it seems
quite prudent to seek these benefits and try to make them part of the 2018
edition.

On the risk-mitigation side, the core futures model is at this point
battle-tested, and is nearing two years of age. While futures 0.2, and this RFC,
both bring a fair amount of change, these changes are all in the form of
simplifications (e.g. dropping the `Error` type), streamlining (the executor
revamp), or making the model more explicit (the task context argument). Speaking
subjectively, the APIs proposed here, relative to the initial futures 0.1
release, feel vastly closer to "canonical".

The RFC carves out a highly conservative set of the most clear-cut APIs,
allowing us plenty of time to iterate on things like streams before bringing
them into the language.

Finally, it's worth noting that futures are already the *de facto* standard for
Rust's async ecosystem, so it's not clear that bringing them into `std`
substantially changes the risk profile.

# Rationale and alternatives
[alternatives]: #alternatives

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

Some additional context and rationale is available in the [companion RFC].

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

None at present
