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
vector. Given that SVE registers don't have a size known at compile time, they can be
represented as a ZST. Therefore to represent an SVE type, we should add an additional `repr()` to say it is scalable
and have a type marker to specify the element type.

This RFC is proposing adding an additional representation, `scalable`, that accepts an integer to determine the number of
elements per granule. See the definitions in [the reference-level explanation](#reference-level-explanation) for more information.

e.g. for a scalable vector f32 type the following could be its representation:

```rust
#[repr(simd, scalable(4))]
#[derive(Clone, Copy)]
pub struct svfloat32_t {
    _ty: [f32; 0],
}
```
`_ty` is purely a type marker, used to get the element type for the LLVM backend.


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

For example, with Arm SVE the scalable vector register (Z register) size has to be a multiple of 128 bits, therefore for `f32`, `elements` would
always be four. At run time `vscale` could be 1, 2, 3, through to 16 which would give register sizes of 128, 256, 384 to 2048.

The scalable representation accepts the number of `elements` rather than the compiler calculating it, which serves
two purposes. The first being that it removes the need for the compiler to know about the user defined types and how to calculate
the required `element` count. The second being that some of these scalable types can have different element counts. For instance,
the predicates used in SVE have different element counts in LLVM depending on the types they are a predicate for.

Within Rust some of the requirements on a SIMD type would need to be relaxed when the scalable attribute is applied, for instance,
currently the type can't be a ZST this check would need to be conditioned on the scalable attribute not being present, and a check
to ensure a scalable vector is a ZST should be added.
Additionally scalable vector types shouldn't be allowed to be stored in a structure, as the layout of that structure wouldn't be known.
Aside from that check, all other SIMD checks should be valid to do with what the type can contain.

This should have minimal impact with other language features, to the same extent that the `repr(simd)` has.


As mentioned previously `vscale` is a runtime constant. With SVE the vector length can be changed at runtime (e.g. by a
[prctl()](https://www.kernel.org/doc/Documentation/arm64/sve.txt) call in Linux), but Rust would consider this undefined
behaviour. This is consistent with C and C++ implementations.

# Drawbacks
[drawbacks]: #drawbacks

One difficulty with this type of approach is typically vector types require a target feature to be enabled.
Currently, a trait implementation can't enable a target feature, so `Clone` can't be implemented without
setting `-C target-feature` via rustc.

However, that isn't a reason to not do this, it's a pain point that another RFC can address.

# Prior art
[prior-art]: #prior-art

This is a relatively new concept, with not much prior art. C has gone a very similar way to this by using a ZST to
represent the SVE types. Aligning with C here means that most of the documentation that already exists for
the intrinsics in C should still be applicable to Rust.

