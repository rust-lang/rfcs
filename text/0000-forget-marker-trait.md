- Feature Name: `forget_marker_trait`
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

<!-- todo: Replace with RFC PR later -->
[`local_default_bounds`]: https://github.com/rust-lang/rfcs/pull/3783

# Summary
[summary]: #summary

Add a `Forget` marker trait indicating whether it is safe to skip the destructor before the value of a type exits the scope and basic utilities to work with `!Forget` types. Introduce a seamless migration route for the standard library and ecosystem.

# Motivation
[motivation]: #motivation

Back in 2015, the [decision was made][safe-mem-forget] to make `mem::forget` safe, making every type effectively implement `Forget`. All the APIs in `std` were able remain safe after this change, except one. This RFC is not targeted at resource leaks in general, but is instead focused on allowing a number of APIs to become safe, by providing new unsafe guarantees using `Forget`.

In short, the lack of `!Forget` types undermines lifetimes, sacrificing all 3 of performance, ergonomics and efficiency. Most Rust code, as well as external APIs, naturally converge towards `!Forget` types, but in the absence of `Forget` trait support, those APIs use a mixture of `Arc`, `'static`, allocations, etc.

`Forget` is designed to allow proxy RAII guards. Futures needed for `io_uring`-like APIs are essentially a proxy RAII guard, it will be explained later.

Many Rust programmers may find the biggest problem with `Forget` to be migration. But this RFC describes how migration can be done easily. See [#ecosystem-migration](#ecosystem-migration) section for details.

[safe-mem-forget]: https://github.com/rust-lang/rfcs/pull/1066

### What is a proxy RAII guard?
[proxy-raii-guards]: #proxy-raii-guards

`thread::scoped` was special because it used the RAII guard [^raii] as a proxy to represent other values, but this proxy was not used to access those values. Instead, we are trusted that the borrow checker will ensure that the guard cannot outlive those values, and therefore that joining the thread in the guard's destructor is enough to ensure that the spawned thread is no longer running. [^proxy-raii-guard-source]

[^raii]: https://rust-unofficial.github.io/patterns/patterns/behavioural/RAII.html

```rust
struct JoinHandle<'a>(/* ... */);

impl Drop for JoinHandle<'_> {
    fn drop() {
        // Join the thread
    }
}

let mut buffer = [0u8; 1024];

// `guard` is now borrowing from `buffer`
let guard = thread::scoped(|| {
    for i in 0..1000 {
        buffer[i] = i;
    }
});

buffer[3] = 4; // Error: `buffer[_]` is assigned to here but it was already borrowed
```

As we can see, `buffer` is borrowed for a lifetime `'a`, until `guard` is live. But `buffer` is used inside another thread, not directly inside `JoinHandle<'a>`. Thus, `JoinHandle` is a *proxy* RAII guard, and its drop handler is used for then necessary cleanup.

[^proxy-raii-guard-source]: https://github.com/rust-lang/rfcs/pull/1084#issuecomment-96875651

### Why is the proxy RAII guard gone?
[proxy-raii-guards-leakpokaplipse]: #proxy-raii-guards-leakpokaplipse

We can't use proxy RAII guard to ensure cleanup anymore. In 2015, the [leakpocalypse] happened, and the language faced the question: do we make it safe to skip destructors or not? [PPYP] allows data structures to provide RAII guards while being resilient to skipping the destructor. The only use case in std that cannot be expressed without destructor always running was `JoinGuard`, [which later got replaced too][thred-scope-doc].

[leakpocalypse]: https://github.com/rust-lang/rust/issues/24292
[PPYP]: https://cglab.ca/~abeinges/blah/everyone-poops/
[thred-scope-doc]: https://doc.rust-lang.org/std/thread/fn.scope.html

In sync Rust, necessary cleanup can be achieved by taking a closure/callback instead of returning a guard object:

```rust
fn something_with_clean_up(f: impl FnOnce(Foo)) {
    // Setup.
    f(Foo);
    // Cleanup. It is *guaranteed* to run, given the proper handling of unwinding.
}

fn main() {
    something_with_clean_up(|foo| {
        foo.bar();
    });

    // rest of the code...
}
```

As you can see, after calling `something_with_clean_up`, the control flow is passed to the library. The rest of the user's code *cannot* continue executing before `something_with_clean_up` performs a cleanup - so it can, for example, derigester pointer from external code or restore broken invariants.

Thus, there was no point in redesigning the language and delaying Rust 1.0, practically all APIs and patterns could be safely expressed without destructors always running, so making `std::mem::forget` safe and removing "Proxy RAII Guard" was a good decision at the time.

## What is different
[what-is-different]: #what-is-different

Edition 2018 introduced `async` Rust. But as turned out, nuances in its design conflicted with an earlier decision. All `async` calls are essentially constructors for state machines which borrow some resources from outside or directly own them. It is user's responsibility to poll those state machines to completion.

`!Forget` use cases could've been expressed by other means in sync Rust (like taking a callback instead of returning a guard or [PPYP]), but with `async`, anything turns directly into `impl Future + use<'a>` which is equivalent to the RAII guard. This means, that sync pattern of taking a closure cannot be used - everything is transformed into RAII guard by the compiler.

Various OS or C/C++ APIs cannot be made `async` without performance or ergonomics costs, because futures become a proxy RAII guard for those APIs. PPYP can work for `Drain<'a>`, but not for `io_uring`. As long as the future is `'static` or directly owns all data it is accessing, `Pin` guarantees are sufficient. Otherwise, there is no way to make a sound API. Currently, they are forced into using `'static` bounds, which is one of the pain points users are reporting about `async` Rust, together with `Send` issues.

Let's try to translate the previous example, a widely used pattern, to `async` Rust.

```rust
async fn something_with_clean_up(f: impl AsyncFnOnce(Foo)) {
    // setup
    f(Foo).await;
    // cleanup
}

async fn main() {
    something_with_clean_up(async |foo| {
        foo.bar().await;
    }).await;

    // rest of the code...
}
```

In this code snippet we added `async` modifiers to our functions, as well as `await`. You may think that cleanup will be done, but it is not guaranteed. All `async` calls are turned into structs - like the RAII guards we talked about earlier:

```rust
async fn something_with_clean_up(f: impl AsyncFnOnce(Foo)) {
    // setup
    f(Foo).await;
    // cleanup
}

async fn main() {
    let fut = something_with_clean_up(async |foo| {
        foo.bar().await;
    });
    {
        // Pin the future.
        let pinned = Box::pin(fut);
        // Poll the future once.
        poll_fn(|cx| Poll::Ready(_ = pinned.poll(cx))).await;
        forget(pinned); // or `_ = Box::leak(pinned);`
    }
    // rest of the code...
}
```

The library is only taking control flow in between `await` points. Here, future is pinned and `Pin`'s [drop guarantee] is met (boxed future remains allocated for `'static`), but cleanup cannot run - `Drop` handler of `fut` is skipped. Thus, APIs that require any cleanup for safety can be expressed in `sync` Rust, but not in `async` Rust, making `async` less attractive, as the operating system APIs and C/C++ libraries *cannot* be used efficiently, ergonomically, and safely.

[drop guarantee]: https://doc.rust-lang.org/std/pin/#drop-guarantee

Another important observation that we can make is that `Pin`'s drop guarantee only applies to the memory of the `Future` itself. But if `Future` borrows a buffer, that buffer *can* be deallocated or re-used before the `drop` of the `Future` is called. See [#connection-to-pin](#connection-to-pin).

## Examples of unsafe async APIs that can be allowed in sync Rust
[example-safe-sync-unsafe-async]: #example-safe-sync-unsafe-async

### Async spawn
[example-async-spawn]: #example-async-spawn

Example from the ecosystem: [spawn_unchecked][spawn_unchecked-example-doc]

[spawn_unchecked-example-doc]: https://docs.rs/async-task/latest/async_task/fn.spawn_unchecked.html.

With the `Forget` trait we can make that API safe:

```rust
struct TaskHandler<'a>(u64, PhantomNonForget, PhantomData<&'a ()>);
// Or `struct TaskHandler<'a>(u64, PhantomNonForget<&'a ()>)`;

