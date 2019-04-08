- Feature Name: close-trait
- Start Date: 2019-04-03
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add a `Close` trait to `std::io` that allows for dropping a value and producing
a `Result`.

# Motivation
[motivation]: #motivation

In many programming languages, it is possible to explicitly `close` many types
of resources.  In Rust, we have the `Drop` trait to automatically cleanup these
resources for us.  For various reasons, we have decided that `Drop` shall never
fail.  Thus errors encountered in a `Drop` implementation should be ignored.
Implementations of `Close` will allow us to explicitly handle errors that could
occur in a `Drop` implementation.  Making this into a trait will allow us to
generically implement this functionality for many types of resources.  This will
also allow us to `close` wrapper types, such as `BufReader` or `BufWriter` when
they wrap any type implementing `Close`.

Adding this method will allow users of the language to handle errors only
revealed when the resource is dropped.  One example of this would be a race
condition for shared resource access.  Another would be in the case of a
resource that cannot be fully flushed before it is dropped.

For specifically `File`s, we can call `sync_all` to force synchronization of the
data to the filesystem.  But this function is no longer directly available when
we use a `BufReader` or `BufWriter`.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Currently in rust, we can manipulate files simply and expect them to be
automatically closed:

```rust
use std::fs::File;
use std::io::prelude::*;

fn main() -> std::io::Result<()> {
    let mut file = File::create("foo.txt")?;
    file.write_all(b"Hello world!")?;
    Ok(())
}
```

Although this checks if there were errors in writing the data to the file, there
sometimes are spurious failures that aren't caught here.  Our program is
reporting no errors and we can't figure out what is going wrong.  To solve this,
we can call `close` and handling the errors it produces.  This method simply
communicates its meaning -- close the file.

```rust
use std::fs::File;
use std::io::prelude::*;
use std::io::Close;

fn main() -> std::io::Result<()> {
    let mut file = File::create("foo.txt")?;
    file.write_all(b"Hello world!")?;
    file.close()?;
    Ok(())
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This trait is added to `src/io/close.rs`:

```rust
pub trait Close {
    type Error;
    
    fn close(self) -> Result<(), Error>;
}
```

## File

This trait will be implemented for `fs::File` with `Error` specified as
`io::Error`.

In `src/libstd/fs.rs`:

```rust
impl Close for File {
    type Error = io::Error;

    fn close(self) -> io::Result<()> {
        let result = self.inner.close();
        std::mem::forget(self);
        result
    }
}
```

## Underlying os-specific behavior

The underlying implementation of `File`s will support a method of the following
signature: `pub fn close(&self) -> io::Result<()>`.  For most of these
implementations, this boils down to wrapping `cvt` around the underlying close
function call and appending `.map(|_| ())` to get rid of the error code.  This
will call the system-level close function and return errors when applicable.
For example, on unix, this is `close`.  On Windows, this is `CloseHandle`.

Here is the example unix implementation.  Add this to
`src/libstd/sys/unix/fs.rs` in the `impl File`:

```rust
pub fn close(&self) -> io::Result<()> {
    self.0.close()
}
```

In `src/libstd/sys/unix/fd.rs`, add this to the `impl FileDesc`:

```rust
pub fn close(&self) -> io::Result<()> {
    cvt(unsafe { libc::close(self.fd) }).map(|_| ())
}
```

## BufReader and BufWriter

For `BufReader` and `BufWriter`, we implement `Close` if and only if the
underlying type implements `Close`.  For `BufReader`, we delegate to the
underlying `Close` implementation.

```rust
impl<R: Close> Close for BufReader<R> {
    type Error = R::Error;
    fn close(self) -> Result<(), Self::Error> {
        self.into_inner().close()
    }
}
```

For `BufWriter`, it is possible that flushing our buffer and closing the wrapped
resource will cause an error.  In this case, we must ignore the error from one
or the other.  If both cause an error, it is likely that it is from the same
cause.  I believe the correct choice is to ignore the error in closing the file
and drop the underlying resource normally.

```rust
impl<R: Close<Error = io::Error> + Write> Close for BufWriter<R> {
    type Error = R::Error;
    fn close(self) -> Result<(), Self::Error> {
        self.into_inner()?.close()
    }
}
```

# Drawbacks
[drawbacks]: #drawbacks

* This adds more clutter to the standard library.
* The `Close` trait wouldn't be a part of the prelude and thus could be
  overlooked.
* `Close` implementations for wrapper types are often times hard to correctly
  implement (see `BufWriter`).

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Alternatives

### As a method of `File`

Although this would lead to less code, it would reduce the applicability of this
functionality to more modules in the future.  This would also prevent us from
implementing `Close` for `BufReader` and `BufWriter`.

We could even have a method in each of these structs that delegates to the
trait's implementation, allowing for people to `close` resources without
explicitly importing `std::io::Close`.

### Default implementation

We could implement `Close` for all `Drop` types by always succeeding.  Since
this change can be done retroactively later without breaking backwards
compatibility, I choose to leave it out of this RFC.

### Named `TryDrop` instead

We could name this trait `TryDrop` instead, and the method `try_drop`.  This
seems well and good as `Close` allows us to generalize the idea of `Drop`
similarly to `TryFrom` and `TryInto`.  I do not prefer this so long as `close`
takes `self` by value because I do not think it is intuitive to have different
input parameters for semantically similar functions.

    fn close(self) -> Result<(), Error>;
    fn drop(&mut self) -> ();

### Taking `self` by `&mut`

`Close` has the signature `fn close(self) -> Result<(), Error>` whereas `Drop`
has `fn drop(&mut self) -> ()`.  These signatures aren't identical, but they
could be made to be.  We could rework the method to be `fn try_drop(&mut self)
-> Result<(), Error>` and have a similar wrapper function `std::mem::try_drop`
that took it by value.  This seems very reasonable at first glance, but becomes
much more difficult when considering how to implement `std::mem::try_drop`.
This function would probably have to have some compiler magic to recursively
`drop` each member after `try_drop` is called without calling the `drop`
instance on the overall `struct`.

# Prior art
[prior-art]: #prior-art

C++ is an apt comparison as it also automatically closes files.
[`std::basic_filebuf`](https://en.cppreference.com/w/cpp/io/basic_filebuf) is
used as the underlying implementation for files.  The
[destructor](https://en.cppreference.com/w/cpp/io/basic_filebuf/%7Ebasic_filebuf)
also ignores errors when closing the file.  It provides a
[`close`](https://en.cppreference.com/w/cpp/io/basic_filebuf/close) method that
rethrows exceptions while guaranteeing the file is closed.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

What module should this go in?  It seems logical to go in `std::io`, but it
might be more appropriate for `std::mem` beside `std::mem::drop`.

# Future possibilities
[future-possibilities]: #future-possibilities

Provide a default implementation of `Close` for any type deriving `Drop`.  This
is a backwards compatible change so it can be addressed in a later RFC.
