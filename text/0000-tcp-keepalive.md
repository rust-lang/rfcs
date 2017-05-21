- Feature Name: tcp_keepalive
- Start Date: 2015-05-18
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Stabilize the `TcpSteram::set_keepalive()` method as
`fn set_keepalive(&self, dur: Duration) -> io::Result<()>`.

# Motivation

This is useful functionality for TCP, exposed in most programming languages.

# Detailed design

Adjust and stabilize the `set_keepalive` method to the following:

```rust

impl TcpStream {
    fn set_keepalive(&self, dur: Duration) -> io::Result<()> {
        ...
    }
}
```

This moves from the previous `seconds` argument to a `Duration` for these
reasons:

- It's more consistent with other timeout/duration APIs.
- While Linux only accepts seconds for this socket option, Windows (and possibly
  other OSes) [accept milliseconds][windows-milliseconds].

[windows-milliseconds]: https://msdn.microsoft.com/en-us/library/windows/desktop/dd877220(v=vs.85).aspx

That leaves handling the finer precision of the `Duration` value up to the
implementation per OS. This RFC suggests dropping precision that the OS cannot
use.

Passing a `Duration` of zero would be equivalent to turning `SO_KEEPALIVE` off,
since no OS docs mention allowing a value of `0`.

# Drawbacks

# Alternatives

An alternative to treating `Duraztion::zero()` as turning off keep-alive would 
be to accept `Option<Duration>`, with `None` turning it off. This has the
downside that `Some(Duration::zero())` would/should be an error.

# Unresolved questions

