- Feature Name: scoped_threads
- Start Date: 2019-02-26
- RFC PR: [rust-lang/rfcs#3151](https://github.com/rust-lang/rfcs/pull/3151)
- Rust Issue: [rust-lang/rust#93203](https://github.com/rust-lang/rust/issues/93203)

# Summary
[summary]: #summary

Add scoped threads to the standard library that allow one to spawn threads
borrowing variables from the parent thread.

Example:

```rust
let var = String::from("foo");

thread::scope(|s| {
    s.spawn(|_| println!("borrowed from thread #1: {}", var));
    s.spawn(|_| println!("borrowed from thread #2: {}", var));
});
```

# Motivation
[motivation]: #motivation

Before Rust 1.0 was released, we had
[`thread::scoped()`](https://docs.rs/thread-scoped/1.0.2/thread_scoped/) with the same
purpose as scoped threads, but then discovered it has a soundness issue that
could lead to use-after-frees so it got removed. This historical event is known as
[leakpocalypse](http://cglab.ca/~abeinges/blah/everyone-poops/).

Fortunately, the old scoped threads could be fixed by relying on closures rather than
guards to ensure spawned threads get automatically joined. But we weren't
feeling completely comfortable with including scoped threads in Rust 1.0 so it
was decided they should live in external crates, with the possibility of going
back into the standard library sometime in the future.
Four years have passed since then and the future is now.

Scoped threads in [Crossbeam](https://docs.rs/crossbeam/0.7.1/crossbeam/thread/index.html)
have matured through years of experience and today we have a design that feels solid
enough to be promoted into the standard library.

See the [Rationale and alternatives](#rationale-and-alternatives) section for more.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The "hello world" of thread spawning might look like this:

```rust
let greeting = String::from("Hello world!");

let handle = thread::spawn(move || {
    println!("thread #1 says: {}", greeting);
});

handle.join().unwrap();
```

Now let's try spawning two threads that use the same greeting.
Unfortunately, we'll have to clone it because
[`thread::spawn()`](https://doc.rust-lang.org/std/thread/fn.spawn.html)
has the `F: 'static` requirement, meaning threads cannot borrow local variables:

```rust
let greeting = String::from("Hello world!");

let handle1 = thread::spawn({
    let greeting = greeting.clone();
    move || {
        println!("thread #1 says: {}", greeting);
    }
});

let handle2 = thread::spawn(move || {
    println!("thread #2 says: {}", greeting);
});

handle1.join().unwrap();
handle2.join().unwrap();
```

Scoped threads to the rescue! By opening a new `thread::scope()` block,
we can prove to the compiler that all threads spawned within this scope will
also die inside the scope:

```rust
let greeting = String::from("Hello world!");

thread::scope(|s| {
    let handle1 = s.spawn(|_| {
        println!("thread #1 says: {}", greeting);
    });

    let handle2 = s.spawn(|_| {
        println!("thread #2 says: {}", greeting);
    });

    handle1.join().unwrap();
    handle2.join().unwrap();
});
```

That means variables living outside the scope can be borrowed without any
problems!

Now we don't have to join threads manually anymore because all unjoined threads
will be automatically joined at the end of the scope:

```rust
let greeting = String::from("Hello world!");

thread::scope(|s| {
    s.spawn(|_| {
        println!("thread #1 says: {}", greeting);
    });

    s.spawn(|_| {
        println!("thread #2 says: {}", greeting);
    });
});
```

When taking advantage of automatic joining in this way, note that `thread::scope()`
will panic if any of the automatically joined threads has panicked.

You might've noticed that scoped threads now take a single argument, which is
just another reference to `s`. Since `s` lives inside the scope, we cannot borrow
it directly. Use the passed argument instead to spawn nested threads:

```rust
thread::scope(|s| {
    s.spawn(|s| {
        s.spawn(|_| {
            println!("I belong to the same `thread::scope()` as my parent thread")
        });
    });
});
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

We add two new types to the `std::thread` module:

```rust
struct Scope<'env> {}
struct ScopedJoinHandle<'scope, T> {}
```

Lifetime `'env` represents the environment outside the scope, while
`'scope` represents the scope itself. More precisely, everything
outside the scope outlives `'env` and `'scope` outlives everything
inside the scope. The lifetime relations are:

```
'variables_outside: 'env: 'scope: 'variables_inside
```

Next, we need the `scope()` and `spawn()` functions:

```rust
fn scope<'env, F, T>(f: F) -> T
where
    F: FnOnce(&Scope<'env>) -> T;

impl<'env> Scope<'env> {
    fn spawn<'scope, F, T>(&'scope self, f: F) -> ScopedJoinHandle<'scope, T>
    where
        F: FnOnce(&Scope<'env>) -> T + Send + 'env,
        T: Send + 'env;
}
```

That's the gist of scoped threads, really.

Now we just need two more things to make the API complete. First, `ScopedJoinHandle`
is equivalent to `JoinHandle` but tied to the `'scope` lifetime, so it will have
the same methods. Second, the thread builder needs to be able to spawn threads
inside a scope:

```rust
impl<'scope, T> ScopedJoinHandle<'scope, T> {
    fn join(self) -> Result<T>;
    fn thread(&self) -> &Thread;
}

impl Builder {
    fn spawn_scoped<'scope, 'env, F, T>(
        self,
        &'scope Scope<'env>,
        f: F,
    ) -> io::Result<ScopedJoinHandle<'scope, T>>
    where
        F: FnOnce(&Scope<'env>) -> T + Send + 'env,
        T: Send + 'env;
}
```

# Drawbacks
[drawbacks]: #drawbacks

The main drawback is that scoped threads make the standard library a little bit bigger.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

* Keep scoped threads in external crates.

  There are several advantages to having them in the standard library:

  * This is a very common and useful utility and is great for learning, testing, and exploratory
    programming. Every person learning Rust will at some point encounter interaction
    of borrowing and threads. There's a very important lesson to be taught that threads
    *can* in fact borrow local variables, but the standard library doesn't reflect this.

  * Some might argue we should discourage using threads altogether and point people to
    executors like Rayon and Tokio instead. But still,
    the fact that `thread::spawn()` requires `F: 'static` and there's no way around it
    feels like a missing piece in the standard library.

  * Implementing scoped threads is very tricky to get right so it's good to have a
    reliable solution provided by the standard library.

  * There are many examples in the official documentation and books that could be
    simplified by scoped threads.

  * Scoped threads are typically a better default than `thread::spawn()` because
    they make sure spawned threads are joined and don't get accidentally "leaked".
    This is sometimes a problem in unit tests, where "dangling" threads can accumulate
    if unit tests spawn threads and forget to join them.

  * Users keep asking for scoped threads on IRC and forums
    all the time. Having them as a "blessed" pattern in `std::thread` would be beneficial
    to everyone.

* Return a `Result` from `scope` with all the captured panics.

  * This quickly gets complicated, as multiple threads might have panicked.
    Returning a `Vec` or other collection of panics isn't always the most useful interface,
    and often unnecessary. Explicitly using `.join()` on the `ScopedJoinHandle`s to
    handle panics is the most flexible and efficient way to handle panics, if the user wants
    to handle them.

* Don't pass a `&Scope` argument to the threads.

  * `scope.spawn(|| ..)` rather than `scope.spawn(|scope| ..)` would require the `move` keyword
    (`scope.spawn(move || ..)`) if you want to use the scope inside that closure, which gets unergonomic.


# Prior art
[prior-art]: #prior-art

Crossbeam has had
[scoped threads](https://docs.rs/crossbeam/0.7.1/crossbeam/thread/index.html)
since Rust 1.0.

There are two designs Crossbeam's scoped threads went through. The old one is from
the time `thread::scoped()` got removed and we wanted a sound alternative for the
Rust 1.0 era. The new one is from the last year's big revamp:

* Old: https://docs.rs/crossbeam/0.2.12/crossbeam/fn.scope.html
* New: https://docs.rs/crossbeam/0.7.1/crossbeam/fn.scope.html

There are several differences between old and new scoped threads:

1. `scope()` now propagates unhandled panics from child threads.
    In the old design, panics were silently ignored.
    Users can still handle panics by manually working with `ScopedJoinHandle`s.

2. The closure passed to `Scope::spawn()` now takes a `&Scope<'env>` argument that
   allows one to spawn nested threads, which was not possible with the old design.
   Rayon similarly passes a reference to child tasks.

3. We removed `Scope::defer()` because it is not really useful, had bugs, and had
   non-obvious behavior.

4. `ScopedJoinHandle` got parametrized over `'scope` in order to prevent it from
   escaping the scope.

Rayon also has [scopes](https://docs.rs/rayon/1.0.3/rayon/struct.Scope.html),
but they work on a different abstraction level - Rayon spawns tasks rather than
threads. Its API is the same as the one proposed in this RFC.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

Can this concept be extended to async? Would there be any behavioral or API differences?

# Future possibilities
[future-possibilities]: #future-possibilities

In the future, we could also have a threadpool like Rayon that can spawn
scoped tasks.
