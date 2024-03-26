- Feature Name: `unix_socket_ancillary_data_v2`
- Start Date: 2023-05-10
- RFC PR: [rust-lang/rfcs#3430](https://github.com/rust-lang/rfcs/pull/3430)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Redesign APIs related to sending and receiving Unix socket ancillary data.

The new APIs should provide safe and ergnomic access to commonly-used
functionality such as file descriptor passing, and also expose extension points
that third-party libraries can use to integrate platform-specific behavior.

# Motivation
[motivation]: #motivation

The existing APIs for ancillary data enabled by the `unix_socket_ancillary_data`
feature are incomplete, non-extensible, and difficult to use correctly. They
cannot be stabilized in their current state.

## API coverage

Ancillary data is widely used among Unix platforms for passing metadata about
a socket operation to userspace. Ancillary data consists of "control messages",
which are platform-specific datastructures usually defined by a C `struct`.
Examples include IP packet timestamps, Unix process PIDs/UIDs/GIDs, and SELinux
security labels.

The current implementation supports only two control message types:

- `SCM_RIGHTS` for transferring file descriptors via `AF_UNIX` sockets.
- `SCM_CREDS` / `SCM_CREDENTIALS` for identifying the peer of an `AF_UNIX` socket.

The only socket domain supported by the current implementation is `AF_UNIX`, via
the `UnixDatagram` and `UnixStream` types. The functions added to these types
do not support per-call options such as `MSG_OOB` or Linux's `MSG_DONTROUTE`.

## Non-extensible

Lack of support in `std` for platform-specific functionality may be acceptable
if the API exposes extension points for third-party libraries, but the current
implementation does not.

- Control messages are represented as an enum (`os::unix::net::AncillaryData`).
  It is not possible to third-party libraries to support additional control
  message types.

- The wrappers around `sendmsg` / `recvmsg` functions are implemented as
  inherent functions of `UnixDatagram` and `UnixStream`, rather than as a trait.
  Adding support for ancillary data to `TcpStream` and `UdpSocket` on Unix
  platforms would require adding similar functions to those types.
  
- Types defined in third-party libraries (for example to support Linux's
  `SOCK_SEQPACKET` or `SOCK_RAW`) cannot support ancillary data via the current
  public API.

It should be possible for a library to define its own control messages and/or
sockets, and have them integrate with the standard library's types

## Resistance to misuse

The current API is easy to use incorrectly, leading to crashes and mysterious
misbehavior.

- File descriptors are represented as `RawFd`. When trying to work with
  `SCM_RIGHTS` messages it is easy to either (1) leak open files, or (2)
  drop an open file before its descriptor has been sent.

- The `SocketAncillary` struct does not take ownership of file descriptors
  received from the socket, which may result in file descriptor exhaustion
  if the peer sends a larger `SCM_RIGHTS` message than expected.

- Various parts of the code interpret a user-provided `&[u8]` as a pointer to
  `struct cmsghdr` and inspect its fields, which may cause errors due to
  unaligned reads on some platforms.

The alignment problems may be fixable with a thorough code review, but the
current handling of file descriptors is incompatible with Rust's ownership
model. It would be better to use `BorrowedFd` and `OwnedFd` to represent the
lifetime of file descriptors encoded within ancillary data.


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

TODO: What level of explanation is appropriate? Should it be written assuming
general knowledge of ancillary data and the Unix sockets API?

## Examples

Sending a file descriptor:

```rust
use std::fs::File;
use std::os::unix::net::{AncillaryDataBuf, MessageSender, UnixStream};
use std::os::fd::AsFd;

fn send_file(stream: &UnixStream, file: File) -> std::io::Result<()> {
    let mut ancillary_buf = AncillaryDataBuf::new();
    ancillary_buf.add_file_descriptors(&[
        file.as_fd(),
    ]);
    let mut ancillary = ancillary_buf.to_ancillary_data();

    MessageSender::new(stream, b"\x00")
        .ancillary_data(&mut ancillary)
        .send()?;
    Ok(())
}
```

Receiving a file descriptor:

```rust
use std::fs::File;
use std::os::unix::net::{AncillaryDataBuf, MessageReceiver, UnixStream};

fn recv_file(stream: &UnixStream) -> std::io::Result<File> {
    // TODO: expose CMSG_SPACE() in a user-friendly way.
    const ANCILLARY_CAPACITY: usize = 100;

    let mut ancillary_buf = AncillaryDataBuf::with_capacity(ANCILLARY_CAPACITY);
    let mut ancillary = ancillary_buf.to_ancillary_data();

    let mut buf = [0u8; 1];
    MessageReceiver::new(stream, &mut buf)
        .ancillary_data(&mut ancillary)
        .recv()?;

    let mut received_fds: Vec<_> = ancillary.received_fds().collect();
    if received_fds.len() != 1 {
        // TODO: error handling (std::io::Error if not enough FDs returned)
        panic!("didn't receive enough FDs");
    }
    let received_fd = received_fds.pop().unwrap();
    Ok(File::from(received_fd))
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Control messages

Each control message is represented as a `ControlMessage` struct, which holds
a reference to the underlying data.

```rust
struct ControlMessage<'a>;

impl ControlMessage<'a> {
    fn new(
        cmsg_level: c_int,
        cmsg_type: c_int,
        data: &'a [u8],
    ) -> ControlMessage<'a>;

    // Opaque platform-specific integers; same as `struct cmsghdr` fields.
    fn cmsg_level(&self) -> c_int;
    fn cmsg_type(&self) -> c_int;

    // The type-specific data of this control message.
    fn data(&self) -> &[u8];

    // Whether this control message is truncated, such as by being received
    // into a too-short buffer.
    fn truncated(&self) -> bool;
}
```

A `&ControlMessages` represents a buffer full of control messages. It can be
inspected and iterated over to obtain `ControlMessage` values.


```rust
struct ControlMessages;

