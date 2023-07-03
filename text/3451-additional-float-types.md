- Feature Name: `additional-float-types`
- Start Date: 2023-6-28
- RFC PR: [rust-lang/rfcs#3451](https://github.com/rust-lang/rfcs/pull/3451)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC proposes new floating point types `f16` and `f128` into core language and standard library. Also, this RFC introduces `f80`, `f64f64`, and `bf16` into `core::arch` for target-specific support, and `core::ffi::c_longdouble` for FFI interop.

# Motivation
[motivation]: #motivation

[IEEE-754] standard defines binary floating point formats, including `binary16`, `binary32`, `binary64` and `binary128`. `binary32` and `binary64` correspond to `f32` and `f64` types in Rust, but there is currently no representation for `binary16` or `binary128`; these have uses in multiple scenarios (machine learning, scientific computing, etc.) and accepted by some modern architectures (by software or hardware), so this RFC proposes to add representations for them to the language.

In C/C++ world, there are already types representing these formats, along with more legacy non-standard types specific to some platform. Introduce them in a limited way would help improve FFI against such code.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

`f16` and `f128` are primitive floating types, they can be used just like `f32` or `f64`. They always conform to the `binary16` and `binary128` formats defined in [IEEE-754], which means size of `f16` is always 16-bit, and size of `f128` is always 128-bit.

```rust
let val1 = 1.0; // Default type is still f64
let val2: f128 = 1.0;
let val3: f16 = 1.0;
let val4 = 1.0f128; // Suffix of f128 literal
let val5 = 1.0f16; // Suffix of f16 literal

println!("Size of f128 in bytes: {}", std::mem::size_of_val(&val2)); // 16
println!("Size of f16 in bytes: {}", std::mem::size_of_val(&val3)); // 2
```

`f16` and `f128` will only be available on hardware that supports or natively emulates these type via LLVM's `half` and `fp128`, as mentioned in the [LLVM reference for floating types]. This means that the semantics of `f16` and `f128` are fixed as IEEE compliant in every supported platform, different from `long double` in C.

Because not every target supports `f16` and `f128`, compiler provides conditional guards.

```rust
#[cfg(target_has_f128)]
fn get_f128() -> f128 { 1.0f128 }

#[cfg(target_has_f16)]
fn get_f16() -> f16 { 1.0f16 }
```

All operators, constants and math functions defined for `f32` and `f64` in core, are also defined for `f16` and `f128`, and guarded by respective conditional guards.

- The `f80` type is defined in `core::arch::{x86, x86_64}` as 80-bit extended precision.
- The `f64f64` type is defined in `core::arch::{powerpc, powerpc64}` and represent's PowerPC's non-IEEE double-double format (two `f64`s used to aproximate `f128`).
- `bf16` type is defined in `core::arch::{arm, aarch64, x86, x86_64}` and  represents the "brain" float, a truncated `f32` with SIMD support on some hardware. These types do not have literal representation.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## `f16` type

`f16` consists of 1 bit of sign, 5 bits of exponent, 10 bits of mantissa. It is always in accordance with [IEEE-754].

The following `From` and `TryFrom` traits are implemented for conversion between `f16` and other types:

```rust
impl From<f16> for f32 { /* ... */ }
impl From<f16> for f64 { /* ... */ }
impl From<bool> for f16 { /* ... */ }
impl From<u8> for f16 { /* ... */ }
impl From<i8> for f16 { /* ... */ }
```

`f16` will generate `half` type in LLVM IR.

## `f128` type

`f128` consists of 1 bit of sign, 15 bits of exponent, 112 bits of mantissa.

`f128` is available for on targets having (1) hardware instructions or software emulation for 128-bit float type; (2) backend support for `f128` type on the target; (3) essential target features enabled (if any).

The list of targets supporting `f128` type may change over time. Initially, it includes `powerpc64le-*`, `x86_64-*` and `aarch64-*`

The following traits are also implemented for conversion between `f128` and other types:

```rust
impl From<f16> for f128 { /* ... */ }
impl From<f32> for f128 { /* ... */ }
impl From<f64> for f128 { /* ... */ }
impl From<bool> for f128 { /* ... */ }
impl From<u8> for f128 { /* ... */ }
impl From<i8> for f128 { /* ... */ }
impl From<u16> for f128 { /* ... */ }
impl From<i16> for f128 { /* ... */ }
impl From<u32> for f128 { /* ... */ }
impl From<i32> for f128 { /* ... */ }
impl From<u64> for f128 { /* ... */ }
impl From<i64> for f128 { /* ... */ }
```

`f128` will generate `fp128` type in LLVM IR.

For `f64f64` type, conversion intrinsics are available under `core::arch::{powerpc, powerpc64}`. For `f80` type, conversion intrinsics are available under `core::arch::{x86, x86_64}`.

## Architectures specific types

- `core::arch::{x86, x86_64}::f80` generates LLVM's `x86_fp80`, 80-bit extended precision
- `core::arch::{powerpc, powerpc64}::f64f64` generates LLVM's `ppc_fp128`, a `f128` emulated type via dual `f64`s
- `core::arch::{arm, aarch64, x86, x86_64}::bf16` generates LLVM's `bfloat`, 16-bit "brain" floats used in AVX and ARMv8.6-A

Where possible, `From` will be implemented to convert `f80` and `f64f64` to `f128`.

## FFI types

`core::ffi::c_longdouble` will always represent whatever `long double` does in C. Rust will defer to the compiler backend (LLVM) for what exactly this represents, but it will approximately be:

- 80-bit extended precision (f80) on `x86` and `x86_64`:
- `f64` double precision with MSVC
- `f128` quadruple precision on AArch64
- `f64f64` on PowerPC

# Drawbacks
[drawbacks]: #drawbacks

Unlike f32 and f64, although there are platform independent implementation of supplementary intrinsics on these types, not every target support the two types natively, with regards to the ABI. Adding them will be a challenge for handling different cases.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

There are some crates aiming for similar functionality:

- [f128](https://github.com/jkarns275/f128) provides binding to `__float128` type in GCC.
- [half](https://github.com/starkat99/half-rs) provides implementation of binary16 and bfloat16 types.

However, besides the disadvantage of usage inconsistency between primitive type and type from crate, there are still issues around those bindings.

The availablity of additional float types depends on CPU/OS/ABI/features of different targets heavily. Evolution of LLVM may also unlock possibility of the types on new targets. Implementing them in compiler handles the stuff at the best location.

Most of such crates defines their type on top of C binding. But extended float type definition in C is complex and confusing. The meaning of `long double`, `_Float128` varies by targets or compiler options. Implementing in Rust compiler helps to maintain a stable codegen interface.

And since third party tools also relies on Rust internal code, implementing additional float types in compiler also help the tools to recognize them.

# Prior art
[prior-art]: #prior-art

We have a previous proposal on `f16b` type to represent `bfloat16`: https://github.com/joshtriplett/rfcs/blob/f16b/text/0000-f16b.md

# Unresolved questions
[unresolved-questions]: #unresolved-questions

This proposal does not introduce `c_longdouble` type for FFI, because it means one of `f128`, `f64f64`, `f64` or `f80` on different cases. Also for `c_float128`.

# Future possibilities
[future-possibilities]: #future-possibilities

More functions will be added to those platform dependent float types, like casting between `f128` and `f64f64`.

For targets not supporting `f16` or `f128`, we may be able to introduce a 'limited mode', where the types are not fully functional, but user can load, store and call functions with such arguments.

[LLVM reference for floating types]: https://llvm.org/docs/LangRef.html#floating-point-types
[IEEE-754]: https://en.wikipedia.org/wiki/IEEE_754
