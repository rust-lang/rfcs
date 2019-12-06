- Feature Name: `core_net_ipaddr_types`
- Start Date: 2019-12-06
- RFC PR: [rust-lang/rfcs#2832](https://github.com/rust-lang/rfcs/pull/2832)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Make the `IpAddr`, `Ipv4Addr` and `Ipv6Addr` types available in `no_std`
contexts by moving them into a `core::net` module.

# Motivation
[motivation]: #motivation

The motivation here is to provide common types for both `no_std` and `std`
targets which in turn will ease the creation of libraries based around IP
addresses. Embedded IoT development is one area where this will be beneficial.
IP addresses are portable across all platforms and have no external
dependencies which is in line with the definition of the core library.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The `core::net::IpAddr`, `core::net::Ipv4Addr` and `core::net::Ipv6Addr` types
are available in `no_std` contexts.

Library developers should use `core::net` to implement abstractions in order
for them to work in `no_std` contexts as well.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Currently, the IP address types depend on their corresponding `libc`
counterpart for their `inner` value.

IPv4 addresses are well-defined. [IETF RFC 791] specifically states:

> Addresses are fixed length of four octets (32 bits).

IPv6 addresses are well-defined. [IETF RFC 4291] specifically states:

> IPv6 addresses are 128-bit identifiers

Since the size and representation of IPv4 and IPv6 addresses are well defined,
we can replace the `inner` value of `Ipv4Addr` with a `[u8; 4]` and the `inner`
value of `IPv6Addr` with a `[u8; 16]`.

The inner types `[u8; 4]` and `[u8; 16]` are expected to correspond to `u32`
and `u128`  in big-endian byte order. Currently, this is already ensured:

- `u32::to_be` is used when constructing the corresponding `libc` type for
  `Ipv4Addr`.
- The corresponding `libc` type for `IPv6Addr` is already represented as a
  `[u8; 16]` internally on all platforms.

[IETF RFC 791]: https://tools.ietf.org/html/rfc791
[IETF RFC 4291]: https://tools.ietf.org/html/rfc4291

# Drawbacks
[drawbacks]: #drawbacks

Moving the `std::net` types to `core::net` makes the core library less *minimal*.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- Given the size of IP addresses is well defined by IETF RFCs, there is no
  inherent need to have these types depend on `libc`.

- Eliminates the need to use different abstractions for `no_std` and `std`.

- Alternatively, move these types into a library other than `core`, so they
  can be used without `std`.

# Prior art
[prior-art]: #prior-art

There was a prior discussion at

https://internals.rust-lang.org/t/std-ipv4addr-in-core/11247/15

and an experimental branch from [@Nemo157](https://github.com/Nemo157) at

https://github.com/Nemo157/rust/tree/core-ip

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None.

# Future possibilities
[future-possibilities]: #future-possibilities

`SocketAddr`, `SocketAddrV4` and `SocketAddrV6` could also be moved in the
future.
