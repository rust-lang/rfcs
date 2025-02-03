- Feature Name: RealtimeSanitizer, `realtime(nonblocking)` and `realtime(blocking)` attributes
- Start Date: 2025-01-30
- RFC PR: https://github.com/rust-lang/rfcs/pull/3766
- Rust Issue: 

# Summary
[summary]: #summary

Many software projects that utilize Rust are subject to real-time constraints. Software that is written for use in audio, embedded, robotics, and aerospace must adhere to strict deterministic-time execution or face consequences that may be catastrophic. LLVM 20 introduces RealtimeSanitizer, one approach to detecting and reporting disallowed non-deterministic execution time calls in real-time contexts.

This RFC proposes that RealtimeSanitizer be integrated into the Rust ecosystem. To serve that end, we propose a few changes, outlined in this document:

1. RealtimeSanitizer can be enabled in unstable mode - like the other sanitizers
2. The introduction of `realtime(nonblocking)` (marking a function as real-time constrained) and `realtime(blocking)` (marking a function as inappropriate for use in a `realtime(nonblocking)` context)
3. The addition of the `rtsan_scoped_disabler!` macro
4. Disabling rtsan for the `panic!` and `assert*!` macros

# Motivation
[motivation]: #motivation
 
Increasingly, Rust is being used in problem spaces that are real-time constrained, such as audio, robotics, aerospace and embedded. Real-time programming is defined by deadlines - if a solution is not provided by a specific deadline, some consequence may occur. 

For example:

> In an autonomous vehicle perception subsystem, it is not enough to detect an obstacle and decide to stop in some unknown amount of time. You must detect the obstacle AND stop within N ms, or you may crash.

> In audio, you must fill a buffer and pass it back to the operating system within N ms, otherwise your user may hear a click or pop which may damage their audio equipment, or minimally annoy them.

> In aerospace guidance systems if your software doesn't update on a regular tick your simulation of what is happening may diverge from reality. Unfortunately this may also mean your rocket converges with the ground.

Code in these environments must run in a deterministic amount of time. Allocations, locks, and other OS resource access are disallowed because they don't have an upper bound on their execution time.

**Historically, it has been very difficult for programmers to detect these issues in their code. RealtimeSanitizer is one approach to detecting real-time safety issues before they run on end-users machines.**

A few resources that go into more depth on real-time programming:
* https://en.wikipedia.org/wiki/Real-time_computing
* http://www.rossbencina.com/code/real-time-audio-programming-101-time-waits-for-nothing
* https://www.youtube.com/watch?v=ndeN983j_GQ

## A note on terminology real-time unsafe

This document uses the common parlance "real-time unsafe" to discuss calls which have no deterministic runtime. This is separate from the rust concept of memory or thread unsafety, typically indicated by the `unsafe` keyword.

To disambiguate, when talking about non-deterministic calls this document will attempt to avoid ambiguous uses of the word "unsafe" and use the phrase "real-time-unsafe" when talking about non-deterministic time calls. Having real-time-unsafe code in your real-time contexts does not risk invoking any kind of UB.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

