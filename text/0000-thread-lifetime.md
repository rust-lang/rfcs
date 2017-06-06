- Feature Name: thread-lifetime
- Start Date: 2016-08-05
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add a bulletin lifetime `'thread`, behaving like `'static`, but with the
limitation of being attached to the current thread.

# Motivation
[motivation]: #motivation

Thread local variables [are currently unsound](https://github.com/rust-lang/rust/issues/17954). Take the following code:

```rust
#![feature(thread_local)]
#[thread_local]
static FOO: usize = 3;

fn main() {
    let a = &FOO;
    let jg = std::thread::spawn(move || {
        println!("{}", a);
    });

    jg.join().unwrap();
}
```

Here we pass a reference to a thread local, FOO, to another thread, exposing
the value. This can easily be exploited to create data races.

Work-around exists ([in libstd](https://doc.rust-lang.org/nightly/std/macro.thread_local!.html), and [in other crates](https://github.com/redox-os/ralloc/blob/perf/src/tls.rs)), but these are hacky and sometimes inconvenient.

To solve this (as well as some other issues), we introduce the `'thread` lifetime.

# Detailed design
[design]: #detailed-design

`'thread` outlives every lifetime with the exception of `'static`.

The behavior is essentially the same as `'static`, with an addition of a crucial property: thread locality. Any type depending on `'thread` (i.e., a type product of the type construction from `'thread`) is `!Send`, and thus bounded to the current thread.

# Drawbacks
[drawbacks]: #drawbacks

This adds more complexity.

# Alternatives
[alternatives]: #alternatives

## Provide a `ThreadRef` primitive in libstd

This should implement a method, `get`, which would return a reference to the inner value. To avoid sending it across thread boundary, it will be `!Send`.

# Unresolved questions
[unresolved]: #unresolved-questions

Does this fully close [issue 17956](https://github.com/rust-lang/rust/issues/17954)?
