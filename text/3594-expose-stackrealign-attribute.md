- Feature Name: (`realign-stack-attr`)
- Start Date: (2024-03-26)
- RFC PR: [rust-lang/rfcs#3594](https://github.com/rust-lang/rfcs/pull/3594)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary


Provide a way to generate functions that are "robust" to being called on a misaligned stack.


# Motivation
[motivation]: #motivation

There are situations where functions will be called with a lower stack alignment than what is typically expected on the current target:

- Interrupt service routines (ISRs) being called by the CPU. When an interrupt occurs, the processor saves the current execution context onto the stack before transferring control to the ISR. However, the stack might not be aligned to the required boundary, especially in embedded systems where memory constraints are tight.
- Interacting with legacy code that was built using a compiler flag that reduces the stack alignment, such as `-mpreferred-stack-boundary` on GCC. This flag says in the documentation "It is recommended that libraries that use callbacks always use the default setting", but not all libraries heed this advice. To make it possible to link those libraries with Rust code, the Rust functions they call must be "robust" to being called on a stack that was not properly aligned.

By exposing a stack realignment feature to Rust, developers working on embedded systems or performance-critical applications gain the ability to guarantee stack alignment within ISRs and other critical code paths directly from Rust code. This not only simplifies development but also enhances the reliability and performance of systems that rely on interrupt handling.

In the example above the "contract" is defined by hardware and very hard\impossible to work around. 

The proposed attribute will tell the compiler that the precondition that the stack is aligned might not be true, and that it might need to fix it up in order to execute the function correctly.


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation
The `[realign_stack]` attribute can be added to a function to tell the compiler to add stack re-alignment to that function if necessary.
Useful in cases where your code is called from a thread or a binary compiled with another compiler, that uses different alignment and thus lead to a corruption.
An example of one such setting could be `-mpreferred-stack-boundary=2` in GCC which would set the stack alignment to 4 instead of the default value for the ABI which is 16.
Other such settings could be present at GCC's "Machine-Dependent Options", which there are many of, and many of them can break ABI compatibility.

```
#[realign_stack]
#[no_mangle]
pub extern "C" fn callback_function() -> i32 {
    println!("Called from callback!!");

    0
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation
The realign_stack attribute is specified as follows:
```
#[realign_stack]
```

When the `realign_stack` attribute is applied to a function, the compiler no longer assumes the stack is properly aligned when the function is called, so will insert code to align the stack as needed for calling other functions, variables requiring alignment, etc.
This alignment is achieved by adjusting the stack pointer accordingly. The alignment is maintained for the duration of the function's execution.
Adding this attribute unnecessarily might "waste" space on the stack which could be crucial in real-time systems.

# Drawbacks
[drawbacks]: #drawbacks
- Introducing a new attribute adds complexity to the language.
- Limited use cases: Stack realignment may not be necessary in most Rust codebases, potentially making this feature less relevant for many developers.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives
An alternative could be a macro workaround instead of adding the attribute.
However it would be more like band-aid than an actual solution.
Another alternative could be adding the any extern "C" function the `stackrealign` attribute implicitly which would solve the main use-case.
An extra option could be not verifying data-layout for custom targets provided via `--target=`, which would allow users to patch the "natural stack alignment" in their custom target which should relax LLVM stack alignment assumptions that are present in the system.

# Prior art
[prior-art]: #prior-art
This feature is present in GCC via the `force_align_arg_pointer` attribute.
Also present in LLVM in general.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

# Future possibilities
[future-possibilities]: #future-possibilities
- Explore additional LLVM features that could be exposed to Rust for performance optimization purposes.
- We could perhaps add a new ABI called something like `"C-unaligned"` which could inform LLVM of the problems specified above.
- Add a rustc complication flag that adds this attribute to every `pub extern` function (similiar to `-mstackrealign` which does this globally in GCC).