impl Drop for TaskHandler<'_> {
    fn drop(&mut self) {
        if let Some(mut mutex) = GLOBAL.get(self.0) {
            // We can block in async context as this mutex is held
            // during the `poll` which should return in a timely manner.
            let fut = mutex.lock();
            // cancel the future and call its drop handler
            drop(fut.take())
        }
    }
}

// Note that this is basically equivalent to async `scope`, as async `scope`
// would be transformed into the `Future` struct, just like `TaskHandler`.
fn spawn<'a>(fut: impl IntoFuture + 'a) -> TaskHandler<'a> {
    GLOBAL.spawn(fut)
}
```

### Async DMA
[example-async-dma]: #example-async-dma

DMA stands for Direct Memory Access, which is used to transfer data between two memory locations in parallel to the operation of the core processor. For the purposes of this example, it can be thought of as `memcpy` in parallel to any other code.

Let's say that the poll of `Serial::read_exact` triggers a DMA transfer. It would be safe if we were to block on this future (basically passing control flow to the future itself), but we may instead trigger undefined behavior with `forget`:

```rust
fn start(serial: &mut Serial) {
    let mut buf = [0; 16];

    let mut fut = Box::pin(serial.read_exact(&mut buf));

    fut.poll(cx); // start dma transfer

    // Memory of the future itself is still valid (inside the allocation),
    // but the buffer lives outside of it and is not protected.
    core::mem::forget(fut); // or `Box::leak`
}

fn corrupted() {
    let mut x = 0;
    let y = 0;

    // do stuff with `x` and `y`
}

