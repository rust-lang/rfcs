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
- A new `Async` trait, which integrates the [`Pin` APIs][pin] with the task system to
  provide async values that can interoperate with futures.

[pin]: https://github.com/rust-lang/rfcs/pull/2349
[companion RFC]: https://github.com/rust-lang/rfcs/pull/2394

The RFC also covers the intended ecosystem migration path, as well as the
possibility of eventually deprecating `Future` in favor of `Async`.

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
implements: `Async`. The goal of this RFC is to provide a concrete proposal for
this `Async` trait, based on the work pioneered by the futures crate.

A secondary benefit of this RFC is that it enshrines the *task system* currently
defined by the futures crate into `libcore`, thereby standardizing and
ultimately stabilizing the async ecosystem around a single lightweight task
mechanism.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The `Async` trait represents an *asynchronous* and lazy computation that may
eventually produce a final value, but doesn't have to block the current thread
to do so.

Async values can be constructed through `async` blocks or `async` functions, e.g.,

```rust
async fn read_frame(socket: &TcpStream) -> Result<Frame, io::Error> { ... }
```

This `async` function, when invoked, produces an async value that represents the
completion of reading a frame from the given socket. The function signature
is equivalent to:

```rust
fn read_frame<'sock>(socket: &'sock TcpStream)
    -> impl Async<Output = Result<Frame, io::Error>> + 'sock;
```

Other async functions can *await* this asynchronous value; see the [companion
RFC] for full details.

In addition to `async fn` definitions, async values can be built using adapters on
the `Async` trait, much like with `Iterator`s. The standard library includes a
number of basic adapters (described in the reference below), while some
particularly interesting variants are iterating in the crates.io ecosystem
first.

