- Feature Name: process-stdio-redirection
- Start Date: 2015-04-10
- RFC PR:
- Rust Issue:

# Summary

Update the standard library with a cross-platform high level API for redirecting
stdio of child processes to any opened file handle or equivalent.

# Motivation

The current API in `std::process` allows the usage of raw file descriptors or
HANDLEs for child process stdio redirection by leveraging the `FromRaw{Fd,
Handle}` and `AsRaw{Fd, Handle}` trait implementations on `Stdio` and
`ChildStd{In, Out, Err}`, respectively.

Unfortunately, since the actual methods pertaining to `FromRaw*` and `AsRaw*`
are OS specific, their usage requires either constant `cfg` checks or OS
specific code duplication. Moreover, the conversion to `Stdio` is `unsafe` and
requires the caller to ensure the OS handle remains valid until spawning the
child. In the event that a caller wishes to give a child exclusive ownership of
an OS handle, they must still go through the headache of manually keeping the
handle alive and valid.

Developing a high level cross-platform API will make stdio redirection more
ergonomic and reduce code noise.

# Detailed design

The de facto method for adding system specific extensions to the standard
library has been to define an extension trait--following this approach a
`StdioExt` trait should be defined under `std::os::$platform::process` to
provide the redirection functionality. Unlike other system specific extensions,
however, the methods of this trait should match lexically, differing only in the
`AsRaw*` type they accept, such that rebuilding the same source on a different
platform will only require the import of the OS specific trait rather than
changing method invodations as well.

This trait should define two methods which accept the appropriate `AsRaw*`
implementor and return an `Stdio`:
* One which (safely) takes ownership of the raw handle or its wrapper. The
  wrapper should be boxed and stored by the `Stdio` wrapper so its destructor
  can run when it goes out of scope.
* Another method which simply extracts the raw handle without taking ownership:
  this method will essentially be a cross-platform abstraction over using
  `FromRaw*` on `Stdio`, thus making this method `unsafe` as well.

```rust
pub trait StdioExt {
   // Take ownership of the handle
   fn redirect<T: AsRaw*>(t: T) -> Stdio;
   // Unsafely borrow the handle, letting caller ensure it is valid
   unsafe fn redirect_by_ref<T: AsRaw*>(t: &T) -> Stdio;
}
```

Example API usage with minimal `cfg` checks and safe methods:

```rust
#[cfg(unix)] use std::os::unix::process::StdioExt;
#[cfg(windows)] use std::os::windows::process::StdioExt;

// Equivalent of `foo | bar`
let foo = Command::new("foo").stdout(Stdio::piped()).spawn().unwrap();
let out = foo.stdout.take().unwrap();
let bar = Command::new("bar").stdin(Stdio::redirect(out)).spawn().unwrap();
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

An alternative approach would be to expose `redirect` methods directly on
`Stdio`. This design, however, blurs the distinction of platform specific
details (e.g. a file and socket are both file descriptors on Unix, but not
necessarily HANDLEs on Windows) and may cause some confusion and give rise to
platform specific bugs.

```rust
impl Stdio {
    pub fn piped() -> Stdio;
    pub fn inherit() -> Stdio;
    pub fn null() -> Stdio;

    // Take ownership of the handle
    #[cfg(unix)] pub fn redirect<T: AsRawFd>(t: T) -> Stdio;
    #[cfg(windows)] pub fn redirect<T: AsRawHandle>(t: T) -> Stdio;

    // Unsafely borrow the handle, letting caller ensure it is valid
    #[cfg(unix)] pub unsafe fn redirect_by_ref<T: AsRawFd>(t: T) -> Stdio ;
    #[cfg(windows)] pub unsafe fn redirect_by_ref<T: AsRawHandle>(t: T) -> Stdio;
}
```

# Unresolved questions

None at the moment.
