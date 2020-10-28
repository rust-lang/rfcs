- Feature Name: read_buf
- Start Date: 2020/05/18
- RFC PR: [rust-lang/rfcs#2930](https://github.com/rust-lang/rfcs/pull/2930)
- Rust Issue: [rust-lang/rust#78485](https://github.com/rust-lang/rust/issues/78485)

# Summary
[summary]: #summary

The current design of the `Read` trait is nonoptimal as it requires that the buffer passed to its various methods be
pre-initialized even though the contents will be immediately overwritten. This RFC proposes an interface to allow
implementors and consumers of `Read` types to robustly and soundly work with uninitialized buffers.

# Motivation
[motivation]: #motivation

## Background
[motivation-background]: #motivation-background

The core of the `Read` trait looks like this:

```rust
pub trait Read {
    /// Reads data into `buf`, returning the number of bytes written.
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>;
}
```

Code working with a reader needs to create the buffer that will be passed to read; the simple approach is something like
this:

```rust
let mut buf = [0; 1024];
let nread = reader.read(&mut buf)?;
process_data(&buf[..nread]);
```

However, that approach isn't ideal since the work spent to zero the buffer is wasted. The reader should be overwriting
the part of the buffer we're working with, after all. Ideally, we wouldn't have to perform any initialization at all:

```rust
let mut buf: [u8; 1024] = unsafe { MaybeUninit::uninit().assume_init() };
let nread = reader.read(&mut buf)?;
process_data(&buf[..nread]);
```

However, whether it is allowed to call `assume_init()` on an array of uninitialized integers is
[still subject of discussion](https://github.com/rust-lang/unsafe-code-guidelines/issues/71).
And either way, this is definitely unsound when working with an arbitrary reader. The `Read` trait is not unsafe, so the soundness of
working with an implementation can't depend on the "reasonableness" of the implementation for soundness. The
implementation could read from the buffer, or return the wrong number of bytes read:

```rust
struct BrokenReader;

impl Read for BrokenReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        Ok(buf.len())
    }
}

struct BrokenReader2;

impl Read for BrokenReader2 {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf[0] == 0 {
            buf[0] = 1;
        } else {
            buf[0] = 2;
        }

        Ok(1)
    }
}
```

In either case, the `process_data` call above would be working with uninitialized memory. Uninitialized memory is a
dangerous (and often misunderstood) beast. Uninitialized memory does not have an *arbitrary* value; it actually has an
*undefined* value. Undefined values can very quickly turn into undefined behavior. Check out
[Ralf's blog post](https://www.ralfj.de/blog/2019/07/14/uninit.html) for a more extensive discussion of uninitialized
memory.

## But how bad are undefined values really?
[motivation-badness]: #motivation-badness

Are undefined values *really* that bad in practice? Consider a function that tries to use an uninitialized buffer with
a reader:

```rust
fn unsound_read_u32_be<R>(r: &mut R) -> io::Result<u32>
where
    R: Read,
{
    let mut buf: [u8; 4] = unsafe { MaybeUninit::uninit().assume_init() };
    r.read_exact(&mut buf)?;
    Ok(u32::from_be_bytes(buf))
}
```

Now consider this function that tries to use `unsound_read_u32_be`:

```rust
pub fn blammo() -> NonZeroU32 {
    let n = unsound_read_u32_be(&mut BrokenReader).unwrap();
    NonZeroU32::new(n).unwrap_or(NonZeroU32::new(1).unwrap())
}
```

It should clearly only be able to return a nonzero value, but if we compile it using rustc 1.42.0 for the
x86_64-unknown-linux-gnu target, the function [compiles down to this](https://rust.godbolt.org/z/Y9rL-5):

```asm
example::blammo:
        ret
```

That means that it will return whatever arbitrary number happened to be in the `%rax` register. That could very well
happen to be 0, which violates the invariant of `NonZeroU32` and any upstream callers of `blammo` will have a bad time.
Because the value that `unsound_read_u32_be` returned was undefined, the compiler completely removed the check for 0!

We want to be able to take advantage of the improved performance of avoiding buffer initialization without triggering
undefined behavior in safe code.

## Why not just initialize?
[motivation-why]: #motivation-why

If working with uninitialized buffers carries these risks, why should we bother with it at all? Code dealing with IO in
both the standard library and the ecosystem today already works with uninitialized buffers because there are concrete,
nontrivial performance improvements from doing so:

* [The standard library measured](https://github.com/rust-lang/rust/pull/26950) a 7% improvement in benchmarks all the
    way back in 2015.
* [The hyper HTTP library measured](https://github.com/tokio-rs/tokio/pull/1744#issuecomment-554543881) a nontrivial
    improvement in benchmarks.
* [The Quinn QUIC library measured](https://github.com/tokio-rs/tokio/pull/1744#issuecomment-553501198) a 0.2%-2.45%
    improvement in benchmarks.

Given that the ecosystem has already found that uninitialized buffer use is important enough to deal with, the standard
library should provide a more robust framework to work with.

In addition, working with regular initialized buffers can be *more complex* than working with uninitialized buffers!
Back in 2015, the standard library's implementation of `Read::read_to_end` was found to be wildly inefficient due to
insufficiently careful management of buffer sizes because it was initializing them.
[The fix](https://github.com/rust-lang/rust/pull/23820) improved the performance of small reads by over 4,000x! If
the buffer did not need to be initialized, the simpler implementation would have been fine.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The `ReadBuf` type manages a *progressively initialized* buffer of bytes. It is primarily used to avoid buffer
initialization overhead when working with types implementing the `Read` trait. It wraps a buffer of
possibly-uninitialized bytes and tracks how much of the buffer has been initialized and how much of the buffer has been
filled. Tracking the set of initialized bytes allows initialization costs to only be paid once, even if the buffer is
used repeatedly in a loop.

Here's a small example of working with a reader using a `ReadBuf`:

```rust
// The base level buffer uses the `MaybeUninit` type to avoid having to initialize the whole 8kb of memory up-front.
let mut buf = [MaybeUninit::<u8>::uninit(); 8192];

// We then wrap that in a `ReadBuf` to track the state of the buffer.
let mut buf = ReadBuf::uninit(&mut buf);

loop {
    // Read some data into the buffer.
    some_reader.read_buf(&mut buf)?;

    // If nothing was written into the buffer, we're at EOF.
    if buf.filled().is_empty() {
        break;
    }

    // Otherwise, process the data.
    process_data(buf.filled());

    // And then clear the buffer out so we can read into it again. This just resets the amount of filled data to 0,
    // but preserves the memory of how much of the buffer has been initialized.
    buf.clear();
}
```

It is important that we created the `ReadBuf` outside of the loop. If we instead created it in each loop iteration we
would fail to preserve the knowledge of how much of it has been initialized.

When implementing `Read`, the author can choose between an entirely safe interface that exposes an initialized buffer,
or an unsafe interface that allows the code to work directly with the uninitialized buffer for higher performance.

A safe `Read` implementation:

```rust
impl Read for MyReader {
    fn read_buf(&mut self, buf: &mut ReadBuf<'_>) -> io::Result<()> {
        // Get access to the unwritten part of the buffer, making sure it has been fully initialized. Since `ReadBuf`
        // tracks the initialization state of the buffer, this is "free" after the first time it's called.
        let unfilled: &mut [u8] = buf.initialize_unfilled();

        // Fill the whole buffer with some nonsense.
        for (i, byte) in unfilled.iter_mut().enumerate() {
            *byte = i as u8;
        }

        // And indicate that we've written the whole thing.
        let len = unfilled.len();
        buf.add_filled(len);

        Ok(())
    }
}
```

An unsafe `Read` implementation:

```rust
impl Read for TcpStream {
    fn read_buf(&mut self, buf: &mut ReadBuf<'_>) -> io::Result<()> {
        unsafe {
            // Get access to the filled part of the buffer, without initializing it. This method is unsafe; we are
            // responsible for ensuring that we don't "de-initialize" portions of it that have previously been
            // initialized.
            let unfilled: &mut [MaybeUninit<u8>] = buf.unfilled_mut();

            // We're just delegating to the libc read function, which returns an `isize`. The return value indicates
            // an error if negative and the number of bytes read otherwise.
            let nread = libc::read(self.fd, unfilled.as_mut_ptr().cast::<libc::c_void>(), unfilled.len());

            if nread < 0 {
                return Err(io::Error::last_os_error());
            }

            let nread = nread as usize;
            // If the read succeeded, tell the buffer that the read-to portion has been initialized. This method is
            // unsafe; we are responsible for ensuring that this portion of the buffer has actually been initialized.
            buf.assume_init(nread);
            // And indicate that we've written the bytes as well. Unlike `assume_initialized`, this method is safe,
            // and asserts that the written portion of the buffer does not advance beyond the initialized portion of
            // the buffer. If we didn't call `assume_init` above, this call could panic.
            buf.add_filled(nread);

            Ok(())
        }
    }
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

```rust
/// A wrapper around a byte buffer that is incrementally filled and initialized.
///
/// This type is a sort of "double cursor". It tracks three regions in the buffer: a region at the beginning of the
/// buffer that has been logically filled with data, a region that has been initialized at some point but not yet
/// logically filled, and a region at the end that is fully uninitialized. The filled region is guaranteed to be a
/// subset of the initialized region.
///
/// In summary, the contents of the buffer can be visualized as:
/// ```not_rust
/// [             capacity              ]
/// [ filled |         unfilled         ]
/// [    initialized    | uninitialized ]
/// ```
pub struct ReadBuf<'a> {
    buf: &'a mut [MaybeUninit<u8>],
    filled: usize,
    initialized: usize,
}

impl<'a> ReadBuf<'a> {
    /// Creates a new `ReadBuf` from a fully initialized buffer.
    #[inline]
    pub fn new(buf: &'a mut [u8]) -> ReadBuf<'a> { ... }

    /// Creates a new `ReadBuf` from a fully uninitialized buffer.
    ///
    /// Use `assume_init` if part of the buffer is known to be already inintialized.
    #[inline]
    pub fn uninit(buf: &'a mut [MaybeUninit<u8>]) -> ReadBuf<'a> { ... }

    /// Returns the total capacity of the buffer.
    #[inline]
    pub fn capacity(&self) -> usize { ... }

    /// Returns a shared reference to the filled portion of the buffer.
    #[inline]
    pub fn filled(&self) -> &[u8] { ... }

    /// Returns a mutable reference to the filled portion of the buffer.
    #[inline]
    pub fn filled_mut(&mut self) -> &mut [u8] { ... }

    /// Returns a shared reference to the initialized portion of the buffer.
    ///
    /// This includes the filled portion.
    #[inline]
    pub fn initialized(&self) -> &[u8] { ... }

    /// Returns a mutable reference to the initialized portion of the buffer.
    ///
    /// This includes the filled portion.
    #[inline]
    pub fn initialized_mut(&mut self) -> &mut [u8] { ... }

    /// Returns a mutable reference to the unfilled part of the buffer without ensuring that it has been fully
    /// initialized.
    ///
    /// # Safety
    ///
    /// The caller must not de-initialize portions of the buffer that have already been initialized.
    #[inline]
    pub unsafe fn unfilled_mut(&mut self) -> &mut [MaybeUninit<u8>] { ... }

    /// Returns a mutable reference to the unfilled part of the buffer, ensuring it is fully initialized.
    ///
    /// Since `ReadBuf` tracks the region of the buffer that has been initialized, this is effectively "free" after
    /// the first use.
    #[inline]
    pub fn initialize_unfilled(&mut self) -> &mut [u8] { ... }

    /// Returns a mutable reference to the first `n` bytes of the unfilled part of the buffer, ensuring it is
    /// fully initialized.
    ///
    /// # Panics
    ///
    /// Panics if `self.remaining()` is less than `n`.
    #[inline]
    pub fn initialize_unfilled_to(&mut self, n: usize) -> &mut [u8] { ... }

    /// Returns the number of bytes at the end of the slice that have not yet been filled.
    #[inline]
    pub fn remaining(&self) -> usize { ... }

    /// Clears the buffer, resetting the filled region to empty.
    ///
    /// The number of initialized bytes is not changed, and the contents of the buffer are not modified.
    #[inline]
    pub fn clear(&mut self) { ... }

    /// Increases the size of the filled region of the buffer.
    ///
    /// The number of initialized bytes is not changed.
    ///
    /// # Panics
    ///
    /// Panics if the filled region of the buffer would become larger than the initialized region.
    #[inline]
    pub fn add_filled(&mut self, n: usize) { ... }

    /// Sets the size of the filled region of the buffer.
    ///
    /// The number of initialized bytes is not changed.
    ///
    /// Note that this can be used to *shrink* the filled region of the buffer in addition to growing it (for
    /// example, by a `Read` implementation that compresses data in-place).
    ///
    /// # Panics
    ///
    /// Panics if the filled region of the buffer would become larger than the initialized region.
    #[inline]
    pub fn set_filled(&mut self, n: usize) { ... }

    /// Asserts that the first `n` unfilled bytes of the buffer are initialized.
    ///
    /// `ReadBuf` assumes that bytes are never de-initialized, so this method does nothing when called with fewer
    /// bytes than are already known to be initialized.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the first `n` unfilled bytes of the buffer have already been initialized.
    #[inline]
    pub unsafe fn assume_init(&mut self, n: usize) { ... }

    /// Appends data to the buffer, advancing the written position and possibly also the initialized position.
    ///
    /// # Panics
    ///
    /// Panics if `self.remaining()` is less than `buf.len()`.
    #[inline]
    pub fn append(&mut self, buf: &[u8]) { ... }
}
```

The `Read` trait uses this type in some of its methods:

```rust
pub trait Read {
    /// Pull some bytes from this source into the specified buffer.
    ///
    /// This is equivalent to the `read` method, except that it is passed a `ReadBuf` rather than `[u8]` to allow use
    /// with uninitialized buffers. The new data will be appended to any existing contents of `buf`.
    ///
    /// The default implementation delegates to `read`.
    fn read_buf(&mut self, buf: &mut ReadBuf<'_>) -> io::Result<()> {
        let n = self.read(buf.initialize_unfilled())?;
        buf.add_filled(n);
        Ok(())
    }

    ...
}
```

The `ReadBuf` type wraps a buffer of maybe-initialized bytes and tracks how much of the buffer has already been
initialized. This tracking is crucial because it avoids repeated initialization of already-initialized portions of the
buffer. It additionally provides the guarantee that the initialized portion of the buffer *is actually initialized*! A
subtle characteristic of `MaybeUninit` is that you can de-initialize values in addition to initializing them, and this
API protects against that.

It additionally tracks the amount of data read into the buffer directly so that code working with `Read` implementations
can be guaranteed that the region of the buffer that the reader claims was written to is minimally initialized.
Thinking back to the `BrokenReader` in the motivation section, the worst an implementation can now do (without writing
unsound unsafe code) is to fail to actually write useful data into the buffer. Code using a `BrokenReader` may see bad
data in the buffer, but the bad data at least has defined contents now!

Note that `read` is still a required method of the `Read` trait. It can be easily written to delegate to `read_buf`:

```rust
impl Read for SomeReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut buf = ReadBuf::new(buf);
        self.read_buf(&mut buf)?;
        Ok(buf.filled().len())
    }

    fn read_buf(&mut self, buf: &mut ReadBuf<'_>) -> io::Result<()> {
        ...
    }
}
```

Some of `Read`'s convenience methods will be modified to take advantage of `read_buf`, and some new convenience methods
will be added:

```rust
pub trait Read {
    /// Read the exact number of bytes required to fill `buf`.
    ///
    /// This is equivalent to the `read_exact` method, except that it is passed a `ReadBuf` rather than `[u8]` to
    /// allow use with uninitialized buffers.
    fn read_buf_exact(&mut self, buf: &mut ReadBuf<'_>) -> io::Result<()> {
        while buf.remaining() > 0 {
            let prev_filled = buf.filled().len();
            match self.read_buf(&mut buf) {
                Ok(()) => {}
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }

            if buf.filled().len() == prev_filled {
                return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "failed to fill buffer"));
            }
        }

        Ok(())
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        let initial_len = buf.len();

        let mut initialized = 0;
        loop {
            if buf.len() == buf.capacity() {
                buf.reserve(32);
            }

            let mut read_buf = ReadBuf::uninit(buf.spare_capacity_mut());
            unsafe {
                read_buf.assume_init(initialized);
            }

            match self.read_buf(&mut read_buf) {
                Ok(()) => {}
                Err(e) if e.kind() = io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }

            if read_buf.filled().is_empty() {
                break;
            }

            initialized = read_buf.initialized().len() - read_buf.filled().len();
            let new_len = buf.len() + read_buf.filled().len();
            unsafe {
                buf.set_len(new_len);
            }
        }

        Ok(buf.len() - initial_len)
    }
}

pub fn copy<R, W>(reader: &mut R, writer: &mut W) -> io::Result<u64>
where
    R: Read,
    W: Write,
{
    let mut buf = [MaybeUninit::uninit(); 4096];
    let mut buf = ReadBuf::uninit(&mut buf);
    let mut len = 0;

    loop {
        match reader.read_buf(&mut buf) {
            Ok(()) => {},
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };

        if buf.filled().is_empty() {
            break;
        }

        len += buf.filled().len() as u64;
        writer.write_all(buf.filled())?;
        buf.clear();
    }

    Ok(len)
}
```

The existing `std::io::Initializer` type and `Read::initializer` method will be removed.

Vectored reads use a similar API:

```rust
/// A possibly-uninitialized version of `IoSliceMut`.
///
/// It is guaranteed to have exactly the same layout and ABI as `IoSliceMut`.
pub struct MaybeUninitIoSliceMut<'a> { ... }

impl<'a> MaybeUninitIoSliceMut<'a> {
    /// Creates a new `MaybeUninitIoSliceMut` from a slice of maybe-uninitialized bytes.
    #[inline]
    pub fn new(buf: &'a mut [MaybeUninit<u8>]) -> MaybeUninitIoSliceMut<'a> { ... }
}

impl<'a> Deref for MaybeUninitIoSliceMut<'a> {
    type Target = [MaybeUninit<u8>];

    ...
}

impl<'a> DerefMut for MaybeUninitIoSliceMut<'a> { ... }


/// A wrapper over a set of incrementally-initialized buffers.
pub struct ReadBufs<'a> { ... }

impl<'a> ReadBufs<'a> {
    /// Creates a new `ReadBufs` from a set of fully initialized buffers.
    #[inline]
    pub fn new(bufs: &'a mut [IoSliceMut<'a>]) -> ReadBufs<'a> { ... }

    /// Creates a new `ReadBufs` from a set of fully uninitialized buffers.
    ///
    /// Use `assume_init` if part of the buffers are known to be already initialized.
    #[inline]
    pub fn uninit(bufs: &'a mut [MaybeUninitIoSliceMut<'a>]) -> ReadBufs<'a> { ... }

    ...
}

pub trait Read {
    /// Pull some bytes from this source into the specified set of buffers.
    ///
    /// This is equivalent to the `read_vectored` method, except that it is passed a `ReadBufs` rather than
    /// `[IoSliceMut]` to allow use with uninitialized buffers. The new data will be appended to any existing contents
    /// of `bufs`.
    ///
    /// The default implementation delegates to `read_vectored`.
    fn read_buf_vectored(&mut self, bufs: &mut ReadBufs<'_>) -> io::Result<()> {
        ...
    }
}
```

# Drawbacks
[drawbacks]: #drawbacks

This introduces a nontrivial amount of complexity to one of the standard library's core traits, and results in sets of
almost-but-not-quite identical methods (`read`/`read_buf`, `read_exact`/`read_buf_exact`, etc). It's unfortunate that
an implementor of `Read` based on `read_buf` needs to add a boilerplate `read` implementation.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Any solution to this problem needs to satisfy a set of constraints:

1. It needs to be backwards compatible. Duh.
2. It needs to be *efficiently* backwards compatible. Code that doesn't write `unsafe` should not be penalized by the
    new APIs. For example, code working with a reader written before these new APIs were introduced should not become
    slower once that code starts trying to use the new APIs.
3. It must be compatible with `dyn Read`. Trait objects are used pervasively in IO code, so a solution can't depend on
    monomorphization or specialization.
4. It needs to work with both normal and vectored IO (via `read_vectored`).
5. It needs to be composable. Readers are very commonly nested (e.g. `GzipReader<TlsStream<TcpStream>>`), and wrapper
    readers should be able to opt-in to fast paths supported by their inner reader.
6. A reader that does want to work directly with uninitialized memory does, at some reasonable point, need to write the
    word `unsafe`.

This RFC covers the proposed solution. For in-depth coverage of other options and the rationale for this particular
approach over others, please refer to this [Dropbox Paper writeup](https://paper.dropbox.com/doc/IO-Buffer-Initialization--Ax97Yz2_GUH23hVjfDf4JhCAAQ-MvytTgjIOTNpJAS6Mvw38)
or my [discussion with Niko Matsakis](http://smallcultfollowing.com/babysteps/blog/2020/01/20/async-interview-5-steven-fackler/).

The proposal in the Dropbox Paper does differ from the proposal in this RFC in one significant way: its definition of
`read_buf` returns an `io::Result<usize>` like `read` does, and the `ReadBuf` only tracks the initialized region and not
the written-to region:

```rust
pub trait Read {
    fn read_buf(&mut self, buf: &mut ReadBuf<'_>) -> io::Result<usize> { ... }
}
```

This has a subtle but important drawback. From the perspective of code working with a `Read` implementation, the
initialization state of the buffer can be trusted to be correct, but the number of bytes read cannot! This mix of
trusted and untrusted information can be quite a footgun for unsafe code working with a reader. For example,
`read_to_end` needs to remember to assert that the number of bytes read is less than the number of bytes initialized
before calling `set_len` on the `Vec<u8>` that it's reading into. Moving that bit of state into `ReadBuf` avoids the
issue by allowing `ReadBuf` to guarantee that these two values stay consistent.

The concept of `ReadBuf` is not inherently tied to working with `u8` buffers;  it could alternatively be parameterized
over the value type and hypothetically used in other contexts. However, the API for such a type can be iterated on
in an external crate.

# Prior art
[prior-art]: #prior-art

The standard library currently has the concept of a buffer "initializer". The `Read` trait has an (unstable) method
which returns an `Initializer` object which can take a `&mut [u8]` of uninitialized memory and initialize it as needed
for use with the associated reader. Then the buffer is just passed to `read` as normal.

The [`tokio::io::AsyncRead`](https://docs.rs/tokio/0.2.21/tokio/io/trait.AsyncRead.html) trait has a somewhat similar
approach, with a `prepare_uninitialized_buffer` method which takes a `&mut [MaybeUninit<u8>]` slice and initializes it
if necessary.

Refer to the links in the "Rationale and alternatives" section above for a discussion of the issues with these
approaches.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

Should `read_buf` return the number of bytes read like `read` does or should the `ReadBuf` track it instead? Some
operations, like checking for EOF, are a bit simpler if `read_buf` returns the value, but the confusion around what is
and is not trustworthy is worrysome for unsafe code working with `Read` implementations.

# Future possibilities
[future-possibilities]: #future-possibilities

Some of the complexity in the implementation of `read_to_end` above is due to having to manually track how much of the
`Vec<u8>`'s spare capacity has already been initialized between iterations of the read loop. There is probably some kind
of abstraction that could be defined to encapsulate that logic.

Users shouldn't be required to manually write a version of `read` that delegates to `read_buf`. We should be able to
eventually add a default implementation of `read`, along with a requirement that one of `read` and `read_buf` must be
overridden.
