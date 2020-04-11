- Feature Name: isa_attribute
- Start Date: 2020-02-16
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC proposes a new function attribute, `#[instruction_set(arch, set)]` which allows you to declare the instruction set to be used when compiling the function for a given arch. It also proposes two initial allowed values for the ARM arch (`a32` and `t32`). Other allowed values could be added to the language later.

# Motivation
[motivation]: #motivation

Starting with `ARMv4T`, many ARM CPUs support two separate instruction sets. At the time they were called "ARM code" and "Thumb code", but with the development of `AArch64`, they're now called `a32` and `t32`. Unlike with the `x86_64` architecture, where the CPU can run both `x86` and `x86_64` code, but a single program still uses just one of the two instruction sets, on ARM you can have a single program that intersperses both `a32` and `t32` code. A particular form of branch instruction allows for the CPU to change between the two modes any time it branches, and so generally code is designated as being either `a32` or `t32` on a per-function basis.

In LLVM, selecting that code should be `a32` or `t32` is done by either disabling (for `a32`) or enabling (for `t32`) the `thumb-mode` target feature. Previously, Rust was able to do this using the `target_feature` attribute because it was able to either add _or subtract_ an LLVM target feature during a function. However, when [RFC 2045](https://github.com/rust-lang/rfcs/blob/master/text/2045-target-feature.md) was accepted, its final form did not allow for the subtraction of target features. Its final form is primarily designed around always opting _in_ to additional features, and it's no longer the correct tool for an "either A or B, but not both" situation like `a32`/`t32` is.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Some platforms support having more than one instruction set used within a single program. Generally, each one will be better for specific parts of a program. Every target has a default instruction set, based on the target triple. If you would like to set a specific function to use an alternate instruction set you use the `#[instruction_set(arch, set)]` attribute. This specifies that when the code is built for then given arch, it should use the alternate instruction set specified instead of the default one.

Currently this is only of use on ARM family CPUs, which support both the `a32` and `t32` instruction sets. Targets starting with `arm` default to `a32` and targets starting with `thumb` default to `t32`.

```rust
// this uses the default instruction set for your target

fn add_one(x: i32) -> i32 {
    x + 1
}

// This will compile as `a32` code on both `arm` and thumb` targets

#[instruction_set(arm, a32)]
fn add_five(x: i32) -> i32 {
    x + 5
}
```

To help with code portability, when the function is compiled for any arch other than the arch given then the attribute has no effect. If the `add_five` function were built for `x86_64` then it would be the same as having no `instruction_set` attribute.

If you specify an instruction set that the compiler doesn't recognize then you will get an error.

```rust
#[instruction_set(arm, unicorn)]
fn this_does_not_build() -> i32 {
    7
}
```

The specifics of _when_ to specify a non-default instruction set on a function are platform specific. Unless a piece of platform documentation has indicated a specific requirement, you do not need to think about adding this attribute at all.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Every target is now considered to have one default instruction set (for functions that lack the `instruction_set` attribute), as well as possibly supporting specific additional instruction sets:

* The targets with names that start with `arm` default to `(arm, a32)`, but can also use `(arm, t32)`.
* The targets with names that start with `thumb` default to `(arm, t32)`, but can also use `(arm, a32)`.
* The `instruction_set` attribute is not currently defined for use with any other arch.

Backend support:
* In LLVM this corresponds to enabling or disabling the `thumb-mode` target feature on a function.
* Other future backends (eg: Cranelift) would presumably support this in some similar way. A "quick and dirty" version of `a32`/`t32` interworking can be achieved simply by simply placing all `a32` code in one translation unit, all `t32` code in another, and then telling the linker to sort it out. Currently, Cranelift does not support ARM chips _at all_, but they can easily work towards this over time.
* Because Miri operates on Rust's MIR stage, this attribute doesn't affect the operation of Miri. If Miri were to some day support inline assembly this attribute would need to be taken into account for that to work right, but Miri could also simply choose to not support this attribute in combination with inline assembly.

Guarantees:
* If an alternate instruction set is designated on a function then the compiler _must_ respect that. It is not a hint, it is a guarantee.

Where can this attribute be used:
* This attribute can be used on any `fn` item that has a body: Free functions, inherent methods, trait default methods, and trait impl methods.
* This attribute cannot be used on closures or within `extern` block declarations.
* (Allowing this on trait prototypes is a Future Possibility.)

What is a Compile Error:
* If an alternate instruction set is designated that doesn't exist (eg: "unicorn") then that is a compiler error. Later versions of the compiler/language are free to add additional arch/instruction set pairs.
* If the attribute appears more than once for a _single arch_ on a function that is a compile error.
* Specifying an alternate instruction set attribute more than once with each usage being for a _different arch_ it is allowed.

Inlining:
* For the alternate instruction sets proposed by this RFC, `a32` and `t32`, what is affected is the actual generated assembly and symbol placement of the generated function. If a function's body is inlined into the caller then the attribute no longer has a meaningful effect within the caller's body, and would be ignored.
* This does mean that any inline `asm!` calls in alternate instruction set functions could be inlined into the wrong instruction set within the caller's body. That is one reason why `asm!` is unsafe.

How _specifically_ does it work on ARM:
* Within an ELF file, all `t32` code functions are stored as having odd value addresses, and when a branch-exchange (`bx`) or branch-link-exchange (`blx`) instruction is used then the target address's lowest bit is used to move the CPU between the `a32` and `t32` states appropriately. See the [ARM ELF spec](https://static.docs.arm.com/ihi0044/g/aaelf32.pdf), section 5.5.3.
* Accordingly, this does _not_ count as a full new ABI of its own. Both "Rust" and "C" ABI functions and function pointers are the same type as they were before. See the [ARM Procedure Call Standard](https://developer.arm.com/docs/ihi0042/g/procedure-call-standard-for-the-arm-architecture-abi-2018q4-documentation).
* Linkers for ARM platforms such as [gnu ld](https://sourceware.org/binutils/docs/ld/ARM.html#ARM) have various flags to help the "interwork" process, depending on your compilation settings. In the case of GNU ld it's called [-mthumb-interwork](https://sourceware.org/binutils/docs/ld/ARM.html)
* This is considered a very low level and platform specific feature, so potentially having to pass additional linker args **is** considered an acceptable level of complexity for the programmer, though we should attempt to provide "good defaults" if we can of course.

TODO: `-mthumb-interwork` is an `as`/`gcc` arg, not an `ld` arg, fix the link above

# Drawbacks
[drawbacks]: #drawbacks

* Adding another attribute complicates Rust's design.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Rationale

Here's a simple but complete-enough program of how this would be used in practice. In this example, the program is for the Game Boy Advance (GBA). I have attempted to limit it to the essentials, so all the MMIO definitions, as well as the assembly runtime you'd need to boot and call `main`, are still omitted from the example.

```rust
// The GBA's BIOS provides some functionality available via software
// interrupt. We expose them to Rust in our assumed assembly "runtime".
extern "C" fn {
    /// Puts the CPU into a low-power state until a vblank interrupt,
    /// and then returns after the interrupt handler completes.
    VBlankInterWait(isize, isize);
}

// We assume that the MMIO stuff is imported from somewhere.
// The exact addresses and constant values aren't important.
mod all_the_gba_mmio_definitions;
use all_the_gba_mmio_definitions::*;

fn main() {
    // All of the `write_volatile` calls here refer to
    // the method of the `*mut T` type. Proper safe abstractions
    // for all of this would complicate the example, so we
    // simply use raw pointers and one large `unsafe` block.
    unsafe {
        // set the interrupt function to be our handler
        INTR_FN_ADDR.write_volatile(core::transmute(my_inter_fn));

        // enable vblank interrupts
        DISPSTAT.write_volatile(DISPSTAT_VBLANK);
        IME.write_volatile(IME_VBLANK);
        IE.write_volatile(true);
        
        // set the device for a basic display mode.
        DISPCNT.write_volatile(MODE3_BG2);
        let mut x = 0;
        loop {
            // wait in a low-power state for the vertial blank to start.
            VBlankInterWait(0, 0);
            // draw one new red pixel per frame along the top.
            VRAM_MODE3.row(0).col(x).write(RED);
            x += 1;
            // loop our position as necessary so that we don't
            // go out of bounds.
            if x >= VRAM_MODE3::WIDTH { x = 0 }
        }
    }
}

/// Responds to any interrupt by clearing all interrupt flags
/// and then immediately returning with no other effect.
#[instruction_set(arm, a32)]
fn my_inter_fn() {
    INTER_BIOS_FLAGS.write_volatile(ALL_INTER_FLAGS);
    INTER_STANDARD_FLAGS.write_volatile(ALL_INTER_FLAGS);
}
```

1) We setup the device with our interrupt handler.
2) We set the device to have an interrupt every time the vertical blank starts.
3) We set the display to use a basic bitmap mode and begin our loop.
4) Each pass of the loop we wait for vetical blank, then draw a single pixel.

In the case of this particular device, the hardware interrupts go to the device's BIOS, which then calls your interrupt handler function. However, because the BIOS is `a32` code and uses a `b` branch instead of a `bx` branch-exchange, it jumps to the handler with the CPU in an `a32` state. If the handler were written as `t32` code it would immediately trigger UB.

## Alternatives 

* Extending `target_feature` to allow `#[target_feature(disable = "...")]` and adding `thumb-mode` to the whitelist would support this functionality without adding another attribute; however, this is verbose, and does not fit with the `target_feature` attribute's current focus on features such as AVX and SSE whose absence is not necessarily compensated for by the presence of something else.

* Doing nothing is an option; it is currently possible to incorporate code using other instruction sets through means such as external assembly and build scripts. However, this has greatly reduced ergonomics.

* Of note is the fact that this is a feature that mostly improves Rust's support for the more legacy end of ARM devices. Newer devices, with much larger amounts of memory (relatively), don't usually benefit as much. They could simply compile the entire program as `a32`, without needing to gain the space savings of `t32` code.

# Prior art
[prior-art]: #prior-art

In C you can use `__attribute__((target("arm")))` and `__attribute__((target("thumb")))` to access similar functionality. It's a compiler-specific extension, but it's supported by both GCC and Clang ([this PR](https://reviews.llvm.org/D33721) appears to be the one that added this feature to LLVM/clang).

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Hopefully none?

# Future possibilities
[future-possibilities]: #future-possibilities

* LLVM might eventually gain support for inter-instruction-set calls that allow calls between two arches (eg: a hybrid PowerPC/RISC-V). In that case, we could extend the attribute to allow new options.

* If Rust gains support for the 65C816, the `#[instruction_set(?)]` attribute might be extended to allow shifting into its 65C02 compatibility mode and back again.

* MIPS has a 16-bit encoding which uses a similar scheme as ARM, where the low bit of a function's address is set when the 16-bit encoding is in use for that function.

* It might become possible to apply this attribute to trait prototypes in a future version. The main problems are properly specifying it and also that it would add additonal compiler complexity for very minimal gain (since each impl of the trait can use it on their impl of a method if they want).
