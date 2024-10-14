- Feature Name: `inline(must)` and `inline(required)`
- Start Date: 2024-09-17
- RFC PR: [rust-lang/rfcs#3711](https://github.com/rust-lang/rfcs/pull/3711)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

rustc supports inlining functions with the `#[inline]`/`#[inline(always)]`/
`#[inline(never)]` attributes, but these are only hints to the compiler.
Sometimes it is desirable to require inlining, not just suggest inlining:
security APIs can depend on being inlined for their security properties, and
performance intrinsics can work poorly if not inlined.

# Motivation
[motivation]: #motivation

rustc's current inlining attributes are only hints, they do not guarantee that
inlining is performed and users need to check manually whether inlining was
performed. While this is desirable for the vast majority of use cases, there are
circumstances where inlining is necessary to maintain the security properties of
an API, or where performance of intrinsics is likely to suffer if not inlined.
For example, consider:

- Armv8.3-A's pointer authentication, when used explicitly with intrinsics[^1],
rely on being inlined to guarantee their security properties. Rust should make
it easier to guarantee that these intrinsics will be inlined and emit an error
if that is not possible or has not occurred.
- Arm's Neon and SVE performance intrinsics have work poorly if not inlined.
Since users rely on these intrinsics for their application's performance, Rust
should be able to warn users when these have not been inlined and performance
will not be as expected.

In general, for many intrinsics, inlining is not an optimisation but an
important part of the semantics, regardless of optimisation level.

[^1]: Armv8.3-A's pointer authentication can be used implicitly (automatic,
hint-based) and explicitly (intrinsics). Implicit pointer authentication is
already implemented by Rust. Explicit pointer authentication intrinsics are not
yet available in Rust. Exposing pointer authentication intrinsics would require
this RFC, or something providing equivalent guarantees, as a prerequisite.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

There are now be five variations of the `#[inline]` attribute, the
existing bare `#[inline]`, `#[inline(always)]`, `#[inline(never)]` and
the new  `#[inline(must)]` and `#[inline(required)]`. Bare `#[inline]`,
`#[inline(always)]` and `#[inline(never)]` are unchanged. `#[inline(must)]`
and `#[inline(required)]` will always attempt to inline the annotated item,
regardless of optimization level.

If it is not possible to inline the annotated item then the compiler will emit
a lint on the caller of the annotated item that the item could not be inlined,
providing a reason why inlining was not possible. `#[inline(must)]` emits a
warn-by-default lint and `#[inline(required)]` emits a deny-by-default lint,
which can be used by items which must be inlined to uphold security properties.
Callers of annotated items can always override this lint with the usual
`#[allow]`/`#[warn]`/`#[deny]`/`#[expect]` attributes. For example:

```rust
// somewhere/in/std/intrinsics.rs
#[inline(required)]
pub unsafe fn ptrauth_auth_and_load_32<const KEY: PtrAuthKey, const MODIFIER: u64, const OFFSET: u32>(value: u64) -> u32 {
    /* irrelevant detail */
}

// main.rs
#[warn(required_inline)]
fn do_ptrauth_warn() {
    intrinsics::ptrauth_auth_and_load_32(/* ... */);
    //~^ WARN `ptrauth_auth_and_load_32` could not be inlined but requires inlining
}
```

Both `#[inline(must)]` and `#[inline(required)]` can optionally provide
the user a justification for why the annotated item is enforcing inlining,
such as `#[inline(must("maintain performance characteristics"))]` or
`#[inline(required("uphold security properties"))]`.

Failures to force inline should not occur sporadically, users will only
encounter this lint when they call an annotated item from a location that the
inliner cannot inline into (e.g. a coroutine - a current limitation of the MIR
inliner), or when compiling with incompatible options (e.g. with code coverage -
a current limitation of the MIR inliner/code coverage implementations).

`#[inline(required)]` and `#[inline(must)]` differs from `#[inline(always)]` in
that it emits the lint when inlining does not happen and rustc will guarantee
that no heuristics/optimization fuel considerations are employed to consider
whether to inline the item.

`#[inline(must)]` and `#[inline(required)]` are intended to remain unstable
indefinitely and be used only within the standard library (e.g. on intrinsics).
This could be relaxed if there were sufficient motivation for use of these
inlining attributes in user code.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

As LLVM does not provide a mechanism to require inlining, only a mechanism
to provide hints as per the current `#[inline]` attributes, `#[inline(must)]`
and `#[inline(required)]` will be implemented in rustc as part of the MIR
inliner. Small modifications to the MIR inliner will make the MIR pass run
unconditionally (i.e. with `-O0`), while only inlining non-`#[inline(must)]`/
`#[inline(required)]` items under the current conditions and always inlining
`#[inline(must)]`/`#[inline(required)]` items. Any current limitations of the
MIR inliner will also apply to `#[inline(must)]`/`#[inline(required)]` items and
will be cases where the lint is emitted. `#[inline(must)]`/`#[inline(required)]`
will be considered an alias of `#[inline(always)]` after MIR inlining when
performing codegen.

# Drawbacks
[drawbacks]: #drawbacks

- It may be undesirable for the MIR inliner to be necessary for the correctness
  of a Rust program (i.e. for the `#[inline(required)]` case).

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- An "intrinsic macro" type of solution could be devised for the "security
properties" use case instead as macros are inherently inlined.
- Given the existence of security features with intrinsics that must be inlined
to guarantee their security properties, not doing this (or something else)
isn't a viable solution unless the project decides these are use cases that the
project does not wish to support.

# Prior art
[prior-art]: #prior-art

gcc and clang both have partial equivalents (e.g. [clang](https://
clang.llvm.org/docs/AttributeReference.html#always-inline-force-inline))
which ignore their inlining heuristics but still only guarantee that they
will attempt inlining, and do not notify the user if inlining was not possible.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

There aren't any unresolved questions in this RFC currently.

# Future possibilities
[future-possibilities]: #future-possibilities

This feature is fairly self-contained and doesn't lend itself to having future expansion.
