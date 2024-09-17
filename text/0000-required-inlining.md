- Feature Name: `inline(required)`
- Start Date: 2024-09-17
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
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

[^1]: Armv8.3-A's pointer authentication can be used implicitly (automatic,
hint-based) and explicitly (intrinsics). Implicit pointer authentication is
already implemented by Rust. Explicit pointer authentication intrinsics are not
yet available in Rust. Exposing pointer authentication intrinsics would require
this RFC, or something providing equivalent guarantees, as a prerequisite.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

There are now be four variations of the `#[inline]` attribute - bare
`#[inline]`, `#[inline(always)]`, `#[inline(never)]` and `#[inline(required)]`.
Bare `#[inline]`, `#[inline(always)]` and `#[inline(never)]` are unchanged.
`#[inline(required)]` will always inline the annotated item, regardless of
optimization level.

If it is not possible to inline the annotated item then the compiler will
emit a warn-by-default lint on the caller of the annotated item that the
item could not be inlined, providing a reason why inlining was not possible.
`#[inline(required)]` is an alias for `#[inline(required = "warn")]`.
`#[inline(required = "deny")]` can be used instead, which will emit a
deny-by-default lint - this variant can be used by items which must be inlined
to uphold security properties. Callers of annotated items can always override
this lint with the usual `#[allow]`/`#[warn]`/`#[deny]`/`#[expect]` attributes.
For example:

```rust
// somewhere/in/std/intrinsics.rs
#[inline(required = "deny")]
pub unsafe fn ptrauth_auth_and_load_32<const KEY: PtrAuthKey, const MODIFIER: u64, const OFFSET: u32>(value: u64) -> u32 {
    /* irrelevant detail */
}

// main.rs
fn do_ptrauth_warn() {
    intrinsics::ptrauth_auth_and_load_32(/* ... */);
    //~^ ERROR `ptrauth_auth_and_load_32` could not be inlined but requires inlining
}
```

Failures to force inline should not occur sporadically, users will only
encounter this lint when they call an annotated item from a location that the
inliner cannot inline into (e.g. a coroutine - a current limitation of the MIR
inliner), or when compiling with incompatible options (e.g. with code coverage -
a current limitation of the MIR inliner/code coverage implementations).

`#[inline(required)]` differs from `#[inline(always)]` in that it emits the
lint when inlining does not happen and rustc will guarantee that no heuristics/
optimization fuel considerations are employed to consider whether to inline
the item.

It is intended that `#[inline(required)]` only be used in cases where inlining
is strictly necessary and is documented to be so, such as with some intrinsics.
`#[inline]`'s documentation should reflect that except in these cases, bare
`#[inline]`, `#[inline(always)]`, and `#[inline(never)]` should be preferred.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

As LLVM does not provide a mechanism to require inlining, only a mechanism to
provide hints as per the current `#[inline]` attributes, `#[inline(required)]`
will be implemented in rustc as part of the MIR inliner. Small modifications to
the MIR inliner will make the MIR pass run unconditionally (i.e. with `-O0`),
while only inlining non-`#[inline(required)]` items under the current conditions
and always inlining `#[inline(required)]` items. Any current limitations of the
MIR inliner will also apply to `#[inline(required)]` items and will be cases
where the lint is emitted. `#[inline(required)]` will be considered an alias of
`#[inline(always)]` after MIR inlining when performing codegen.

# Drawbacks
[drawbacks]: #drawbacks

- It may be undesirable for the MIR inliner to be necessary for the correctness
of a Rust program (i.e. for the `#[inline(required)]` case).

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- Instead of `#[inline(required = "deny")]`, instead have `#[inline(required)]`
and `#[forced_inlining = "deny"]` or something like that - two separate
attributes.
- An "intrinsic macro" type of solution could be devised for the "security
properties" use case instead as macros are inherently inlined.
- Given the existence of security features with intrinsics that must be inlined
to guarantee their security properties, not doing this (or something else)
isn't a viable solution unless the project decides these are use cases that the
project does not wish to support.

# Prior art
[prior-art]: #prior-art

gcc and clang both have equivalents of `inline(always)` (e.g. [clang](https://
clang.llvm.org/docs/AttributeReference.html#always-inline-force-inline))
which ignore their heuristics but still only guarantee that they will attempt
inlining, and do not notify the user if inlining was not possible.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

There aren't any unresolved questions in this RFC currently.

# Future possibilities
[future-possibilities]: #future-possibilities

This feature is fairly self-contained and doesn't lend itself to having future expansion.