Ultimately asynchronous computations are executed by *tasks*, which are
lightweight threads. In particular, an *executor* is able to "spawn" a
`()`-producing `Async` value as an independent task; these tasks are then
cooperatively scheduled onto one or more operating system threads. The
`Executor` trait defines this interface, and the `task` module provides a host
of related definitions needed when manually implementing `Async` values or
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
    pub fn new<A: Async<Output = ()> + Send + 'static>(f: A) -> TaskObj;
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
    fn spawn(&mut self, task: Async<Output = ()> + Send) -> Result<(), SpawnErrorKind> {
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
    pub fn spawn(&mut self, f: impl Async<Output = ()> + 'static + Send);

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

## `core::ops` module

With all of the above task infrastructure in place, defining `Async` is
straightforward. Since it's a "lang item", i.e. a trait that the language itself
treats specially when compiling `async` blocks, the trait goes into the
`core::ops` module, much as with the `Fn` family of traits.

```rust
pub trait Async {
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
    /// Once an async value has completed, clients should not `poll` it again.
    ///
    /// When an async value is not ready yet, `poll` returns `Poll::Pending`.
    /// The computation will *also* register the interest of the current task in the
    /// value being produced. For example, if the async value represents the availability
    /// of data on a socket, then the task is recorded so that when data arrives,
    /// it is woken up (via `cx.waker()`). Once a task has been woken up,
    /// it should attempt to `poll` the computation again, which may or may not
    /// produce a final value at that time.
    ///
    /// Note that if `Pending` is returned it only means that the *current* task
    /// (represented by the argument `cx`) will receive a notification. Tasks
    /// from previous calls to `poll` will *not* receive notifications.
    ///
    /// # Runtime characteristics
    ///
    /// `Async` values are *inert*; they must be *actively* `poll`ed to make
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
    /// Once an async value has completed (returned `Ready` from `poll`), subsequent
    /// calls to `poll` may panic, block forever, or otherwise cause bad behavior.
    /// The `Async` trait itself provides no guarantees about the behavior of
    /// `poll` after a future has completed.
    fn poll(self: Pin<Self>, cx: &mut task::Context) -> Poll<Self::Output>;
}
```

Most of the explanation here follows what we've already said about the task
system. The one twist is the use of `Pin`, which makes it possible to keep data
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
will look like. However, in broad outlines:

- The 0.3 release will continue to provide a `Future` trait that resembles the
  one in 0.2. In particular, the trait will *not* use `Pin`, and hence will
  continue to work with today's stable Rust. (You can think of `Future` roughly
  as an alias for `Async + Unpin`).

- When a `nightly` flag is enabled, the crate will re-export the libcore version
  of the task system rather than defining its own.

  - The nightly flag will include mechanisms for going between `Future` and `Async`
  traits; any `Future` is trivially `Async`, and any `Async` value within a
  `PinBox` can implement `Future`.

  - It will also provide combinators for `Async` values, including select and join,
  so that these values can be composed without going through boxing/`Future`.

The upshot is:

- Futures 0.3 can be released very quickly and immedately work on stable Rust.
- Migrating to 0.3 should be as easy as migrating to 0.2.
- Code that works with futures 0.3 will be compatible with async/await when used
  on nightly, allowing for experimentation while using the futures ecosystem.
- The futures API remains "out of tree", allowing for further iteration even
  as `Async` and async/await notation are stabilized.

The downside is that using `Async` values with code that expects a `Future` will
require boxing for the time being.

In the long run, as we strengthen support and abstractions for working directly
with `Pin` values, it may turn out that there's little reason to avoid working
with `Async`/`Pin` directly, in which case we can deprecate `Future` and move
the ecosystem toward `Async`. However, the strategy proposed here gives us time
and leeway to investigate this question decoupled from shipping async/await
itself.

The question of whether to build in an `Error` type for `Future`, and other such
details, will be tackled more directly in the futures repo.

## Stabilization plan

The ultimate goal is to ship async/await as part of Rust 2018 (roughly by
mid-September).

This RFC is designed to migrate to a version of futures that can work with
async/await. In particular, foundational crates in the ecosystem (like Tokio and
Hyper) can continue working on *stable* Rust and the futures crate, while
*clients* of those crates can opt in to nightly to start using async/await. If
we can move quickly on these foundational crates, we should be able to have
several additional months of testing with async/await while still shipping in
the new edition.

The task system has existed in roughly the proposed shape for quite a long time
in the futures crate, and has thus already been quite thoroughly vetted.

The `Async` design is newer, but is closely related to `Future` in the futures
0.2 crate, which has seen very heavy usage in at least Google's Fuchsia OS. The
main differences from 0.1 are the use of an explicit task context argument
(rather than thread-local storage) and not baking in `Result`. But it's possible
to provide a futures 0.1-style API on top, if for some reason that is deemed
necessary. Thus, `Async` is also low-risk to stabilize.

Probably the biggest open question for stabilization is the `Pin`
type. Stabilizing `Async` wholesale would require stabilizing `Pin`, but if for
some reason we are not comfortable doing so for the 2018 edition, there's a very
simple way to punt:

```rust
// Essentially `PinBox<dyn Async<Output = T>>`
struct BoxAsync<T> { .. }

impl<T> BoxAsync<T> {
    fn new<A: Async<Output = T>>(a: A) -> BoxAsync<T>;
    fn poll(&mut self, cx: &mut task::Context) -> Poll<T>;
}
```

That is, we can provide an `&mut`-based polling API that's tied to boxing
`Async` values, and provide a way to put them into boxes, without stabilizing
`Pin` or even `Async` itself.

In short, it should be quite plausible to stabilize async/await in time for the
2018 edition given that the minimal such stabilization covers mechanisms and
APIs that have either already been thoroughly vetted, or are minimal commitment.

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

# Drawbacks
[drawbacks]: #drawbacks

This RFC is one of the most substantial additions to `std` proposed since
1.0. It commits us to including a particular task and polling model in the
standard library. The stakes are high.

However, as argued in the stabilization section above, the meat of the proposal
has at this point already been thoroughly vetted; the core ideas go back about
two years at this point. It's possible to carve an extremely minimal path to
stabilization that essentially sticks to these already-proven ideas. Likewise,
async/await support (via generators) has already existing on the nightly channel
for quite a long time.

So far we've been able to push the task/polling model into virtually every niche
Rust wishes to occupy, and the main downside has been, in essence, the lack of
async/await syntax (and
the
[borrowing it supports](http://aturon.github.io/2018/04/24/async-borrowing/)).

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

- The futures 0.3 API, including how we wish to handle `Result` (and whether
  e.g. it should provide an `AsyncResult` trait as well). This discussion will
  take place separately from the RFC.
