- Start Date: (fill me in with today's date, 2014-08-21)
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

This RFC proposes a significant simplification to the I/O stack distributed with
Rust. It proposes to move green threading into an external Cargo package, and
instead weld `std::io` directly to the native threading model.

The `std::io` module will remain completely cross-platform.

# Motivation

## Where Rust is now

Rust has gradually migrated from a green threading (lightweight task) model
toward a native threading model:

* In the green threading (M:N) model, there is no direct correspondence between
  a Rust task and a system-level thread. Instead, Rust tasks are managed using a
  runtime scheduler that maps them to some small number of
  underlying system threads. Blocking I/O operations at the Rust level are
  mapped into asyc I/O operations in the runtime system, allowing the green task
  scheduler to context switch to another task.

* In the native threading (1:1) model, a Rust task is equivalent to a
  system-level thread. I/O operations can block the underlying thread, and
  scheduling is performed entirely by the OS kernel.

Initially, Rust supported only the green threading model. Later, native
threading was added and ultimately became the default.

In today's Rust, there is a single I/O API -- `std::io` -- that provides
blocking operations only and works with both threading models. It is even
possible to use both threading models within the same program.

## The problems

While the situation described above may sound good in principle, there are
several problems in practice.

**Forced co-evolution.** With today's design, the green and native
  threading models must provide the same I/O API at all times. But
  there is functionality that is only appropriate or efficient in one
  of the threading models.

  For example, the lightest-weight green threading models are essentially just
  collections of closures, and do not provide any special I/O support (this
  style of green threading is used in Servo, but also shows up in
  [java.util.concurrent's exectors](http://docs.oracle.com/javase/7/docs/api/java/util/concurrent/Executors.html)
  and [Haskell's par monad](https://hackage.haskell.org/package/monad-par),
  among many others). On the other hand, green threading systems designed
  explicitly to support I/O may also want to provide low-level access to the
  underlying event loop -- an API surface that doesn't make sense for the native
  threading model.

  Under the native model we ultimately want to provide non-blocking and/or
  asynchronous I/O support. These APIs may involve some platform-specific
  abstractions (Posix `select` versus Windows iocp) for maximal performance. But
  integrating them cleanly with a green threading model may be difficult or
  impossible -- and at the very least, makes it difficult to add them quickly
  and seamlessly to the current I/O system.

  In short, the current design couples threading and I/O models together, and
  thus forces the green and native models to supply a common I/O interface --
  despite the fact that they are pulling in different directions.

**Overhead.** The current Rust model allows runtime mixtures of the green and
  native models. The implementation achieves this flexibility by using trait
  objects to model the entire I/O API. Unfortunately, this flexibility has
  several downsides:

- *Binary sizes*. A significant overhead caused by the trait object design is that
  the entire I/O system is included in any binary that statically links to
  `libstd`. See
  [this comment](https://github.com/rust-lang/rust/issues/10740#issuecomment-31475987)
  for more details.

- *Task-local storage*. The current implementation of task-local storage is
  designed to work seamlessly across native and green threads, and its performs
  substantially suffers as a result. While it is feasible to provide a more
  efficient form of "hybrid" TLS that works across models, doing so is *far*
  more difficult than simply using native thread-local storage.

- *Allocation and dynamic dispatch*. With the current design, any invocation of
  I/O involves at least dynamic dispatch, and in many cases allocation, due to
  the use of trait objects. However, in most cases these costs are trivial when
  compared to the cost of actually doing the I/O (or even simply making a
  syscall), so they are not strong arguments against the current design.

**Problematic I/O interactions.** As the
  [documentation for libgreen](http://doc.rust-lang.org/green/#considerations-when-using-libgreen)
  explains, only some I/O and synchronization methods work seamlessly across
  native and green tasks. For example, any invocation of native code that calls
  blocking I/O has the potential to block the worker thread running the green
  scheduler. In particular, `std::io` objects created on a native task cannot
  safely be used within a green task. Thus, even though `std::io` presents a
  unified I/O API for green and native tasks, it is not fully interoperable.

**Embedding Rust.** When embedding Rust code into other contexts -- whether
  calling from C code or embedding in high-level languages -- there is a fair
  amount of setup needed to provide the "runtime" infrastructure that `libstd`
  relies on. If `libstd` was instead bound to the native threading and I/O
  system, the embedding setup would be much simpler.

**Maintenance burden.** Finally, `libstd` is made somewhat more complex by
  providing such a flexible threading model. As this RFC will explain, moving to
  a strictly native threading model will allow substantial simplification and
  reorganization of the structure of Rust's libraries.

# Detailed design

To mitigate the above problems, this RFC proposes to tie `std::io` directly to
the native threading model, while moving `libgreen` and its supporting
infrastructure into an external Cargo package with its own I/O API.

## A more detailed look at today's architecture

To understand the detailed proposal, it's first necessary to understand how
today's libraries are structured.

Currently, Rust's runtime and I/O abstraction is provided through `librustrt`,
which is re-exported as `std::rt`:

* The `Runtime` trait abstracts over the scheduler (via methods like
  `deschedule` and `spawn_sibling`) as well as the entire I/O API (via
  `local_io`).

* The `rtio` module provides a number of traits that define the standard I/O
  abstraction.

* The `Task` struct includes a `Runtime` trait object as the dynamic entry point
  into the runtime.

In this setup, `libstd` works directly against the runtime interface. When
invoking an I/O or scheduling operation, it first finds the current `Task`, and
then extracts the `Runtime` trait object to actually perform the operation.

The actual scheduler and I/O implementations -- `libgreen` and `libnative` --
then live as crates "above" `libstd`.

## The near-term plan

The basic plan is to decouple *task scheduling* from the basic *I/O*
interface:

- An API for abstracting over schedulers -- the ability to block and wake a
  "task" -- will remain available, but as part of `libsync` rather than
  `librustrt`.

- The `std::io` API will be tied directly to native I/O.

### Tasks versus threads

In the proposed model, threads and tasks *both* exist and play a role in Rust:

- Rust code always runs in the context of some (native) thread. We will add
  direct support for native thread-local storage, spawning native threads, etc.

- The `libsync` crate will provide a notion of *task* that supports explicit
  blocking and waking operations (**NOTE**: this is different from e.g. calling
  blocking I/O; it is an explicit request from Rust code to block a task). At
  the outset, Rust programs will run in the context of a native task, where
  blocking just blocks the underlying thread. But green threading libraries can
  introduce their own task implementation, via scoped thread-local storage,
  which will allow blocking a green task without blocking the underlying native
  worker thread.

The notion of task and its associated API is described next.

### Scheduler abstraction

Above we described numerous problems with trying to couple I/O and threading
models and thereby impose a single I/O model.

However, concurrency structures built within Rust -- locks, barriers, channels,
concurrent containers, fork/join and data-parallel frameworks, etc. -- will all
need the ability to block and wake threads/tasks. **NOTE**: this is an
*explicit* request to block in Rust code, rather than as a side-effect of making
a system call.

This RFC proposes a simple scheduler abstraction, partly inspired by
[java.util.concurrent](http://docs.oracle.com/javase/7/docs/api/java/util/concurrent/locks/LockSupport.html)
and partly by our current runtime infrastructure.  Concurrency structures that
use this abstraction can be used freely under multiple threading models at the
same time. Here is a *sketch* of the API, which will need some experimentation
before nailing down in full detail:

```rust
// details TBD, but WakeupHandle: Send + Clone
type WakeupHandle = ...;

impl WakeupHandle {
    /// Attempt to wake up the task connected to this handle.
    ///
    /// Each `WakupHandle` is associated with a particular invocation of
    /// `block_after`; only one call to `wake` will take effect per invocation.
    /// Returns `true` if `wake` actually woke up the task.
    ///
    /// Note that the task may no longer exist by the time `wake` is invoked.
    fn wake(self) -> bool;
}

trait Task {
    /// Give up the current timeslice.
    fn yield_now(&self);

    /// Blocks the current task after executing the callback `f`.
    ///
    /// Blocking can be canceled by using `wake` on the `WakeupHandle`.
    fn block_after(&self, f: |WakeupHandle|);
}

/// Get access to scheduling operations on the current task.
fn cur_task(|&Task|);
```

The above API will be exported in `libsync`. The idea is that `cur_task` reads
from a dynamically-scoped thread-local variable to get a handle to a `Task`
implementation. By default, that implementation will equate "task" and "thread",
blocking and waking the underlying native thread. But green threading libraries
can run code with an updated task that hooks into their scheduling infrastructure.

To build a synchronization construct like blocking channels, you use the
`block_after` method. That method invokes a callback with a *wakeup handle*,
which is `Send` and `Clone`, and can be used to wake up the task. The task will
block after the callback finishes execution, but the wakeup handle can be used
to abort blocking.

For example, when attempting to receive from an empty channel, you would use
`block_after` to get a wakeup handle, and the store that handle within the
channel so that future senders can wake up the receiver.  After storing the
handle, however, the receiver's callback for `block_after` must check that no
messages arrived in the meantime, canceling blocking if they have.

The API is designed to avoid spurious wakeups by tying wakeup handles to
specific `block_after` invocations, which is an improvement over the
java.util.concurrent API.

A key point with the design is that wakeup handles are abstracted over the
actual scheduler being used, which means that for example a blocked green task
can safely be woken by a native task. While the exact definition of a wakeup
handle still needs to be worked out, it will contain a trait object so that the
`wake` method will dispatch to the scheduler that created the handle.

### `std::io` and native threading

The plan is to entirely remove `librustrt`, including all of the traits.
The abstraction layers will then become:

- Highest level: `libstd`, providing cross-platform, high-level I/O and
  scheduling abstractions.  The crate will depend on `libnative` (the opposite
  of today's situation).

- Mid-level: `libnative`, providing a cross-platform Rust interface for I/O and
  scheduling. The API will be relatively low-level, compared to `libstd`. The
  crate will depend on `libsys`.

- Low-level: `libsys` (renamed from `liblibc`), providing platform-specific Rust
  bindings to system C APIs.

In this scheme, the actual API of `libstd` will not change significantly. But
its implementation will invoke functions in `libnative` directly, rather than
going through a trait object.

A goal of this work is to minimize the complexity of embedding Rust code in
other contexts. It is not yet clear what the final embedding API will look like.

### Green threading

Despite tying `libstd` to native threading, however, green threading will still
be supported. The infrastructure in `libgreen` and friends will move into its
own Cargo package.

Initially, the green threading package will support essentially the same
interface it does today; there are no immediate plans to change its API, since
the focus will be on first improving the native threading API. Note, however,
that the I/O API will be exposed separately within `libgreen`, as opposed to the
current exposure through `std::io`.

The library will be maintained to track Rust's development, and may ultimately
undergo significant new development; see "The long-term plan" below.

## The long-term plan

Ultimately, a large motivation for the proposed refactoring is to allow the APIs
for native and green threading and I/O to grow and diverge.

In particular, over time we should expose more of the underlying system
capabilities under the native threading model. Whenever possible, these
capabilities should be provided at the `libstd` level -- the highest level of
cross-platform abstraction. However, an important goal is also to provide
nonblocking and/or asynchronous I/O, for which system APIs differ greatly (Posix
`select` versus Windows `iocp`). It may be necessary to provide additional,
platform-specific crates to expose this functionality. Ideally, these crates
would interoperate smoothly with `libstd`, so that for example a `libposix`
crate would allow using a `select` operation directly against a
`std::io::fs::File` value.

We may also wish to expose "lowering" operations in `libstd` -- APIs that allow
you to get at the file descriptor underlying a `std::io::fs::File`, for example.

Finally, there is a lot of room to evolve `libgreen` by exposing more of the
underlying event loop functionality. At the same time, it is probably worthwhile
to build an alternative, "very lightweight" green threading library that does
not provide any event loop or I/O support -- the "green threads" are essentially
just closures. Servo already makes use of such a model in some places internally.

All of the above long-term plans will require substantial new design and
implementation work, and the specifics are out of scope for this RFC. The main
point, though, is that the refactoring proposed by this RFC will make it much
more plausible to carry out such work.

# Drawbacks

The main drawback of this proposal is that green I/O will be provided by a
forked interface of `std::io`. This change makes green threading feel a bit
"second class", and means there's more to learn when using both models
together.

This setup also somewhat increases the risk of invoking native blocking I/O on a
green thread -- though of course that risk is very much present today. One way
of mitigating this risk in general is the Java executor approach, where the
native "worker" threads that are executing the green thread scheduler are
monitored for blocking, and new worker threads are spun up as needed.

# Unresolved questions

There are may unresolved questions about the exact details of the refactoring,
but these are considered implementation details since the `libstd` interface
itself will not substantially change as part of this RFC.
