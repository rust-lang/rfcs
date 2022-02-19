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

The most common alternative calling convention supported by Rust is `extern "C"`, which can be used to interface with most code written in C. In addition, Rust supports various [other calling conventions](https://doc.rust-lang.org/stable/reference/items/external-blocks.html#abi), which are required in more specific cases. Most alternative calling conventions are only supported on a single architecture, for example the `"aapcs"` ABI that is only supported on ARM systems.

## Interrupt Handlers
While most functions are invoked by other software, there are some cases where the hardware (or its firmware) invokes a function directly. The most common example are [interrupt handler](https://en.wikipedia.org/wiki/Interrupt_handler) functions defined in embedded systems or operating system kernels. These functions are set up to be called directly by the hardware when a specific interrupt fires (e.g. when a network packet arrives). Interrupt handlers are also called _interrupt service routines_ (ISRs).

Depending on the architecture, a special calling convention is required for such interrupt handler functions. For example, interrupt handlers are often required to restore all registers to their previous state before returning because interrupts happen asynchronously while other code is running. Also, they often receive additional state as input and need to follow a special procedure on return (e.g. use the `iretq` instruction on `x86_64`).

# Motivation
[motivation]: #motivation

Since the hardware platform requires a special calling convention for interrupt handlers, we cannot define the handler functions directly in Rust using any of the currently supported calling conventions. Instead, we need to define a wrapper function in raw assembly that acts as a compatibilty layer between the interrupt calling convention and the calling convention of the Rust function. For example, with an `extern "C"` Rust function using the System V AMD64 ABI, the wrapper would need to do the following steps on `x86_64`:

- Backup all registers on the stack that are not preserved by the C calling convention
  - This includes all registers except `RBX`, `RSP`, `RBP`, and `R12`–`R15` (these are restored by `extern "C"` functions)
  - This also includes floating point and SSE state, which can be huge (unless we are sure that the interrupt handler does not use them)
- Align the stack on a 16-byte boundary (the C calling convention requies)
- Copy the arguments (passed on the stack) into registers (where the C calling convention expects them)
- Call the Rust function
- Clean up the stack, including the alignment bytes and arguments.
- Restore all registers
- Invoke the `iretq` instruction to return from the interrupt

This approach has lots of issues. For one, assembly code is difficult to write and especially difficult to write _correctly_. Errors can easily lead to silent undefined behavior, for example when mixing up two registers when restoring their values. What makes things worse is that the correctness also depends on the compilation settings. For example, there are multiple variants of the C calling convention for `x86_64`, depending on whether the target system is specified as Windows or Unix-compatible.

The other issue of the above approach is its performance overhead. Interrupt handlers are often invoked with a very high frequency and at a high priority, so they should be as efficient as possible. However, custom assembly code cannot be optimized by Rust or LLVM, so no inlining or copy elision happens. Also, the wrapper function needs to save all registers that the Rust function could _possibly_ use, because it does not know which registers are actually written by the function.

To avoid these issues, this RFC proposes to add support for _interrupt calling conventions_ to the Rust language. This makes it possible to define interrupt handlers directly as Rust functions, without requiring any wrapper functions or custom assembly code.

Rust already supports three different interrupt calling conventions as experimental features: `msp430-interrupt`, `x86-interrupt`, and `avr-interrupt`. They are already widely used in embedded and operating system kernel projects, so this feature also seems to be useful in practice.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

In addition to ABIs for interfacing with external code, Rust also supports so-called _interrupt ABIs_ to define interrupt handler functions that can interface directly with the hardware. These ABIs are only needed for bare-metal applications such as embedded systems or operating system kernels. The ABIs are special because they impose requirements on the whole signature of the function, including arguments and return values.

The following interrupt ABIs are currently supported:

- _(unstable)_ `extern "msp430-interrupt"`: Allows to create interrupt handlers MSP430 microcontrollers. Functions must have the signature `unsafe extern "msp430-interrupt" fn()`. To add a function to the interrupt table, use the following snippet:

  ```rust
  #[no_mangle]
  #[link_section = "__interrupt_vector_10"]
  pub static TIM0_VECTOR: unsafe extern "msp430-interrupt" fn() = tim0;

  unsafe extern "msp430-interrupt" fn tim0() {...}
  ```
- _(unstable)_ `extern "x86-interrupt"`: This calling convention can be used for definining interrupt handlers on 32-bit and 64-bit `x86` targets. Functions must have one of the following two signatures, depending on the interrupt vector:

  ```rust
  extern "x86-interrupt" fn(stack_frame: &ExceptionStackFrame);
  extern "x86-interrupt" fn(stack_frame: &ExceptionStackFrame, error_code: u64);
  ```
  The `error_code` argument is _not_ an optional argument. It set by the hardware for some interrupt vector, but not for others. The programmer must make sure to always use the correct signature for each interrupt vector.
- _(unstable)_ `extern "avr-interrupt"` and `extern "avr-non-blocking-interrupt"`

_(The above calling conventions are just listed as an example. They are **not** part of this RFC.)_

By using these ABIs, it is possible to implement interrupt handlers directly in Rust, without writing any custom assembly code. This is not only safer and more convenient, it also often results in better performance. The reason for this is that the compiler can employ (cross-function) optimization techniques this way, for example to only backup the CPU registers that are actually used.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

- lowered to llvm
- functions should not be called by software
- functions are safe, but not fool-proof
- some calling conventions are only available for certain platforms
  - e.g.: `x86-interrupt`: no red-zone, no SSE?

<!--
This is the technical portion of the RFC. Explain the design in sufficient detail that:

- Its interaction with other features is clear.
- It is reasonably clear how the feature would be implemented.
- Corner cases are dissected by example.

The section should return to the examples given in the previous section, and explain more fully how the detailed proposal makes those examples work.
-->

# Drawbacks
[drawbacks]: #drawbacks

<!-- Why should we *not* do this? -->

- more work for alternative Rust compilers/code generators such as [`cranelift`](https://github.com/bytecodealliance/wasmtime/tree/main/cranelift), [`gccrs`](https://github.com/Rust-GCC/gccrs), or [`mrustc`](https://github.com/thepowersgang/mrustc)
- the implemenations in LLVM have been quite fragile in the past, i.e. there was repeated breakage
  - this might make it more difficult to update `rustc` to newer LLVM versions
  - leads to higher maintenance overhead

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- use a single `interrupt` ABI whose meaning changes depending on the target platform
  - pro: less ABI variants in the list
  - con: some targets have multiple interrupt calling conventions (e.g. avr and avr-non-blocking)
  - con: different targets often require different function signatures -> functions are not cross platform
  - con: interrupt handler implementations are often highly target specific -> there is not much value in cross-platform handlers anyway

## Use the `PreserveAll` Calling Convention

Many of the advantages of compiler-supported interrupt calling conventions come from the automated register handling, i.e. that all registers are restored to their previous state before returning. We might also be able achieve this by using LLVM's [`preserve_all`](https://llvm.org/docs/LangRef.html#calling-conventions) calling convention. While this calling convention is also marked as experimental, it is at least mentioned in the reference and is probably more stable. It is also relatively platform-independent and might have use cases outside of interrupt handling.

The remaining parts of interrupt calling conventions could then be implemented in small [naked](https://github.com/rust-lang/rust/issues/90957) wrapper function. These wrapper functions could also be provided by libraries using macros. Depending on the architecture, the wrapper functions would need to implement the following steps:

- stack alignment
- argument preprocessing
- calling the `extern "preserve_all" fn`
- stack cleanup
- interrupt return

**Open question:** Would it work as described?

<!--
- Why is this design the best in the space of possible designs?
- What other designs have been considered and what is the rationale for not choosing them?
- What is the impact of not doing this?
-->


# Prior art
[prior-art]: #prior-art

- some interrupt calling conventions are already implemented (see above)
- old RFC
- naked functions
- interrupt attribute in C
<!--
Discuss prior art, both the good and the bad, in relation to this proposal.
A few examples of what this can include are:

- For language, library, cargo, tools, and compiler proposals: Does this feature exist in other programming languages and what experience have their community had?
- For community proposals: Is this done by some other community and what were their experiences with it?
- For other teams: What lessons can we learn from what other communities have done here?
- Papers: Are there any published papers or great posts that discuss this? If you have some relevant papers to refer to, this can serve as a more detailed theoretical background.

This section is intended to encourage you as an author to think about the lessons from other languages, provide readers of your RFC with a fuller picture.
If there is no prior art, that is fine - your ideas are interesting to us whether they are brand new or if it is an adaptation from other languages.

Note that while precedent set by other languages is some motivation, it does not on its own motivate an RFC.
Please also take into consideration that rust sometimes intentionally diverges from common language features.
-->

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- What are the requirements for stabilizing an interrupt calling convention?

<!--
- What parts of the design do you expect to resolve through the RFC process before this gets merged?
- What parts of the design do you expect to resolve through the implementation of this feature before stabilization?
- What related issues do you consider out of scope for this RFC that could be addressed in the future independently of the solution that comes out of this RFC?
-->

# Future possibilities
[future-possibilities]: #future-possibilities

- support for more platforms

<!--
Think about what the natural extension and evolution of your proposal would
be and how it would affect the language and project as a whole in a holistic
way. Try to use this section as a tool to more fully consider all possible
interactions with the project and language in your proposal.
Also consider how this all fits into the roadmap for the project
and of the relevant sub-team.

This is also a good place to "dump ideas", if they are out of scope for the
RFC you are writing but otherwise related.

If you have tried and cannot think of any future possibilities,
you may simply state that you cannot think of anything.

Note that having something written down in the future-possibilities section
is not a reason to accept the current or a future RFC; such notes should be
in the section on motivation or rationale in this or subsequent RFCs.
The section merely provides additional information.
-->
