- Feature Name: Backtrace in `core`
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC proposes moving the `Backtrace` type from `std` to `core` (and some changes to `Backtrace` to facilitate this). The change is motivated by the desire to move `Error` from `std` to `core`, which requires either moving `Backtrace` or abstracting it out of `Error`.

It is still unclear whether `Backtrace` itself does need to be moved to `core` and in time this RFC will be unanimous in this matter.

# Motivation
[motivation]: #motivation

The main reason behind moving `Backtrace` to `core` is to have essential types available for wider usage without the need to import `std`. While `Error` had a valid reason for not being in `core` (it relies on `Box` type for different conversions), `Backtrace` does not have similar blockers apart from its frame-allocating API

Additionally, having this type in `core` will allow its users to provide their own implementations of the backtrace collection and reporting and not rely on the `std`-provided one if they don't want to.

The outcome of this RFC will be a `Backtrace` type in `core` with implementation defined in `std` and compiler-generated implementation when `std` is not linked and user did not provide their own implementation for the reporting functions.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Backtraces are an essential part of the Rust ecosystem, being close coupled with error handling and reporting, be it to the user or the developer. They are usually of a common form:
```rust
   Compiling playground v0.0.1 (/playground)
    Finished dev [unoptimized + debuginfo] target(s) in 1.66s
     Running `target/debug/playground`
thread 'main' panicked at 'index out of bounds: the len is 4 but the index is 4', src/main.rs:5:5
stack backtrace:
   0: rust_begin_unwind
             at /rustc/53cb7b09b00cbea8754ffb78e7e3cb521cb8af4b/library/std/src/panicking.rs:493:5
   1: core::panicking::panic_fmt
             at /rustc/53cb7b09b00cbea8754ffb78e7e3cb521cb8af4b/library/core/src/panicking.rs:92:14
   2: core::panicking::panic_bounds_check
             at /rustc/53cb7b09b00cbea8754ffb78e7e3cb521cb8af4b/library/core/src/panicking.rs:69:5
   3: <usize as core::slice::index::SliceIndex<[T]>>::index_mut
             at /rustc/53cb7b09b00cbea8754ffb78e7e3cb521cb8af4b/library/core/src/slice/index.rs:190:14
   4: core::slice::index::<impl core::ops::index::IndexMut<I> for [T]>::index_mut
             at /rustc/53cb7b09b00cbea8754ffb78e7e3cb521cb8af4b/library/core/src/slice/index.rs:26:9
   5: <alloc::vec::Vec<T,A> as core::ops::index::IndexMut<I>>::index_mut
             at /rustc/53cb7b09b00cbea8754ffb78e7e3cb521cb8af4b/library/alloc/src/vec/mod.rs:2396:9
   6: playground::main
             at ./src/main.rs:5:5
   7: core::ops::function::FnOnce::call_once
             at /rustc/53cb7b09b00cbea8754ffb78e7e3cb521cb8af4b/library/core/src/ops/function.rs:227:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
```

They allow for a detailed inspection of what crashed at runtime and which libraries and functions were involved in the process. Obviously reporting and formatting it is also of a big importance, so `Error` allows for backtrace printing with help of a `backtrace()` method.

Currently, `Backtrace` is an essential part of the `std` library and user can control whether the backtrace is enabled or disabled using the environmental variable: `RUST_BACKTRACE`.

In terms of Guide-level changes, there is not much to be discussed - only that it is moved to `core` and if `std` is not linked, automatic backtrace handlers will be generated. Otherwise, the regular implementation of `Backtrace` is present.

TODO: what else should be here?

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The following changes need to be made to implement this proposal:

## Move the `Backtrace` to `core` and add a thin wrapper in `std`

This way, a `StdBacktrace` struct is introduced in `std` that allows for `Debug` and `Display` formatting. 

The regular API of `Backtrace` comprising `enabled()`, `create()` and `status()` would be left in the `std` as free-standing functions. Since, they are lang items they need to have a default implementation in case `std` is not linked, so they will be provided in such a form in the `core` library (and overwritten once `std` is linked):

```rust
/// Global implementation of backtrace functionality. Called to create
/// `RawBacktrace` trait objects.
#[cfg(not(bootstrap))]
extern "Rust" {
    #[lang = "backtrace_create"]
    fn backtrace_create(ip: usize) -> *mut dyn RawBacktrace;

    #[lang = "backtrace_enabled"]
    fn backtrace_enabled() -> bool;

    #[lang = "backtrace_status"]
    fn backtrace_status(raw: *mut dyn RawBacktrace) -> BacktraceStatus;
}

#[cfg(bootstrap)]
unsafe fn backtrace_create(_ip: usize) -> *mut dyn RawBacktrace {
    UnsupportedBacktrace::create().inner
}

#[cfg(bootstrap)]
unsafe fn backtrace_enabled() -> bool {
    false
}

#[cfg(bootstrap)]
unsafe fn backtrace_status(_raw: *mut dyn RawBacktrace) -> BacktraceStatus {
    BacktraceStatus::Unsupported
}

```

This change will make the `Backtrace` an optional part of the library and 
This way, if the `Backtrace` is not enabled (on the library level) there is no need for it to report 

Implementation-wise, `Backtrace` is **declared** in the `core` module and **defined** in `std` via *lang_items* which act as function hooks which are resolved during link-time. Special type `StdBacktrace` is introduced for the `std` part of implementation which acts as a proxy for `Debug` and `Display` trait impls.

// TODO: add examples of how would one implement these functions themselves like panic hooks



# Drawbacks
[drawbacks]: #drawbacks

There are actually many drawbacks, most important of them being `Backtrace` using a lot of `std` for the actual backtrace capturing. 

The other one is a potential code bloat in `no_std` contexts, so a possible alternative may be only enabling the `Backtrace` conditionally via `cfg` settings. (not so sure about this though)


`no_std` code bloat - Mara mentioned out
`Error:backtrace` also seems to be blocking??

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The proposed solution is the one which is currently implementable. However, if the Generic Member Access RFC was implemented (link!) we would not have to move the `Backtrace` to `core` at all. In the alternative solution, we would leave the `Backtrace` as it is and instead the `Error` trait will provide a `backtrace()` method which will use the Generic Access to extract the concrete `Backtrace` out of the propagated error.

Alternatively, since the actual implementation of `Backtrace` uses allocating structures like `Box` for backtrace creation, it would be prudent to move it to where this type resides. This will in turn allow users of `alloc` module in embedded and `no_std` contexts to use this functionality without `std`. _should we go for this though instead of the core when we introduce such big changes?_

A viable solution to allocating functions of `Backtrace` might be adding an API where the users could provide themselves the memory in which the backtrace should reside and truncate/report a failure in case the backtrace does not fit this preallocated space. 

# Prior art
[prior-art]: #prior-art

This type is already implemented, but it seems like no type was moved from `std` to `core` previously so we have no point of reference on this one.

As for `no_std` and embedded contexts, there exists the [mini-backtrace](https://github.com/amanieu/mini-backtrace) library that provides backtrace support via LLVM's libunwind.

# Unresolved questions
[unresolved-questions]: #unresolved-questions


# Future possibilities
[future-possibilities]: #future-possibilities

