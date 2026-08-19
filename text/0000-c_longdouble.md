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

impl Debug for f80x87 { /* ... */ } // using feature(float_format_hex)

// Etc.
From<f64> for f80x87 { /* ... */ }

impl f80x87 {
    // etc.
    const BITS: u32 = 80;

    // And BE and NE.
    fn from_le_bytes(bytes: [u8; 10]) -> Self { /* ... */ }
    fn to_le_bytes(self) -> [u8; 10] { /* ... */ }

    // Conversion methods from feature(float_conversions)
    fn cast<Flt>(self) -> Flt where Self: FloatToFloat<Flt>;
    fn to_int_saturating<Int>(self) -> Int where Self: FloatToInt<Int>;
    fn to_int_checked<Int>(self) -> Option<Int> where Self: FloatToInt<Int>;
    fn to_int_strict<Int>(self) -> Int where Self: FloatToInt<Int>;
}
```

- on x86 the alignment is 4 bytes and the size is 12 bytes
- on x86_64 the alignment is 8 bytes and the size is 16 bytes.

This RFC deliberately does not propose support for arithmetic.

### Conversions

See https://godbolt.org/z/9o1Mf8x1P.

Conversions to and from `f32` and `f64` have hardware support, for `f16` and `f128` libcalls are needed.

This functionality can be exposed with the functions from [`feature(float_conversions)`](https://github.com/rust-lang/rust/issues/159913).

## `f128ppc`

This type lives in `core::arch::{powerpc, powerpc64}::f128ppc`, and can only be used on those targets.

At least for now, its API is very minimal.

```rust
#[lang = "f128ppc"]
struct f128ppc {}

impl Clone for f128ppc { /* ... */ }
impl Copy for f128ppc {}

impl PartialEq for f80x87 { /* ... */ }
impl PartialOrd for f80x87 { /* ... */ }

impl Debug for f80x87 { /* ... */ } // using feature(float_format_hex)

// Etc.
From<f64> for f128ppc { /* ... */ }

impl f128ppc {
    // etc.
    const BITS: u32 = 128;

    // And BE and NE.
    fn from_le_bytes(bytes: [u8; 16]) -> Self { /* ... */ }
    fn to_le_bytes(self) -> [u8; 16] { /* ... */ }

    fn from_components(large: f64, small: f64) -> Self { /* ... */ }
    fn to_components(self) -> (f64, f64) { /* ... */ }

