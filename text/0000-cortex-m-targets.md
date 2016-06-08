- Feature Name: cortex_m_targets
- Start Date: 2016-06-07
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

It's been long known that [one can build Rust program for ARM Cortex-M microcontrollers][zinc].
This RFC proposes making these cross compilation targets more first-class by adding them to the
compiler and providing binary releases of standard crates like `core` for them.

[zinc]: https://github.com/hackndev/zinc

# Motivation
[motivation]: #motivation

Make building Rust programs for Cortex-M microcontrollers easier.

Currently, cross compiling for a Cortex-M microcontroller requires:

1. A target specification file, like `cortex-m3.json`, that defines a new cross compilation target.
2. Cross compiled standard crates: at least the `core` crate.
3. Cross compiled `libcompiler-rt.a` which provides the definitions of *intrinsics* that the LLVM
   backend may lower Rust code to.
4. A C cross linker, like `arm-none-eabi-gcc`, if building an executable.
5. A linker script that describes the memory layout (Flash, RAM, memory mapped peripherals, etc.) of
   the target device if building an executable.

This RFC aims to ease the cross compilation setup by providing a new `rust-core` "component" that
can be installed via `rustup` and packages most of the requirements listed above:

- [x] **Target specification file**. Writing this file requires not only knowing which processor one
  wants to compile for but also the many LLVM options that control code generation. Furthermore, the
  format of this file is unstable and has [broken][b] [twice][t] since 1.0. This RFC proposes moving
  these targets into the Rust compiler which makes these files unnecessary.
- [x] **Standard crates**. Cross compiling these crates is relatively easy now that the Rust
  repository has a Cargo based build system. However, the user still has to manually package the
  compiled crates in a "sysroot" and then manage a sysroot (e.g. recompile it when they update their
  nightly compiler) for each Cortex-M target they want to work with. Once the `rust-core` component
  is available, users will be able to use tools like `rustup` to easily manage several sysroots
  across the different Rust channels.
- [x] `compiler-rt`. Cross compiling `compiler-rt` is (slightly) annoying as its build system
  requires having `cmake` and `llvm-config` installed. The `rust-core` component will ship with a
  pre-compiled `libcompiler-rt.a` library.
- [ ] **Cross linker**. Automatic installation of toolchains or SDKs is out of scope for this RFC as
  the ideal solution must be general enough to support other targets like Android and iOS.
- [ ] **Linker script**. Linker scripts are device specific and there are hundreds of different ARM
  Cortex-M microcontrollers. This RFC doesn't attempt to provide a "one size fits all" linker
  script.

[b]: https://github.com/rust-lang/rust/issues/31367
[t]: https://github.com/rust-lang/rust/pull/32939

# Detailed design
[design]: #detailed-design

## New targets

Currently, there exist 6 Cortex-M processors and two of them have two and three variants
respectively due to the presence (or lack) of an optional [FPU][0] (Floating Point Unit):

[0]: https://en.wikipedia.org/wiki/Floating-point_unit

> **Note**: If you haven't heard of FPUs before, you can check the [appendix] for a quick overview.

> **Note**: SP stands for Single Precision. DP stands for Double Precision

| Processor  | FPU                       | Architecture |
|------------|---------------------------|--------------|
| Cortex-M0  | No                        | ARMv6-M      |
| Cortex-M0+ | No                        | ARMv6-M      |
| Cortex-M1  | No                        | ARMv6-M      |
| Cortex-M3  | No                        | ARMv7-M      |
| Cortex-M4  | Optional: SP              | ARMv7E-M     |
| Cortex-M7  | Optional: SP or SP and DP | ARMv7E-M     |

To support all these processors and their FPU(less) variants. The following targets will be added to
the compiler:

- `cortex-m0`
- `cortex-m0plus`
- `cortex-m1`
- `cortex-m3`
- `cortex-m4`. Cortex-M4 devices **without** FPU
- `cortex-m4f`. Cortex-M4 devices with FPU. Supports SP FPU instructions.
- `cortex-m7`. Cortex-M7 devices **without** FPU
- `cortex-m7f`. Cortex-M7 devices with FPU. Supports **both** SP and DP FPU instructions.
- `cortex-m7f-sp`. Cortex-M7 devices with FPU. Supports **only** SP FPU instructions.

## Target specifications

This section contains the target specifications for all the new targets. To avoid repetition, the
fields common to all the target specifications are shown below, and the extra (target-specific)
fields are shown as "diff"s.

### Common fields

> **NOTE** For simplicity, the author has written the target specifications as JSON (with trailing
> commas).

