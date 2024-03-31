- Feature Name: (`realign-stack-attr`)
- Start Date: (2024-03-26)
- RFC PR: [rust-lang/rfcs#3594](https://github.com/rust-lang/rfcs/pull/3594)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Expose the `alignstack` function attribute to rustc.

# Motivation
[motivation]: #motivation
This is usefull when you have no guarantees about the alignment of the stack in an extern "C" function.
Mainly happens when there is an ABI incompatibility and the "contract" between the caller and the callee is broken.

Interrupt service routines (ISRs) often require special treatment regarding stack alignment. When an interrupt occurs, the processor saves the current execution context onto the stack before transferring control to the ISR. However, the stack might not be aligned to the required boundary, especially in embedded systems where memory constraints are tight.

By exposing a stack realignment feature to Rust, developers working on embedded systems or performance-critical applications gain the ability to guarantee stack alignment within ISRs and other critical code paths directly from Rust code. This not only simplifies development but also enhances the reliability and performance of systems that rely on interrupt handling.

In the example above the "contract" is defined by hardware and very hard\impossible to work around. 

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation
The `[realign_stack]` attribute can be added to a function to force the compiler to add alignment to that function.
Usefull in cases where your code is called from a thread or a binary compiled with another compiler, that uses different aligmnet and thus lead to a corruption.

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

When the `realign_stack` attribute is applied to a function, the compiler ensures that the stack is aligned to the "natural alligmnet" specified in the Data Layout that LLVM uses before executing the function's body. 
This alignment is achieved by adjusting the stack pointer accordingly. The alignment is maintained for the duration of the function's execution.


# Drawbacks
[drawbacks]: #drawbacks
- Introducing a new attribute adds complexity to the language.
- Limited use cases: Stack realignment may not be necessary in most Rust codebases, potentially making this feature less relevant for many developers.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives
An alternative could be a macro workaround instead of adding the attribute.
However it would be more like band-aid than an actual solution.
Another alternative could be adding the any extern "C" function the `stackrealign` attribute implicitly which would solve the main use-case.

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
