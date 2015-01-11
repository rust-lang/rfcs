- Start Date: 2015-01-10
- RFC PR: 
- Rust Issue: 

# Summary

Bring back `bytes!`: a macro to produce a static expression of type
`&'static [u8]` by concatenating the parameters, which may be string or
byte string literals.

# Motivation

There is currently no way to concatenate byte string literals in macros the
way `concat!` works for strings:
```rust
macro_rules! c_str {
    ($lit:expr) => { concat!($lit, "\0") }
}
```

The macro `bytes!` used to be the way to concatenate static byte slices
out of multiple parameters. That macro was never updated to support byte
string literals and was eventually removed. There is still need for it,
for example, in code that works with C libraries accepting null-terminated
strings. It is tedious and error-prone to have to terminate string literals
with "\0", or otherwise a dynamic conversion is needed for usual Rust
literals not ending with a NUL character, carrying unnecessary performance
overhead.

# Detailed design

The intrinsically implemented macro `bytes!` takes any number of
comma-separated parameters which may be `&'static str` or `&'static [u8]`
values computable at compile time (i.e. string literals, byte string literals,
and byte array initializer expressions). The result is a byte slice value of
type `&'static [u8]`, concatenating the byte string representations of the
parameters.

There is no automatic stringification for numeric literals in the manner of
`concat!`, due to potentially
ambiguous meaning:
```rust
let line = bytes!("line with an ending", 0x0d, 0x0a);  // bytes or numbers?
```

As an aside, this feature of `concat!` can be considered harmful, as there is
the explicit `stringify!` macro.

# Drawbacks

None known.

# Alternatives

None. There is no way to construct byte string literals in macro substitution
without this intrinsic macro.

# Unresolved questions

The macro name could be more self-explanatory `concat_bytes!`.
