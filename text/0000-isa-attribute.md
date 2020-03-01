- Feature Name: isa_attribute
- Start Date: 2020-02-16
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC proposes a new function attribute, `#[instruction_set(arch, set)]` which allows you to declare the instruction set to be used when compiling the function for a given arch. It also proposes two initial allowed values for the ARM arch (`a32` and `t32`). Other allowed values could be added to the language later.

# Motivation
[motivation]: #motivation

Most programmers are familiar with the idea of a CPU family having more than one instruction set. `x86_64` is backwards compatible with `x86`, and an `x86_64` CPU can run an `x86` program if necessary.

Starting with `ARMv4T`, many ARM CPUs support two separate instruction sets. At the time they were called "ARM code" and "Thumb code", but with the development of `AArch64`, they're now called `a32` and `t32`. Unlike with the `x86` / `x86_64` situation, on ARM you can have a single program that intersperses both `a32` and `t32` code. A particular form of branch instruction allows for the CPU to change between the two modes any time it branches, and so generally code is designated as being either `a32` or `t32` on a per-function basis.

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

Guarantees:
* If an alternate instruction set is designated on a function then the compiler _must_ respect that. It is not a hint, it is a guarantee.

What is a Compile Error:
* If an alternate instruction set is designated that doesn't exist (eg: "unicorn") then that is a compiler error.
* If the attribute appears more than once for a _single arch_ on a function that is a compile error.
* Specifying an alternate instruction set attribute more than once with each usage being for a _different arch_ it is allowed.

Inlining:
* For the alternate instruction sets proposed by this RFC, `a32` and `t32`, what is affected is the actual generated assembly and symbol placement of the generated function. If a function's body is inlined into the caller then the attribute no longer has a meaningful effect within the caller's body, and would be ignored.
* This does mean that any inline `asm!` calls in alternate instruction set functions could be inlined into the wrong instruction set within the caller's body. It would be up to the programmer to specify `inline(never)` if this is a concern. However, the primary goal of this RFC is to eliminate the need for inline `asm!` in the first place.

How _specifically_ does it work on ARM:
* Within an ELF file, all `t32` code functions are stored as having odd value addresses, and when a branch-exchange (`bx`) or branch-link-exchange (`blx`) instruction is used then the target address's lowest bit is used to move the CPU between the `a32` and `t32` states appropriately.
* Accordingly, this does _not_ count as a full new ABI of its own. Both "Rust" and "C" ABI functions and function pointers are the same type as they were before.
* Linkers for ARM platforms such as [gnu ld](https://sourceware.org/binutils/docs/ld/ARM.html#ARM) have various flags to help the "interwork" process, depending on your compilation settings.
* This is considered a very low level and platform specific feature, so potentially having to pass additional linker args **is** considered an acceptable level of complexity for the programmer, though we should attempt to provide "good defaults" if we can of course.

# Drawbacks
[drawbacks]: #drawbacks

* Adding another attribute complicates Rust's design.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

* Extending `target_feature` to allow `#[target_feature(disable = "...")]` and adding `thumb-mode` to the whitelist would support this functionality without adding another attribute; however, this is verbose, and does not fit with the `target_feature` attribute's current focus on features such as AVX and SSE whose absence is not necessarily compensated for by the presence of something else.

* Doing nothing is an option; it is currently possible to incorporate code using other instruction sets through means such as external assembly and build scripts. However, this has greatly reduced ergonomics.

# Prior art
[prior-art]: #prior-art

In C you can use `__attribute__((target("arm")))` and `__attribute__((target("thumb")))` to access similar functionality. It's a compiler-specific extension, but it's supported by both GCC and Clang.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Hopefully none?

# Future possibilities
[future-possibilities]: #future-possibilities

* LLVM might eventually gain support for inter-instruction-set calls that allow calls between two arches (eg: a hybrid PowerPC/RISC-V). In that case, we could extend the attribute to allow new options.

* If Rust gains support for the 65C816, the `#[instruction_set(?)]` attribute might be extended to allow shifting into its 65C02 compatibility mode and back again.

* MIPS has a 16-bit encoding which uses a similar scheme as ARM, where the low bit of a function's address is set when the 16-bit encoding is in use for that function.
