- Feature Name: (`realign-stack-attr`)
- Start Date: (2024-03-26)
- RFC PR: [rust-lang/rfcs#3594](https://github.com/rust-lang/rfcs/pull/3594)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Expose the `stackrealign` of LLVM IR to rustc via a attribute.

# Motivation
[motivation]: #motivation
This is usefull when you have no guarantees about the alignment of the stack in an extern "C" function.
Also verbatim from the attribute reference:
Legacy x86 code uses 4-byte stack alignment. Newer aligned SSE instructions (like ‘movaps’) that work with the stack require operands to be 16-byte aligned. This attribute realigns the stack in the function prologue to make sure the stack can be used with SSE instructions.

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
The stack_realign attribute is specified as follows:
```
#[stack_realign]
```
or
```
#[stack_realign(align)]
```

Where align is an optional parameter representing the desired alignment boundary in bytes. If align is not provided, the compiler uses its default alignment behavior.

When the stack_realign attribute is applied to a function, the compiler ensures that the stack is aligned to the specified boundary before executing the function's body. This alignment is achieved by adjusting the stack pointer accordingly. The alignment is maintained for the duration of the function's execution.

If the align parameter is provided, the compiler adjusts the stack alignment to the nearest multiple of align bytes.

align must be a power of 2.

# Drawbacks
[drawbacks]: #drawbacks
Introducing a new attribute adds complexity to the language.

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
