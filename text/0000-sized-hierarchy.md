- Feature Name: `sized_hierarchy`
- Start Date: 2024-09-30
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

All of Rust's types are either *sized*, which implement the `Sized` trait and
have a statically known size during compilation, or *unsized*, which do not
implement the `Sized` trait and are assumed to have a size which can be computed
at runtime. However, this dichotomy misses two categories of type - types whose
size is unknown during compilation but is a runtime constant, and types
whose size can never be known. Supporting the former is a prerequisite to stable
scalable vector types and supporting the latter is a prerequisite to unblocking
extern types.

This RFC necessarily depends on the experimental feature
[`const Trait`][goal_const_traits] and is written assuming familiarity with that
feature.

# Background
[background]: #background

Rust has the [`Sized`][api_sized] marker trait which indicates that a type's size
is statically known at compilation time. `Sized` is a trait which is automatically
implemented by the compiler on any type that has a statically known size. All type
parameters have a default bound of `Sized` and `?Sized` syntax can be used to remove
this bound.

There are two functions in the standard library which can be used to get a size,
[`std::mem::size_of`][api_size_of] and [`std::mem::size_of_val`][api_size_of_val]:

```rust=
pub const fn size_of<T>() -> usize {
    /* .. */
}

pub fn size_of_val<T>(val: &T) -> usize
where
    T: ?Sized,
{
    /* .. */
}
```

Due to `size_of_val::<T>`'s `T: ?Sized` bound, it is expected that the size of a
`?Sized` type can be computable at runtime, and therefore a `T` with `T: ?Sized`
cannot be a type with no size.

## Terminology
[terminology]: #terminology

In the Rust community, "unsized" and "dynamically sized" are often used
interchangeably to describe any type that does not implement `Sized`. This is
unsurprising as any type which does not implement `Sized` is necessarily
"unsized" and currently the only types this description captures are those which
are dynamically sized.

In this RFC, a distinction is made between "unsized" and "dynamically sized"
types. Unsized types is used to refer only to those which have no known
size/alignment, such as those described by [the extern types
RFC][rfc_extern_types]. Dynamically-sized types describes those types whose size
cannot be known statically at compilation time and must be computed at runtime.

Within this RFC, no terminology is introduced to describe all types which do not
implement `Sized` in the same sense as "unsized" is colloquially used.

Throughout the RFC, the following terminology will be used:

- "`Trait` types" will be used to refer to those types which implement `Trait`
  and all of its supertraits but none of its subtraits. For example, a `ValueSized`
  type would be a type which implements `ValueSized`, and `Pointee`, but not
  `Sized`. `[usize]` would be referred to as a "`ValueSized` type".
- "Runtime-sized" types will be used those types whose size is a runtime constant
  and unknown at compilation time. These would include the scalable vector types
  mentioned in the motivation below, or those that implement `Sized` but not
  `const Sized` in the RFC.
- The bounds on the generic parameters of a function may be referred to simply
  as the bounds on the function (e.g. "the caller's bounds").

# Motivation
[motivation]: #motivation

Introducing a hierarchy of `Sized` and elaborating on the implications of constness
on `Sized` traits resolves blockers for other RFCs which have had significant interest.

## Runtime-sized types and scalable vector types
[runtime-sized-types-and-scalable-vector-types]: #runtime-sized-types-and-scalable-vector-types

Rust already supports [SIMD][rfc_simd] (*Single Instruction Multiple Data*),
which allows operating on multiple values in a single instruction. Processors
have SIMD registers of a known, fixed length and a variety of intrinsics
which operate on these registers. For example, x86-64 introduced 128-bit SIMD
registers with SSE, 256-bit SIMD registers with AVX, and 512-bit SIMD registers
with AVX-512, and Arm introduced 128-bit SIMD registers with Neon.

As an alternative to releasing SIMD extensions with greater bit widths, Arm and
RISC-V have vector extensions (SVE/Scalable Vector Extension and the "V" Vector
Extension/RVV respectively) where the bit width of vector registers depends on the
CPU implementation, and the instructions which operate these registers are bit
width-agnostic.

As a consequence, these types are not `Sized` in the Rust sense, as the size of
a scalable vector cannot be known during compilation, but is a runtime constant.
For example, the size of these types could be determined by inspecting the value
in a register - this is not available at compilation time and the value may
differ between any given CPU implementation. Both SVE and RVV have mechanisms to
change the system's vector length (up to the maximum supported by the CPU
implementations) but this is not supported by the proposed ABI for these types.

However, despite not implementing `Sized`, these are value types which should
implement `Copy` and can be returned from functions, can be variables on the
stack, etc. These types should implement `Copy` but given that `Copy` is a
supertrait of `Sized`, they cannot be `Copy` without being `Sized`, and
they aren't `Sized`.

