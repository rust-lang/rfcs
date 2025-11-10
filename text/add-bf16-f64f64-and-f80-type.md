- Feature Name: `add-bf16-f64f64-and-f80-type`
- Start Date: 2023-7-10
- RFC PR: [rust-lang/rfcs#3456](https://github.com/rust-lang/rfcs/pull/3456)
- Rust Issue: [rust-lang/rust#2629](https://github.com/rust-lang/rfcs/issues/2629)

# Summary
[summary]: #summary

This RFC proposes new floating point types to enhance FFI with specific targets:

- `bf16` as builtin type for the 'brain floating point' format, widely used in machine learning, different from the IEEE 754 standard `binary16` representation
- `f64f64` in `core::arch` for the legacy extended float format used in the PowerPC architecture
- `f80` in `core::arch` for the extended float format used in the x86 and x86_64 architectures

Also, this proposal introduces `c_longdouble` in `core::ffi` to represent the correct format for 'long double' in C.

# Motivation
[motivation]: #motivation

The types listed above may be widely used in existing native code, but are not available on all targets. Their underlying representations are quite different from 16-bit and 128-bit binary floating format defined in IEEE 754.

In respective targets (namely PowerPC and x86), the target-specific extended types are referenced by `long double`, which makes `long double` ambiguous in the context of FFI. Thus defining `c_longdouble` should help interoperating with C code using the `long double` type.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

`bf16` is available on all targets. The operators and constants defined for `f32` are also available for `bf16`.

For `f64f64` and `f80`, their availability is limited to the following targets, but this may change over time:

- `f64f64` is supported on `powerpc-*` and `powerpc64(le)-*`, available in `core::arch::{powerpc, powerpc64}`
- `f80` is supported on `i[356]86-*` and `x86_64-*`, available in `core::arch::{x86, x86_64}`

The operators and constants defined for `f32` and `f64` are available for `f64f64` and `f80` in their respective arch-specific modules.

All proposed types do not have literal representation. Instead, they can be converted to or from IEEE 754 compliant types.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## `bf16` type

`bf16` consists of 1 sign bit, 8 bits of exponent, 7 bits of mantissa. Some ARM, AArch64, x86 and x86_64 targets support `bf16` operations natively. For other targets, they will be promoted into `f32` before computation and truncated back into `bf16`.

`bf16` will generate the `bfloat` type in LLVM IR.

## `f64f64` type

`f64f64` is the legacy extended floating point format used on PowerPC targets. It consists of two `f64`s, with the former acting as a normal `f64` and the latter for an extended mantissa.

The following `From` traits are implemented in `core::arch::{powerpc, powerpc64}` for conversion between `f64f64` and other floating point types:

```rust
impl From<bf16> for f64f64 { /* ... */ }
impl From<f32> for f64f64 { /* ... */ }
impl From<f64> for f64f64 { /* ... */ }
```

`f64f64` will generate `ppc_fp128` type in LLVM IR.

## `f80` type

`f80` represents the extended precision floating point type on x86 targets, with 1 sign bit, 15 bits of exponent and 63 bits of mantissa.

The following `From` traits are implemented in `core::arch::{x86, x86_64}` for conversion between `f80` and other floating point types:

```rust
impl From<bf16> for f80 { /* ... */ }
impl From<f32> for f80 { /* ... */ }
impl From<f64> for f80 { /* ... */ }
```

`f80` will generate the `x86_fp80` type in LLVM IR.

## `c_longdouble` type in FFI

`core::ffi::c_longdouble` will always represent whatever `long double` does in C. Rust will defer to the compiler backend (LLVM) for what exactly this represents, but it will approximately be:

- `f80` extended precision on `x86` and `x86_64`
- `f64` double precision with MSVC
- `f128` quadruple precision on AArch64
- `f64f64` on PowerPC

# Drawbacks
[drawbacks]: #drawbacks

`bf16` is not an IEEE 754 standard type, so adding it as primitive type may break existing consistency for builtin float types. The truncation after calculations on targets not supporting `bf16` natively also breaks how Rust treats precision loss in other cases.

`c_longdouble` are not uniquely determined by architecture, OS and ABI. On the same target, C compiler options may change what representation `long double` uses.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The [half](https://github.com/starkat99/half-rs) crate provides an implementation of the binary16 and bfloat16 formats.

However, besides the disadvantage of usage inconsistency between primitive types and types from crates, there are still issues around those bindings.

The availablity of additional float types heavily depends on CPU/OS/ABI/features of different targets. Evolution of LLVM may also unlock the possibility of the types on new targets. Implementing them in the compiler handles the stuff at the best location.

Most of such crates define their type on top of C bindings. However the extended float type definition in C is complex and confusing. The meaning of `long double` and `_Float128` varies by targets or compiler options. Implementing them in the Rust compiler helps to maintain a stable codegen interface.

And since third party tools also rely on Rust internal code, implementing additional float types in the compiler also helps the tools to recognize them.

# Prior art
[prior-art]: #prior-art

There is a previous proposal on `f16b` type to represent `bfloat16`: https://github.com/rust-lang/rfcs/pull/2690.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

This proposal does not contain information for FFI with C's `_Float128` and `__float128` type. [RFC #3453](https://github.com/rust-lang/rfcs/pull/3453) focuses on type conforming to IEEE 754 `binary128`.

Although statements like `X target supports A type` is used in above text, some target may only support some type when some target features are enabled. Such features are assumed to be enabled, with precedents like `core::arch::x86_64::__m256d` (which is part of AVX).

Representation of `long double` in C may depend on some compiler options. For example, Clang on `powerpc64le-*`, `-mabi=ieeelongdouble`/`-mabi=ibmlongdouble`/`-mlong-double-64` will set `long double` as `fp128`/`ppc_fp128`/`double` in LLVM. Currently, the default option is assumed.

# Future possibilities
[future-possibilities]: #future-possibilities

[LLVM reference for floating types]: https://llvm.org/docs/LangRef.html#floating-point-types