impl ControlMessages {
    fn from_bytes(bytes: &[u8]) -> &ControlMessages;
    fn as_bytes(&self) -> &[u8];
    fn is_empty(&self) -> bool;
    fn iter(&self) -> ControlMessagesIter<'_>;
}

impl<'a> IntoIterator for &'a ControlMessages {
    type Item = ControlMessage<'a>;
    type IntoIter = ControlMessagesIter<'a>;
}

struct ControlMessagesIter<'a>;

impl<'a> Iterator for ControlMessagesIter<'a> {
    type Item = ControlMessage<'a>;
}

impl ControlMessagesIter<'a> {
    // For inspecting non-iterable fragment in truncated buffer.
    fn into_bytes(self) -> &'a [u8];
}
```

## Ancillary data

### `struct AncillaryData`

An `AncillaryData` is responsible for combining the serialized control messages
with a notion of file descriptor ownership, ensuring that (1) borrowed FDs live
long enough to be sent, and (2) received FDs aren't leaked.

```rust
struct AncillaryData<'a, 'fd>;

impl Drop for AncillaryData;

impl AncillaryData<'a, 'fd> {
    fn new(
        control_messages_buf: &'a mut [MaybeUninit<u8>],
    ) -> AncillaryData<'a, 'fd>;

    // returns initialized portion of `control_messages_buf`.
    fn control_messages(&self) -> &ControlMessages;

    // copy a control message into the ancillary data; error on out-of-capacity.
    fn add_control_message(
        &mut self,
        control_message: impl Into<ControlMessage<'_>>,
    ) -> Result<(), AncillaryDataNoCapacity>;