Introducing a `const Sized` trait will enable `Copy` to be implemented for
those types whose size is a runtime constant to function correctly without
special cases in the type system. See
[rfcs#3268: Scalable Vectors][rfc_scalable_vectors].

## Unsized types and extern types
[unsized-types-and-extern-types]: #unsized-types-and-extern-types

[Extern types][rfc_extern_types] has long been blocked on these types being
neither `Sized` nor `?Sized` ([relevant issue][issue_extern_types_align_size]).

RFC #1861 defined that `std::mem::size_of_val` and `std::mem::align_of_val`
should not be defined for extern types but not how this should be achieved, and
suggested an initial implementation could panic when called with extern types,
but this is always wrong, and not desirable behavior in keeping with Rust's
values. `size_of_val` returns 0 and `align_of_val` returns 1 for extern types in
the current implementation. Ideally `size_of_val` and `align_of_val` would error
if called with an extern type, but this cannot be expressed in the bounds of
`size_of_val` and `align_of_val` and this remains a blocker for extern types.

Furthermore, unsized types cannot be members of structs as their alignment is
unknown and this is necessary to calculate field offsets. Extern types also
cannot be used in `Box` as `Box` requires size and alignment for both allocation
and deallocation.

Introducing a hierarchy of `Sized` traits will enable the backwards-compatible
introduction of a trait which only extern types do not implement and will
therefore enable the bounds of `size_of_val` and `align_of_val` to disallow
instantiations with extern types.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation
Most types in Rust have a size known at compilation time, such as `u32` or
`String`. However, some types in Rust do not have known sizes.

For example, slices have an unknown length while compiling and are
known as *dynamically-sized types*, their size must be computed at runtime.
There are also types with a runtime constant size (but unknown at compile time),
and *unsized types* with no size whatsoever.

Various parts of Rust depend on knowledge of the size of a type to work, for
example:

- [`std::mem::size_of_val`][api_size_of_val] computes the size of a value,
  and thus cannot accept extern types which have no size, and this should
  be prevented by the type system.
- Rust allows dynamically-sized types to be used as the final field in a struct,
  but the alignment of the type must be known, which is not the case for extern
  types.
- Allocation and deallocation of an object with `Box` requires knowledge of
  its size and alignment, which extern types do not have.
- For a value type to be allocated on the stack, it needs to have constant
  known size, which dynamically-sized and unsized types do not have (but 
  sized and "runtime sized" types do).

Rust uses marker traits to indicate the necessary knowledge required to know
the size of a type, if it can be known. There are three traits related to the size
of a type in Rust: `Sized`, `ValueSized`, and the existing unstable
`std::ptr::Pointee`. Each of these traits can be implemented as `const` when the
size is knowable at compilation time.

`Sized` is a supertrait of `ValueSized`, so every type which implements `Sized`
also implements `ValueSized`. Likewise, `ValueSized` is a supertrait of `Pointee`.
`Sized` is `const` if-and-only-if `ValueSized` is `const`, and `ValueSized` is
`const` if-and-only-if `Pointee` is `const`.

```
┌──────────────────────────────────────────────────────────────────────────┐
│ ┌──────────────────────────────────────────────────────────────────────┐ │
│ │ ┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓                                    │ │
│ │ ┃ ┌─────────────────────────────╂──────────────────────────────────┐ │ │
│ │ ┃ │ ┏━━━━━━━━━━━━━━━━━━━━━━━┓   ┃                                  │ │ │
│ │ ┃ │ ┃ ┏━━━━━━━━━━━━━━━━━━━┓ ┃   ┃                                  │ │ │
│ │ ┃ │ ┃ ┃ const Sized       ┃ ┃   ┃ Sized                            │ │ │
│ │ ┃ │ ┃ ┃ {type, target}    ┃ ┃   ┃ {type, target, runtime env}      │ │ │
│ │ ┃ │ ┃ ┗━━━━━━━━━━━━━━━━━━━┛ ┃   ┃                                  │ │ │
│ │ ┃ └─╂───────────────────────╂───╂──────────────────────────────────┘ │ │
│ │ ┃   ┃                       ┃   ┃                                    │ │
│ │ ┃   ┃ const ValueSized      ┃   ┃ ValueSized                         │ │
│ │ ┃   ┃ {type, target, value} ┃   ┃ {type, target, runtime env, value} │ │
│ │ ┃   ┗━━━━━━━━━━━━━━━━━━━━━━━┛   ┃                                    │ │
│ └─╂───────────────────────────────╂────────────────────────────────────┘ │
│   ┃                               ┃                                      │
│   ┃ const Pointee                 ┃ Pointee                              │
│   ┃ {*}                           ┃ {*, runtime env}                     │
│   ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛                                      │
└──────────────────────────────────────────────────────────────────────────┘
```

`const Sized` is implemented on types which require knowledge of only the
type and target platform in order to compute their size. For example, `usize`
implements `const Sized` as knowing only the type is `usize` and the target is
`aarch64-unknown-linux-gnu` then we can know the size is eight bytes, and
likewise with `armv7-unknown-linux-gnueabi` and a size of four bytes.

Some types' size can be computed using only knowledge of the type and target
platform, but only at runtime, and these types implement `Sized` (notably,
not `const`). For example, `svint8_t` is a scalable vector type (from
[rfc#3268][rfc_scalable_vectors]) whose length depends on the specific
implementation of the target (i.e. it may be 128 bits on one processor and
256 bits on another).

`const ValueSized` requires more knowledge than `const Sized` to compute the size:
it may additionally require a value  (therefore `size_of` is not implemented for
`ValueSized`, only `size_of_val`). For example, `[usize]` implements
`const ValueSized` as knowing the type and target is not sufficient, the number of
elements in the slice must also be known, which requires having the value.

As `Sized` is to `const Sized`, `ValueSized` is to `const ValueSized`: `ValueSized`
requires a value, knowledge of the type and target platform, and can only be
computed at runtime. For example, `[svint8_t]` requires a value to know how
many elements there are, and then information from the runtime environment
to know the size of a `svint8_t`.

`Pointee` is implemented by any type that can be used behind a pointer, which is
to say, every type (put otherwise, these types may or may not be sized at all).
For example, `Pointee` is therefore implemented on a `u32` which is trivially
sized, a `[usize]` which is dynamically sized, a `svint8_t` which is runtime
sized and an `extern type` (from [rfcs#1861][rfc_extern_types]) which has no
known size. `Pointee` is implemented as `const` when knowledge of the runtime
environment is not required (e.g. `const Pointee` for `u32` and `[usize]` but
bare `Pointee` for `svint8_t` or `[svint8_t]`).

All type parameters have an implicit bound of `const Sized` which will be
automatically removed if a `Sized`, `const ValueSized`, `ValueSized`,
`const Pointee` or `Pointee` bound is present instead.

Prior to the introduction of `ValueSized` and `Pointee`, `Sized`'s implicit bound
(now a `const Sized` implicit bound) could be removed using the `?Sized` syntax,
which is now equivalent to a `ValueSized` bound in non-`const fn`s and
`~const ValueSized` in `const fn`s and will be deprecated in the next edition.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Introduce a new marker trait, `ValueSized`, adding it to a trait hierarchy with the
[`Sized`][api_sized] and [`Pointee`][api_pointee] traits, and make all sizedness
traits `const`:

```
    ┌────────────────┐                  ┌─────────────────────────────┐
    │ const Sized    │ ───────────────→ │ Sized                       │
    │ {type, target} │     implies      │ {type, target, runtime env} │
    └────────────────┘                  └─────────────────────────────┘
            │                                          │
         implies                                    implies
            │                                          │
            ↓                                          ↓
┌───────────────────────┐             ┌────────────────────────────────────┐
│ const ValueSized      │ ──────────→ │ ValueSized                         │
│ {type, target, value} │   implies   │ {type, target, runtime env, value} │
└───────────────────────┘             └────────────────────────────────────┘
            │                                          │
         implies                                    implies
            │                                          │
            ↓                                          ↓
    ┌───────────────┐                         ┌──────────────────┐
    │ const Pointee │ ──────────────────────→ │ Pointee          │
    │ {*}           │         implies         │ {runtime env, *} │
    └───────────────┘                         └──────────────────┘
```

Or, in Rust syntax:

```rust=
const trait Sized: ~const ValueSized {}

const trait ValueSized: ~const std::ptr::Pointee {}
```

`Pointee` was specified in [rfcs#2580][rfc_pointer_metadata_vtable] and is
currently unstable. `Pointee` would become a `const` trait. It could be moved
to `std::marker` alongside the other sizedness traits or kept in `std::ptr`.

## Implementing `Sized`
[implementing-sized]: #implementing-sized

Implementations of the proposed traits are automatically generated by
the compiler and cannot be implemented manually:

- [`Pointee`][api_pointee]
    - Types that which can be used from behind a pointer (they may or may
      not have a size).
    - `Pointee` will be implemented for:
        - `ValueSized` types
        - compound types where every element is `Pointee`
    - `const Pointee` will be implemented for:
        - `const ValueSized` types
        - `extern type`s from [rfcs#1861][rfc_extern_types]
        - compound types where every element is `const Pointee`
    - In practice, every type will implement `Pointee` (as in
      [rfcs#2580][rfc_pointer_metadata_vtable]).
- `ValueSized`
    - Types whose size is computable given a value, and knowledge of the
      type, target platform and runtime environment.
      - `const ValueSized` does not require knowledge of the runtime
        environment
    - `ValueSized` is a subtrait of `Pointee`
    - `ValueSized` will be implemented for:
        - `Sized` types
        - slices `[T]` where every element is `ValueSized`
        - compound types where every element is `ValueSized`
    - `const ValueSized` will be implemented for:
        - `const Sized` types
        - slices `[T]` where every element is `const Sized`
        - string slice `str`
        - trait objects `dyn Trait`
        - compound types where every element is `const ValueSized`
- `Sized`
    - Types whose size is computable given knowledge of the type, target
      platform and runtime environment.
      - `const Sized` does not require knowledge of the runtime environment
    - `Sized` is a subtrait of `ValueSized`.
    - `Sized` will be implemented for:
        - scalable vectors from [rfcs#3268][rfc_scalable_vectors]
        - compound types where every element is `Sized`
    - `const Sized` will be implemented for:
        - primitives `iN`, `uN`, `fN`, `char`, `bool`
        - pointers `*const T`, `*mut T`
        - function pointers `fn(T, U) -> V`
        - arrays `[T; n]`
        - never type `!`
        - unit tuple `()`
        - closures and generators
        - compound types where every element is `const Sized`
        - anything else which currently implements `Sized`

Introducing new automatically implemented traits is backwards-incompatible,
at least if you try to add it as a bound to an existing function[^1][^2] (and
new auto traits that which go unused aren't that useful), but due to being
supertraits of `Sized` and `Sized` being a default bound, these
backwards-incompatibilities are avoided for `ValueSized` and `Pointee`.

Relaxing a bound from `Sized` to `ValueSized` or `Pointee` is non-breaking as
the calling bound must have either `T: Sized` or `T: ?Sized`, both of which
would satisfy any relaxed bound[^3].

However, it would still be backwards-incompatible to relax the `Sized` bound on
a trait's associated type[^4] for the proposed traits.

[^1]: Adding a new automatically implemented trait and adding it as a bound to
      an existing function is backwards-incompatible with generic functions. Even
      though all types could implement the trait, existing generic functions will be
      missing the bound.

      If `Foo` were introduced to the standard library and implemented on every
      type, and it was added as a bound to `size_of` (or any other generic
      parameter)..

      ```rust=
      auto trait Foo;

      fn size_of<T: Sized + Foo>() { /* .. */ } // `Foo` bound is new!
      ```

      ...then user code would break:

      ```rust=
      fn do_stuff<T>(value: T) { size_of(value) }
      // error! the trait bound `T: Foo` is not satisfied
      ```
[^2]: Trait objects passed by callers would not imply the new trait.

      If `Foo` were introduced to the standard library and implemented on every
      type, and it was added as a bound to `size_of_val` (or any other generic
      parameter)..

      ```rust=
      auto trait Foo;

      fn size_of_val<T: ?Sized + Foo>(x: val) { /* .. */ } // `Foo` bound is new!
      ```

      ...then user code would break:

      ```rust
      fn do_stuff(value: Box<dyn Display>) { size_of_val(value) }
      // error! the trait bound `dyn Display: Foo` is not satisfied in `Box<dyn Display>`
      ```
[^3]: Callers of existing APIs will have one of the following `Sized` bounds:

      | Before ed. migration              | After ed. migration |
      | --------------------------------- | ------------------- |
      | `T: Sized` (implicit or explicit) | `T: const Sized`    |
      | `T: ?Sized`                       | `T: const ValueSized` |

      Any existing function in the standard library with a `T: Sized` bound
      could be changed to one of the following bounds and remain compatible with
      any callers that currently exist (as per the above table):

      |                | `const Sized` | `Sized` | `const ValueSized` | `ValueSized` | `const Pointee` | `Pointee`
      | -------------- | ------------- | ------- | ------------------ | ------------ | --------------- | ---------
      | `const Sized`  | ✔             | ✔       | ✔                  | ✔            | ✔               | ✔

      Likewise with a `T: ?Sized` bound:

      |                    | `const ValueSized` | `ValueSized` | `const Pointee` | `Pointee`
      | ------------------ | ------------------ | ------------ | --------------- | ---------
      | `const Sized`      | ✔                  | ✔            | ✔               | ✔
      | `const ValueSized` | ✔                  | ✔            | ✔               | ✔
[^4]: Associated types of traits have default `Sized` bounds which cannot be
      relaxed. For example, relaxing a `Sized` bound on `Add::Output` breaks
      a function which takes a `T: Add` and passes `<T as Add>::Output` to
      `size_of` as not all types which implement the relaxed bound will
      implement `Sized`.

      If a default `Sized` bound on an associated trait, such as
      `Add::Output`, were relaxed in the standard library...

      ```rust=
      trait Add<Rhs = Self> {
          type Output: ValueSized;
      }
      ```

      ...then user code would break:

      ```rust=
      fn do_stuff<T: Add>() -> usize { std::mem::size_of::<<T as Add>::Output>() }
      //~^ error! the trait bound `<T as Add>::Output: Sized` is not satisfied
      ```

      Relaxing the bounds of an associated type is in effect giving existing
      parameters a less restrictive bound which is not backwards compatible.

## `Sized` bounds
[sized-bounds]: #sized-bounds

`?Sized` would be made syntactic sugar for a `const ValueSized` bound. A
`const ValueSized` bound is equivalent to a `?Sized` bound as all values in Rust
today whose types do not implement `Sized` are valid arguments to
[`std::mem::size_of_val`][api_size_of_val] and as such have a size which can be
computed given a value and knowledge of the type and target platform, and
therefore will implement `const ValueSized`. As there are currently no
extern types or other types which would not implement `const ValueSized`,
every type in Rust today which would satisfy a `?Sized` bound would satisfy
a `const ValueSized` bound.

**Edition change:** In the current edition,`?Sized` will be syntatic sugar for
a `const ValueSized` bound. In the next edition, use of `?Sized` syntax will be prohibited
over an edition and all uses of it will be rewritten to a `const ValueSized` bound.

A default implicit bound of `const Sized` is added by the compiler to every type
parameter `T` that does not have an explicit `Sized`, `?Sized`, `const ValueSized`,
`ValueSized`, `const Pointee` or `Pointee` bound. It is backwards compatible to change
the current implicit `Sized` bound to an `const Sized` bound as every type which
exists currently will implement `const Sized`.

**Edition change:** In the current edition, all existing `Sized` bounds will be
sugar for `const Sized`. In the next edition, existing `Sized` bounds will be rewritten
to `const Sized` bounds and new bare `Sized` bounds will be non-const.

An implicit `const ValueSized` bound is added to the `Self` type of traits. Like
implicit `const Sized` bounds, this is omitted if an explicit `const Sized`, `Sized`,
`ValueSized` or `Pointee` bound is present.

As `ValueSized` and `Pointee` are not default bounds, there is no equivalent to `?Sized`
for these traits.

## `size_of` and `size_of_val`
[size-of-and-size-of-val]: #size-of-and-size-of-val

Runtime-sized types should be able to be passed to both `size_of` and `size_of_val`,
but only at runtime, which requires ensuring these functions are only const if their
arguments have const implementations of the relevant sizedness traits.

[`size_of`][api_size_of] is a const-stable function since Rust 1.24.0 and currently
accepts a `T: Sized` bound. Therefore, even when used in a const context, `size_of`
could accept a runtime-sized type. It is therefore necessary to modify the bounds of
`size_of` to accept a `T: ~const Sized`, so that `size_of` is a const function
if-and-only-if `Sized` has a `const` implementation.

```rust=
pub const fn size_of<T: ~const Sized>() -> usize {
    /* .. */
}
```

This has the potential to break existing code like `uses_size_of` in the below
example. However, due to the changes described in [`Sized` bounds][sized-bounds]
(changing the implicit `T: Sized` to `T: const Sized`, and treating explicit
`T: Sized` bounds as `T: const Sized` in the current edition), this code would
not break.

```rust=
fn uses_size_of<T: Sized>() -> usize {
    const { std::mem::size_of<T>() }
}
```

[`size_of_val`][api_size_of_val] is currently const-unstable, so its bound can be
changed from `?Sized` to `~const ValueSized` without any backwards compatibility
issues.

```rust=
pub const fn size_of_val<T>(val: &T) -> usize
where
    T: ~const ValueSized,
{
    /* .. */
}
```

While `ValueSized` is equivalent to the current `?Sized` bound it replaces, it
excludes extern types (which `?Sized` by definition cannot), which prevents
`size_of_val` from being called with extern types from
[rfcs#1861][rfc_extern_types].

These same changes apply to `align_of` and `align_of_val`.

## Implementing `Copy` for runtime-sized types

Runtime-sized types are value types which should be able to be used as
locals, be copied, etc. To be able to implement [`Copy`][api_copy], these
types would need to implement [`Clone`][api_clone], and to implement
`Clone`, they need to implement `Sized`.

This property trivially falls out of the previous proposals in this RFC:
runtime-sized types are `Sized` (but not `const Sized`) and therefore can
implement `Clone` and `Copy`.

## Restrictions in compound types
[restrictions-on-compound-types]: #restrictions-on-compound-types

`Pointee` types cannot be used in non-`#[repr(transparent)]` compound types
as the alignment of these types would need to be known in order to calculate field
offsets. `const Sized` types can be used in compound types with no restrictions.
`Sized`, `const ValueSized` and `ValueSized` types can be used in compound types, but
only as the last element.

## Compiler performance implications
[compiler-performance-implications]: #compiler-performance-implications

There is a potential performance impact within the trait system to adding
supertraits to `Sized`, as implementation of these supertraits will need to be
proven whenever a `Sized` obligation is being proven (and this happens very
frequently, being a default bound). It may be necessary to implement an
optimisation whereby `Sized`'s supertraits are assumed to be implemented and
checking them is skipped - this should be sound as all of these traits are
implemented by the compiler and therefore this property can be guaranteed.

## Forward compatibility with supertraits implying default bound removal
[forward-compatibility-with-supertraits-default-bound]: #forward-compatibility-with-supertraits-implying-default-bound-removal

Traits which are a supertrait of any of the proposed traits will not
automatically imply the proposed trait in any bounds where the trait is
used, e.g.

```rust
trait NewTrait: ValueSized {}

struct NewRc<T: NewTrait> {} // equiv to `T: NewTrait + Sized` as today
```

If the user wanted `T: ValueSized` then it would need to be written explicitly.
This is forward compatible with trait bounds which have sizedness supertraits
implying the removal of the default `const Sized` bound.

## Ecosystem churn
[ecosystem-churn]: #ecosystem-churn

It is not expected that this RFC's additions would result in much churn within
the ecosystem. Almost all of the necessary changes would happen automatically
during edition migration.

All bounds in the standard library should be re-evaluated during the
implementation of this RFC, but bounds in third-party crates need not be.

As runtime-sized types will primarily be used for localised performance optimisation,
and `Pointee` types will primarily be used for localised FFI, neither is expected
to be so pervasive throughout Rust codebases to the extent that all existing
`const Sized`, `~const ValueSized` or `ValueSized` bounds (after edition migration) would
need to be immediately reconsidered in light of their addition, even if in many cases
these could be relaxed.

If a user of a runtime-sized type or a `Pointee` type did encounter a bound that
needed to be relaxed, this could be changed in a patch to the relevant crate without
breaking backwards compatibility as-and-when such cases are encountered.

If edition migration were able to attempt migrating each bound to a more relaxed bound
and then use the guaranteed-to-work bound as a last resort then this could further
minimise any changes required by users.

## Other changes to the standard library
[other-changes-to-the-standard-library]: #other-changes-to-the-standard-library

With these new traits and having established changes to existing bounds which
can be made while preserving backwards compatibility, the following changes
could be made to the standard library:

- [`std::boxed::Box`][api_box]
    - `T: ?Sized` becomes `T: ValueSized`
    - As before, this is not a breaking change and prevents types only
      implementing `Pointee` from being used with `Box`, as these types do
      not have the necessary size and alignment for allocation/deallocation.

As part of the implementation of this RFC, each `Sized`/`?Sized` bound in
the standard library would need to be reviewed and updated as appropriate.

# Drawbacks
[drawbacks]: #drawbacks

- This is a fairly significant change to the `Sized` trait, which has been in
  the language since 1.0 and is now well-understood.
- This RFC's proposal that adding a bound of `const Sized`, `const ValueSized`,
  `ValueSized`, `const Pointee` or `Pointee` would remove the default `Sized`
  bound is somewhat unintuitive. Typically adding a trait bound does not
  remove another trait bound, however it's debatable whether this is more or
  less confusing than existing `?Sized` bounds.
- As this RFC depends on `const Trait`, it inherits all of the drawbacks of
  `const Trait`.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

There are various points of difference to the [prior art](#prior-art) related to
`Sized`, which spans almost the entirety of the design space:

- In contrast to [rfcs#709][rfc_truly_unsized_types], marker *types* aren't used
  to disable `Sized` because we are able to introduce traits to do so without
  backwards compatibility hazards and that feels more appropriate for Rust.
- In contrast to [rfcs#1524][rfc_custom_dst], items are not added to the `Sized`
  trait as this wouldn't be sufficient to capture the range in sizedness that this
  RFC aims to capture (i.e. for scalable vectors with `const Sized` or
  extern types with `Pointee`), even if it theoretically could enable Custom DSTs.
- In contrast to [rfcs#1993][rfc_opaque_data_structs], [rust#44469][pr_dynsized],
  [rust#46108][pr_dynsized_rebase],  [rfcs#2984][rfc_pointee_dynsized] and
  [eRFC: Minimal Custom DSTs via Extern Type (DynSized)][erfc_minimal_custom_dsts_via_extern_type],
  none of the traits proposed in this RFC are default bounds and therefore do not
  need to support being relaxed bounds (i.e. no `?ValueSized`), which avoids
  additional language complexity and backwards compatibility hazards related to
  relaxed bounds and associated types.
- In contrast to [rfcs#1524][rfc_custom_dst], [rfc#1993][rfc_opaque_data_structs],
  [Pre-eRFC: Let's fix DSTs][pre_erfc_fix_dsts], [Pre-RFC: Custom DSTs][prerfc_custom_dst]
  and [eRFC: Minimal Custom DSTs via Extern Type (DynSized)][erfc_minimal_custom_dsts_via_extern_type],
  `ValueSized` does not have `size_of_val`/`align_of_val` methods to support 
  custom DSTs as this would add to the complexity of this proposal and custom DSTs
  are not this RFC's focus, see the [Custom DSTs][custom-dsts] section later.

## Why have `Pointee`?
[why-have-pointee]: #why-have-pointee

It may seem that re-using the `Pointee` trait at the bottom of the trait hierarchy
is unnecessary as this is equivalent to the absense of any bounds whatsoever, but
having an `Pointee` trait is necessary to enable the meaning of `?Sized` to be re-defined
to be equivalent to `const ValueSized` and avoid complicated behaviour change over an edition.

Without `Pointee`, if a user wanted to remove all sizedness bounds from a generic
parameter then they would have two options:

1. Introduce new relaxed bounds (i.e. `?ValueSized`), which has been found
   unacceptable in previous RFCs ([rfcs#2255][issue_more_implicit_bounds]
   summarizes these discussions)
2. Keep `?Sized`'s existing meaning of removing the implicit `Sized` bound
      
   This is the only viable option, but this would complicate changing
   `size_of_val`'s existing `?Sized` bound:

   Without `Pointee`, `?Sized` would be equivalent to `const ValueSized` until
   extern types are stabilised (e.g. a `?Sized` bound would accept exactly the
   same types as a `const ValueSized` bound, but after extern types are introduced,
   `?Sized` bounds would accept extern types and `const ValueSized` bounds would not).
   extern types would need to be introduced over an edition and all existing `?Sized`
   bounds rewritten to `?Sized + const ValueSized`. This is the same mechanism described
   in [rfcs#3396][rfc_extern_types_v2] to introduce its `MetaSized` trait.

## Why use const traits?
[why-use-const-traits]: #why-use-const-traits

Previous iterations of this RFC had both linear[^5] and non-linear[^6] trait hierarchies
which included a `RuntimeSized` trait and did not use const traits. However, both of
these were found to be backwards-incompatible due to being unable to relax the
supertrait of `Clone`. Without const traits, it is not possible to represent
runtime-sized types.

[^5]: In previous iterations, the proposed linear trait hierarchy was:

      ```
      ┌───────────────────────────────────────────────────┐
      │ ┌───────────────────────────────────────┐         │
      │ │ ┌────────────────────────┐            │         │
      │ │ │ ┌───────┐              │            │         │
      │ │ │ │ Sized │ RuntimeSized │ ValueSized │ Pointee │
      │ │ │ └───────┘              │            │         │
      │ │ └────────────────────────┘            │         │
      │ └───────────────────────────────────────┘         │
      └───────────────────────────────────────────────────┘
      ```

      This approach was scrapped once it became clear that a `const`-stable
      `size_of_val` would need to be able to be instantiated with `ValueSized`
      types and not `RuntimeSized` types, and that this could not be
      represented.
[^6]: In previous iterations, the proposed non-linear trait hierarchy was:

      ```
      ┌───────────────────────────────────────────────────────────────┐
      │ ┌───────────────────────────────────────────────────────────┐ │
      │ │ ┌──────────────────────────────┐┌───────────────────────┐ │ │
      │ │ │ ┌────────────────────────────┴┴─────────────────────┐ │ │ │
      │ │ │ │ Sized                                             │ │ │ │
      │ │ │ │ {type, target}                                    │ │ │ │
      │ │ │ └────────────────────────────┬┬─────────────────────┘ │ │ │
      │ │ │ RuntimeSized                 ││ ValueSized            │ │ │
      │ │ │ {type, target, runtime env}  ││ {type, target, value} │ │ │
      │ │ └──────────────────────────────┘└───────────────────────┘ │ │
      │ │ DynRuntimeSized                                           │ │
      │ │ {type, target, runtime env, value}                        │ │
      │ └───────────────────────────────────────────────────────────┘ │
      │ Pointee                                                       │
      │ {*}                                                           │
      └───────────────────────────────────────────────────────────────┘
      ```

      This approach proposed modifying the bounds on the `size_of` function
      from `RuntimeSized` to `Sized` when used in a const context (and from
      `DynRuntimeSized` to `ValueSized` for `size_of_val`) to try and work
      around the issues with constness, but this would have been unsound,
      and ultimately the inability to relax `Clone`'s supertrait made it
      infeasible anyway.

## Alternatives to this accepting this RFC
[alternatives-to-this-rfc]: #alternatives-to-this-rfc

There are not many alternatives to this RFC to unblock extern types and
scalable vectors:

- Without this RFC, scalable vectors from [rfcs#3268][rfc_scalable_vectors]
  would remain blocked unless special-cased by the compiler in the type
  system.
      - It is not possible to add these without const traits: relaxing the
        supertrait of `Clone` is backwards-incompatible and const traits
        are the only way to avoid that.
- Extern types from [rfcs#1861][rfc_extern_types] would remain blocked if no
  action was taken, unless:
    - The language team decided that having `size_of_val` and `align_of_val`
      panic was acceptable.
    - The language team decided that having `size_of_val` and `align_of_val`
      return `0` and `1` respectively was acceptable.
    - The language team decided that extern types could not be instantiated
      into generics and that this was acceptable.
    - The language team decided that having `size_of_val` and `align_of_val`
      produce post-monomorphisation errors for extern types was acceptable.

## Bikeshedding
[bikeshedding]: #bikeshedding

All of the trait names proposed in the RFC can be bikeshed and changed, they'll
ultimately need to be decided but aren't the important part of the RFC.

# Prior art
[prior-art]: #prior-art

There have been many previous proposals and discussions attempting to resolve
the `size_of_val` and `align_of_val` for extern types through modifications to
the `Sized` trait. Many of these proposals include a `DynSized` trait, of which
this RFC's `ValueSized` trait is inspired, just renamed.

- [rfcs#709: truly unsized types][rfc_truly_unsized_types], [mzabaluev][author_mzabaluev], Jan 2015
    - Earliest attempt to opt-out of `Sized`.
    - Proposes dividing types which do not implement `Sized` into DSTs and types
      of indeterminate size.
        - Adding a field with a `std::marker::NotSized` type will make a type
          opt-out of `Sized`, preventing the type from being used in all the places where
          it needs to be `Sized`.
        - Dynamically sized types will "intrinsically" implement `DynamicSize`,
          references to these types will use fat pointers.
    - Ultimately postponed for post-1.0.
- [rfcs#813: truly unsized types (issue)][issue_truly_unsized_types], [pnkfelix][author_pnkfelix], Feb 2015
    - Tracking issue for postponed [rfcs#709][rfc_truly_unsized_types].
    - Links to an newer version of [rfcs#709][rfc_truly_unsized_types], still
      authored by [mzabaluev][author_mzabaluev].
    - Proposes being able to opt-out of `Sized` with a negative impl (a `CStr`
      type containing only a `c_char` is the example given of a DST which would
      opt-out of `Sized`).
        - Also proposes removing `Sized` bound on various `AsPtr`/`AsMutPtr`/
          `FromPtr`/`FromMutPtr` traits as they existed at the time, so that a user might
          be able to implement these to preserve the ability to use a thin pointer for
          their unsized type when that is possible.
    - Ultimately closed after [rfcs#1861][rfc_extern_types] was merged and
      intended that [rfcs#2255][issue_more_implicit_bounds] be used to discuss the
      complexities of that proposal.
- [rfcs#1524: Custom Dynamically Sized Types][rfc_custom_dst], [strega-nil][author_strega_nil], Mar 2016
    - Successor of [rfcs#709][rfc_truly_unsized_types]/[rfcs#813][issue_truly_unsized_types].
    - Proposes an `unsafe trait !Sized` (which isn't just a negative impl), with
      an associated type `Meta` and `size_of_val` method.
        - Under this proposal, users would create a "borrowed" version of their
          type (e.g. what `[T]` is to `Vec<T>`) which has a zero-sized last field, which
          is described in the RFC as "the jumping off point for indexing your block of
          memory".
        - These types would implement `!Sized`, providing a type for `Meta`
          containing any extra information necessary to compute the size of the DST (e.g.
          a number of strides) and an implementation of `size_of_val` for the type.
        - There would be intrinsics to help make create instances of
          these dynamically sized types, namely `make_fat_ptr`, `fat_ptr_meta` and
          `size_of_prelude`.
- [rfcs#1861: extern types][rfc_extern_types], [canndrew][author_canndrew], Jan 2017
    - Merged in Jul 2017.
    - This RFC mentions the issue with `size_of_val` and `align_of_val` but
      suggests that these functions panic in an initial implementation and that
      "before this is stabilised, there should be some trait bound or similar on them
      that prevents there use statically". Inventing an exact mechanism was intended
      to be completed by [rfcs#1524][rfc_custom_dst] or its like.
- [rfcs#1993: Opaque Data structs for FFI][rfc_opaque_data_structs], [mystor][author_mystor], May 2017
    - This RFC was an alternative to the original extern types RFC
      ([rfcs#1861][rfc_extern_types]) and introduced the idea of a `DynSized` auto
      trait.
    - Proposes a `DynSized` trait which was a built-in, unsafe, auto trait,
      a supertrait of `Sized`, and a default bound which could be relaxed with
      `?DynSized`.
        - It would automatically implemented for everything that didn't have an
          `Opaque` type in it (RFC 1993's equivalent of an `extern type`).
        - `size_of_val` and `align_of_val` would have their bounds changed to
          `DynSized`.
        - Trait objects would have a `DynSized` bound by default and the
          `DynSized` trait would have `size_of_val` and `align_of_val` member functions.
    - Ultimately closed as [rfcs#1861][rfc_extern_types] was entering final
      comment period.
- [rust#43467: Tracking issue for RFC 1861][issue_tracking_extern_types], [aturon][author_aturon], Jul 2017
    - Tracking thread created for the implementation of [rfc#1861][rfc_extern_types].
    - In 2018, the language team had consensus against having `size_of_val`
      return a sentinel value and adding any trait machinery, like `DynSized`, didn't
      seem worth it, preferring to panic or abort.
        - This was considering `DynSized` with a relaxed bound.
        - Anticipating some form of custom DSTs, there was the possibility
          that `size_of_val` could run user code and panic anyway, so making it panic
          for extern types wasn't as big an issue. `size_of_val` running in unsafe code
          could be a footgun and that caused mild concern.
        - See [this comment](https://github.com/rust-lang/rust/issues/43467#issuecomment-377521693)
          and [this comment](https://github.com/rust-lang/rust/issues/43467#issuecomment-377665733).
    - Conversation became more sporadic following 2018 and most
      recent discussion was spurred by the
      [Sized, DynSized and Unsized][blog_dynsized_unsized] blog post.
        - See [this comment](https://github.com/rust-lang/rust/issues/43467#issuecomment-2073513472)
          onwards.
        - It's unclear how different language team opinion is since the 2018
          commentary, but posts like above suggest some change.
- [rust#44469: Add a `DynSized` trait][pr_dynsized], [plietar][author_plietar], Sep 2017
    - This pull request intended to implement the `DynSized` trait from
      [rfcs#1993][rfc_opaque_data_structs].
    - `DynSized` as implemented is similar to that from
      [rfcs#1993][rfc_opaque_data_structs] except it is implemented for every
      type with a known size and alignment at runtime, rather than requiring an
      `Opaque` type.
    - In addition to preventing extern types being used in `size_of_val` and
      `align_of_val`, this PR is motivated by wanting to have a mechanism by which
      `!DynSized` types can be prevented from being valid in struct tails due to needing
       to know the alignment of the tail in order to calculate its field offset.
    - `DynSized` had to be made a implicit supertrait of all traits in this
      implementation - it is presumed this is necessary to avoid unsized types
      implementing traits.
    - This actually went through FCP and would have been merged if not
      eventually closed for inactivity.
- [rust#46108: Add DynSized trait (rebase of #44469)][pr_dynsized_rebase], [mikeyhew][author_mikeyhew], Nov 2017
    - This pull request is a resurrection of [rust#44469][pr_dynsized].
    - Concerns were raised about the complexity of adding another `?Trait` to
      the language, and suggested that having `size_of_val` panic was sufficient (the
      current implementation does not panic and returns zero instead, which is also
      deemed undesirable).
        - It was argued that `?Trait`s are powerful and should be made more
          ergonomic rather than avoided.
    - [kennytm][author_kennytm] left a useful comment summarising [which
      standard library bounds would benefit from relaxation to a `DynSized` 
      bound](https://github.com/rust-lang/rust/pull/46108#issuecomment-353672604).
    - Ultimately this was closed [after a language team meeting](https://github.com/rust-lang/rust/pull/46108#issuecomment-360903211)
      deciding that `?DynSized` was ultimately too complex and couldn't be
      justified by support for a relatively niche feature like extern types.
- [rfcs#2255: More implicit bounds (?Sized, ?DynSized, ?Move)][issue_more_implicit_bounds], [kennytm][author_kennytm], Dec 2017
    - Issue created following [rust#46108][pr_dynsized_rebase] to discuss the
      complexities surrounding adding new traits which would benefit from relaxed
      bounds (`?Trait` syntax).
    - There have been various attempts to introduce new auto traits with
      implicit bounds, such as `DynSized`, `Move`, `Leak`, etc. Often rejected due to
      the ergonomic cost of relaxed bounds.
        - `?Trait` being a negative feature confuses users.
        - Downstream crates need to re-evaluate every API to determine if adding
          `?Trait` makes sense, for each `?Trait` added.
        - `?Trait` isn't actually backwards compatible like everyone thought due
          to interactions with associated types.
    - This thread was largely motivated by the `Move` trait and that was
      replaced by the `Pin` type, but there was an emerging consensus that `DynSized`
      may be more feasible due to its relationship with `Sized`.
- [Pre-eRFC: Let's fix DSTs][pre_erfc_fix_dsts], [mikeyhew][author_mikeyhew], Jan 2018
    - This eRFC was written as a successor to [rfcs#1524][rfc_custom_dst].
    - It proposes `DynSized` trait and a bunch of others. `DynSized` is a
      supertrait of `Sized` (indirectly) and contains a `size_of_val` method. This
      proposal is the first to remove `Sized` bounds if another sized trait (e.g.
      `DynSized`) has an explicit bound.
        - This enables deprecation of `?Sized` like this RFC proposes.
    - A `Thin` type to allow thin pointers to DSTs is also proposed in
      this pre-eRFC - it is a different `Thin` from the currently unstable
      `core::ptr::Thin` and it's out-of-scope for this RFC to include a similar type
      and accepted [rfcs#2580][rfc_pointer_metadata_vtable] overlaps.
    - This pre-eRFC may be the origin of the idea for a family of `Sized`
      traits, later cited in [Sized, DynSized, and Unsized][blog_dynsized_unsized].
    - [rfcs#2510][rfc_pointer_metadata_vtable] was later submitted which was a
      subset of this proposal (but none of the `DynSized` parts).
    - This eRFC ultimately fizzled out and didn't seem to result in a proper RFC
      being submitted.
- [rfcs#2310: DynSized without ?DynSized][rfc_dynsized_without_dynsized], [kennytm][author_kennytm], Jan 2018
    - This RFC proposed an alternative version of `DynSized` from
      [rfcs#1993][rfc_opaque_data_structs]/[rust#44469][pr_dynsized] but without
      being an implicit bound and being able to be a relaxed bound (i.e. no
      `?DynSized`).
        - Adding new implicit bounds which can be relaxed has backwards
          compatibility hazards, see [rfcs#2255][issue_more_implicit_bounds].
    - The proposed `DynSized` trait in [rfcs#2310][rfc_dynsized_without_dynsized]
      is really quite similar to the `ValueSized` trait proposed by this RFC except:
        - It includes an `#[assume_dyn_sized]` attribute to be added to
          `T: ?Sized` bounds instead of replacing them with `T: const ValueSized`,
          which would warn instead of error when a non-`const ValueSized` type is
          substituted into `T`.
            - This is to avoid a backwards compatibility break for uses of
              `size_of_val` and `align_of_val` with extern types, but it is
              unclear why this is necessary given that extern types are
              unstable.
        - It does not include `Pointee` or any of the const traits.
        - Adding an explicit bound for `ValueSized` would not remove the implicit
          bound for `Sized`.
- [rust#49708: `extern type` cannot support `size_of_val` and `align_of_val`][issue_extern_types_align_size], [joshtriplett][author_joshtriplett], Apr 2018
    - Primary issue for the `size_of_val`/`align_of_val` extern types
      blocker, following no resolution from either of [rfcs#1524][rfc_custom_dst] and
      [rust#44469][pr_dynsized] or their successors.
    - This issue largely just re-hashes the arguments made in other threads
      summarised here.
- [Pre-RFC: Custom DSTs][prerfc_custom_dst], [ubsan][author_ubsan], Nov 2018
    - This eRFC was written as a successor to [rfcs#1524][rfc_custom_dst].
    - Proposes addition of a `DynamicallySized` trait with a `Metadata`
      associated type and `size_of_val` and `align_of_val` member functions.
        - It has an automatic implementation for all `Sized` types, where
          `Metadata = ()` and `size_of_val` and `align_of_val` just call `size_of` and
          `align_of`.
        - It can be manually implemented for DSTs and if it is, the type will
          not implement `Sized`.
    - Due to `DynamicallySized` not being a supertrait of `Sized`, this proposal
      had no way of modifying the bounds of `size_of_val` and `align_of_val` without
      it being a breaking change (and so did not propose doing so).
    - This eRFC ultimately fizzled out and didn't seem to result in a proper RFC
      being submitted.
- [rfcs#2594: Custom DSTs][rfc_custom_dst_electric_boogaloo], [strega-nil][author_strega_nil], Nov 2018 
    - This eRFC was written as a successor to [rfcs#1524][rfc_custom_dst].
        - This is more clearly an direct evolution of [rfcs#1524][rfc_custom_dst]
          than other successors were, unsurprisingly given the same author.
    - Proposes a `Pointee` trait with `Metadata` associated type and a
      `Contiguous` supertrait of `Pointee` with `size_of_val` and `align_of_val`
      members.
        - `Sized` is a subtrait of `Pointee<Metadata = ()>`  (as sized types
          have thin pointers). `Sized` also implements `Contiguous` calling `size_of` and
          `align_of` for each of the member functions.
        - Dynamically sized types can implement `Pointee` manually and provide
          a `Metadata` associated type, and then `Contiguous` to implement `size_of_val`
          and `align_of_val`.
        - Intrinsics are added for constructing a pointer to a dynamically
          sized type from its metadata and value, and for accessing the metadata of a
          dynamically sized type.
        - extern types do not implement `Contiguous` but do implement `Pointee`.
        - `Contiguous` is a default bound and so has a relaxed form `?Contiguous`.
    - There's plenty of overlap here with [rfcs#2580][rfc_pointer_metadata_vtable]
      and its `Pointee` trait - the accepted [rfcs#2580][rfc_pointer_metadata_vtable]
      does not make `Sized` a subtrait of `Pointee` or have a `Contiguous` trait but
      the `Pointee` trait is more or less compatible.
    - Discussed in a [November 4th 2020 design meeting](https://www.youtube.com/watch?v=wYmJK62SSOM&list=PL85XCvVPmGQg-gYy7R6a_Y91oQLdsbSpa&index=63)
      ([pre-meeting notes](https://hackmd.io/1Fq9TcAQRWa4_weWTe9adA) and
      [post-meeting notes](https://github.com/rust-lang/lang-team/blob/master/design-meeting-minutes/2020-11-04-RFC-2580-and-custom-dst.md)).
        - Meeting was mostly around [rfcs#2580][rfc_pointer_metadata_vtable] but
          mentioned the state of Custom DSTs.
    - Mentioned briefly in a [language team triage meeting](https://www.youtube.com/watch?v=NzURKQouuEU&t=3292s)
      in March 2021 and postponed until [rfcs#2510][rfc_pointer_metadata_vtable]
      was implemented.
- [Design Meeting][design_meeting], Language Team, Jan 2020
    - Custom DSTs and `DynSized` are mentioned but there aren't any implications
      for this RFC.
- [rfcs#2984: introduce `Pointee` and `DynSized`][rfc_pointee_dynsized], [nox][author_nox], Sep 2020
    - This RFC aims to land some traits in isolation so as to enable progress on
      other RFCs.
    - Proposes a `Pointee` trait with associated type `Meta` (very similar to
      accepted [rfcs#2580][rfc_pointer_metadata_vtable]) and a `DynSized` trait which
      is a supertrait of it. `Sized` is made a supertrait of `DynSized<Meta = ()>`.
      Neither new trait can be implemented by hand.
        - It's implied that `DynSized` is implemented for all dynamically sized
          types, but it isn't clear.
    - Despite being relatively brief, RFC 2984 has lots of comments.
        - The author argues that `?DynSized` is okay and disagrees with
           previous concerns about complexity and that all existing bounds would need to be
           reconsidered in light of `?DynSized`.
            - In response, it is repeatedly argued that there is a mild
              preference for making `size_of_val` and `align_of_val` panic instead of adding
              `?Trait` bounds and that having the ability to do `Pointee<Meta = ()>` type
              bounds is sufficient.
- [Exotically sized types (`DynSized` and `extern type`)][design_notes_dynsized_constraints], Language Team, Jun 2022
    - Despite being published in Jun 2022, these are reportedly notes from a
      previous Jan 2020 meeting, but not the one above.
    - Explores constraints `Arc`/`Rc` and `Mutex` imply on `DynSized` bounds.
    - `MetaSized` is first mentioned in these meeting notes, as when the size/
      alignment can be known from pointer metadata.
- [eRFC: Minimal Custom DSTs via Extern Type (DynSized)][erfc_minimal_custom_dsts_via_extern_type], [CAD97][author_cad97], May 2022
    - This RFC proposes a forever-unstable default-bound unsafe trait `DynSized`
      with `size_of_val_raw` and `align_of_val_raw`, implemented for everything other
      than extern types. Users can implement `DynSized` for their own types. This
      proposal doesn't say whether `DynSized` is a default bound but does mention a
      relaxed form of the trait `?DynSized`.
- [rfcs#3319: Aligned][rfc_aligned], [Jules-Bertholet][author_jules_bertholet], Sep 2022
    - This RFC aims to separate the alignment of a type from the size of the
      type with an `Aligned` trait.
        - Automatically implemented for all types with an alignment (includes
          all `Sized` types).
        - `Aligned` is a supertrait of `Sized`.
- [rfcs#3396: Extern types v2][rfc_extern_types_v2], [Skepfyr][author_skepfyr], Feb 2023
    - Proposes a `MetaSized` trait for types whose size and alignment can
      be determined solely from pointer metadata without having to dereference the
      pointer or inspect the pointer's address.
        - Under this proposal, `[T]` is `MetaSized` as the pointer metadata
          knows the size, rather than `DynSized`.
        - `MetaSized` is automatically implemented for all types except extern
          types.
        - `MetaSized` types can be the last field of a struct as the offset can
          be determined from the pointer metadata alone.
        - `Sized` is not a supertrait or subtrait of `MetaSized`.
            - This may make the proposal subject to backwards
              incompatibilities described in [Auto traits and backwards
              compatibility][auto-traits-and-backwards-compatibility].
        - `size_of_val`'s bound would be changed to `T: ?Sized + MetaSized`.
    - Attempts to sidestep backwards compatibility issues with introducing a
      default bound via changing what `?Sized` means across an edition boundary.
        - This [may be backwards incompatible](https://github.com/rust-lang/rfcs/pull/3396#issuecomment-1728509626).
    - Discussed in [a language team design meeting](https://hackmd.io/TSXpOX4iS3qqDdVD00z7tw?view).
- [rfcs#3536: Trait for `!Sized` thin pointers][rfc_not_sized_thin_pointers], [jmillikin][author_jmillikin], Nov 2023
    - Introduces unsafe trait `DynSized` with a `size_of_val` method.
        - It can be implemented on `!Sized` types. 
            - It is an error to implement it on `Sized` types.
        - References to types that implement `DynSized` do not need to store the
          size in pointer metadata. Types implementing `DynSized` without other pointer
          metadata are thin pointers.
    - This proposal has no solution for extern type limitations, its sole aim
      is to enable more pointers to be thin pointers.
- [Sized, DynSized, and Unsized][blog_dynsized_unsized], [Niko Matsakis][author_nikomatsakis], Apr 2024
    - This proposes a hierarchy of `Sized`, `DynSized` and `Unsized` traits
      like in this RFC and proposes deprecating `T: ?Sized` in place of `T: Unsized`
      and sometimes `T: DynSized`. Adding a bound for any of `DynSized` or `Unsized`
      removes the default `Sized` bound.
      - `DynSized` is the same as this RFC's `ValueSized`
      - `Unsized` is the same as this RFC's `Pointee`
    - As described below it is the closest inspiration for this RFC.

There are some even older RFCs that have tangential relevance that are listed
below but not summarized:

- [rfcs#5: virtual structs][rfc_virtual_structs], [nrc][author_nrc], Mar 2014
- [rfcs#9: RFC for "fat objects" for DSTs][rfc_fat_objects], [MicahChalmer][author_micahchalmer], Mar 2014
- [pre-RFC: unsized types][rfc_unsized_types], [japaric][author_japaric], Mar 2016

There haven't been any particular proposals which have included a solution for
runtime-sized types, as the scalable vector types proposal in [RFC 3268][rfc_scalable_vectors]
is relatively newer and less well known:

- [rfcs#3268: Add scalable representation to allow support for scalable vectors][rfc_scalable_vectors], [JamieCunliffe][author_jamiecunliffe], May 2022
    - Proposes temporarily special-casing scalable vector types to be able to
      implement `Copy` without implementing `Sized` and allows function return values
      to be `Copy` or `Sized` (not just `Sized`).
        - Neither of these changes would be necessary with this RFC, scalable
          vectors would just be `Sized` types (not `const Sized`) and function return
          values would continue to need to implement `Sized`.

To summarise the above exhaustive listing of prior art:

- No previous works have proposed an equivalent of the const size traits.
- One proposal proposed adding a marker type that as a field would result in the
  containing type no longer implementing `Sized`.
- Often proposals focused at Custom DSTs preferred to combine the
  escape-the-sized-hierarchy part with the Custom DST machinery.
    - e.g. `DynSized` trait with `Metadata` associated types and `size_of_val`
      and `align_of_val` methods, or a `!Sized` pseudo-trait that you could implement.
    - Given the acceptance of [rfcs#2580][rfc_pointer_metadata_vtable], Rust
      doesn't seem to be trending in this direction, as the `Metadata` part of this is
      now part of a separate `Pointee` trait.
- Most early `DynSized` trait proposals (independent or as part of Custom DSTs)
  would make `DynSized` a default bound mirroring `Sized`, and consequently had a
  relaxed form `?DynSized`.
    - Later proposals were more aware of the language team's resistance towards
      adding new relaxed bounds and tried to avoid this.
- Backwards compatibility concerns were the overriding reason for the rejection
  of previous `DynSized` proposals.
    - These can be sidestepped by avoiding having a relaxed form and by relying
      on being a supertrait of `Sized`.

The [Rationale and Alternatives](#rationale-and-alternatives) section provides
rationale for some of the decisions made in this RFC and references the prior
art above when those proposals made different decisions.

No previous proposal captures the specific part of the design space that this
proposal attempts to, but these proposals are the closest matches for parts of
this proposal:

- [Pre-eRFC: Let's fix DSTs][pre_erfc_fix_dsts] was the only other proposal
  removing `Sized` bounds when a bound for another sized trait (only `DynSized`
  in that pre-eRFC's case) was present, which makes reasoning simpler by avoiding
  relaxed bounds.
    - However, this proposal had `size_of_val` methods in its `DynSized` trait and
      proposed a bunch of other things necessary for Custom DSTs.
- [rfcs#2310: DynSized without ?DynSized][rfc_dynsized_without_dynsized] was
  proposed at a similar time and was similarly focused only on making `Sized` more
  flexible, but had a bunch of machinery for avoiding backwards incompatibility
  that this RFC believes is unnecessary. Like this proposal, it avoided making
  `DynSized` a default bound and avoided having a relaxed form of it.
    - However, this proposal didn't suggest removing default `Sized` bounds in
      the presence of other size trait bounds.
- [Sized, DynSized, and Unsized][blog_dynsized_unsized] is very similar and a
  major inspiration for this proposal. It has everything this proposal has except
  for the const size traits and all the additional context an RFC needs.

Some prior art referenced [rust#21974][issue_regions_too_simplistic] as a limitation
of the type system which can result in new implicit bounds or implicit supertraits
being infeasible for implementation reasons, but [it is believed that this is no
longer relevant][zulip_issue_regions_too_simplistic].

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None currently.

# Future possibilities
[future-possibilities]: #future-possibilities

- Additional size traits could be added as supertraits of `Sized` if there are
  other delineations in sized-ness that make sense to be drawn (subject to
  avoiding backwards-incompatibilities when changing APIs).
    - e.g. `MetaSized` from [rfcs#3396][rfc_extern_types_v2] could be added between
      `Sized` and `ValueSized`
- The requirement that users cannot implement any of these traits could be
  relaxed in future if required.
- Depending on a trait which has one of the proposed traits as a supertrait could
  imply a bound of the proposed trait, enabling the removal of boilerplate.
  - However, this would limit the ability to relax a supertrait, e.g. if
    `trait Clone: Sized` and `T: Clone` is used as a bound of a function
    and `Sized` is relied on in that function, then the supertrait of
    `Clone` could no longer be relaxed as it can today. 
- Consider allowing associated type bounds to be relaxed over an edition.
    - i.e. `type Output: if_rust_2021(Sized) + NewAutoTrait` or something like that,
      out of scope for this RFC.

## externref
[externref]: #externref

Another compelling feature that requires extensions to Rust's sizedness traits to
fully support is wasm's `externref`. `externref` types are opaque types that cannot
be put in memory [^7]. `externref`s are used as abstract handles to resources in the
host environment of the wasm program, such as a JavaScript object. Similarly, when
targetting some GPU IRs (such as SPIR-V), there are types which are opaque handles
to resources (such as textures) and these types, like wasm's `externref`, cannot
be put in memory.

[^7]: When Rust is compiled to wasm, we can think of the memory of the Rust program
as being backed by something like a `[u8]`, `externref`s exist outside of that `[u8]`
and there is no way to put an `externref` into this memory, so it is impossible to have
a reference or pointer to a `externref`. `wasm-bindgen` currently supports `externref`
by creating a array of the items which would be referenced by an `externref` on the
host side and passes indices into this array across the wasm-host boundary in lieu
of `externref`s. It isn't possible to support opaque types from some GPU targets using
this technique.

`externref` are similar to `Pointee` in that the type's size is not known, but unlike
`Pointee` cannot be used behind a pointer. This RFC's proposed hierarchy of traits could
support this by adding another supertrait, `Value`:

```
    ┌────────────────┐                  ┌─────────────────────────────┐
    │ const Sized    │ ───────────────→ │ Sized                       │
    │ {type, target} │     implies      │ {type, target, runtime env} │
    └────────────────┘                  └─────────────────────────────┘
            │                                          │
         implies                                    implies
            │                                          │
            ↓                                          ↓
┌───────────────────────┐             ┌────────────────────────────────────┐
│ const ValueSized      │ ──────────→ │ ValueSized                         │
│ {type, target, value} │   implies   │ {type, target, runtime env, value} │
└───────────────────────┘             └────────────────────────────────────┘
            │                                          │
         implies                                    implies
            │                                          │
            ↓                                          ↓
    ┌───────────────┐                         ┌──────────────────┐
    │ const Pointee │ ──────────────────────→ │ Pointee          │
    │ {*}           │         implies         │ {runtime env, *} │
    └───────────────┘                         └──────────────────┘
            │                                          │
         implies                                    implies
            │                                          │
            ↓                                          ↓
     ┌─────────────┐                          ┌──────────────────┐
     │ const Value │ ───────────────────────→ │ Value            │
     │ {*}         │         implies          │ {runtime env, *} │
     └─────────────┘                          └──────────────────┘
```

`Pointee` is still defined as being implemented for any type that can be used
behind a pointer and may not be sized at all, this would be implemented for
effectively every type except wasm's `externref` (or similar opaque types from
some GPU targets). `Value` is defined as being implemented for any type that can
be used as a value, which is all types, and also may not be sized at all.

Earlier in this RFC, `extern type`s have previously been described as not being
able to be used as a value, but it could instead be permitted to write functions
which use extern types as values (e.g. such as taking an extern type as an argument),
and instead rely on it being impossible to get a extern type that is not behind a
pointer or a reference. This also implies that `ValueSized` types can be used as values,
which would remain prohibited behind the `unsized_locals` and `unsized_fn_params`
features until these are stabilised.

With these changes to the RFC, it would be possible to support wasm's `externref` and
opaque types from some GPU targets.

## Alignment
[alignment]: #alignment

There has been community interest in an [`Aligned` trait][rfc_aligned] and there
are examples of `Aligned` traits being added in the ecosystem:

- `rustc` has [its own `Aligned` trait][rustc_aligned] to support pointer tagging.
- [`unsized-vec`][crate_unsized_vec] implements a `Vec` that depends on knowing
  whether a type has an alignment or not.

Furthermore, the existing incorrect behaviour of `align_of_val` returning one is
used in `rustc` with the [`OpaqueListContents` type][rustc_opaquelistcontents] to
implement a custom DST. This use case would break with this RFC as written, as
the alignment of an extern type would correctly be undefined.

An `Aligned` trait could be added to this proposal between `ValueSized` and `Pointee`
in the trait hierarchy which would be implemented automatically by the compiler for
all `ValueSized` and `Sized` types, but could be implemented for extern types by
users when the alignment of an extern type is known. Any type implementing `Aligned`
could be used as the last element in a compound type.

## Custom DSTs
[custom-dsts]: #custom-dsts

Given the community interest in supporting custom DSTs in future (see
[prior art][prior-art]), this RFC was written considering future-compatibility with
custom DSTs in mind.

There are various future changes to these traits which could be used to support
custom DSTs on top of this RFC. None of these have been considered thoroughly, and are
written here only to illustrate.

- Allow `Pointee` to be implemented manually on user types, which would replace 
  the compiler's implementation.
- Introduce a trait like [rfcs#2594][rfc_custom_dst_electric_boogaloo]'s `Contiguous`
  which users can implement on their custom DSTs, or add methods to `ValueSized` and
  allow it to be implemented by users.
- Introduce intrinsics which enable creation of pointers with metadata and for
  accessing the metadata of a pointer.

[api_align_of]: https://doc.rust-lang.org/std/mem/fn.align_of.html
[api_align_of_val]: https://doc.rust-lang.org/std/mem/fn.align_of_val.html
[api_box]: https://doc.rust-lang.org/std/boxed/struct.Box.html
[api_copy]: https://doc.rust-lang.org/std/marker/trait.Copy.html
[api_clone]: https://doc.rust-lang.org/std/clone/trait.Clone.html
[api_pointee]: https://doc.rust-lang.org/std/ptr/trait.Pointee.html
[api_sized]: https://doc.rust-lang.org/std/marker/trait.Sized.html
[api_size_of]: https://doc.rust-lang.org/std/mem/fn.size_of.html
[api_size_of_val]: https://doc.rust-lang.org/std/mem/fn.size_of_val.html
[author_aturon]: https://github.com/aturon
[author_cad97]: https://github.com/CAD97
[author_canndrew]: https://github.com/canndrew
[author_jamiecunliffe]: https://github.com/JamieCunliffe
[author_japaric]: https://github.com/japaric
[author_jmillikin]: https://github.com/jmillikin
[author_joshtriplett]: https://github.com/joshtriplett
[author_jules_bertholet]: https://github.com/Jules-Bertholet
[author_kennytm]: https://github.com/kennytm
[author_micahchalmer]: https://github.com/MicahChalmer
[author_mikeyhew]: https://github.com/mikeyhew
[author_mystor]: https://github.com/mystor
[author_mzabaluev]: https://github.com/mzabaluev 
[author_nikomatsakis]: https://github.com/nikomatsakis
[author_nox]: https://github.com/nox
[author_nrc]: https://github.com/nrc
[author_plietar]: https://github.com/plietar
[author_pnkfelix]: https://github.com/pnkfelix
[author_skepfyr]: https://github.com/Skepfyr
[author_strega_nil]: https://github.com/strega-nil
[author_ubsan]: https://github.com/ubsan
[blog_dynsized_unsized]: https://smallcultfollowing.com/babysteps/blog/2024/04/23/dynsized-unsized/
[changing_rules_of_rust]: https://without.boats/blog/changing-the-rules-of-rust/
[crate_unsized_vec]: https://docs.rs/unsized-vec/0.0.2-alpha.7/unsized_vec/
[design_meeting]: https://hackmd.io/7r3_is6uTz-163fsOV8Vfg
[design_notes_dynsized_constraints]: https://github.com/rust-lang/lang-team/blob/master/src/design_notes/dynsized_constraints.md
[erfc_minimal_custom_dsts_via_extern_type]: https://internals.rust-lang.org/t/erfc-minimal-custom-dsts-via-extern-type-dynsized/16591?u=cad97
[goal_const_traits]: https://rust-lang.github.io/rust-project-goals/2024h2/const-traits.html
[issue_extern_types_align_size]: https://github.com/rust-lang/rust/issues/49708
[issue_more_implicit_bounds]: https://github.com/rust-lang/rfcs/issues/2255
[issue_regions_too_simplistic]: https://github.com/rust-lang/rust/issues/21974#issuecomment-331886186
[issue_tracking_extern_types]: https://github.com/rust-lang/rust/issues/43467
[issue_truly_unsized_types]: https://github.com/rust-lang/rfcs/issues/813
[pr_dynsized]: https://github.com/rust-lang/rust/pull/44469
[pr_dynsized_rebase]: https://github.com/rust-lang/rust/pull/46108
[pre_erfc_fix_dsts]: https://internals.rust-lang.org/t/pre-erfc-lets-fix-dsts/6663
[prerfc_custom_dst]: https://internals.rust-lang.org/t/pre-rfc-custom-dsts/8777
[rfc_aligned]: https://github.com/rust-lang/rfcs/pull/3319
[rfc_custom_dst_electric_boogaloo]: https://github.com/rust-lang/rfcs/pull/2594
[rfc_custom_dst]: https://github.com/rust-lang/rfcs/pull/1524
[rfc_dynsized_without_dynsized]: https://github.com/rust-lang/rfcs/pull/2310
[rfc_extern_types]: https://rust-lang.github.io/rfcs/1861-extern-types.html
[rfc_extern_types_v2]: https://github.com/rust-lang/rfcs/pull/3396
[rfc_fat_objects]: https://github.com/rust-lang/rfcs/pull/9
[rfc_not_sized_thin_pointers]: https://github.com/rust-lang/rfcs/pull/3536
[rfc_opaque_data_structs]: https://github.com/rust-lang/rfcs/pull/1993
[rfc_pointee_dynsized]: https://github.com/rust-lang/rfcs/pull/2984
[rfc_pointer_metadata_vtable]: https://github.com/rust-lang/rfcs/pull/2580
[rfc_scalable_vectors]: https://github.com/rust-lang/rfcs/pull/3268
[rfc_simd]: https://rust-lang.github.io/rfcs/1199-simd-infrastructure.html
[rfc_truly_unsized_types]: https://github.com/rust-lang/rfcs/pull/709
[rfc_unsized_types]: https://github.com/japaric/rfcs/blob/unsized2/text/0000-unsized-types.md
[rfc_virtual_structs]: https://github.com/rust-lang/rfcs/pull/5
[rustc_aligned]: https://github.com/rust-lang/rust/blob/a76ec181fba25f9fe64999ec2ae84bdc393560f2/compiler/rustc_data_structures/src/aligned.rs#L22-L25
[rustc_opaquelistcontents]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/ty/list/foreigntype.OpaqueListContents.html
[zulip_issue_regions_too_simplistic]: https://rust-lang.zulipchat.com/#narrow/channel/144729-t-types/topic/.2321984.20.2B.20implicit.20supertraits.20-.20still.20relevant.3F/near/477630998
