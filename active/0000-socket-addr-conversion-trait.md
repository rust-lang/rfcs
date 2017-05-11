- Start Date: (2014-07-21)
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Make `std::io::net` socket-wrapping structs (`TcpStream`, `UdpStream` etc.) accept bounded generic
type for address.

# Motivation

Currently there is an inconsistency in our networking API when an address to connect to/listen
on should be provided. `TcpStream::connect()` method accepts an `&str` address and `u16` port;
`TcpStream::connect_timeout()` accepts `SocketAddr`; `TcpListener::bind()` accepts `&str` and `u16`;
`UdpSocket::bind()` and `UdpSocket::connect()` accept `SocketAddr`.

This inconsistency itself is disturbing; however, there is more than this. Even if we changed
everything to use `&str/u16` pair or back to `SocketAddr`, there still will be usability problems.
`SocketAddr` is inconvenient to construct manually; `&str/u16`, on the other hand, requires needless
conversion from `SocketAddr` and back when `SocketAddr` is *the* original value of the address (for
example, it can be retrieved from `getifaddrs()` system call).

Ideally, the user should be able to provide anything resembling IP address/port pair, and the
library should convert it internally to `SocketAddr` to use in low-level socket calls. And Rust
has all necessary facilities to allow so.

# Detailed design

A new trait is introduced to the standard library:

```rust
pub trait ToSocketAddr {
    fn to_socket_addr(&self) -> IoResult<SocketAddr> {
        match self.to_socket_addr_all().map(|v| v.move_iter().next()) {
            Ok(Some(addr)) => addr,
            Ok(None) => Err(/* some error on that no addresses are available */),
            Err(e) => Err(e)
        }
    }

    #[inline]
    fn to_socket_addr_all(&self) -> IoResult<Vec<SocketAddr>> {
        self.to_socket_addr().map(|a| vec![a])
    }
}
```

This trait contains two methods which are defined in terms of each other. This is done to simplify
implementations; for those types which can be represented with multiple `SocketAddr`esses,
`to_socket_addr_all()` method should be implemented, and its counterpart will be automatically
there; for those types which can only be represented by one `SocketAddr`ess, `to_socket_addr()`
method should be implemented, and the trivial vector implementation will be readily available.

The method returning a vector is required because some values correspond to potentially multiple IP
addresses, for example, host names. All these addresses are sometimes needed in the underlying
libraries to choose the most appropriate one.

Then all functions/methods which need an address accept a generic parameter bounded by this trait:

```rust
impl TcpStream {
    pub fn connect<A: ToSocketAddr>(addr: A) -> IoResult<TcpStream> { ... }
    pub fn connect_timeout<A: ToSocketAddr>(addr: A, timeout_ms: u64) -> IoResult<TcpStream> { ... }
}

impl TcpListener {
    pub fn bind<A: ToSocketAddr>(addr: A) -> IoResult<TcpListener> { ... }
}

impl UdpSocket {
    pub fn bind<A: ToSocketAddr>(addr: A) -> IoResult<UdpSocket> { ... }
    pub fn connect<A: ToSocketAddr>(self, other: A) -> UdpStream { ... }
}
```

This trait is implemented for a number of types:

```rust
impl ToSocketAddr for SocketAddr {
    #[inline]
    fn to_socket_addr(&self) -> IoResult<SocketAddr> { Ok(*self) }
}

impl ToSocketAddr for (IpAddr, u16) {
    #[inline]
    fn to_socket_addr(&self) -> IoResult<SocketAddr> {
        let (ip, port) = *self;
        Ok(SocketAddr { ip: ip, port: port });
    }
}

// Better use `<S: Str>` instead of `&'a str`, but it will conflict with other implementations
// in the current trait matching system

impl<'a> ToSocketAddr for (&'a str, u16) {
    fn to_socket_addr_all(&self) -> IoResult<Vec<SocketAddr>> {
        let (host, port) = *self;
        get_host_addresses(host).map(|v|
            v.move_iter().map(|a| SocketAddr { 
                ip: a, 
                port: port
            }).collect()
        )
    }
}

