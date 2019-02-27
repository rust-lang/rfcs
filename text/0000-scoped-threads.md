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

See the [rationale-and-alternatives](#rationale-and-alternatives) section for more.

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

We add a single new type to the `std::thread` module:

```rust
struct Scope<'a> {}
```

Next, we need the `scope()` and `spawn()` functions:

```rust
fn scope<'a, F, T>(f: F) -> Result<T>
where
    F: FnOnce(&Scope<'a>) -> T;

impl<'a> Scope<'a> {
    fn spawn<F, T>(&self, f: F) -> JoinHandle<T>
    where
        F: FnOnce(&Scope<'a>) -> T + Send + 'a,
        T: Send + 'a;
}
```

There's just one more thing that will make the API complete: The thread builder
needs to be able to spawn threads inside a scope.

```rust
impl Builder {
    fn spawn_scoped<'a, F, T>(self, scope: &Scope<'a>, f: F) -> io::Result<JoinHandle<T>>
    where
        F: FnOnce(&Scope<'a>) -> T + Send + 'a,
        T: Send + 'a;
}
```

Note that this interface is a bit simpler than the one in Crossbeam
because we can now merge `JoinHandle` and `ScopedJoinHandle` into a single type.
Moreover, in Crossbeam, `ScopedJoinHandle` is generic over `'scope`, which is
not really necessary for soundness so we can remove that lifetime to simplify
things further.

It's also worth discussing what exactly happens at the scope end when all
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

Crossbeam's logic for error handling can be found
[here](https://github.com/crossbeam-rs/crossbeam/blob/79210d6ae34a3e84b23546d8abc5c4b81b206019/crossbeam-utils/src/thread.rs#L167-L193).

# Drawbacks
[drawbacks]: #drawbacks

The main drawback is that scoped threads make the standard library a little bit bigger.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The alternative is to keep scoped threads in external crates. However, there are
several advantages to having them in the standard library:

* This is a very common and useful utility and is great for learning, testing, and exploratory
  programming. Every person learning Rust will at some point encounter interaction
  of borrowing and threads. There's a very important lesson to be taught that threads
  *can* in fact borrow local variables, but the standard library doesn't reflect this.

* Some might argue we should discourage using threads altogether and point people to
  executors like Rayon and Tokio instead. But still,
  the fact that `thread::spawn()` requires `F: 'static` and there's no way around it
  feels like a missing piece in the standard library.

* Implementing scoped threads is very tricky to get right so it's good to have a
  reliable solution provided by the standard library. Also, scoped threads in `libstd`
  will be simpler because we don't need to introduce a special type for
  [scoped join handles](https://docs.rs/crossbeam/0.7.1/crossbeam/thread/struct.ScopedJoinHandle.html)
  or [builders](https://docs.rs/crossbeam/0.7.1/crossbeam/thread/struct.ScopedThreadBuilder.html).

* There are many examples in the official documentation and books that could be
  simplified by scoped threads.

* Scoped threads are typically a better default than `thread::spawn()` because
  they make sure spawned threads are joined and don't get accidentally "leaked".
  This is sometimes a problem in unit tests, where "dangling" threads can accumulate
  if unit tests spawn threads and forget to join them.

* It's indisputable that users keep asking for scoped threads on IRC and forums
  all the time. Having them as a "blessed" pattern in `std::thread` would be beneficial
  to everyone.

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

1. `scope()` now returns a `thread::Result<T>` rather than `T`. This is because
   panics in the old design were just silently ignored, which is not good.
   By returning a `Result`, the user can handle panics in whatever way they want.

2. The closure passed to `Scope::spawn()` now takes a `&Scope<'env>` argument that
   allows one to spawn nested threads, which was not possible with the old design.
   Rayon similarly passes a reference to child tasks.

3. We removed `Scope::defer()` because it is not really useful, had bugs, and had
   non-obvious behavior.

4. `ScopedJoinHandle` got parametrized over `'scope` in order to prevent it from
   escaping the scope. However, it turns out this is not really necessary for
   soundness and was just a conservative safeguard.

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

In the future, we could also have a threadpool like Rayon that can spawn
scoped tasks.
