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

- Cannot be used in arrays

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

## Implementing `rustc_scalable_vector`
[implementing-rustc_scalable_vector]: #implementing-rustc_scalable_vector

Implementing `rustc_scalable_vector` largely involves lowering scalable vectors
to the appropriate type in the codegen backend. LLVM has robust support for
scalable vectors and is the default backend, so this section will focus on
implementation in the LLVM codegen backend. Other codegen backends can implement
support when scalable vectors are supported by the backend.

Most of the complexity of SVE is handled by LLVM: lowering Rust's scalable
vectors to the correct type in LLVM and the `vscale` modifier that is applied to
LLVM's vector types.

LLVM's scalable vector type is of the form `<vscale × element_count × type>`.
`vscale` is the scaling factor determined by the hardware at runtime, it can be
any value providing it gives a legal vector register size for the architecture.

For example, a `<vscale × 4 × f32>` is a scalable vector with a minimum of four
`f32` elements and with SVE, `vscale` could then be any power of two which would
result in register sizes of 128, 256, 512, 1024 or 2048 and 4, 8, 16, 32, or 64
`f32` elements respectively.

The `N` in the `#[rustc_scalable_vector(N)]` determines the `element_count` used
in the LLVM type for a scalable vector.

While it is possible to change the vector length at runtime using a
[`prctl()`][prctl] call to the kernel, this would require that `vscale` change,
which is unsupported. `prctl` must only be used to set up the vector length for
child processes, not to change the vector length of the current process. As Rust
cannot prevent users from doing this, it will be documented as undefined
behaviour, consistent with C and C++.

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
with scalable vectors are available in architectures as either the only or
recommended way to do SIMD, lack of support in Rust would severely limit Rust's
suitability on these architectures compared to other systems programming
languages.

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

3. Also with Arm SVE, some load and store intrinsics take tuples of vectors,
   such as `svfloat32x2_t`:

   ```text
    ◁───────────── vscale x f32 x 4 ─────────────▷ ◁────── f32 x 4 ──────▷
   ┌──────────────────────────────────────────────┬───────────────────────┐  ┐
   │                      ...                     │ f32 │ f32 │ f32 │ f32 │  │
   └──────────────────────────────────────────────┴───────────────────────┘  ├─ svfloat32x2_t
   ┌──────────────────────────────────────────────┬───────────────────────┐  │  vscale x f32 x 8
   │                      ...                     │ f32 │ f32 │ f32 │ f32 │  │
   └──────────────────────────────────────────────┴───────────────────────┘  ┘
   ```

   These types are the opposite of the previous complicating case, containing
   more elements than `vunit / element_size`. These use two or more registers to
   represent the vector.

   `vscale x f32 x 8` cannot be defined without the attribute accepting
   arbitrary specification of `N` or an argument to the attribute to specify the
   number of registers used:

   ```rust
   // alternative: user-provided arbitrary `N`
   #[rustc_scalable_vector(8)]
   struct svfloat32x2_t(f32);

   // alternative: add `tuple_of` to attribute
   #[rustc_scalable_vector(tuple_of = "2")] // either `1` (default), `2`, `3` or `4`
   struct svfloat32x2_t(f32);
   ```

4. RISC-V RVV's scalable vectors are quite different from Arm's SVE, while
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
    vectors to be passed by-register, improving performance

## Compound types
[compound-types]: #compound-types

The restriction that scalable vectors cannot be used in compound types could be
relaxed at a later time either by extending rustc's codegen or leveraging newly
added support in LLVM.

However, as C also has thus restriction and scalable vectors are nevertheless
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
[dotnet]: https://github.com/dotnet/runtime/issues/93095
[prctl]: https://www.kernel.org/doc/Documentation/arm64/sve.txt
[quote_amanieu]: https://github.com/rust-lang/rust/pull/118917#issuecomment-2202256754
[rfcs#1199]: https://rust-lang.github.io/rfcs/1199-simd-infrastructure.html
[rfcs#3268]: https://github.com/rust-lang/rfcs/pull/3268
[rfcs#3729]: https://github.com/rust-lang/rfcs/pull/3729
[rfcs#3280]: https://github.com/rust-lang/rfcs/pull/3280
[rust#63633]: https://github.com/rust-lang/rust/issues/63633
[rust#143352]: https://github.com/rust-lang/rust/issues/143352
[rvv_bitsperblock]: https://github.com/llvm/llvm-project/blob/837b2d464ff16fe0d892dcf2827747c97dd5465e/llvm/include/llvm/TargetParser/RISCVTargetParser.h#L51
[rvv_typesystem]: https://github.com/riscv-non-isa/rvv-intrinsic-doc/blob/main/doc/rvv-intrinsic-spec.adoc#type-system
[sve_minlength]: https://developer.arm.com/documentation/102476/0101/Introducing-SVE#:~:text=a%20minimum%20of%20128%20bits
