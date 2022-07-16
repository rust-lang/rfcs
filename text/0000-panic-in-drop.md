- Feature Name: `panic_in_drop`
- Start Date: 2022-07-06
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Panics which happen inside a `Drop` impl currently unwind as long as the drop was not itself called during unwinding (which would otherwise abort immediated due to a double-panic). This RFC proposes to change this behavior to always abort if a panic attempts to escape a drop, even if this drop was invoked as part of normal function execution (as opposed to unwinding which already aborts with a double-panic).

# Motivation
[motivation]: #motivation

## Exception safety

Unsafe code must be [exception-safe](https://doc.rust-lang.org/nomicon/exception-safety.html). That is, it must ensure that it does not expose any potential unsoundness to safe code even when an unwinding panic occurs in the middle of it. This is particularly tricky to handle when the unsafe call must call user-provided functions that may panic, usually via trait methods. This is *exceptionally* tricky in the case of panics caused from dropping user-provided types since drops are usually invisible and implicit in the source code.

Forgetting to take panics from drop into account has been the source of several bugs in popular crates and even the standard library:
- Double drop in the standard library: https://github.com/rust-lang/rust/issues/83618
- Double drop in `arrayvec`: https://github.com/bluss/arrayvec/issues/3
- The exact same double-drop bug in `smallvec`: https://github.com/servo/rust-smallvec/issues/14

### `catch_unwind`

One particularly nasty case of missed exception safety is [rust-lang/rust#86027](https://github.com/rust-lang/rust/issues/86027).

Code using `catch_unwind` is not typically prepared to handle an object that panics in its `Drop` impl. Even the standard library has had serious bugs in this regard, and if the standard library doesn't consistently get it right, we can hardly expect others to do so.

`catch_unwind(code)` is often used to make sure no panics from `code` can cause further unwinding/panics. However, when catching a panic with a payload that panics on Drop, most usages of `catch_unwind(code)` will still result in further unwinding and often unsoundness.

Example in the standard library (issue [rust-lang/rust#86030](https://github.com/rust-lang/rust/issues/86030)):

```rust
struct Bomb;

impl Drop for Bomb {
    fn drop(&mut self) {
        panic!();
    }
}

std::panic::panic_any(Bomb);
```

https://github.com/rust-lang/rust/blob/5ea19239d9d6f49fdd76513a36386d7e83708e3f/library/std/src/rt.rs#L34-L39

```rust
fn main() {
    std::panic::panic_any(Bomb);
}
```
```
thread 'main' panicked at 'Box<Any>', src/main.rs:12:5
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
thread 'main' panicked at 'explicit panic', src/main.rs:7:9
fatal runtime error: failed to initiate panic, error 5
abort (core dumped)
```

### Non-obvious semantics of failing drops

The exact semantics of a panicking drop are not obvious because it is often unclear what it means for a destructor to fail. The exact rules of panicking inside the `drop` function of an object are:
- Execution of the rest of `fn drop` is skipped, just like a normal panic in a function.
- The fields of the object are still dropped.
- If the object was a field in an outer object, then the remaining fields of the outer objects are dropped.
- If the object was an element in a slice/array, then the remaining elements are dropped.
- If another panic occurs in any of the above then execution aborts due to a double panic.

The first point often causes memory leaks when the `drop` function is responsible for freeing memory used by an object. For example, this is the case for `LinkedList`, `HashMap`, etc.

The second point is subtle and makes it easy to accidentally introduce unsoundness if the drop object fields depends on some operation in the outer `drop` for soundness.

## Memory leaks

The Rust standard library is written to a high standard of code quality. In particular, it aims to never leak memory, even in the presence of panics from user code (e.g. a user-provided `Hash` impl). There are only 2 circumstances where the standard library may leak memory:
- If the user leaks a `Drain` iterator then all elements in the collection being drained may be leaked due to [leak amplification](https://doc.rust-lang.org/nomicon/leaking.html#drain). In practice this is not a problem since it is difficult to accidentally leak an object in Rust.
- Any panics from drops in objects owned by a collection such as `Vec`, `HashMap`, `LinkedList`, `BTreeMap` may cause all remaining elements and any memory used by the collection to be leaked.

Writing exception-safe code is difficult because objects need to be rolled back to a safe state. Avoiding calling user code during this rollback process is crucial since this could cause another panic and prevent the rollback from completing. Unfortunately such rollbacks often involve dropping user defined types, which is outside the control of the author and could potentially panic.

Consider the `Drop` impl for `LinkedList`: if a `Drop` call for an element in the list panics, there are few good options:
- it could continue attempting to drop other elements, risking a double panic (this is what `Vec` does).
- it could abort the drop and leak all remaining elements (this is what `LinkedList` does).
- it could give up and abort the process.

Although accepting this RFC would effectively force the third option, this will encourage users to be more careful about potential panics in their `Drop` impls, which should improve overall reliability.

## Code size & compilation time

This RFC is already implemented as an unstable compiler flag which controls the behavior of panics escaping from a `Drop` impl.

Performance results from [rust-lang/rust#88759](https://github.com/rust-lang/rust/pull/88759) show up to 10% reduction in compilation time ([perf](https://perf.rust-lang.org/compare.html?start=c9db3e0fbc84d8409285698486375f080d361ef3&end=1f815a30705b7e96c149fc4fa88a98ca04e2deee)) and a 5MB (3%) reduction in the size of `librustc_driver.so`.

As another example, the `ripgrep` binary size is reduced by 5% when compiling with `-Z panic-in-drop=abort`:
```
-Z panic-in-drop=unwind: 4242864 bytes
-Z panic-in-drop=abort: 4033968 bytes
```

The main reason for the code size increase is that rustc needs to insert landing pads around every drop call to handle unwinding from drops. As an example, here's how rustc currently expands implicit drop calls and how it would be expanded if this RFC is accepted:

```rust
// Codegen with -Z panic-in-drop=unwind (old behavior)
unsafe fn drop_in_place<T>(ptr: *mut T) {
    // Call the Drop impl if there is one.
    try {
        <T as Drop>::drop(&mut *ptr);
    } catch {
        goto 'unwind_drop_field1;
    }

    // Drop the first field.
    try {
        drop_in_place(&mut (*ptr).field1);
    } catch {
        // If dropping field1 panics the keep trying to drop the remaining fields.
        goto 'unwind_drop_field2;
    }

    // Drop the second field.
    try {
        drop_in_place(&mut (*ptr).field2);
    } catch {
        // If dropping field2 panics the keep trying to drop the remaining fields.
        goto 'unwind_drop_field3;
    }

    // Drop the third field.
    drop_in_place(&mut (*ptr).field3);
    return;

    // Unwind path
    try {
        'unwind_drop_field1:
        drop_in_place(&mut (*ptr).field1);
        'unwind_drop_field2:
        drop_in_place(&mut (*ptr).field2);
        'unwind_drop_field3:
        drop_in_place(&mut (*ptr).field3);

        // Resume unwinding in the parent function. This doesn't return.
        resume_unwind();
    } catch {
        // Double-unwinds force an abort. panic_no_unwind is a lang item
        // (not exposed in the public API) which calls the panic hook to print
        // a message and then aborts instead of unwinding the stack.
        std::panicking::panic_no_unwind();
    }
}

// Codegen with -Z panic-in-drop=abort (new behavior)
// Also, this function is marked as `nounwind` for LLVM optimizations.
unsafe fn drop_in_place<T>(ptr: *mut T) {
    // Call the Drop impl if there is one.
    try {
        <T as Drop>::drop(&mut *ptr);
    } catch {
        // If the Drop impl panics then abort the process.
        std::panicking::panic_no_unwind();
    }

    // Drop the fields: no unwind guards are needed since drop_in_place never
    // unwinds.
    drop_in_place(&mut (*ptr).field1);
    drop_in_place(&mut (*ptr).field2);
    drop_in_place(&mut (*ptr).field3);
}
```

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This RFC changes how panics behave when called from within a `Drop` implementation. Previously, a panicking `Drop` would behave differently depending on whether another panic was currently in the process of unwinding the stack.

- If it was, then the process was immediately aborted with a double-panic error.
- Otherwise, the panic would:
  - Skip the rest of the `Drop` impl.
  - Continue dropping the fields of the object with the panicking `Drop` impl.
  - If that object was itself part of a `struct` or slice then the remaining fields/slice elements are also dropped.

This behavior was non-obvious and error-prone. It has been the source of several security bugs in unsafe code both [inside](https://github.com/rust-lang/rust/issues/83618) and [outside](https://github.com/bluss/arrayvec/issues/3) the standard library.

With this RFC the behavior of panics in `Drop` is changed to always abort the process. This mirrors a [similar change](https://akrzemi1.wordpress.com/2013/08/20/noexcept-destructors/) made in C++11 where destructors terminate the process if an exception escapes them.

If you have any existing code which relies on panics in `Drop` not aborting the process, you should refactor your code to return errors using a method that consumes `self`. For example consider this hypothetical `close` method on `BufWriter`:

```rust
/// Simplified BufWriter that just works for files.
struct BufWriter {
    file: File,
    buf: Vec<u8>,
}

impl BufWriter {
    pub fn flush(&mut self) -> std::io::Result<()> {
        ...
    }

    /// Closes the underlying file and reports any errors that occurred while
    /// flushing the buffer.
    ///
    /// Note that the underlying file is still closed even if I/O errors
    /// occurred.
    pub fn close(mut self) -> std::io::Result<()> {
        // Write the remaining data to the file.
        let err = self.flush();

        // Skip the normal Drop impl to avoid double-flushing in case of an
        // error.
        std::mem::forget(self);

        err
    }
}

impl Drop for BufWriter {
    fn drop(&mut self) {
        // Write the remaining data to the file but ignore any errors.
        let _ = self.flush().
    }
}
```

Note that it is still possible to catch panics inside a `Drop` impl using `catch_unwind`. It's when the panic attempts to unwind out of the `drop` function that the process is aborted.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The basic functionality behind this RFC is already implemented as an unstable compiler flag `-Z panic-in-drop={abort,unwind}`. This RFC proposes to change the default to `abort` instead of `unwind`.

This flag modifies the behavior of the `drop_in_place<T>` intrinsic function, which is responsible for calling the `drop` function of a type (if it has one) and then recursively calling `drop_in_place` for all its fields. Specifically:
- rustc recognizes `drop_in_place` as a function that can never unwind and therefore avoids generating unnecessary landing pads when calling it.
- Within `drop_in_place`, rustc inserts an abort guard to catch any unwinds escaping a call to `Drop::drop` to turn them into aborts by calling the `panic_no_unwind` lang item.
- The `drop_in_place` function is marked with the `nounwind` LLVM attribute, which allows LLVM to optimize away unnecessary landing pads.

 The `panic_no_unwind` lang item is implemented in `core::panicking::panic_no_unwind` which is a private API. This function calls the panic handler with a special `PanicInfo` that indicates that the panic handler should not unwind (via `PanicInfo::can_unwind`). The `std` panic handler will call the panic hook as normal (which by default prints a message to stderr with a backtrace) but will then abort the process instead of initiating unwinding.

One important restriction is that all crates that are linked together must have the same `panic-in-drop` setting. This is necessary so that the compiler can assume that `drop_in_place<T>` never unwinds, even for `T` types defined in other crates. This is enforced by the compiler.

# Drawbacks
[drawbacks]: #drawbacks

## Deferring execution

A common pattern is to use `Drop` to defer the execution of arbitrary code until the end of a scope. One example of this is the `scopeguard` crate which provides a `defer!` macro. With this RFC, if code within a `defer!` panics then this would abort the process.

Already today, panicking in a `defer!` has the downside of causing a double-panic if the deferred code is invoked as part of an unwind. This is already a problem today: for example, rustc's `delay_span_bug` will defer a panic until the end of a session and only emit it if no other errors occurred. However this can sometimes lead to the compiler double-panicking if rustc is already unwinding due to a previous `span_bug` or `panic`. However if this RFC is accepted then this problem might become more frequent since they will occur in the normal execution path.

## Reporting errors when dropping

Some types can trigger error conditions when dropped. Consider the example of `BufWriter` which needs to flush its buffer when dropped, but where the writing to the underlying `Writer` is a potentially faillible operation. Since it is not possible to "cancel" a drop and return an error, this leaves only 2 possibilities: panic or ignore the error.

The preferred approach is the one used by `BufWriter`: it ignores errors when dropping. If precise error reporting is required then the user can explicitly flush the `BufWriter` and report any errors before dropping the buffer.

## Reliability

Some programs aim to keep running in the face of panics. One example of this is a web server where a panic when servicing one request should not be able to take down the entire server. This works fine with panics outside `Drop`: unwinding will release any temporary resources used by the request and control will return to a `catch_unwind` at the root of the request. In the rare case of a panic from within a `Drop` this may lead to memory leaks (due to incomplete drops) or aborts if the `Drop` was already called due to an unwind. This RFC would make these rare cases always abort the entire program, which could lead to DoS if the panic can be triggered remotely.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Stabilize `-Z panic-in-drop`

A way to restore the existing behavior could be stablized as a compiler flag. However this would run counter to the goal of improving exception safety in unsafe code: if it is possible for safe code to cause a panic to unwind out of a `Drop` impl then all unsafe code needs to properly handle this case.

## Add a third `panic-in-drop` panic model

This is similar to the compiler flag, but also introduces an additional complication: crates compiled with `-Z panic-in-drop=unwind` cannot be linked with crates compiled with `-Z panic-in-drop=abort` because they disagree on whether a drop can unwind. This new panic model would be incompatible with a pre-built `std` distributed by rustup and would require rebuilding the standard library with `-Z build-std` (which is also unstable).

## Allow individual `Drop` impls to opt-in to unwinding

Rather than a global flag, an individual `Drop` impl could be marked with an attribute to indicate that it may allow unwinds to escape it. This would require the `Drop` to be an `unsafe impl` to achieve the exception safety goal of this RFC: most unsafe code should not have to worry about about unwinding drops. It is the responsability of the user of this type to ensure that it is only used with APIs that explicitly support unwinding drops.

This mirrors the behavior of C++ where a throwing destructor can be opted into using `noexcept(false)` but actually throwing from such a destructor is UB if that object is stored in any standard library container.

Unfortunately this doesn't really fit Rust's safety model: such a type would have to be unsafe to create and use since it would be easy to invoke UB by using this type with an API that doesn't support unwinding drops.

## Allow individual `Drop` impls to opt-out of unwinding

This is a reverse of the previous point: the default behavior of drops could stay as it is, but individual `Drop` impls could opt-in to aborting if an unwind escapes it.

However this doesn't help generic code such as collections which still need to be able to handle user-defined types which do abort on drop.

## Only abort for unwinds out of implicit drops

This would still allow unwinds to escape calls to `drop_in_place`. Abort guards would be generated for implicit drops at the end of a scope.

## Only address the `catch_unwind` issue

Several solutions have been proposed ([disabling unwinding for unwind payloads](https://github.com/rust-lang/rust/pull/99032), [`drop_unwind`](https://github.com/rust-lang/rust/pull/85927), [`catch_unwind_v2`](https://internals.rust-lang.org/t/some-thoughts-on-a-less-slippery-catch-unwind/16902)) to specifically address the [issue](https://github.com/rust-lang/rust/issues/86027) with `catch_unwind`. However these increase API complexity and do not address the remaining issues.

## Add a lint to warn about implicit drops

As a tool for developers of unsafe code, an allow-by-default lint could be added to warn about implicit drop calls made in a function.

## Add explicit language support for `defer!`

If this RFC is accepted then `defer!` from the `scopeguard` crate (and its variants `defer_on_success!` and `defer_on_unwind!`) could be modified to use new language support instead of `Drop` impls. This has several advantages:
- These could be allowed to unwind even though normal drops would not.
- `defer_on_success!` and `defer_on_unwind!` would no longer need to rely on `std::thread::panicking` to determine whether an unwind is in progress. `std::thread::panicking` is quite slow due to a TLS access and does not take unwinding from foreign exceptions into account.

# Prior art
[prior-art]: #prior-art

## C++11 `noexcept` destructors

A similar change was done in C++11: destructors were changed to be `noexcept` by default unless explicitly opted into using `noexcept(false)`. However actually throwing an exception from a destructor is UB if that object is owned by a standard library container such as `std::vector` (this was also the case before C++11). Thus throwing from a destructor is only safe if done from a local object or one which is contained within a custom user-defined type.

The rationale for C++ making this change is similar to that for this RFC: it eliminates many surprising edge cases which can be a source of undefined behavior and also improves code generation. However the issue of exception safety is much more severe in the case of C++ since it is often impossible to make code exception-safe in the face of throwing destructors due to C++'s non-destructive move semantics and potentially throwing move constructors.

## Finalization/destruction in other languages

- [JDK 18 `Object#finalize`](https://docs.oracle.com/en/java/javase/18/docs/api/java.base/java/lang/Object.html#finalize()):
  > **Deprecated, for removal: This API element is subject to removal in a future version.** _Finalization is deprecated and subject to removal in a future release._ [...] If an uncaught exception is thrown by the finalize method, the exception is ignored and finalization of that object terminates. [...] Any exception thrown by the `finalize` method causes the finalization of this object to be halted, but is otherwise ignored. [...] Finalizer invocations are not automatically chained, unlike constructors. If a subclass overrides `finalize` it must invoke the superclass finalizer explicitly.

  Unwinds from finalization are silently discarded. No finalization of an object is done other than the single `Object#finalize` call. See also [JEP 421](https://openjdk.org/jeps/421) which deprecated finalization for removal and discusses the issues / alternatives.

- [Python 3 `object.__del__(self)`](https://docs.python.org/3/reference/datamodel.html#object.__del__):
  > If a base class has a [__del__()](https://docs.python.org/3/reference/datamodel.html#object.__del__) method, the derived class’s [__del__()](https://docs.python.org/3/reference/datamodel.html#object.__del__) method, if any, must explicitly call it to ensure proper deletion of the base class part of the instance. [...] Due to the precarious circumstances under which [__del__()](https://docs.python.org/3/reference/datamodel.html#object.__del__) methods are invoked, exceptions that occur during their execution are ignored, and a warning is printed to `sys.stderr` instead.

  Unwinds from finalization are discarded after printing a warning message to stderr. No finalization of an object is done other than the single `object.__del__(self)` call.

- [Swift `deinit`](https://docs.swift.org/swift-book/LanguageGuide/Deinitialization.html):
  > Superclass deinitializers are inherited by their subclasses, and the superclass deinitializer is called automatically at the end of a subclass deinitializer implementation.

  Note that Swift is not a tracing GC; `deinit` is deterministically called when the [all strong references are released](https://docs.swift.org/swift-book/LanguageGuide/AutomaticReferenceCounting.html#ID50). Via testing, it looks like it is *impossible* to declare `deinit throws`, and any attempt to `throw` will thus be an error.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

# Future possibilities
[future-possibilities]: #future-possibilities