    // Add an `SCM_RIGHTS` control message with given borrowed FDs.
    fn add_file_descriptors(
        &mut self,
        borrowed_fds: &[BorrowedFd<'fd>],
    ) -> Result<(), AncillaryDataNoCapacity>;

    // Transfers ownership of received FDs to the iterator.
    fn received_fds(&mut self) -> AncillaryDataReceivedFds<'_>;

    // Obtain a mutable buffer usable as the `msg_control` pointer in a call
    // to `sendmsg()` or `recvmsg()`.
    fn control_messages_buf(&mut self) -> Option<&mut [u8]>;

    // Update the control messages buffer length according to the result of
    // calling `sendmsg()` or `recvmsg()`.
    fn set_control_messages_len(&mut self, len: usize);

    // Scan the control messages buffer for `SCM_RIGHTS` and take ownership of
    // any file descriptors found within.
    unsafe fn take_ownership_of_scm_rights(&mut self);
}

struct AncillaryDataReceivedFds<'a>;

impl<'a> Iterator for AncillaryDataReceivedFds<'a> {
    type Item = OwnedFd;
}
```

#### `fn AncillaryData::received_fds`

By default an `AncillaryData` has no received FDs, and this method will
ignore FDs added via `add_file_descriptors`. The iterator holds a mutable
borrow on the `AncillaryData`, and will close any unclaimed FDs when
it's dropped.

Internally, the iteration works by scanning the control message buffer
for `SCM_RIGHTS`. As the iteration proceeds the buffer is mutated to set
the "taken" FDs to `-1` (a sentinal value for Unix file descriptors).

#### `fn AncillaryData::control_messages_buf`

The returned slice, if not `None`, will (1) be non-empty and (2) have the same
length as the `control_messages_buf` passed to `AncillaryData::new()`. Any
portion of the buffer not initialized by `add_control_message` or
`add_file_descriptors` will be zeroed.

When this method is called the `AncillaryData` will have its control
messages length cleared, so a subsequent call to `control_messages_buf()`
returns `None`. If the `AncillaryData` contains received FDs, they will
be closed.

#### `fn AncillaryData::set_control_messages_len`

Does not take ownership of FDs in any `SCM_RIGHTS` control messages that
might exist within the new buffer length.

**Panics**:
 * if `len > control_messages_buf.len()`
 * if `control_messages_buf()` hasn't been called to clear the length.

The second panic condition means that creating an `AncillaryData` and then
immediately calling `set_control_messages_len` will panic to avoid potentially
reading uninitialized data.

Also, calling `set_control_messages_len()` twice without an intervening
`control_messages_buf()` will panic to avoid leaking received FDs.

#### `fn AncillaryData::take_ownership_of_scm_rights`

**Panics**:
  * if `set_control_messages_len()` hasn't been called since the most
    recent call to `control_messages_buf()`.
    * That method is what keeps track of how much of the received buffer
      contains owned FDs, and trying to take ownership of `SCM_RIGHTS` without
      knowing how much to scan is almost certainly a programming error.

**Safety**: contents of control messages become `OwnedFd`, so this has all
the safety requirements of `OwnedFd::from_raw_fd()`.

### `struct AncillaryDataBuf`

An `AncillaryDataBuf` is an owned variant of `AncillaryData`, using heap
allocation (an internal `Vec<u8>`). It exposes a subset of the `Vec` capacity
management methods.

```rust
struct AncillaryDataBuf<'fd>;

impl AncillaryDataBuf<'fd> {
    fn new() -> AncillaryDataBuf<'static>;
    fn with_capacity(capacity: usize) -> AncillaryDataBuf<'static>;

    fn capacity(&self) -> usize;

    fn control_messages(&self) -> &ControlMessages;

    // copy a control message into the ancillary data; panic on alloc failure.
    fn add_control_message(
        &mut self,
        control_message: impl Into<ControlMessage<'_>>,
    );

