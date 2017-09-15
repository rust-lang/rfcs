- Feature Name: debuginfo_based_panic
- Start Date: 2017-09-15
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Instead of generating location strings for panic information, read it at runtime from debug information. To make this work in release builds, introduce symbol files (external debug symbols) which are built in release mode by default.

# Motivation
[motivation]: #motivation

By convention, recoverable errors are usually handled in Rust with the `Result` type while unrecoverable errors (usually bugs) cause a panic. When that happens, it is thus often necessary to figure out exactly what happened so the cause can be fixed.

Today, `panic!()` attempts to help with that by printing a message along with the source code location where it happened:

```
thread 'main' panicked at 'called `Option::unwrap()` on a `None` value', /checkout/src/libcore/option.rs:335
note: Run with `RUST_BACKTRACE=1` for a backtrace.
```

However, this is not very useful. A panic's root cause is rarely located within the panicking function - most commonly, it happened because a function's *preconditions were violated*.

The textbook example for this is of course `unwrap()` (on both `Option` and `Result`) as well as similar interfaces like `Index::index`/`IndexMut::index_mut`. The above example refers to the location in `option.rs` where the panic happened, but the `unwrap()` implementation is just doing its job here - it was really caused by a bug that caused the user application to believe an `Option` has to be `Some` in a case where it's not. When debugging this, the given location information is utterly useless - the `unwrap()` function panicked so of course `panic!()` was called from there!

When dealing with issues like this, one usually enables `RUST_BACKTRACE=1`. This generates a full stack trace with information about where the bug may have originated. When even that isn't enough, a debugger can provide valuable insights into what's really going on.

The main purpose of panic location information as well as backtraces and debuggers is **debugging**: Finding and fixing the bug. Debugging code written in a compiled systems language is a hard problem but Rust is not unique here - the C ecosystem was facing exactly the same problem and debug info (in a platform-specific format like DWARF or PDB) exists to solve it. Of course, Rust is already leveraging this to support both backtraces and debuggers. But from this perspective it certainly seems very redundant to compile dedicated panic location strings when this information already exists in a binary's debug sections.

Furthermore, there is no automated solution right now that could remove panic location strings from binaries. This is a problem for developers of closed-source software as it can, in some circumstances, faciliate reverse engineering.

Because including debug info in release binaries is not viable for various reasons (most notably debug info is often *larger* than code), many platforms already store it in a separate location (`*.pdb` on Windows, `*.dSYM` on MacOS). On Linux and several other unixoid platforms, this approach is still quite new and uncommon but nonetheless perfectly possible. 

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

* The Rust compiler simply continues to generate platform-compatible debug information. It gains a new switch that causes it to generate *external* debug info. What this means is that the binary itself is stripped - all debug info is written to a separate **symbol file** (in a platform-specific format).
* Cargo's release profile enables external debug info.
* Panic messages now always include a backtrace (instead of the old location string).
	* This means that the root cause of panics from `unwrap()` and similar interfaces are now much clearer.
	* It's simple and obvious as most modern languages print backtraces for unhandled errors.
	* The backtrace is generated from debug information. Without debug symbols, memory addresses are printed instead. The standard `addr2line` utility can be used to manually obtain location information once debug info is available.
	* User perspectives:
		* You're developing an application. You build in the `dev` profile. You have debug symbols and thus complete backtraces for every panic.
		* You're testing your application (think QA). It's a `release` build. You have debug symbols and thus complete backtraces for every panic.
		* You ship your application to an end user. You ship only the binary, not the symbol file. The end user runs into a panic and receives a raw stack trace (of memory addresses). The end user now reports the bug to you. Using `addr2line` you obtain a complete backtrace.
		* Alternatively, you ship the symbol file to the user. The user can now debug your application without recompiling and - obviously - gets complete backtraces for every panic.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

**Symbol files** already exist (and work) on Windows and MacOS. On Linux, it's as simple as writing the main binary to one ELF image and all the debug sections to another. The existing `-C debuginfo` compiler options gains a third level "3 = full debug info in an external symbol file". The cargo release profile sets `debuginfo=3` (instead of `debuginfo=0`).