impl<'a> ToSocketAddr for &'a str {
    // accepts strings like 'localhost:12345'
    fn to_socket_addr(&self) -> IoResult<SocketAddr> {
        // split the string by ':' and convert the second part to u16
    }
}
```

This will allow calling aforementioned methods with different types of arguments:
```rust
let mut stream = TcpStream::connect("localhost:12345").unwrap();
// or
let mut stream = TcpStream::connect(("localhost", 12345)).unwrap();
// or
let mut stream = TcpStream::connect((Ipv4Addr(127, 0, 0, 1), 12345)).unwrap();
// or
let addr: SocketAddr = first_address_for_interface("eth0").unwrap();
let mut stream = TcpStream::connect_timeout(addr, 10_000).unwrap();
```

This provides great flexibility, does not hamper performance at all (due to static dispatch) and
still gives nice and clean interface.

Underlying socket API may use `to_socket_addr_all()` method to obtain all available addresses and
make a decision on which to use, if it is required.

Note that this pattern is already used in `std`, namely, in `std::path` module. There is a trait,
[`BytesContainer`](http://doc.rust-lang.org/std/path/trait.BytesContainer.html), which represents
generic string of bytes, and `Path` constructors accept values of types implementing this trait.
This allow calling `Path::new()` with string slices, byte vectors or even other paths.

# Drawbacks

Adding such "overloading" trait increases complexity of implementation slightly. It also somewhat increases
cognitive load on the programmer when he or she reads API documentation because it won't be immediately
obvious which arguments constructor methods can accept. However, this can be mitigated easily by
emphasizing polymorphic behavior of such methods in the same documentation. Apparently, there seem
to be no problems with `Path` API now.

Another drawback is esthetical: currently connecting to an address represented as a `&str/u16` looks
like this:
```rust
let mut stream = TcpStream::connect("localhost", 12345).unwrap();
```
Under this proposal it will look like this:
```rust
let mut stream = TcpStream::connect(("localhost", 12345)).unwrap();
```
Note an additional set of parentheses. This may be somewhat confusing. However, implementing
`ToSocketAddr` for `&str` greatly reduces this problem: when a fixed address is needed, it can be
written as a string directly (see example above).

# Alternatives

Leave everything as it is and be stuck with inconsistent and sometimes inconvenient API forever.

One thing that really should be done regardless of whether this proposal is accepted or not is
that all aforementioned methods have to be unified, for example, they all should accept `&str/u16`
arguments, without variations. This counts as an alternative.

Another alternative is to provide several methods, for example, one accepting `&str/u16` and another
`SocketAddr`. However, this will increase amount of available methods, and they will have different
names as Rust does not have methods overloading, cluttering socket interfaces.

Even another possibility is to introduce new enum for this purpose:
```rust
enum SocketAddrWrapper<'a> {
    SocketAddr(SocketAddr),
    HostPort(&'a str, u16),
    Address(&'a str)
}

impl<'a> SocketAddrWrapper<'a> {
    pub fn to_socket_addr(&self) -> SocketAddr {
        match *self {
            // whatever
        }
    }
}
```
This will require calling constructor methods like this:
```rust
let mut stream = TcpStream::connect(Address("localhost:12345")).unwrap();
// or
let addr: SocketAddr = first_address_for_interface("eth0").unwrap();
let mut stream = TcpStream::connect(SocketAddr(addr)).unwrap();
```

However, this alternative is less flexible (since enums are closed) and more syntactically heavy at
the call site, which is undesirable.

# Unresolved questions

Exact name of the trait and its method is an open question.

Is it OK that the trait contains mutually recursive default methods? This is in spirit of Haskell
`Eq` trait with its `(==)` and `(/=)` methods, but it is debatable whether such approach is
appropriate in Rust.
