- Feature Name: manyregs_abi
- Start Date: 18-10-2026
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary
[summary]: #summary

Add an ABI (provisional name: "manyregs") that passes arguments in registers as much
as possible.

## Motivation
[motivation]: #motivation

Most of the time, passing arguments in registers is significantly faster than passing
them by stack.

At the moment, most ABIs only allow a limited number of registers to be used for
passing or returning arguments - especially for vector code.

The idea of the `extern "Rust_manyregs"` ABI is to allow for as many arguments as
allowed by the architecture to be passed directly in registers. This would be
especially relevant in specialized SIMD DSP kernels, but such function calls could
provide subtle speedups in other locations too.

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

An `extern "Rust_manyregs"` function uses as many registers as possible for argument
passing. This can be faster than the standard Rust ABI in some cases.

Unwinding through a "manyregs" function causes termination.

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The specifics of `Rust_manyregs` are architecture dependent and somewhat dependant on
what LLVM would allow. It is an internal ABI, and thus no explicit guarantees are made
(especially across compiler versions).

As this ABI passes vectors by registers, it is subject to the same correctness checks
as the extern "C" ABI - in particular, it is an error to pass an argument that would
be passed by register if a target feature were present if the feature is not present.

## Drawbacks
[drawbacks]: #drawbacks

It is one more ABI. It would need some amount of LLVM support.

If the ABI actually uses "as many registers as possible", some ISA extensions
(i.e. avx512 on x86) can extend the number of registers available. Thus,
whether avx512 is enabled on the function can change the ABI, which can lead to
UB.


## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

See the "motivation" section.

The naming of the ABI string could definitely be improved.

I propose disallowing unwinding through such functions because they are usually
performance-critical and do not unwind, but we could allow unwinding or we could
add a `-unwind` variant.

We could have the Rust ABI behave this way. I believe this would break compatibility
in cases in which one passes by value register-like arguments to functions that don't
have the required target features.


## Prior art
[prior-art]: #prior-art

On x86, `x86_regcallcc` passes up to 16 integer / vector arguments via registers.

The proposed `rust_preserve_none_cc` ABI (https://github.com/rust-lang/rust/issues/151401)
is somewhat similar in spirit.

## Unresolved questions
[unresolved-questions]: #unresolved-questions

- What should we do with ISAs that extend the set of available vector registers?
  - Have a separate ABI string for those
  - Limit the register count to the minimum guaranteed available
  - Limit the register count to the minimum that the arguments guarantee is
    available (i.e. 32 if a _mm512 argument is present)
- How should the ABI behave in terms of callee-saved / caller-saved registers?
  - All registers are caller-saved
  - Some number of registers are callee-saved unless they are used as arguments
  - ???

## Future possibilities
[future-possibilities]: #future-possibilities

Does it make sense to have the "Rust" ABI behave in a similar way?
