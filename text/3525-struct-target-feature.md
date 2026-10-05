# Summary

[summary]: #summary

Allow adding `#[target_feature(enable = "...")]` attributes to unit structs, and
enable the corresponding target features to functions taking those structs as
parameters.

# Motivation

[motivation]: #motivation

Currently, the only way to tell the compiler it can assume the availability of
hardware features is by annotating a function with the corresponding
`#[target_feature]` attribute. This requires that the annotated function be
marked as unsafe as the caller must check whether the features are available at
runtime.  
This also makes it difficult for library authors to use in certain situations, as
they may not know which features the library user wants to detect, and at what
level the dynamic dispatch should be done.

Assume we want to implement a library function that multiplies a slice of `f64`
values by `2.0`.

```rust
pub fn times_two(v: &mut [f64]) {
    for v in v {
        *v *= 2.0;
    }
}
```

Generally speaking, during code generation, the compiler will only assume the
availability of globally enabled target features (e.g., `sse2` on `x86-64`
unless additional feature flags are passed to the compiler).

This means that if the code is run on a machine with more efficient features
such as `avx2`, the function will not be able to make good use of them.

To improve performance, the library author may decide to add runtime feature
detection to their implementation, choosing subsets of features to detect.

```rust
#[inline(always)]
fn times_two_generic(v: &mut [f64]) {
    for v in v {
        *v *= 2.0;
    }
}

#[target_feature(enable = "avx")]
unsafe fn times_two_avx(v: &mut [f64]) {
    times_two_generic(v);
}

#[target_feature(enable = "avx512f")]
unsafe fn times_two_avx512f(v: &mut [f64]) {
    times_two_generic(v);
}

pub fn times_two(v: &mut[f64]) {
    if is_x86_feature_detected!("avx512f") {
        times_two_avx512f(v);
    } else if is_x86_feature_detected!("avx") {
        times_two_avx(v);
    } else {
        times_two_generic(v);
    }
}
```

This decision, however, comes with a few drawbacks:

- The runtime dispatch now implies that the code has some additional overhead
  to detect the hardware features, which can harm performance for small
  slices.
- The addition of more code paths increases binary size.
- The dispatch acts as a barrier that prevents inlining, which can prevent
  compiler optimizations at the call-site.
- This requires adding unsafe code to the library, which has a maintenance cost.

The proposed alternative offers solutions for these issues.

# Guide-level explanation

[guide-level-explanation]: #guide-level-explanation

Suppose we define the following structs:

```rust
#[target_feature(enable = "avx")]
#[derive(Clone, Copy, Debug)]
pub struct Avx;

#[target_feature(enable = "avx512f")]
#[derive(Clone, Copy, Debug)]
pub struct Avx512f;
```

The `#[target_feature(enable = "avx")]` annotation informs the compiler that
instances of this struct can only be created if the `avx` target feature is
available, and allows it to optimize code based on that assumption.

Note that this makes the creation of instances of type `Avx` unsafe.

Now assume that the following methods are defined.

```rust
#[inline]
pub fn try_new_avx() -> Option<Avx> {
    if is_x86_feature_detected!("avx") {
        Some(unsafe { Avx })
    } else {
        None
    }
}

#[inline]
pub fn try_new_avx512f() -> Option<Avx512f> {
    if is_x86_feature_detected!("avxf") {
        Some(unsafe { Avx512f })
    } else {
        None
    }
}
```

Then the library code can now be written as

```rust
#[target_feature(inherit)]
pub fn times_two<S>(simd: S, v: &mut [f64]) {
    for v in v {
        *v *= 2.0;
    }
}
```

The user can now call this function in this manner.

```rust
fn main() {
    let mut v = [1.0; 1024];

    if let Some(simd) = try_new_avx512f() {
        times_two(simd, &mut v); // 1
    } else if let Some(simd) = try_new_avx() {
        times_two(simd, &mut v); // 2
    } else {
        times_two((), &mut v); // 3
    }
}
```

