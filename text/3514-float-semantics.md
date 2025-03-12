- Feature Name: `float_semantics`
- Start Date: 2023-10-14
- RFC PR: [rust-lang/rfcs#3514](https://github.com/rust-lang/rfcs/pull/3514)
- Tracking Issue: [rust-lang/rust#128288](https://github.com/rust-lang/rust/issues/128288)

# Summary
[summary]: #summary

This RFC proposes a specification for how our floating point operations are expected to behave.
The current implementation in rustc already matches the specification, so after accepting the RFC no compiler changes are required.
(However we might be able to stabilize some `const fn` features, see below for details.)

Rust's floating point operations follow IEEE 754-2008 -- with some caveats around operations producing NaNs: IEEE makes almost no guarantees about the sign and payload bits of the NaN; however, actual hardware does not pick those bits completely arbitrarily, and Rust will expose some of those hardware-provided guarantees to programmers.
On the flip side, NaN generation is non-deterministic: running the same operation on the same inputs several times can produce different results.
And there is a caveat: while IEEE specifies that float operations can never output a signaling NaN, Rust float operations *can* produce signaling NaNs, *but only if* an input is signaling.
That means the only way to ever see a signaling NaN in a program is to create one with `from_bits` (or equivalent unsafe operations).

Floating-point operations at compile-time follow the same specification. In particular, since operations involving NaN bit patterns are non-deterministic, the same operation can lead to different NaN bit patterns when executed at compile-time (in a `const` context) vs at run-time.
Of course, the compile-time interpreter is still deterministic. It is entirely possible to implement a non-deterministic language on a deterministic machine, by simply making some fixed choices. However, we will not specify a particular choice, and we will not guarantee it to remain the same in the future.

# Motivation
[motivation]: #motivation

We have a plethora of open issues that boil down to "is this sequence of float operations allowed to produce the given result".
This is caused by a combination of surprising effects introduced by LLVM optimizations, bugs in MIR and LLVM optimizations, and bugs (or at least non-conformance) in certain targets. See [here](https://github.com/rust-lang/unsafe-code-guidelines/issues/237) for a collection of issues.

It's time to stop leaving our users in the dark about what actually is and is not guaranteed.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Primitive operations on floating-point types generally produce results that exactly match IEEE 754-2008:
if you never use `to_bits`, `copysign`, `is_sign_negative`, or `is_sign_positive` on a NaN, and don't construct a NaN using `from_bits`, nor use any unsafe operations that are equivalent to these safe methods, then your code will not be able to observe any non-determinism and behave according to the IEEE specification.

If you *do* use these operations on NaNs, then the exact behavior you see can depend on compiler version, compiler flags, target architecture, and it can even be non-deterministic (i.e., running the same operation on the same inputs twice can yield different results).
The results produced in these cases do *not* always conform to the IEEE specification.
See the reference section for what exactly is guaranteed.

When a floating-point value is just passed around, its contents (including the bits of a NaN) do *not* change.

When you use a floating-point operation in [`const` context](https://doc.rust-lang.org/reference/const_eval.html#const-context), the same specification applies: NaN bit patterns are non-deterministic.
In particular, the bit pattern produced at compile-time can differ from the bit pattern produced by the same operation at run-time.

Certain targets unfortunately are known to not implement these semantics precisely (see [below](#target-specific-problems)).
The [platform support page](https://doc.rust-lang.org/rustc/platform-support.html) will list those caveats.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This RFC specifies the behavior of `+`, `-` (unary and binary), `*`, `/`, `%`, `abs`, `copysign`, `mul_add`, `sqrt`, `as`-casts that involve floating-point types, and all comparison operations on floating-point types.
Here, "floating-point types" are `f32` and `f64` and all similar types that might be added in the future such as `f16`, `f128`.
Except for the cases handled below, these operations produce results that exactly match IEEE 754-2008 (with roundTiesToEven [except for float-to-int casts, which round towards zero] and default exception handling without traps, without abruptUnderflow/flush-to-zero).
`%` matches the behavior of `fmod` in C (this operation is not in the IEEE spec).
When a floating-point value is just passed around (i.e., outside the operations above or any library-provided float operation), its representation bits do *not* change.

Exceptions apply when the output of an operation is a NaN, and the operation is not a "bitwise" operation (unary `-`, `abs`, `copysign`).
In that case, we generally follow [the same rules as LLVM](https://llvm.org/docs/LangRef.html#behavior-of-floating-point-nan-values), which differ from the IEEE specification.
Furthermore, Rust considers floating-point status bits to not be observable, and Rust does not support executing floating-point operations with alternative rounding modes or otherwise changed floating-point control bits.

To be concrete, we first establish some terminology:
A floating-point NaN value consists of a sign bit, a quiet/signaling bit, and a payload (which makes up the rest of the significand (i.e., the mantissa) except for the quiet/signaling bit). Rust assumes that the quiet/signaling bit being set to ``1`` indicates a quiet NaN (QNaN), and a value of ``0`` indicates a signaling NaN (SNaN). In the following we will hence just call it the "quiet bit".

For the operations listed above, the following rules apply when a NaN value is returned:
the result has a non-deterministic sign; the quiet bit and payload are non-deterministically chosen from the following set of options:

- **Preferred NaN**: The quiet bit is set and the payload is all-zero.
- **Quieting NaN propagation**: The quiet bit is set and the payload is copied from any input operand that is a NaN.
  If the inputs and outputs do not have the same payload size (i.e., for `as` casts), then
  - If the output is smaller than the input, low-order bits of the payload get dropped.
  - If the output is larger than the input, the payload gets filled up with 0s in the low-order bits.
- **Unchanged NaN propagation**: The quiet bit and payload are copied from any input operand that is a NaN.
  If the inputs and outputs do not have the same size (i.e., for `as` casts), the same rules as for "quieting NaN propagation" apply, with one caveat: if the output is smaller than the input, droppig the low-order bits may result in a payload of 0; a payload of 0 is not possible with a signaling NaN (the all-0 significand encodes an infinity) so unchanged NaN propagation cannot occur with some inputs.
- **Target-specific NaN**: The quiet bit is set and the payload is picked from a target-specific set of
  "extra" possible NaN payloads. The set can depend on the input operand values.
  This set is empty on x86, ARM, and RISC-V (32bit and 64bit), but can be non-empty on other architectures. Targets where this set is non-empty should document this in a suitable location, e.g. their platform support page.
  (For instance, on wasm, if any input NaN does not have the preferred all-zero   payload or any input NaN is an SNaN, then this set contains all possible payloads; otherwise, it is empty. On SPARC, this set consists of the all-one payload.)

In particular, if all input NaNs are quiet (or if there are no input NaNs), then
the output NaN is definitely quiet. Signaling NaN outputs can only occur if they
are provided as an input value. For example, "fmul SNaN, 1.0" may be simplified
to SNaN rather than QNaN. Similarly, if all input NaNs are preferred (or if
there are no input NaNs) and the target does not have any "extra" NaN payloads,
then the output NaN is guaranteed to be preferred.

The non-deterministic choice happens when the operation is executed; i.e., the result of a NaN-producing floating point operation is a stable bit pattern (looking at these bits multiple times will yield consistent results), but running the same operation twice with the same inputs can produce different results.

Unless noted otherwise, the same rules also apply to NaNs returned by other library functions (e.g. `min`, `minimum`, `max`, `maximum`); other aspects of their semantics and which IEEE 754-2008 operation they correspond to are documented with the respective functions.

### `const` semantics

Evaluation of `const` items (and other entry points to CTFE) must necessarily be deterministic to ensure soundness of the type system.
`const` use of floating points does not make any guarantees beyond that:
when a floating-point operation produces a NaN result, the resulting NaN bit pattern is *some* deterministic function of the operation's inputs that satisfies the constraints placed on run-time floating point semantics.
However, the exact function is not specified, and it is allowed to change across targets and Rust versions, and even with compiler flags.
In particular, there is no guarantee that the choice made in const evaluation is consistent with the choice made at runtime.
That is, the following assertion is allowed to fail (and in fact, it [fails on current versions of Rust](https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=a594d2975c29b1c7fa457a4ec4ae4b87)):

```rust
use std::hint::black_box;

const C: f32 = 0.0 / 0.0;

fn main() {
    let c: f32 = 0.0 / black_box(0.0);
    assert_eq!(C.to_bits(), c.to_bits());
}
```

This means that evaluating the same `const fn` on the same arguments can produce different results at compile-time and run-time.
However, note that these functions are already non-deterministic: even evaluating the same function with the same arguments twice at runtime can [and does](https://play.rust-lang.org/?version=stable&mode=release&edition=2021&gist=50b5a549fa1fe259cea5ad138066ccf0) produce different results!

In other words, consider this code:

```rust
const fn div(x: f32) -> i32 {
    unsafe { std::mem::transmute(x / x) }
}

// This is not guaranteed to always succeed. That's new, currently
// all `const fn` you can write guarantee this.
assert_eq!(div(0.0), div(0.0));
// Consequently, different results can be observed at compile-time and at run-time.
const C: i32 = div(0.0);
assert_eq!(C, div(0.0));
```

The first assertion is very unlikely to fail in practice (it would require the two invocations of `div` to be optimized differently).
The second however [actually fails](https://play.rust-lang.org/?version=nightly&mode=debug&edition=2021&gist=0b4b952929c9ebcd2bd50aee54e6cdf4) on current nightlies in debug mode.

Even running the same expression (such as `0.0 / 0.0`) twice as part of evaluating a given `const` item, or in two different `const` items, may produce different results.
The only guarantee the type system needs is that evaluating `some_crate::SOME_CONST` will produce consistent results if evaluation is repeated in different compilation units, and so that is all we guarantee.

This resolves the last open question blocking floating-point operations in `const fn`.
When the RFC is accepted, the `const_fn_floating_point_arithmetic` feature gate and the `const fn to_bits` methods can be stabilized.

### Assumptions about floating-point environment

This RFC is primarily concerned with the guarantee Rust provides to its users.
How exactly those guarantees are achieved is an implementation detail, and not observable when writing pure Rust code.
However, when mixing Rust with inline assembly, those details *do* become observable.
To ensure that Rust can provide the above guarantees to user code, it is UB for inline assembly to alter the behavior of floating-point operations in any way: when leaving the inline assembly block, the floating-point control bits must be in exactly the same state as when the inline assembly block was entered.
This is just an instance of the general principle that it is UB for inline assembly to violate any of the invariants that the Rust compiler relies on when implementing Rust semantics on the target hardware.
Furthermore, observing the floating-point exception state yields entirely unspecified results: Rust floating-point operations may or may not be executed at the place in the code where they were originally written, and the floating-point status bits can change even if no floating-point operation exists in the source code.

(This is very similar to C without `#pragma STD FENV_ACCESS`.)

**Debugging floating-point computations by trapping on NaN generation.**
One way that people make use of floating-point control bits is for debugging: by enabling a trap on NaN generation, the program can be aborted or a debugger can be triggered when a float operation generates a NaN.
This is UB under the wording above.
The reason for this is that compiler transformations can and will change when and where that trap is triggered:
for instance, the optimizer may move a float operation `a / b` out of a loop without proving that the loop ever executes, and if this operation turns out to produce a NaN, the program would now trap even though according to Rust source semantics, there wasn't even any floating-point operation being executed.
That is the sense in which the compiler relies on floating-point operations to never trap, and the sense in which violating that assumption is UB.
However, for all the RFC author knows, this is currently the worst possible consequence that this particular UB can have: the trap might trigger even when there was no float operation in the original program, and the trap might *not* trigger when though there was a NaN generated at some point (due to optimizations moving or removing this operation).
Programmers that use trap-on-NaN as a debugging technique can still use this technique as long as they are aware of these caveats.
That said, this is not a stable guarantee, and it is hard to figure out how exactly a stable guaranteed could be worded.

# Drawbacks
[drawbacks]: #drawbacks

- This RFC is too restrictive for some targets and too vague for some users: it is too vague since NaN signs are still left completely non-deterministic, which is not actually the case on hardware.
  It is also too restrictive as shown by the following subsection.
- It [looks](https://reviews.freebsd.org/D33599) like some targets, such FreeBSD, leave the floating-point environment in signal handlers in an unspecified state.
  This is something that [Linux specifically avoided](https://yarchive.net/comp/linux/fp_state_save.html) since it breaks e.g. the glibc `memcpy` implementation (which uses SSE registers if available).
  Rust (just like C without `#pragma STD FENV_ACCESS`) cannot currently be used to write signal handlers on such targets.
  There is little we can do here with the current state of LLVM; and even once LLVM provides the necessary features, these signal handlers will need annotations in the code that tell the compiler about the non-default floating point state.
  Those targets chose to use a semantics that is hard to support well in a highly optimized language, and there's not much we can do to paper over such target quirks.
  We can only hope that eventually those targets will decide to provide a reliable floating-point environment.
  Meanwhile, the best work-around is to use inline assembly to change the floating-point environment to the state that rustc expects it to be in.
  While strictly speaking that would have to happen before any Rust function gets called, practically speaking if this is the first thing the signal handler does, it is very unlikely to cause a problem:
  that would require the compiler to put a floating-point operation between the function start and the inline assembly block.

### Target-specific problems

Certain targets are known to not properly implement this specification for reasons that are deeply rooted in platform capabilities or ABI, and hence unlikely to ever be completely fixed.
These are bugs in the Rust implementation for those targets.
We should consider documenting on the "platform support" page (and we probably want to have one issue tracking each of these points):
- On 32bit x86 (with and without SSE), return values of float type are passed via the x87 registers, altering NaN payloads. This means that looking at the bit pattern of a float before and after the return can produce different results, violating the guarantee that NaN payloads are stable bit patterns once they have been produced. [Tracking issue](https://github.com/rust-lang/rust/issues/115567)
- On 32bit x86 without SSE2 (i586 targets), x87 registers are used even more pervasively, leading to more opportunities for unstable bit patterns. Furthermore, operations are internally computed with a different precision, which can lead to results that differ from IEEE 754-2008 even outside of NaNs. [Tracking issue](https://github.com/rust-lang/rust/issues/114479)
- On old MIPS, the interpretation of "signaling" and "quiet" is the opposite of what has been specified above. The effective spec on those targets is that any NaN-producing operation can non-deterministically produce an arbitrary (signaling or quiet) NaN. Currently, LLVM does not have a way of implementing their own NaN semantics for this target, so there's not a lot we can do on the Rust side. [LLVM issue](https://github.com/llvm/llvm-project/issues/60796)
- On 32bit ARM, NEON SIMD operations [always flush-to-zero](https://rust-lang.zulipchat.com/#narrow/stream/213817-t-lang/topic/Pre-RFC.3A.20floating.20point.20guarantees/near/376893307). *If* LLVM auto-vectorizes code for that target, that would lead to divergence from IEEE semantics. It is currently unclear whether this is the case; people keep bringing this up as a cause of potential non-conformance but the author was unable to find concrete records of any actual misbehavior. However, this will become an issue if NEON operations are ever exposed to Rust users: we would expect SIMD operations to follow the same NaN rules as their non-SIMD counterparts, but ARM NEON would violate those semantics.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Alternative options for run-time semantics

### Alternative: a spec that actually works for all targets, or at least all tier 1 targets

To achieve this we would at least have to make the i686 targets compliant, i.e., we'd have to say that NaN bit patterns are allowed to change when a floating-point value is returned from a function.
That is certainly not a sane semantics, and not something we want to impose on targets that are not aflicted by x87.

Our tier 1 i686 targets with `-C target-cpu=pentium` also suffer from the "no SSE2" issue;
if we want that to be spec-compliant in its current form, we can make basically no statement at all about the behavior of `f64` operations.

The most widely used targets (64bit x86 and 64bit ARM, and also the increasingly popular RISC-V) are fully compliant. The hopes of getting *all* targets in full compliance are slim, and hence documenting their non-compliance is considered a reasonable compromise when the alternative is making life worse on major recent targets just to support old targets.
This doesn't preclude code from going out of its way to be portable to targets with buggy FP behavior, but it does establish that there is no general expectation that code can deal with non-IEEE-compliant results (due to rounding differences or flush-to-zero) or unstable NaN bits.

Meanwhile, any effort to improve these targets' compliance is certainly very welcome, such as [this recent PR](https://github.com/rust-lang/rust/pull/115919) that would fix the unstable-NaN-bits-in-return-values issue at least for the Rust ABI.
However, given the seeming impossibility of getting targets fully into compliance, that should not block this RFC.

### Alternative: a fully deterministic specification

Why don't we "just" say that NaNs in Rust behave exactly like they do on the underlying hardware?

One reason is that such a guarantee prevents even basic transformations that are otherwise correct for IEEE floats such as using commutativity of arithmetic operations or neutral elements of addition/multiplication: `a * b` will not (or at least not on all hardware) produce the same NaN as `b * a` if both inputs are NaN.

The other reason is that LLVM is currently architecturally unable to reflect target-dependent behavior in its constant-folder, and the LLVM developers have not shown any interest in providing a guarantee of matching target behavior (except via "strict" floating-point intrinsics which are just not optimized).
It is pragmatically very hard for us to provide guarantees that LLVM does not intend to provide.

Also note that in some cases even the underlying target behavior is non-deterministic, namely on wasm.

### Alternative: operations can never produce a signaling NaN

An operation that returns a signaling NaN violates the basic IEEE guarantee that floating-point operations never produce signaling NaNs. However, LLVM considers it legal to fold `x * 1.0` to `x`, so if `x` is a signaling NaN then the multiplication can have that signaling NaN as the output.
Similarly, `(x: f32) as f64 as f32` may get folded into `x`, so even casts can produce signaling NaNs.
Given that signaling NaNs are basically useless in Rust since we do not consider the exception status flag to be observable (and hence optimizations can change whether an exception is triggered ot not), there is no point in constraining optimizations just to achieve a guarantee about signaling NaNs.

### Alternative: don't make any guarantee about the signaling/quiet bit ever

This alternative would simplify the spec and make old MIPS hardware compliant.
The signaling/quiet distinction also basically does not matter since floating-point exception flags are not exposed.
However, there is one operation in C that can sometimes produce a non-NaN result on *quiet* NaN inputs specifically: [`pow`](https://en.cppreference.com/w/c/numeric/math/pow). For instance, "`pow(+1, exponent)` returns `1` for any `exponent`, even when `exponent` is `NaN`".
C also says "This specification does not define the behavior of signaling NaNs", and in practice, `pow(1, sNaN)` returns a NaN.
In other words, `pow` *does* [make a difference](https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=5a0855c2cb6630256e4407624650c4c9) between quiet and signaling NaN.
So, if we say that any operation can arbitrairly produce signaling NaNs, then it becomes impossible to rely on `pow`'s property that "`pow(+1, exponent)` returns `1` for any `exponent`".
Since the quiet/signaling distinction matters, we provide a guarantee which ensures that programs will almost always only ever deal with quiet NaNs (the only safe way to get a signaling NaN is to use `from_bits`).

### Alternative: no strict semantic guarantees

Providing strict IEEE 754-2008 guarantees precludes many transformations, such as turning `a*b + c` into FMA operations (since the result can change due to different rounding).
We could provide weaker guarantees to allow such transformations.
(That would also make using the higher-precision x87 instructions conforming with the spec.)
However, there is no way to bound the effect of such transformations on program behavior: they lead to different outcome observable via `to_bits` and even via pure float operations, which can balloon into arbitrary changes in behavior.
There is also no "monotonicity of precision": while e.g. an FMA instead of mul-then-add will lead to a higher-precision result for this particular operation, this can in principle lead to a lower-precision result in later computations (e.g., `x*x - x*x` can produce non-zero results after being transformed to `x.mul_add(x, -x*x)`).

Given all these caveats, it seems preferable to have explicit opt-in for such semantics-alternating transformations, e.g. via "fast" floating point operation intrinsics that don't provide strict IEEE semantics (and a type that uses those intrinsics for its operations).

## Alternative options for `const` semantics

### Alternative: `const` must be deterministic, and match runtime behavior

This RFC proposes that we accept, for the first time, that a `const fn` can behave non-deterministically at run-time, and produce target/version/flag-specific results at compile-time. It can also produce different results when called at compile-time and at run-time.
[Another proposed RFC](https://github.com/rust-lang/rfcs/pull/3352) gathers some general arguments for why we should allow such behavior in a `const fn`.
The gist of it is that the benefits of forbidding such behavior are speculative (unsafe code *could* exploit that a `const fn` is deterministic, even at runtime, but there is no known practial example that would actually do that -- and it would unnecessarily limit the function to things that are possible at compile-time).
On the other hand, the downsides of requiring determinism are big: given the non-deterministic spec for floating-point operations, we cannot allow floating-point operations in `const fn` until we either have a deterministic spec for them or allow `const fn` to behave non-deterministically when called at runtime.
(See above for why the RFC does not propose a fully deterministic spec for floating-point operations.)

To summarize, achieving deterministic floating-point behavior is too hard with the current state of the art; the downsides of accepting non-determinism are low; so it is not worth blocking floating-point operations in `const fn` on this issue.

Note that this RFC does *not* imply a reliable way for code to detect whether it runs at compile-time or run-time.

### Alternative: `const` just fails when a NaN would be produced

Another alternative to handling floating-point operations in `const` is to just fail when a non-deterministic choice would occur at runtime, i.e., each time a NaN is produced.
However, this is a breaking change: `const C: f32 = 0.0 / 0.0;` has worked on stable Rust since Rust 1.0.

Even under this alternative we would allow `const fn` to perform operations that are non-deterministic at run-time, i.e., unsafe code could still not rely on `const fn` as being a marker for "this is deterministic at run-time". Thus it shares all the downsides with the previous alternative, it just avoids making NaN choices observable in `const`.

The core advantage of this option is that it avoids having `const` results change when the unspecified compile-time NaN changes on a compiler update or across compilers.
However, having `const` results depend on NaN bits should be very rare, and we already have other (more common) cases of `const` results depending on unspecified implementation details that can and sometimes do change on compiler updates, namely the layout of `repr(Rust)` types (observable via `size_of` and `offset_of`).
We can also consider adding a lint against accidentally producing a NaN in CTFE.

### Alternative: `const` tracks NaN values, fails when their bits matter during compile-time

`const` could in principle track NaNs symbolically, similar to how it tracks pointers, and delay choosing NaN payload bits until codegen.
Const-evaluation would then abort if the bits of a NaN are observed (eg. if `to_bits` is called during const-evaluation).
This would keep `const C = 0.0/0.0;` working, but requires `const fn is_nan` to be an intrinsic.
However, it would require massive amounts of work in the compile-time interpreter, comparable in complexity to all the work that is already required to support symbolic pointers (and the RFC author doubts that there will be a lot of opportunity for those two kinds of symbolic state to share infrastructure).
That effort should only be invested if there is a significant payoff.
The RFC author considers the downsides of unspecified NaN bit patterns being observable in const-evaluation to be minimal, and hence the payoff of this alternative to be low.
A less invasive approach to dealing with potential problems from NaN non-determinism is to lint against producing NaNs in `const` evaluation.

# Prior art
[prior-art]: #prior-art

C23 clarifies its stance on signaling NaNs:

> Where specification of signaling NaNs is not provided, the behavior of signaling NaNs is implementation-defined (either treated as an IEC 60559 quiet NaN or treated as an IEC 60559 signaling NaN).

It doesn't say anything about the bit patterns inside NaNs.
This means that strictly speaking, any form of NaN boxing has to re-normalize NaNs after every single operation.
To the author's knowledge, this is not actually done in practice; code instead relies on compilers implementing a more strict semantics.
However, neither GCC nor MSVC document that they actually provide a more strict semantics.

The interpretation of that standard by compilers seems to be that if an operation has a signaling NaN as input, then it may produce a signaling NaN as output.
(It is not clear to the author how that is a valid interpretation of the above sentence, but the standard itself mentions transforming `x * 1.0` to `x` as a valid transformations
when the implementation does not have strict support for signaling NaNs.)

LLVM [recently adopted](https://github.com/llvm/llvm-project/pull/66579) new NaN rules that this RFC copies exactly into Rust.
This means that even though arithmetic operations can produce signaling NaNs, there is a guarantee that signaling NaNs will never appear "out of thin air".
LLVM does not actually document that they are using IEEE float semantics, but de-facto they do on almost all targets (the exception are targets that use x87 instructions, as noted above).

Java requires exact IEEE 754-2008 compliance and goes through a lot of effort to realize that on 32bit x86 without SSE (see [here](https://open-std.org/jtc1/sc22/jsg/docs/m3/docs/jsgn325.pdf) and [here](https://open-std.org/JTC1/SC22/JSG/docs/m3/docs/jsgn326.pdf)). However, they do not seem to tackle the issue of specifying NaN payload bits, even though those bits [can be observed](https://docs.oracle.com/en/java/javase/11/docs/api/java.base/java/lang/Float.html#floatToRawIntBits(float)).

wasm [guarantees](https://webassembly.github.io/spec/core/exec/numerics.html#nan-propagation) that "if all input NaNs are canonical, then any output NaN is canonical". Here ["canonical"](https://webassembly.github.io/spec/core/syntax/values.html#canonical-nan) is defined as a NaN with a "payload whose most significant bit 1 is while all others are 0", i.e. it matches what we call "preferred" above.
(We are departing from wasm terminology since "canonical" already has a different meaning in the IEEE spec. For similar reasons, we consider the NaN payload to *not* include the quiet/signaling bit, whereas wasm considers the quiet/signaling bit to be part of the payload.)
The sign bit is left unspecified, i.e., there are two canonical NaNs (and both are quiet under the standard interpretation of the signaling/quiet bit).

In Rust itself, questions around float semantics have been discussed for a long time.
[This issue](https://github.com/rust-lang/unsafe-code-guidelines/issues/237) collects a lot of that discussion, which culminated in this RFC.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Should we be concerned that LLVM does [not actually document](https://github.com/llvm/llvm-project/issues/60942) that it uses IEEE float semantics?
  It does assume IEEE semantics in its own optimization passes.
- Are there any other targets with floating-point trouble?
- What exactly is the set of "extra" NaNs for all remaining targets?
- To what extend does this specification apply to platform intrinsics?
  On the one hand, it seems reasonable to expect platform intrinsics to have the behavior of the platform instructions.
  On the other hand, we implement some platform intrinsics with the portable LLVM `simd` intrinsics, and those are subject to the NaN-non-determinism described above.
  So the current de-facto semantics of at least some platform intrinsics is that they do *not* match what the platform does.

# Future possibilities
[future-possibilities]: #future-possibilities

- Currently this RFC only talks about scalar `f32`/`f64` operations. What about their (unstable) SIMD equivalents in `std::simd`? Presumably we want all the same rules to apply. The main problem here seems to be 32bit ARM, whose NEON SIMD operations do not follow the usual semantics (they always flush to zero). We can either document this as an errata for that target, or avoid using NEON for `std::simd`.
- In the future, we could attempt to obtain a deterministic specification for the sign bit produced by `0.0 / 0.0` (and in general, by operations that create a NaN without there being a NaN input). However, behavior here differs between x86 and ARM: x86 produces a negative NaN and ARM a positive NaN. LLVM always constant-folds this to a positive NaN -- so doing anything like this is blocked on making the LLVM float const-folder more target-aware.
- For some usecases it can be valuable to run Rust code with a different floating-point environment. However there are major open questions around how to achieve this: without assuming that the floating-point environment is in its default state, compile-time folding of floating-point operations becomes hard to impossible. Any proposal for allowing alternative floating-point operations has to explain how it can avoid penalizing optimizations of code that just wants to use the default settings. [Here's what LLVM offers on that front](https://llvm.org/docs/LangRef.html#constrained-floating-point-intrinsics). C has `#pragma STD FENV_ACCESS` and `#pragma STDC FENV_ROUND` for that; once clang supports those directives, we should determine if we are happy with their semantics and consider also exposing them in Rust.
- To support fast-math transformations, separate fast-path intrinsics / types could be introduced in the future (also see [this issue](https://github.com/rust-lang/rust/issues/21690)).
- There might be a way to specify floating-point operations such that if they return a signaling NaN, then the sign is deterministic.
  However, doing so would require (a) coming up with a suitable specification and then (b) convincing LLVM to adopt that specification.
- We could have a lint that triggers when a compile-time float operation produces a NaN, as that will usually not be intended behavior.
