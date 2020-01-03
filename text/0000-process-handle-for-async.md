- Feature Name: `process_handle_for_async`
- Start Date: 2019-11-28
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Introduce process descriptor handles for each process in `'std::process'` for async polling of process-termination.

# Motivation
[motivation]: #motivation?

Currently `std::process` is managing processes (on Posix/Linux systems) either in synchronous way using the wait-function-family, or asynchronously via signal-handlers (SIGCHLD). 

The problem is that such signals may be delivered to any thread running within the parent-process. So, dealing with the termination of child-processes in async manner will require a shared collection containing the process-status of all child-processes.

This RFC proposes a portable alternative to signal-handling to deal with the termination of child-processes in async manner, and introducing strict ownership for child-processe using a process-handle (aka process descriptor)

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Depending on the underlying operating system, sub-processes may be associated with a process-id (pid) and most operating systems provide  so called process-handles (process descriptors). Such handles may be used to perform async polling or sending signals to the corresponding process. Such process handles may be used to establish strict ownership for child-processes in Rust.

The process-handle feature shall permit a portable API for various operating systems. The following listing shows the process-handle features being available for different operating systems, and might be the base for a generalized Rust-API in `std::process`.

- As with Windows, the fucntion `CreateProcess` provides the HANDLE for the newly created child-process.
- As with OpenBSD Unix provides process handles 
- As with FreeBSD Unix the function `libc::pdfork(..)` provides a process descriptor for the newly forked child-process.
- As with Linux, the `pidfd`-familiy provides funtionality to convert a pid int a file-descriptor (merged into release Linux-5.4)
- Older Linux-releases or Posix-Systems may use the `forkfd`-concept to establsih a descriptor/handle for a child-process
 
As for the latter case when the underlying OS does not provide a suitable native process-handle, the forkfd concept may be used. This is based on so called death-pipes: An anonymous pipe is established between parent-process (RX end) and the child-process (TX end). Here the parent-process may use RX end to poll for read-events (file descriptor) in async manner, for example using a future.
Now, if the child-process terminates the TX end is being destroyed and the RX-end in the parent process will receive and EOF read-event.

All of the listed process-handle flavors share the common concept to perform async polling for process-termination and might be the base for a generalized process-handle concept in Rust `std::process`, leveraging a strong ownership-concept for child-processes. The handling of signal SIGCHLD and sharing of child-process meta-data would no longer be required.

Pros:

- permits strict ownership and async-handling of termination of child-processes
- no shared collection required to deal with SIGCHLD signal and exit-status of child-processes
- on Posix/VxWorks etal the process-handle will be a file-descriptor, and can be handled by async/await framework.
- on Windows the process-handle will be the HANDLE-object provided by function CreateProcessA(..).

Cons:

- on Posix/VxWorks etal each child-process will cause an additional file-descriptor in parent process and a non-referenced file-descriptor in child-process.
- legacy signal-handlers dealing with SIGCHLD might consume the exit-status before the owning async task got a chance to read the exit-status, therefore the SIGCHLD signal-handling should be disabled in the parent-process, or just be used for legacy code.


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The following patch is a first sketch of the concept (under construction)
Branch https://github.com/frehberg/rust/tree/process_handle
Patch frehberg/rust@337690e

The API impl of `std::process::Process` shall be extended to provide access to the handle for the corresponding child-process. The ProcHandle API shall abstract from underlying native process-handle feature.

```rust
// Module std::process

impl Process {
...
 /// Provides access to the underlying process handle for async polling for child-process-termination
 pub fn handle(&self) ->  &ProcHandle;
...
}
```
This ProcHandle shall provide access to the underlying native process-handle, permitting integration into the async polling framework using futures and for example tokio.


# Drawbacks
[drawbacks]: #drawbacks

In case the underlying operating system does not provide a native concept for process handles, this functionality may introduce an overhead per child-process.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Intregating the process-handle into the `std::process` module will provide a common API for all async-await frameworks dealing with child-processes. A process-handle may be handled either in a synchronous manner or asynchronously using ftutures and tokio.

# Prior art
[prior-art]: #prior-art

Platform specific features and abstract APIs dealing with process-handles.
- pidfd explained https://lwn.net/Articles/794707/
- pidfds-process-file-descriptors-on-linux https://kernel-recipes.org/en/2019/talks/pidfds-process-file-descriptors-on-linux/
- Qt/QProcess generic API https://code.woboq.org/qt5/qtbase/src/corelib/io/qprocess_unix.cpp.html#_ZN15QProcessPrivate12startProcessEv
- mio-pidfd https://github.com/samuelbrian/mio-pidfd
- crate pidfd https://crates.io/crates/pidfd

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- How to deal with embedded operating systems.

# Future possibilities
[future-possibilities]: #future-possibilities
