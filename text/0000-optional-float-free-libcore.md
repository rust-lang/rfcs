- Feature Name: cfg_target_has_floating_point
- Start Date: 2016-04-20
- RFC Issue: [rust-lang/rfcs#1364](https://github.com/rust-lang/rfcs/issues/1364)
- RFC PR: 
- Rust Issue: 
- Rust PR: [rust-lang/rust#32651](https://github.com/rust-lang/rust/pull/32651)

# Summary
[summary]: #summary

Add target `has_floating_point` property and disable floating-point instruction emission when compiling libcore if false.

# Motivation
[motivation]: #motivation

* Some processors, e.g. some ARM processors, lack hardware for floating-point operations.
* Many kernels, e.g. Linux, forbid floating-point. Saving the floating-point registers is costly, and if the kernel uses floating-point instructions it must do so at every interrupt and system call. As kernel code usually never needs floating-point, it makes sense to forbid it altogether to not pay the cost of saving the registers.

The modifications proposed in this RFC would enable writing code for such processors or for such kernels in Rust.

Even if floating-point features of libcore are not used, when it is built for a target with floating-point enabled, some non-floating-point operations may be emitted as MMX or SSE instructions on architectures which have them, so emitting such instructions must be disabled, but on some such architectures, e.g. amd64, the ABI specifies floating-point arguments passed in SSE registers, so floating-point must be disabled altogether.

# Detailed design
[design]: #detailed-design

Add an optional `has_floating_point` property, default true, gated as `cfg_target_has_floating_point`. Disable all floating-point use in libcore if this flag is false.

# Drawbacks
[drawbacks]: #drawbacks

This increases the complexity of the libcore code slightly.

# Alternatives
[alternatives]: #alternatives

* Delete all floating-point code from libcore
* Do nil, and let users who need this patch their own libcore
* Switch to soft-float rather than disable if the flag is false

# Unresolved questions
[unresolved]: #unresolved-questions

None so far
