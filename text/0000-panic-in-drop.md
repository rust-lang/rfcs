- Feature Name: `panic_in_drop`
- Start Date: 2022-07-06
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Panics which happen inside a `Drop` impl currently unwind as long as the drop was not itself called during unwinding (which would otherwise abort immediated due to a double-panic). This RFC proposes to change this behavior to always abort if a panic attempts to escape a drop, even if this drop was invoked as part of normal function execution (as opposed to unwinding which already aborts with a double-panic).

# Motivation
[motivation]: #motivation

There are two primary motivations for making this change: improving code size & compilation speed, and making it easier to write exception-safe unsafe code.

## Exception safety

Unsafe code must be [exception-safe](https://doc.rust-lang.org/nomicon/exception-safety.html). That is, it must ensure that it does not expose any potential unsoundness to safe code even when an unwinding panic occurs in the middle of it. This is particularly tricky to handle when the unsafe call must call user-provided functions that may panic, usually via trait methods. This is *exceptionally* tricky in the case of panics caused from dropping user-provided types since drops are usually invisible and implicit in the source code.

Forgetting to take panics from drop into account has been the source of several bugs in popular crates and even the standard library:
- Double drop in the standard library: https://github.com/rust-lang/rust/issues/83618
- Double drop in `arrayvec`: https://github.com/bluss/arrayvec/issues/3
- The exact same double-drop bug in `smallvec`: https://github.com/servo/rust-smallvec/issues/14

### `catch_unwind`

One particularly nasty case of missed exception safety is [rust-lang/rust#86027](https://github.com/rust-lang/rust/issues/86027).

Code using `catch_unwind` is not typically prepared to handle an object that panics in its `Drop` impl. Even the standard library has had rious bugs in this regard, and if the standard library doesn't consistently get it right, we can hardly expect others to do so.

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

The exact semantics of a panicking drop are not obvious because it is often unclear what it means for a destructor to fail. The exact rules of panicking inside the `drop` function of an object seem to be:
- Execution of the rest of `fn drop` is skipped, just like a normal panic in a function.
- The fields of the object are still dropped.
- If the object was a field in an outer object, then the remaining fields of the outer objects are dropped.
- If the object was an element in a slice/array, then the remaining elements are dropped.
- If another panic occurs in any of the above then execution aborts due to a double panic.

The first point often causes memory leaks when the `drop` function is responsible for freeing memory used by an object. For example, this is the case for `LinkedList`, `HashMap`, etc.

The second point is subtle and makes it easy to accidentally introduce unsoundness if the drop object fields depends on some operation in the outer `drop` for soundness.

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
fn original() {
    let a = Box::new(1);
    let b = Box::new(2);
    some_function_that_may_panic();
}

fn expanded() {
    let a = Box::new(1);
    let b = try {
        Box::new(2)
    } catch {
        try {
            drop(a);
        } catch {
            abort();
        }
        resume_unwind();
    };

    try {
        some_function_that_may_panic();
    } catch {
        try {
            drop(a);
            drop(b);
        } catch {
            abort();
        }
        resume_unwind();
    }

    try {
        drop(a);
    } catch {
        try {
            drop(b);
        } catch {
            abort();
        }
        resume_unwind();
    }

    drop(b);
}

fn expanded_without_drop_unwind() {
    let a = Box::new(1);
    let b = try {
        Box::new(2)
    } catch {
        drop(a);
        resume_unwind();
    };

    try {
        some_function_that_may_panic();
    } catch {
        drop(a);
        drop(b);
        resume_unwind();
    }

    drop(a);
    drop(b);
}
```

Even if the exception safety concerns can be dismissed as "just buggy unsafe code", the code size and compilation time benefits alone should justify making this change.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This RFC changes how panics behave when called from within a `Drop` implementation. Previously, a panicking `Drop` would behave differently depending on whether another panic was currently in the process of unwinding the stack.

- If it was, then the process was immediately aborted with a double-panic error.
- Otherwise, the panic would:
  - Skip the rest of the `Drop` impl.
  - Continue dropping the fields of the object with the panicking `Drop` impl.
  - If that object was itself part of a `struct`, array or `Vec` then the remaining fields/array elements are also dropped.

This behavior was non-obvious and error-prone. It has been the source of several security bugs in unsafe code both [inside](https://github.com/rust-lang/rust/issues/83618) and [outside](https://github.com/bluss/arrayvec/issues/3) the standard library.

With this RFC the behavior of panics in `Drop` is changed to always abort the process. This mirrors a [similar change](https://akrzemi1.wordpress.com/2013/08/20/noexcept-destructors/) made in C++11 where destructors terminate the process if an exception escapes them.

If you have any existing code which relies on panics in `Drop` not aborting the process, you should refactor your code to return errors using a method that consumes `self`. For example consider this hypothetical `close` method on `BufWriter`:

```rust
impl BufWriter {
    /// Closes the underlying file and reports any errors that occurred while
    /// flushing the buffer.
    ///
    /// Note that the underlying file is still closed even if I/O errors
    /// occurred.
    pub fn close(self) -> std::io::Result<()> {
        let err = self.flush();
        drop(self);
        err
    }
}
```

Note that it is still possible to catch panics inside a `Drop` impl using `catch_unwind`. It's when the panic attempts to unwind out of the `drop` function that the process is aborted.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The basic functionality behind this RFC is already implemented as an unstable compiler flag `-Z panic-in-drop={abort,unwind}`. This RFC proposes to change the default to `abort` instead of `unwind`.

This flag modifies the behavior of the `drop_in_place<T>` intrinsic function, which is responsible for calling the `drop` function of a type (if it has one) and then recursively calling `drop_in_place` for all its fields. Specifically:
- rustc recognizes `drop_in_place` as a function that can never unwind and therefore avoids generating unnecessary landing pads when calling it.
- Within `drop_in_place`, rustc inserts an abort guard to catch any unwinds escaping a call to `Drop::drop` to turn them into aborts.
- The `drop_in_place` function is marked with the `nounwind` LLVM attribute, which allows LLVM to optimize away unnecessary landing pads.

This is best shown in this pseudo-rust code:

```rust
// Codegen with -Z panic-in-drop=unwind (old behavior)
unsafe fn drop_in_place<T>(ptr: *mut T) {
    // Call the Drop impl if there is one.
    try {
        <T as Drop>::drop(&mut *ptr);
    } catch {
        // If the Drop impl panics the keep trying to drop fields.
        try {
            drop_in_place(&mut (*ptr).field1);
            drop_in_place(&mut (*ptr).field2);
            drop_in_place(&mut (*ptr).field3);
        } catch {
            abort();
        }
    }

    // Drop the first field.
    try {
        drop_in_place(&mut (*ptr).field1);
    } catch {
        // If dropping field1 panics the keep trying to drop the remaining fields.
        try {
            drop_in_place(&mut (*ptr).field2);
            drop_in_place(&mut (*ptr).field3);
        } catch {
            abort();
        }
    }

    // Drop the second field.
    try {
        drop_in_place(&mut (*ptr).field2);
    } catch {
        // If dropping field2 panics the keep trying to drop the remaining fields.
        try {
            drop_in_place(&mut (*ptr).field3);
        } catch {
            abort();
        }
    }

    // Drop the third field.
    drop_in_place(&mut (*ptr).field3);
}

// Codegen with -Z panic-in-drop=abort (new behavior)
// Also, this function is marked as `nounwind` for LLVM optimizations.
unsafe fn drop_in_place<T>(ptr: *mut T) {
    // Call the Drop impl if there is one.
    try {
        <T as Drop>::drop(&mut *ptr);
    } catch {
        // If the Drop impl panics then abort the process.
        abort();
    }

    // Drop the fields: no unwind guards are needed since drop_in_place never
    // unwinds.
    drop_in_place(&mut (*ptr).field1);
    drop_in_place(&mut (*ptr).field2);
    drop_in_place(&mut (*ptr).field3);
}
```

One important restriction is that all crates that are linked together must have the same `panic-in-drop` setting. This is necessary so that the compiler can assume that `drop_in_place<T>` never unwinds, even for `T` types defines in other crates.

# Drawbacks
[drawbacks]: #drawbacks

## Deferring execution

A common pattern is to use `Drop` to defer the execution of arbitrary code until the end of a scope. One example of this is the `scopeguard` crate which provides a `defer!` macro. If code within a `defer!` panics then this would abort the process.

Already today, panicking in a `defer!` has the downside of causing a double-panic if the deferred code is invoked as part of an unwind. This is already a problem today: for example, rustc's `delay_span_bug` will defer a panic until the end of a session and only emit it if no other errors occurred. However this can sometimes lead to the compiler double-panicking if rustc is already unwinding due to a previous `span_bug` or `panic`. However if this RFC is accepted then this problem might become more frequent since they will occur in the normal execution path.

## Reporting errors when dropping

Some types can trigger error conditions when dropped. Consider the example of `BufWriter` which needs to flush its buffer when dropped, but where the writing to the underlying `Writer` is a potentially faillible operation. Since it is not possible to "cancel" a drop and return an error, this leaves only 2 possibilities: panic or ignore the error.

The preferred approach is the one used by `BufWriter`: it ignores errors when dropping. If precise error reporting is required then the user can explicitly flush the `BufWriter` and report any errors before dropping the buffer.

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

Another downside of this is that the optimizer can no longer assume that dropping a `Box<dyn Any>` will never unwind, which requires codegen to insert additional landing pads to handle this case.

# Prior art
[prior-art]: #prior-art

A similar change was done in C++11: destructors were changed to be `noexcept` by default unless explicitly opted into using `noexcept(false)`. However actually throwing an exception from a destructor is UB if that object is owned by a standard library container such as `std::vector` (this was also the case before C++11). Thus throwing from a destructor is only safe if done from a local object or one which is contained within a custom user-defined type.

The rationale for C++ making this change is similar to that for this RFC: it eliminates many surprising edge cases which can be a source of undefined behavior and also improves code generation. However the issue of exception safety is much more severe in the case of C++ since it is often impossible to make code exception-safe in the face of throwing destructors due to C++'s non-destructive move semantics and potentially throwing move constructors.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

# Future possibilities
[future-possibilities]: #future-possibilities