``` json
{
    "arch": "arm",
    "data-layout": "e-m:e-p:32:32-i64:64-v128:64:128-a:0:32-n32-S64",
    "executables": true,
    "os": "none",
    "pre-link-args": ["-Wl,-("],
    "post-link-args": ["-Wl,-)"],
    "target-endian": "little",
    "target-pointer-width": "32",
}
```

### cortex-m0

``` diff
     "target-pointer-width": "32",
+    "cpu": "cortex-m0",
+    "llvm-target": "thumbv6m-none-eabi",
 }
```

### cortex-m0plus

``` diff
     "target-pointer-width": "32",
+    "cpu": "cortex-m0plus",
+    "llvm-target": "thumbv6m-none-eabi",
 }
```

### cortex-m1

``` diff
     "target-pointer-width": "32",
+    "cpu": "cortex-m1",
+    "llvm-target": "thumbv6m-none-eabi",
 }
```

### cortex-m3

``` diff
     "target-pointer-width": "32",
+    "cpu": "cortex-m3",
+    "llvm-target": "thumbv7m-none-eabi",
 }
```

### cortex-m4

``` diff
     "target-pointer-width": "32",
+    "cpu": "cortex-m4",
+    "features": "+soft-float",
+    "llvm-target": "thumbv7em-none-eabi",
 }
```

### cortex-m4f

``` diff
     "target-pointer-width": "32",
+    "cpu": "cortex-m4",
+    "llvm-target": "thumbv7em-none-eabi",
 }
```

### cortex-m7

``` diff
     "target-pointer-width": "32",
+    "cpu": "cortex-m7",
+    "features": "+soft-float",
+    "llvm-target": "thumbv7em-none-eabi",
 }
```

### cortex-m7f

``` diff
     "target-pointer-width": "32",
+    "cpu": "cortex-m7",
+    "llvm-target": "thumbv7em-none-eabi",
 }
```

### cortex-m7f-sp

``` diff
     "target-pointer-width": "32",
+    "cpu": "cortex-m7",
+    "features": "+fp-only-sp",
+    "llvm-target": "thumbv7em-none-eabi",
 }
```

### Rationale

Behind the election of these values for the fields of the target specifications:

- `arch`. (Used for conditional compilation). Set to `arm` to match the other ARM targets.
- `data-layout`. (Code generation related). Also matches the value of the other ARM targets.
- `executables`. Set to `true` because the Rust compiler can generate both libraries and executables
  for these targets.
- `os`. (For conditional compilation). No Rust precedent for this value as all the targets currently
  supported by Rust have some form of OS or runtime. `none` has been chosen as this follows the
  existing tooling (gcc and LLVM) convention of using `none` for the OS field of triples like
  `arm-none-eabi`.
- `pre-link-args` and `post-link-args`. By default, linkers are sensitive to the order of the
  arguments they receive. Using these linker arguments undoes that default and can help avoid
  "undefined reference" problems when linking statically. This technique is already used in the MUSL
  targets that do static linking (`i686` and `x86_64` variants).
- `target-endian`. (For conditional compilation). The Cortex-M architecture can be either little or
  big endian. However, for microcontrollers, endianness is chosen at manufacture time and, as far as
  the author knows, most (or all?) devices are little endian.
- `target-pointer-width`. (For conditional compilation). All the existing Cortex-M processors are
  32-bit processors.
- `cpu`. (Enables CPU specific optimizations). The value pretty much matches the target name.
- `features`. (Used to control the generation of FPU instructions). Only needed on the Cortex-M4 and
  Cortex-M7 targets.
