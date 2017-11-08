- Feature Name: `minimal_target_feature_unsafe`
- Start Date: 2017-11-07
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

[RFC 2045][1] defines [`[#target_feature]`][2] as applying only to unsafe
functions. This RFC allows `[#target_feature]` to apply safe functions outside
trait implementations but makes it unsafe to call a function that has
instruction set extensions enabled that the caller doesn't have enabled
(even if the callee isn't marked `unsafe`). Taking a function pointer to a
safe function that has `[#target_feature]` is prohibited.

[1]: https://github.com/rust-lang/rfcs/blob/master/text/2133-all-the-clones.md
[2]: https://github.com/rust-lang/rfcs/blob/master/text/2045-target-feature.md#unconditional-code-generation-target_feature

# Motivation
[motivation]: #motivation

`[#target_feature]` applying only to functions that are declared `unsafe`
makes Rust's safe/`unsafe` distinction less useful, because it causes
unnecessarily many things to become `unsafe`. Specifically, it causes
operations that depend on instruction set extensions, such as [SIMD
operations][3] to become `unsafe` wholesale and it causes the entire part
of the program that uses instruction set extensions (compared to the
program-wide baseline) to become `unsafe`.

Worse, the logic that causes instruction set extension-using operation like
SIMD operations to become `unsafe` (they might execute UB unless the
programmer has properly checked at run time that the instruction set extension
is supported but the host CPU) means that it's impossible to create
_efficient_ safe abstraction over the `unsafe` operations without cheating
about the notion of safety.

[3]: https://github.com/rust-lang-nursery/stdsimd/issues/159

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

`[#target_feature]` is allowed on `unsafe` functions and safe functions that
are not part of a trait definition or implementation. Taking a function
pointer to a safe function that has `[#target_feature]` is not allowed.

Each function has a _set of instruction set extensions_ that it is compliled
with. It is a set of zero or more _instruction set extension_ is a set of
instructions that the compiler can conditionally be permitted to emit or be
told to refrain from emitting. Usually these map to sets of instruction for
which support is optional on the CPU level. However, the compiler may treat
even guaranteed-supported instructions as being part of an _instruction set
extension_ in order to be able to refrain from using them e.g. in kernel-mode
code to make it unnecessary to save some register upon entering a system
call. (For example, floating point instructions may be treated as an
_instruction set extension_ by the compiler when the compiler is capable of
refraining from emitting them e.g. for kernel-mode code even when an FPU
is a guaranteed part of the CPU architecture.)

All functions in a program have a (possibly empty) baseline of _instruction
set extensions_ that depend on the program-wide compliation target. For
example, the `i686-*` targets have the SSE and SSE2 instruction set extensions
enabled, so, by default, when when building for a `i686-*` target, the _set
of instruction set extensions_ for every function includes SSE and SSE2.

To make these not appear in the _set of instruction set extensions_ for every
function, one would have to build for a `i586-*` target instead.

On the other hand, SSE4.1 _instruction set extension_ can be enabled by
for every function in the program by specifying
`RUSTFLAGS="-C target-cpu=+sse4.1"` when invoking `cargo` or for a particular
function by specifying `#[target_feature(enable = "sse4.1")]` on the function.

Some _instruction set extensions_ are defined to imply other _instruction
set extensions_. In particular, a given version of the SSE family of
_instruction set extensions_ implies the earlier version. Therefore, even if
only `sse4.1` is defined via `#[target_feature]`, the _set of instruction set
extensions_ ends up containing SSE, SSE2, SSE3 and SSSE3 in addition to
SSE4.1.

The _set of instruction set extensions_ is considered as part of of the
type of the callee for the purpose of determining if a safe function callee
is compatible with a given caller.

If the callee is safe, calling it is allowed without an `unsafe` block if
the _set of instruction set extensions_ of the callee is a subset of the
_set of instruction set extensions_ of the caller, including them being the
same set (and other instruction set extension and `target_feature`-unrelated
conditions that Rust requires are met).

To call a function whose _set of instruction set extensions_ includes items
not preset in the _set of instruction set extensions_ of the caller, an
`unsafe` block is required (or the caller as a whole has to be declared
`unsafe`). This `unsafe` means that the programmer asserts to the compiler
that the present host CPU supports the additional instruction set extensions
present in the callee's _set of instruction set extensions_.

(Note: This use of `unsafe` is unusual in the sense that it is required in
a situation where the callee isn't declade as `unsafe`.)

For example, if `foo()`, `bar()` and `baz()` are safe functions and the
_set of instruction set extensions_ of `foo()` contains SSE and SSE2 and
the _set of instruction set extensions_ for both `bar()` and `baz()` contains
SSE, SSE2, SSE3, SSSE3 and SSE4.1, an `unsafe` block is required to call
either `bar()` or `baz()` from `foo()`, but `bar()` and `baz()` can call
`foo()` or each other without `unsafe`.

As a result, `unsafe` is needed only at the transition to code that may
invoke instructions that the caller couldn't and otherwise code can remain
safe even if it uses instructions that might not be guaranteed to be supported
by every host CPU.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The compiler must maintain the _set of instruction set extensions_ for each
function (including serializing it in crate metadata for exported functions)
and emit an error (failing compilation) if function is called without `unsafe`
such that the _set of instruction set extensions_ of the callee is not a
subset of the caller (in the standard sense of "subset" where a set is a
subset of itself).

The compiler must prohibit `[#target_feature]` on safe function in trait
definitions and implementations.

The compiler must prohibit taking a function pointer to a safe function that
has `[#target_feature]`.

(Issues related to inlining and ABI on the boundary where the caller and
callee differ in their _set of instruction set extensions_ are out of scope
of this RFC, because they already arise from `[#target_feature]` without this
RFC.)

# Drawbacks
[drawbacks]: #drawbacks

This complicates the notion of `unsafe` a bit by requiring the caller context
to be designated as `unsafe` in a case where the callee isn't declared
`unsafe`.

# Rationale and alternatives
[alternatives]: #alternatives

This formulation avoids the need to make SIMD operations `unsafe` wholesale
and avoid having to mark `unsafe` entire constellations of functions that
implement conditionally-executed acceleration using instructions not supported
by all CPUs. By minimizing `unsafe`, the `unsafe` that remains is more
meaningful and useful (e.g. for locating points in the program that require
special review).

Alternatively, instead of using `unsafe` for this, a new `unsafe`-like keyword
could be minted for the case where the call is determined to be unsafe without
the callee being declared `unsafe`. However, the precedent in Rust is to use
`unsafe` for all kinds of `unsafe` instead of having a taxonomy of different
checks that `unsafe` waives.

As an alternative to prohibiting `[#target_feature]` on safe functions in
trait definitions or implementations, taking a trait object reference to a
struct in the case where the trait definition or the struct's implementation
of the trait contains `[#target_feature]` on safe functions could be
prohibited. This might be less teachable.

# Unresolved questions
[unresolved]: #unresolved-questions

See the last paragraph of the previous section. Should taking the problematic
kind of trait object be probibited instead of prohibiting `[#target_feature]`
on safe functions even in the case of static dispatch?
