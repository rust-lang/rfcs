- Start Date: 2015-01-21
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

While Rust does not have exceptions, it does do unwinding. The primary reason for unwinding is that
it allows resources to be cleaned up in the event of an unrecoverable error. Part of Rust's model
for failure is that failures are only catchable on task boundaries, and that the state of a failed
task is not visible from the outside. This is a simple, clean model that allows for efficient robust
programs.

This RFC is a proposal to allow unwinding to be caught at the boundaries between non-Rust and Rust
functions. It also provides the user with the ability specify whether a given function will unwind
or not for cases when the function body is not available.

# Motivation

The major use case for this feature is to allow users to prevent and handle unwinding into non-Rust
code. Unwinding into non-Rust code is currently considered undefined behaviour but we give users no
tools to prevent this behaviour. While in some cases the code can be manually audited for potential
unwinding, if arbitrary user code can be executed (for example, in a callback), then this unwinding
is impossible to prevent.

# Detailed design

## Semantics

The current Rust semantics are that failure leaves the task in an invalid state and therefore
continued execution within that task is undefined behaviour. "Task" currently refers to threads,
however this RFC proposes to extend the definition of a task to include the entry into Rust code
from non-Rust code. As such, the task ends when control returns to original non-Rust code.

This extension essentially produces "nested tasks", where the thread is one task, then each transfer
of control from non-Rust code to Rust code is another. Unwinding out of a task is undefined
behaviour, meaning that the current behaviour regarding unwinding into non-Rust code is preserved.
However, the extended definition of "task" allows for unwinding to be caught at the entry point of
the Rust code. From there a user may decide to return an error code or abort the process.

Attempts to catch unwinding at any point other than a task entry point is considered undefined behaviour.

## Implementation

Rust already has a significant amount of code for handling unwinding. However, it is currently not
safe to use nested invocations of the `try` function. The first part of the implementation would be
to improve the unwinding system to allow for nested calls to the try function.

The `try` function itself doesn't need changing. The current signature is `fn try<F:FnOnce()>(f: F)
-> Result<(), Box<Any + Send>>`, which provides means for the caller to know if the invocation was
successful. The function would remain unsafe.

An example:

```rust
extern "C" fn foo() -> c_int {
    let mut ret = 0;
    let res = try(|| {
	    ret = inner_call();
    });

    match res {
	    Err(_) => return -1,
		Ok(_) => return ret,
	}
}
```

As a fallback, if the user does not catch the unwinding themselves, the process is aborted. This is
the only thing can reasonably be done if we attempt to unwind into code that likely does not know
how to handle it.

## Edge Cases and Other Miscellany

Sometime you need to be able to unwind from a non-Rust ABI function, a particularly relevant
instance of this is the `rust_try` function itself, as it uses the C ABI. With the above semantics,
this would cause the process to be aborted, not what would be wanted. Therefore a new attribute
`#[can_unwind]` is used to indicate that the function in question may unwind, and that this
unwinding is expected.

As a complement to `#[can_unwind]`, another attribute `#[unsafe_no_unwind]` (named such as it is
unsafe when used incorrectly) can be used to mark functions as specifically not unwinding. This is
primarily for performance reasons in cases where the function cannot be inferred to not unwind. The
primary use case for this attribute is the `oom` function that aborts the process in the case of a
out-of-memory condition.

Unwinding from a function marked `#[unsafe_no_unwind]` is undefined behaviour.

# Drawbacks

* Exposes two new attributes, one of which can cause undefined behaviour when used improperly.
* Exposes the unwinding machinery to users. It's current status as an implementation detail gives us
  flexibilty.

# Alternatives

* Allow for catching unwinding at arbitrary points in execution. The current design only allows for
  unwinding to be caught at specific points, limiting the potential applications of the
  feature. However, support this would require a "rethrow" mechanism and quickly raises the semantic
  and practical complexity of the feature.

* Don't allow the user to catch unwinding at all. This is the current case (other than the code in
  `std::rt::unwind`). We could either always abort when we detect unwinding into non-Rust code, or
  leave it and make the user either risk undefined behaviour.

  Both options are unsatisfactory as it does not provide the user any control over the unwinding
  process. Interfacing with C functions that use callbacks would become practically impossible
  without potentially aborting the process or risking undefined behaviour.

# Unresolved questions

* Should the `#[unsafe_no_unwind]` attribute be available? It has some performance benefits, but is
  dangerous when used incorrectly. The `oom` function is a prime candidate though, as it is not
  inlined, but cannot unwind.

* The current return value of the `try` function is `Result<(), Box<Any + Send>>`. It may be more
  appropriate to return a custom error type.