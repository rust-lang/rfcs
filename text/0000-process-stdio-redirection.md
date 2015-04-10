- Feature Name: process-stdio-redirection
- Start Date: 2015-04-10
- RFC PR:
- Rust Issue:

# Summary

Update the `std::process` API with the ability to redirect stdio of child
processes to any opened file handle.

# Motivation

The current API in `std::process` allows to either pipe stdio between parent and
child process or redirect stdio to `/dev/null`. It would also be largely useful
to allow stdio redirection to any currently opened `std::fs::File` (henceforth
`File`) handle. This would allow redirecting stdio with a physical file or even
another process (via OS pipe) without forcing the parent process to buffer the
data itself.

For example, one may wish to spawn a process which prints gigabytes
of data (e.g. logs) and use another process to filter through it, and save the
result to a file. The current API would force the parent process to dedicate a
thread just to buffer all data between child processes and disk, which is
impractical given that the OS can stream the data for us via pipes and file
redirection.

# Detailed design

First, the standard library should provide an OS agnostic way of creating OS
in-memory pipes, i.e. a `Pipe`, providing reader and writer handles as `File`s.
This would avoid the need for users to write OS specific (and `unsafe`) code
while retaining all the benefits offered by `File`. This proposal considers
making the `Pipe`'s reader and writer fields public to allow easily moving
ownership of the two handles, but any other appropriate interface is acceptable.

```rust
pub struct Pipe {
	pub reader: File,
	pub writer: File,
}
```

Next, `std::process::Stdio` should provide a `redirect` method which accepts and
stores a `&File`, ensuring the underlying handle will not go out of scope
unexpectedly before the child is spawned. The spawning implementation can then
extract and use the `File`'s OS specific handle when creating the child
process.

This `File` reference should be an immutable one, to allow "reuse" of the handle
across several `Command`s simultaneously. This, however, can allow code to
indirectly mutate a `File` through an immutable reference by passing it on to a
child process, although retrieving and mutating through the underlying OS handle
(via `AsRaw{Fd, Socket, Handle}`) is already possible through a `&File`. Thus
this API would not introduce any "mutability leaks" of `File`s that were not
already present.

This design also offers benefits when the user may wish to close (drop) a
`File`, for example, closing the read end of a pipe and sending EOF to its
child. The compiler can infer the lifetimes of all references to the original
`File`, eliminating some guesswork on whether all ends of a pipe have been
closed. This would not be possible in a design which duplicates the OS handle
internally which could more easily lead to a deadlock (e.g. waiting on a child
to exit while the same scope holds an open pipe to the child's stdin).

Reclaiming ownership of the borrowed `File` may require locally scoping the
creation of `Stdio` or `Command` instances to force their lifetimes to end,
however, this is minimally intrusive compared to alternative designs.

# Drawbacks

Implementing a design based on `File` borrows will require adding lifetime
parameters on stabilized `Stdio` and `Command` close to the 1.0 release.

# Alternatives

Do nothing now and choose a stability compatible design, possibly being stuck
with less ergonomic APIs.

One alternative strategy is to duplicate the underlying OS handle and have the
`std::process` APIs take ownership of the copy. When working with OS pipes,
however, the user would have to manually keep track where the duplicates have
gone if they wish to close them all; failing to do so may cause deadlocks.

Another strategy would be for `Stdio` to take ownership of a `File` and wrap it
as a `Rc<File>`, allowing it to be "reused" in any number of redirections by
cloning the `Stdio` wrapper. A caller could try to regain ownership (via
`try_unwrap` on the internal wrapper), but they would have to (manually) ensure
all other `Stdio` clones are dropped. This design would also suffer from
potential deadlocks, making it by far the least ergonomic option.

# Unresolved questions

None at the moment.
