- Start Date: 2014-04-04
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

This is an RFC to provide better support for bit fields, including simpler notation and matching.

# Motivation

Bit fields are commonly used in embedded and system software where rust should be an better option compared to C/C++. Working with bit fields is a hard process full of magic numbers and prone to errors.

# Detailed design

The first part of this RFC is defenition of a bit access for integer types. For the sake of simplicity, only fixed width unsigned integer types (u8, u16, u32, u64) are supported.

Bit access operation is defined as

```rust
let mut val: u32 = ...;
let bits1 = val[4..5];  // equivalent to bits = (val >> 4) & 3
let bits2 = val[0,4..5]; // equivalent to bits = ((val >> 4) & 3) | (val & 1)

val[2..7] = 10; // equivalent to val = (val & (0xffffffff ^ 0xfc)) | (10 << 2)
val[0] = 3; // doesn't compile, as you can't fit 0b11 into one bit place
```

The second part of this RFC is matching on bits. It is often required to perform different actions based on few bits of an integer. Currently rust requires `_ => ...` in the end for such cases, as one cannot cover all the integer options (while it's possible to cover all the possible bit options). The proposed solution matched with above is:

```rust
match val[4..5] {
  0b00 => ...,
  0b01 => ...,
  0b10 => ...,
  0b11 => ...
} // no match all provided, all variants must be included.
```

# Alternatives

Provide a bit extraction macros that would perform the first part of this RFC. Doesn't solve the problem of second part.

Erlang has an even better bit matching:

```erlang
-define(IP_VERSION, 4).
-define(IP_MIN_HDR_LEN, 5).

DgramSize = byte_size(Dgram),
case Dgram of 
    <<?IP_VERSION:4, HLen:4, SrvcType:8, TotLen:16, 
      ID:16, Flgs:3, FragOff:13,
      TTL:8, Proto:8, HdrChkSum:16,
      SrcIP:32,
      DestIP:32, RestDgram/binary>> when HLen>=5, 4*HLen=<DgramSize ->
        OptsLen = 4*(HLen - ?IP_MIN_HDR_LEN),
        <<Opts:OptsLen/binary,Data/binary>> = RestDgram,
    ...
end.
```

# Unresolved questions

TBD
