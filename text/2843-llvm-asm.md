- Feature Name: `llvm_asm`
- Start Date: 2019-12-31
- RFC PR: [rust-lang/rfcs#2843](https://github.com/rust-lang/rfcs/pull/2843)
- Rust Issue: [rust-lang/rust#70173](https://github.com/rust-lang/rust/issues/70173)

# Summary
[summary]: #summary

Deprecate the existing `asm!` macro and provide an identical one called
`llvm_asm!`. The feature gate is also renamed from `asm` to `llvm_asm`.

Unlike `asm!`, `llvm_asm!` is not intended to ever become stable.

# Motivation
[motivation]: #motivation

This change frees up the `asm!` macro so that it can be used for the new
`asm!` macro designed by the inline asm project group while giving existing
users of `asm!` an easy way to keep their code working.

It may also be useful to have an inline asm implementation available
(on nightly) for architectures that the new `asm!` macro does not support yet.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The Rust team is currently in the process of redesigning the `asm!` macro.
You should replace all uses of `asm!` with `llvm_asm!` in your code to avoid breakage when the new `asm!` macro is implemented.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

All references to `asm!` inside the compiler will be changed to refer to `llvm_asm!` instead.
`asm!` will become a simple (deprecated) `macro_rules!` which redirects to `llvm_asm!`.
The deprecation warning will advise users that the semantics of `asm!` will change in the future and invite them to use `llvm_asm!` instead. The `llvm_asm!` macro will be guarded by the `llvm_asm` feature gate.

# Drawbacks
[drawbacks]: #drawbacks

This change may require people to change their code twice: first to `llvm_asm!`, and then to the new
`asm!` macro once it is implemented.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

We could skip the deprecation period and perform the renaming at the same time the new `asm!` macro
is implemented. However this is guaranteed to break a lot of code using nightly Rust at once without
any transition period.

# Prior art
[prior-art]: #prior-art

The D programming language also support 2 forms of inline assembly. The [first one][d-asm] provides an embedded DSL
for inline assembly, which allows direct access to variables in scope and does not require the use of clobbers, but is only available on x86 and x86_64. The [second one][d-llvm-asm] is a raw interface to LLVM's internal inline assembly syntax, which is available on all architectures but only on the LDC backend.

[d-asm]: https://dlang.org/spec/iasm.html
[d-llvm-asm]: https://wiki.dlang.org/LDC_inline_assembly_expressions

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None

# Future possibilities
[future-possibilities]: #future-possibilities

When the [new `asm!` macro][inline-asm-rfc] is implemented it will replace the current one. This
will break anyone who has not yet transitioned their code to `llvm_asm!`. No
silent miscompilations are expected since the operand separator will be changed
from `:` to `,`, which will guarantee that any existing `asm!` invocations will
fail with a syntax error with the new `asm!` macro.

[inline-asm-rfc]: https://github.com/rust-lang/rfcs/pull/2873
