- Feature Name: `allow_extra_fp_precision`
- Start Date: 2019-04-08
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Update the Rust specification to allow floating-point operations to provide
*more* precision than specified, but not less precision; this allows many safe
optimizations.

# Motivation
[motivation]: #motivation

Some platforms provide instructions to run a series of floating-point
operations quickly, such as fused multiply-add instructions; using these
instructions can provide performance wins up to 2x or more. These instructions
may provide *more* precision than required by IEEE floating-point operations,
such as by doing multiple operations before rounding or losing precision.
Similarly, high-performance floating-point code could perform multiple
operations with higher-precision floating-point registers before converting
back to a lower-precision format.

In general, providing more precision than required will only bring a
calculation closer to the mathematically precise answer, never further away.

This RFC proposes allowing floating-point types to perform intermediate
calculations using more precision than the type itself, as long as they provide
*at least* as much precision as the IEEE 754 standard requires.

See the [prior art section](#prior-art) for precedent in several other
languages and compilers.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

After this RFC, we could explain floating-point operations as follows:

Floating-point operations in Rust have a guaranteed minimum accuracy, which
specifies how far the result may differ from an infinitely accurate,
mathematically exact answer. The implementation of Rust for any target platform
must provide at least that much accuracy. In some cases, Rust can perform
operations with higher accuracy than required, and doing so provides greater
performance (such as by removing intermediate rounding steps).

A note for users of other languages: this is *not* the equivalent of the "fast
math" option provided by some compilers. Unlike such options, this behavior
will never make any floating-point operation *less* accurate, but it can make
floating-point operations *more* accurate, making the result closer to the
mathematically exact answer.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Currently, Rust's [specification for floating-point
types](https://doc.rust-lang.org/reference/types/numeric.html#floating-point-types)
states that:
> The IEEE 754-2008 "binary32" and "binary64" floating-point types are f32 and f64, respectively.

This RFC proposes updating that definition as follows:

The `f32` and `f64` types represent the IEEE 754-2008 "binary32" and "binary64"
floating-point types. Operations on those types must provide no less
precision than the IEEE standard requires; such operations may provide *more*
precision than the standard requires, such as by doing a series of operations
with higher precision before storing a value of the desired precision.

rustc may provide a codegen (`-C`) option to disable this behavior, such as `-C
disable-extra-fp-precision`. Rust may also provide an attribute to disable this
behavior from within code, such as `#[disable_extra_fp_precision]`.

# Drawbacks
[drawbacks]: #drawbacks

If Rust already provided bit-for-bit identical floating-point computations
across platforms, this change could potentially allow floating-point
computations to differ by platform (though never below the standards-required
accuracy). However, standards-compliant implementations of math functions on
floating-point values may already vary slightly by platform, sufficiently so to
produce different binary results. This proposal can never make results *less*
accurate, it can only make results *more* accurate.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

We could provide a separate set of types and allow extra accuracy in their
operations; however, this would create ABI differences between floating-point
functions, and the longer, less-well-known types seem unlikely to see
widespread use.

We could provide an option to enable extra accuracy for the default
floating-point types, but disable it by default. This would leave the majority
of floating-point code unable to use these optimizations, however; defaults
matter, and the majority of code seems likely to use the defaults.

We could do nothing, and require code to use `a.mul_add(b, c)` for
optimization; however, this would not allow for similar future optimizations,
and would not allow code to easily enable this optimization without substantial
code changes.

We could narrow the scope of optimization opportunities to *only* include
floating-point contraction but not any other precision-increasing operations.
See the [future possibilities](#future-possibilities) section for further
discussion on this point.

# Prior art
[prior-art]: #prior-art

This has precedent in several other languages and compilers:

- [C11](http://www.open-std.org/jtc1/sc22/wg14/www/docs/n1570.pdf) allows
  this with the `STDC FP_CONTRACT` pragma enabled, and the default state
  of that pragma is implementation-defined. GCC enables this pragma by
  default, [as does the Microsoft C
  compiler](https://docs.microsoft.com/en-us/cpp/preprocessor/fp-contract?view=vs-2019).

- [The C++ standard](http://eel.is/c++draft/expr.pre#6) states that "The
  values of the floating operands and the results of floating
  expressions may be represented in greater precision and range than
  that required by the type; the types are not changed thereby."

- The [Fortran standard](https://www.fortran.com/F77_std/rjcnf0001-sh-6.html#sh-6.6.4)
  states that "the processor may evaluate any mathematically equivalent
  expression", where "Two arithmetic expressions are mathematically
  equivalent if, for all possible values of their primaries, their
  mathematical values are equal. However, mathematically equivalent
  arithmetic expressions may produce different computational results."

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Should we provide a rustc codegen option?
- Should we provide an attribute?

# Future possibilities
[future-possibilities]: #future-possibilities

The initial implementation of this RFC can simply enable floating-point
contraction within LLVM (and equivalent options in future codegen backends).
However, this RFC also allows other precision-increasing optimizations; in
particular, this RFC would allow the implementation of f32 or future f16
formats using higher-precision registers, without having to apply rounding
after each operation.