    // Add an `SCM_RIGHTS` control message with given borrowed FDs; panic on
    // alloc failure.
    fn add_file_descriptors(&mut self, borrowed_fds: &[BorrowedFd<'fd>]);

    // Used to obtain `AncillaryData` for passing to send/recv calls.
    fn to_ancillary_data<'a>(&'a mut self) -> AncillaryData<'a, 'fd>;

    // Clears the control message buffer, without affecting capacity.
    //
    // This will not leak FDs because the `AncillaryData` type holds a mutable
    // reference to the `AncillaryDataBuf`, so if `clear()` is called then there
    // are no outstanding `AncillaryData`s and thus no received FDs.
    fn clear(&mut self);

    // as in Vec
    fn reserve(&mut self, capacity: usize);
    // as in Vec
    fn reserve_exact(&mut self, capacity: usize);

    // as in Vec
    fn try_reserve(
        &mut self,
        capacity: usize,
    ) -> Result<(), TryReserveError>;

    // as in Vec
    fn try_reserve_exact(
        &mut self,
        capacity: usize,
    ) -> Result<(), TryReserveError>;
}

impl Extend<ControlMessage<'_>> for AncillaryDataBuf;
impl Extend<&ControlMessage<'_>> for AncillaryDataBuf;
```

#### `fn AncillaryDataBuf::to_ancillary_data`

The returned `AncillaryData` will be initialized with the same control messages,
capacity, and borrowed FDs as the `AncillaryDataBuf`. Specifically, it's as if
the entire capacity of the internal `Vec` is passed to `AncillaryData::new()`.

When the `AncillaryData` is dropped its received FDs will be closed. The
`AncillaryDataBuf` does not retain ownership of received FDs. Otherwise the
API to reuse an `AncillaryDataBuf` between calls gets really complicated.

The only subtle part of `to_ancillary_data` is that it transfers logical
ownership of the control messages, but not the control messages *buffer*. So
after calling this function the `AncillaryDataBuf::control_messages()` method
returns an empty `&ControlMessages`, and any calls to `add_control_message` or
`add_file_descriptors`. It's basically an implicit `clear()`.

## Sending messages

### The `SendMessage` and `SendMessageTo` traits

These traits are implemented for socket types that implement `sendmsg`. They
are equivalent to the current `send_vectored_with_ancillary` and
`send_vectored_with_ancillary_to` functions, but also accept `SendOptions` for
per-send flags, and aren't specific to `AF_UNIX` sockets.

```rust
trait SendMessage {
    fn send_message(
        &self,
        bufs: &[IoSlice<'_>],
        ancillary_data: &mut AncillaryData<'_, '_>,
        options: SendOptions,
    ) -> io::Result<usize>;
}

trait SendMessageTo {
    type SocketAddr;

    fn send_message_to(
        &self,
        addr: &Self::SocketAddr,
        bufs: &[IoSlice<'_>],
        ancillary_data: &mut AncillaryData<'_, '_>,
        options: SendOptions,
    ) -> io::Result<usize>;
}
```

The `SendMessage` trait will be implemented for:
- `std::net::TcpStream`
- `std::net::UdpSocket`
- `std::os::unix::net::UnixDatagram`
- `std::os::unix::net::UnixStream`

The `SendMessageTo` trait will be implemented for:
- `std::net::UdpSocket`
- `std::os::unix::net::UnixDatagram`

These traits aren't sealed, so they can be implemented by third-party libraries.

### `SendOptions`

The `std::os::unix::net::SendOptions` type wraps flags passed to the Unix
sockets API functions [`send`] and [`sendmsg`]. It has inherent methods for
options defined by the POSIX standard.

[`send`]: https://pubs.opengroup.org/onlinepubs/9699919799/functions/send.html
[`sendmsg`]: https://pubs.opengroup.org/onlinepubs/9699919799/functions/sendmsg.html

```rust
struct SendOptions;

impl Default for SendOptions;

impl SendOptions {
    fn new() -> SendOptions;

    fn as_send_flags(&self) -> c_int;

    fn custom_flags(&mut self, flags: c_int) -> &mut Self;

    // MSG_EOR
    fn end_of_record(&mut self, end_of_record: bool) -> &mut Self;

    // MSG_OOB
    fn out_of_band(&mut self, out_of_band: bool) -> &mut Self;

    // MSG_NOSIGNAL
    fn no_signal(&mut self, no_signal: bool) -> &mut Self;
}
```

### `MessageSender`

The `MessageSender` helper struct provides an ergonomic API around the
`SendMessage` / `SendMessageTo` traits.

```rust
struct MessageSender<S, 'a>;

impl<S, 'a> MessageSender<S, 'a> {
    fn new(
        socket: &'a S,
        buf: &'a [u8],
    ) -> MessageSender<S, 'a>;

    fn new_vectored(
        socket: &'a S,
        bufs: &'a [IoSlice<'a>],
    ) -> MessageSender<S, 'a>;

    fn ancillary_data(
        &mut self,
        ancillary_data: &'a mut AncillaryData<'_, '_>,
    ) -> &mut MessageSender<S, 'a>;

