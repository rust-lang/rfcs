- Feature Name: repr_scalable
- Start Date: 2022-05-19
- RFC PR: [rust-lang/rfcs#3268](https://github.com/rust-lang/rfcs/pull/3268)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Expanding the SIMD functionality to allow for runtime determined vector lengths.

# Motivation
[motivation]: #motivation

Without some support in the compiler it would be impossible to use the
[ACLE](https://developer.arm.com/architectures/system-architectures/software-standards/acle)
[SVE](https://developer.arm.com/documentation/102476/latest/) intrinsics from Arm.

This RFC will focus on the Arm vector extensions, and will use them for all examples. A large amount of what this
RFC covers is emitting the vscale attribute from LLVM, therefore other scalable vector extensions should work.
In an LLVM developer meeting it was mentioned that RISC-V would use what's accepted for Arm SVE for their vector extensions.
\[[see slide 17](https://llvm.org/devmtg/2019-04/slides/TechTalk-Kruppe-Espasa-RISC-V_Vectors_and_LLVM.pdf)\]

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This is mostly an extension to [RFC 1199 SIMD Infrastructure](https://rust-lang.github.io/rfcs/1199-simd-infrastructure.html).
An understanding of that is expected from the reader of this. In addition to that, a basic understanding of
[Arm SVE](https://developer.arm.com/documentation/102476/latest/) is assumed.

Existing SIMD types are tagged with a `repr(simd)` and contain an array or multiple fields to represent the size of the
vector. Scalable vectors have a size known (and constant) at run-time, but unknown at compile time. For this we propose a
new kind of exotic type, denoted by an additional `repr()`. This additional representation, `scalable`,
accepts an integer to determine the number of elements per granule. See the definitions in
[the reference-level explanation](#reference-level-explanation) for more information.

e.g. for a scalable vector f32 type the following could be its representation:

```rust
#[repr(simd, scalable(4))]
pub struct svfloat32_t {
    _ty: [f32],
}
```
`_ty` is purely a type marker, used to get the element type for the LLVM backend.


This new class of type has the following properties:
* Not `Sized`, but it does exist as a value type.
  * These can be returned from functions.
* Heap allocation of these types is not possible.
* Can be passed by value, reference and pointer.
* The types can't have a `'static` lifetime.
* These types can be loaded and stored to/from memory for spilling to the stack,
  and to follow any calling conventions.
* Can't be stored in a struct, enum, union or compound type.
  * This includes single field structs with `#[repr(trasparent)]`.
  * This also means that closures can't capture them by value.
* Traits can be implemented for these types.
* These types are `Unpin`.

A simple example that an end user would be able to write for summing of two arrays using functions from the ACLE
for SVE is shown below:

```rust
unsafe {
    let step = svcntw() as usize;
    for i in (0..SIZE).step_by(step) {
        let a = data_a.as_ptr().add(i);
        let b = data_b.as_ptr().add(i);
        let c = &mut data_c as *mut f32;
        let c = c.add(i);

        let pred = svwhilelt_b32(i as _, SIZE as _);
        let sva = svld1_f32(pred, a);
        let svb = svld1_f32(pred, b);
        let svc = svadd_f32_m(pred, sva, svb);

        svst1_f32(svc, pred, c);
    }
}
```
As can be seen by that example the end user wouldn't necessarily interact directly with the changes that are
proposed by this RFC, but might use types and functions that depend on them.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This will focus on LLVM. No investigation has been done into the alternative codegen back ends. At the time of
writing I believe cranelift doesn't support scalable vectors ([current proposal](https://github.com/bytecodealliance/rfcs/pull/19)),
and the GCC backend is not mature enough to be thinking about this.

Most of the complexity of SVE will be handled by LLVM and the `vscale` modifier that is applied to vector types. Therefore
changes for this should be fairly minimal for Rust. From the LLVM side this is as simple as calling `LLVMScalableVectorType`
rather than `LLVMVectorType`.

For a Scalable Vector Type LLVM takes the form `<vscale x elements x type>`.
* `elements` multiplied by sizeof(`type`) gives the smallest allowed register size and the increment size.
* `vscale` is a run time constant that is used to determine the actual vector register size.

For example, with Arm SVE the scalable vector register (Z register) size has to
be a multiple of 128 bits and a power of 2 (via a retrospective change to the
architecture), therefore for `f32`, `elements` would always be four. At run time
`vscale` could be 1, 2, 4, 8, 16 which would give register sizes of 128, 256,
512, 1024 and 2048. While SVE now has the power of 2 restriction, `vscale` could
be any value providing it gives a legal vector register size for the
architecture.

The scalable representation accepts the number of `elements` rather than the compiler calculating it, which serves
two purposes. The first being that it removes the need for the compiler to know about the user defined types and how to calculate
the required `element` count. The second being that some of these scalable types can have different element counts. For instance,
the predicates used in SVE have different element counts in LLVM depending on the types they are a predicate for.

As mentioned previously `vscale` is a runtime constant. With SVE the vector length can be changed at runtime (e.g. by a
[prctl()](https://www.kernel.org/doc/Documentation/arm64/sve.txt) call in Linux). However, since this would require a change
to `vscale`, this is considered undefined behaviour in Rust. This is consistent with C and C++ implementations.

## Unsized rules
These types aren't `Sized`, but they need to exist in local variables, and we
need to be able to pass them to, and return them from functions. This means
adding an exception to the rules around returning unsized types in Rust. There
are also some traits (`Copy`) that have a bound on being `Sized`.

We will implement `Copy` for these types within the compiler, without having to
implement the traits when the types are defined.

This RFC also changes the rules so that function return values can be `Copy` or
`Sized` (or both). Once returning of unsized is allowed this part of the rule
would be superseded by that mechanism. It's worth noting that, if any other
types are created that are `Copy` but not `Sized` this rule would apply to
those.

# Drawbacks
[drawbacks]: #drawbacks

## Target Features
One difficulty with this type of approach is typically vector types require a
target feature to be enabled.  Currently, a trait implementation can't enable a
target feature, so some traits can't be implemented correctly without setting `-C
target-feature` via rustc.

However, that isn't a reason to not do this, it's a pain point that another RFC
can address.

# Prior art
[prior-art]: #prior-art

This is a relatively new concept, with not much prior art. C has gone a very
similar way to this by using sizeless incomplete types to represent the SVE
types. Aligning with C here means that most of the documentation that already
exists for the intrinsics in C should still be applicable to Rust.

# Future possibilities
[future-possibilities]: #future-possibilities

## Relaxing restrictions
Some of the restrictions that have been placed on these types could possibly be
relaxed at a later time. This could be done in a backwards compatible way. For
instance, we could perhaps relax the rules around placing these in other
types. It could be possible to allow a struct to contain these types by value,
with certain rules such as requiring them to be the last element(s) of the
struct. Doing this could then allow closures to capture them by value.

## Portable SIMD
For this to work with portable SIMD in the way that portable SIMD is currently
implemented, a const generic parameter would be needed in the
`repr(scalable)`. Creating this dependency would probably be a source of bugs
from an implementation point of view as it would require support for symbols
within the literals.

One potential for having portable SIMD working in its current style would be to have a trait as follows:
```rust
pub trait RuntimeScalable {
    type Increment;
}
```

Which the compiler can use to get the `elements` and `type` from.

The above representation could then be implemented as:
```rust
#[repr(simd, scalable)]
pub struct svfloat32_t {}
impl RuntimeScalable for svfloat32_t {
    type Increment = [f32; 4];
}
```

Given the differences in how scalable SIMD works with current instruction sets it's worth experimenting with
architecture specific implementations first. Therefore portable scalable SIMD should be fully addressed with
another RFC as there should be questions as to how it's going to work with adjusting the active lanes (e.g.
predication).
