- Feature Name: cfg_target_has_floating_point
- Start Date: 2016-04-20
- RFC Issue: [rust-lang/rfcs#1364](https://github.com/rust-lang/rfcs/issues/1364)
- RFC PR: 
- Rust Issue: 
- Rust PR: [rust-lang/rust#32651](https://github.com/rust-lang/rust/pull/32651)

# Summary
[summary]: #summary

Add `has_floating_point` property of target in target spec and disable floating-point parts of libcore if false.

# Motivation
[motivation]: #motivation

* Some processors, e.g. some ARM processors, lack hardware for floating-point operations.
* Many kernels, e.g. Linux, forbid floating-point. Saving the floating-point registers is costly, and if the kernel uses floating-point instructions it must do so at every interrupt and system call. As kernel code usually never needs floating-point, it makes sense to forbid it altogether to not pay the cost of saving the registers.

The modifications proposed in this RFC would enable writing code for such processors or for such kernels in Rust.

Even if floating-point features of libcore are not used, when it is built for a target with floating-point enabled, some non-floating-point operations may be emitted as MMX or SSE instructions on architectures which have them, so emitting such instructions must be disabled, but on some such architectures, e.g. amd64, the ABI specifies floating-point arguments passed in SSE registers, so floating-point must be disabled altogether.

# Detailed design
[design]: #detailed-design

Add an optional `has_floating_point` property in target spec, default true, gated as `cfg_target_has_floating_point`. Add a `cfg` flag `target_has_floating_point` which has the same value as the target property. Add `#[cfg(target_has_floating_point)]` attribute to all items of libcore involving floating-point.

# Drawbacks
[drawbacks]: #drawbacks

* This increases the complexity of the libcore code slightly.
* "Conditionally removing parts of core is not great!" - brson
* Crates would potentially and surprisingly fail in `not(target_has_floating_point)` environment.

# Alternatives
[alternatives]: #alternatives

* Move all float-free code to another crate and re-export it from core
* Do nil, and let users who need this patch their own libcore
* Switch to soft-float rather than disable if the flag is false

# Unresolved questions
[unresolved]: #unresolved-questions

* Will this affect code generation, or will that be left to the `features` flag?
* If `has_floating_point` is false, is it legal to use `f32` and `f64`?
* Would any other global target properties have this pattern?
