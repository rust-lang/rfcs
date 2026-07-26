- Feature Name: `c_longdouble`
- Start Date: 2026-07-28
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add `core::ffi::c_longdouble`, to match C `long double`, and its prerequisites.

```rust
use core::ffi::c_longdouble;

extern "C" fn f(x: c_longdouble) -> c_longdouble {
    x
}
```

This type is [highly platform-specific](https://github.com/llvm/llvm-project/blob/f6b3f9399e98ef8b79388176e94c910d67669e1f/llvm/lib/TargetParser/Triple.cpp#L2541-L2597),  A `long double` can be:

- `f32`, e.g. on AVR
- `f64`, e.g. on many 32-bit targets and some 64-bit target using MUSL
- `f128`, e.g. on aarch64 and riscv

So far so good, but on some platforms this type corresponds to a type that Rust cannot currently express:

- X87 f80 on `x86` and `x86_64`, an IEEE (ish) 80-bit floating point type
- IBM f128 on `powerpc` and `powerpc64`, a "double double"

This RFC proposes the addition of these two types, with extremely minimal APIs, so that a portable `core::ffi::c_longdouble` can be defined.

# Motivation
[motivation]: #motivation

A systems programming language should be able to express any signature that C can express. Rust already provides many ABI-compatible types in `core::ffi::*`, and the recent stabilization of c-variadic functions and the `VaList` type is another step in this direction. But, holes remain, and they should be plugged.

The goal of this RFC is interop: to be able to define the types and pass them across and ABI boundary. The goal is not a fully-fledged API on the Rust side for these types. If truly needed, inline assembly can be used to provide missing functionality.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Note: names are provisional, see [Unresolved questions](#unresolved-questions).

## `f80x87`

This type lives in `core::arch::{x86, x86_64}::f80x87`, and is hence only available on those target architectures.

At least for now, its API is very minimal.

```rust
#[lang = "f80x87"]
struct f80x87 {}

impl Clone for f80x87 { /* ... */ }
impl Copy for f80x87 {}

impl PartialEq for f80x87 { /* ... */ }
impl PartialOrd for f80x87 { /* ... */ }

// Etc.
From<f64> for f80x87 { /* ... */ }
TryFrom<f80x87> for f64 { /* ... */ }

impl f80x87 {
    // etc.
    const BITS: u32 = 80;

    // And BE and NE.
    fn from_le_bytes(bytes: [u8; 10]) -> Self { /* ... */ }
    fn to_le_bytes(self) -> [u8; 10] { /* ... */ }
}
```

- on x86 the alignment is 4 bytes and the size is 12 bytes
- on x86_64 the alignment is 8 bytes and the size is 16 bytes.

This type deliberately does not support any arithmetic, because the precision on floating point operations for this type is not guaranteed.

### Conversions

See https://godbolt.org/z/9o1Mf8x1P.

Conversions to and from `f32` and `f64` have hardware support, for `f16` and `f128` libcalls are needed.

## `f128ppc`

This type lives in `core::arch::{powerpc, powerpc64}::f128ppc`, and can only be used on those targets.

At least for now, its API is very minimal.

```rust
#[lang = "f128ppc"]
struct f128ppc {}

impl Clone for f128ppc { /* ... */ }
impl Copy for f128ppc {}

// And PartialEq, PartialOrd

// Etc.
From<f64> for f128ppc { /* ... */ }
TryFrom<f128> for f128ppc { /* ... */ }

impl f128ppc {
    // etc.
    const BITS: u32 = 128;

    // And BE and NE.
    fn from_le_bytes(bytes: [u8; 16]) -> Self { /* ... */ }
    fn to_le_bytes(self) -> [u8; 16] { /* ... */ }

    // Maybe.
    fn from_ne_components(components: [f64; 2]) -> Self { /* ... */ }
    fn to_ne_components(self) -> [f64; 2] { /* ... */ }
}
```

This type is unfortunately plagued by several serious LLVM bugs, for which (partial) fixes at the time of writing have been submitted by the author, but these have not yet been merged:

- [fix `ppc_fp128` FABS miscompile](https://github.com/llvm/llvm-project/pull/208969) on little-endian 64-bit powerpc
- [fix bitcast on ppc_f128 swapping the two halves](https://github.com/llvm/llvm-project/pull/208969) on 32-bit powerpc

Supporting floating point operations on this type is not in scope.

There are some libcalls for this type, but they appear to pass float arguments separately and hence the signature is independent of target features:

https://godbolt.org/z/rEa551eEq

### Conversions

See https://godbolt.org/z/G5r7eW1M6.

Conversions from and to `f32` and `f64` have hardware support or are expanded (using the two `f64` components).

For `f16` it's more complicated:

- `fptrunc ppc_fp128 %x to half` fails in LLVM expansion
- `fpext half %x to ppc_fp128` calls `__extendhfsf2`

For `f128` there are:

- `declare ppc_fp128 @llvm.ppc.convert.f128.to.f128ppc(fp128)`
- `declare fp128 @llvm.ppc.convert.f128ppc.to.f128(ppc_fp128)`

## Conversions from and to `String`

For conversion to `String` we can make use of the accepted, but currently unimplemented, [`feature(float_from_hex)`](https://github.com/rust-lang/rust/issues/160626). For conversion from string we can use [`feature(float_format_hex)`](https://github.com/rust-lang/rust/issues/160626), part of the same tracking issue. The `FromStr` and `ToString` implementations for `f64` don't easily extend to wider floats, and doing so would have a relatively large binary size penalty.

Down the line we plan to use [Zmij](https://github.com/vitaut/zmij) for the `Display` implementation of `f128` and the other float types. The C++ library recently gained support for `long double`, the [Rust port](https://github.com/dtolnay/zmij) hasn't added support yet.

## `core::ffi::c_longdouble`

With the above two types in place, we can define `c_longdouble`, which will look something like this:

```rust
type c_longdouble = cfg_select! {
    target_env = "musl" => f64,
    target_arch = "aarch64" => f128,
    target_arch = "riscv64gc" => f128,
    any(target_arch = "x86", target_arch = "x86_64") => f80x87,
    target_arch = "avr" => f32,
    // ...
};
```

LLVM now defines what `long double` resolves to for each target triple ([source](https://github.com/llvm/llvm-project/blob/f6b3f9399e98ef8b79388176e94c910d67669e1f/llvm/lib/TargetParser/Triple.cpp#L2541-L2597)).

An implication of `f80x87` and `f128ppc` not supporting any arithmetic is that code attempting e.g. to add two `c_longdouble` values is not portable. However, the same is often already true for other `core::ffi` types, e.g. `c_char` may be signed or unsigned, and `c_long` may or may not transmute to `[u8; 8]`. This limitation is unsatisfying, but the tradeoff seems better than not having `c_longdouble` at all.

We should nevertheless try to match the APIs of `f64` and `f128`, e.g. by defining constants (e.g. `BITS`, `MIN`, `INF`) and methods that operate only on the bits (e.g. `is_nan`, `is_inf`, `abs`, `copysign`).

## variadic arguments

It should be possible to read a `c_longdouble` from a [`VaList`](https://doc.rust-lang.org/std/ffi/struct.VaList.html), hence [`VaArgSafe`](https://doc.rust-lang.org/std/ffi/trait.VaArgSafe.html) should be implemented for the types that it aliases:

```rust
#[cfg(any(/* targets that define _Float128 */))]
unsafe impl VaArgSafe for f128 {}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
unsafe impl VaArgSafe for f80x87 {}
#[cfg(any(target_arch = "powerpc", target_arch = "powerpc64"))]
unsafe impl VaArgSafe for f128ppc {}

// Check that relevant `core::ffi` types implement `VaArgSafe`.
const _: () = {
    const fn va_arg_safe_check<T: VaArgSafe>() {}

    // ...

    va_arg_safe_check::<crate::ffi::c_longdouble>();
}
```

Empirically this is well-supported for the two new types, and for `f128` on C targets that define `_Float128`.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Compatibility

- the `f80x87` type is FFI-compatible with the C `long double`
- the `f128ppc` type is FFI-compatible with the C `long double`
- `Complex<c_longdouble>` is FFI-compatible with `_Complex long double`

## Syntax

This RFC does not propose any literals or other syntax for `f80x87` or `f128ppc`.

## Inline Assembly

Because we don't intend to support most operations on these types, inline assembly is an escape hatch if a rust program truly needs to perform some arithmetic operations on `f80x87`.  `f128ppc` has no custom instructions.

### `f80x87`

The `x87_reg` register class is currently clobber-only. These registers also operate as a stack, and modifying them can change the outcome of surrounding floating-point code
(see e.g. [rule `asm.rules.x86-x87`](https://doc.rust-lang.org/nightly/reference/inline-assembly.html#r-asm.rules.x86-x87)).

This RFC keeps `x87_reg` clobber-only, values can be loaded into these registers from memory.

### `f128ppc`

There is no dedicated register class for `f128ppc`. We provide methods like `to_ne_components` to extract the two `f64` components, which can then be passed to assembly by existing means.

# Drawbacks
[drawbacks]: #drawbacks

Why should we *not* do this?

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- Why is this design the best in the space of possible designs?
- What other designs have been considered and what is the rationale for not choosing them?
- What is the impact of not doing this?

# Prior art
[prior-art]: #prior-art

## C

GCC supports the `__float80` and `__ibm128` types, with literals using the `w` suffix.

```c
// These are all equivalent.
__ibm128 x = 1.0w; // on powerpc
__float80 x = 1.0w; // on x86
long double y = 1.0w;
long double z = 1.0L;
```

Clang does define the `__ibm128` type, but does not define `__float80` or accept the `w` suffix.

```c
__ibm128 x = 1.0L; // on powerpc
long double x = 1.0L;
```

In both compilers `long double` is a first-class type, with literals, formatting, arithmetic, etc.

### Configuration

Exactly what type `long double` corresponds to depends on the target, but can be influenced with compiler flags:

- `-mlong-double-{64, 80, 128}` sets the size of `long double`
- `-mabi={ibmlongdouble, ieeelongdouble}` sets the representation of `longdouble` on PowerPC

What flags are available is target-specific.

## LLVM

In LLVM these types are defined as `x86_fp80` and `ppc_fp128`. LLVM provides conversion and arithmetic operations like for any other primitive type.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

## Name bikeshed

Everyone's favorite thing to talk about, no technical knowledge required. And boy do we have some options here.

**X87 F80**

- `f80x87`, consistent with the recent `f16b`
- `f80` (rejected because it suggests this is a first-class type like f32 and f128, it is not)
- `x86f80` or `x86_f80`
- `x87f80` or `x87_f80`
- `f80x87`

**IBM F128**

- `f128ppc`, consistent with the recent `f16b`
- `ppcf128` or `ppc_f128`
- `ibmf128` or `ibm_f128`
- `doubledouble` or `DoubleDouble`
- `f64f64` or `F64F64`

## Availability

Should `f80x87` and `f128ppc` be available on targets where they do not correspond to `c_longdouble`? `c_longdouble` is not always `f80x87` or `f128ppc` on the `x86{_64}` and `powerpc{64}` target architectures, quoting LLVM source:

```c
  case ppc:
  case ppcle:
  case ppc64:
  case ppc64le:
    // PowerPC uses IBM double-double, except on a handful of OSes that use
    // plain IEEE double. NetBSD only switches to IEEE double on 32-bit PowerPC.
    if (isOSAIX() || isOSFreeBSD() || isOSOpenBSD() || isMusl() ||
        (isOSNetBSD() && isPPC32()))
      return LongDoubleFormat::IEEEdouble;
    return LongDoubleFormat::PPCDoubleDouble;
  case x86:
  case x86_64:
    // Android and OHOS use IEEE double on 32-bit and IEEE quad on 64-bit.
    if (isAndroid() || isOHOSFamily())
      return isX86_64() ? LongDoubleFormat::IEEEquad
                        : LongDoubleFormat::IEEEdouble;
    // Windows-MSVC and UEFI use IEEE double. MinGW and Cygwin keep x87.
    if (isWindowsMSVCEnvironment() || isUEFI())
      return LongDoubleFormat::IEEEdouble;
    return LongDoubleFormat::X87DoubleExtended;
```

# Future possibilities
[future-possibilities]: #future-possibilities

- We could add more (arithmetic) operations to make Rust code using `c_longdouble` more portable.
- We could allow user control over x87 registers

# History

- RFC 3456 ["add `bf16`, `f64f64` and `f80 types"](https://github.com/rust-lang/rfcs/pull/3456)
- [#t-libs > &#96;f80&#96;, &#96;f128&#96; and &#96;c_longdouble&#96;](#narrow/channel/219381-t-libs/topic/.60f80.60.2C.20.60f128.60.20and.20.60c_longdouble.60)
- [#t-compiler > &#96;x87_f80&#96; is weird](#narrow/channel/131828-t-compiler/topic/.60x87_f80.60.20is.20weird)
