- Feature Name: `core_net_types`
- Start Date: 2019-12-06
- RFC PR: [rust-lang/rfcs#2832](https://github.com/rust-lang/rfcs/pull/2832)
- Rust Issue: [rust-lang/rust#108443](https://github.com/rust-lang/rust/issues/108443)

# Summary
[summary]: #summary

Make the `IpAddr`, `Ipv4Addr`, `Ipv6Addr`, `SocketAddr`, `SocketAddrV4`,
`SocketAddrV6`, `Ipv6MulticastScope` and `AddrParseError` types available in `no_std`
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

The `core::net::IpAddr`, `core::net::Ipv4Addr`, `core::net::Ipv6Addr`,
`core::net::SocketAddr`, `core::net::SocketAddrV4`, `core::net::SocketAddrV6`,
`core::net::Ipv6MulticastScope` and `core::net::AddrParseError` types are
available in `no_std` contexts.

Library developers should use `core::net` to implement abstractions in order
for them to work in `no_std` contexts as well.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Since https://github.com/rust-lang/rust/pull/78802 has been merged, IP and
socket address types are implemented in ideal Rust layout instead of wrapping
their corresponding `libc` representation.

Formatting for these types has also been adjusted in
https://github.com/rust-lang/rust/pull/100625 and
https://github.com/rust-lang/rust/pull/100640 in order to remove the dependency
on `std::io::Write`.

This means the types are now platform-agnostic, allowing them to be moved from
`std::net` into `core::net`.

# Drawbacks
[drawbacks]: #drawbacks

Moving the `std::net` types to `core::net` makes the core library less *minimal*.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- Eliminates the need to use different abstractions for `no_std` and `std`.

- Alternatively, move these types into a library other than `core`, so they
  can be used without `std`, and re-export them in `std`.

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

Move the `ToSocketAddrs` trait to `core::net` as well. This depends on having `core::io::Result`.
