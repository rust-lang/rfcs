- Feature Name: hex-literals
- Start Date: 2017-12-12
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Addition of hex literals in the form of `h"00 aa cc ff"`, which will be
transformed by compiler at compile time to `&'static [u8; N]`, in this case to
`&'static [0u8, 170u8, 204u8, 255u8]`.

# Motivation
[motivation]: #motivation

Hexadecimal representation is a very common for binary data. Currently Rust has
two ways to provide byte array constants:
- `b"foo"` notation, which is convenient if binary data is an ASCII string,
but becomes harder to use for general byte string with a lot of `\x` escaping.
- Explicit arrays: `[0x00, 0x01, ..]`. It takes three times more
space compared to a pure hex notation and thus harder to read and copy-paste
from external sources. Additionally its harder to group bytes, e.g. by groups
of 4 or 8.

By introducing hex literals we can improve readability and writability of code which
works with binary constants. As a side effect we will be able to make code
examples smaller and easier to read. For example:

```Rust
let udp_data = h"
1111 2222
0c00 ffff
6461 7461
";
let packet = parse_udp(udp_data);
assert_eq!(packet.source_port, 0x1111);
assert_eq!(packet.dest_port, 0x2222);
assert_eq!(packet.data, b"data")
```

Also it will allow to copy-paste hexidicimal data directly into Rust code without
an additional transformation step.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Literals which start with `h` are called hex literals. They allow to
conviniently represent byte array constants in the hexadecimal form. String
inside `h"..."` accepts the following characters:

- Hexadecimal characters: `0-9, a-f, A-F`
- Formatting characters: unicode whitespace class characters, tab, carriage feed and return.

Formatting characters will be ignored by compiler. The following conditions
must be true for hex strings:

- Must contain even number of hexadecimal characters
- Must not contain only hexadecimal and formatting characters
- Formatting characters must not split octets, i.e. `h"a bc d"` is forbidden

Not complying with thise conditions will result in a compilation error.

Hexadecimal string will be converted to a byte array by compiler at compile time.

Usage examples:
```Rust
assert_eq!(h"00ff", &[0u8, 255u8]);
assert_eq!(h"abcdef", h"ABCDEF");
assert_eq!(h"64 61 74 61", b"data");

assert_eq!(h"
    00010203 0405060708
", &[0u8, 1, 2, 3, 4, 5, 6, 7, 8]);

assert_eq!(h"
    00010203
    10111213
", &[
    0x00, 0x01, 0x02, 0x03,
    0x10, 0x11, 0x12, 0x13
]);
```


# How We Teach This
[how-we-teach-this]: #how-we-teach-this
The book will need a page which will introduce and explain all variations of
string literals: `"..."`, `b"..."`, `r"..."`, `h"..."`. (and other future
extensions, e.g. like `s"..."` as a syntactic sugar for `"...".to_string()`)

# Drawbacks
[drawbacks]: #drawbacks

Additional syntax, which can be conceived by some as overly specialized for
niche use-cases.

# Rationale and Alternatives
[alternatives]: #alternatives

The proposed solution is arguably the simplest to use and read. Although
the following alternatives can be proposed:

- Instead of `h` use a base a modifier on the `b` prefix, e.g. `bx` for hex
binaries, `bo` for octal ones, `bb` for binary, or `bN` where N is the base
(between 2 and 36 included?)
- Built-in or procedural macro, e.g. `hex!("00 ff ee")`
- `const fn` (?)
- Do nothing and rely on existing tools.