start(&mut serial);
// `DMA` keeps writing to `buf`, which is on the stack. `x` and `y` live on the stack too,
// so they will be corrupted.
corrupted();
```

### GPU
[example-async-cuda]: #example-async-cuda

[`async-cuda`], an ergonomic library for interacting with the GPU asynchronously. GPU is just another I/O device (from the point of view of the program), the async model fits surprisingly well. But, this library enforces `!Forget` via documentation requirements.

[`async-cuda`]: https://crates.io/crates/async-cuda

> Internally, the `Future` type in this crate schedules a CUDA call on a separate runtime thread. To make the API as ergonomic as possible, the lifetime bounds of the closure (that is sent to the runtime) are tied to the future object. To enforce this bound, the future will block and wait if it is dropped. This mechanism relies on the future being driven to completion, and not forgotten. This is not necessarily guaranteed. Unsafety may arise if either the runtime gives up on or forgets the future, or the caller manually polls the future, and then forgets it.

### `io_uring`
[example-async-io_uring]: #example-async-io_uring

`io_uring` is another API that needs `!Forget` to function properly. There are attempts at making safe wrappers like [`ringbahn`], which introduces an internal buffer, or [`tokio_uring`], that requires passing ownership of the target buffer.

[`rio`] took an approach like `async-cuda`, implicitly making its futures `!Forget` via documentation.

> `rio` aims to leverage Rust's compile-time checks to be misuse-resistant compared to io_uring interfaces in other languages, but users should beware that use-after-free bugs are still possible without `unsafe` when using `rio`. `Completion` borrows the buffers involved in a request and its destructor blocks to delay the freeing of those buffers until the corresponding request has been completed, but it is considered safe in Rust for an object's lifetime and borrows to end without its destructor running, and this can happen in various ways, including through `std::mem::forget`. Be careful not to let completions leak in this way, and if Rust's soundness guarantees are important to you, you may want to avoid this crate.

[`ringbahn`]: https://github.com/ringbahn/ringbahn/
[`tokio_uring`]: https://docs.rs/tokio-uring/latest/tokio_uring/
[`rio`]: https://lib.rs/crates/rio/

### WASI 0.3

WASI 0.3 has an [uring-like design], and the lack of guaranteed destructors means that for the Rust bindings we have to choose between different options, none of which are great. Thanks @yoshuawuyts for bringing that up!

[uring-like design]: https://github.com/WebAssembly/component-model/issues/471

### `take_mut`

The async version of [`take_mut`] cannot be created as it relies on cleanup code to abort the program.

[`take_mut`]: https://docs.rs/take_mut/latest/take_mut/

### Performance

As we saw earlier, `async` code is forced into `'static` bounds on any non-trivial task such as spawning or sending messages between tasks. That way, references cannot be used, and users must fall back into `Arc` or owned types. `Arc` will [ping-pong] cache line with the counter between the cores, while owned types enforce unnecessary allocations and clones. Example would be a [rumqttc `publish`] which takes `topic` as `Into<String>`. Why? Because it sends this topic to another task. If `!Forget` types were available, a better API choice would be to make the `Future` returned by `publish` be `!Forget` and wait until another task formats the `topic` into the output buffer and reports either success or failure of the publish.

[ping-pong]: https://assets.bitbashing.io/papers/concurrency-primer.pdf
[rumqttc `publish`]: https://docs.rs/rumqttc/latest/rumqttc/struct.Client.html#method.publish

### C/C++ bindings + async do not work well together
[example-async-c-cpp-bindings]: #example-async-c-cpp-bindings

It is common for C/C++ APIs to require some cleanup. It is not an issue for `sync` rust, as wrappers can just take a closure/callback and ensure that cleanup. But all `async` calls are transformed into `impl Future + use<'a>`, not passing control flow to the wrapper. `io_uring`, WASI 0.3 and `async-cuda` fall into that category too. By not having `!Forget` types Rust clearly lags behind current moments towards efficient asynchronous APIs. For embedded/kernel development this issue is even worse, as you often cannot afford an allocation due to the lack of resources or complex locking, making borrows your only option and making `Pin`'s drop guarantee not useful for you.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The core goal of `Forget` trait, as proposed in that RFC, is to bring back the "Proxy Guard" idiom for non-static types, with `async` being the primary motivation.

If any resources are borrowed by some type `T: !Forget`, they will remain borrowed until `T` is dropped.

```rust
let mut resource = [0u8; 1024];

let borrower: Borrower<'_> = Borrower::new(&mut resource);

// Violation of the unsafe contract - `resource` is no longer borrowed,
// so repurposing protected memory is safe.
unsafe { std::mem::forget_unchecked(borrower) };

let first_byte = resource[0]; // Potential UB
```

## Relation between `Forget` and `Pin`
[connection-to-pin]: #connection-to-pin

Both `Forget` and `Pin` concepts serve a similar purpose - guaranteeing that some memory is not moved or repurposed. How `Forget` does it? If any resource is borrowed, you cannot take `&mut` reference to it, as it would be aliased by `!Forget` type that is borrowing from it. Before `!Forget` type goes out of scope, removing the borrow, its drop handler must be executed, just like `Pin`'s [drop guarantee]. So `!Unpin` protects directly owned memory, while `!Forget` protects *borrowed* memory. It is important to note that `Forget` is not defined around memory, but around values - see [#reference-level-explanation](#reference-level-explanation).

With `Forget`, some authors may have the option of borrowing data rather than owning it, making their futures `Unpin`, but `!Forget`.

It is possible that in the future we may teach `Pin` in terms of `Forget`, because new rustaceans will already be familiar with the borrow checker, which is enough to grasp `Forget` and how they pin other values using borrows.

## Undefined Behavior without `!Forget`

Consider that example

```rust
fn spawn<F: IntoFuture>(fut: F) -> JoinHandle<F> {
    // store the future in global storage
}

fn main() {
    let mut buf = [0u8; 64];
    let fut = async {
        let mut i = 0;
        loop {
            // `&mut` to `buf`.
            buf[i] = i;
            i = (i + 1) % 64;
            yield_now().await;
        }
    };

    let handle = spawn(fut);
    std::mem::forget(handle);

    // `fut` might still be running in the background, but `buf` is no longer protected by the borrow checker.

    // Undefined Behavior - aliasing a mutable reference.
    let fourth = buf[4];
}
```

