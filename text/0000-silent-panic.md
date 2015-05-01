- Feature Name: silent_panic
- Start Date: 2015-05-01
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Make `panic!` output nothing when called without arguments.

# Motivation

Currently, `panic!()` is the same as `panic!("explicit panic")`, and hence logs an error message along with the thread, file and line number every time it is called, even if the panic is caught with [`std::thread::catch_panic`](http://doc.rust-lang.org/nightly/std/thread/fn.catch_panic.html). 

This is problematic because `panic!` enables returning from a chain of deeply nested functions in an exceptional situation without complicating their signatures.

An example: you have a function, ten levels deep in a child thread, that suddently can encounter a non-recoverable error (after additions to the code). Do you want to refactor ten functions to use `Result` - at a point where you have not yet decided what the best long-term approach is, as it could require heavy refactoring of the existing codebase - or use `panic!` for a while to experiment without hurting user experience (by that I mean, displaying messages best used for debugging to the user - does the user really care about the thread and line number a function failed in?). I'm not advocating the overuse of `panic!` as a cheap error handling mechanism, but it has its uses. Otherwise it would just abort the whole program instead of being possibly caught by `catch_panic`.

Using `Result`s or `Option`s and creating new enums (to use with `Result`) just to handle these exceptional situations feels overkill, adds a lot of boilerplate while providing little extra functionality, and makes it harder to move around these functions without modifying their return types, while `panic!` gives very similar functionality without all the boilerplate, but since `panic!()` clutters logs with messages appropriate for debugging but not for the end user, one is often forced to resort to the former.



# Detailed design

Modify the `panic!` macro to call a logging-less version of `std::rt::begin_unwind` when called without arguments:

```rust
macro_rules! panic {
    () => ({
    		//panic!("explicit panic")
        $crate::rt::begin_unwind_silent()
        //          ~~~~~~~~~~~~~~~~~~^
        // Change happens here ^
    });
    ($msg:expr) => ({
        $crate::rt::begin_unwind($msg, {
            // static requires less code at runtime, more constant data
            static _FILE_LINE: (&'static str, u32) = (file!(), line!());
            &_FILE_LINE
        })
    });
    ($fmt:expr, $($arg:tt)+) => ({
        $crate::rt::begin_unwind_fmt(format_args!($fmt, $($arg)+), {
            // The leading _'s are to avoid dead code warnings if this is
            // used inside a dead function. Just `#[allow(dead_code)]` is
            // insufficient, since the user may have
            // `#[forbid(dead_code)]` and which cannot be overridden.
            static _FILE_LINE: (&'static str, u32) = (file!(), line!());
            &_FILE_LINE
        })
    });
}
```

I'm unfortunately not familiar enough with the compiler to fully understand the workings of `begin_unwind_inner` and `rust_panic` (found [here](https://github.com/rust-lang/rust/blob/master/src/libstd/rt/unwind.rs)), but they appear to have logging bolted-on, and implement this RFC will require quite a bit of refactoring.

The new 'silent panic' would then be used like the older version. One particularly use case would be to abort a worker thread silently when it encounters a fatal error, catch the panic then spin up a new one.



# Drawbacks

People using `panic!()` in their code and expecting to see the error message with a line number will not see it.

However, using `panic!()` without a descriptive message is arguably not a good practice, and this is easily fixed by adding one in the code.



# Alternatives

Another solution - which is not strictly a replacement for this - is to add a '*panic handler*' to threads, which defaults to the current behavior (logging the panic along with the line number, file and thread), but can be overriden for each individual thread.

This would allow for an even finer control over `panic!` logging, especially in case a third-party library panics with a message and you cannot change its code to prevent the message from being displayed. Eventually, it would be best to also implement this.

Credits to [@Diggsey](https://github.com/Diggsey) for this idea. 

# Unresolved questions

At the implementation level, how should the `rt` functions for unwinding be refactored to allow logging-less panics without sacrificing functionality or duplicating code?
