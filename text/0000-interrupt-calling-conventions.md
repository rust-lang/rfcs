# Interrupt Calling Conventions

- Feature Name: `interrupt_calling_conventions`
- Start Date: 2022-02-19
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add compiler support for interrupt calling conventions that are specific to an architecture or target. This way, interrupt handler functions can be written directly in Rust without needing assembly shims.

# Background

This section gives some introduction to calling conventions in general, summarizes the current support of alternative calling conventions in Rust, and explains why interrupt handlers require special calling conventions.

## Calling Conventions
Calling conventions define how function calls are performed, including:

- how function arguments are passed, e.g. in specific CPU registers, on the stack, as a pointer, etc.
- how the function returns its result
- which registers must be preserved by the function
- setup and clean-up of the stack frame, e.g. whether the caller or callee restores the stack to it's previous state again

Calling conventions are a large part of a function's ABI ([application binary interface](https://en.wikipedia.org/wiki/Application_binary_interface)), so the terms are sometimes used interchangeably.

## Current Support
By default, Rust uses an internal `"Rust"` calling convention, which is not standardized and might change in the future. For interoperating with external code, Rust allows to set the calling convention of a function explicitly through an `extern "calling_conv" fn foo() {}` function qualifier. The calling convention of external functions can be specified through `extern "calling_conv" { fn bar(); }`.