In this case, `handle` borrows from `buf`, but the code that accessing `buf` is not directly tied to `handle`, it runs independently of it. Because of this, even if we pin `handle`, we still can *remove the borrow* (by ending the lifetime of `handle`) on `buf` while `JoinHandle`'s memory remains available (`forget(Box::pin(handle))`). Thus, `Pin` guarantees are not enough, we need `!Forget`.

Functions having signatures with weakening can remove a type from the scope without running its destructor. The following function is an example of a weakening function - after it is called, the borrow checker assumes that the lifetime of `T` has ended, as well as all borrows held by `T`.

```rust
fn weakener<T>(foo: T) -> i32 {
    std::mem::forget(T);
    0
}
```

## How API of channels needs to change in order to work with `!Forget` types.
[channels-unsoundness]: #channels-unsoundness

Currently, channels are created via `let (tx, rx) = channel()`. This is not compatible with `!Forget` types. This section explains how developers should use them instead and why.

There exists a way to exploit the old `thread::scoped` API without any memory leaks[^no_leaks_sidenode]. We can move `JoinHandle` inside the thread it is meant to protect, thereby creating a cyclic relationship:

[^no_leaks_sidenode]: This would become a memory leak if instead of spawning the thread we will just return the closure as the handle, but without it's type mentioned to prevent cycle errors. See later.

```rust
use std::{
    hint::black_box,
    marker::PhantomData,
    sync::{Arc, Mutex},
};

struct JoinHandle<'a>(PhantomData<&'a ()>);

impl Drop for JoinHandle<'_> {
    fn drop(&mut self) {}
}

fn scoped<'a, F>(_f: F) -> JoinHandle<'a>
where
    F: FnOnce() -> (),
    F: Send + 'a,
{
    todo!("schedule `F` on the actual thread")
}

fn main() {
    let arc1 = Arc::new(Mutex::new(None));
    let arc2 = arc1.clone();

    let mut buf = [0; 1024];
    let buf_ref = &mut buf;

    let handle = scoped(move || {
        let _handle = arc2.lock().unwrap().take();
        for _ in 0..100000 {
            black_box(&mut *buf_ref);
        }
        drop(arc2);
    });

    arc1.lock().unwrap().replace(handle);
    drop(arc1);

    // aliased `&mut`
    buf[0] = 1;
}
```

This code is clearly unsound because we are aliasing a mutable reference, which permits potential data races and use-after-free issues. Furthermore, many types of channels - including rendezvous channels - can be vulnerable to this issue if their signatures allow an equivalent implementation using reference counting.

```rust
fn main() {
    let (tx, rx) = std::sync::mpsc::channel();

    let mut buf = [0; 1024];
    let buf_ref = &mut buf;

    let handle = scoped(move || {
        let _handle = rx.recv().unwrap();
        for _ in 0..100000 {
            black_box(&mut *buf_ref);
        }
        drop(_handle);
    });

    tx.send(handle);
    drop(tx);

    buf[0] = 1;
}
```

What is the actual problem?

### The reason of the channels unsoundness
[channels-unsoundness-reason]: #channels-unsoundness-reason

The core issue is not inherent to `scoped` or `JoinHandle` per se - it lies in the API design and its interaction with `!Forget` types. From the type system's perspective, `scoped` is consuming `F` and returning another type with some lifetime. This erasure plays a critical role to avoid a cyclic type that will not compile. It creates a pathway for unsoundness when combined with signatures resembling reference-counted types like `Arc`.

```rust
trait Erase { }
impl<T> Erase for T {}

struct JoinHandle<'a>(Box<dyn Erase + Send + 'a>);

impl Drop for JoinHandle<'_> {
    fn drop(&mut self) {}
}

fn scoped<'a, F>(f: F) -> JoinHandle<'a>
where
    F: FnOnce() -> (),
    F: Send + 'a,
{
    JoinHandle(Box::new(f))
}

fn main() {
    let (tx, rx) = std::sync::mpsc::channel();

    let mut buf = [0; 1024];
    let buf_ref = &mut buf;

    let handle = scoped(move || {
        buf_ref[0] = 1;
        drop(rx);
    });

    _ = tx.send(handle);
    drop(tx);

    // handle no longer guards `buf`.

    // "aliased" `&mut`, but it is not UB in that case, because `f` in not running.
    buf[0] = 1;
}
```

`JoinHandle` and `scoped` have exactly the same signature as before, but use basic language primitives under the hood. The signature of `scoped` can be summarized as `F + 'a -> 'a` - it erased the concrete type `F` and returned just `JoinHandle<'a>`. `Box<dyn Trait>` is a core language feature, we can't remove it for `!Forget` types, as it would render them unusable.

We will call APis that split ownership of the allocation between `tx` and `rx` and allow writes `Arc`-like. `Box<dyn Trait>` can cause `Arc`-like APIs to leak, because we can erase the type of `rx` and place it in the shared allocation using `tx`, keeping it alive indefinitely because `rx` is inside.

*Any* `Arc`-like API is capable of causing leaks, without any `unsafe` code on the `scoped` side. This demonstrates that approaches such as making `JoinHandle: !Send` are not feasible. How can we fix it?

### Solution for message passing of `!Forget` types.
[solution-to-self-referential-problem]: #solution-to-self-referential-problem