- `llvm-target`. (Controls code generation). Meaning of each field of the triple:
  - `thumbv*m`. `thumb` indicates that the targets will use the Thumb instruction set instead of the
    (older) ARM instruction set ("These processors support the Thumb instruction set only. -
    [ARM]"). The `v*m` bit indicates the version/variant of the ARM architecture: `ARMv6-M`,
    `ARMv7-M` or `ARMv7E-M`.
  - `none`. Indicates the lack of an operating system.
  - `eabi`. Indicates that the target uses the soft-float calling convention instead of the
    hard-float calling convention (more details about this below).

[ARM]: http://infocenter.arm.com/help/index.jsp?topic=/com.arm.doc.set.cortexm/index.html

### Calling convention

Targets that have a FPU can choose between two calling conventions (CC): *soft-float* or
*hard-float*. Objects compiled with different CCs have different ABIs and can't always be linked
together. Although the *hard-float* CC generates faster code around function calls that need to pass
floating point arguments, this RFC proposes using the soft-float CC for these targets. The reason is
that this ensures maximum interoperability with pre-compiled C libraries. For example, even if a C
library has been compiled for the Cortex-M3, which doesn't have a FPU, it can still be linked with
objects compiled for the `cortex-m4f` target because they also use the soft-float ABI.

## The `rust-core` component

The build system will learn to package a new `rust-core` component that will look like this:

```
$ tree .
.
└── lib
     └── rustlib
        └── cortex-m3
            │── liballoc-$HASH.rlib
            │── libcollections-$HASH.rlib
            │── libcompiler-rt.a
            │── libcore-$HASH.rlib
            │── librand-$HASH.rlib
            └── librustc_unicode-$HASH.rlib
```

This new component will be similar to the existing `rust-std` component. The only difference is that
the `rust-core` component will ship with a smaller set of standard crates; only the ones that are
*freestanding*. A freestanding crate is a crate that doesn't depend on an underlying OS, kernel or
runtime. Currently, this is the full list of freestanding crates:

- `alloc`
- `collections`
- `core`
- `rand`
- `rustc_unicode`

This set will grow if new freestanding crates are added or if an existing crate is split in smaller
crates.

## User experience

Once these changes are in place, this is how the user experience will look like:

### Building a library

Should be as straightforward as installing the `rust-core` component for the desired target (via
rustup or any other mean) and using the `--target` flag.

```
$ rustup target add cortex-m3

$ cargo new hal && cd $_

$ edit src/lib.rs && cat src/lib.rs
```

``` rust
#![feature(asm)]
#![no_std]

fn foo() {
    unsafe {
        asm!("bkpt");
    }
}

// LLVM will lower this operation to the `__aeabi_fadd` intrinsic which will be provided by libcompiler-rt.a
fn bar(x: f32, y: f32) -> f32 {
    x + y
}

// ...
```

```
$ cargo build --target cortex-m3

$ arm-none-eabi-readelf -A target/cortex-m3/debug/libhal.rlib
File: target/cortex-m3/debug/libhal.rlib(hal.0.o)
Attribute Section: aeabi
File Attributes
  Tag_conformance: "2.09"
  Tag_CPU_name: "cortex-m3"
  Tag_CPU_arch: v7
  Tag_CPU_arch_profile: Microcontroller
  Tag_ARM_ISA_use: No
  Tag_THUMB_ISA_use: Thumb-2
  Tag_ABI_PCS_R9_use: V6
  Tag_ABI_PCS_RW_data: PC-relative
  Tag_ABI_PCS_RO_data: PC-relative
  Tag_ABI_PCS_GOT_use: GOT-indirect
  Tag_ABI_FP_denormal: Needed
  Tag_ABI_FP_exceptions: Needed
  Tag_ABI_FP_number_model: IEEE 754
  Tag_ABI_align_needed: 8-byte
  Tag_ABI_align_preserved: 8-byte, except leaf SP
  Tag_ABI_optimization_goals: Prefer Debug
  Tag_CPU_unaligned_access: v6
  Tag_ABI_FP_16bit_format: IEEE 754
```

### Building a binary

Two extra steps are required to build a binary:

- Tell Cargo which linker to use.
- Supply a linker script to the linker.
  
[vector table]: http://infocenter.arm.com/help/topic/com.arm.doc.dui0552a/BABIFJFG.html

```
$ rustup target add cortex-m3

$ cargo new --bin app && cd $_

# Tell Cargo about the linker and the linker script.
$ mkdir .cargo && edit .cargo/config && cat .cargo/config
```

``` toml
[build]
rustflags = ["-C", "link-args=-Tlayout.ld"]

[target.cortex-m3]
linker = "arm-none-eabi-gcc"
```

```
# The is the linker script
$ edit layout.ld && cat layout.ld
/* Device-specific memory regions */
MEMORY
{
  FLASH : ORIGIN = 0x08000000, LENGTH = 128K
  SRAM  : ORIGIN = 0x20000000, LENGTH = 8K
}

/* Omitted:
   - Definition and placement of the vector table and other program sections
   - Addresses of memory mapped peripherals */

$ edit src/main.rs && cat src/main.rs
```

``` rust
#![no_main]
#![no_std]

// Program entry point
fn start() -> ! {
    // ...
}

// Omitted: definition of lang items, placing `start` in the vector table, etc.
```

```
$ cargo build --target cortex-m3

$ file target/cortex-m3/debug/app
target/cortex-m3/debug/app ELF 32-bit LSB executable, ARM, EABI5 version 1 (SYSV), statically linked, not stripped
```

# Drawbacks
[drawbacks]: #drawbacks

None so far.

# Alternatives
[alternatives]: #alternatives

## Don't do this

Leave it up to the user figure out how to set up their development environment.

Luckily, the author is currently writing [extensive documentation] on the area of Rust on
microcontrollers, and also has developed tools that make cross compiling [freestanding crates] and
[compiler-rt] relatively easy. However, adding these targets to the compiler will make the setup
process simpler (less steps, requires less tooling, etc).

[extensive documentation]: http://japaric.github.io/copper
[freestanding crates]: https://github.com/japaric/xargo
[compiler-rt]: https://github.com/japaric/compiler-rt.rs

## Use the hard-float CC instead of the soft-float one

The main proposal opted for the soft-float CC, favoring interoperability over performance. However,
using the hard-float CC to favor performance is also a valid alternative.

Yet another alternative is to provide both CC variants for the targets that support them. For
example, instead of a single `cortex-m4f` target now there would be a `cortex-m4f` target for the
soft-float CC and a `cortex-m4f-hf` target for the hard-float CC.

## Less targets

It's possible to reduce the number of targets that need to be added if the CPU specific
optimizations (the `cpu` field in the target specifications) are dropped. This leads to the
following smaller set of targets:

- `thumbv6m-none-eabi`: Subsumes the `cortex-m0`, `cortex-m0plus` and `cortex-m1` targets.
- `thumbv7m-none-eabi`: Equivalent to the `cortex-m3` target.
- `thumbv7em-none-eabi`: Subsumes the `cortex-m4` and `cortex-m7` targets. No FPU instructions.
  Soft-float calling convention.
- `thumbv7em-none-eabihf`: Like `thumbv7em-none-eabi` but uses the hard-float calling convention.
  Also no FPU instructions.
  
### Emulating the targets of the original proposal

All the targets from the original proposal can be emulated using this smaller set of targets plus
a few extra compilation flags:

| Original target | New target            | Extra compilation flags                                 |
|-----------------|-----------------------|---------------------------------------------------------|
| `cortex-m0`     | `thumbv6m-none-eabi`  | `-C target-cpu=cortex-m0`                               |
| `cortex-m0plus` | `thumbv6m-none-eabi`  | `-C target-cpu=cortex-m0plus`                           |
| `cortex-m1`     | `thumbv6m-none-eabi`  | `-C target-cpu=cortex-m1`                               |
| `cortex-m3`     | `thumbv7m-none-eabi`  | `-C target-cpu=cortex-m3`                               |
| `cortex-m4`     | `thumbv7em-none-eabi` | `-C target-cpu=cortex-m4 -C target-feature=+soft-float` |
| `cortex-m4f`    | `thumbv7em-none-eabi` | `-C target-cpu=cortex-m4`                               |
| `cortex-m7`     | `thumbv7em-none-eabi` | `-C target-cpu=cortex-m7 -C target-feature=+soft-float` |
| `cortex-m7f`    | `thumbv7em-none-eabi` | `-C target-cpu=cortex-m7`                               |
| `cortex-m7f-sp` | `thumbv7em-none-eabi` | `-C target-cpu=cortex-m7 -C target-feature=+fp-only-sp` |

For example:

```
# Original proposal
$ cargo build --target cortex-m4

# This proposal
$ cargo rustc --target thumbv7em-none-eabi -- -C target-cpu=cortex-m4 -C target-feature=+soft-float

# Or, alternatively, one can use Cargo's build.rustflags or the RUSTFLAGS env variable
$ cat .cargo/config
[build]
rustflags = ["-C", "target-cpu=cortex-m4", "-C", "target-feature=+soft-float"]

$ cargo build --target thumbv7em-none-eabi
```

The advantage of this proposal is that one is not limited to the soft-float CC on devices with FPU
because one can change the target to `thumbv7em-none-eabihf` to switch to the hard-float CC.

```
# `cortex-m4f` target but with hard float CC
$ cargo rustc --target thumbv7em-none-eabihf -- -C target-cpu=cortex-m4
```
  
### Downsides

Although more standard these target triples are more cryptic. It's unclear that one must use the
`thumbv7m-none-eabi` target if one wants to cross compile for a Cortex-M3 processor whereas the
`cortex-m*` targets make the election of the target obvious. This issue can be addressed with
documentation, though.

In the case of the `thumbv7em-none-eabi*` targets, the crates that ship with the `rust-core`
component are always compiled without CPU optimizations and, as a consequence, don't contain any FPU
instruction as all their floating point operations get lowered to intrinsics. This means that a
program compiled with flags that enable FPU instructions *may* (\*) still end up using intrinsics
instead of FPU instructions if the program calls a routine from e.g. `libcore.rlib` that performs a
floating point operation.

#### (\*)  Author note

I tested the scenario described above. But, to my surprise, LLVM appears to be smart enough to
convert an intrinsic back into a FPU instruction when CPU optimizations are enabled.

Here's the program I tried:

```
$ cat src/main.rs
```

``` rust
fn start() -> ! {
    let x = 1f64;
    let y = x.to_degrees();
    
    // ...
}
```

`libcore.rlib` was compiled *without* CPU optimizations and, as seen below, its `to_degrees` routine
uses the `__aeabi_dmul` intrinsic instead of FPU instructions.

```
$ arm-none-eabi-objdump -Cd $(rustc --print sysroot)/lib/rustlib/thumbv7em-none-eabi/lib/libcore-*.rlib
(..)

00000000 <core::f64::_$LT$impl$u20$core..num..Float$u20$for$u20$f64$GT$::to_degrees::he4eaab80e305699e>:
   0:   b580            push    {r7, lr}
   2:   f24c 12f8       movw    r2, #49656      ; 0xc1f8
   6:   f24a 53dc       movw    r3, #42460      ; 0xa5dc
   a:   f6c1 2263       movt    r2, #6755       ; 0x1a63
   e:   f2c4 034c       movt    r3, #16460      ; 0x404c
  12:   f7ff fffe       bl      0 <__aeabi_dmul>
  16:   bd80            pop     {r7, pc}

(..)
```

However, when I compiled the Cargo project with CPU optimizations enabled...

````
$ cargo rustc --target thumbv7em-none-eabi -- -C target-cpu=cortex-m7
$ arm-none-eabi-objdump -Cd target/thumbv7em-none-eabi/debug/app
(..)

00000038 <core::f64::_$LT$impl$u20$core..num..Float$u20$for$u20$f64$GT$::to_degrees::he4eaab80e305699e>:
  38:   b082            sub     sp, #8
  3a:   ec41 0b10       vmov    d0, r0, r1
  3e:   ed8d 0b00       vstr    d0, [sp]
  42:   ed9f 1b05       vldr    d1, [pc, #20]   ; 58 <core::f64::_$LT$impl$u20$core..num..Float$u20$for$u20$f64$GT$::to_degrees::he4eaab80e305699e+0x20>
  46:   ee20 0b01       vmul.f64        d0, d0, d1
  4a:   ec51 0b10       vmov    r0, r1, d0
  4e:   b002            add     sp, #8
  50:   4770            bx      lr
  52:   bf00            nop
  54:   bf00            nop
  56:   bf00            nop
  58:   1a63c1f8        .word   0x1a63c1f8
  5c:   404ca5dc        .word   0x404ca5dc

(..)
```

:tada: LLVM updated the `to_degrees` function by replacing the `__aeabi_dmul` intrinsic with a more
performant implementation that uses the `vmul.vf64` FPU instruction.

However, **I'm** not sure if we can rely on LLVM **always** being capable of doing this.

# Unresolved questions
[unresolved]: #unresolved-questions

## Linker field

Should we set the `linker` field of all the target specifications to `arm-none-eabi-gcc`?
`arm-none-eabi-gcc` seems to be the most common (or maybe the only one?) bare-metal ARM toolchain.
This would save the user the trouble of having to specify the `target.$TARGET.linker` field in all
their projects' `.cargo/config`.

# Appendix
[appendix]: #appendix

## About FPUs and floating point operations

A processor that sports a FPU, like the Cortex-M7F, can perform floating point arithmetic, like
double precision addition, in a single *FPU* instruction (`vadd.f64`). Whereas, a processor that has
no FPU, like the Cortex-M0, doesn't have such instructions; it only has instruction to perform
integer arithmetic.

However, a processor without FPUs can still do floating point arithmetic by implementing operations
like addition as software routines that only use integer arithmetic. The `compiler-rt` library
provides such software routines as LLVM *intrinsics*, e.g. `__aeabi_dadd` which is the software
counterpart of `vadd.f64`. When LLVM needs to generate code for a floating point operation for a
FPUless target, it translates that operation to an intrinsic instead of to a FPU instruction.

It goes without saying that a emulating floating point arithmetic in software is slower that
directly using FPU instructions.