    fn options(
        &mut self,
        options: SendOptions,
    ) -> &mut MessageSender<S, 'a>;
}

impl<S: SendMessage> MessageSender<S, '_> {
    fn send(&mut self) -> io::Result<usize>;
}

impl<S: SendMessageTo> MessageSender<S, '_> {
    fn send_to(&mut self, addr: &S::SocketAddr) -> io::Result<usize>;
}
```

## Receiving messages

### The `RecvMessage` and `RecvMessageFrom` traits

These traits are implemented for socket types that implement `recvmsg`. They
are equivalent to the current `recv_vectored_with_ancillary` and
`recv_vectored_with_ancillary_from` functions, but also accept `RecvOptions`
for per-recv flags, and aren't specific to `AF_UNIX` sockets.

```rust
trait RecvMessage {
    fn recv_message(
        &self,
        bufs: &mut [IoSliceMut<'_>],
        ancillary_data: &mut AncillaryData<'_, '_>,
        options: RecvOptions,
    ) -> io::Result<(usize, RecvResult)>;
}

trait RecvMessageFrom {
    type SocketAddr;

    fn recv_message_from(
        &self,
        bufs: &mut [IoSliceMut<'_>],
        ancillary_data: &mut AncillaryData<'_, '_>,
        options: RecvOptions,
    ) -> io::Result<(usize, RecvResult, Self::SocketAddr)>;
}
```

The `RecvMessage` trait will be implemented for:
- `std::net::TcpStream`
- `std::net::UdpSocket`
- `std::os::unix::net::UnixDatagram`
- `std::os::unix::net::UnixStream`

The `RecvMessageFrom` trait will be implemented for:
- `std::net::UdpSocket`
- `std::os::unix::net::UnixDatagram`

These traits aren't sealed, so they can be implemented by third-party libraries.

### `RecvOptions`

The `std::os::unix::net::RecvOptions` type wraps flags passed to the Unix
sockets API functions [`recv`] and [`recvmsg`]. It has inherent methods for
options defined by the POSIX standard.

The `MSG_CMSG_CLOEXEC` flag is always set on platforms for which it is defined.

[`recv`]: https://pubs.opengroup.org/onlinepubs/9699919799/functions/recv.html
[`recvmsg`]: https://pubs.opengroup.org/onlinepubs/9699919799/functions/recvmsg.html

```rust
struct RecvOptions;

impl Default for RecvOptions;

impl RecvOptions {
    fn new() -> RecvOptions;

    fn as_recv_flags(&self) -> c_int;

    fn custom_flags(&mut self, flags: c_int) -> &mut Self;

    // MSG_OOB
    fn out_of_band(&mut self, out_of_band: bool) -> &mut Self;

    // MSG_PEEK
    fn peek(&mut self, peek: bool) -> &mut Self;

    // MSG_WAITALL
    fn wait_all(&mut self, wait_all: bool) -> &mut Self;
}
```

### `RecvResult`

The `std::os::unix::net::RecvResult` type wraps flags returned from the Unix
sockets API function [`recvmsg`]. These flags indicate various status
conditions about the received data. It has inherent methods for flags defined
by the POSIX standard.

```rust
struct RecvResult;

impl RecvResult {
    fn new(flags: c_int) -> RecvResult;

    fn custom_flags(&self, flags: c_int) -> bool;

    // MSG_EOR
    fn end_of_record(&self) -> bool;

    // MSG_OOB
    fn out_of_band(&self) -> bool;

    // MSG_TRUNC
    fn normal_data_truncated(&self) -> bool;

    // MSG_CTRUNC
    fn control_data_truncated(&self) -> bool;
}
```

### `MessageReceiver`

The `MessageReceiver` helper struct provides an ergonomic API around the
`RecvMessage` / `RecvMessageFrom` traits.

```rust
struct MessageReceiver<S, 'a>;

impl<S, 'a> MessageReceiver<S, 'a> {
    fn new(
        socket: &'a S,
        buf: &'a mut [u8],
    ) -> MessageReceiver<S, 'a>;

    fn new_vectored(
        socket: &'a S,
        bufs: &'a mut [IoSliceMut<'a>],
    ) -> MessageReceiver<S, 'a>;

    fn ancillary_data(
        &mut self,
        ancillary_data: &'a mut AncillaryData<'_, '_>,
    ) -> &mut MessageReceiver<S, 'a>;

