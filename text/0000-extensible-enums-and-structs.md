- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Introduce a notation for declaring an enum to be *extensible*, meaning
that more variants or fields may be added in the future. This notation
does not affect the use of the enum within the current crate, but it
prevents downstream crates from employing features (such as exhaustive
matches or struct literals) that would fail to compile if new
variants/fields were added.

# Motivation

Imagine that one is writing a public-facing API that includes an
`Error` enum (this exact scenario arises in libstd with the I/O error
enum):

```rust
pub enum Error {
    NoSuchFile,
    NoPermissions,
}
```

Providing an enum listing out the possible sources of error is a very
convenient API for your users, because they can be use a `match`
statement to easily identify and handle particular kinds of
errors. Unfortunately, it's rather limiting on the future evolution of
your library: if you wish to add a third kind of error in the future,
you will be potentially breaking downstream code. After all, it is
possible that some user was doing an exhaustive match against all
possible sources of error:

```rust
match error {
    Error::NoSuchFile => ...,
    Error::NoPermissions => ...,
}
```

To resolve this dilemna, this RFC proposes permitting enums to be declared
as *extensible*:

```rust
pub enum Error {
    NoSuchFile,
    NoPermissions,
    ..
}
```

Extensible enums from other crates can never be exhaustively
matched. This means that a downstream user attempting to match against
an error from your library would have to include a wildcard option:

```rust
// from an external crate
match error {
    Error::NoSuchFile => ...,
    Error::NoPermissions => ...,
    _ => ...,
}
```

Due to the presence of this wildcard, you can safely add variants
without fear of breaking source compatibility with downstream clients.

Note that extensibility is ignored within the current
crate. Therefore, your library may internally write a match that
exhaustively covers all sources of error. You don't need to worry
about source compatibility with yourself, after all.

# Detailed design

Enum grammar is extended to permit a list of variants to be terminated
with `..`. An enum declared with `..` is considered *extensible*.

Extensible enums that are local to the current crate are treated the
same as any other enum. Exhaustiveness checking is modified so that
extensible enums that are not local to the current crate are never
considered completely covered. (As if there was one additional variant
beyond those declared, essentially.)

Extensibility does not currently affect the enum in any other way. The
representation in particular remains unchanged. This RFC does not
attempt to address binary-level compatibility, only source
compatibility.

# Drawbacks

More language syntax.

# Alternatives

**Private enum variants.** There are various privacy-based workarounds
one could use to get a similar guarantee. For example, if private
variants were permitted (or perhaps variants were private by default),
then one could add a private variant to the enum. Privacy however is not a perfect fit:

1. Private variants would also be private within your crate,
   preventing you from using local exhaustive matching as well.
2. Even if you work around the previous problem and manage to make
   enum variants visible only within the current crate, all of your
   match statements will include an extra, unreachable arm
   corresponding to this private variant (since it never occurs in
   practice).
   
**Extensible structs.** In the same way that enums can be extended
with more variants, it might be useful to be able to declare structs
as being extensible with additional fields. This would prevent the
various bits of syntax (struct literal matching, struct literal
constructors) that require knowledge of the full set of fields to
work. Originally this feature was intended for inclusion in this RFC
but was omitted to keep things simple. The need to add more fields in
the future is both rather unusual and easily addressed using a dummy
private field (private fields do have some of the same downsides as
private enum variants, though). In practice though most libraries will
just declare all fields as private and use getters to access their
contents, which is a better pattern in any case.
   
# Unresolved questions

What parts of the design are still TBD?