    // Conversion methods from feature(float_conversions)
    fn cast<Flt>(self) -> Flt where Self: FloatToFloat<Flt>;
    fn to_int_saturating<Int>(self) -> Int where Self: FloatToInt<Int>;
    fn to_int_checked<Int>(self) -> Option<Int> where Self: FloatToInt<Int>;
    fn to_int_strict<Int>(self) -> Int where Self: FloatToInt<Int>;
}
```

Supporting floating point operations on this type is not in scope.

There are some libcalls for this type, but they appear to pass float arguments separately and hence the signature is independent of target features:

https://godbolt.org/z/rEa551eEq

### Normalization

Operations on `f128ppc` assume that the value is normalized: the high component is assumed to be at least as large was the low component. Hence whether a value is positive or negative can only look at the sign bit of the high component.

When creating a `f128ppc` value from bytes (using `transmute`, `from_le_bytes`, etc) this invariant can be broken. That can cause nonsensical results, but cannot cause UB.

### Conversions

See https://godbolt.org/z/G5r7eW1M6.

Conversions from and to `f32` and `f64` have hardware support or are expanded (using the two `f64` components).

For `f16` it's more complicated:

- `fptrunc ppc_fp128 %x to half` fails in LLVM expansion
- `fpext half %x to ppc_fp128` calls `__extendhfsf2`

For `f128` there are:

- `declare ppc_fp128 @llvm.ppc.convert.f128.to.f128ppc(fp128)`
- `declare fp128 @llvm.ppc.convert.f128ppc.to.f128(ppc_fp128)`

This functionality can be exposed with the functions from [`feature(float_conversions)`](https://github.com/rust-lang/rust/issues/159913).

## Conversions from and to `String`

For conversion to `String` we can make use of the accepted, but currently unimplemented, [`feature(float_from_hex)`](https://github.com/rust-lang/rust/issues/160626). For conversion from string we can use [`feature(float_format_hex)`](https://github.com/rust-lang/rust/issues/160626), part of the same tracking issue. The `FromStr` and `ToString` implementations for `f64` don't easily extend to wider floats, and doing so would have a relatively large binary size penalty.

The `Debug` implementations can use the hex format for now. See also [notes] on future plans for `Display`.

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

We should nevertheless try to match the APIs of `f64` and `f128`, e.g. by defining constants (e.g. `BITS`, `MIN`, `INFINITY`) and methods that operate only on the bits (e.g. `is_nan`, `is_infinite`, `abs`, `copysign`).

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

## `c_longdouble` is niche

This type is uncommon. Most of its original use cases are better served by `f128` today.

## Distributions configure `long double`

See [unresolved-questions]. We may be forcing a split of powerpc targets with this type.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Rationale: why `c_longdouble` must be a builtin type

The reason `c_longdouble` can't be defined in user space is that `f80x87` and `f128ppc` have special ABI requirements. Therefore compiler support is required.

## Alternative: make `c_longdouble` an opaque type

Rather than an alias for existing types, `c_longdouble` could be a custom type that matches C `long double`. This type can have a more consistent interface across targets (e.g. addition or `.sqrt()` don't work at all, rather than only sometimes).

There is some similarity with `usize` being a distinc type rather than an alias for `u16`/`u32`/`u64`.

A downside of this approach is that it breaks the existing pattern of `core::ffi::c_*` types being aliases.

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

Some distributions configure the default in their system C compiler. See [unresolved-questions].

## LLVM

In LLVM these types are defined as `x86_fp80` and `ppc_fp128`. LLVM provides conversion and arithmetic operations like for any other primitive type.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

## Name bikeshed

Everyone's favorite thing to talk about, no technical knowledge required. And boy do we have some options here.

**X87 F80**

- `f80x87`, consistent with the recent `f16b`
- `f80`, rejected because it suggests this is a first-class type like f32 and f128, it is not
- `__float80`, similar to `__m256i` and similar platform-specific types in stdarch
- `x86f80` or `x86_f80` or `X86F80`
- `x87f80` or `x87_f80` or `X87F80`

**IBM F128**

- `f128ppc`, consistent with the recent `f16b`
- `__ibm128`, similar to `__m256i` and similar platform-specific types in stdarch
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

Clang does accept `__ibm128` when compiling for target triples where it is not `long double`, see https://godbolt.org/z/3jKch7db1.

## Distributions configuring `long double`

Some distributions (e.g. RHEL8 and Fedora 44 powerpc) configure a non-standard `long double` in their system C compiler. This is a problem when
C code compiled with those compilers is combined with Rust code compiled with a compiler downloaded via `rustup`: Rust will use the inferred
alias based on the target triple, which does not match the `long double` used by the system C compiler.

As a concrete example RHEL8 and Fedora 44 configure IEEE f128 where `f128ppc` is expected based on the target triple. Because `rustup` downloads a pre-built `core`, we can't detect the right type. Fundamentally, picking a different `long double` type is a different ABI.

We can solve this problem with special target tuples, e.g. by having the standard target use IEEE f128 and introducing a legacy target tuple for IBM f128 compatibility.

Picking IEEE as the default is only an option when the target baseline includes the `vsx` target feature, because Clang and GCC only support `_Float128` when `vsx` is enabled. For big-endian `powerpc{64}` the baseline only includes `altivec`, which has the required vector registers but not the C compiler support for IEEE f128. The little-endian `powerpc64le` baseline does enable `vsx` by default.

See [this thread](https://github.com/folkertdev/rust-rfcs/pull/3#discussion_r3807256613) for more context.

# Future possibilities
[future-possibilities]: #future-possibilities

- We could add more (arithmetic) operations to make Rust code using `c_longdouble` more portable.
- We could allow user control over x87 registers
- We could implement `Display`, see also [notes].

## `f80` on M68k

The rust `m68k` target is currently very tier 3. For `m68k-unknown-linux-gnu`, `std` does not even build (due to https://github.com/llvm/llvm-project/issues/217135). But `rustc_codegen_gcc` is experimenting with this target, so it may become relevant in the future.

Some m68k targets use a variant of `f80` that stores the same bits, but layed out in memory in a slightly different way.

# History

- RFC 3456 ["add `bf16`, `f64f64` and `f80 types"](https://github.com/rust-lang/rfcs/pull/3456)
- [#t-libs > &#96;f80&#96;, &#96;f128&#96; and &#96;c_longdouble&#96;](#narrow/channel/219381-t-libs/topic/.60f80.60.2C.20.60f128.60.20and.20.60c_longdouble.60)
- [#t-compiler > &#96;x87_f80&#96; is weird](#narrow/channel/131828-t-compiler/topic/.60x87_f80.60.20is.20weird)

# Notes
[notes]: #notes

## LLVM `ppc_fp128` bugs

This type is unfortunately plagued by several serious LLVM bugs, for which (partial) fixes at the time of writing have been submitted by the author, but these have not yet been merged:

- [fix `ppc_fp128` FABS miscompile](https://github.com/llvm/llvm-project/pull/208969) on little-endian 64-bit powerpc
- [fix bitcast on ppc_f128 swapping the two halves](https://github.com/llvm/llvm-project/pull/208969) on 32-bit powerpc

## Implementing `Display`

The current plan is to use [Zmij](https://github.com/vitaut/zmij) for the `Display` implementation of `f128` and the other float types. The C++ library recently gained support for `long double`, the [Rust port](https://github.com/dtolnay/zmij) hasn't added support yet.