In the first branch, the compiler instantiates and calls the function
`times_two::<Avx512f>`, which has the signature `fn(Avx512f, &mut [f64])`.
Since the function takes as an input parameter `Avx512f`, that means that
calling this function implies that the `avx512f` feature is available, which
allows the compiler to perform optimizations that wouldn't otherwise be
possible (in this case, automatically vectorizing the code with AVX512
instructions).

In the second branch, the same logic applies but for the `Avx` struct and the
`avx` feature.

In the third branch, the called function has the signature `fn((), &mut [f64])`.
None of its parameters have types that were annotated with the
`#[target_feature]` attribute, so the compiler can't assume the availability of
features other than those that are enabled at the global scope.

Moving the dispatch responsibility to the caller allows more control over how
the dispatch is performed, whether to optimize for code size or performance.

Additionally, the process no longer requires any unsafe code.

# Reference-level explanation

[reference-level-explanation]: #reference-level-explanation

This RFC proposes that structs and tuple structs be allowed to have one or several
`#[target_feature(enable = "...")]` attributes.

Structs with such annotations are unsafe to construct, except in the body
of a function with the same target features. Creating an instance of
such a struct has the same safety requirements as calling a function marked with
the same `#[target_feature]` attribute.

This RFC additionally proposes that functions annotated with `#[target_feature(inherit)]`, and
taking parameters with a type that has been annotated with a `#[target_feature]`,
also behave as if they have been annotated with the corresponding
`#[target_feature(enable = "...")]`, except that this doesn't impose on them
the requirement of having to be marked `unsafe`.

Structs and tuple structs
containing members with such types can also opt into inheriting its members' `#[target_feature]`
attributes with `#[target_feature(inherit)]`. Unlike the structs annotated, they remain safe to
construct. This is sound because creating them requires creating an instance of
the target feature type to exist, which guarantees the target features' availability.

Note: `PhantomData<T>` must not inherit target feature attributes from `<T>`,
as it is always safe to construct, despite acting like it contains `T`.

The advantage of this extension is that it allows target features to naturally compose.
If a user wants to define a structure that enables both of `avx` and `fma`, then they could
define the structures of `Avx` and `Fma` separately, marked with the appropriate target features.
And then simply pass them together in a tuple or a struct containing both.

```rust
#[target_feature(inherit)]
pub fn times_two<S>(simd: S, v: &mut [f64]) {
    for v in v {
        *v *= 2.0;
    }
}

#[target_feature(inherit)]
struct AvxFma(Avx, Fma);

fn main() {
    let mut v = [1.0; 1024];
    if let (Some(avx), Some(fma)) = (try_new_avx(), try_new_fma()) {
        times_two(AvxFma(avx, fma), &mut v);
    }
}
```

# Drawbacks

[drawbacks]: #drawbacks

Since the proposed API is opt-in, this has no effect on existing code.

# Rationale and alternatives

[rationale-and-alternatives]: #rationale-and-alternatives

An alternative option to inheriting all target features could be to make them also opt-in
at the parameter level. Let us take our previous example:

```rust
#[target_feature(inherit)]
pub fn times_two<S>(simd: S, v: &mut [f64]) {
    // ...
}
```

This RFC suggests that all of the input parameters of `times_two` are scanned
during monomorphization, and target features are inherited from them
appropriately. The alternative is to explicitly mark which parameters
`times_two` is allowed to inherit target features from. Perhaps through the use
of a second attribute.

```rust
#[target_feature(inherit)]
pub fn times_two<S>(#[target_feature] simd: S, v: &mut [f64]) {
    // ...
}
```

It is not clear if there are any advantages to this approach, other than being
more explicit.

# Unresolved questions

[unresolved-questions]: #unresolved-questions

Should we also allow `#[target_feature(disable = "...")]` to be used with structs?

Should references implicitly inherit their pointee's target features? This would
potentially impose more strict validity requirements for references of such types
than other types, which may break unsafe generic code that creates references pointing
to uninit data, without dereferencing them.

# Future possibilities

[future-possibilities]: #future-possibilities

