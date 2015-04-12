- Feature Name: process-stdio-redirection
- Start Date: 2015-04-10
- RFC PR:
- Rust Issue:

# Summary

Update the `std::process` API with the ability to redirect stdio of child
processes to any opened file handle or equivalent.

# Motivation

The current API in `std::process` allows to either pipe stdio between parent and
child process or redirect stdio to `/dev/null`. It would also be largely useful
to allow stdio redirection to any currently opened file or pipe handles. This
would allow redirecting stdio with a physical file or even another process (via
OS pipe) without forcing the parent process to buffer the data itself.

For example, one may wish to spawn a process which prints gigabytes
of data (e.g. logs) and use another process to filter through it, and save the
result to a file. The current API would force the parent process to dedicate a
thread just to buffer all data between child processes and disk, which is
impractical given that the OS can stream the data for us via pipes and file
redirection.

# Detailed design

First, the standard library should provide an OS agnostic way of creating OS
in-memory pipes, i.e. a `Pipe`, providing reader and writer handles as
`PipeReader` and `PipeWriter` respectively. The reader and writer will simply be
an abstraction over the OS specific file descriptor/HANDLE, and will implement
the respective `Read`/`Write` trait, making it impossible to confuse the two
together. In addition, each should implement the appropriate `AsRaw{Fd,
Handle}`/`FromRaw{Fd, Handle}` for wrapping/unwrapping of the OS handles.

On Unix systems the pipe can be created with `libc::pipe` and the file
descriptors wrapped as appropriate (e.g. `sys::fs2::File`). On Windows the pipe
can be created via Windows' `CreatePipe` (a stub for which is missing in
`liblibc` at the moment) and the resulting HANDLEs also appropriately wrapped
(using `sys::fs2::File`).

This proposal considers making the `Pipe`'s reader and writer fields public
to allow easily moving ownership of the two handles, but any other appropriate
interface is acceptable.

```rust
pub struct Pipe {
	pub reader: PipeReader,
	pub writer: PipeWriter,
}

pub struct PipeReader(sys::fs2::File);
pub struct PipeWriter(sys::fs2::File);

impl Read for PipeReader { ... }
impl Write for PipeWriter { ... }
```

Next, several `redirect_*` methods should be added to `Stdio` for certain
"blessed" types offered by the standard library, such as `File`, `PipeRead`, and
`PipeWrite`. By white-listing the accepted file-like types we can ensure the API
and its behavior is consistent across Windows and Unix.

```rust
fn redirect_file(f: File) -> Stdio { ... }
fn redirect_pipe_read(r: PipeRead) -> Stdio { ... }
fn redirect_pipe_write(w: PipeWrite) -> Stdio { ... }
```

These methods should take ownership of their arguments since storing references
will require `Stdio` and `Command` to gain lifetime parameters, which will break
the currently stabilized implementations. Thus the caller will be responsible
for duplicating their handles appropriately if they wish to retain ownership.

To make redirections easier to use `Stdio` should become clonable so that once a
file-like handle is wrapped, the wrapper can be passed to any number of
`Commands` simultaneously. This can be accomplished by reference counting the
wrapped file handle.

```rust
impl Clone for Stdio { ... }

#[deriving(Clone)]
struct StdioImp {
	...
	// sys::fs2::File is a safe wrapper over a fd/HANDLE,
	// not to be confused with the public `std::fs::File`
	Redirect(Rc<sys::fs2::File>),
}
```

# Drawbacks

If one wishes to close (drop) a redirected handle, for example, closing the
read end of a pipe and sending EOF to its child, they will have to manually
ensure all cloned `Stdio` wrappers are dropped so that the underlying handle is
closed. Otherwise it would be possible to deadlock while waiting on a child
which is waiting on and input handle held in the same thread.

In addition, if one desires to use a file-like handle outside of process
redirection, they will need to rely on an external mechanism for duplicating the
handle. In the case of an actual file, it can simply be reopened, but in the
case of OS pipes or sockets the underlying OS handle may need to be duplicated
via `libc` calls or the object itself would need to provide a duplication
mechanism.

# Alternatives

* Instead of specifying numerous `redirect_*` methods, simply accept anything
  that implements the appropriate `AsRaw{Fd, Handle}` trait. This will cause OS
  specific issues, however. For example, on Unix sockets are simple file
  descriptors, but on Windows sockets aren't quite HANDLES, meaning the API
  would be inconsistent across OS types.

* Duplicate the underlying OS handle and have the `std::process` APIs take
  ownership of the copy. When working with OS pipes, however, the user would
  have to manually keep track where the duplicates have gone if they wish to
  close them all; failing to do so may cause deadlocks.

* Do not make `Stdio` clonable, and rely on caller duplicating all handles as
  necessary. This could be particularly limiting if certain implementations do
  not allow duplication.

# Unresolved questions

None at the moment.
