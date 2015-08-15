- Feature Name: custom_panic_handlers
- Start Date: 2015-05-01
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Allow control over what happens beside unwinding when a thread panic by **replacing** the *panic handler* of **a specific thread**.


# Motivation

Custom panic handlers would allow for a more flexible handling of panics: threads could, for example, send a message to a channel, write status information to a file, display a more user-friendly message. `Error` objects could be passed by panic to the parent thread for finer-grained handling later on without a long chain of `Result`s.

There is currently no way to prevent the call of `panic!` from writing an error message along with the thread, file and line number to `stderr`, even if the panic is caught with [`std::thread::catch_panic`](http://doc.rust-lang.org/nightly/std/thread/fn.catch_panic.html) or happens within a child thread. This is problematic because `panic!` enables returning from a chain of deeply nested functions in an exceptional situation without complicating their signatures.

An example: you have a function, ten levels deep in a child thread. After small additions to the code, it can now encounter a non-recoverable error.
Is it better to refactor ten functions to use `Result` for a one-time, unrecoverable error which should exit the thread anyway, or use `panic!`? I'm not advocating the overuse of `panic!` as a cheap error handling mechanism, but it has its uses. Otherwise, panics would just crash the whole program instead of being possibly caught.

Using `Result`s or `Option`s and creating new enums (to use with `Result`) just to handle these exceptional situations feels overkill, adds a lot of boilerplate while providing little extra functionality, and makes it harder to move around these functions without modifying their return types, while `panic!` gives very similar functionality without all the boilerplate, but since `panic!()` clutters logs with messages appropriate for debugging but not for the end user, one is often forced to resort to the former. Moreover, using Result in this situation creates a lot of unnecessary branching.

Another concern is that some third-party library functions you have no control upon may panic on exceptional situations, and it is often not desirable to output debugging information to the user in this case.

Currently, it is partially possible to customize the panic behavior by registering new callbacks with [`rt::unwind::register`](http://doc.rust-lang.org/std/rt/unwind/fn.register.html), however this solution is limited because it allows a maximum of 16 callbacks, and those are global, whereas the proposed solution uses a thread-local handler.


# Detailed design

Handlers are `Fn(&PanicData) + 'static` closures.  
The `PanicData` struct holds all the data related to a given panic, in order to allow later extension of the data passed later on (see [@sfackler's comment](https://github.com/rust-lang/rfcs/pull/1100#discussion_r33882931)).


Handlers are also thread-local. Since `Fn()` closures cannot be cloned, threads store a `RefCell<Rc<Fn(&PanicData)>>` in order to allow accessing a handler after it has been set. A thread's handler is initialized to the default handler defined in libstd/panicking.rs, `on_panic`.

`rt::unwind` exposes two accessor functions:

```rust
pub fn set_panic_handler<T: Fn(&PanicData) + 'static>(new_handler: T) {
  ON_PANIC.with(|cb_refcell| *cb_refcell.borrow_mut() = Rc::new(new_handler));
}

pub fn get_panic_handler() -> Rc<Fn(&PanicData) + 'static> {
  ON_PANIC.with(|cb_refcell| cb_refcell.borrow().clone())
}
```

Since `rt` is unstable, `thread` defines similar accessors that simply forward their arguments to those in `rt::unwind` (I'm unaware of any other way to expose unstable items in a stable module).

```rust
pub fn set_panic_handler<T: Fn(&PanicData) + 'static>(handler: T) {
  unwind::set_panic_handler(handler);
}

pub fn get_panic_handler() -> Rc<Fn(&PanicData) + 'static> {
  unwind::get_panic_handler()
}
```

`panicking::on_panic` is modified to take a `&PanicData` and thereby match the `Fn(&PanicData) + 'static` signature.

`std::rt::unwind::begin_unwind_inner` calls the thread's panic handler before processing regular callbacks from the old implementation (unless they are removed - see unresolved questions), without needing synchronization.

If the user wishes not to log panics, he can define an empty function following the `Callback` signature and pass it to `set_panic_handler`.

No handlers other than the default one are added to `std`, as it is trivial to define more advanced handlers tailored to the needs of the program in user code. Common, reusable handlers can grow on crates.io without being tied to the standard library.

**For implementation details, please see [this diff](https://github.com/filsmick/rust-tlcph-poc/compare/d2ec6c375aeebacb8e792cbcb9e0f770c236a120...master) and [this example of usage](https://gist.github.com/filsmick/e0505d7171c997b45a53)**


# Drawbacks

This proposed solution does not implement inheritance of panic handlers between threads (that is, if thread A has the handler `foo` and spawns a second thread B, B will also have `foo` as a handler).
For now, it is probably best to keep the changes minimal and let this new API mature, as this can be implemented later in the thread spawning code relatively easily (read the calling thread's panic handler and set the new thread's handler to it).


# Alternatives

The original solution I proposed was to just make `panic!()` output nothing to `stderr` when called without arguments, instead of calling `panic!("explicit panic")` internally. This is simpler to implement and requires less changes to `rt::unwind`. However, it turned out a more flexible implementation would be better to support most use cases; one does not always have control over the functions he calls, and hence cannot prevent them from panicking and logging a debug message without rewriting their code.

We could also not change anything to the panicking mechanisms, use `catch_panic` to react differently on panics and ignore unwanted panic messages, or run the panicking process from a wrapper which removes all "thread {} panicked" entries from the output of the program before forwarding it. However, this approach is much more cumbersome than the proposed solution and much less efficient as it requires running a new wrapper process.

In case one simply wishes to prevent logging of panics in a specific thread entirely, they can also use [this workaround](https://github.com/rust-lang/rust/issues/24099#issuecomment-89908401).


# Unresolved questions

What should happen to the [current unstable implementation of callbacks](http://doc.rust-lang.org/std/rt/unwind/fn.register.html)? A single custom panic handler as proposed in this RFC can call other handlers, eliminating the need for a callback list, but the old mechanism still allows for global panic handlers. However if a new RFC for handler inheritance between threads land later, they will effectively have lost most of their interest.
