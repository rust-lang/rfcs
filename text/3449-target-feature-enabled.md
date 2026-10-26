- Feature Name: `target_feature_enabled`
- Start Date: 2023-06-17
- RFC PR: [rust-lang/rfcs#3449](https://github.com/rust-lang/rfcs/pull/3449)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add new `is_{arch}_feature_enabled` macros for detecting at compile time if a target feature is enabled in a context.

# Motivation
[motivation]: #motivation

[RFC #2045](https://github.com/rust-lang/rfcs/pull/2045) added two methods of querying target features:
* conditional compilation, e.g. `#[cfg(target_feature = "avx")]`
* runtime detection, e.g. `is_x86_feature_detected!("avx")`

Conditional compilation only allows querying the base target features, and does not interact with `#[target_feature]`.
Runtime detection is necessary for safely calling functions tagged with `#[target_feature]`, but incurs a runtime overhead.
In some cases, it is necessary to determine which target features are enabled at code generation (particularly after inlining) to allow various optimizations:

```rust
#[inline(always)]
fn conditional_compilation(...) {
    // This branch is always optimized out, and depends on the target features enabled at code generation.
    // If this were `cfg!(target_feature = "avx")`, this branch would never select the AVX version
    // (without enabling AVX for the entire binary).
    if is_x86_feature_enabled!("avx") {
        unsafe { avx_implementation(...) }
    } else {
        generic_implementation(...)
    }
}

#[inline(always)]
fn runtime_detection(...) {
    // If AVX is enabled at code generation, this branch is optimized out and runtime detection is skipped.
    if is_x86_feature_enabled!("avx") || is_x86_feature_detected!("avx") {
        unsafe { avx_implementation(...) }
    } else {
        generic_implementation(...)
    }
}

#[target_feature(enable = "avx")]
unsafe fn with_avx_enabled(...) {
    // This call selects the AVX implementation, because of this function's target features.
    conditional_compilation(...);

    // This call selects the AVX implementation, skipping runtime detection!
    runtime_detection(...);
}
```

A particularly useful case is nested runtime detection:

```rust
#[inline(always)]
fn first(...) {
    #[target_feature(enable = "avx")]
    #[inline]
    unsafe fn first_avx(...) {
        second(...)
    }
    
    #[inline(always)]
    fn first_generic(...) {
        second(...)
    }

    if is_x86_feature_detected!("avx") {
        unsafe { first_avx(...) }
    } else {
        first_generic(...)
    }
}

#[inline(always)]
fn second(...) {
    #[target_feature(enable = "avx")]
    #[inline]
    unsafe fn second_avx(...) {
        ...
    }
    
    #[inline(always)]
    fn second_generic(...) {
        ...
    }

    if is_x86_feature_enabled!("avx") || is_x86_feature_detected!("avx") {
        unsafe { second_avx(...) }
    } else {
        second_generic(...)
    }
}
```

After inlining, calling `first` only runs target feature detection once!  The nested runtime detection is elided.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The syntax for `is_{arch}_feature_enabled` macro is like `is_{arch}_feature_detected`.

Instead of detecting if the target feature is supported at runtime, the macro returns whether the target feature is supported in the current context at compile time.

A target feature is supported in a particular context if at least one of the following is true:
* The target supports the feature by default, or it is enabled with the `-Ctarget-feature` flag (i.e. `cfg!(target_feature = "feature")` is true)
* The function containing the macro invocation is annotated with `#[target_feature(enable = "feature")]`
* The function containing the macro invocation is inlined into a function annotated with `#[target_feature(enable = "feature")]`

## Example
```rust
/// Computes `(a * b) + c` with unspecified rounding.
#[inline]
fn fast_mul_add(a: f32, b: f32, c: f32) -> f32 {
    if is_x86_feature_enabled!("fma") {
        a.mul_add(b, c) // a single instruction faster than separate mul and add
    } else {
        a * b + c // separate mul and add is faster than calling the library `fmaf` function
    }
}
```

## Keep in mind
The accuracy of this macro is "best-effort": depending on your particular code and Rust configuration, it may still return `false` even if the feature is enabled.
For example:
* Using `#[inline(always)]` may be more accurate than `#[inline]`
* This feature depends on MIR inlining being enabled

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The macros generate calls to new compiler intrinsics.
These intrinsics are lowered by each codegen backend to the appropriate `true` or `false` value depending on the target features of the parent function.

Codegen operates on optimized MIR, so the intrinsic is lowered after MIR inlining has occurred.
When lowering the intrinsic, the backend can optionally account for additional inlining passes.

# Drawbacks
[drawbacks]: #drawbacks

Target features are already complicated, and adding this macro increases complexity.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Compared to runtime detection
In small functions, the runtime cost of feature detection and branching is unacceptable.
Runtime detection is also inefficient and redundant when used in many functions that may call each other.

## Compared to conditional compilation
[RFC #2045](https://github.com/rust-lang/rfcs/pull/2045) proprosed making `#[cfg(target_feature = "feature")]` context-dependent, but this was never implemented.
`cfg` is always consistent throughout a program, so making it context dependent might be confusing and lead to mistakes.
Additionally, it's not clear how context-dependent `#[cfg(target_feature = "feature")]` on items (rather than blocks) should or could work.
Adding a new `is_{arch}_feature_enabled` macro avoids this complexity.

## Compared to a library
This kind of feature necessarily requires compiler support.

Some crates, such as [`glam`](https://github.com/bitshifter/glam-rs/blob/47630c35adb52c08def02cce4ddfbddaa45d1b9d/src/f32/sse2/vec4.rs#L648-L658) or [`simd-json`](https://github.com/simd-lite/simd-json/blob/21f7878505a472ab93077b18294888d73ce330b9/src/lib.rs#L152-L159), opt to use `cfg` which simplifies the library API, but is pessimistic since many applications compile with default target features to maximize compatibility.
Other libraries, such as [`rustfft`](https://docs.rs/rustfft/6.1.0/rustfft/struct.FftPlannerAvx.html) or [`highway`](https://docs.rs/highway/1.0.0/highway/index.html), leave the decision up to the user at the cost of a more verbose API, especially for users that are not aware of or interested in SIMD or target features.

The [`multiversion`](https://github.com/calebzulawski/multiversion) crate provides a `target_cfg` macro which is identical to `cfg`, but accounts for the `#[target_feature]` attribute.
This is similar functionality to this RFC, but is limited to functions tagged with special attributes and does not account for inlining.
Therefore, `multiversion` is unsuitable for creating small reusable functions that are intended to inline into another function with target features enabled.

# Prior art
[prior-art]: #prior-art

I've already mentioned [RFC #2045](https://github.com/rust-lang/rfcs/pull/2045) a couple times, which briefly discusses this problem, but the proposed solution was never implemented.

As far as I know, something like this has not been implemented in another compiler or language.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

* Is relying on MIR inlining sufficient, or does this need to interact with the LLVM inline pass?
* How would the LLVM backend lower the intrinsic to account for LLVM's inline pass? A few possibilities:
  * Add a new intrinsic to LLVM
  * Use a custom optimization pass
* Is the identifier too similar to `is_{arch}_feature_detected`?
  * The names are very similar (only 6 characters different) and may appear to be the same at a glance.
  * On the other hand, the two macros are very closely related, to the point of being interchangeable with just a compile time vs runtime tradeoff.

# Future possibilities
[future-possibilities]: #future-possibilities

The proposed macros are relatively simple, but there are a few avenues for future changes:
* `is_{arch}_feature_detected` could make use of this macro for additional optimization opportunity.
* Since inlining is necessarily late in the compilation process, it would be difficult to make the macro evaluate to a `const` value, and this RFC does not propose that.  Future improvement could make it `const`, however.
* There is additional opportunity for inlining after the MIR inliner, performed by LLVM.  This functionality could be integrated into LLVM to improve accuracy in those cases.
* New language features affecting target features (such as hypothetical `#[target_feature]` closure or block) could interact with these macros.