Looking at our first example with `Arc`, let's replace our `Arc` with a reference:

```rust
fn main() {
    let mutex = Mutex::new(None);
    let mutex_ref = &mutex;

    let mut buf = [0; 1024];
    let buf_ref = &mut buf;

    let handle = scoped(move || {
        let _handle = mutex_ref.lock().unwrap().take();
        for _ in 0..100000 {
            black_box(&mut *buf_ref);
        }
    });

    mutex.lock().unwrap().replace(handle);
    drop(mutex);

    buf[0] = 1;
}
```

This change results in compiler errors that prevent the unsound behavior:

```rust
 error[E0597]: `mutex` does not live long enough
  --> src/main.rs:23:21
   |
22 |     let mutex = Mutex::new(None);
   |         ----- binding `mutex` declared here
23 |     let mutex_ref = &mutex;
   |                     ^^^^^^ borrowed value does not live long enough
...
40 | }
   | -
   | |
   | `mutex` dropped here while still borrowed
   | borrow might be used here, when `mutex` is dropped and runs the destructor for type `Mutex<Option<JoinHandle<'_>>>`

error[E0505]: cannot move out of `mutex` because it is borrowed
  --> src/main.rs:37:10
   |
22 |     let mutex = Mutex::new(None);
   |         ----- binding `mutex` declared here
23 |     let mutex_ref = &mutex;
   |                     ------ borrow of `mutex` occurs here
...
37 |     drop(mutex);
   |          ^^^^^
   |          |
   |          move out of `mutex` occurs here
   |          borrow later used here
```

Here, a single step to replace `Arc<T>` with `&'a T` allowed code to become sound - `JoinHandle: !Forget` allowes to pass references into tasks and removes the need for the reference counting in that case - the allocation is not *owned* by `tx` and `rx`, they are *borrowing* from it, making them not `Arc`-like. The same approach applies to channels. This approach is not compatible with `JoinHandle: Forget` and cannot be used today with functions like `tokio::spawn` due to `'static` bound, but ecosystem has some examples:

```rust
fn main() {
    let mut queue = heapless::spsc::Queue::<_, 2>::new();
    let (mut tx, mut rx) = queue.split();

    let mut buf = [0; 1024];
    let buf_ref = &mut buf;

    let handle = scoped(move || {
        let _handle = rx.dequeue();
        for _ in 0..100000 {
            black_box(&mut *buf_ref);
        }
    });

    // Moving `handle` into `queue`, causing a self-referential borrow (`handle` -> `rx` -> `queue` -> `handle`).
    tx.enqueue(handle);
    drop(tx);

    buf[0] = 1;
}
```

This code fails to compile, which prevents the unsound behavior. The failure occurs because the borrow checker detects a self-reference: `handle` borrows `queue`, but moving `handle` into `queue` causes `queue` to indirectly borrow itself. Since the compiler inserts a call to `drop` on `queue`, this self-referential borrow is caught at compile time. But `Arc` is *designed* to remove the lifetime, removing borrow-checker's ability to prevent loops, self-references and leaks.

Thus, to support message passing with `!Forget` types, API authors must rely more heavily on lifetimes. Since `Forget` types inherently involve lifetime management, using explicit lifetimes (for example, by replacing `Arc<T>` with `&'a T`, having `PhantomData` together with a pointer etc) prevents the formation of cycles that can lead to unsoundness. While this approach is not compatible with APIs that require a `'static` bound (such as `tokio::spawn`), it works in environments like `thread::scope` or async scopes, where the future itself can be `!Forget`. Notably, rendezvous channels can be soundly expressed using this API alongside `PhantomData`.

## Traditional combinators and patterns
[traditional-workflows]: #traditional-workflows

Async combinators with `join`, `race`, or `merge` semantics will continue to work as they do. If some future passed into them is `!Forget`, their future becomes `!Forget` too. `Arc` cannot be used with `!Forget` types, but the need for `Arc`, [which is quite a pain point][ergonomic-refcounting], will decrease, as users will be able to spawn with references directly.

[ergonomic-refcounting]: https://github.com/rust-lang/rfcs/pull/3680

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This new auto trait is added to the `core::marker` and `std::marker` modules:

```rust
pub unsafe auto trait Forget { }
```

Let `T` be `T: !Forget` and `value` be a value borrowed by value of type `T`. Unsafe code is given the following guarantees:

- If `value` is borrowed by `T` as `&mut`, `value` cannot be moved/invalidated/borrowed until `T` is dropped.
- If `value` is borrowed by `T` as `&`, `value` cannot be moved/invalidated/exclusively borrowed until `T` is dropped.

In practice, we disallow skipping the destructor of `!Forget` types before they exit the scope. Violation is not an immediate undefined behavior, but other code can rely on the destructor running, which can lead to undefined behavior down the road. Unsafe code authors can freely violate this rule, if responsibility is taken.

Several observations can be made about this guarantee. For `T: 'static` we don't have to run the destructor to fulfill it, as `T: 'static` can only have `'static` borrows, which are assumed to be valid indefinite borrows (like with [`Pin::static_ref`]). Another one is, memory borrowed by  `T: !Forget` type cannot be reused or invalidated, as safe code needs to move/take a reference to `value`, similar to the [drop guarantee]. `value` cannot be dropped, as it requires moving.