[RealtimeSanitizer](https://clang.llvm.org/docs/RealtimeSanitizer.html) can detect and alert users to real-time safety issues when they occur. This new sanitizer has been integrated into LLVM 20. You can explore this tool using clang in Compiler Explorer using the `-fsanitize=realtime` flag. This proposal aims to mimic much of the behavior available in clang. 

A function marked with the new attribute `realtime(nonblocking)` is the real-time restricted execution context.  **In these `realtime(nonblocking)` functions, two broad sets of actions are disallowed:**

1. Intercepted calls into libc, such as `malloc`, `socket`, `write`, `pthread_mutex_*` and many more, representing a broad collection of allocations, locks and system calls.

Each of these actions are known to have non-deterministic execution time. When these actions occur during a `realtime(nonblocking)` function, or any function invoked by this function, they print the stack and abort.

[Example of this working in Compiler Explorer in C++](https://godbolt.org/z/sPTh63o67).

The full list of intercepted functions can be found on [GitHub](https://github.com/llvm/llvm-project/blob/main/compiler-rt/lib/rtsan/rtsan_interceptors_posix.cpp), and is continually growing.

2. User defined functions marked `realtime(blocking)`

The new `realtime(blocking)` attribute allows users to mark a function as having non-deterministic runtime, disallowing its use in functions marked `realtime(nonblocking)`.

[Example of this working in Compiler Explorer in C++](https://godbolt.org/z/dErqE5nnM)

One classic example of this is a spin-lock `lock` method. Spin locks do not call into a `pthread_mutex_lock`, so they cannot be intercepted. However they are still prone to spinning indefinitely, so they are disallowed in real-time contexts. The `realtime(blocking)` attribute allows a user to document this behavior in their code.

An example of an improper allocation being detected in a `realtime(nonblocking)` function:
```rust
> cat example/src/main.rs
#[realtime(nonblocking)]
pub fn process() {
    let audio = vec![1.0; 256]; // allocates memory
}

fn main() {
    process();
}

> cargo run --package example 
==16304==ERROR: RealtimeSanitizer: unsafe-library-call
Intercepted call to real-time unsafe function `malloc` in real-time context!
    #0 0x0001052c5bcc in malloc+0x20 (libclang_rt.rtsan_osx_dynamic.dylib:arm64+0x5bcc)
    #1 0x000104cd7360 in alloc::alloc::alloc::h213dba927a6f8af7 alloc.rs:98
    #2 0x000104cd7478 in alloc::alloc::Global::alloc_impl::h7034d3dd14644937 alloc.rs:181
    #3 0x000104cd7bac in _$LT$alloc..alloc..Global$u20$as$u20$core..alloc..Allocator$GT$::allocate::hd5e7c341a83b5ed4 alloc.rs:241
    ... snip ...
    #11 0x000104cd7d30 in std::sys::backtrace::__rust_begin_short_backtrace::h73bbd1f9991c56fb backtrace.rs:154
    #12 0x000104cd72cc in std::rt::lang_start::_$u7b$$u7b$closure$u7d$$u7d$::hf6a09edd6941dbc1 rt.rs:164
    #13 0x000104cf1958 in std::rt::lang_start_internal::h9e88109c8deb8787+0x324 (example:arm64+0x10001d958)
    #14 0x000104cd7298 in std::rt::lang_start::ha4c268826019738b rt.rs:163
    #15 0x000104cd9658 in main+0x20 (example:arm64+0x100005658)
    #16 0x00018d8860dc  (<unknown module>)
    #17 0xf062fffffffffffc  (<unknown module>)

SUMMARY: RealtimeSanitizer: unsafe-library-call alloc.rs:98 in alloc::alloc::alloc::h213dba927a6f8af7
fish: Job 1, 'cargo run --package example --f…' terminated by signal SIGABRT (Abort)
```
# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

We would like to propose that RTSan be integrated into Rust using similar semantics to clang.

There are a few sub-pieces to consider when integrating RTSan:
### The integration of the sanitizer

Similar to ASan and TSan, we propose adding RTSan as an unstable feature. Enabling RTSan will be done via the same method.

```
RUSTFLAGS=-Zsanitizer=realtime cargo build
```

Much of the heavy lifting in this tool is done in the LLVM IR and runtime library, so the changes to `rustc` front-end should be light. 

This process to enable a sanitizer has been completed by many of the other LLVM sanitizers, and we would follow their template for exposing the RUSTFLAGS.

[PR adding support for lsan, tsan, msan, asan](https://github.com/rust-lang/rust/pull/38699)

### The addition two new outer attributes to rust - `#[realtime(nonblocking)]` `#[realtime(blocking)]`

`#[realtime(nonblocking)]` defines a scope as real-time constrained. During this scope, one cannot call any intercepted call (`malloc`, `socket` etc) or call any function marked `#[realtime(blocking)]`. The rustc front-end will parse the `#[realtime(nonblocking)]` attribute and add the LLVM attribute `llvm::Attribute::SanitizeRealtime`. This will leave most of the work to the already-implemented LLVM instrumentation pass.

`#[realtime(blocking)]` defines a function as unfit for execution within a `#[realtime(nonblocking)]` function. The rustc front-end will parse the `#[realtime(blocking)]` attribute and add the LLVM attribute `llvm::Attribute::SanitizeRealtimeBlocking`.

The example in the previous section shows that the interceptors written for the RealtimeSanitizer runtime are mostly shared across Rust and C/C++. Rust calls into libc `malloc` for basic allocation operations so it is automatically intercepted with no additional changes to the Rust version. A vast majority of the real-time-unsafe behavior that RTSan detects will be detected in this way.

Users may also mark their own functions as unfit for a `realtime(nonblocking)` context with the `#[realtime(blocking)]` attribute, as seen below. This allows for detection of calls that do not result in a system call, but may be non-deterministically delayed.

```rust
#[realtime(blocking)]
fn spin() {
    loop {}
}


#[realtime(nonblocking)]
fn process() {
    spin();
}

fn main() {
    process();
}
> cargo run --package example --features rtsan
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.05s
     Running `target/debug/example`
==16364==ERROR: RealtimeSanitizer: blocking-call
Call to blocking function `spin` in real-time context!
    #0 0x0001021a6940 in rtsan::notify_blocking_call::ha111b6bcc1e1c566+0x24 (example:arm64+0x100002940)
    #1 0x0001021a6864 in example::spin::h3f7c52e56f3fcffa main.rs:1
    #2 0x0001021a6880 in example::process::hace23451997fd8f2 main.rs:9
    #3 0x0001021a6840 in example::main::h700c0ae2eb60a977 main.rs:16
    #4 0x0001021a670c in core::ops::function::FnOnce::call_once::hd7f394a2ba7ce532 function.rs:250
    #5 0x0001021a6824 in std::sys::backtrace::__rust_begin_short_backtrace::h73bbd1f9991c56fb backtrace.rs:154
    #6 0x0001021a67ec in std::rt::lang_start::_$u7b$$u7b$closure$u7d$$u7d$::hf6a09edd6941dbc1 rt.rs:164
    #7 0x0001021bebbc in std::rt::lang_start_internal::h9e88109c8deb8787+0x324 (example:arm64+0x10001abbc)
    #8 0x0001021a67b8 in std::rt::lang_start::ha4c268826019738b rt.rs:163
    #9 0x0001021a68b4 in main+0x20 (example:arm64+0x1000028b4)
    #10 0x00018d8860dc  (<unknown module>)
    #11 0x4c3d7ffffffffffc  (<unknown module>)

SUMMARY: RealtimeSanitizer: blocking-call (example:arm64+0x100002940) in rtsan::notify_blocking_call::ha111b6bcc1e1c566+0x24
fish: Job 1, 'cargo run --package example --f…' terminated by signal SIGABRT (Abort)

```

### The addition of the `rtsan_scoped_disabler!` macro

It will be important to allow users to opt-out of rtsan detection for a specific scope. This may be useful if the end user thinks RTsan has a false positive, or it happens in third-party code they don't control.

We propose addition of the `rtsan_scoped_disabler!` macro:
```rust
#[realtime(nonblocking)]
fn process() {
  rtsan_scoped_disabler!({
        let audio = vec![1.0; 256]; // report is suppressed
  });
}
```

When rtsan is not enabled with RUSTFLAGS, this will become a no-op, so it will be safe for users to leave in their code. 

We are open to advice on which file this macro belongs in.

### Run-time suppression list
There is one other way to opt-out of rtsan checking which will automatically work with Rust.

Users may specify suppression lists, which may be passed in via an environment variable
```
> cat suppressions.supp
call-stack-contains:*spin*
> RTSAN_OPTIONS=suppressions=suppressions.supp cargo run
```

### `no_sanitize`
Another approach we could take is similar to the ASan and TSan `no_sanitize` attribute. We advocate for the scoped disabler macro, as it allows users to specify a more specific scope to disable the tool in. This means users will not have to extract real-time-unsafe code into helper functions to disable them at the function level.

To match the other sanitizers, adding in `no_sanitize` could be considered instead of/in addition to the macro, depending on input on this RFC.

### Disabling RealtimeSanitizer in the `panic!` and all `assert!` macros.

If users rely on `panic!` or `assert!` while running under RealtimeSanitizer, they will hit an intercepted call before the message is printed.

For example:

```rust
#[realtime(nonblocking)]
fn processor(buffer: &[f32]) {
    buffer[512]; // Oops, out of bounds!! should panic!
}

processor(&[0.0; 512]);
```

Running under RTSan the user gets this message. This is due to some memory being allocated to prepare to print the panic message.
```
==31969==ERROR: RealtimeSanitizer: unsafe-library-call
Intercepted call to real-time unsafe function `malloc` in real-time context!
```

A user should expect this out of bounds access to print:
```
index out of bounds: the len is 512 but the index is 512
```

To adhere to this expected behavior, RTSan should be disabled for `panic!` and each of the assertion macros: `assert`, `assert_eq`, `assert_ne`, `debug_assert`, `debug_assert_eq`, `debug_assert_ne`.

# Drawbacks
[drawbacks]: #drawbacks

Many of the drawbacks are minimized by the fact that RTSan will only be available in unstable Rust, until the rest of the sanitizers are stabilized this is not likely to change.

Introducing new attributes to the language means more code bloat and maintenance cost. 

RTSan may also inadvertently increase the number of "bug reports" the rust language gets if the real-time safety of a standard library implementation changes.

For instance, let's pretend the standard library exposes `foo::bar()`, which then gets used in real-time code, checked with RealtimeSanitizer, and is later changed to have an allocation. The end user detecting this change could file a bug against the standard library complaining about the new allocation and asking the new allocation to be reverted.

In the opinion of the author, unless `foo::bar()` is annotated as `realtime(nonblocking)` this should be closed as "Not A Bug", but dealing with this case still takes resources from the maintainers.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

RealtimeSanitizer is a run-time approach to detecting real-time safety issues. The run-time approach has many positives, but also some drawbacks. As an alternative to this approach, we could do a compile time check that any `realtime(nonblocking)` function can only call other `realtime(nonblocking)` functions. This would be similar to LLVM's new [function effect analysis system](https://clang.llvm.org/docs/FunctionEffectAnalysis.html).

We designed RTSan hand-in-hand with the static functions effects system, and through that design process we came to the conclusion the compile-time and run-time approaches complement each other. It is our recommendation that if a static approach is considered, it is done **in addition** to rtsan, not **instead of**.

Some strengths of rtsan/run-time detection:
- Lighter weight on the end code-writer, easier to get started.
- Isn't prone to false-positives, like pushing into a pre-reserved vector.
- Can easily work with third party libraries.
- Can "see through" false annotations, and sanity check a partial implementation of the static approach.

Some strengths of a static approach:
- Heavier lift to get started for the end code-writer, but more foolproof in the end.
- Not prone to false negatives, if some path is not hit in the code, or an interceptor is not implemented.

Overall, both approaches complement each other. Taking this proposal would allow for a future extension of a function-effects-like system to be added to rustc in the future.


# Prior art
[prior-art]: #prior-art

[A link to RealtimeSanitizer in C/C++ in LLVM 20.](https://clang.llvm.org/docs/RealtimeSanitizer.html)

# Unresolved questions
[unresolved-questions]: #unresolved-questions

* What file should the `rtsan_scoped_disabler!` macro be inserted into?
* Should `no_sanitize` be supported in addition to `rtsan_scoped_disabler`?

# Future possibilities
[future-possibilities]: #future-possibilities

As stated above in Rationales and Alternatives, the addition of the `realtime(nonblocking)` and `realtime(blocking)` attributes allows for a future static analysis extension similar to clang's function effect analysis system.