    fn options(
        &mut self,
        options: RecvOptions,
    ) -> &mut MessageReceiver<S, 'a>;
}

impl<S: RecvMessage> MessageReceiver<S, '_> {
    fn recv<S>(&mut self) -> io::Result<(usize, RecvResult)> {
}

impl<S: RecvMessageFrom> MessageReceiver<S, '_> {
    fn recv_from(&mut self) -> io::Result<(usize, RecvResult, S::SocketAddr)>;
}
```

## Linux-specific extensions

### `os::linux::net::SendOptionsExt`

Linux-specific flags to [`sendmsg`][linux-sendmsg].

[linux-sendmsg]: https://linux.die.net/man/2/sendmsg

```rust
trait SendOptionsExt: Sealed {
    // MSG_CONFIRM
    fn confirm(&mut self, confirm: bool) -> &mut Self;

    // MSG_DONTROUTE
    fn dont_route(&mut self, dont_route: bool) -> &mut Self;

    // MSG_DONTWAIT
    fn dont_wait(&mut self, dont_wait: bool) -> &mut Self;

    // MSG_MORE
    fn more(&mut self, more: bool) -> &mut Self;
}
```

### `os::linux::net::RecvOptionsExt`

Linux-specific flags to [`recvmsg`][linux-recvmsg].

[linux-recvmsg]: https://linux.die.net/man/2/recvmsg

```rust
trait RecvOptionsExt: Sealed {
    // MSG_DONTWAIT
    fn dont_wait(&mut self, dont_wait: bool) -> &mut Self;

    // MSG_ERRQUEUE
    fn err_queue(&mut self, err_queue: bool) -> &mut Self;

    // MSG_TRUNC
    fn truncate(&mut self, truncate: bool) -> &mut Self;
}

impl RecvOptionsExt for os::unix::net::RecvOptions;
```

### `os::linux::net::RecvResultExt`

Linux-specific flags returned by [`recvmsg`][linux-recvmsg].

```rust
trait RecvResultExt: Sealed {
    // MSG_ERRQUEUE
    fn err_queue(&self) -> bool;
}

impl RecvResultExt for os::unix::net::RecvResult;
```

### `os::linux::net::ScmCredentials`

Represents the `SCM_CREDENTIALS` control message, and can be converted to/from
a `ControlMessage`.

```rust
struct ScmCredentials;

impl ScmCredentials {
    fn matches(msg: &ControlMessage<'a>) -> bool;

    fn pid(&self) -> c_int;
    fn uid(&self) -> c_int;
    fn gid(&self) -> c_int;
}
```

### `os::linux::net::ScmSecurity`

Represents the `SCM_SECURITY` control message, and can be converted to/from
a `ControlMessage`.

```rust
struct ScmSecurity<'a>;

impl ScmSecurity<'_> {
    fn matches(msg: &ControlMessage<'a>) -> bool;

    fn selinux_security_label(&self) -> &CStr;
}
```

## FreeBSD-specific extensions

### `os::freebsd::net::SendOptionsExt`

FreeBSD-specific flags to [`sendmsg`][freebsd-sendmsg].

[freebsd-sendmsg]: https://man.freebsd.org/cgi/man.cgi?query=sendmsg&sektion=2

```rust
trait SendOptionsExt: Sealed {
    // MSG_DONTROUTE
    fn dont_route(&mut self, dont_route: bool) -> &mut Self;

    // MSG_DONTWAIT
    fn dont_wait(&mut self, dont_wait: bool) -> &mut Self;

    // MSG_EOF
    fn eof(&mut self, eof: bool) -> &mut Self;
}
```

### `os::freebsd::net::RecvOptionsExt`

FreeBSD-specific flags to [`recvmsg`][freebsd-recvmsg].

[freebsd-recvmsg]: https://man.freebsd.org/cgi/man.cgi?query=recvmsg&sektion=2

```rust
trait RecvOptionsExt: Sealed {
    // MSG_TRUNC
    fn truncate(&mut self, truncate: bool) -> &mut Self;