```rust
struct Foo<T>(PhantomNonForget, T);
struct Baz;

impl<T> Drop for Foo<T> { fn drop(&mut self) { } }

let ref_buf = [0u8; 64];
let mut_buf = [0u8; 64];

// We have a guarantee, that no `&mut` can be taken to `ref_buf` until `Foo`'s `drop`.
let foo_ref = Foo(PhantomNonForget, &ref_buf);
// We have a guarantee, that no `&/&mut` can be taken to `ref_buf` until `Foo`'s `drop`.
let foo_mut = Foo(PhantomNonForget, &mut ref_buf);

drop(ref_buf); // error[E0505]: cannot move out of `ref_buf` because it is borrowed
drop(mut_buf); // error[E0505]: cannot move out of `mut_buf` because it is borrowed

let ref_first_byte = ref_buf.0[0]; // Allowed
let mut_first_byte = mut_buf.0[0]; // error[E0503]: cannot use `mut_buf.0[_]` because it was mutably borrowed

drop(foo_ref);
drop(foo_mut);

let mut_first_byte = mut_buf.0[0]; // Allowed
drop(ref_buf); // Allowed
drop(mut_buf); // Allowed

// `Baz` cannot be moved or exclusively borrowed until `Foo` is dropped.
fn phantom<'a>(baz: &'a Baz) -> Foo<PhantomData<&'a ()>> {
    Foo(PhantomNonForget, PhantomData)
}
```

[`Pin::static_ref`]: https://doc.rust-lang.org/std/pin/struct.Pin.html#method.static_ref

Type becomes `!Forget` if it directly contains `!Forget` member.

We should either allow `!Forget` types in statics or make all `'static` types `Forget` because it fulfills the unsafe guarantee and we can't enforce any code running before the program exits or aborts.

```rust
let mut resource = [0u8; 64];
let _unforget = Subsystem::execute(&mut resource);
std::process::abort(); // `resource` is (forcefully) borrowed for `'static`
resource[0] = 42; // unreachable
```

## Standard Library
[std]: #std

All APIs in the standard library should be migrated at once. With available migration strategies, there is no benefit in gradual migration, it will greatly reduce the productivity of rustc developers by adding boilerplate and noise into the codebase. An audit must be performed to ensure which APIs must remain `Forget`. See [#ecosystem-migration](#ecosystem-migration) for more details.

All existing non-generic types in std will continue to be `Forget`.

## `Copy`
[copy]: #copy

All types that implement `Copy` must implement `Forget` too.

## Unions
[unions]: #unions

Unions are always `Forget`. All members of `union` must be `Forget`, but it is already covered by other rules and does not need to be enforced.

## API changes
[library-api-changes]: #library-api-changes

- `Rc`/`Arc` - all APIs for construction,  except the new `Rc::new_unchecked` method, only exist for `T: Forget` types. If we decide to not have `impl<T: 'static> Forget for T {}`, in the future we *may* allow safe constructors for `T: ?Forget + 'static` (resources are borrowed for `'static`, it fulfills the guarantee we are giving to the unsafe code) and something along the lines of `T: ?Forget + Freeze` (to forbid cycles), author of the RFC is not familiar enough with interior mutability questions.
- `ManuallyDrop<T>` always implements `Forget`, regardless of the `T`. `ManuallyDrop::new` is available for types with `T: Forget`.  New unsafe method `ManuallyDrop::new_unchecked`, available for `T: ?Forget`, is introduced. We may add a safe constructor with `T: ?Forget + 'static`, as we allow forgetting in statics.
- `Box::<T>::into_ptr` is available only for `T: Forget`. As for `T: !Forget` users should `ManuallyDrop::new_unchecked` and take the pointer via `&raw mut`. It will still be allowed to pass this pointer to `Box::from_ptr`.
- `Box::<T>::leak` is available only for `T: Forget`.
- `forget_unchecked`, a new unsafe function, is added to forget `T: ?Forget` types. It is a wrapper around `ManuallyDrop::new_unchecked`, just as `forget` is a wrapper around `ManuallyDrop::new`.
- `PhantomNonForget` is a `!Forget` ZST for types to become `!Forget`. If we decide to have `impl<T: 'static> Forget for T {}`, we should add a generic parameter/lifetime to `PhantomNonForget`.
- Similar to other APIs, `T: Forget` -> `Drait<T>: Forget` and `T: !Forget` -> `Drait<T>: !Forget`.
- APIs like `std::sync::mpsc::Sender::send` are available only for `T: Forget`.
- Possibly new channels should be introduced, that are compatible with `T: !Forget` types too.
- `Vec::set_len` is available only for `T: Forget` types, to not create a footgun for `unsafe` code in the wild. Maybe a new method should be added to support `T: ?Forget`.
- `ptr::write` is available for all types.
- Etc

## Ecosystem Migration
[ecosystem-migration]: #ecosystem-migration

### Migration using [`local_default_bounds`] RFC
[local-defaults-migration]: #local-defaults-migration

The local_default_bounds RFC facilitates a smoother migration without requiring an edition change. In essence, it allows users to override default bounds on generics and associated types, such as change from `Forget` to `?Forget`. The process is comparable to the adoption of `const fn`, which is already accepted and loved feature, that keeps expanding and does not cause ecosystem splitting.

We will provide an opt-in mechanism for crates to modify default bounds in function signatures.

