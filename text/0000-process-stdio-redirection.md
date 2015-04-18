- Feature Name: process-stdio-redirection
- Start Date: 2015-04-10
- RFC PR:
- Rust Issue:

# Summary

Update the standard library API with the ability to redirect stdio of child
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

Process redirection should be provided as a system dependent extension which
accepts the appropriate OS file/pipe representation. Namely, the API should be
provided as a `StdioExt` implementation, whose `redirect` method accepts
implementors of `AsRawFd` and `AsRawHandle` for Unix and Windows, respectively.
This implementation should be publicly exported under the
`std::os::$platform::process` module.

The private `StdioImp` enum in `std::process` should be extended with a
`Redirect` variant which will hold the appropriate OS handle type. To avoid
breaking changes with the stabilized interfaces (such as `Stdio` and `Command`)
or dealing with internal reference counts over the handle to be redirected, the
most convenient implementation would be to `unsafe`ly extract and store the raw
fd/HANDLE to which the redirection should occur. Thus it would be the caller's
responsibility to ensure the open file or pipe remains valid until a child
process is spawned.

```rust
StdioImp {
    ...
    #[cfg(unix)] Redirect(sys::io::RawFd),
    #[cfg(windows)] Redirect(sys::io::RawHandle),
}

// Unix, in libstd/sys/unix/ext.rs
pub struct StdioExt;
impl StdioExt {
	unsafe fn redirect<T: AsRawFd>(t: &T) -> Stdio { ... }
}

// Windows, in libstd/sys/windows/ext.rs
pub struct StdioExt;
impl StdioExt {
	unsafe fn redirect<T: AsRawHandle>(t: &T) -> Stdio { ... }
}
```

One of the benefits of this design is that it still makes the caller aware they
are using system dependent extensions, while being able to utilize the API
without suffering the constant need of `cfg!` checks to specify the exact target
OS. For example, the code below would work on both Unix and Windows:

```rust
#[cfg(unix)] use std::os::unix::process::StdioExt;
#[cfg(windows)] use std::os::windows::process::StdioExt;

let file = File::open(...);
let stdio = unsafe { StdioExt::redirect(&file) };
...
```

Next, the `AsRaw{Fd, Handle}` traits should be implemented for the
`ChildStd{in,out,err}` types. This would allow easily piping output from one
child process to another by leveraging the underlying OS pipes that were already
created when spawning the child.

```rust
// Equivalent of `foo | bar`
let foo = Command::new("foo").spawn().unwrap();
let out = foo.stdout.as_ref().unwrap();
let bar = Command::new("bar").stdin(StdioExt::redirect(out)).spawn().unwrap();
// close foo.stdout here so that bar is the only pipe reader

// Alternatively
let bar = Command::new("bar").spawn().unwrap();
let in  = bar.stdin.as_ref().unwrap();
let foo = Command::new("foo").stdout(StdioExt::redirect(in)).spawn().unwrap();
// close bar.stdin here so that foo is the only pipe writer
```

This would require that the internally defined `AnonPipe` wrapper be implemented
using HANDLEs (and not file descriptors) on Windows. This can easily be
accomplished by wrapping the resulting HANDLEs from Windows' `CreatePipe` API
(a stub for which is missing in `libc` at the moment). The Unix implementation
can continue to use `libc::pipe`, of course. With these changes in place,
`AnonPipe` can implement `AsRaw{Fd, Handle}`, and allow `ChildStd{in, out, err}`
to implement the traits as well.

# Drawbacks

Unsafely using raw OS file handles could potentially cause issues, however,
`Command`s are are usually spawned immediately after building during which time
open file descriptors/HANDLEs are still valid.

# Alternatives

None that don't involve breaking changes or verbose interfaces.

# Unresolved questions

None at the moment.
