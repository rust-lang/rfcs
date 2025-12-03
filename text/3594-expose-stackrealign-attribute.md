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

When the `realign_stack` attribute is applied to a function, the compiler no longer assumes the stack is properly aligned when the function is called, and so will insert code to forcefully realign the stack as needed for calling other functions, variables requiring alignment, etc.
This alignment is achieved by adjusting the stack pointer accordingly to the stack alignment specified in the target ABI's data layout.

In LLVM the `alignstack` gets an argument that specifies the alignment in bytes(also must be a power of 2). 
Below is an example of how it would work for an example data-layout:
`e-m:e-i64:64-f80:128-n8:16:32:64-S128`.
The `S128` is the part that describes the natural stack alignment in bits.
So practically, we just need to divide that value by 8, and place it as the argument of `alignstack`.
So in the `S128` case it would look like this: `alignstack=16`.

```
define i32 @callback_function() #0 {
start:
  ret i32 0
}

attributes #0 = { alignstack=16 }
```


# Drawbacks
[drawbacks]: #drawbacks
- Introducing a new attribute adds complexity to the language.
- Limited use cases: Stack realignment may not be necessary in most Rust codebases, potentially making this feature less relevant for many developers.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives
An alternative could be a macro workaround instead of adding the attribute.
However it would be more like band-aid than an actual solution as llvm exports this functionality anyways.

A different alternative could be adding the any extern "C" function the `stackrealign` attribute implicitly which would solve the main use-case. However, this is at the cost of added overhead where callers abide by the target's stack alignment, as in the majority of cases.
Also, this flag could be exported as an option in the Cargo.toml file so only projects that need it can use it and other projects will remain unaffected.
The downside for this is less control on which function has it's stack realigned.

An extra option could be not verifying data-layout for custom targets provided via `--target=`, which would allow users to patch the "natural stack alignment" in their custom target which should relax LLVM stack alignment assumptions that are present in the system.
Another alternative could be adding a new ABI that captures "function which can be called with any stack alignment".
I chose to propose this RFC and not any of the alternatives because it seems to me that this proposition provides the simplest solution, a solution that is very close to `force_align_arg_pointer` function attribute in GCC and a solution that is easy to implement for rustc.
While creating a different ABIs to handle stack realignment could be a viable alternative, introducing a new function attribute like realign_stack in Rust offers several advantages. Firstly, leveraging function attributes aligns with Rust's philosophy of providing clear and concise language features, ensuring that developers can easily understand and apply stack realignment where necessary. Also, if the realign_stack was a part of the ABI we would need to practically duplicate every ABI and create a copy where one has that attribute and the other does not. This would lead to a higher level of complexity and would require higher maintenance over time. 
Using a function attribute offers finer granularity and control, enabling developers to selectively apply stack realignment to specific functions without affecting the entire ABI.

In the future if it is seems import enough, we should reconsider the added ABI option which also has benifits of its own: stack alignment is really part of the ABI so it could be perhaps easier to new comers of the language to find this "realign_stack" feature there. New ABIs can be standardized later for other languages as well which would improve interoperability overall. 
The main thing we get with `realign_stack` function attribute as opposed to a new ABI is the simplicity of implementation and "closeness" of implementation to c with `force_align_arg_pointer`.

# Prior art
[prior-art]: #prior-art
This feature is present in GCC via the `force_align_arg_pointer` attribute.
Also present in LLVM in general.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

# Future possibilities
[future-possibilities]: #future-possibilities
- Explore additional LLVM features that could be exposed to Rust for performance optimization purposes.
- Add a rustc compilation flag that adds this attribute to every `pub extern "C"` function (similiar to `-mstackrealign` which does this globally in GCC).
- Similar to `extern "C-unwind"` and `#[unwind(allowed)]` we could add new ABIs (something like `extern "C-realign-stack"`) which would do the same as `#[realign_stack]`.