The most common alternative calling convention supported by Rust is `extern "C"`, which can be used to interface with most code written in C. In addition, Rust supports various [other calling conventions](https://doc.rust-lang.org/stable/reference/items/external-blocks.html#abi), which are required in more specific cases. Most alternative calling conventions are only supported on a single architecture, for example the `"aapcs"` ABI is only supported on ARM systems.

## Interrupt Handlers
While most functions are invoked by other software, there are some cases where the hardware (or its firmware) invokes a function directly. The most common example are [interrupt handler](https://en.wikipedia.org/wiki/Interrupt_handler) functions defined in embedded systems or operating system kernels. These functions are set up to be called directly by the hardware when a specific interrupt fires (e.g. when a network packet arrives). Interrupt handlers are also called _interrupt service routines_ (ISRs).

Depending on the architecture, a special calling convention is required for such interrupt handler functions. For example, interrupt handlers are often required to restore all registers to their previous state before returning because interrupts happen asynchronously while other code is running. Also, they often receive additional state as input and need to follow a special procedure on return (e.g. use the `iretq` instruction on `x86_64`).

# Motivation
[motivation]: #motivation

Since the hardware platform requires a special calling convention for interrupt handlers, we cannot define the handler functions directly in Rust using any of the currently supported calling conventions. Instead, we need to define a wrapper function in raw assembly that acts as a compatibility layer between the interrupt calling convention and the calling convention of the Rust function. For example, with an `extern "C"` Rust function using the System V AMD64 ABI, the wrapper would need to do the following steps on `x86_64`:

- Backup all registers on the stack that are not preserved by the C calling convention
  - This includes all registers except `RBX`, `RSP`, `RBP`, and `R12`–`R15` (these are restored by `extern "C"` functions)
  - This also includes floating point and SSE state, which can be huge (unless we are sure that the interrupt handler does not use the corresponding registers)
- Align the stack on a 16-byte boundary (required by the C calling convention)
- Copy the arguments (passed on the stack) into registers (where the C calling convention expects them)
- Call the Rust function
- Clean up the stack, including the alignment bytes and arguments.
- Restore all registers
- Invoke the `iretq` instruction to return from the interrupt

This approach has lots of issues. For one, assembly code is difficult to write and especially difficult to write _correctly_. Errors can easily lead to silent undefined behavior, for example when mixing up two registers when restoring their values. What makes things worse is that the correctness also depends on the compilation settings. For example, there are multiple variants of the C calling convention for `x86_64`, depending on whether the target system is specified as Windows or Unix-compatible.

The other issue of the above approach is its performance overhead. Interrupt handlers are often invoked with a very high frequency and at a high priority, so they should be as efficient as possible. However, custom assembly code cannot be optimized by Rust or LLVM, so no inlining or copy elision happens.

Another source of additional performance overhead is caused by the register saving step. The assembly wrapper function has no way to know which registers are actually used by the wrapped function, so it has to save and restore all registers that the Rust function could _possibly_ use, even if only a subset of them is actually overwritten.

To avoid these issues, this RFC proposes to add native support for _interrupt calling conventions_ to the Rust language. This makes it possible to define interrupt handlers directly as Rust functions, without requiring any wrapper functions or custom assembly code. Because all code is generated by the compiler, full code optimization is possible.

Rust already supports three different interrupt calling conventions as experimental features: `msp430-interrupt`, `x86-interrupt`, and `avr-interrupt`. They are already widely used in embedded and operating system kernel projects, so this feature also seems to be useful in practice.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

In addition to ABIs for interfacing with external code, Rust also supports so-called _interrupt ABIs_ to define interrupt handler functions that can interface directly with the hardware. These ABIs are only needed for bare-metal applications such as embedded systems or operating system kernels. The ABIs are special because they impose requirements on the whole signature of the function, including arguments and return values.

The following interrupt ABIs are currently supported:

- _(unstable)_ `extern "msp430-interrupt"`: Allows to create interrupt handlers on MSP430 microcontrollers. Functions must have the signature `unsafe extern "msp430-interrupt" fn()`. To add a function to the interrupt table, use the following snippet:

  ```rust
  #[no_mangle]
  #[link_section = "__interrupt_vector_10"]
  pub static TIM0_VECTOR: unsafe extern "msp430-interrupt" fn() = tim0;

  unsafe extern "msp430-interrupt" fn tim0() {...}
  ```

  Then place the `__interrupt_vector_10` section in the interrupt handler table using a linker script.
- _(unstable)_ `extern "x86-interrupt"`: This calling convention can be used for defining interrupt handlers on 32-bit and 64-bit `x86` targets. Functions must have one of the following two signatures, depending on the interrupt vector:

  ```rust
  extern "x86-interrupt" fn(stack_frame: &StackFrame);
  extern "x86-interrupt" fn(stack_frame: &StackFrame, error_code: ErrorCode);
  ```
  The `error_code` argument is _not_ an optional argument. It is set by the hardware for some interrupt vector, but not for others. The programmer must make sure to always use the correct signature for each interrupt vector, otherwise undefined behavior occurs.

  The `StackFrame` type must be a struct that matches the stack frame pushed by the CPU. The `ErrorCode` type must be `u64` on 64-bit targets and `u32` on 32-bit targets. These types are currently _not_ checked by `rustc`.
- _(unstable)_ `extern "avr-interrupt"` and `extern "avr-non-blocking-interrupt"`

_(The above calling conventions are just listed as an example. They are **not** part of this RFC.)_

By using these ABIs, it is possible to implement interrupt handlers directly in Rust, without writing any custom assembly code. This is not only safer and more convenient, it also often results in better performance. The reason for this is that the compiler can employ (cross-function) optimization techniques this way, for example to only backup the CPU registers that are actually overwritten by the interrupt handler.

On some platforms, there might be multiple interrupt calling conventions with different behavior. For example, on AVR there are separate calling conventions that either disable global interrupts while the interrupt handler is running, or keep them enabled. Another example is the ARM architecture, where it might make sense to add multiple interrupt calling conventions to allow switching to a specific CPU mode before invoking the interrupt handler.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The exact requirements and properties of the individual interrupt calling conventions must be defined and documented before stabilizing them. However, there are some properties and requirements that apply to all interrupt calling conventions.

## Compiler Checks
Interrupt calling conventions have strict requirements that are checked by the Rust compiler:

- They must not be called by Rust code.
- The function signature must match a specific template.
- They are only available on specific targets and might require specific target settings.
- All other requirements imposed by the implementation of the calling convention in LLVM.

If any of these conditions are violated, the compiler throws an error. It should not be possible to cause LLVM errors using interrupt calling conventions.

## Platform Support
Since interrupt calling conventions are closely tied to a target and only available on that specific target, they are treated as a platform feature and fall under Rust's [target tier policy](https://doc.rust-lang.org/nightly/rustc/target-tier-policy.html).

This means that calling conventions are only as well supported as the corresponding target. So on _tier 3_ targets, there are no guarantees that corresponding interrupt calling conventions will build, even if they're stabilized. On _tier 2_ targets, interrupt calling coventions are guaranteed to build, but no automated tests are run. Only on _tier 1_ targets, there is a guarantee that interrupt calling conventions will work.

## Stability
Interrupt calling conventions are a language feature, so they fall under Rust's normal stability guarantees, with one exception: If official support for a target is dropped, the corresponding interrupt calling convention can be removed from the Rust language, even if it is stabilized. This is not considered a breaking change because no code on other targets is broken by this, since it was never possible to use the calling convention on other targets.

As soon as a calling convention is stabilized, breaking changes are no longer allowed, independent of the target tier. For this reason, special caution must be taken before stabilizing interrupt calling conventions for _tier 3_ targets, as these targets might still evolve. Special care must also be taken before stabilizing interrupt calling conventions that are implemented outside of `rustc` (e.g. in LLVM).

## Safety
Functions with interrupt calling conventions are considered normal Rust functions. No `unsafe` annotations are required to declare them and there are no restrictions on their implementation. However, it is not allowed to call such functions from (Rust) code since the custom prelude and epilogue of the functions could lead to memory safety violations. For this reason, the attempt to call a function defined with an interrupt calling convention must result in an hard error that cannot be circumvented through `unsafe` blocks or by allowing some lints.

The only valid way to invoke a function with an interrupt calling convention is to register them as an interrupt handler directly on the hardware, for example by placing their address in an _interrupt descriptor table_ on `x86_64`. There is no way for the compiler to verify that this operation is correct, so special care needs to be taken by the programmer to ensure that no violation of memory safety can occur.

# Drawbacks
[drawbacks]: #drawbacks

Interrupt calling conventions can be quite complex. So even though they are a very isolated feature, they still **add a considerable amount of complexity to the Rust language**. This added complexity could lead to considerable work for alternative Rust compilers/code generators that don't build on top of LLVM. Examples are [`cranelift`](https://github.com/bytecodealliance/wasmtime/tree/main/cranelift), [`gccrs`](https://github.com/Rust-GCC/gccrs), [`rust_codegen_gcc`](https://github.com/rust-lang/rustc_codegen_gcc), or [`mrustc`](https://github.com/thepowersgang/mrustc).

Most interrupt calling conventions are still unstable/undocumented features of LLVM, so we need to be cautious about stabilizing them in Rust. Stabilizing them too early could lead to maintenance problems and **might make LLVM updates more difficult**, e.g. when some barely maintained calling convention is accidentally broken in the latest LLVM release. There is also the danger that LLVM drops support for an interrupt calling convention at some point. If the calling convention is already stabilized in Rust, we would need to find an alternative way to provide that functionality.

The proposed feature is **only needed for applications in a specific niche**, namely embedded programs and operating system kernel.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

As described in the [_Motivation_](#motivation), the main alternative to interrupt calling conventions are wrapper functions written in assembly, e.g. in a naked function. This reduces the maintenance burden for the Rust compiler, but makes interrupt handlers inconvenient to write, more dangerous, and less performant.

## Alternative: Calling Convention that Preserves all Registers

Many of the advantages of compiler-supported interrupt calling conventions come from the automated register handling, i.e. that all registers are restored to their previous state before returning. We might also be able achieve this using a calling convention that preserves all registers, for example LLVM's [`preserve_all`](https://llvm.org/docs/LangRef.html#calling-conventions) calling convention.

Such a calling convention could be platform independent and should be much easier to maintain. It could also be called normally from Rust code and might thus have use cases outside of interrupt handling, e.g. similar to functions annotated as [`#[cold]`](https://doc.rust-lang.org/reference/attributes/codegen.html#the-cold-attribute).

Using such a calling convention, it should be possible to create interrupt handler wrappers in assembly with comparable performance. These wrapper functions would handle the platform-specific steps of interrupt handling, such as stack alignment, argument preprocessing, and the interrupt epilogue. Since no language support is required for these wrapper functions, they don't impact the maintainability of the compiler and can evolve independently in libraries. Using proc macros, they could even provide a similar level of usability to users.

While this approach could be considered a good middle ground, full compiler support for interrupt calling conventions is still be the better solution from a usability and performance perspective.

## Alternative: Implementation in `rustc`

Instead of relying on LLVM (or alternative code generators) to implement the interrupt calling conventions, we could also try to implement support for the calling conventions in `rustc` directly. This way, LLVM upgrades would not be affected by this feature and we would be less dependent on LLVM in general. One possible implementation approach for this could be to build upon a calling convention that preserves all registers (see the previous section).

The drawback of this approach is increased complexity and maintenance cost for `rustc`.

## Alternative: Single `interrupt` ABI that depends on the target

Instead of adding multiple target-specific interrupt calling conventions under different names, we could add support for a single cross-platform `extern "interrupt"` calling convention. This calling convention would be an alias for the interrupt calling convention of the target system, e.g. `x86-interrupt` when compiling for an `x86` target.

The main advantage of this approach would be that we keep the list of supported ABI variants short, which might make the documentation clearer. However, there are also several drawbacks:

- Some targets have multiple interrupt calling conventions (e.g. `avr` and `avr-non-blocking`). This would be difficult to represent with a single `extern "interrupt"` calling convention.
- Interrupt handlers on different targets require different function signatures. It would be difficult to abstract this cleanly.
- Interrupt handler implementations are often highly target-specific, so that there is not much value in cross-platform handlers. In fact, it could even lead to bugs when an interrupt handler is accidentally reused on a different platform.

# Prior art
[prior-art]: #prior-art

The three interrupt calling conventions that are mentioned in this RFC are already implemented as experimental features in `rustc` since multiple years (`msp430-interrupt` and `x86-interrupt` since 2017, `avr-interrupt` since 2020). They are already in use in several projects and were deemed useful enough that the Rust language team [decided](https://github.com/rust-lang/rust/issues/40180#issuecomment-1022507941) to consider this feature for proper inclusion.

There was already a [prior RFC](https://github.com/rust-lang/rfcs/pull/1275) for interrupt calling conventions in 2015. The RFC was [closed](https://github.com/rust-lang/rfcs/pull/1275#issuecomment-154494283) for the time being to explore naked functions as a potential alternative first. Naked functions are now on the [path to stabilization](https://github.com/rust-lang/rust/issues/90957#issuecomment-1028297041), but there is still value in support for interrupt calling conventions, as described in this RFC.

GCC supports a cross-platform [`__interrupt__` attribute](https://gcc.gnu.org/onlinedocs/gcc/Function-Attributes.html) for creating interrupt handlers. The behavior is target-specific and very similar to the proposal of this RFC. The LLVM-based Clang compiler also supports this attribute for a [subset of targets](https://clang.llvm.org/docs/AttributeReference.html#interrupt-arm).

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- What are the exact requirements for stabilizing an interrupt calling convention? What level of stability of the LLVM implementation is required?
- Is there a way to implement interrupt calling conventions directly in `rustc` without LLVM support?

# Future possibilities
[future-possibilities]: #future-possibilities

This feature is relatively isolated in limited in scope, so it is not expected that this feature will be extended in the future.
