- Feature Name: `thread_spawn_hook`
- Start Date: 2024-05-22
- RFC PR: [rust-lang/rfcs#3642](https://github.com/rust-lang/rfcs/pull/3642)
- Rust Issue: [rust-lang/rust#132951](https://github.com/rust-lang/rust/issues/132951)

# Summary

Add `std::thread::add_spawn_hook` to register a hook that runs for newly spawned threads.
This will effectively provide us with "inheriting thread locals", a much requested feature.

```rust
thread_local! {
    static MY_THREAD_LOCAL: Cell<u32> = Cell::new(0);
}

std::thread::add_spawn_hook(|_| {
    // Get the value of X in the spawning thread.
    let value = MY_THREAD_LOCAL.get();

    // Set the value of X in the newly spawned thread.
    move || MY_THREAD_LOCAL.set(value)
});
```

# Motivation

Thread local variables are often used for scoped "global" state.
For example, a testing framework might store the status or name of the current
unit test in a thread local variable, such that multiple tests can be run in
parallel in the same process.

However, this information will not be preserved across threads when a unit test
will spawn a new thread, which is problematic.

The solution seems to be "inheriting thread locals": thread locals that are
automatically inherited by new threads.

However, adding this property to thread local variables is not easily possible.
Thread locals are initialized lazily. And by the time they are initialized, the
parent thread might have already disappeared, such that there is no value left
to inherit from.
Additionally, even if the parent thread was still alive, there is no way to
access the value in the parent thread without causing race conditions.

Allowing hooks to be run as part of spawning a thread allows precise control
over how thread locals are "inherited".
One could simply `clone()` them, but one could also add additional information
to them, or even add relevant information to some (global) data structure.

For example, not only could a custom testing framework keep track of unit test
state even across spawned threads, but a logging/debugging/tracing library could
keeps track of which thread spawned which thread to provide more useful
information to the user.

# Public Interface

For adding a hook:

```rust
// In std::thread:

/// Registers a function to run for every newly thread spawned.
///
/// The hook is executed in the parent thread, and returns a function
/// that will be executed in the new thread.
///
/// The hook is called with the `Thread` handle for the new thread.
///
/// The hook will only be added for the current thread and is inherited by the threads it spawns.
/// In other words, adding a hook has no effect on already running threads (other than the current
/// thread) and the threads they might spawn in the future.
///
/// The hooks will run in order, starting with the most recently added.
///
/// # Usage
///
/// ```
/// std::thread::add_spawn_hook(|_| {
///     ..; // This will run in the parent (spawning) thread.
///     move || {
///         ..; // This will run it the child (spawned) thread.
///     }
/// });
/// ```
///
/// # Example
///
/// A spawn hook can be used to "inherit" a thread local from the parent thread:
///
/// ```
/// use std::cell::Cell;
///
/// thread_local! {
///     static X: Cell<u32> = Cell::new(0);
/// }
///
/// // This needs to be done once in the main thread before spawning any threads.
/// std::thread::add_spawn_hook(|_| {
///     // Get the value of X in the spawning thread.
///     let value = X.get();
///     // Set the value of X in the newly spawned thread.
///     move || X.set(value)
/// });
///
/// X.set(123);
///
/// std::thread::spawn(|| {
///     assert_eq!(X.get(), 123);
/// }).join().unwrap();
/// ```
pub fn add_spawn_hook<F, G>(hook: F)
where
    F: 'static + Send + Sync + Fn(&Thread) -> G,
    G: 'static + Send + FnOnce();
```

And for opting out when spawning a hook:

```rust
// In std::thread:

impl Builder {
    /// Disables running and inheriting [spawn hooks](add_spawn_hook).
    ///
    /// Use this if the parent thread is in no way relevant for the child thread.
    /// For example, when lazily spawning threads for a thread pool.
    pub fn no_hooks(mut self) -> Builder;
}
```

# Implementation

The implementation is a *thread local* linked list of hooks, which is inherited by newly spawned threads.
This means that adding a hook will only affect the current thread and all (direct and indirect) future child threads of the current thread.
It will not globally affect all already running threads.

Functions that spawn a thread, such as `std::thread::spawn` will eventually call
`spawn_unchecked_`, which will call the hooks in the parent thread, after the
child `Thread` object has been created, but before the child thread has been
spawned. The resulting `FnOnce` objects are stored and passed on to the child
thread afterwards, which will execute them one by one before continuing with its
main function.

# Downsides

- The implementation requires allocation for each hook (to store them in the
  list of hooks), and an allocation each time a hook is spawned
  (to store the resulting closure).

- A library that wants to make use of inheriting thread locals will have to
  register a global hook (e.g. at the start of `main`),
  and will need to keep track of whether its hook has already been added.

- The hooks will not run if threads are spawned through e.g. pthread directly,
  bypassing the Rust standard library.
  (However, this is already the case for output capturing in libtest:
  that does not work across threads when not spawned by libstd.)

# Rationale and alternatives

## Global vs thread local effect

Unlike e.g. libc's `atexit()`, which has a global effect, `add_spawn_hook` has a thread local effect.

This means that adding a hook will only affect the current thread and all (direct and indirect) future child threads of the current thread.
In other words, adding a hook has no effect on already running threads (other than the current thread) and the threads they might spawn in the future.

An alternative could be to have a global set of hooks that affects all newly spawned threads, on any existing and future thread.

Both are relatively easy and efficient to implement (as long as removing hooks
is not an option).

The global behavior was proposed in an earlier version of this RFC,
but the library-api team expressed a preference for exploring a "more local" solution.

Having a "lexicographically local" solution doesn't seem to be possible other than for scoped threads, however,
since threads can outlive their parent thread and then spawn more threads.

A thread local effect (affecting all future child threads) seems to be the most "local" behavior we can achieve here.

## Add but no remove

Having only an `add_spawn_hook` but not a `remove_spawn_hook` keeps things
simple, by not needing a way to identify a specific hook (through a
handle or a name).

If a hook only needs to execute conditionally, one can make use of an
`if` statement.

If no hooks should be executed or inherited, one can use `Builder::no_hooks`.

## Requiring storage on spawning

Because the hooks run on the parent thread first, before the child thread is
spawned, the results of those hooks (the functions to be executed in the child)
need to be stored. This will require heap allocations (although it might be
possible for an optimization to save small objects on the stack up to a certain
size).

An alternative interface that wouldn't require any store is possible, but has
downsides. Such an interface would spawn the child thread *before* running the
hooks, and allow the hooks to execute a closure on the child (before it moves on
to its main function). That looks roughly like this:

```rust
std::thread::add_spawn_hook(|child| {
    // Get the value on the parent thread.
    let value = MY_THREAD_LOCAL.get();
    // Set the value on the child thread.
    child.exec(|| MY_THREAD_LOCAL.set(value));
});
```

This could be implemented without allocations, as the function executed by the
child can now be borrowed from the parent thread.

However, this means that the parent thread will have to block until the child
thread has been spawned, and block for each hook to be finished on both threads,
significantly slowing down thread creation.

Considering that spawning a thread involves several allocations and syscalls,
it doesn't seem very useful to try to minimize an extra allocation when that
comes at a significant cost.

## `impl` vs `dyn` in the signature

An alternative interface could use `dyn` instead of generics, as follows:

```rust
pub fn add_spawn_hook<F, G>(
    hook: Box<dyn Send + Sync + Fn(&Thread) -> Box<dyn FnOnce() + Send>>
);
```

However, this mostly has downsides: it requires the user to write `Box::new` in
a few places, and it prevents us from ever implementing some optimization tricks
to, for example, use a single allocation for multiple hook results.

## A regular function vs some lang feature

Just like `std::panic::set_hook`, `std::thread::add_spawn_hook` is just regular function.

An alternative would be to have some special attribute, like `#[thread_spawn_hook]`,
similar to `#[panic_handler]` in `no_std` programs, or to make use of
a potential future [global registration feature](https://github.com/rust-lang/rust/issues/125119).

While such things might make sense in a `no_std` world, spawning threads (like
panic hooks) is an `std` only feature, where we can use global state and allocations.

The only potential advantage of such an approach might be a small reduction in overhead,
but this potential overhead is insignificant compared to the overall cost of spwaning a thread.

The downsides are plenty, including limitations on what your hook can do and return,
needing a macro or special syntax to register a hook, potential issues with dynamic linking,
additional implementation complexity, and possibly having to block on a language feature.

# Unresolved questions

- Should the return value of the hook be an `Option`, for when the hook does not
  require any code to be run in the child?

- Should the hook be able to access/configure more information about the child
  thread? E.g. set its stack size.
  (Note that settings that can be changed afterwards by the child thread, such as
  the thread name, can already be set by simply setting it as part of the code
  that runs on the child thread.)

# Future possibilities

- Using this in libtest for output capturing (instead of today's
  implementation that has special hardcoded support in libstd).

# Relevant history

- The original reason I wrote [RFC 3184 "Thread local Cell methods"](https://github.com/rust-lang/rfcs/pull/3184)
  was to simplify thread spawn hooks (which I was experimenting with at the time).
  Without that RFC, thread spawn hooks would look something like `let v = X.with(|x| x.get()); || X.with(|x| x.set(v))`, instead of just `let v = X.get(); || X.set(v)`,
  which is far less ergonomic (and behaves subtly differently). This is the reason I waited with this RFC until that RFC was merged and stabilized.
