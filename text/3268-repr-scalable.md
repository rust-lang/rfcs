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
[SVE](https://developer.arm.com/documentation/102476/latest/) intrinsics from
Arm.

This RFC will focus on the Arm vector extensions, and will use them for all
examples. A large amount of what this RFC covers is emitting the vscale
attribute from LLVM, therefore other scalable vector extensions should work.  In
an LLVM developer meeting it was mentioned that RISC-V would use what's accepted
for Arm SVE for their vector extensions.  \[[see slide
17](https://llvm.org/devmtg/2019-04/slides/TechTalk-Kruppe-Espasa-RISC-V_Vectors_and_LLVM.pdf)\]

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This is an extension to [RFC 1199 SIMD
Infrastructure](https://rust-lang.github.io/rfcs/1199-simd-infrastructure.html).
An understanding of that is expected from the reader of this.

## Brief introduction to scalable vectors

A scalable vector type maps to a CPU register where the size of the register
isn't known during compile time, but it will be a fixed size during
runtime. Certain properties will be known about this during compile time for
example, an architecture might specify there is a minimum size to this register
and that its size must be a multiple of specific amounts, or the maximum
possible size of the register for instance.

## Using SVE intrinsics
A simple example that an end user would be able to write for summing of two
arrays using functions from the ACLE for SVE is shown below:

```rust
unsafe {
    // svcntw returns the actual number of elements that are in a 32 bit element vector
    let step = svcntw() as usize;
    for i in (0..SIZE).step_by(step) {
        let a = data_a.as_ptr().add(i);
        let b = data_b.as_ptr().add(i);
        let c = &mut data_c as *mut f32;
        let c = c.add(i);

        // svwhilelt_b32 generates a mask based on comparing the current index
        // against the SIZE
        let pred = svwhilelt_b32(i as _, SIZE as _);

        // svld1_f32 loads a vector register with the data from address a,
        // zeroing any elements in the vector that are masked out.
        let sva = svld1_f32(pred, a);
        let svb = svld1_f32(pred, b);

        // svadd_f32_m adds a and b, any lanes that are masked out will take the
        // keep value of a
        let svc = svadd_f32_m(pred, sva, svb);

        // svst1_f32 will store the result without accessing any memory
        // locations that are masked out
        svst1_f32(svc, pred, c);
    }
}
```
As can be seen in this example, from a user perspective of writing code for
scalable vectors, it's not all that different from when writing code with a
fixed sized vector. Its arguably easier when working with scalable as you don't
have to worry about being a multiple of your fixed vector size.

## Internal core_arch library details
Existing SIMD types are tagged with a `repr(simd)` and contain an array or
multiple fields to represent the size of the vector. Scalable vectors have a
size known (and constant) at run-time, but unknown at compile time. For this we
propose a new kind of exotic type, denoted by an additional `repr()`. This
additional representation, `scalable`, accepts an integer to determine the
minimum number of elements the vector contains. See the definitions in [the
reference-level explanation](#reference-level-explanation) for more information.

e.g. for a scalable vector f32 type the following could be its representation:

```rust
#[repr(simd, scalable(4))]
pub struct svfloat32_t {
    _ty: [f32],
}
```
`_ty` is purely a type marker, used to get the element type for the LLVM backend.

It's worth noting that currently there are no plans for stabilizing the
`repr_scalable` attribute. This is purely an internal implementation detail,
that could be changed at a later time if required. The only expected use of this
attribute is to be within stdarch.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This will focus on LLVM. No investigation has been done into the alternative
codegen back ends. At the time of writing I believe cranelift doesn't support
scalable vectors ([current
proposal](https://github.com/bytecodealliance/rfcs/pull/19)), and the GCC
backend is not mature enough to be thinking about this.

Most of the complexity of SVE will be handled by LLVM and the `vscale` modifier
that is applied to vector types. Therefore changes for this should be fairly
minimal for Rust. From the LLVM side this is as simple as calling
`LLVMScalableVectorType` rather than `LLVMVectorType`.

For a Scalable Vector Type LLVM takes the form `<vscale x elements x type>`.
* `elements` multiplied by sizeof(`type`) gives the smallest allowed register
  size and the increment size.
* `vscale` is a run time constant that is used to determine the actual vector
  register size.

For example, with Arm SVE the scalable vector register (Z register) size has to
be a multiple of 128 bits and a power of 2 (via a retrospective change to the
architecture), therefore for `f32`, `elements` would always be four, as with the
minimum `vscale` of 1 you would have `1 * 4 * sizeof(f32)` which would give you
the 128 bit minimum register size. At run time `vscale` could then be 1, 2, 4,
8, 16 which would give register sizes of 128, 256, 512, 1024 and 2048. While SVE
now has the power of 2 restriction, `vscale` could be any value providing it
gives a legal vector register size for the architecture.

The scalable representation accepts the number of `elements` rather than the
compiler calculating it, which serves two purposes. The first being that it
removes the need for the compiler to know about the user defined types and how
to calculate the required `element` count. The second being that some of these
scalable types can have different element counts. For instance, the predicates
used in SVE have different element counts in LLVM depending on the types they
are a predicate for.

As mentioned previously `vscale` is a runtime constant. With SVE the vector
length can be changed at runtime (e.g. by a
[prctl()](https://www.kernel.org/doc/Documentation/arm64/sve.txt) call in
Linux). However, since this would require a change to `vscale`, this is
considered undefined behaviour in Rust. This is consistent with C and C++
implementations.

## Properties of these types
* Not `Sized`, but it does exist as a value type.
  * More details will be addressed in the sections below.
  * These can be returned from functions.
  * They can be passed to a function by value, reference or pointer.
  * They can exist as local values on the stack.
* Can be passed by value, reference and pointer.
* The types can't be a static variable.
* These types can be loaded and stored to/from memory for spilling to the stack,
  and to follow any calling conventions.
* Can't be stored in a struct, enum, union or compound type.
  * This includes single field structs with `#[repr(transparent)]`.
  * This also means that closures can't capture them by value.
* Traits can be implemented for these types.


## ABI
With the existing SIMD types in Rust they are currently always passed on the
stack, this was done for ABI reasons as target features at the call site then
don't need to match the function being called. For scalable SIMD the types are
passed in registers. These types cannot exist in a function that
doesn't have the correct target feature for the types. If a call site doesn't
have the feature for the types, we wouldn't even be able to put them onto the
stack, as we wouldn't be able to access the size we need to allocate as the
instruction for that would also be gated under the same target feature.

## Unsized rules
These types aren't `Sized`, but they need to exist in local variables, and we
need to be able to pass them to, and return them from functions. The existing
unsized features (`unsized_locals` and `unsized_fn_params`) can be used for
this. We won't be depending on the features directly but rather using the
existing support within the compiler when the type is a scalable vector type.

Future RFCs might provide alternatives to those features. For instance, if we
have a [hierarchy of size traits](https://github.com/rust-lang/rfcs/pull/3729)
then we could change the bounds check to use different traits, and avoid the
need for those features.

As mentioned these types need to be returned from functions to, this RFC also
changes the rules so that functions can return values that can be copied or are
`Sized`. Once returning of unsized is allowed this part of the rule would be
superseded by that mechanism. It's worth noting that, if any other types are
created that can be copied but are not `Sized` this rule would apply to those.

### Copy
The `Copy` trait has a bound on the type being `Sized`, this is a problem for
these types as they can be copied.

We will implement the ability for these types to be copied within the compiler,
without having to implement the traits when the types are defined. This is
unsound as it will leave two views of the world to the compiler, any future
checks internally with the `Copy` trait will also need to have this special case
logic.

Future RFCs (e.g. [sized
hierarchy](https://github.com/rust-lang/rfcs/pull/3729)) can work on solutions
for allowing the bounds of `Copy` to be changed so that these types can
implement the trait to fix this issue up. The `repr(scalable)` attribute has no
plans to be stabilised, and the actual `repr(scalable)` types will not be
stabilised until this issue is addressed. Allowing the bypassing of the `Copy`
trait for now has value to inform the future design of these types. A lot of
restrictions are currently placed on these types, for the type of code that is
expected to be written this might not be too much of an issue, therefore getting
some real world Rust examples will inform the future relaxing of any
restrictions that are placed on these.

# Prior art
[prior-art]: #prior-art

This is a relatively new concept, with not much prior art. C has gone a very
similar way to this by using sizeless incomplete types to represent the SVE
types. Although using those types in C means that you have to apply a edit to
the C and C++ standards. Aligning with C here means that most of the
documentation that already exists for the intrinsics in C should still be
applicable to Rust. In the future we can move away from this close alignment
with C and start to allow uses in more places.

# Future possibilities
[future-possibilities]: #future-possibilities

## Relaxing restrictions
Some of the restrictions that have been placed on these types could possibly be
relaxed at a later time. This could be done in a backwards compatible way. For
instance, we could perhaps relax the rules around placing these in other
types. It could be possible to allow a struct to contain these types by value.
Doing this could then allow closures to capture them by value. It's possible
that they could exist in arrays as it would just be a runtime multiplication to
get the offset.

This is left for the future as it complicates the stack a lot. LLVM IR mostly
follows C therefore the support for these types in LLVM IR closely matches C. If
we was to attempt to support doing that it would be a lot of codegen work in
rustc to do all the required calculations which then might not lead optimal
code.

As C doesn't have these features, and code is used in production for SVE, I'm
not aware of much demand for those rules to be changed. So for that reason we
should postpone complicating this feature until real world use cases come up.

## Portable SIMD
For this to work with portable SIMD in the way that portable SIMD is currently
implemented, a const generic parameter would be needed in the
`repr(scalable)`. Creating this dependency would probably be a source of bugs
from an implementation point of view as it would require support for symbols
within the literals.

One potential for having portable SIMD working in its current style would be to
have a trait as follows:
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

Given the differences in how scalable SIMD works with current instruction sets
it's worth experimenting with architecture specific implementations
first. Therefore portable scalable SIMD should be fully addressed with another
RFC as there should be questions as to how it's going to work with adjusting the
active lanes (e.g.  predication).
