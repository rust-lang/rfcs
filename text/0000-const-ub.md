- Feature Name: `const_ub`
- Start Date: 2020-10-10
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Define how UB during const evaluation is treated:
some kinds of UB must be detected, the rest leads to an unspecified result for the affected CTFE query (but does not otherwise "taint" the compilation process).

# Motivation
[motivation]: #motivation

So far, nothing is specified about what happens when `unsafe` code leads to UB during CTFE.
This is a major blocker for stabilizing `unsafe` operations in const-contexts.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

There are some values that Rust needs to compute at compile-time.
This includes the initial value of a `const`/`static`, and array lengths (and more general, const generics).
Computing these initial values is called compile-time function evaluation (CTFE).
CTFE in Rust is very powerful and permits running almost arbitrary Rust code.
This begs the question, what happens when there is `unsafe` code and it causes [Undefined Behavior (UB)][UB]?

The answer depends on the kind of UB: some kinds of UB are guaranteed to be detected,
while other kinds of UB might either be detected, or else evaluation will continue as if the violated UB condition did not exist (i.e., as if this operation was actually defined).
This can change from compiler version to compiler version: CTFE code that causes UB could build fine with one compiler and fail to build with another.
(This is in accordance with the general policy that unsound code is not subject to strict stability guarantees.)

[UB]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The following kinds of UB are detected by CTFE, and will cause compilation to stop with an error:
* Incorrect use of compiler intrinsics (e.g., reaching an `unreachable` or violating the assumptions of `exact_div`).
* Dereferencing dangling pointers.
* Using an invalid value in an arithmetic, logical or control-flow operation.

These kinds of UB have in common that there is nothing sensible evaluation can do besides stopping with an error.

Other kinds of UB might or might not be detected:
* Dereferencing unaligned pointers.
* Violating Rust's aliasing rules.
* Producing an invalid value (but not using it in one of the ways defined above).
* Any [other UB][UB] not listed here.

All of this UB has in common that there is an "obvious" way to continue evaluation even though the program has caused UB:
we can just access the underlying memory despite alignment and/or aliasing rules being violated, and we can just ignore the existence of an invalid value as long as it is not used in some arithmetic, logical or control-flow operation.
There is no guarantee that CTFE detects such UB: evaluation may either fail with an error, or continue with the "obvious" result.

If the compile-time evaluation uses operations that are specified as non-deterministic,
and only some of the non-deterministic choices lead to CTFE-detected UB,
then CTFE may choose any possible execution and thus miss the possible UB.
For example, if we end up specifying the value of padding after a typed copy to be non-deterministically chosen, then padding will be initialized in some executions and uninitialized in others.
If the program then performs integer arithmetic on a padding byte, that might or might not be detected as UB, depending on the non-deterministic choice made by CTFE.

## Note to implementors

This requirement implies that CTFE must happen on code that was *not subject to UB-exploiting optimizations*.
In general, optimizations of Rust code may assume that the source program does not have UB, so programs that exhibit UB can simply be ignored when arguing for the correctness of an optimization.
However, this can lead to programs with UB being translated into programs without UB, so if constant evaluation runs after such an optimization, it might fail to detect the UB.
The only permissible optimizations are those that preserve all UB and that preserve the behavior of programs whose UB CTFE does not detect.
Formally speaking this means they must be correct optimizations for the abstract machine *that CTFE actually implements*, not just for the abstract machine that specifies Rust; and moreover they must preserve the location and kind of UB that is detected by CTFE.

# Drawbacks
[drawbacks]: #drawbacks

To be able to either detect UB or continue evaluation in a well-defined way, CTFE must run on unoptimized code.
This means when compiling a `const fn` in some crate, the unoptimized code needs to be stored.
So either the code is stored twice (optimized and unoptimized), or optimizations can only happen after all CTFE results have been computed.
[Experiments in rustc](https://perf.rust-lang.org/compare.html?start=35debd4c111610317346f46d791f32551d449bd8&end=3dbdd3b981f75f965ac04452739653a3d47ff0ed) showed a severe performance impact on CTFE stress-tests, but no impact on real code except for a slowdown of "incr-unchanged" (which are rather fast so small changes lead to large percentages).

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The most obvious alternative is to say that UB during CTFE will definitely be detected.
However, that is expensive and might even be impossible.
Even Miri does not currently detect all UB, and Miri is already performing many additional checks that would significantly slow down CTFE.
Furthermore, implementing these checks requires a more precise understanding of UB than we currently have; basically, this would block having any potentially-UB operations at const-time on having a spec for Rust that precisely describes their UB in a checkable way.
In particular, this would mean we need to decide on an aliasing model before permitting raw pointers in CTFE.

To avoid the need for keeping the unoptimized sources of `const fn` around, we could weaken the requirement for detecting UB and instead say that UB might cause arbitrary evaluation results.
Under the assumption that unsound code is not subject to the usual stability guarantees, this is an option we can still move to in the future, should it turn out that the proposal made in this RFC is too expensive.

Another extreme alternative would be to say that UB during CTFE may have arbitrary effects in the host compiler, including host-level UB.
Basically this would mean that CTFE would be allowed to "leave its sandbox".
This would allow JIT'ing CTFE and running the resulting code unchecked.
While compiling untrusted code should only be done with care (including additional sandboxing), this seems like an unnecessary extra footgun.

# Prior art
[prior-art]: #prior-art

C++ requires compilers to detect UB in `constexpr`.
However, the fragment of C++ that is available to `constexpr` excludes pointer casts, pointer arithmetic (beyond array bounds), and union-based type punning, which makes such checks not very complicated and avoids most of the poorly specified parts of UB.
The corresponding type-punning-free fragment of Rust (no raw pointers, no `union`, no `transmute`) can only cause UB that is defined UB to be definitely detected during CTFE.
In that sense, rust achieves feature parity with C++ in terms of UB detection during CTFE.
(Indeed, this was the prime motivation for making such strict UB detection requirements in the first place.)

# Unresolved questions
[unresolved-questions]: #unresolved-questions

Currently none.

# Future possibilities
[future-possibilities]: #future-possibilities

This RFC provides an easy way forward for "unconst" operations, i.e., operations that are safe at run-time but not at compile-time.
Primary examples of such operations are anything involving the integer representation of pointers, which cannot be known at compile-time.
If this RFC were accepted, we could declare such operations "definitely detected UB" during CTFE (and thus naturally they would only be permitted in an `unsafe` block).
