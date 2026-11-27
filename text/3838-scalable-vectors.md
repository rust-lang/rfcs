- Feature Name: `rustc_scalable_vector`
- Start Date: 2025-07-07
- RFC PR: [rust-lang/rfcs#3838](https://github.com/rust-lang/rfcs/pull/3838)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Introduces a new attribute, `#[rustc_scalable_vector(N)]`, which can be used to
define new scalable vector types, such as those in Arm's Scalable Vector
Extension (SVE), or RISC-V's Vector Extension (RVV).

`rustc_scalable_vector(N)` is internal compiler infrastructure that will be used
only in the standard library to introduce scalable vector types which can then
be stabilised. Only the infrastructure to define these types are introduced in
this RFC, not the types or intrinsics that use it.

This RFC depends on [rfcs#3729: Hierarchy of Sized traits][rfcs#3729].

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
operating on these registers. By using scalable vectors, code won't need to be
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

`rustc_scalable_vector` as described later in
[*Reference-level explanation*][reference-level-explanation] is perma-unstable
and exists only to enable scalable vector types to be defined in the standard
library. The specific vector types are intended to eventually be stabilised, but
none are proposed in this RFC.

## Using scalable vectors
[using-scalable-vectors]: #using-scalable-vectors

From a user's perspective, writing code for scalable vectors isn't too different
from when writing code with a fixed sized vector. To illustrate how the types
and intrinsics that this infrastructure will enable could be used, consider the
following example that sums two input vectors:

```rust
use std::arch::aarch64::{
    // These intrinsics and types are not proposed by this RFC
    svcntw, svwhilelt_b32, svld1_f32, svadd_f32_m, svst1_f32
};

fn sve_add(in_a: Vec<f32>, in_b: Vec<f32>, out_c: &mut Vec<f32>) {
    assert_eq!(in_a.len(), in_b.len());
    assert_eq!(in_a.len(), out_c.len());
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

            // `svwhilelt_b32` generates a predicate vector that deals with
            // the tail of the iteration - it enables the operations which
            // follow for the first `len` elements overall, but disables
            // the last `len % step` elements in the last iteration
            let pred = svwhilelt_b32(i as _, len as _);

            // `svld1_f32` loads a vector register with the data from address
            // `a`, zeroing any elements in the vector that are masked out
            //
            // Does not access memory for inactive elements
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

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Scalable vectors are similar to fixed length vectors already supported by Rust,
enabling operations to be performed on multiple values at once in a single
instruction. Unlike fixed length vectors, the length of scalable vectors is not
fixed, and intrinsics which operate on scalable vectors are length-agnostic.

Scalable vectors types supported by the `rustc_scalable_vector(N)` attribute can
be thought of as having the form `vscale × N × ty`, where `vscale` is a single,
global, fixed-for-the-runtime-of-the-program "scaling factor" of the CPU.

Vector registers are of the length `vscale × vunit`, with `vunit` being an
architecture-specific value:

- ARM SVE: `vunit` is the minimum length of the vector register in the
  architecture - [128 bits][sve_minlength].
- RISC-V V: `vunit` is the least common multiple of the supported element
  widths - [64 bits][rvv_bitsperblock].

> [!TIP]
>
> While the `vscale` terminology is borrowed from LLVM, `vunit` is invented for
> the purposes of aiding this explanation.

`N` in `rustc_scalable_vector(N)` defines the value of `N` in a scalable vector
type. Any value of `N` is accepted by the attribute and it is the responsibility
of whomever is defining the type to provide a valid value. A correct value for
`N` depends on the purpose of the specific scalable vector type and the
architecture. See
[*Manually-chosen or compiler-calculated element count*][manual-or-calculated-element-count]
for rationale.

In the simplest case, a scalable vector register could be depicted as follows:

```text
 ◁────── vscale x vunit ──────▷ ◁─── vunit ───▷
 ◁────── vscale x ty x N ─────▷ ◁─── ty x N ──▷
┌──────────────────────────────┬───────────────┐
│              ...             │ ty │ ty │ ... │ ← a vector register
└──────────────────────────────┴───────────────┘
```

Scalable vector types contain a single field which is used to determine `ty`:

```rust
#[rustc_scalable_vector(4)]
pub struct svfloat32_t(f32);
```

In the example above, `svfloat32_t` is a scalable vector with a minimum of four
`f32` elements when `vscale = 1` and more when `vscale > 1`. `svfloat32_t` could
be depicted as..

```text
 ◁───────────── vscale x f32 x 4 ─────────────▷ ◁────── f32 x 4 ──────▷
┌──────────────────────────────────────────────┬───────────────────────┐
│                      ...                     │ f32 │ f32 │ f32 │ f32 │ ← `svfloat32_t`
└──────────────────────────────────────────────┴───────────────────────┘
```

..and when running on hardware with `vscale=2`..

```text
 ◁──── 2 x f32 x 4 ────▷ ◁────── f32 x 4 ──────▷
┌───────────────────────┬───────────────────────┐
│ f32 │ f32 │ f32 │ f32 │ f32 │ f32 │ f32 │ f32 │ ← `svfloat32_t`
└───────────────────────┴───────────────────────┘
```

..or `vscale=3`:

```text
 ◁──────────────── 3 x f32 x 4 ────────────────▷ ◁────── f32 x 4 ──────▷
┌───────────────────────┬───────────────────────┬───────────────────────┐
│ f32 │ f32 │ f32 │ f32 │ f32 │ f32 │ f32 │ f32 │ f32 │ f32 │ f32 │ f32 │ ← `svfloat32_t`
└───────────────────────┴───────────────────────┴───────────────────────┘
```

The type marker field is solely used to get the element type for the codegen
backend. It must one of the following types:

- `u8`, `u16`, `u32` or `u64`
- `i8`, `i16`, `i32` or `i64`
- `f16`, `f32` or `f64`
- `bool`

It is not permitted to project into scalable vector types and access the type
marker field.

## Tuples of vectors
[tuples-of-vectors]: #tuples-of-vectors

Structs of scalable vectors are supported, but every element of the struct must
have the same scalable vector type. This will enable definition of "tuple of
vector" types, such as `svfloat32x2_t` below, that are used in some load and
store intrinsics.

```rust
#[rustc_scalable_vector]
pub struct svfloat32x2_t(svfloat32_t, svfloat32_t);
```

```text
◁───────────── vscale x f32 x 4 ─────────────▷ ◁────── f32 x 4 ──────▷
┌──────────────────────────────────────────────┬───────────────────────┐  ┐
│                      ...                     │ f32 │ f32 │ f32 │ f32 │  │
└──────────────────────────────────────────────┴───────────────────────┘  ├─ svfloat32x2_t
┌──────────────────────────────────────────────┬───────────────────────┐  │
│                      ...                     │ f32 │ f32 │ f32 │ f32 │  │
└──────────────────────────────────────────────┴───────────────────────┘  ┘
```

Structs must be still be annotated with `#[rustc_scalable_vector]`, so end-users
cannot define their own structs of scalable vectors. It is not permitted to
project into structs and access the individual vectors.

## Properties of scalable vectors
[properties-of-scalable-vector-types]: #properties-of-scalable-vectors

Scalable vectors are necessarily non-`const Sized` (from [rfcs#3729]) as they
behave like value types but the exact size cannot be known at compilation time.

[rfcs#3729] allows these types to implement `Clone` (and consequently `Copy`) as
`Clone` only requires an implementation of `Sized`, irrespective of constness.

Scalable vector types have some further restrictions due to limitations of the
codegen backend:

- Can only be in the signature of a function if it is annotated with the
  appropriate target feature (see [*ABI*][abi])

- Cannot be stored in compound types (structs, enums, etc)

    - Including coroutines, so these types cannot be held across an await
      boundary in async functions

    - `repr(transparent)` newtypes could be permitted with scalable vectors

    - **Exception:** Scalable vectors can be stored in arrays

    - **Exception:** Scalable vectors can be stored in structs with every
      element of the same type (but only if that struct is annotated with
      `#[rustc_scalable_vector]`)

- Cannot be the type of a static variable

- Cannot be instantiated into generic functions (see
  [*Target features*][target-features])

- Cannot have trait implementations (see [*Target features*][target-features])

  - Including blanket implementations (i.e. `impl<T> Foo for T` is not a valid
    candidate for a scalable vector)

Some of these limitations may be able to be lifted in future depending on what
is supported by rustc's codegen backends or with evolution of the language.

### ABI
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

Any such functions would make a trait containing them dyn-incompatible.

It is permitted to create pointers to function that have scalable vector types
in their arguments or return types, even though function pointers themselves
cannot be annotated as having the target feature. When the function pointer was
created, the user must have made the unsafe promise that it was okay to call the
`#[target_feature]`-annotated function, so it is sound to permit function
pointers.

As scalable vectors will always be passed as immediates, they will therefore
have the same ABI as in C, so should be considered FFI-safe.

### Target features
[target-features]: #target-features

Similarly to the challenges with the ABI of scalable vectors, without the
relevant target features, few operations can actually be performed on scalable
vectors - causing issues for the use of scalable vectors in generic code and
with traits implementations.

For example, implementations of traits like `Clone` would not be able to
actually perform a clone, and generic functions that are instantiated with
scalable vectors would during instruction selection in the codegen backend.

Without a mechanism for a generic function to be able to inherit target features
from its instantiated types or for trait methods to have target features, it is
not possible for these types to be used with generic functions or traits.

See
[*Trait implementations and generic instantiation*][trait-implementations-and-generic-instantiation].

## Changing vector lengths at runtime
[changing-vector-lengths-at-runtime]: #changing-vector-lengths-at-runtime

It is possible to change the vector length at runtime using a
[`prctl()`][prctl] call to the Linux kernel, or via similar mechanisms in other
operating systems.

Doing so would require that `vscale` change, which Rust will not supported.

`prctl` or similar must only be used to set up the vector length for child
processes, not to change the vector length of the current process. As Rust
cannot prevent users from doing this, it will be documented as undefined
behaviour, consistent with C and C++.

## Implementing `rustc_scalable_vector`
[implementing-rustc_scalable_vector]: #implementing-rustc_scalable_vector

Implementing `rustc_scalable_vector` largely involves lowering scalable vectors
to the appropriate type in the codegen backend. LLVM has robust support for
scalable vectors and is the default backend, so this section will focus on
implementation in the LLVM codegen backend. Other backends should be able to
support scalable vectors in Rust once they support scalable vectors in general.

Most of the complexity of scalable vectors are handled by LLVM: lowering Rust's
scalable vectors to the correct type in LLVM and the `vscale` modifier that is
applied to LLVM's vector types.

LLVM's scalable vector type is of the form `<vscale × element_count × type>`.
`vscale` is the scaling factor determined by the hardware at runtime, it can be
any value providing it gives a legal vector register size for the architecture.

For example, a `<vscale × 4 × f32>` is a scalable vector with a minimum of four
`f32` elements and with SVE, `vscale` could then be any power of two which would
result in register sizes of 128, 256, 512, 1024 or 2048 and 4, 8, 16, 32, or 64
`f32` elements respectively.

The `N` in the `#[rustc_scalable_vector(N)]` determines the `element_count` used
in the LLVM type for a scalable vector.

Structs of vectors are lowered to LLVM as struct types containing scalable
vector types. This is supported since the
[*Permit load/store/alloca for struct of the same scalable vector type* LLVM RFC][llvm-rfc-structs].

Arrays of vectors are lowered to LLVM as array types containing scalable vector
types. Arrays of vectors are also supported by LLVM since the
[*Enable arrays of scalable vector types* LLVM RFC][llvm-rfc-arrays].

Tuples in RISC-V's V Extension lower to target-specific types in LLVM rather
than generic scalable vector types, so `rustc_scalable_vector` will not
initially support RVV tuples (see
[*RISC-V Vector Extension's tuple types*][rvv-tuples]).

# Drawbacks
[drawbacks]: #drawbacks

- `rustc_scalable_vector(N)` is inherently additional complexity to the
  language, despite being largely hidden from users.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Without support for scalable vectors in the language and compiler, it is not
possible to leverage hardware with scalable vectors from Rust. As extensions
with scalable vectors are available in architectures as the recommended way to
do SIMD, lack of support in Rust would limit Rust's suitability on these
architectures compared to other systems programming languages.

`rustc_scalable_vector` is preferred over a `repr(scalable)` attribute as there
is existing dissatisfaction with fixed-length vectors being defined using the
`repr(simd)` attribute ([rust#63633]).

By aligning with the approach taken by C (discussed in the
[*Prior art*][prior-art] below), most of the documentation that already exists
for scalable vector intrinsics in C should still be applicable to Rust.

## Manually-chosen or compiler-calculated element count
[manual-or-calculated-element-count]: #manually-chosen-or-compiler-calculated-element-count

`rustc_scalable_vector(N)` expects `N` to be provided rather than calculating
it. Calculating `N` would make this attribute more robust and decrease the
likelihood of it being used incorrectly - even for permanently unstable internal
attributes like `rustc_scalable_vector`, this would be worthwhile if feasible.

In the simplest case, calculating `N` is a simple division: `vunit` (as
previously above) divided by the `element_size`. For example, with ARM SVE,
`vunit=128` so with an `f32` element, `N = 128/32 = 4`; and with RISC-V RVV,
`vunit=64` so with an `f32` element, `N = 64/32 = 2` (assuming `LMUL=1`, but
more on that later).

There are more complicated scalable vector definitions than those presented in
the [*Reference-level explanation*][reference-level-explanation], which
`rustc_scalable_vector` can support, but that would require more complicated
calculations for `N` or architecture-specific knowledge:

1. With Arm SVE, each intrinsic that takes a predicate takes a `svbool_t`
   (`vscale x i1 x 16`). `svbool_t` could have its `N` calculated as above with
   simple division.

   `svbool_t` is used even when the data arguments have fewer elements, e.g. an
   `svfloat32_t` (`vscale x f32 x 4`). `svbool_t` has predicates for sixteen
   lanes but there are only four lanes in the `svfloat32_t` arguments to enable
   or disable. This is slightly unintuitive but matches the definitions of the
   intrinsics in the Arm ACLE.

   Within the definition of those intrinsics, the `svbool_t` is cast to a
   private `svboolN_t` type which has a number of lanes matching the data
   argument (e.g. a `svbool4_t`/`vscale x i1 x 4` for
   `svfloat32_t`/`vscale x f32 x 4`).

   ```text
    ├──────── vscale x i32 x 4 ───────┤ ├──────────── i32 x 4 ────────────┤
   ┌───────────────────────────────────┬───────────────────────────────────┐
   │                ...                │ 0x0000 │ 0x0000 │ 0x0000 │ 0x0000 │
   └───────────────────────────────────┴───────────────────────────────────┘
                     △                   △        △        △        △
                     │       ┌───────────┘        │        │        │
                     │       │     ┌──────────────┘        │        │
                     │       │     │     ┌─────────────────┘        │
               ┌─────┘       │     │     │    ┌─────────────────────┘
   ┌───────────────────────┬───────────────────────┐
   │          ...          │ 0x0 │ 0x0 │ 0x0 │ 0x0 │ + unused space for 12x `i1`s
   └───────────────────────┴───────────────────────┘
   ├── vscale x i1 x 4 ──┤ ├─────── i1 x 4 ──────┤
   ```

   Defining a `svboolN_t` is more complicated than trivial division, requiring
   the attribute accept either arbitrary specification of `N` or a type to
   calculate `N` with, for example:

   ```rust
   // alternative: user-provided arbitrary `N`
   #[rustc_scalable_vector(4)]
   struct svbool4_t(bool);

   // alternative: add `predicate_of` to attribute
   #[rustc_scalable_vector(predicate_of = "u32")]
   struct svbool4_t(bool);

   // alternative: use another field to separate element type and size to use for `N`
   #[rustc_scalable_vector]
   struct svbool4_t(bool, u32);
   ```

2. Similarly, with Arm SVE, the sign extending intrinsics will internally use
   LLVM intrinsics which return vectors with fewer elements than
   `vunit / element_size` (similar to `svboolN_t` but for other types).

   For example, the `svldnt1sb_gather_s64offset_s64` intrinsic wraps the
   `llvm.aarch64.sve.ldnt1.gather.nxv2i8` intrinsic in LLVM. It returns
   `nxv2i8`, which is a `vscale x i8 x 2` that is then cast to `svint64_t`.

   Like in the previous case, `vscale x i8 x 2` cannot be defined without the
   attribute accepting arbitrary specification of `N` or a type to calculate `N`
   with.

3. RISC-V RVV's scalable vectors are quite different from Arm's SVE, while
   sharing the same underlying infrastructure in LLVM.

   SVE's scalable vector types map directly onto LLVM scalable vector types, and
   all of the dynamic parts of the vectors are abstracted by `vscale`:

   ```text
    ├───────── vscale x 128 ────────┤ ├── 128 ──┤
   ┌─────────────────────────────────┬───────────┐
   │               ...               │           │
   └─────────────────────────────────┴───────────┘
   ```

   RVV's scalable vector types have an extra dimensions of flexibility, the
   "register grouping factor" or `LMUL`, and `SEW`:

   - `SEW` is the "selected element width", and corresponds to the size of the
     element type of the vector (`element_size`).

     RVV uses the least common multiple of the supported element types as
     `vunit` so that the overall vector length is `VLEN = vscale * vunit`, which
     is a constant, rather than `VLEN = vscale * element_size`, which is not a
     constant.

   - `LMUL` configures how many vector registers are grouped together to form a
     larger logical vector register. `LMUL` can be 1/8, 1/4, 1/2, 1, 2, 4, or 8.
     Not all `LMUL` values are valid for each type.

   `LMUL` and `SEW` are part of the processor state and are changed by
   compiler-inserted `vsetli` instructions depending on the vector types being
   used.

   `LMUL` is distinct from tuple types, which are a separate variable named
   `NFIELD` (which is not part of the processor state, as with SVE tuples).
   `NFIELD` can be 1, 2, 3, 4, 5, 6, 7 or 8. Not all `NFIELD` values are valid
   for each type.

   Scalable vector types which vary in both `LMUL` and `NFIELD` could be exposed
   to the user (see [RVV Type System Documentation][rvv_typesystem]). For
   example, consider the following types:

   - `vint8mf2_t` has `NFIELD=1`, `LMUL=1/2` and `ty=i8`
   - `vuint32m4_t` has `NFIELD=1`, `LMUL=4` and `ty=i32`
   - `vint16mf4x6_t` has `NFIELD=6`, `LMUL=1/4` and `ty=i16`
   - `vint64m2x3_t` has `NFIELD=3`, `LMUL=2` and `ty=i64`

   This can include types which have different representation but have the same
   `N`:

   - `vint32m1x2_t` has `NFIELD=2`, `LMUL=1` and `ty=i32` (`N=4` elements)
   - `vint32m2_t` has `NFIELD=1`, `LMUL=2` and `ty=i32` (`N=4` elements)

   When `NFIELD=1`, `LMUL=4` and `ty=i64`, four registers are grouped together
   to form a logical vector register, and this has the type
   `<vscale x 4 x i64>`:

   ```text
   ├────────────────────── VLEN ────────────────────┤
    ├── vscale x 64 bits ──┤ ├────── 64 bits ──────┤
                          ┏━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
   ┌──────────────────────╋─┬───────────────────────┐ ┃  ┬
   │          ...         ┃ │          i64          │ ┃  │
   └──────────────────────╋─┴───────────────────────┘ ┃  │
   ┌──────────────────────╋─┬───────────────────────┐ ┃  │
   │          ...         ┃ │          i64          │ ┃  │
   └──────────────────────╋─┴───────────────────────┘ ┃  │ LMUL=4 (vint64m4_t)
   ┌──────────────────────╋─┬───────────────────────┐ ┃  │ vscale x 4 x i64
   │          ...         ┃ │          i64          │ ┃  │
   └──────────────────────╋─┴───────────────────────┘ ┃  │
   ┌──────────────────────╋─┬───────────────────────┐ ┃  │
   │          ...         ┃ │          i64          │ ┃  │
   └──────────────────────╋─┴───────────────────────┘ ┃  ┴
                          ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
   ```

   Similarly, when `NFIELD=1`, `LMUL=4` and `ty=i32`, the smaller element type
   results in each register containing more elements to add up to `vunit` and
   this is repeated across all four registers:

   ```text
   ├────────────────────── VLEN ────────────────────┤
    ├── vscale x 64 bits ──┤ ├────── 64 bits ──────┤
                          ┏━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
   ┌──────────────────────╋─┬───────────┬───────────┐ ┃  ┬
   │          ...         ┃ │    i32    │    i32    │ ┃  │
   └──────────────────────╋─┴───────────┴───────────┘ ┃  │
   ┌──────────────────────╋─┬───────────┬───────────┐ ┃  │
   │          ...         ┃ │    i32    │    i32    │ ┃  │
   └──────────────────────╋─┴───────────┴───────────┘ ┃  │ LMUL=4 (vint32m4_t)
   ┌──────────────────────╋─┬───────────┬───────────┐ ┃  │ vscale x 8 x i32
   │          ...         ┃ │    i32    │    i32    │ ┃  │
   └──────────────────────╋─┴───────────┴───────────┘ ┃  │
   ┌──────────────────────╋─┬───────────┬───────────┐ ┃  │
   │          ...         ┃ │    i32    │    i32    │ ┃  │
   └──────────────────────╋─┴───────────┴───────────┘ ┃  ┴
                          ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
   ```

   It is possible for different scalable vector types with RVV to have the same
   value of `N`, consider `NFIELD=1`, `LMUL=2` and `ty=i32`..

   ```text
   ├────────────────────── VLEN ────────────────────┤
    ├── vscale x 64 bits ──┤ ├────── 64 bits ──────┤
                          ┏━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
   ┌──────────────────────╋─┬───────────┬───────────┐ ┃  ┬
   │          ...         ┃ │    i32    │    i32    │ ┃  │
   └──────────────────────╋─┴───────────┴───────────┘ ┃  │ LMUL=2 (vint32m2_t)
   ┌──────────────────────╋─┬───────────┬───────────┐ ┃  │ vscale x 4 x i32
   │          ...         ┃ │    i32    │    i32    │ ┃  │
   └──────────────────────╋─┴───────────┴───────────┘ ┃  ┴
                          ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
   ```

   ..and `NFIELD=2`, `LMUL=1` and `ty=i32`:

   ```text
   ├────────────────────── VLEN ────────────────────┤
    ├── vscale x 64 bits ──┤ ├────── 64 bits ──────┤
                          ┏━━━━━━━━━━━━━━━━━━━━━━━━━━━┓            ┐
   ┌──────────────────────╋─┬───────────┬───────────┐ ┃  ┬         │
   │          ...         ┃ │    i32    │    i32    │ ┃  │ LMUL=1  │
   └──────────────────────╋─┴───────────┴───────────┘ ┃  ┴         │
                          ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━┛            ├─ vint32m1x2_t
                          ┏━━━━━━━━━━━━━━━━━━━━━━━━━━━┓            │  vscale x 4 x i32
   ┌──────────────────────╋─┬───────────┬───────────┐ ┃  ┬         │
   │          ...         ┃ │    i32    │    i32    │ ┃  │ LMUL=1  │
   └──────────────────────╋─┴───────────┴───────────┘ ┃  ┴         │
                          ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━┛            ┘
   ```

   Despite `vint32m2_t` and `vint32m1x2_t` having the same value of `N`, and
   hence same LLVM type, `vscale x 4 x i32`, the value of `LMUL` in the
   processor state will be different when each are used. In practice, RVV tuples
   lower to target-specific types in LLVM rather than generic scalable vector
   types, so `rustc_scalable_vector` will not initially support RVV tuples (see
   [*RISC-V Vector Extension's tuple types*][rvv-tuples]).

   RVV's scalable vectors cannot be defined without the attribute accepting
   arbitrary specification of `N` or an argument to the attribute to specify the
   `lmul` used:

   ```rust
   // alternative: user-provided arbitrary `N`
   #[rustc_scalable_vector(4)]
   struct vint32m2_t(i32);

   #[rustc_scalable_vector(1)]
   struct vint16mf4_t(i16);

   // alternative: user-provided `LMUL`
   #[rustc_scalable_vector(lmul = "2")]
   struct vint32m2_t(i32);

   #[rustc_scalable_vector(lmul = "1/4")]
   struct vint16mf4_t(i16);
   ```

It is technically possible to calculate `N`, requiring lots of additional
machinery in the `rustc_scalable_vector` attribute, much of which would be
mutually exclusive or produce invalid types with many values of the parameters.

There will be a fixed number of scalable vector types that will be defined in
the standard library alongside their intrinsics and well-tested. It is very
likely that their implementations will be automatically generated. For example,
while not being proposed in this RFC, it is expected Arm SVE will define 55
scalable vector types, exhaustively covering the possible vector types enabled
by the architecture extension (it is assumed that the same will be true for
RISC-V RVV):

- `svbool_t`
- `sv{int8,uint8}{,x2,x3,x4}_t`
- `sv{int16,uint16}{,x2,x3,x4}_t`
- `sv{float32,int32,uint32}{,x2,x3,x4}_t`
- `sv{float64,int64,uint64}{,x2,x3,x4}_t`
- `svbool{2,4,8}_t` for internal use
- `nxv{2,4,8}{i8,u8}` for internal use
- `nxv{2,4}{i16,u16}` for internal use
- `nxv2{i32,u32}` for internal use

Given the complexity required in `rustc_scalable_vector` to be able to calculate
`N`, that it would still be possible to use the attribute incorrectly, that the
attribute is permanently unstable, and the low risk of misuse given the intended
use, this proposal argues that allowing arbitrary specification of `N` is
reasonable.

# Prior art
[prior-art]: #prior-art

[rfcs#3268] was a previous iteration of this RFC.

## Other languages
[prior-art-inother-languages]: #other-languages

There are not many languages with support for scalable vectors:

- SVE in C takes a similar approach as this proposal by using sizeless
  incomplete types to represent scalable vectors. However, sizeless types are
  not part of the C specification and Arm's C Language Extensions (ACLE) provide
  [an edit to the C standard][acle_sizeless] which formally define "sizeless
  types".
- [.NET 9 has experimental support for SVE][dotnet], but as a managed language,
  the design and implementation considerations in .NET are quite different to
  Rust.

## `repr(simd)` and `target_feature`
[prior-art-in-rust]: #reprsimd-and-target_feature

Both `repr(simd)` and `target_feature` attributes were initially proposed in
RFCs:

- **[rfcs#2045]: `target_feature`**
  - Original accepted RFC for `#[target_feature(enable = "..")]`.
  - Of relevance to ABI-affecting target features, there was various discussion
    around the RFC which led to discussion of ABI issues being
    [relegated to an unresolved question][rfcs#2045-abi]
    - At the time of writing the RFC, Portable SIMD types were the only types
      that were considered as potentially having ABI issues, rather than types
      to be used with vendor intrinsics
      - > However, there are types that we might want to add to the language at
        > some point, like portable vector types, for which this \[a lack of ABI
        > changes] is not the case.
        >
        > The behaviour of `#[target_feature]` for those types should be
        > specified in the RFC that proposes to stabilize those types, and this RFC
        > should be amended as necessary.
      - It does not appear that this has been considered for any intrinsic types
        that were later stabilised (e.g.
        [Neon intrinsics on AArch64][rust#90972]) and as such that these types
        can exist in featureless functions is an accident of history

- **[rust#44839]: Tracking issue for RFC 2045: improving `#[target_feature]`**
  - Tracking issue for the `#[target_feature]` parts of RFC 2045
  - This issue hasn't been well-maintained and the description is out-of-date.
    It aims to track both the addition of intrinsics for various architectures
    which use `#[target_feature]` as well as improvements to the
    `#[target_feature]` attribute itself
  - It was last triaged by the language team in Mar 2022, concluding that the
    issue needed an owner

- **[rfcs#2396]: `#[target_feature]` 1.1**
  - Allows specifying `#[target_feature]` functions without making them unsafe,
    still requiring calls to be in unsafe blocks unless the calling function
    also has the target features enabled

- **[rfcs#1199]: `repr_simd`**
  - Proposed `repr(simd)` attribute, applied to structs with multiple fields,
    one for each element in the corresponding vector
    - `repr(simd)` has since changed it was proposed in this RFC
      - It is used for both portable SIMD types and non-portable types, and now
        contains an array (i.e. `[f32; 4]` instead of `(f32, f32, f32, f32)`)
  - Largely focused on portable SIMD, rather than non-portable intrinsics
  - Proposed intrinsics be declared in `extern "platform-intrinsic"` blocks and
    that platform detection be available (though this part was later subsumed by
    [rfcs#2045])

- **[rust#27731]: Tracking issue for SIMD support**
  - Initially tracked implementation of [rfcs#1199], eventually ended up
    tracking `simd_ffi` ([rust#53346]), `repr(simd)` and `core::arch` intrinsics
  - It was later closed and split up into tracking issues for each
    architecture's intrinsics

There are many existing issues and RFCs related to `repr(simd)`; interactions
between SIMD types and target features; and ABI incompatibilities with SIMD
types, surveyed in the sections below.

Many of these issues were related to specific intrinsics on specific platforms
(adding, stabilising or fixing bugs with them), these have been omitted and only
issues that affect generic infrastructure are included.

### Projections into `repr(simd)`
[prior-art-projections]: #projections-into-reprsimd

A handful of issues are related to projections into `repr(simd)` types being
initially permitted..

- **[rust#105439]: ICE due to generating LLVM bitcast vec -> array**
  - Accessing the field of a `repr(simd)` type causes an ICE
  - Fixed by [rust#105583], changing codegen to remove the illegal operation
  - Later addressed holistically by [compiler-team#838] which will ban projecting into `repr(simd)` types
    - Landed in [rust#143833]

- **[rust#137108]: Projecting into non-power-of-two-lanes `repr(simd)` types does the wrong thing**
  - Repeat of [rust#105439]:
    - Accessing the field of a `repr(simd)` type misbehaves
    - Similarly addressed by [compiler-team#838]

- **[rust#113465]: transmute + tuple access + eq on `repr(simd)`'s inner value seems UB to valgrind**
  - Repeat of [rust#105439], except with Portable SIMD type ([portable-simd#339])
    - Accessing the field of a `repr(simd)` type misbehaves
    - Similarly addressed by [compiler-team#838]

These issues are informative for scalable vectors and projection into scalable
vectors will not be supported, as described in
[*Reference-level explanation*][reference-level-explanation].

### Inheritance of `target_feature`
[prior-art-target-feat-inheritance]: #inheritance-of-target_feature

Other issues discussed confusion related to inheritance of `target_feature` to
nested functions and closures...

- **[rust#58729]: target_feature doesn't trickle down to closures and internal fns**
  - `target_feature` attribute doesn't apply to nested functions and closures
    - Interaction with nested functions is expected, these never inherit from their parent
    - Interaction with closures was a bug
      - Prior to [rfcs#2396], closures would have required the ability to be marked as unsafe to support `target_feature`
      - After [rfcs#2396], closures inheriting target features was accepted in [rust#73631] (then implemented in [rust#78231])
        - Interactions with `inline(always)` fixed in [rust#111836]
          - `target_feature` attributes are ignored from `inline(always)`-annotated closures

- **[rust#108338]: closure doesn't seem to inherit the target attributes for codegen purposes**
  - Basically a dupe of [rust#58729] with same resolution

- **[rust#111836]: Fix #\[inline(always)] on closures with target feature 1.1**
  - Allows `#[inline(always)]` to be used with `#[target_feature]` on closures,
    assuming that target features only affect codegen

Scalable vectors will inherit the behaviour described above.

### `repr(simd)` syntax
[prior-art-syntax]: #reprsimd-syntax

There are issues related to how well `repr(simd)` syntax works with other
representation hints and whether a language item would be better:

- **[rust#47103]: What to do about repr(C, simd)?**
  - Unclear what the behaviour of `repr(C, simd)` should be
    - When submitted, a warning of incompatible representation hints was emitted
    - When omitted, a FFI unsafety warning was emitted when SIMD types used in
      FFI
  - Passing vectors as immediates is trickier, later resolved in [rfcs#2574], so
    discussion focused on passing vectors indirectly over the FFI boundary
  - Discussion fizzled out, but with [rust#116558], it may be possible to allow
    `repr(C, simd)`

- **[rust#130402]: When a type is `#[repr(simd)]`, `#[repr(align(N))]` annotations are ignored**
  - `repr(align)` ignored when `repr(simd)` is present
  - Intended to be fixed after [rust#137256] which refactored layout logic
    within the compiler
    - Unclear if the fix happened, but the code from the bug report still has
      the unexpected alignment

- **[rust#63633]: Remove repr(simd) attribute and use a lang-item instead**
  - Suggests using a language item for the `Simd` type (part of Portable SIMD)
    instead of using `repr(simd)`
  - Doesn't address what would happen for non-portable intrinsics that also use
    this infrastructure
  - Various other issues cited as motivation:
    - [rust#18147] used Portable SIMD `f64x2` and found that constant
      initialisers weren't optimised with `-Copt-level=0`
      - Not clear that this applies to types intended for use with
        architecture-specific intrinsics
    - [rust#47103]
      - See above
    - [rust#53346]
      - See below
    - [rust#77529]
      - See below
    - [rust#77866] defines its own `repr(simd)` type and then passes it to an
      LLVM intrinsic binding that has been declared incorrectly
    - [rust#81931] defines its own `repr(simd)` type and finds that it is
      misaligned according to recommendations for achieving best performance

On account of these concerns, scalable vectors use `rustc_scalable_vector`
instead.

### Portable SIMD-specific
[prior-art-portable-simd]: #portable-simd-specific

A handful of architecture-agnostic issues only relate to Portable SIMD:

- **[rust#126217]: What should SIMD bitmasks look like?**
  - Design discussion related to Portable SIMD
    `simd_bitmask`/`simd_bitmask_select` intrinsics
  - Not relevant to architecture-specific scalable vector intrinsics

- **[rust#99211]: fn where clause "constrains trait impls" or something**
  - Writing extension traits with const generics which. apply to Portable SIMD
    types can run into tricky compiler errors related to the type system
  - Only applies to Portable SIMD

- **[rust#77529]: Invalid monomorphisation when `-Clink-dead-code` is used**
  - `repr(simd)` types w/ generics (i.e. Portable SIMD or hand-rolled
    equivalents) can have invalid instantiations with `-Clink-dead-code`

These issues don't apply to scalable vectors.

### Const-initialisation of vectors
[prior-art-const-init]: #const-initialisation-of-vectors

There was a single issue related to const-initialisation of non-portable vector
types:

- **[rust#48745]: Provide a way to const-initialise vendor-specific vector types**
  - Initialisation of fixed length vectors was not possible in a const context
    for non-portable SIMD
  - `mem::transmute` being made constant has addressed this issue

This issue doesn't apply to scalable vectors as they are inherently non-const.

### `target_feature` ABI

There have been well-documented issues with the ABI of fixed-length SIMD
vectors, many of which apply to scalable vectors too, but are harder to resolve:

- **[rust#44367]: `repr(simd)` is unsound**
  - `repr(simd)` types in functions with different target features enabled can
    have different ABIs
  - Fixed by passing SIMD types indirectly in [rust#47743]

- **[rust#53346]: `repr(simd)` is unsound in C FFI**
  - Same issue as in [rust#44367] but only with `extern "C"` functions where the
    Rust ABI does not apply
  - Later fixed by [rust#116558]

- **[rust#87438]: future-incompat: use of SIMD types aren't gated properly**
  - Calling a `extern "C"` function with an SIMD vector type in a `repr(C)` or
    `repr(transparent)` struct doesn't error
  - Later fixed by [rust#116558]

- **[rfcs#2574]: `simd_ffi`**
  - Permits calls to `extern "C"` functions with SIMD types so long as those
    functions have the appropriate `target_feature` attribute
  - Never fully implemented until [rust#116558] effectively did so

- **[rust#131800]: Figure out which target features are required for which SIMD size**
  - As part of [rust#116558], solicited input in determining which target
    features were required for a given vector length so that the lint could
    check for those

- **[rust#133146]: How should we handle dynamic vector ABIs?**
  - Follow-up to [rust#131800]:
    - Existing ABI compatibility checks rely on the length of the vector and the
      architecture to identify an appropriate target feature that must be
      enabled, but this approach does not scale to scalable vectors

- **[rust#133144]: How should we handle matrix ABIs?**
  - Follow-up to [rust#131800]:
    - Same as [rust#133146] but for matrix extensions
    - Out-of-scope for this RFC

- **[rust#116558]: The `extern "C"` ABI of SIMD vector types depends on target features (tracking issue for `abi_unsupported_vector_types` future-incompatibility lint)**
  - Identified ABI incompatibility when calling `extern "C"` functions that used SIMD types
    - Rust passes SIMD types indirectly for functions with and without
      `target_feature` annotations in its ABI. `extern "C"` functions take SIMD
      types as immediates
    - Calls from annotated functions to `extern "C"` could use immediates, but
      calls from non-annotated functions could not. Rust did not prevent calls
      from non-annotated functions.
  - A `abi_unsupported_vector_types` future-incompatibility lint was introduced
    to enforce that `extern "C"` functions could not have SIMD types in their
    signatures without the appropriate target feature being enabled
    - The lint has since been removed and replaced with a hard error
    - It only triggers when such a function is called

- **[rust#132865]: Support calling functions with SIMD vectors that couldn't be used in the caller**
  - Follow-up to [rust#116558]
  - There are valid calls to `extern "C"` functions which take SIMD types that
    are not currently accepted, such as checking for the presence of the target
    feature and then calling the `extern "C"` function with a newly created
    vector
  - It is hard to support this as it is not possible to generate a call with a
    specific ABI without annotating the entire containing function as having the
    target feature ([llvm#70563])
    - This limitation also causes similar issues with inlining ([rust#116573])

- **[Pre-RFC: Fixing ABI for SIMD types][pre_rfc_simd]**
  - Proposes requiring appropriate target features be enabled when a x86 SIMD
    type is used in a function signature
    - Written primarily considering x86 SIMD
    - Considers both globally-enabled target features (e.g. `-Ctarget-feature`
      or default features from target specification) and per-function-enabled
      target features (`#[target_feature]`)
    - Proposes generating shims to translate between ABIs when calling annotated
      functions with SIMD type arguments from non-annotated functions
      - Avoids breakage in cases similar to [rust#132865] but between annotated
        and non-annotated functions, rather than just Rust ABI to non-Rust ABI
    - Errors will be emitted for function pointers based on the target features
      of the caller
  - Prompted by discussion in [rust#116558]
  - Never progressed to being a submitted RFC
  - Discussed [on Zulip][pre_rfc_simd_zulip]
    - How does the pre-RFC interact with Portable SIMD efforts?
      - The inherent portability of these types means that they will need a
        matching featureful and featureless ABI. It is suggested that this be
        the current indirect ABI, but this isn't seen as desirable - ABI shims
        or per-target-feature monomorphisation is to be explored
    - Should there be a difference in codegen for calls to function items vs
      function pointers (e.g. use of a shim)?
      - Suggestion that an ABI shim be used for function pointers rather than
        requiring target feature on functions with the call
    - Is there a proper featureless ABI for x86 SIMD types?
      - Yes, details in thread
    - Should these changes also apply to `extern "Rust"`?
      - Mixed opinions - enables use of performant ABI, larger breaking change
      - Concern that doing this jeopardizes the entire proposal and that Rust is
        stuck with the current behaviour

        > I still don't like the idea I'm forced to use an FFI calling
        > convention in pure rust code because the default is fundamentally too
        > slow

        > it's a tradeoff. should the default be portable or fast. I dont think
        > there is an obvious right answer here. might be worth digging out the
        > history that led to the current situation -- possibly this decision
        > has been made in the past, in favor of "portable", and that's why the
        > ABI works the way it does?
      - Could use the performant ABI when global target features have feature
        enabled
      - References later design meeting ([lang-team#235])

- **[lang-team#235]: Design meeting: resolve ABI issues around target-feature**
  ([meeting notes][lang-team#235-notes])
  - Proposes property that functions with the same signature will always have
    the same ABI (i.e. that target features will not be considered in the ABI)
    and three possible fixes:
    - Track target features as part of function signatures, which is hard to do
      without changing function pointer syntax
      - Discussed briefly but not proposed due to concerns regarding breaking
        change and expectation that function pointer syntax would need
        changed/extended, and that it introduces a new semver hazard (adding a
        SIMD type field)
      - Suggested that if this route were taken then allowing target features
        in extern blocks would be desirable and passing SIMD types using
        registers could be considered
      - Did not discuss challenges related to trait methods and generic
        functions
      - References [rust#111836]
    - Define an ABI which does not depend on target features
      - i.e. as the Rust ABI today with indirect passing of SIMD types
    - Reject declaring/calling functions with target-feature-requiring types
      when the ABI target feature is not available/enabled
      - References [Pre-RFC: Fixing ABI for SIMD types][pre_rfc_simd], proposing
        a variant of the RFC:
        - Instead of applying to all ABIs (as in the pre-RFC) and using the
          performant calling convention, it would only to non-Rust ABIs (e.g.
          `extern "C"`) and would be based on a size of the vector to feature
          mapping (rather than annotating types w/ the required features)
          - This narrowly fixes the soundness issue and is the basis for the
            currently implemented future incompatibility warning
    - Reject enabling/disabling certain ABI-affecting features
      - Discusses this as a solution for ABI issues relating to floats
  - During the meeting, various points were discussed:
    - Should it be possible to declare (non-Rust ABI) functions which take SIMD
      vectors as long as there aren't calls?
      - Intended to reduce potential opportunities breakage.
    - How plausible is it to include ABI-affecting target features in function
      pointer types?
      - It is suggested that this information could be smuggled through the ABI
        name: e.g. `extern "Rust+avx"`
      - Concern that this is a wider breaking change and would require treating
        ABIs specially or would require shims
      - Feeling that this should be possible but not discussed further as an
        immediate solution was desired to resolve unsoundness
    - There was consensus that crater runs were needed to gather data
  - After the meeting, the language team concluded:
    > We discussed this question in the T-lang design meeting on 2023-12-20. The
    > consensus was generally in favor of fixing this, and we were interested in
    > seeing more work in this direction. While we considered various ways that
    > this could be fixed, we were supportive of finding the most minimal,
    > simplest way to fix this as the first step, assuming that such an approach
    > proves to be feasible. We'll need to see at least the results of a crater
    > run and further analysis to confirm that feasibility.
    - It was explicitly noted in the notes that the language team didn't want to
      rule out considering target features as part of the ABI:

      > > Track the target features in the function signature. This would
      > > basically mean that function pointers now have to also list the set of
      > > ABI-relevant target features that were enabled. This would be a rather
      > > fundamental change requiring new function pointer syntax, and hard to
      > > do without breaking code, so we mention it only for completeness'
      > > sake.
      >
      > I don't think we should rule this out for the future. We've already
      > talked about being able to track calling-convention ABI (extern "C" vs
      > other ABIs) in function pointers somehow, so that we can safely track
      > which kind of function we have. We've also talked about having this work
      > in generics somehow, so that the Fn traits have a (defaulted) parameter
      > for ABI or similar, and the monomorphization of a call will call the
      > right ABI.

See [*ABI*][abi] for discussion of these challenges as they apply to scalable
vectors.

### Multiversioning and effects
[prior-art-future]: #multiversioning-and-effects

There are ongoing efforts related to improving Rust's SIMD support:

- **[rust-project-goals#261]: Nightly support for ergonomic SIMD multiversioning**
  - Generating efficient code for specific SIMD ISAs requires `target_feature`
    attributes on functions, which isn't particularly ergonomic
    - Need to do runtime checks then dispatch to functions with target features
      - Must be repeated when leaving and entering these functions
    - Intermediate functions use `inline(always)` to avoid having to have
      different versions for each target feature, which impacts code size
  - Various solutions have been proposed - witness types carrying target feature
    information, inherited target features from callers, features being const
    generic arguments
  - Goal aims to explore design space and experiment

- **[lang-team#309]: SIMD multiversioning** ([pre-read][lang-team#309-notes])
  - Design meeting proposal, has not yet taken place
  - Compares two related proposals that address some of the problems with
    multiversioning
    - Ideas similar to [rfcs#3525]
      - In brief: attaching target features to types, introduce traits that
        abstract over common operations, functions are generic over those traits
        and when instantiated with a function-carrying type, inherit the target
        feature and use the trait methods to do SIMD operations
    - Ideas similar to [unopened contextual target features RFC][rfcs-contextual]
        - In brief: `#[target_features(caller)]` which causes a function to
          inherit the features of the caller
        - Expands on this with a `#[target_features(generic)]` attribute which
          takes target features from the first const generic argument of the
          function (i.e. a `const FEATURES: str`)
  - Both ideas have many open design questions

- **[rust#143352]: Tracking issue for Effective Target Features**
  - [Initial proposal][rust#143352-proposal] aims to experiment with SIMD
    multiversioning based on the effect model used with const traits
  - In brief: traits can be defined as having a target feature effect,
    implementations of those traits define the target feature that is enabled by
    the effect, bounds on the trait in functions will enable the target feature
    from the impl

- **[lang-team#317]: Design meeting: "Marker effects"** ([meeting notes][lang-team#317-notes])
  - Discusses the findings from investigations into keyword generics and the
    implementation of const traits, proposing a categorisation of effects and a
    subset to focus on initially
  - Briefly discusses that effects overlap with the SIMD multiversioning efforts

These efforts are followed with interest as they may synergise well with
resolving the similar challenges that scalable vectors face. See
[*Trait implementations and generic instantiation*][trait-implementations-and-generic-instantiation].

# Unresolved questions
[unresolved-questions]: #unresolved-questions

There is one outstanding unresolved question for scalable vectors:

- How to support trait implementations and generic instantiation for scalable vectors?

  - See [*Target features*][target-features] and
    [*Trait implementations and generic instantiation*][trait-implementations-and-generic-instantiation]

# Future possibilities
[future-possibilities]: #future-possibilities

There are a handful of future possibilities enabled by this RFC - relaxing
restrictions, architecture-agnostic use or extending the feature to support more
features of the architecture extensions:

## Trait implementations and generic instantiation
[trait-implementations-and-generic-instantiation]: #trait-implementations-and-generic-instantiation

Improvements to the language's `target_feature` infrastructure could enable the
restrictions on trait implementations and generic instantiation to be lifted:

- Some variety of [rfcs#3820: `target_feature_traits`][rfcs#3280] could help
  traits be implemented on scalable vectors

- Efforts to integrate target features with the effect system ([rust#143352])
  may help enable generic instantiation of scalable vectors

  - Any mechanism that could be applied to scalable vector types could also be
    used to enforce that existing SIMD types are only used in
    `target_feature`-annotated functions, which would enable fixed-length
    vectors to be passed as immediates, improving performance

- It may be possible to support scalable vector types without the target feature
  being enabled by using an indirect ABI similarly to fixed length vectors.

  - This would enable these restrictions to be lifted and for scalable vector
    types to be the same as fixed length vectors with respect to interactions
    with the `target_feature` attribute.

    - As with fixed length vectors, it would still be desirable for them to
      avoid needing to be passed indirectly between annotated functions, but
      this could be addressed in a follow-up.

  - Experimentation is required to determine if this is feasible.

## Compound types
[compound-types]: #compound-types

The restriction that scalable vectors cannot be used in compound types could be
relaxed at a later time either by extending rustc's codegen or leveraging newly
added support in LLVM.

However, as C also has this restriction and scalable vectors are nevertheless
used in production code, it is unlikely there will be much demand for those
restrictions to be relaxed in LLVM.

## RISC-V Vector Extension's tuple types
[rvv-tuples]: #risc-v-vector-extensions-tuple-types

As explained in
[*Manually-chosen or compiler-calculated element count*][manual-or-calculated-element-count],
there is a distinction in RVV between vectors which would have the same `N` in a
scalable vector type, but which vary in `LMUL` and `NFIELD`.

For example, `vint32m2_t` and `vint32m1x2_t`, if lowered to scalable vector
types in LLVM, would both be `<vscale x 4 x i32>`.

RVV's tuple types need to be lowered to target-specific types in the backend
which is out-of-scope of this general infrastructure for scalable vectors.

## Portable SIMD
[portable-simd]: #portable-simd

Given that there are significant differences between scalable vectors and
fixed-length vectors, and that `std::simd` is unstable, it is worth
experimenting with architecture-specific support and implementation initially.
Later, there are a variety of approaches that could be taken to incorporate
support for scalable vectors into Portable SIMD.

[acle_sizeless]: https://arm-software.github.io/acle/main/acle.html#formal-definition-of-sizeless-types
[compiler-team#838]: https://github.com/rust-lang/compiler-team/issues/838
[dotnet]: https://github.com/dotnet/runtime/issues/93095
[lang-team#235-notes]: https://hackmd.io/Dnd0ZIN6RjqbRlEs2GWE5Q
[lang-team#235]: https://github.com/rust-lang/lang-team/issues/235
[lang-team#309-notes]: https://hackmd.io/@veluca93/simd-multiversioning
[lang-team#309]: https://github.com/rust-lang/lang-team/issues/309
[lang-team#317-notes]: https://hackmd.io/xydafCtMQ1aqUbm6wqmEmA?view
[lang-team#317]: https://github.com/rust-lang/lang-team/issues/317
[llvm-rfc-arrays]: https://discourse.llvm.org/t/rfc-enable-arrays-of-scalable-vector-types/72935
[llvm-rfc-structs]: https://discourse.llvm.org/t/rfc-ir-permit-load-store-alloca-for-struct-of-the-same-scalable-vector-type/69527
[llvm#70563]: https://github.com/llvm/llvm-project/issues/70563
[portable-simd#339]: https://github.com/rust-lang/portable-simd/issues/339
[prctl]: https://www.kernel.org/doc/Documentation/arm64/sve.txt
[pre_rfc_simd_zulip]: https://rust-lang.zulipchat.com/#narrow/channel/213817-t-lang/topic/Pre-RFC.20discussion.3A.20Forbidding.20SIMD.20types.20w.2Fo.20features/near/399174036
[pre_rfc_simd]: https://hackmd.io/@chorman0773/SJ1rZPWZ6
[quote_amanieu]: https://github.com/rust-lang/rust/pull/118917#issuecomment-2202256754
[rfcs-contextual]: https://github.com/calebzulawski/rfcs/blob/contextual-target-features/text/0000-contextual-target-features.md
[rfcs#1199]: https://rust-lang.github.io/rfcs/1199-simd-infrastructure.html
[rfcs#2045-abi]: https://rust-lang.github.io/rfcs/2045-target-feature.html#how-do-we-handle-abi-issues-with-portable-vector-types
[rfcs#2045]: https://github.com/rust-lang/rfcs/pull/2045
[rfcs#2396]: https://github.com/rust-lang/rfcs/pull/2396
[rfcs#2574]: https://github.com/rust-lang/rfcs/pull/2574
[rfcs#3268]: https://github.com/rust-lang/rfcs/pull/3268
[rfcs#3280]: https://github.com/rust-lang/rfcs/pull/3280
[rfcs#3525]: https://github.com/rust-lang/rfcs/pull/3525
[rfcs#3729]: https://github.com/rust-lang/rfcs/pull/3729
[rust-project-goals#261]: https://github.com/rust-lang/rust-project-goals/issues/261
[rust#105439]: https://github.com/rust-lang/rust/issues/105439
[rust#105583]: https://github.com/rust-lang/rust/issues/105583
[rust#108338]: https://github.com/rust-lang/rust/issues/108338
[rust#111836]: https://github.com/rust-lang/rust/issues/111836
[rust#113465]: https://github.com/rust-lang/rust/issues/113465
[rust#116558]: https://github.com/rust-lang/rust/issues/116558
[rust#116573]: https://github.com/rust-lang/rust/issues/116573
[rust#126217]: https://github.com/rust-lang/rust/issues/126217
[rust#130402]: https://github.com/rust-lang/rust/issues/130402
[rust#131800]: https://github.com/rust-lang/rust/issues/131800
[rust#132865]: https://github.com/rust-lang/rust/issues/132865
[rust#133144]: https://github.com/rust-lang/rust/issues/133144
[rust#133146]: https://github.com/rust-lang/rust/issues/133146
[rust#137108]: https://github.com/rust-lang/rust/issues/137108
[rust#137256]: https://github.com/rust-lang/rust/issues/137256
[rust#143352-proposal]: https://hackmd.io/M5ZAoRqSTb27oLBuEpByIA
[rust#143352]: https://github.com/rust-lang/rust/issues/143352
[rust#143833]: https://github.com/rust-lang/rust/issues/143833
[rust#18147]: https://github.com/rust-lang/rust/issues/18147
[rust#27731]: https://github.com/rust-lang/rust/issues/27731
[rust#44367]: https://github.com/rust-lang/rust/issues/44367
[rust#44839]: https://github.com/rust-lang/rust/issues/44839
[rust#47103]: https://github.com/rust-lang/rust/issues/47103
[rust#47743]: https://github.com/rust-lang/rust/issues/47743
[rust#48745]: https://github.com/rust-lang/rust/issues/48745
[rust#53346]: https://github.com/rust-lang/rust/issues/53346
[rust#58729]: https://github.com/rust-lang/rust/issues/58729
[rust#63633]: https://github.com/rust-lang/rust/issues/63633
[rust#73631]: https://github.com/rust-lang/rust/issues/73631
[rust#77529]: https://github.com/rust-lang/rust/issues/77529
[rust#77866]: https://github.com/rust-lang/rust/issues/77866
[rust#78231]: https://github.com/rust-lang/rust/issues/78231
[rust#81931]: https://github.com/rust-lang/rust/issues/81931
[rust#87438]: https://github.com/rust-lang/rust/issues/87438
[rust#90972]: https://github.com/rust-lang/rust/issues/90972
[rust#99211]: https://github.com/rust-lang/rust/issues/99211
[rvv_bitsperblock]: https://github.com/llvm/llvm-project/blob/837b2d464ff16fe0d892dcf2827747c97dd5465e/llvm/include/llvm/TargetParser/RISCVTargetParser.h#L51
[rvv_typesystem]: https://github.com/riscv-non-isa/rvv-intrinsic-doc/blob/main/doc/rvv-intrinsic-spec.adoc#type-system
[sve_minlength]: https://developer.arm.com/documentation/102476/0101/Introducing-SVE#:~:text=a%20minimum%20of%20128%20bits
