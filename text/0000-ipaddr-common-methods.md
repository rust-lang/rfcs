- Feature Name: ipaddr-common-methods
- Start Date: 2016-07-06
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

As of Rust v1.7.0, net::Ipv4Addr and net::Ipv6Addr both expose `is_loopback()` and `is_multicast()`.
net::IpAddr should expose those directly rather than requiring a redundant `match`.
This should carry forward as [more methods are stabilized](https://github.com/rust-lang/rust/issues/27709).

# Motivation
[motivation]: #motivation

If I have an IpAddr and I want to test whether or not it is a loopback address, I do not need or want to know
if it is v4 or v6. As of Rust v1.7.0, testing if an IpAddr `addr` is a loopback address looks like this:
``` rust
match addr {
    IpAddr::V4(v4addr) => v4addr.is_loopback(),
    IpAddr::V6(v6addr) => v6addr.is_loopback(),
}
```
If this RFC is adopted, this would become simply
``` rust
addr.is_loopback()
```
which is much simpler.

Additionally, net::SocketAddr.ip() and .port() do the same thing elsewhere in the standard library.

# Detailed design
[design]: #detailed-design

Since `is_loopback()` and `is_multicast()` are stable as of Rust v1.7.0 in both
[net::Ipv4Addr](https://doc.rust-lang.org/std/net/struct.Ipv4Addr.html) and
[net::Ipv6Addr](https://doc.rust-lang.org/std/net/struct.Ipv6Addr.html),
it seems natural to add those methods, and any others that are shared and stable, to
[net::IpAddr](https://doc.rust-lang.org/std/net/enum.IpAddr.html).

These implementations would be written as `match`es like the one given above, similarly to
[the implementation of net::SocketAddr.ip()](https://github.com/rust-lang/rust/blob/master/src/libstd/net/addr.rs#L63-68),
which is a method that seems to exist for the same reason.

# Drawbacks
[drawbacks]: #drawbacks

This makes the standard library slightly larger and increases overhead in changes to Ipv4Addr and Ipv6Addr
(since newly stable methods in both structs would be added to IpAddr as well).

# Alternatives
[alternatives]: #alternatives

This RFC does not make anything new possible. It simply makes something slightly easier.
As such, to simply leave things as they are would be the primary alternative.

# Unresolved questions
[unresolved]: #unresolved-questions

Was this a deliberate omission or just something nobody asked for yet?
