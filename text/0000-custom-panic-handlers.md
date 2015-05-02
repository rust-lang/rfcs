- Feature Name: custom_panic_handlers
- Start Date: 2015-05-01
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Allow the registration of alternative *panic handlers* to customize the output of the `panic!` macro.

# Motivation


There is currently no way to prevent the call of `panic!` from writing an error message along with the thread, file and line number to `stderr`, even if the panic is caught with [`std::thread::catch_panic`](http://doc.rust-lang.org/nightly/std/thread/fn.catch_panic.html) or happens within a child thread.

This is problematic because `panic!` enables returning from a chain of deeply nested functions in an exceptional situation without complicating their signatures.

An example: you have a function, ten levels deep in a child thread, that suddently can encounter a non-recoverable error (after additions to the code). Do you want to refactor ten functions to use `Result` - at a point where you have not yet decided what the best long-term approach is, as it could require heavy refactoring of the existing codebase - or use `panic!` for a while to experiment without hurting user experience (by that I mean, displaying messages best used for debugging to the user - does the user really care about the thread and line number a function failed in?). I'm not advocating the overuse of `panic!` as a cheap error handling mechanism, but it has its uses. Otherwise it would just crash the whole program instead of being possibly caught.

Using `Result`s or `Option`s and creating new enums (to use with `Result`) just to handle these exceptional situations feels overkill, adds a lot of boilerplate while providing little extra functionality, and makes it harder to move around these functions without modifying their return types, while `panic!` gives very similar functionality without all the boilerplate, but since `panic!()` clutters logs with messages appropriate for debugging but not for the end user, one is often forced to resort to the former.

Another concern is that some third-party library functions you have no control upon may panic on exceptional situations, and it is often not desirable to output debugging information to the user in this case.


# Detailed design

The current panic handler is defined [here](https://github.com/rust-lang/rust/blob/2b8c9b12f91c0bf2c1e6278a5f803c2df3698432/src/libstd/panicking.rs#L28).

This RFC proposes to allow control over what happens beside unwinding when a thread panic by **changing** the *panic handler* of this thread, instead of simply adding new ones with [`rt::unwind::register`](http://doc.rust-lang.org/std/rt/unwind/fn.register.html).


A few common handlers would be added to `std::thread`: **`debug`**, **`basic`** and **`silent`**.  
The `debug` handler would be the default and have a behavior similar to the current `on_panic`, to avoid modifying the behavior of existing code.
`basic` would print whatever is passed to `panic!()` without adding information to the message.
`silent` would never output anything. 

A handler (or a *callback*, as this was called in `rt::unwind`'s code) is registered with `thread::on_panic`. It has the following signature:

``` rust
fn(msg: &Any + Send, file: &'static str, line: u32) -> bool
```

It is almost the same as the current [`Callback`](http://doc.rust-lang.org/std/rt/unwind/type.Callback.html) signature, except that it returns a `bool` to indicate whether handler execution should go on after the execution of the callee. 

Handlers that simply add a layer of logging return `true` to execute the next one in the list, and 'overriding' handlers return `false` to stop there.  
Handlers are called in reverse order of registration (first registered, last called), to place the 'most important' ones (usually one of `thread::handlers`) at the top.  


This new API would be used as follows:

```rust
use std::thread;

fn main() {
    thread::spawn(|| {
        // use the default panic handler, `debug`, the one we currently have

        some_optional.unwrap();
    }).join();

    thread::spawn(|| {
        // Always use the `silent` handler
        thread::on_panic(thread::handlers::silent);

        // This will never display a message
        
        // Use cases:
        // - abort thread silently on errors unrecoverable at its level,
        //   but recoverable at the process level, and spawn a new one
        a_third_party_function_that_could_fail();
    }).join();

    thread::spawn(|| {
        // Always use the `basic` handler
        thread::on_panic(thread::handlers::basic);

        // This will simply display whatever is passed to `panic!` on panic,
        // much like println!()
        
        // Use cases:
        // - abort thread on invalid input with a user-friendly message
        if !is_valid(input) {
            panic!("Invalid input - please try again");
        } else {
            // ...
        }
        
    }).join();
    
    
}
```

Users wishing to have `debug` messages on debug builds and `basic` messages on release builds could simply do this:

```rust
thread::on_panic(
    if cfg!(debug) {
        thread::handlers::debug
    } else {
        thread::handlers::basic
    }
);
```


**The `basic` handler does *not* print a newline when `panic!` is called without arguments**, to allow silencing `panic!` in specific cases (silent recovery by spawning a new thread) without hiding other `panic` messages which may be useful for debugging.


# Drawbacks

As this doesn't break existing code, the only one I can think of is that this will require a pretty heavy refactoring of `rt::unwind`.


# Alternatives

The original solution I proposed was to just make `panic!()` output nothing to `stderr` when called without arguments, instead of calling `panic!("explicit panic")` internally. This is simpler to implement and requires less changes to `rt::unwind`.

However, it turns out a more flexible implementation would be better to support most use cases; one does not always have control over the functions he calls, and hence cannot prevent them from panicking and logging a debug message without rewriting their code.

We could also not change anything to the panicking mechanisms and accept unwanted messages in log files.


# Unresolved questions

How should the `rt::unwind` functions be refactored to implement this modified panic callback / handler mechanism?