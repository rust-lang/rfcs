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

The least intrusive way to implement this functionality would be to implement
`FromRaw{Fd, Handle}` for `Stdio`, taking ownership of the underlying handle.
This approach would not require the addition of any new public APIs.

There are several disadvantages to this design: it does not allow for using
one handle for several redirections without forcing the caller to `unsafe`ly
duplicate the handle, in addition, any cross-platform code will need to be
littered with `cfg!` to call the appropriate methods. Furthermore, trying to use
standard types like `File` will require that they be leaked by `mem::forget` so
that their own destructors do not close the underlying handles. However, these
inefficiencies can be addressed through a cross-platform, high level API.

Lastly, the `AsRaw{Fd, Handle}` traits should be implemented for the
`ChildStd{in,out,err}` types. This would allow easily piping output from one
child process to another by leveraging the underlying OS pipes that were already
created when spawning the child.

## A High Level API Proposal

In order to facilitate cross-platform usage, the API should be defined to
operate on the respective `AsRaw{Fd, Handle}` traits for Unix and Windows,
respectively. All method signatures should match lexically such that
using `cfg!($platform)` checks will not be necessary. The private `StdioImp`
enum in `std::process` should be extended with a `Redirect` variant which will
hold a wrapper over the appropriate OS handle type to ensure it is properly
closed (e.g. using an `AnonPipe`).

Next, the API should expose methods for redirection that both do and do not take
ownership of the underlying handle, e.g. `redirect<T: AsRaw*>(t: T)` and
`redirect_by_ref<T: AsRaw*>(t: &T)`.

The method that takes ownership retains the
benefits of using `FromRaw*` directly while helping the caller avoid making
platform specific calls. Unfortunately, since we cannot guarantee that an
implementor of `AsRaw*` is the sole owner of the OS handle they return, this
method will have to be `unsafe`.

The method which does not take ownership allows the
caller to reuse their handle without making excessive duplications, which would
not be possible by using `FromRaw*` directly. The caller is, however, forced to
ensure the handle will remain valid until the child is spawned, making this
method `unsafe` as well.

Below are several alternative ways of exposing a high level API. They are
ordered in the author's personal preference, but neither is strictly better than
the others designs.

1.
Exposing `redirect` methods via separate `StdioExt` struct: It will live in
`std::os::$platform::process`, thus making it apparent to the caller that they
are using an OS specific extension when importing it. This design offers large
flexibility in external libraries need only define `AsRaw*` or have access to
the raw OS handle itself (which would trivially define `AsRaw*` for itself).

```rust
pub struct StdioExt;
impl StdioExt {
   // Take ownership of the handle
   pub fn redirect<T: AsRaw*>(t: T) -> Stdio;
   // Unsafely borrow the handle, letting caller ensure it is valid
   pub unsafe fn redirect_by_ref<T: AsRaw*>(t: T) -> Stdio;
}
```

2.
Exposing `redirect` methods via trait, e.g. `ToStdio`: This design will give
greatest control to us (std) as to what can be used for redirection, however, it
gives less flexibility to external libraries as they may need to implement
additional traits. Moreover, and any blanket impls over `AsRaw*` invalidate the
tight control (if it is desired) of redirectables. An unresolved question is
what to name this trait as there are no such clear patterns established in the
standard libraries or on `crates.io`. For example, the trait could be `ToStdio`,
`To<Stdio>`, `Into<Stdio>`, etc.

```rust
pub trait ToStdio {
    unsafe fn to_stdio<T: AsRaw*>(t: T) -> Stdio;
}

impl<T> ToStdio for T where T: AsRaw* {
    // Unsafely borrow the handle, letting caller ensure it is valid
    unsafe fn to_stdio<T: AsRaw*>(t: T) -> Stdio;
}
```

3.
Expose `redirect` methods directly on `Stdio`: Cutting out the middleman
(middletrait?) and defining the methods directly on the source minimizes APIs
that will be eventually stabilized. This design, however, blurs the distinction
that OS specifics apply (e.g. a file and socket are both file descriptors on
Unix, but not necessarily HANDLEs on Windows).

```rust
impl Stdio {
    pub fn piped() -> Stdio;
    pub fn inherit() -> Stdio;
    pub fn null() -> Stdio;

    // Take ownership of the handle
    pub fn redirect<T: AsRaw*>(t: T) -> Stdio;
    // Unsafely borrow the handle, letting caller ensure it is valid
    pub unsafe fn redirect_by_ref<T: AsRaw*>(t: T) -> Stdio;
}
```

Example API usage based on the `StdioExt` design described above:

```rust
#[cfg(unix)] use std::os::unix::process::StdioExt;
#[cfg(windows)] use std::os::windows::process::StdioExt;

// Equivalent of `foo | bar`
let foo = Command::new("foo").stdout(Stdio::piped()).spawn().unwrap();
let out = foo.stdout.take().unwrap();
let bar = Command::new("bar").stdin(StdioExt::redirect(out)).spawn().unwrap();
```

# Drawbacks

Without using a high level API callers will be forced to use verbose and
`unsafe` code more than they should or could get away with. Even with using a
high level API there will be `unsafe`ty present due to stability lock on
`Command` and `Stdio` (i.e. we cannot simply store references to the handles
ensuring they remain valid). However, `Command`s are are usually spawned
immediately after building during which time open file descriptors/HANDLEs are
still valid.

# Alternatives

High level API alternatives discussed above.

# Unresolved questions

None at the moment.
