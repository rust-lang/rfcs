- Feature Name: `simple_c_panic_abi`
- Start Date: 2019-08-29
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Provides a new ABI string `extern "C+panic"` to denote functions that use the C ABI, but may also
unwind with a Rust panic.

# Motivation
[motivation]: #motivation

TODO

- soundness & optimization
- libjpeg/mozjpeg

## Generated code

When working with generated code, such as the result of a JIT compiler, calls to generated code
frequently must use the C ABI. Similarly to the `mozjpeg` case, we want it to be possible for panics
to unwind across calls to foreign code. However the dynamic nature of the foreign code distinguishes
this case from the `mozjpeg` case, specifically because the set of foreign functions that may be
called is not known at Rust compile time.

This property makes wrappers that use `setjmp`/`longjmp` or C++ exceptions much more difficult to
generate, as we have no header files to provide to a wrapper generator like
[`ffi_wrapper_nounwind`][ffi_wrapper_nounwind]. It further means that a solution must accommodate
foreign function pointers that don't have an `extern "C" { ... }` declaration where we can attach an
attribute.

[Lucet][lucet] and [Weld][weld] are two projects with these FFI use patterns that would concretely
benefit from permitting unwinding through FFI boundaries.

[ffi_wrapper_nounwind]: https://docs.rs/ffi_wrapper_nounwind
[lucet]: https://github.com/fastly/lucet
[weld]: https://www.weld.rs/

## Less dependence on C/C++

Some of the [proposed workarounds][cffi-panic] to the lack of cross-FFI unwinding require the use of
wrappers written in C (for `setjmp` and `longjmp`) or C++ (for exceptions). This solution reduces
the amount of non-Rust code that must be generated and maintained when the foreign code is
compatible with the unspecified Rust unwinding mechanism.

[cffi-panic]: https://github.com/gnzlbg/cffi-panic

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Rust function definitions with the `"C+panic"` ABI string (e.g., `extern "C+panic" fn`) are
permitted to unwind with a panic, as opposed to `extern "C" fn` functions which will abort the
process if a panic reaches the function boundary.

When used on declarations of imported functions (e.g., `extern "C+panic" { fn ... }`), or function
pointers (e.g., `extern "C+panic" fn()`) the `"C+panic"` ABI string means that if the function
unwinds, the unwind will be propagated though any calling code. If an `extern "C"` imported function
or function pointer unwinds, the behavior is undefined.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Currently, Rust assumes that an external function imported with `extern "C" {
... }` (or another ABI other than `"Rust"`) cannot unwind, and Rust will abort if a panic would propagate out of a
Rust function with a non-"Rust" ABI ("`extern "ABI" fn`") specification. Under this RFC,
functions with the `"C+panic"` ABI string
instead allows an unwind (such as a panic) to proceed through that function
boundary using Rust's normal unwind mechanism. This may potentially allow Rust
code to call non-Rust code that calls back into Rust code, and then allow a
panic to propagate from Rust to Rust across the non-Rust code.

The Rust unwind mechanism is intentionally not specified here. Catching a Rust
panic in Rust code compiled with a different Rust toolchain or options is not
specified. Catching a Rust panic in another language or vice versa is not
specified. The Rust unwind may or may not run non-Rust destructors as it
unwinds. Propagating a Rust panic through non-Rust code is unspecified;
implementations that define the behavior may require target-specific options
for the non-Rust code, or this feature may not be supported at all.

For the purposes of the type system, `"C+panic"` is considered a totally distinct ABI string from
`"C"`. While there may be some circumstances where it is sensible to use an `extern "C" fn` in place
of an `extern "C+panic" fn`, and vice-versa, this introduces questions of subtyping and variance
that are beyond the scope of this RFC. This restrictive approach is forwards-compatible with more
permissive typing in future work like #2699.

# Drawbacks
[drawbacks]: #drawbacks

- Only works as long as the foreign code supports the same unwinding mechanism as Rust. (Currently, Rust and C++ code compiled for ABI-compatible backends use the same mechanism.)

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The goal is to enable the current use-cases for unwinding panics through foreign code while providing safer behaviour towards bindings not expecting that, with minimal changes to the language.

We should try not to prevent the RFC 2699 or other later RFC from resolving the above disadvantages on top of this one.

- https://github.com/rust-lang/rfcs/pull/2699

The alternatives considered are:

1. Stabilize the [abort-on-FFI-boundary behavior](https://github.com/rust-lang/rust/issues/52652)
   without providing any mechanism to change this behavior. As a workaround for applications that would
   otherwise need this unwinding, we would recommend the use of wrappers to translate Rust panics to
   and from C ABI-compatible values at FFI boundaries, with a foreign control mechanism like
   `setjmp`/`longjmp` or C++ exceptions to skip or unwind segments of foreign stack.

   While using these types of wrappers will likely continue to be a recommendation for
   maximally-compatible code, it comes with a number of downsides:

   - Additional non-Rust code must be maintained.
   - `setjmp`/`longjmp` incur runtime overhead even when no unwinding is required, whereas many
     unwinding mechanisms incur runtime overhead only when unwinding.
   - Wrappers that must be present at Rust compile time are not suitable for applications with
     dynamic, generated code.

   If an application has enough control over its compilers and runtime environment to be assured
   that its foreign components are compatible with the unspecified Rust unwinding mechanism, these
   downsides can be avoided by allowing unwinding across FFI boundaries.

2. Address unwinding more thoroughly, perhaps through the introduction of additional `extern "ABI"`
   strings. This is the approach being pursued in #2699, and is widely seen as a better long-term
   solution than adding a single attribute. However, work in #2699 has stalled due to a number of
   thorny questions that will likely take significant time and effort to resolve, such as:

   - What should the syntax be for unwind-capable ABIs?
   - What are the type system implications for new ABI strings?
   - How should the semantics of an unwind-capable ABI be defined across different platforms?

   In the meantime, we are caught between wanting to fix the soundness bug in Rust, and not wanting
   to disrupt current development on a number of projects that depend on unwinding. Adding an unwind
   attribute means that we can address those current needs right away, and then transition to
   #2699's eventual solution by converting the attribute into a deprecated proc-macro.

3. Using an attribute on function definitions and declarations to indicate that unwinding should be
   allowed, regardless of the ABI string. This would be easy to implement, as there is currently
   such an attribute in unstable Rust. An attribute is not a complete solution, though, as there is
   no current way to syntactically attach an attribute to a function pointer type (see
   https://github.com/rust-lang/rfcs/pull/2602). We considered making all function pointers
   unwindable without changing the existing syntax, because Rust currently does not emit `nounwind`
   for calls to function pointers, however this would require changes to the language reference that
   would codify inconsistency between function pointers and definitions/declarations.

# Prior art
[prior-art]: #prior-art

TODO

# Unresolved questions
[unresolved-questions]: #unresolved-questions

TODO

# Future possibilities
[future-possibilities]: #future-possibilities

TODO

- https://github.com/rust-lang/rfcs/pull/2699

- `unwind(abort)`
- non-"C" ABIs
