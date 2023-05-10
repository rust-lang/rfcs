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
use std::os::unix::net::{AncillaryData, MessageSender, UnixStream};
use std::os::fd::AsFd;

fn send_file(stream: &UnixStream, file: std::fs::File) -> std::io::Result<()> {
    let mut ancillary = AncillaryData::new();
    ancillary.add_file_descriptors(&[
        file.as_fd(),
    ]);

    MessageSender::new(stream, b"\x00")
        .ancillary_data(&mut ancillary)
        .send(&stream)?;
    Ok(())
}
```

Receiving a file descriptor:

```rust
use std::os::unix::net::{AncillaryData, MessageReceiver, UnixStream};

fn recv_file(stream: &UnixStream) -> std::io::Result<std::fs::File> {
    // TODO: expose CMSG_SPACE() in a user-friendly way.
    const ANCILLARY_CAPACITY: usize = 100;

    let mut ancillary = AncillaryData::with_capacity(ANCILLARY_CAPACITY);
    let mut buf = [0u8; 1];
    MessageReceiver::new(stream, &mut buf)
        .ancillary_data(&mut ancillary)
        .recv(&stream)?;

    // TODO: error handling (std::io::Error if not enough FDs returned)
    let mut owned_fds = ancillary.take_owned_fds().unwrap();
    let sent_fd = owned_fds.swap_remove(0);
    Ok(sent_fd.into())
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
    fn as_bytes(&self) -> &[u8];
    fn is_empty(&self) -> bool;
    fn len(&self) -> usize;
}

impl<'a> IntoIterator for &'a ControlMessages {
    type Item = ControlMessage<'a>;
    type IntoIter = ControlMessagesIter<'a>;
}

struct ControlMessagesIter<'a> { ... }

impl<'a> Iterator for ControlMessagesIter<'a> {
    type Item = ControlMessage<'a>;
}
```

A `ControlMessagesBuf` is the owned variant of `ControlMessages`. It exposes a
subset of the `Vec` capacity management functions, with a public API that only
allows operations that won't reduce its length (since that would risk losing
information about received file descriptors).

```rust
struct ControlMessagesBuf;

impl ControlMessagesBuf {
    fn new() -> ControlMessagesBuf;
    fn with_capacity(capacity: usize) -> ControlMessagesBuf;

    fn capacity(&self) -> usize;

    fn push(&mut self, message: &ControlMessage<'_>);
    fn extend_from_slice(&mut self, messages: &[ControlMessage<'_>]);

    fn reserve(&mut self, additional: usize);

    fn try_reserve(
        &mut self,
        additional: usize,
    ) -> Result<(), TryReserveError>;

    fn reserve_exact(&mut self, additional: usize);

    fn try_reserve_exact(
        &mut self,
        additional: usize
    ) -> Result<(), TryReserveError>
}

impl AsRef<ControlMessages> for ControlMessagesBuf;
impl AsMut<ControlMessages> for ControlMessagesBuf;

impl Deref for ControlMessagesBuf {
    type Target = ControlMessages;
}
impl DerefMut for ControlMessagesBuf;

impl<'a> IntoIterator for &'a ControlMessagesBuf {
    type Item = ControlMessage<'a>;
    type IntoIter = ControlMessagesIter<'a>;
}
```

## Ancillary data

The `AncillaryData` struct is responsible for combining the serialized control
messages with a notion of file descriptor ownership, ensuring that (1) borrowed
FDs live long enough to be sent, and (2) received FDs aren't leaked.

```rust
struct AncillaryData<'fd>;

impl AncillaryData<'fd> {
    fn new() -> AncillaryData<'fd>;
    fn with_capacity(capacity: usize) -> AncillaryData<'fd>;

    fn control_messages(&self) -> &ControlMessages;
    fn control_messages_mut(&mut self) -> &mut ControlMessagesBuf;

    // Helper for control_messages_mut().push(message.into())
    fn add_control_message<'b>(&mut self, message: impl Into<ControlMessage<'b>>);

    // Adds FDs to the control messages buffer so they can be sent.
    fn add_file_descriptors(&mut self, borrowed_fds: &impl AsRef<[BorrowedFd<'fd>]>);

    // Clears the control message buffer and drops owned FDs.
    fn clear(&mut self);

    // Takes ownership of FDs received from the socket. After the FDs are
    // taken, returns `None` until the next call to `finish_recvmsg`.
    fn take_owned_fds(&mut self) -> Option<Vec<OwnedFd>>;

    // API for sockets performing `recvmsg`:
    //
    // 1. Call `start_recvmsg` to obtain a mutable buffer to pass into the
    //    `cmsghdr`. The buffer len equals `control_messages().capacity()`.
    //
    // 2. Call `finish_recvmsg` to update the control messages buffer length
    //    according to the new `cmsghdr` length. This function will also take
    //    ownership of FDs received from the socket.
    //
    // The caller is responsible for ensuring that the control messages buffer
    // content and length are provided by a successful call to `recvmsg`.
    fn start_recvmsg(&mut self) -> Option<&mut [u8]>;
    unsafe fn finish_recvmsg(&mut self, control_messages_len: usize);

    // API for sockets performing `sendmsg` -- basically the same as the
    // `recvmsg` version, but doesn't scan the message buffer for `SCM_RIGHTS`
    // to take ownership of.
    //
    // The caller is responsible for ensuring that the control messages buffer
    // content and length are provided by a successful call to `sendmsg`.
    fn start_sendmsg(&mut self) -> Option<&mut [u8]>;
    unsafe fn finish_sendmsg(&mut self, control_messages_len: usize);
}
```

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
        ancillary_data: &mut AncillaryData<'_>,
        options: SendOptions,
    ) -> io::Result<usize>;
}

trait SendMessageTo {
    type SocketAddr;

    fn send_message_to(
        &self,
        addr: &Self::SocketAddr,
        bufs: &[IoSlice<'_>],
        ancillary_data: &mut AncillaryData<'_>,
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
        ancillary_data: &'a mut AncillaryData<'a>,
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
        ancillary_data: &mut AncillaryData<'_>,
        options: RecvOptions,
    ) -> io::Result<(usize, RecvResult)>;
}

trait RecvMessageFrom {
    type SocketAddr;

    fn recv_message_from(
        &self,
        bufs: &mut [IoSliceMut<'_>],
        ancillary_data: &mut AncillaryData<'_>,
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
        ancillary_data: &'a mut AncillaryData<'a>,
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
found to call `finish_recvmsg` on a user-defined buffer from safe code.

The API described in this RFC doesn't provide a way for a stack-allocated
buffer to be used as `AncillaryData` capacity.

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
