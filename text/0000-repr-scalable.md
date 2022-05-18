- Feature Name: `repr_scalable`
- Start Date: 2025-07-07
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Extends Rust's existing SIMD infrastructure, `#[repr(simd)]`, with a
complementary scalable representation, `#[repr(scalable)]`, to support scalable
vector types, such as Arm's Scalable Vector Extension (SVE), or RISC-V's Vector
Extension (RVV).

Like the existing `repr(simd)` representation, `repr(scalable)` is internal
compiler infrastructure that will be used only in the standard library to
introduce scalable vector types which can then be stablised. Only the
infrastructure to define these types are introduced in this RFC, not the types
or intrinsics that use it.

This RFC builds on Rust's existing SIMD infrastructure, introduced in
[rfcs#1199: SIMD Infrastructure][rfcs#1199]. It depends on
[rfcs#3729: Hierarchy of Sized traits][rfcs#3729].

SVE is used in examples throughout this RFC, but the proposed features should be
sufficient to enable support for similar extensions in other architectures, such
as RISC-V's V Extension.

# Motivation
[motivation]: #motivation

SIMD types and instructions are a crucial element of high-performance Rust
applications and allow for operating on multiple values in a single instruction.
Many processors have SIMD registers of a known fixed length and provide
intrinsics which operate on these registers. For example, Arm's Neon extension
is well-supported by Rust and provides 128-bit registers and a wide range of
intrinsics.

Instead of releasing more extensions with ever increasing register bit widths,
AArch64 has introduced a Scalable Vector Extension (SVE). Similarly, RISC-V has
a Vector Extension (RVV). These extensions have vector registers whose width
depends on the CPU implementation and bit-width-agnostic intrinsics for
operating on these registers. By using scalale vectors, code won't need to be
re-written using new architecture extensions with larger registers, new types
and intrinsics, but instead will work on newer processors with different vector
register lengths and performance characteristics.

Scalable vectors have interesting and challenging implications for Rust,
introducing value types with sizes that can only be known at runtime, requiring
significant changes to the language's notion of sizedness - this support is
being proposed in the [rfcs#3729].

Hardware is generally available with SVE, and key Rust stakeholders want to be
able to use these architecture features from Rust. In a [recent discussion on
SVE, Amanieu, co-lead of the library team, said][quote_amanieu]:

> I've talked with several people in Google, Huawei and Microsoft, all of whom
> have expressed a rather urgent desire for the ability to use SVE intrinsics in
> Rust code, especially now that SVE hardware is generally available.

Without support in the compiler, leveraging the
[*Hierarchy of Sized traits*][rfcs#3729] proposal, it is not possible to
introduce intrinsics and types exposing the scalable vector support in hardware.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

None of the infrastructure proposed in this RFC is intended to be used directly
by Rust users.

`repr(scalable)` as described later in
[*Reference-level explanation*][reference-level-explanation] is perma-unstable
and exists only enables scalable vector types to be defined in the standard
library. The specific vector types are intended to eventually be stabilised, but
none are proposed in this RFC.

## Using scalable vectors
[using-scalable-vectors]: #using-scalable-vectors

Scalable vector types correspond to vector registers in hardware with unknown
size at compile time. However, it will be a known and fixed size at runtime.
Additional properties could be known during compilation, depending on the
architecture, such as a minimum or maximum size or that the size must be a
multiple of some factor.

As previously described, users will not define their own scalable vector types
and instead use intrinsics from `std::arch`, and this RFC is not proposing any
such intrinsics, just the infrastructure. However, to illustrate how the types
and intrinsics that this infrastructure will enable can be used, consider the
following example that sums two input vectors:

```rust
fn sve_add(in_a: Vec<f32>, in_b: Vec<f32>, out_c: &mut Vec<f32>) {
    let len = in_a.len();
    unsafe {
        // `svcntw` returns the actual number of elements that are in a 32-bit
        // element vector
        let step = svcntw() as usize;
        for i in (0..len).step_by(step) {
            let a = in_a.as_ptr().add(i);
            let b = in_b.as_ptr().add(i);
            let c = out_c as *mut f32;
            let c = c.add(i);

            // `svwhilelt_b32` generates a mask based on comparing the current
            // index against the `len`
            let pred = svwhilelt_b32(i as _, len as _);

            // `svld1_f32` loads a vector register with the data from address
            // `a`, zeroing any elements in the vector that are masked out
            let sva = svld1_f32(pred, a);
            let svb = svld1_f32(pred, b);

            // `svadd_f32_m` adds `a` and `b`, any lanes that are masked out will
            // take the keep value of `a`
            let svc = svadd_f32_m(pred, sva, svb);

            // `svst1_f32` will store the result without accessing any memory
            // locations that are masked out
            svst1_f32(svc, pred, c);
        }
    }
}
```

From a user's perspective, writing code for scalable vectors isn't too different
from when writing code with a fixed sized vector.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Types annotated with the `#[repr(simd)]` attribute contains either an array
field or multiple fields to indicate the intended size of the SIMD vector that
the type represents.

Similarly, a `scalable` repr is introduced to define a scalable vector type.
`scalable` accepts an integer to determine the minimum number of elements the
vector contains. For example:

```rust
#[repr(simd, scalable(4))]
pub struct svfloat32_t { _ty: [f32], }
```

As with the existing `repr(simd)`, `_ty` is purely a type marker, used to get
the element type for the codegen backend.

## Properties of scalable vectors
[properties-of-scalable-vector-types]: #properties-of-scalable-vectors

Scalable vectors are necessarily non-`const Sized` (from [rfcs#3729]) as they
behave like value types but the exact size cannot be known at compilation time.

[rfcs#3729] allows these types to implement `Clone` (and consequently `Copy`) as
`Clone` only requires an implementation of `Sized`, irrespective of constness.

Scalable vector types have some further restrictions due to limitations of the
codegen backend:

- Cannot be stored in compound types (structs, enums, etc) 
    - Including coroutines, so these types cannot be held across
      an await boundary in async functions.
- Cannot be used in arrays
- Cannot be the type of a static variable.

Some of these limitations may be able to be lifted in future depending on what
is supported by rustc's codegen backends.

## ABI
[abi]: #abi

Rust currently always passes SIMD vectors on the stack to avoid ABI mismatches
between functions annotated with `target_feature` - where the relevant vector
register is guaranteed to be present - and those without - where the relevant
vector register might not be present.

However, this approach will not work for scalable vector types as the relevant
target feature must to be present to use the instruction that can allocate the
correct size on the stack for the scalable vector.

Therefore, there is an additional restriction that these types cannot be used in
the argument or return types of functions unless those functions are annotated
with the relevant target feature.

## Target features
[target-features]: #target-features

Similarly to the issues with the ABI of scalable vectors, without the relevant
target features, few operations can actually be performed on scalable vectors -
causing issues for the use of scalable vectors in generic code and with traits.
For example, implementations of traits like `Clone` would not be able to
actually perform a clone, and generic functions that are instantiated with
scalable vectors would during instruction selection in the codegen backend.

When a scalable vector is instantiated into a generic function during
monomorphisation, or a trait method is being implemented for a scalable vector,
then the relevant target feature will be added to the function.

For example, when instantiating `std::mem::size_of_val` with a scalable vector
during monomorphisation, the relevant target feature will be added to `size_of_val`
for codegen.

## Implementing `repr(scalable)`
[implementing-repr-scalable]: #implementing-reprscalable

Implementing `repr(scalable)` largely involves lowering scalable vectors to the
appropriate type in the codegen backend. LLVM has robust support for scalable
vectors and is the default backend, so this section will focus on implementation
in the LLVM codegen backend. Other codegen backends can implement support when
scalable vectors are supported by the backend.

Most of the complexity of SVE is handled by LLVM: lowering Rust's scalable
vectors to the correct type in LLVM and the `vscale` modifier that is applied to
LLVM's vector types.

LLVM's scalable vector type is of the form `<vscale x elements x type>`:

- `elements` multiplied by `size_of::<$ty>` gives the smallest allowed register
  size and the increment size
- `vscale` is a runtime constant that is used to determine the actual vector
  register size

For example, with SVE, the scalable vector register (`Z` register) size has to
be a multiple of 128 bits and a power of 2. Only the value of `elements` can be
chosen by compiler. For `f32`, `elements` must always be four, as with the
minimum `vscale` of one, `1 * 4 * sizeof(f32)` is the 128-bit minimum register
size.

At runtime `vscale` could then be any power of two which would result in
register sizes of 128, 256, 512, 1024 and 2048. `vscale` could be any value
providing it gives a legal vector register size for the architecture.

`repr(scalable)` expects the number of `elements` to be provided rather than
calculating it. This avoids needing to teach the compiler how to calculate the
required `element` count, particularly as some of these scalable types can have
different element counts. For instance, the predicates used in SVE have
different element counts depending on the types they are a predicate for.

While it is possible to change the vector length at runtime using a
[`prctl()`][prctl] call to the kernel, this would require that `vscale` change,
which is unsupported. As Rust cannot prevent users from doing this, it will be
documented as undefined behaviour, consistent with C and C++.

# Drawbacks
[drawbacks]: #drawbacks

- `repr(scalable)` is inherently additional complexity to the language, despite
  being largely hidden from users.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Without support for scalable vectors in the language and compiler, it is not
possible to leverage hardware with scalable vectors from Rust. As extensions
with scalable vectors are available in architectures as either the only or
recommended way to do SIMD, lack of support in Rust would severely limit Rust's
suitability on these architectures compared to other systems programming
languages.

By aligning with the approach taken by C (discussed in the
[*Prior art*][prior-art] below), most of the documentation that already exists
for scalable vector intrinsics in C should still be applicable to Rust.

# Prior art
[prior-art]: #prior-art

There are not many languages with support for scalable vectors:

- SVE in C takes a similar approach as this proposal by using sizeless
  incomplete types to represent scalable vectors. However, sizeless types are
  not part of the C specification and Arm's C Language Extensions (ACLE) provide
  [an edit to the C standard][acle_sizeless] which formally define "sizeless
  types".
- [.NET 9 has experimental support for SVE][dotnet], but as a managed language,
  the design and implementation considerations in .NET are quite different to
  Rust.

[rfcs#3268] was a previous iteration of this RFC.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

There are currently no unresolved questions.

# Future possibilities
[future-possibilities]: #future-possibilities

There are a handful of future possibilities enabled by this RFC:

## General mechanism for target-feature-affected types
[general-mechanism-target-feature-types]: #general-mechanism-for-target-feature-affected-types

A more general mechanism for enforcing that SIMD types are only used in
`target_feature`-annotated functions would be useful, as this would enable SVE
types to have fewer distinct restrictions than other SIMD types, and would
enable SIMD vectors to be passed by-register, a performance improvement.

Such a mechanism would need be introduced gradually to existing SIMD types with
a forward compatibility lint. This will be addressed in a forthcoming RFC.

## Relaxed restrictions
[relaxed-restrictions]: #relaxed-restrictions

Some of the restrictions on these types (e.g. use in compound types) could be
relaxed at a later time either by extending rustc's codegen or leveraging newly
added support in LLVM.

However, as C also has restriction and scalable vectors are nevertheless used in
production code, it is unlikely there will be much demand for those restrictions
to be relaxed.

## Portable SIMD
[portable-simd]: #portable-simd

Given that there are significant differences between scalable vectors and
fixed-length vectors, and that `std::simd` is unstable, it is worth
experimenting with architecture-specific support and implementation initially.
Later, there are a variety of approaches that could be taken to incorporate
support for scalable vectors into Portable SIMD.

[acle_sizeless]: https://arm-software.github.io/acle/main/acle.html#formal-definition-of-sizeless-types
[dotnet]: https://github.com/dotnet/runtime/issues/93095
[prctl]: https://www.kernel.org/doc/Documentation/arm64/sve.txt
[rfcs#1199]: https://rust-lang.github.io/rfcs/1199-simd-infrastructure.html
[rfcs#3268]: https://github.com/rust-lang/rfcs/pull/3268
[rfcs#3729]: https://github.com/rust-lang/rfcs/pull/3729
[quote_amanieu]: https://github.com/rust-lang/rust/pull/118917#issuecomment-2202256754