The default panic handler no longer uses the location information supplied by `panic!()`. Instead, it always displays a backtrace. If there is no debug info in the binary itself, it looks for symbol files and loads those (this already works on Windows and on MacOS since [rust-lang/rust#44251](https://github.com/rust-lang/rust/pull/44251), still missing for Linux though).

It does so through an interface in `std::panic` by first obtaining a raw stack trace (basically `Iterator<usize>`) and then requesting symbols for those stack frames. This interface can be very useful for projects like error-chain as well, especially since *obtaining* the raw stack trace is very *cheap*.

# Drawbacks
[drawbacks]: #drawbacks

The single major drawback here is implementation complexity. Walking the stack is usually quite simple but parsing debug info to obtain symbols is not. On the other hand, backtraces already work on most platforms and are even essential to debugging for many users, so most of this is either already solved or worth the effort on its own.

# Rationale and Alternatives
[alternatives]: #alternatives

This is, quite obviously, a counter-proposal to [#2091](https://github.com/rust-lang/rfcs/pull/2091#issuecomment-329148747): Implicit caller location.

Where #2091 is specifically geared towards small helper functions that always want to blame panics on their caller, this proposal focuses on a much broader class of problems. In reality, the lines are blurred: Some functions obviously are small helpers that should be annotated with `#[blame_caller]` and others obviously aren't. But there definitely is a gap where it's not entirely clear where the error should be reported. The approach's big weakness is that it focuses entirely on finding a *single stack frame that takes all the blame*. In the exemplary case of `unwrap()`, printing the location of the call to `unwrap()` instead of the code inside the implementation that ends up panicking is of course so much more useful - but it still doesn't come close to the picture you get from a full backtrace.

While #2091 does acknowledge that relying on debug info is an alternative, its current draft argues:

>Programmatic access to the stack backtrace is often used in interpreted or runtime-heavy languages like Python and Java. However, the stack backtrace is not suitable as the only solution for systems languages like Rust because optimization often collapses multiple levels of function calls. In some embedded systems, the backtrace may even be unavailable!

It is of course possible for the compiler to annotate code from inlined functions with their original source information. In fact, rustc is already capable of doing this today! Missing inlined frames are a bug in the backtracing implementation, not an argument against backtracing in general.

>The debug information is usually not provided in release mode.

Not relevant either as this RFC changes that (with no downsides except maybe slightly longer compile times).

>Even if this is generated, the debug symbols are generally not distributed to end-users, which means the error reports will only contain numerical addresses. This can be seen as a benefit, as the implementation detail won't be exposed, but how to submit/analyze an error report would be out-of-scope for this RFC.

Again, the C ecosystem has solved this exact same issue many years ago:

```
$ addr2line -Cipf -e target/release/tyrion -a 0x7e9e 0x7fe3 0x6ec5 0x20381 0x6b54 0x6d84 0x50b0b
0x0000000000007e9e: backtrace::backtrace::libunwind::trace at /home/.cargo/registry/src/github.com-1ecc6299db9ec823/backtrace-0.3.2/src/backtrace/libunwind.rs:53
 (inlined by) backtrace::backtrace::trace<closure> at /home/.cargo/registry/src/github.com-1ecc6299db9ec823/backtrace-0.3.2/src/backtrace/mod.rs:42
0x0000000000007fe3: backtrace::capture::{{impl}}::new at /home/.cargo/registry/src/github.com-1ecc6299db9ec823/backtrace-0.3.2/src/capture.rs:64
0x0000000000006ec5: tyrion::main::{{closure}} at /overtime/git/tyrion/src/main.rs:41
0x0000000000020381: ?? ??:0
0x0000000000006b54: std::panicking::begin_panic<&str> at /build/rust/src/rustc-1.19.0-src/src/libstd/panicking.rs:511
0x0000000000006d84: tyrion::test at /overtime/git/tyrion/src/main.rs:12
 (inlined by) tyrion::main at /overtime/git/tyrion/src/main.rs:81
0x0000000000050b0b: malloc_usable_size at ??:?
```

It **is** a benefit, no matter how you look at it.

>There are multiple issues preventing us from relying on debug info nowadays.
>
>[...]
>
>These signal that debuginfo support is not reliable enough if we want to solve the unwrap/expect issue now.

Implementation complexity (which includes fixing critical issues) is obviously this RFC's big drawback. But #2091 is very complex as well! Programmers would have to remember to annotate every little helper function *everywhere* with `#[blame_caller]`, at the price of sacrificing debuggability *inside* those functions. All of that - for what? Again, all of these practical conerns are directly caused by the focus on printing just *one single stack frame* instead of a trace.

Dedicated panic location strings are not a zero-cost abstraction in Rust's traditional sense - they redundantly reproduce information that already has a much better home somewhere else: In the debug sections.


# Unresolved questions
[unresolved]: #unresolved-questions

* What exactly should the new API in `std::panic` look like?
* What about the `no_std` world?
