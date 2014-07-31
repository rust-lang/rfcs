- Start Date: 2014-07-24
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Split out OS-specific bits in the Rust standard library into its own crate, with well-specified interfaces that can be implemented for freestanding targets, as well as for new OS targets.

# Motivation

Currently, OS-specific bits in the standard library are scattered throughout many crates. The `rustrt` crate in particular contains a substantial amount of OS-specific bits in `imp` modules. For example, the `rustrt::mutex::imp` module defines the interface for OS-specific mutexes. This scatter makes bringing the standard library to a new operating system challenging. A quick search for the use of `#[cfg(target_os = "...")]` returns at least 9 crates excluding compiler-internals (`alloc`, `backtrace`, `green`, `libc`, `native`, `rustrt`, `rustuv`, `std`, `time`). It is worth noting that the use of this configuration condition does not explicitly imply that this is something that a platform abstraction crate would better solve.

While this makes bringing Rust to a platform with a reasonable amount of POSIX support reasonably doable, this makes using Rust in a freestanding environment truly daunting. I have worked around this in my own project, [Rustic](https://github.com/miselin/rustic), by using the `*-unknown-linux-gnu` target and writing `pthread_*` glue. It is not practical, nor reasonable, to expect developers writing freestanding Rust code to reimplement the entire `rustrt` crate; the duplication of effort here is wasteful.

The expected outcome of this work is to provide a central crate for OS-specific bits that offers stable abstractions to the rest of the Rust standard library. These abstractions can then be implemented by developers who want the standard library on their freestanding project (ie, defining their own implementations rather than pretending that they are Linux) or for a new target OS (eg, Solaris).

# Detailed design

In order to achieve this goal, the addition of a new crate `platform` is proposed. This crate will:
* Define abstractions that can be used by the rest of the Rust standard library to access OS-specific functionality,
* Be well-documented, such that a developer can implement this crate for a new OS with relative ease, and
* Be the single source for all OS-specific functionality across the standard library, and
* Provide a layer of glue between the Rust standard library and runtime, and the operating system itself.

This crate will *not*:
* Offer actual implementations of the features it is abstracting; the crate is merely a layer between the standard library and the target system.

This will require moving things like `rustrt::mutex::imp` to, for exmaple, `platform::mutex`. Additionally, further work may be necessary in the `libc` crate to make a distinction between architecture-specific definitions (which should remain in the `libc` crate), and OS-specific definitions (which should be re-exported by the `libc` crate).

It is expected that, for a new system to fully support the Rust runtime, work will need to be done in `platform`, `libc`, and `native`, at a minimum. This does not require any modifications to the Rust language, but requires significant work across numerous crates in the standard library.

For UNIXy or Windowsy configurations (`#[cfg(unix)]`, `#[cfg(windows)]`), the standard `platform` crate should be able to provide standard interfaces (eg, pthreads) to avoid duplicating code for POSIX-compatible targets.

This RFC does not propose to remove every `#[cfg(unix)]` or `#[cfg(windows)]` module/function/declaration from the standard library, but rather to remove as many `#[cfg(target_os = X)]`s from the standard library as possible. Targets generally fall under either a UNIXish or Windowsish banner; this proposal is directly concerned with the implementation specifics between different operating systems within these global categories. Freestanding targets may have custom system libraries (or none at all), but can implement stubs for the needed functionality. In general only the supporting functions for what is actually used are required, which means a freestanding developer can elect to simply avoid certain modules that would otherwise pull in dependencies on a system library.

Initial work at developing a proof-of-concept can be found in [my Rust fork](https://github.com/miselin/rust/compare/create-os-crate).

Notes about the POC:
* `rustrt::args::imp` remains, as `args` is only valid on Linux anyway.
* Separating `native::io` from `native` requires the crate for `io` to go into to have access to `std`. Also, `native::io` uses `native::task`.
* `std::os` depends on `sync` (which implicitly depends on `platform` via `rustrt`).
* `std::rt::backtrace::imp` submodules depend on `std` and `rustrt`, and have `target_os` checks.
* `std::rand::os` depends on `std`. It behaves differently for iOS (`target_os` check), but is otherwise specific to Windows/UNIX.
* `std::rtdeps` could perhaps move OS-speciifc bits to `platform` to allow that crate to define the correct `#[link]` attributes for the platform, if that is supported. It is using `target_os` to decide which libraries are needed to link against.
* `librustc::metadata::filesearch` and `librustc_back` have not had any platform-specific parts moved to `platform`.

The following are checking for Windows/UNIX rather than specific operating systems, and are therefore OK as per this proposal. Several of these have `target_os` checks in tests, but the general implementation of the module does not special-case based on the OS.
* `std::dynamic_lib::dl` submodules depend on `std`, but use Windows/UNIX checks rather than a specific OS.
* `std::num::{f64,f32}` have `#[cfg(unix)]` and `#[cfg(windows)]` around pulling in `lgamma_r` (OK as per above).
* `std::io::process` has quite a few `#[cfg(unix)]` and `#[cfg(windows)]` in it, including on function-local variable declarations (OK as per above).
* `std::io::fs::from_rtio()` defines `stat()`'s file mode type as a UNIX/Windows-specific local (OK as per above).
* `std::io::IoError::from_errno()`'s `get_err` would be easy to move to `platform`, if `IoErrorKind` was also moved. Currently using UNIX/Windows `#[cfg]` so OK as per above.
* `std::path` could move its UNIX/Windows-specific type definitions to `platform`, but not using `target_os`, so OK as per above.
* `rustdoc::flock` has UNIX specifics in `platform` now, but Windows specifics in `rustdoc::flock`, because the former specializes based on OS.

The outcome of this POC indicates that, with the `platform` crate, to bring the standard library to a new target the following work must be done.
* Modify `platform` to add OS-specific glue (`platform::mutex`, `platform::stack`, `platform::thread`, `platform::thread_local_storage`, `platform::time`, `platform::libunwind`, `platform::unwind`, and `platform::flock`).
* Modify `libc` to add OS-specific structs and definitions (this is something that could potentially be done with automation of some sort).
* Implement needed bits in `native::io`.
* Implement needed bits in `green` (`green::stack`, `green::sched`, `green::context`).
* Implement needed bits in `std` (touches `std::os`, `std::rt::backtrace`, `std::rand`, `std::rtdeps`, at a minimum). If the system is not compatible with existing Windows/UNIX checks in `std`, extra checks for `target_os` will need to be added to more modules.
* Implement needed bits in `alloc`, if needed.
* Check bindings for `libuv` (`rustuv`), `libgraphviz`, `libminiz`, etc...

# Drawbacks

One of the most significant arguments against doing this is that it involves substantial changes to the standard library and may result in some duplication of code.

Moving things into a new crate can create unexpected circular dependencies that can be challenging to break.

For freestanding targets, the new crate may redirect to Rust functions implemented in the main freestanding application's crate. However, none of these functions will be able to safely use the Rust standard library, as it is not always clear whether calling into the standard library will cause a recursive call back into the platform-specific crate. This is a considerable drawback as the effect of such recursion is seen at runtime. It is possible that this could be resolved by allowing some functions to be marked with some sort of `no_recurse` attribute (and using LLVM's function call graph analysis), but this is a brittle way of working around the problem.

# Alternatives

An alternative that was considered was adding a specific 'freestanding' target that uses a `freestanding` crate. Not doing this provides a much better environment for the creation of new targets for the standard library. The concept of being able to add support for Solaris, for example, by writing code in only one crate, makes the design in this RFC far more usable and flexible than the `freestanding` crate idea.

Another alternative that has been considered has been to define a set of C APIs that Rust calls out to to complete its OS-specific operations. For example, `opaque *rust_create_os_mutex();`. However, this adds another non-Rust dependency, reducing the appeal of Rust (that is, why use Rust if you have to write a ton of C to use it?).

# Unresolved questions

* How is the freestanding target recursion issue solved?
* Concerns about code duplication are not mentioned or addressed here.