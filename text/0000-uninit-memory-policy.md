- Feature Name: (fill me in with a unique ident, my_awesome_feature)
- Start Date: 2015-02-13
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Set an explicit policy that uninitialized memory can ever be exposed
in safe Rust, even when it would not lead to undefined behavior.

# Motivation

Exactly what is guaranteed by safe Rust code is not entirely
clear. There are some clear baseline guarantees: data-race freedom,
memory safety, type safety. But what about cases like reading from an
uninitialized, but allocated slice of scalars? These cases can be made
memory and typesafe, but they carry security risks.

In particular, it may be possible to exploit a bug in safe Rust code
that leads that code to reveal the contents of memory.

Consider the `std::io::Read` trait:

```rust
pub trait Read {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<()> { ... }
}
```

The `read_to_end` convenience function will extend the given vector's capacity,
then pass the resulting (allocated but uninitialized) memory to the
underlying `read` method.

While the `read` method may be implemented in pure safe code, it is
nonetheless given read access to uninitialized memory.  The
implementation of `read_to_end` guarantees that no UB will arise as a
result. But nevertheless, an incorrect implementation of `read` -- for
example, one that returned an incorrect number of bytes read -- could
result in that memory being exposed (and then potentially sent over
the wire).

# Detailed design

While we do not have a formal spec/contract for unsafe code, this RFC
will serve to set an explicit policy that:

**Uninitialized memory can ever be exposed in safe Rust, even when it
would not lead to undefined behavior**.

# Drawbacks

In some cases, this policy may incur a performance overhead due to
having to initialize memory that will just be overwritten
later. However, these situations would be better served by improved
implementation techniques and/or introducing something like a `&out`
pointer expressing this idiom.

In addition, in most cases `unsafe` variants of APIs can always be
provided for maximal performance.

# Alternatives

The main alternative is to limit safety in Rust to e.g. having defined
behavior (which generally entails memory and type safety and data-race
freedom). While this is a good baseline, it seems worthwhile to aspire
to greater guarantees where they come at relatively low cost.

# Unresolved questions

Are there APIs in `std` besides the convenience functions in IO that
this policy would affect?