```rust
// Crate that has migrated
mod migrated {
    #![default_generic_bounds(?Forget)]

    fn foo<T>(value: T) { /* ... */ } // T: ?Forget
}

// Crate that has not migrated
mod not_migrated {
    fn foo<T>(value: T) { /* ... */ } // T: Forget
}
```

In the context of the `local_default_bounds` RFC, along with introducing the `Forget` trait, Bounds for  `Self` and associated types should default to `?Forget` rather than `Forget`. This change is not observable for code that does not explicitly opt into using `Forget`, as `default_generic_bounds` will continue to default to `Forget`. A more detailed explanation will follow later.

As discussed in [#semver-and-ecosystem](#semver-and-ecosystem), libraries adopting `?Forget` in their signatures will, at most, require a minor semver change. Consequently, migrating to `?Forget` would be equivalent to the now stable `const fn` feature. Libraries have already been adopting `const fn` without causing ecosystem fragmentation, as pull requests continue to be merged, progressively making more functions `const`.

#### Not interested in migration crates
[no-local-defaults-migration]: #no-local-defaults-migration

Some crates may refuse to migrate due to being unmaintained, the only difference is that for downstream crates their signatures would be filled with `T: Forget`. This is only natural, as those crates were written with that assumption as if they manually put `T: Forget` on their signatures. Some automatic methods to determine that function can accept `Forget` types are not feasible. We are already not doing it for `const fn`, it would be a semver hazard and only safe code can touch `T`, as analysing `unsafe` code is against the design of the language.

If the crate is maintained, however, migration should not be difficult.

#### `#![forbid(unsafe)]` crates
[safe-local-defaults-migration]: #safe-local-defaults-migration

1. Set the appropriate bounds:

```rust
// can be with `cfg_attr`
#![default_generic_bounds(?Forget)]
```

2. Resolve any compilation errors by explicitly adding `+ Forget` where needed.

3. Optionally: Recurse into your dependencies, applying the same changes as needed. Most probably you will use `!Forget` types with well-maintained crates providing combinators or containers.

#### For crates with `unsafe` code (like `libcore`)
[unsafe-local-defaults-migration]: #unsafe-local-defaults-migration

1. Set the appropriate bounds:

```rust
#![default_generic_bounds(?Forget)]
```

2. Audit your codebase to work properly with `!Forget` types.

3. Resolve any compilation errors by explicitly adding `+ Forget` where needed.

4. Optionally: Recurse into your dependencies, applying the same changes as needed.

#### Semver and compatibility, ecosystem splitting
[semver-and-ecosystem]: #semver-and-ecosystem

This approach is targeted at minimizing problems between different crates in the ecosystem. For any library, opting into using `Forget` and accepting those types will be a minor semver change.

Earlier it was stated that Bounds for `Self` and associated types should default to `?Forget` instead of `Forget`. This is due to an important case. If a user of the library updated earlier than the library, then without that change it will observe that associated types of traits are `Forget`, so it would be a breaking change for the library to lift that constraint in the future. But now, the user will observe `?Forget`, thus it cannot rely on them being `Forget`. But for users that did not migrate, as well as the library itself, it will not be observable due to `default_generic_bounds` still being `Forget`.

```rust
// This indicates that user explicity opted in
#![default_generic_bounds(?Forget)]

// After opting in, user needs to add `T::baz(..): Forget` to silence the error - quite easy.
async fn foo<T: other_crate::Trait>(bar: T) {
    let fut = bar.baz();
    // Compiler will emit an error, as `fut` maybe `!Forget`, because we set `default_generic_bounds`
    // to `?Forget`, and default for associated types in `other_crate` is already `?Forget`. Otherwise it
    // would have been a breaking change for `other_crate` to make future provided by `baz` `!Forget`,
    // as this code would've compiled now but not in the future.
    core::mem::forget(fut);
}

// A library that did not migrate yet. `Trait::bar(..)` is not locked into `Forget`, but
// this `other_crate` and other crates can only observe `Trait::bar(..): Forget` cases.
mod other_crate {
    trait Trait {
        async fn baz();
    }
}
```

#### Macros

If macro-library generates code, some problems during the migration are possible:

```rust
mod user {
    #![default_generic_bounds(?Forget)]

    ::library::make!(); // Will not compile because `T` is `?Forget`.
}

mod user {
    #[macro_export]
    macro_rules! make {
        () => {
            pub fn foo<T>(t: T) {
                ::core::mem::forget(t);
            }
        }
    }
}
```

#### Changing default

It is not required, but in next editions we may swap the default for `default_generic_bounds`. Crates that want to continue using old default in next editions will set `#![default_generic_bounds(Forget)]`.

### Migration over the edition with [default auto traits]
[edition-migration-with-mask]: #edition-migration-with-mask

If [`local_default_bounds`] is not accepted, we can make a satisfactory migration by having editions <= 2024 have `Forget` as the default, and editions after 2024 have `?Forget` as the default.

While it will not split the ecosystem, it will require everyone to make a migration just as in the [`local_default_bounds`] solution. There are concern about locking crates into `Forget` bounds on associated types in traits (like `async fn`). Migration can probably be automated for crates without unsafe code.

[default auto traits]: https://github.com/rust-lang/rust/pull/120706

# Drawbacks
[drawbacks]: #drawbacks

## Migration
[drawbacks-migration]: #drawbacks-migration

If [`local_default_bounds`] is accepted, migration would be practically seamless, as described in [#migration](#migration). Even if it's not accepted, less seamless but still acceptable solution would be a change over edition.

## Message Passing
[drawbacks-message-passing]: #drawbacks-message-passing

A traditional approach to the message-passing cannot be applied to `!Forget` types - slightly different APIs should be developed, preserving a
lifetime connection between `tx` and `rx` handles.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

All types were assumed to be `!Forget` in Rust's early days, and then it was changed in hurry. They flow naturally out of Rust's type system, do not clash with any preexisting concepts that do not directly involve forgetting and are used pleasantly and intuitively, modulo migration. With the `async` built around `Future` trait it became apparent that language directly lacks this feature. Being simple and non-disturbing, it's hard to find something that would fit that purpose better.

We can do nothing, but use cases just keep piling up.

The author of https://zetanumbers.github.io/book/myosotis.html is working on another approach to that problem, but it is not public yet.

It is also possible to preserve current `Arc`-like channels by disabling the type ereasure for `!Forget` types - disallow the signatures like `F + 'a -> 'a`. This way, `JoinHandle` would be generic over the closure, not a lifetime: `JoinHandle<F>`. That way, we will trigger the infinitely recursive type error instead of a borrow-checker's error. But this will be a major downside to `!Forget` types, making them barely usable.

# Prior Art
[prior-art]: #prior-art

- https://github.com/rust-lang/rfcs/pull/1084#issuecomment-96875651
- https://github.com/rust-lang/rfcs/pull/1094
- https://internals.rust-lang.org/t/forgetting-futures-with-borrowed-data/10824
- https://github.com/aturon/rfcs/blob/scoped-take-2/text/0000-scoped-take-2.md
- https://without.boats/blog/the-scoped-task-trilemma
- https://without.boats/blog/asynchronous-clean-up/
- https://zetanumbers.github.io/book/myosotis.html: an independent exploration of the same problem space with similar, but subtly different, conclusions.
- https://hackmd.io/@wg-async/S1Q6Leam0: a design meeting regarding the previous post
- https://blog.yoshuawuyts.com/linear-types-one-pager/

## Leakpocalypse
[leakpocalypse-prior-art]: #leakpocalypse-prior-art

- https://github.com/rust-lang/rfcs/pull/1066
- https://github.com/rust-lang/rust/issues/24292
- https://cglab.ca/~abeinges/blah/everyone-poops/
- https://github.com/rust-lang/rfcs/pull/1085
- https://doc.rust-lang.org/std/thread/fn.scope.html
- https://smallcultfollowing.com/babysteps/blog/2015/04/29/on-reference-counting-and-leaks/

## Usage of the pattern
[usage-prior-art]: #usage-prior-art

- https://doc.rust-lang.org/std/thread/struct.Builder.html#method.spawn_unchecked
- https://docs.rs/async-task/latest/async_task/fn.spawn_unchecked.html
- https://blog.japaric.io/safe-dma/
- https://docs.rs/async_nursery/latest/async_nursery/
- https://without.boats/blog/the-scoped-task-trilemma/
- https://docs.rs/async-scoped/latest/async_scoped/struct.Scope.html#method.scope_and_collect

## Miscellaneous
[misc-prior-art]: #misc-prior-art

- https://github.com/rust-lang/rfcs/issues/1111
- https://tmandry.gitlab.io/blog/posts/2023-03-01-scoped-tasks/

## MustMove types
[must-move-prior-art]: #must-move-prior-art

- https://faultlore.com/blah/linear-rust/
- https://smallcultfollowing.com/babysteps/blog/2023/03/16/must-move-types/
- https://blog.yoshuawuyts.com/linearity-and-control/

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- [ ] Maybe the name `Forget` is misleading, as its core is around the `unsafe` guarantee of borrowed resources and the destructor?
- [ ] Maybe force `impl Forget for T where T: 'static {}` and add a generic to the `PhantomNonForget`? Use cases and unsafe guarantee are fine with it, and we already allow `!Forget` in `static`.
- [ ] Maybe add `StaticForget<T: ?Forget + 'static>: Forget`.
- [ ] How does it interact with `&own`?
- [ ] Maybe make `Vec::set_len` available for `T: ?Forget`, but with a new unsafe precondition. Crates with unsafe code that are manually migrating to support `!Forget` would need to be aware of that change and verify/modify their unsafe code to work correctly with `!Forget` types, or manually restrain them to `Forget`.
- [ ] Which approach to migration should be followed?
- [ ] How should it interact with `async Drop`?

# Future possibilities
[future-possibilities]: #future-possibilities

This RFC will allow `async` Rust to come closer to the sync ergonomics, but some code will not be able to reach this end goal and insert "abort bombs" into mandatory destructors. This is strictly better than today's status quo: `unsafe` in application code - you can work with it, but this defies the whole point of Rust. A more robust approach would be the `Linear`/`MustMove`/`!Drop` types. This RFC makes a step towards more liveness guarantees, making them closer. As for the biggest problem - unwinding - with `async`, we have more choice over our behavior during unwinds. Even if we do not succeed with effects forbidding unwinding, the future containing linear type may catch any unwind during the poll and return `Poll::Pending`, potentially recovering - `async Drop` looks promising too.

Maybe if `!Forget` type borrows itself, it would be equivalent to the pinning?

