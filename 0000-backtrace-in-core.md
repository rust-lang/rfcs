- Feature Name: Backtrace in `core`
- Start Date: 2021-07-03
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC proposes moving the `Backtrace` type from std to core (and some changes to `Backtrace` to facilitate this). The change is motivated by the desire to move `Error` from std to core, which requires either moving `Backtrace` or abstracting it out of `Error`.

Backtrace is a (often specifically formatted) list of function calls which the program was in at the moment of its generation. Backtraces in Rust are generated when a program reaches an unrecoverable error and panics or they can be captured and stored when constructing recoverable errors by manually calling `Backtrace::capture()`.

This RFC does not cover eventually expanding the `Backtrace` API to include more functions and it does not solve the way backtraces are collected (although it proposes different takes on the matter).

# Motivation
[motivation]: #motivation

The main reason behind moving `Backtrace` to core is to have essential types available for wider usage without the need to import std. [Error Handling Group Post](https://blog.rust-lang.org/inside-rust/2021/07/01/What-the-error-handling-project-group-is-working-towards.html#1-error-trait--panic-runtime-integration) goes into details on why one would want the `Error` type (and consequently `Backtrace`) in core. The end goal is to have a `panic_error` function which would allow for generating panic errors with detailed error informations.

The [original PR](https://github.com/rust-lang/rust/pull/72981) which aims to stabilize the `Backtrace` already described that it is desirable to have this type in core.

While `Error` had a valid reason for not being in core (it relies on `Box` type for different conversions), `Backtrace` does not have similar blockers apart from its frame-allocating API.

There are several approaches we can take, each of them having their own drawbacks and advantages. [Current solution](https://github.com/rust-lang/rust/pull/77384) backing this RFC uses `lang_items` which require tight integration with the Rust compiler. Also, this introduces additional requirement on `no_std` users who have to implement additional functions to have a compiling binary. 

Another solution would be to leave the `Backtrace` as it is and instead just wait until the[Generic Member Access RFC](https://github.com/rust-lang/rfcs/pull/2895) gets accepted and merged. This way `Error` could be moved to core on its own and `Backtrace` would be [provided to it only when desired](https://github.com/rust-lang/rfcs/pull/2895/files#diff-bb91f37510dc7a6369183c40dbdcfb52164335efa7a04f005a906d0006754d27R88).

Yet another take on the subject is to store and capture the backtrace in core and do the actual generation in std. This way users could implement their own backtrace generation if they do not wish to use std. This would, however, require some bigger changes in the current implementation, but would not require the lang items.

Additionally, having this type in core will allow its users to provide their own implementations of the backtrace capture and reporting and not rely on the std-provided one if they don't want to.

The outcome of this RFC will be a `Backtrace` type in core with implementation defined in std and compiler-generated implementation when std is not linked and user did not provide their own implementation for the reporting functions.


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Backtraces are an essential part of the Rust ecosystem, being close coupled with error handling and reporting, be it to the user or the developer. They are usually of a common form:
```
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

They allow for a detailed inspection of what crashed at runtime and which libraries and functions were involved in the process. Backtraces act like a map describing the origin of an error within a program's source code. This is useful information when debugging errors but does not itself describe the reason for an error, and so it does not belong in the error message itself. To model this the `Error` trait provides separate methods for retrieving backtraces (`fn backtrace()`) and error messages (`Display`) to provide flexibility when constructing error reports.

Currently, `Backtrace` is part of the std library and users can control whether the backtrace is enabled or disabled using the environmental variable: `RUST_BACKTRACE` and `RUST_LIB_BACKTRACE` - see the [documentation](https://doc.rust-lang.org/std/backtrace/index.html) for differences between them.

More specifically, Rust's `Backtrace` is a struct which is a wrapper over a stack backtrace:
```rust
pub struct Backtrace {
    inner: Inner,
}
```
It can be captured or not, depending on the environment settings:
```rust
/// The current status of a backtrace, indicating whether it was captured or
/// whether it is empty for some other reason.
#[non_exhaustive]
#[derive(Debug, PartialEq, Eq)]
pub enum BacktraceStatus {
    /// Capturing a backtrace is not supported, likely because it's not
    /// implemented for the current platform.
    Unsupported,
    /// Capturing a backtrace has been disabled through either the
    /// `RUST_LIB_BACKTRACE` or `RUST_BACKTRACE` environment variables.
    Disabled,
    /// A backtrace has been captured and the `Backtrace` should print
    /// reasonable information when rendered.
    Captured,
}

enum Inner {
    Unsupported,
    Disabled,
    Captured(LazilyResolvedCapture),
}
```
The `Capture` option of our `Backtrace` looks like this, and contains stack frames:
```rust
struct Capture {
    actual_start: usize,
    resolved: bool,
    frames: Vec<BacktraceFrame>,
}
```
Once captured, it is filled with `BacktraceFrame`s which contain the actual frame and symbols relevant to this frame:
```rust
/// A single frame of a backtrace.
#[unstable(feature = "backtrace_frames", issue = "79676")]
pub struct BacktraceFrame {
    frame: RawFrame,
    symbols: Vec<BacktraceSymbol>,
}
```


Users are interested in utilizing `Backtrace` for several reasons, primarily to inspect or reformat the frames at the point of `Backtrace` generation or to present the frames in some different way to the user. While most for most Rust programmers this type is transparent, people who use the recent versions of nightly even got a new API which returns an [iterator over the captured frames](https://doc.rust-lang.org/src/std/backtrace.rs.html#367). 

Two main groups of users can be distinguished: regular and `no_std`. While former don't care whether this type is in core or in std (it gets re-exported by std), the latter do not have the possibility to use this functionality right now and would be interested in having it available. On the other hand, the consumers of the type are satisfied with simply having the `Backtrace` printed upon panicking and seeing where exactly the program failed.

One uses this API like this:
```rust
#![feature(backtrace)]

fn main() {
    let backtrace = std::backtrace::Backtrace::capture();
    println!("{}", backtrace);
}
```

For inspecting frames one-by-one, we can use this API:
```rust
#![feature(backtrace)]
#![feature(backtrace_frames)]

fn main() {
    let backtrace = std::backtrace::Backtrace::capture();
    for frame in backtrace.frames() {
        println!("{:?}", frame);
    }
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

After this RFC is merged the `Backtrace` type will reside in core and the interfacing lang items will be in std.

## The `Backtrace` implementation in core

The core part of `Backtrace` looks as follows:
```rust
/// The current status of a backtrace, indicating whether it was captured or
/// whether it is empty for some other reason.
#[non_exhaustive]
#[derive(Debug, PartialEq, Eq)]
pub enum BacktraceStatus {
    /// Capturing a backtrace is not supported, likely because it's not
    /// implemented for the current platform.
    Unsupported,
    /// Capturing a backtrace has been disabled through either the
    /// `RUST_LIB_BACKTRACE` or `RUST_BACKTRACE` environment variables.
    Disabled,
    /// A backtrace has been captured and the `Backtrace` should print
    /// reasonable information when rendered.
    Captured,
}

#[unstable(feature = "backtrace", issue = "53487")]
///
pub struct Backtrace {
    ///
    inner: *mut dyn RawBacktrace,
}

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

impl Backtrace {
    fn create(ip: usize) -> Backtrace {
        // SAFETY: trust me
        let inner = unsafe { backtrace_create(ip) };
        Backtrace { inner }
    }

    /// Returns whether backtrace captures are enabled through environment
    /// variables.
    fn enabled() -> bool {
        // SAFETY: trust me
        unsafe { backtrace_enabled() }
    }

    /// Capture a stack backtrace of the current thread.
    ///
    /// This function will capture a stack backtrace of the current OS thread of
    /// execution, returning a `Backtrace` type which can be later used to print
    /// the entire stack trace or render it to a string.
    ///
    /// This function will be a noop if the `RUST_BACKTRACE` or
    /// `RUST_LIB_BACKTRACE` backtrace variables are both not set. If either
    /// environment variable is set and enabled then this function will actually
    /// capture a backtrace. Capturing a backtrace can be both memory intensive
    /// and slow, so these environment variables allow liberally using
    /// `Backtrace::capture` and only incurring a slowdown when the environment
    /// variables are set.
    ///
    /// To forcibly capture a backtrace regardless of environment variables, use
    /// the `Backtrace::force_capture` function.
    #[inline(never)] // want to make sure there's a frame here to remove
    pub fn capture() -> Backtrace {
        if !Backtrace::enabled() {
            return Backtrace::disabled();
        }

        Self::create(Backtrace::capture as usize)
    }

    /// Forcibly captures a full backtrace, regardless of environment variable
    /// configuration.
    ///
    /// This function behaves the same as `capture` except that it ignores the
    /// values of the `RUST_BACKTRACE` and `RUST_LIB_BACKTRACE` environment
    /// variables, always capturing a backtrace.
    ///
    /// Note that capturing a backtrace can be an expensive operation on some
    /// platforms, so this should be used with caution in performance-sensitive
    /// parts of code.
    #[inline(never)] // want to make sure there's a frame here to remove
    pub fn force_capture() -> Backtrace {
        Self::create(Backtrace::force_capture as usize)
    }

    /// Forcibly captures a disabled backtrace, regardless of environment
    /// variable configuration.
    pub const fn disabled() -> Backtrace {
        DisabledBacktrace::create()
    }

    /// Returns the status of this backtrace, indicating whether this backtrace
    /// request was unsupported, disabled, or a stack trace was actually
    /// captured.
    pub fn status(&self) -> BacktraceStatus {
        // SAFETY: trust me
        unsafe { backtrace_status(self.inner) }
    }
}
```

The part of `Backtrace` in std: 
```rust
struct StdBacktrace {
    inner: Inner,
}
```

This struct is used to interface the actual backtraces from the std in the three functions described below.

## The 3 new lang-items

The `std` and `core` sides of the `Backtrace` API would be connected via three new lang items: `enabled()`, `create()` and `status()`. These functions are declared and called from `core` and defined in `std`. `no_std` binaries would need to provide their own implementations of these lang items or conditionally compile `core` to not include `Backtrace` support.

```rust
pub use core::backtrace::Backtrace;
pub use core::backtrace::BacktraceStatus;
use core::backtrace::RawBacktrace;

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

# Drawbacks
[drawbacks]: #drawbacks

The solution proposed by this RFC needs to integrate `Backtrace` implementation tightly with the Rust compiler machinery via usage of `lang_items`. This adds maintenance cost and mental overhead required to remember why this functionality is implemented in such special way. Ideally we would add functionality to the language without edge cases and cutting corners, but it is not always possible (refer to panic hooks implementation). 

Also, moving `Backtrace` to core was met with moderate reluctance by the [#rust-embedded community on Matrix](https://github.com/rust-embedded/wg) because of how the capturing API uses allocating functions ([logs here](https://libera.irclog.whitequark.org/rust-embedded/2021-08-17)).

Current implementation uses a fat pointer for storing the `RawBacktrace` inside the `Backtrace`:
```rust
#[unstable(feature = "backtrace", issue = "53487")]
///
pub struct Backtrace {
    ///
    inner: *mut dyn RawBacktrace,
}
``` 
and this may induce some code bloat that is suboptimal but livable-with. It could be changed via some vtable sheaningans but we will leave it as it is for simplicity's sake.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The proposed solution is the one which is currently implementable. However, if the Generic Member Access RFC was implemented as discussed in the Motivation section, we would not have to move the `Backtrace` to core at all. In the alternative solution, we would leave the `Backtrace` as it is and instead the `Error` trait will provide a `backtrace()` method which will use the Generic Access to extract the concrete `Backtrace` out of the propagated error.

The `Backtrace` functionality could be implemented as a trait in order to be extensible in the future and would allow for backwards compatibility. However, since in the long goal we want to deprecate `Backtrace` in the `Error` trait we do not pursue that idea.

During the [conversation on #rust-embedded IRC](https://libera.irclog.whitequark.org/rust-embedded/2021-08-17), various takes on the matter from the embedded contexts were given. What was most threatening for people engaged in the discussion is the allocating capabilities of `Backtrace`. Though the implementation in std uses `Vec` for allocating backtrace frames, the API declaration in core leaves the implementation to the user (if no std is supplied).

This implementation may look like this: provide the memory in which the backtrace should reside and truncate/report a failure in case the backtrace does not fit this preallocated space. In case the user did not provide providing their `capture()` implementation, there should be a no-op provided by the language. 

There was also an idea of providing general backtrace capturing functions for each family of embedded devices, but that would be too difficult to implement cohesively due to differences in implementations between them. Thus, what is proposed above seems like a valid alternative. 

# Prior art
[prior-art]: #prior-art

This type is already implemented, but it seems like no type was moved from std to core previously so we have no point of reference on this one.

The [`backtrace-rs` crate](https://github.com/rust-lang/backtrace-rs) is a cornerstone of the `Backtrace` type in the std library. It contains all the nitty-gritty details of obtaining the actual backtraces.

As for `no_std` and embedded contexts, there exists the [mini-backtrace](https://github.com/amanieu/mini-backtrace) library that provides backtrace support via LLVM's libunwind.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

Will introducing this implementation be worth in the long term? - `no_std` users can implement their own backtrace capturing mechanisms either way and not care about `Backtrace` in core.

Is this better than Generic Member Access? - This will be answered with either implementing this RFC or dropping it.

# Future possibilities
[future-possibilities]: #future-possibilities
Since the RFC proposes a solution based on `lang_items`, one could wish to implement these functions themselves. We could support such endeavours and provide dummy implementations if the compiler does not see the overrides. This would be implemented via weak linkage (though, unfortunately not all platforms support it). It would be a breaking change, since we would require the linker to be run in `no_std` contexts and currently it is run only on several occasions, therefore this seems like a dead end or at least not worth the effort and overhead.