    // MSG_DONTWAIT
    fn dont_wait(&mut self, dont_wait: bool) -> &mut Self;
}
```

### `os::freebsd::net::ScmCreds`

Represents the `SCM_CREDS` control message, and can be converted to/from
a `ControlMessage`.

```rust
struct ScmCreds;

impl ScmCreds {
    fn matches(msg: &ControlMessage<'a>) -> bool;

    fn pid(&self) -> c_int;
    fn uid(&self) -> c_int;
    fn euid(&self) -> c_int;
    fn gid(&self) -> c_int;

    fn groups(&self) -> &[c_int];
}
```

## MacOS-specific extensions

### `os::macos::net::SendOptionsExt`

MacOS-specific flags to `sendmsg`.

```rust
trait SendOptionsExt: Sealed {
    // MSG_DONTROUTE
    fn dont_route(&mut self, dont_route: bool) -> &mut Self;
}
```

### `os::macos::net::ScmCreds`

Represents the `SCM_CREDS` control message, and can be converted to/from
a `ControlMessage`.

```rust
struct ScmCreds;

impl ScmCreds {
    fn matches(msg: &ControlMessage<'a>) -> bool;

    fn pid(&self) -> c_int;
    fn uid(&self) -> c_int;
    fn euid(&self) -> c_int;
    fn gid(&self) -> c_int;

    fn groups(&self) -> &[c_int];
}
```

# Drawbacks
[drawbacks]: #drawbacks

This RFC would significantly expand the public API surface of `os::unix::net`.

The handling of file descriptor ownership is more complex than the current
implementation, which uses `RawFd`. There may be soundness issues in the
conversion of `SCM_RIGHTS` into a `Vec<OwnedFd>`, for example if a way is
found to call `take_ownership_of_scm_rights` on a user-defined buffer from
safe code.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The primary alternative is to continue using the existing API, which has known
blockers to stabilization (see [Motiviation](#motivation)).

# Prior art
[prior-art]: #prior-art

This RFC would obsolete and/or significantly alter the implemenation of the
following unstable features:

- [`unix_socket_ancillary_data`](https://github.com/rust-lang/rust/issues/76915)
- [`unix_socket_peek`](https://github.com/rust-lang/rust/issues/76923)
- [`peer_credentials_unix_socket`](https://github.com/rust-lang/rust/issues/42839)

Several third-party Rust libraries provide access to ancillary data

- [`nix::sys::socket`](https://docs.rs/nix/0.26.2/nix/sys/socket/index.html)

Some other languages include ancillary data support as part of their standard
library's network functionality, in various levels of abstraction:

- C: The original BSD sockets API, nowadays included in every major libc.
- Python: [`socket.send_fds()`], [`socket.recv_fds()`], and the `socket.CMSG_*`
  functions.
- Go: The connection-specific `ReadMsg*` functions, such as
  [`(*net.UnixConn).ReadMsgUnix()`].
- Ruby: The [`Socket::AncillaryData`] class.

[`socket.recv_fds()`]: https://docs.python.org/3.11/library/socket.html#socket.recv_fds
[`socket.send_fds()`]: https://docs.python.org/3.11/library/socket.html#socket.send_fds
[`(*net.UnixConn).ReadMsgUnix()`]: https://pkg.go.dev/net#UnixConn.ReadMsgUnix
[`Socket::AncillaryData`]: https://docs.ruby-lang.org/en/3.2/Socket/AncillaryData.html

# Unresolved questions
[unresolved-questions]: #unresolved-questions

It might be nice to support a unified abstraction over `SCM_CREDS` or
equivalent on the minor Unix and Unix-like platforms ({Dragonfly,Net,Open}BSD,
Solaris, Fuchsia).
- My preference is to leave this to separate ACPs so that the MVP
  implementation can be developed on platforms with easy CI coverage.

Depending on how difficult they are to implement, some or all of the
OS-specific extensions described in this RFC might be handled as separate
features on their own stabilization schedule.
- For example, the new types in `os::freebsd::net::*` might be part of a
  `freebsd_ancillary_data_exts` feature if they cause issues similar to those
  experienced by the current implementation.

# Future possibilities
[future-possibilities]: #future-possibilities

For this initial version I plan to minimize the number of platform-specific
control message types supported by the standard library. Those types can be
added in future RFCs and/or ACPs, possibly after prototyping as third-party
libraries.

Specifically, I don't want to cover the Linux-specific control messages
related to TCP/UDP/IP in the initial stabilization.
