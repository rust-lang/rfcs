- Feature Name: scoped_threads
- Start Date: 2019-02-26
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

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
})
.unwrap();
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

Scoped threads coming to the rescue! By opening a new `thread::scope()` block,
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
})
.unwrap();
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
})
.unwrap();
```

Note that `thread::scope()` returns a `Result` that will be `Ok` if all
automatically joined threads have successfully completed, i.e. they haven't
panicked.

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
})
.unwrap();
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
fn scope<'env, F, T>(f: F) -> Result<T>
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

It's also worth pointing out what exactly happens at the scope end when all
unjoined threads get automatically joined. If all joins succeed, we take
the result of the main closure passed to `scope()` and wrap it inside `Ok`.

If any thread panics (and in fact multiple threads can panic), we collect
all those panics into a `Vec`, box it, and finally wrap it inside `Err`.
The error type is then erased because `thread::Result<T>` is just an
alias for:

```rust
Result<T, Box<dyn Any + Send + 'static>>
```

This way we can do `thread::scope(...).unwrap()` to propagate all panics
in child threads into the main parent thread.

If the main `scope()` closure has panicked after spawning threads, we
just resume unwinding after joining child threads.

# Drawbacks
[drawbacks]: #drawbacks

The main drawback is that scoped threads make the standard library a little bit bigger.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The alternative is to keep scoped threads in external crates. However, there are
several advantages to having them in the standard library.

This is a very common and useful utility and is great for learning, testing, and exploratory
programming. Every person learning Rust will at some point encounter interaction
of borrowing and threads. There's a very important lesson to be taught that threads
*can* in fact borrow local variables, but the standard library doesn't reflect this.

Some might argue we should discourage using threads altogether and point people to
executors like Rayon and Tokio instead. But still,
the fact that `thread::spawn()` requires `F: 'static` and there's no way around it
feels like a missing piece in the standard library.

Finally, it's indisputable that users keep asking for scoped threads on IRC and forums
all the time. Having them as a "blessed" pattern in `std::thread` would be beneficial
to everyone.

# Prior art
[prior-art]: #prior-art

Crossbeam has had
[scoped threads](https://docs.rs/crossbeam/0.7.1/crossbeam/thread/index.html)
since Rust 1.0.

Rayon also has [scopes](https://docs.rs/rayon/1.0.3/rayon/struct.Scope.html),
but they work on a different abstraction level - Rayon spawns tasks rather than
threads. Its API is almost the same as proposed in this RFC, the only
difference being that `scope()` propagates panics instead of returning `Result`.
This behavior makes more sense for tasks than threads.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None.

# Future possibilities
[future-possibilities]: #future-possibilities

None.
