- Start Date: 2014-05-03
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Rename `i8`, `u16`, `f32`, etc. to `int8`, `uint16`, `float32`, etc., respectively.

# Motivation

The goal of this change is to make Rust types adhere more closely to established standards (especially those of C99), and to increase consistency and readability.

We have a type `int`, but an 8-bit integer is not `int8` but rather `i8`. The abbrevation of "integer" is not consistent between these two names.

Type names like `u8` are commonly used in the Linux kernel, but are rare elsewhere. The use of `u8` in the Linux kernel predates the introduction of `stdint.h`, and it is not clear that Linux would have chosen this name if `stdint.h` had existed at the time.

`uint64` has many more hits on Google than `u64`, as Go, C#, js-ctypes, Matlab, Vala, Pascal, and JNI. just to name a few off the first couple of search results, all use that name.

Even more oddly, we actually have `uint8_t` and friends inside the `libc` crate, and code that uses the FFI is usually a mishmash of the `libc` types (e.g. `uint8_t`) and the built-in types (e.g. `u8`). The distinction is jarring.

# Drawbacks

The disadvantages are that `uint8` is longer (although this can also be seen as an advantage, depending on your point of view), and that `u8` is consistent with the literal form (`1u8`).

# Detailed design

We change:

* `i8` to `int8`;
* `i16` to `int16`;
* `i32` to `int32`;
* `i64` to `int64`;
* `u8` to `uint8`;
* `u16` to `uint16`;
* `u32` to `uint32`;
* `u64` to `uint64`;
* `f32` to `float32`;
* `f64` to `float64`;
* `f128` to `float128`.

# Alternatives

The impact of not doing this would be that Rust would continue to look the way it is, with the drawbacks and advantages noted above.

# Unresolved questions

None.