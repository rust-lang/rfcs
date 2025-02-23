- Feature Name: `sized_hierarchy`
- Start Date: 2024-09-30
- RFC PR: [rust-lang/rfcs#3729](https://github.com/rust-lang/rfcs/pull/3729)
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

## Dependencies
[dependencies]: #dependencies

This RFC is written assuming [const traits (RFC #3762)][rfc_const_traits] is
accepted, but ultimately does not strictly depend on it. Almost everything
proposed could be represented in a Rust without const traits - this is
described in the [*Why use const traits?*][why-use-const-traits] section.

However, const traits is a very natural way to express the concepts from this
proposal and it is the opinion of the author that proposing/accepting a
non-const traits version of this proposal when const traits are a likely
addition to the language would be misguided.

The [*Why use const traits?*][why-use-const-traits] section is included solely
to demonstrate that this RFC does not strictly depend on const traits.

# Background
[background]: #background

Rust has the [`Sized`][api_sized] marker trait which indicates that a type's size
is statically known at compilation time. `Sized` is a trait which is automatically
implemented by the compiler on any type that has a statically known size. All type
parameters have a default bound of `Sized` and `?Sized` syntax can be used to remove
this bound.

There are two functions in the standard library which can be used to get a size,
[`std::mem::size_of`][api_size_of] and [`std::mem::size_of_val`][api_size_of_val]:

```rust
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
  and all of its supertraits but none of its subtraits. For example, a `MetaSized`
  type would be a type which implements `MetaSized`, and `Pointee`, but not
  `Sized`. `[usize]` would be referred to as a "`MetaSized` type".
- "Runtime-sized" types will be used those types whose size is a runtime constant
  and unknown at compilation time. These would include the scalable vector types
  mentioned in the motivation below, or those that implement `Sized` but not
  `const Sized` in the RFC.
- The bounds on the generic parameters of a function may be referred to simply
  as the bounds on the function (e.g. "the caller's bounds").

## Acknowledgements
[acknowledgements]: #acknowledgements

This RFC wouldn't have been possible without the reviews and feedback of
[@JamieCunliffe][author_jamiecunliffe], [@JacobBramley][ack_jacobbramley],
[@nikomatsakis][author_nikomatsakis] and [@scottmcm][ack_scottmcm];
[@eddyb][ack_eddyb] for the `externref` future possibility; the expertise
of [@compiler-errors][ack_compiler_errors] on the type system and suggesting
the use of const traits; [@fee1-dead][ack_fee1dead] for reviewing the usage
of const traits and fixing the ASCII diagrams; and the authors of all of the
prior art for influencing these ideas.

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
stack, etc. These types should implement `Copy` but given that `Sized` is a
supertrait of `Copy`, they cannot be `Copy` without being `Sized`, and
they aren't `Sized`.

Making `Sized` a const trait will enable `Copy` to be implemented for
those types whose size is a runtime constant to function correctly without
special cases in the type system. See
[rfcs#3268: Scalable Vectors][rfc_scalable_vectors].

## Unsized types and extern types
[unsized-types-and-extern-types]: #unsized-types-and-extern-types

[Extern types][rfc_extern_types] has long been blocked on these types being
neither `Sized` nor `?Sized` ([relevant issue][issue_extern_types_align_size]).
Extern types are listed as a "nice to have" feature in [Rust for Linux's requests
of the Rust project][rfl_want].

RFC #1861 defined that `std::mem::size_of_val` and `std::mem::align_of_val`
should not be defined for extern types but not how this should be achieved, and
suggested an initial implementation could panic when called with extern types,
but this is always wrong, and not desirable behavior in keeping with Rust's
values. `size_of_val` and `align_of_val` both panic for extern types in
the current implementation. Ideally `size_of_val` and `align_of_val` would error
if called with an extern type, but this cannot be expressed in the bounds of
`size_of_val` and `align_of_val` and this remains a blocker for extern types.

Furthermore, unsized types can only be the final member of structs as their
size is unknown and this is necessary to calculate the offsets of later fields.
Extern types also cannot be used in `Box` as `Box` requires size and alignment
for both allocation and deallocation.

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
  known size[^1], which dynamically-sized and unsized types do not have (but 
  sized and "runtime-sized" types do).

[^1]: Dynamic stack allocation does exist, such as in C's Variable Length Arrays
      (VLA), but not in Rust (without incomplete features like `unsized_locals`
      and `unsized_fn_params`).

Rust uses marker traits to indicate the necessary knowledge required to know
the size of a type, if it can be known. There are three traits related to the size
of a type in Rust: `Sized`, `MetaSized`, and `Pointee`. `Sized` and
`MetaSized` can be implemented as `const` when the size is knowable at compilation
time.

`Sized` is a subtrait of `MetaSized`, so every type which implements `Sized`
also implements `MetaSized`. Likewise, `MetaSized` is a subtrait of `Pointee`.
`Sized` is `const` if-and-only-if `MetaSized` is `const`.

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│ ┌──────────────────────────────────────────────────────────────────────────────┐ │
│ │ ┌──────────────────────────────────────────────────────────────────────────┐ │ │
│ │ │ ┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓                                         │ │ │
│ │ │ ┃ ┏━━━━━━━━━━━━━━━━━━━━━━━━━━┓ ┃                                         │ │ │
│ │ │ ┃ ┃ const Sized              ┃ ┃ Sized                                   │ │ │
│ │ │ ┃ ┃ {type, target}           ┃ ┃ {type, target, runtime env}             │ │ │
│ │ │ ┃ ┗━━━━━━━━━━━━━━━━━━━━━━━━━━┛ ┃                                         │ │ │
│ │ └─╂──────────────────────────────╂─────────────────────────────────────────┘ │ │
│ │   ┃                              ┃                                           │ │
│ │   ┃ const MetaSized              ┃ MetaSized                                 │ │
│ │   ┃ {type, target, ptr metadata} ┃ {type, target, ptr metadata, runtime env} │ │
│ │   ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛                                           │ │
│ └──────────────────────────────────────────────────────────────────────────────┘ │
│  Pointee                                                                         │
│  {*}                                                                             │
└──────────────────────────────────────────────────────────────────────────────────┘
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

`const MetaSized` requires more knowledge than `const Sized` to compute the size:
it may additionally require pointer metadata (therefore `size_of` is not implemented
for `MetaSized`, only `size_of_val`). For example, `[usize]` implements
`const MetaSized` as knowing the type and target is not sufficient, the number of
elements in the slice must also be known, which requires reading the pointer
metadata.

As `Sized` is to `const Sized`, `MetaSized` is to `const MetaSized`: `MetaSized`
requires pointer metadata, knowledge of the type and target platform, and can
only be computed at runtime. For example, the size of `[svint8_t]` is the number of
elements multiplied by the size of an element - pointer metadata is required to know
how many elements there are, and then information from the runtime environment to
know the size of a `svint8_t` element.

`Pointee` is implemented by any type that can be used behind a pointer, which is
to say, every type (put otherwise, these types may or may not be sized at all).
For example, `Pointee` is therefore implemented on a `u32` which is trivially
sized, a `[usize]` which is dynamically sized, a `svint8_t` which is
runtime-sized and an `extern type` (from [rfcs#1861][rfc_extern_types]) which has
no known size.

All type parameters have an implicit bound of `const Sized` which will be
automatically removed if a `~const Sized`, `Sized`, `const MetaSized`,
`~const MetaSized`, `MetaSized` or `Pointee` bound is present instead.

Prior to the introduction of `MetaSized` and `Pointee`, `Sized`'s implicit bound
(now a `const Sized` implicit bound) could be removed using the `?Sized` syntax,
which is now equivalent to a `MetaSized` bound in non-`const fn`s and
`~const MetaSized` in `const fn`s and will be deprecated in the next edition.

Traits now have an implicit default bound on `Self` of `const MetaSized`.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Introduce new marker traits, `MetaSized` and `Pointee`, adding it to a trait
hierarchy with the [`Sized`][api_sized] trait, and make `Sized` and `MetaSized`
traits `const`:

```
       ┌────────────────┐                          ┌─────────────────────────────┐
       │ const Sized    │ ───────────────────────→ │ Sized                       │
       │ {type, target} │         implies          │ {type, target, runtime env} │
       └────────────────┘                          └─────────────────────────────┘
               │                                                  │
            implies                                            implies
               │                                                  │
               ↓                                                  ↓
┌──────────────────────────────┐             ┌───────────────────────────────────────────┐
│ const MetaSized              │ ──────────→ │ MetaSized                                 │
│ {type, target, ptr metadata} │   implies   │ {type, target, ptr metadata, runtime env} │
└──────────────────────────────┘             └───────────────────────────────────────────┘
                                                                  │
                                                               implies
                                                                  │
                                      ┌───────────────────────────┘
                                      ↓
                            ┌──────────────────┐
                            │ Pointee          │
                            │ {runtime env, *} │
                            └──────────────────┘
```

Or, in Rust syntax:

```rust
#![feature(const_trait_impl)]

#[const_trait] trait Sized: ~const MetaSized {}

#[const_trait] trait MetaSized: Pointee {}

trait Pointee {}
```

## Implementing `Sized`
[implementing-sized]: #implementing-sized

Implementations of the proposed traits are automatically generated by
the compiler and cannot be implemented manually:

- `Pointee`
    - Types that which can be used from behind a pointer (they may or may
      not have a size).
    - `Pointee` will be implemented for:
        - `MetaSized` and `const MetaSized` types
        - `extern type`s from [rfcs#1861][rfc_extern_types]
        - compound types where any element is `Pointee`
    - In practice, every type will implement `Pointee`.
- `MetaSized`
    - Types whose size is computable given pointer metadata, and knowledge of
      the type, target platform and runtime environment.
      - `const MetaSized` does not require knowledge of the runtime
        environment
    - `MetaSized` is a subtrait of `Pointee`
    - `MetaSized` will be implemented for:
        - `Sized` types
        - slices `[T]` where `T` is `Sized`
        - compound types where any element is `MetaSized`
    - `const MetaSized` will be implemented for:
        - `const Sized` types
        - slices `[T]` where `T` is `const Sized`
        - string slice `str`
        - trait objects `dyn Trait`
        - compound types where any element is `const MetaSized`
- `Sized`
    - Types whose size is computable given knowledge of the type, target
      platform and runtime environment.
      - `const Sized` does not require knowledge of the runtime environment
    - `Sized` is a subtrait of `MetaSized`
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
    - Non-const `Sized` types have the same properties as `const Sized` types
        - i.e. `Sized` types..
            - ..can be returned from functions
            - ..can be passed to functions by value, reference or pointer
            - ..can exist as local values in the stack
            - ..can be the types of static variables
            - ..can be loaded/stored to/from memory for spilling to the stack and
              to follow calling conventions
            - ..can be fields in abstract data types
            - ..can be the implementor of traits
        - Specific `Sized` types, such as scalable vectors using `repr(scalable)`
          from [rfcs#3268][rfc_scalable_vectors], may impose further restrictions
          such as being unable to be used in abstract data types or be used as
          static variables
    - Types implementing `Sized` will not require special machinery in the compiler,
      such as `unsized_locals` or `unsized_fn_params`, to be considered `Sized`

Introducing new automatically implemented traits is backwards-incompatible,
at least if you try to add it as a bound to an existing function[^2][^3] (and
new auto traits that which go unused aren't that useful), but due to being
supertraits of `Sized` and `Sized` being a default bound, these
backwards-incompatibilities are avoided for `MetaSized` and `Pointee`.

As an implementation detail, `Pointee` could be treated as syntax for expressing
a lack of any bounds on the sizedness of a parameter, rather than being an additional
obligation to be proven.

Relaxing a bound from `Sized` to `MetaSized` or `Pointee` is non-breaking as
the calling bound must have either `T: Sized` or `T: ?Sized`, both of which
would satisfy any relaxed bound[^4]. A parameter bounded by `Sized` and used
as the return type of a function could not be relaxed as function return types
would still need to implement `Sized`.

However, it is not backwards compatible to relax the bounds of trait methods[^5]
and it would still be backwards-incompatible to relax the `Sized` bound on
a trait's associated type[^6] for the proposed traits.

It is possible further extend this hierarchy in future by adding new traits before,
between, or after the traits proposed in this RFC without breaking backwards
compatibility, depending on the bounds that would be introduced[^7].

[^2]: Adding a new automatically implemented trait and adding it as a bound to
      an existing function is backwards-incompatible with generic functions. Even
      though all types could implement the trait, existing generic functions will be
      missing the bound.

      If `Foo` were introduced to the standard library and implemented on every
      type, and it was added as a bound to `size_of` (or any other generic
      parameter)..

      ```rust
      auto trait Foo {}

      fn size_of<T: Sized + Foo>() { /* .. */ } // `Foo` bound is new!
      ```

      ...then user code would break:

      ```rust
      fn do_stuff<T>(value: T) { size_of(value) }
      // error! the trait bound `T: Foo` is not satisfied
      ```
[^3]: Trait objects passed by callers would not imply the new trait.

      If `Foo` were introduced to the standard library and implemented on every
      type, and it was added as a bound to `size_of_val` (or any other generic
      parameter)..

      ```rust
      auto trait Foo {}

      fn size_of_val<T: ?Sized + Foo>(x: val) { /* .. */ } // `Foo` bound is new!
      ```

      ...then user code would break:

      ```rust
      fn do_stuff(value: Box<dyn Display>) { size_of_val(value) }
      // error! the trait bound `dyn Display: Foo` is not satisfied in `Box<dyn Display>`
      ```
[^4]: Callers of existing APIs will have one of the following `Sized` bounds:

      | Before ed. migration              | After ed. migration  |
      | --------------------------------- | -------------------- |
      | `T: Sized` (implicit or explicit) | `T: const Sized`     |
      | `T: ?Sized`                       | `T: const MetaSized` |

      Any existing free function in the standard library with a `T: Sized` bound
      could be changed to one of the following bounds and remain compatible with
      any callers that currently exist (as per the above table):

      |                | `const Sized` | `Sized` | `const MetaSized` | `MetaSized` | `Pointee`
      | -------------- | ------------- | ------- | ----------------- | ----------- | ---------
      | `const Sized`  | ✔             | ✔       | ✔                 | ✔           | ✔

      Likewise with a `T: ?Sized` bound:

      |                   | `const MetaSized` | `MetaSized` | `Pointee`
      | ----------------- | ----------------- | ----------- | ---------
      | `const Sized`     | ✔                 | ✔           | ✔
      | `const MetaSized` | ✔                 | ✔           | ✔
[^5]: In a crate defining a trait which has method with sizedness bounds, such
      as...

      ```rust
      trait Foo {
          fn bar<T: Sized>(t: T) -> usize;
          fn baz<T: ?Sized>(t: T) -> usize;
      }
      ```

      ..then an implementor of `Foo` may rely on the existing bound and
      these implementors of `Foo` would break if the bounds of `bar` or
      `baz` were relaxed.

      ```rust
      struct Qux;

      impl Foo for Qux {
          fn bar<T: Sized>(_: T) -> usize { std::mem::size_of<T> }
          fn baz<T: ?Sized>(t: T) -> usize { std::mem::size_of_val(t) }
      }
      ```
[^6]: Associated types of traits have default `Sized` bounds which cannot be
      relaxed. For example, relaxing a `Sized` bound on `Add::Output` breaks
      a function which takes a `T: Add` and passes `<T as Add>::Output` to
      `size_of` as not all types which implement the relaxed bound will
      implement `Sized`.

      If a default `Sized` bound on an associated trait, such as
      `Add::Output`, were relaxed in the standard library...

      ```rust
      trait Add<Rhs = Self> {
          type Output: MetaSized;
      }
      ```

      ...then user code would break:

      ```rust
      fn do_stuff<T: Add>() -> usize { std::mem::size_of::<<T as Add>::Output>() }
      //~^ error! the trait bound `<T as Add>::Output: Sized` is not satisfied
      ```

      Relaxing the bounds of an associated type is in effect giving existing
      parameters a less restrictive bound which is not backwards compatible.
[^7]: If it was desirable to add a new trait to this RFC's proposed hierarchy
      then there are four possibilities:

      - Before `Sized`
        - i.e. `NewSized: Sized: MetaSized: Pointee`
        - `NewSized` would need to replace `Sized` as a default bound over an
          edition to avoid this being a breaking change.
      - Between `Sized` and `MetaSized`
        - i.e. `Sized: NewSized: MetaSized: Pointee`
        - Adding bounds on a new trait between `Sized` and `MetaSized` is
          backwards compatible for some functions (except for on a trait's
          associated type). An existing function with a `Sized` bound could
          be relaxed to the new trait without breaking existing code.
          Existing functions with a `MetaSized` or `Pointee` bound could not.
      - Between `MetaSized` and `Pointee`
        - i.e. `Sized: MetaSized: NewSized: Pointee`
        - Adding bounds on a new trait between `MetaSized` and `Pointee` is
          backwards compatible for some functions (except for on a trait's
          associated type). An existing function with a `Sized` or `MetaSized`
          bound could be relaxed to the new trait without breaking existing code.
          Existing functions with a `Pointee` bound could not.
      - After `Pointee`
        - i.e. `Sized: MetaSized: Pointee: NewSized`
        - For the same reasons as the traits proposed in this RFC, adding bounds
          on a new trait after `Pointee` is backwards compatible (except for on a
          trait's associated type). Any existing function will have stricter
          bounds than the new trait and so could be relaxed to the new trait
          without breaking existing code.

## `Sized` bounds
[sized-bounds]: #sized-bounds

`?Sized` would be made syntactic sugar for a `const MetaSized` bound. A
`const MetaSized` bound is equivalent to a `?Sized` bound as all values in Rust
today whose types do not implement `Sized` are valid arguments to
[`std::mem::size_of_val`][api_size_of_val] and as such have a size which can be
computed given pointer metadata and knowledge of the type and target platform, and
therefore will implement `const MetaSized`. As there are currently no
extern types or other types which would not implement `const MetaSized`,
every type in Rust today which would satisfy a `?Sized` bound would satisfy
a `const MetaSized` bound.

**Edition change:** In the current edition,`?Sized` will be syntatic sugar for
a `const MetaSized` bound. The `?Trait` syntax is currently an error for any trait
except `Sized`, which would continue. In the next edition, any uses of `?Sized`
syntax will be rewritten to a `const MetaSized` bound. Any other uses of the
`?Trait` syntax will be removed as part of the migration and the `?Trait` syntax
will be prohibited.

A default implicit bound of `const Sized` is added by the compiler to every type
parameter `T` that does not have an explicit `~const Sized`, `Sized`, `?Sized`,
`const MetaSized`, `~const MetaSized`, `MetaSized` or `Pointee` bound. It is
backwards compatible to change the current implicit `Sized` bound to an `const Sized`
bound as every type which exists currently will implement `const Sized`.

**Edition change:** In the current edition, all existing `Sized` bounds will be
syntatic sugar for `const Sized` (both explicitly written and implicit). In the next
edition, existing `Sized` bounds will be rewritten to `const Sized` bounds and new
bare `Sized` bounds will be non-const. `Sized` supertraits (such as that on `Clone`
or `Default`) will not be sugar for `const Sized`, these will remain bare `Sized`.
If traits with a `Sized` supertrait are later made const, then their supertrait
would be made `~const Sized`. In the current edition, due to this proposed migration, there
is no syntax for referring to non-const `Sized`.

As `MetaSized` and `Pointee` are not default bounds, there is no equivalent to `?Sized`
for these traits.

**Edition change:** In the current edition, new marker traits would not be added
to the prelude.

### Implicit `const MetaSized` supertraits
[implicit-const-metasized-supertraits]: #implicit-const-metasized-supertraits

It is necessary to introduce a implicit default bound of `const MetaSized` on a trait's
`Self` type in order to maintain backwards compatibility in the current edition (referred
to as an implicit supertrait hereafter for brevity). Like implicit `const Sized` bounds,
this is omitted if an explicit `const Sized`, `Sized`, `MetaSized` or `Pointee` bound is
present.

Without this implicit supertrait, the below example would no longer compile: `needs_drop`'s
`T: ?Sized` would be migrated to a `const MetaSized` bound which is not guaranteed to
be implemented by `Foo`.

```rust
trait Foo {
    fn implementor_needs_dropped() -> bool {
        // `fn needs_drop<T: ?Sized>() -> bool`
        std::mem::needs_drop::<Self>() // error! `Self: const MetaSized` is not satisfied
    }
}
```

With the implicit supertrait, the above example would be equivalent to the following
example, which would compile successfully.

```rust
trait Foo: const MetaSized {
    fn implementor_needs_dropped() -> bool {
        // `fn needs_drop<T: ?Sized>() -> bool`
        std::mem::needs_drop::<Self>() // ok!
    }
}
```

For the same reasons that `?Sized` is equivalent to `const MetaSized`, adding
a `const MetaSized` implicit supertrait will not break any existing implementations
of traits as every existing type already implements `const MetaSized`.

**Edition change:** In the current edition, a default `const MetaSized` supertrait is
added. In the next edition, no default supertrait is added. During edition migration,
`const MetaSized` is explicitly added as a supertrait to all existing traits not explicitly
relaxed further. By avoiding having a default supertrait in the next edition, new traits
will be implementable on `Pointee` types.

This implicit supertrait could be relaxed without breaking changes within the standard
library and in third party crates.

If the implicit supertrait was strengthened to a `Sized` supertrait, it would be a
breaking change as that trait could be being implemented on a type which does not
implement `Sized` - this is true regardless of whether there is an implicit supertrait
and adding a `Sized` supertrait to a trait without one would be a breaking change today.

If the implicit supertrait was weakened to a `MetaSized` or `Pointee` supertrait
then this would not break any existing callers using this trait as a bound - any
parameters bound by this trait must also have either a `?Sized`/`const MetaSized` bound
or a `Sized` bound which would ensure any existing uses of `size_of_val` (or other
functions taking `?Sized`/`const MetaSized`) continue to compile.

In the below example, if an existing trait `Foo`'s implicit `const MetaSized`
supertrait was relaxed to `Pointee` or `MetaSized` then its uses would continue
to compile:

```rust
trait Foo: Pointee {} // or `MetaSized`
//         ^^^^^^^ new!

fn foo<T: Foo>(t: &T) -> usize { size_of_val(t) }
fn foo_unsized<T: const MetaSized + Foo>(t: &T) -> usize { size_of_val(t) }
```

Once users can write `Pointee` or `MetaSized` bounds and then it is possible for users
to write functions which would no longer compile if the function was relying on the
implicit supertrait of another bounded trait which was then relaxed:

```rust
// This only compiled because `Foo: const MetaSized`, but if that bound were relaxed
// then it would fail to compile.
fn foo<T: Pointee + Foo>(t: &T) -> usize { size_of_val(t) }
```

Implementations of traits in would also not be broken when an implicit supertrait
is relaxed.

Any existing implementation of a trait will be on a type which implement at least
`const MetaSized`, therefore a relaxation of the implicit supertrait to `MetaSized`
or `Pointee` will be trivially satisfied by any existing implementor.

```rust
struct Local;

impl Foo for Local {} // not broken!
```

In the bodies of trait implementations, the only circumstance in which there
could be a backwards incompatibility due to relaxation of the implicit supertrait
is when the sizedness traits implemented by `Self` can be observed - there are
three cases which must be considered: in trait implementations, trait definitions
and subtraits.

In trait implementations, `Self` refers to the specific implementing type, this
could be a concrete type like `u32` or it could be a generic parameter in a
blanket impl. In either case, the type is guaranteed to implement `const Sized`
or `const MetaSized` as no types which do not implement one of these two traits
currently exist.

```rust
impl Foo for u32 {
    fn example(t: &Self) -> usize { std::mem::size_of_val(t) }
    // `Self` = `u32`, even if `Foo`'s implicit supertrait is relaxed, `u32` still
    // implements `const MetaSized`
}

impl<T> Foo for T {
    fn example(t: &Self) -> usize { std::mem::size_of_val(t) }
    // `Self` = `T`, even if `Foo`'s implicit supertrait is relaxed, `T` still
    // implements `Sized` because of the default bound
}

impl<T: ?Sized> Foo for T {
    fn example(t: &Self) -> usize { std::mem::size_of_val(t) }
    // `Self` = `T`, even if `Foo`'s implicit supertrait is relaxed, `T` still
    // implements `const MetaSized` because of the default bound
}
```

Trait definitions are unlike trait implementations in that `Self` in their bodies
always refers to any possible implementor and the only known bounds on that `Self`
are the supertraits of the trait. A default body of a method can test whether `Self`
implements any given sizedness trait (e.g. by calling `needs_drop::<Self>()` as in the
examples at the start of this section). However, trait definitions can be updated when
an implicit supertrait is relaxed so do not pose any risk of breakage.

Like with trait definitions above, a subtrait defined in a downstream crate can
observe the sizedness traits implemented by `Self` in default bodies. However,
subtraits will also have an implicit `const MetaSized` supertrait which would
guarantee that their bodies continue to compile if a supertraits relaxed its implicit
supertrait:

```rust
trait Sub: Foo { // equiv to `Sub: Foo + const MetaSized`
    // `fn needs_drop<T: ?Sized>() -> bool`
    fn example() -> bool { std::mem::needs_drop::<Self>() } // ok!
}
```

In many of the cases above, relaxation of the supertrait is only guaranteed
to be backwards compatible in third party crates while there is no user code using
the new traits this proposal introduces.

### Summary of bounds changes
[summary-of-bounds-changes]: #summary-of-bounds-changes

To summarise, in the current edition, the following bounds will be treated as
equivalent to:

| Written           | Interpretation    | Notes                                                                                |
| ----------------- | ----------------- | ------------------------------------------------------------------------------------ |
| `const Sized`     | `const Sized`     |                                                                                      |
| `Sized`           | `const Sized`     | Necessary for backwards compatibility, later rewritten to `const Sized` by `rustfix` |
| `?Sized`          | `const MetaSized` | Necessary for backwards compatibility, later prohibited                              |
| `const MetaSized` | `const MetaSized` |                                                                                      |
| `MetaSized`       | `MetaSized`       |                                                                                      |
| `Pointee`         | `Pointee`         |                                                                                      |

After the aforementioned edition migration, the following bounds will be
treated as equivalent to:

| Written           | Interpretation    |
| ----------------- | ----------------- |
| `const Sized`     | `const Sized`     |
| `Sized`           | `Sized`           |
| `?Sized`          | Prohibited        |
| `const MetaSized` | `const MetaSized` |
| `MetaSized`       | `MetaSized`       |
| `Pointee`         | `Pointee`         |

## `size_of` and `size_of_val`
[size-of-and-size-of-val]: #size_of-and-size_of_val

Runtime-sized types should be able to be passed to both `size_of` and `size_of_val`,
but only at runtime, which requires ensuring these functions are only const if their
arguments have const implementations of the relevant sizedness traits.

[`size_of`][api_size_of] is a const-stable function since Rust 1.24.0 and currently
accepts a `T: Sized` bound. Therefore, even when used in a const context, `size_of`
could accept a runtime-sized type. It is therefore necessary to modify the bounds of
`size_of` to accept a `T: ~const Sized`, so that `size_of` is a const function
if-and-only-if `Sized` has a `const` implementation.

```rust
pub const fn size_of<T: ~const Sized>() -> usize {
    /* .. */
}
```

This has the potential to break existing code like `uses_size_of` in the below
example. However, due to the changes described in [`Sized` bounds][sized-bounds]
(changing the implicit `T: Sized` to `T: const Sized`, and treating explicit
`T: Sized` bounds as `T: const Sized` in the current edition), this code would
not break:

```rust
fn uses_size_of<T: Sized>() -> usize {
    const { std::mem::size_of<T>() }
}

fn another_use_of_size_of<T: Sized>() -> [u8; size_of::<T>()] {
    std::array::repeat(0)
}
```

It could make sense to delay relaxation of `size_of`'s bound from `T: const Sized`
(post-migration) to `T: ~const Sized`: the only non-const `Sized` types which
this relaxation would impact are architecture-specific scalable vectors. Unless these
types were behind `cfg` attributes (which limits their use when target features are
dynamically detected) then they could be used with `size_of` on unsupported targets.
Delaying this relaxation would have no impact on existing code and `size_of_val`
could still be used for scalable vectors when introduced.

[`size_of_val`][api_size_of_val] is also const-stable, so like `size_of` above,
its bound should be changed to `T: ~const MetaSized` and this would not result in
any breakage due to the previously described edition migration.

```rust
pub const fn size_of_val<T>(val: &T) -> usize
where
    T: ~const MetaSized,
{
    /* .. */
}
```

While `MetaSized` is equivalent to the current `?Sized` bound it replaces, it
excludes extern types (which `?Sized` by definition cannot), which prevents
`size_of_val` from being called with extern types from [rfcs#1861][rfc_extern_types].
Due to the changes described in [`Sized` bounds][sized-bounds] (migrating
`T: ?Sized` to `T: const MetaSized`), changing the bound of `size_of_val` will
not break any existing callers.

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
`Sized`, `const MetaSized` and `MetaSized` types can be used in compound types, but
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

Traits which are a subtrait of any of the proposed traits will not
automatically imply the proposed trait in any bounds where the trait is
used, e.g.

```rust
trait NewTrait: MetaSized {}

// Subtractive case (adding a trait bound will not weaken the existing bounds)
struct NewRc<T: NewTrait> {} // equiv to `T: NewTrait + Sized` as today

// Additive case (adding a trait bound can strengthen the existing bounds)
struct NewRc<T: Pointee + NewTrait> {} // equiv to `T: NewTrait + MetaSized` as today
```

It remains the case with this proposal that if the user wanted `T: MetaSized`
then it would need to be written explicitly.

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
`const Sized`, `~const MetaSized` or `MetaSized` bounds (after edition migration) would
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
    - `T: ?Sized` becomes `T: MetaSized`
    - It is not a breaking change to relax this bound and it prevents types
      only implementing `Pointee` from being used with `Box`, as these types
      do not have the necessary size and alignment for allocation/deallocation.
- [`std::marker::PhantomData`][api_phantomdata]
    - `T: ?Sized` becomes `T: Pointee`
    - It is not a breaking change to relax this bound and there's no reason why
      any type should not be able to be used with `PhantomData`.

As part of the implementation of this RFC, each `Sized`/`?Sized` bound in
the standard library would need to be reviewed and updated as appropriate.

## Summary of backwards (in)compatibilities
[summary-of-backwards-incompatibilities]: #summary-of-backwards-incompatibilities

In the above sections, this proposal argues that..

- ..adding bounds of new automatically implemented supertraits of a default bound..
  - see [*Implementing `Sized`*][implementing-sized].
- ..relaxing a sizedness bound in a free function..
  - see [*Implementing `Sized`*][implementing-sized].
- ..relaxing implicit sizedness supertraits..
  - see [*Implicit `const MetaSized` supertraits*][implicit-const-metasized-supertraits].

..is backwards compatible and that..

- ..relaxing a sizedness bound for a generic parameter used as a return type..
  - see [*Implementing `Sized`*][implementing-sized].
- ..relaxing a sizedness bound in a trait method..
  - see [*Implementing `Sized`*][implementing-sized].
- ..relaxing the bound on an associated type..
  - see [*Implementing `Sized`*][implementing-sized].

..is backwards incompatible.

## Incremental stabilisation
[incremental-stabilisation]: #incremental-stabilisation

It is possible to stabilise these traits (or their constness) independently
of each another.

This would have the obvious implications of limiting which bounds a user
can write, but there is no technical limitation on stabilisation.

# Drawbacks
[drawbacks]: #drawbacks

- This is a fairly significant change to the `Sized` trait, which has been in
  the language since 1.0 and is now well-understood.
- This RFC's proposal that adding a bound of `const Sized`, `const MetaSized`,
  `MetaSized` or `Pointee` would remove the default `Sized` bound is a significant
  change from the current `?Sized` mechanism and can be considered confusing.
    - Typically adding a trait bound does not remove another trait bound, however
      this RFC argues that this behaviour scales better to hierarchies of traits
      with default bounds and constness.
- As this RFC depends on `const Trait`, it inherits all of the drawbacks of
  `const Trait`.

## Backwards Incompatibility
[backwards-incompatibility]: #backwards-incompatibility

While every effort has been made to avoid backwards incompatibilities in this
proposal, there is one niche circumstance in which this could result in breakage:

```rust
const fn f<T: Clone + ?Sized>() {
    let _ = size_of::<T>();
}

// or..

fn f<T: Clone + ?Sized>() {
    let _ = const { size_of::<T>() };
}
```

In the above example, `f` opts-out of the default `Sized` bound but a `Sized`
bound is implied by its `Clone` bound. As `Sized` supertraits are not
interpreted as `const Sized` in the proposed migration (as bounds are),
with `size_of`'s bound being changed to `const Sized` or `~const Sized`
then this example would no longer compile.

This is a niche case - it relies on code explicitly opting out of a `Sized`
bound, but having a `Sized` implied by another bound, and then using that
parameter somewhere with a `const Sized` requirement. It is hoped that
this is sufficiently rare that it does not block this proposal, and that
any such cases in the open source ecosystem could be identified with a crater
run and addressed by a patch.

It could be easily fixed by removing the unnecessary `?Sized` relaxation
and using the implicit `T: const Sized` bound, or by adding an explicit
`T: const Sized` bound.

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
  require additional relaxed bounds be accepted (i.e. no `?MetaSized`), which has
  had mixed reception in previous RFCs ([rfcs#2255][issue_more_implicit_bounds]
  summarizes these discussions).
- In contrast to [rfcs#1524][rfc_custom_dst], [rfc#1993][rfc_opaque_data_structs],
  [Pre-eRFC: Let's fix DSTs][pre_erfc_fix_dsts], [Pre-RFC: Custom DSTs][prerfc_custom_dst]
  and [eRFC: Minimal Custom DSTs via Extern Type (DynSized)][erfc_minimal_custom_dsts_via_extern_type],
  `MetaSized` does not have `size_of_val`/`align_of_val` methods to support 
  custom DSTs as this would add to the complexity of this proposal and custom DSTs
  are not this RFC's focus, see the [Custom DSTs][custom-dsts] section later.

## Why have `Pointee`?
[why-have-pointee]: #why-have-pointee

`Pointee` exists at the bottom of the trait hierarchy as a consequence of migrating
away from the `?Sized` syntax - enabling the meaning of `?Sized` to be re-defined
to be equivalent to `const MetaSized` and avoid complicated behaviour change over an
edition.

If an alternative is adopted which keeps the `?Sized` syntax then the `Pointee`
trait is not necessary, such as that described in
[*Adding `?MetaSized`*][adding-metasized].

## Why migrate away from `?Sized`?
[why-migrate-away-from-sized]: #why-migrate-away-from-sized

`?Sized` is frequently regarded as confusing for new users and came up in the
[prior art][prior-art] as a reason why new `?Trait` bounds were not seen as desirable
(see [rfcs#2255][issue_more_implicit_bounds]). Furthermore, it isn't clear that `?Sized`
scales well to opting out of default bounds that have constness or hierarchies.

This RFC's proposal to migrate from `?Sized` is based on ideas from an [earlier
pre-eRFC][pre_erfc_fix_dsts] and then a [blog post][blog_dynsized_unsized] which developed
on those ideas, and the feedback to this RFC's prior art, but is not a load-bearing part
of this RFC.

To preserve backwards compatibility, `Sized` bounds must be migrated to `const Sized` (see
[the `size_of` and `size_of_val` section][size-of-and-size-of-val] for rationale), but
`?Sized` bounds could retain their existing behaviour of removing the default `Sized` bound:

### Keeping only `?Sized`
[keeping-only-sized]: #keeping-only-sized

Without adding any additional default bounds or relaxed forms, keeping `?Sized` could be
compatible with this proposal as follows:

In the current edition, `?Sized` would need to be equivalent to `?Sized + const MetaSized` to
maintain backwards compatibility (see [the `size_of` and `size_of_val`
section][size_of-and-size_of_val] for rationale). In the next edition, `?Sized` would be
rewritten to `?Sized + const MetaSized` (remove the `Sized` default and add a `const MetaSized`
bound) and bare `?Sized` would only remove the `Sized` default bound.

Prior to the edition migration, with positive bounds and keeping `?Sized`, the default bound is
`Sized` (interpreted as `const Sized` for backwards compatibility), which could be changed using
the following syntax:

| Canonically       | Syntax with positive bounds          | Syntax keeping `?Sized`              |
| ----------------- | ------------------------------------ | ------------------------------------ |
| `const Sized`     | `T: const Sized`, `T: Sized`, or `T` | `T: const Sized`, `T: Sized`, or `T` |
| `Sized`           | Not possible                         | Not possible                         |
| `const MetaSized` | `T: const MetaSized`                 | `T: ?Sized`                          |
| `MetaSized`       | `T: MetaSized`                       | Not possible                         |
| `Pointee`         | `T: Pointee`                         | Not possible                         |

After the edition migration, with positive bounds and keeping `?Sized`, the default bound is
`const Sized`, which could be changed using the following syntax:

| Canonically        | Syntax with positive bounds | Syntax keeping `?Sized`               |
| ------------------ | --------------------------- | ------------------------------------- |
| `const Sized`      | `T: const Sized` or `T`     | `T: const Sized` or `T`               |
| `Sized`            | `T: Sized`                  | `T: ?(const Sized) + Sized`           |
| `const MetaSized`  | `T: const MetaSized`        | `T: ?(const Sized) + const MetaSized` |
| `MetaSized`        | `T: MetaSized`              | `T: ?(const Sized) + MetaSized`       |
| `Pointee`          | `T: Pointee`                | `T: ?(const Sized)`                   |

In other words, `?(const Sized)` fully opts out of all default bounds and then one has to
explicitly opt back in.

### Adding `?MetaSized`
[adding-metasized]: #adding-metasized

Another alternative is to make `MetaSized` a default bound in addition to `Sized` and establish
that relaxing a supertrait bound also implies relaxing subtrait bounds (but that relaxing a
subtrait bound does not imply relaxing supertrait bounds):

Prior to the edition migration, when adding `?MetaSized`, the default bound is
`MetaSized + const MetaSized + Sized + const Sized`, which could be changed using:

| Canonically       | Syntax with positive bounds         | Syntax adding `?MetaSized`          |
| ----------------- | ----------------------------------- | ----------------------------------- |
| `const Sized`     | `T: const Sized`, `T: Sized` or `T` | `T: const Sized`, `T: Sized` or `T` |
| `Sized`           | Not possible                        | Not possible                        |
| `const MetaSized` | `T: const MetaSized`                | `T: ?(const Sized)` or `T: ?Sized`  |
| `MetaSized`       | `T: MetaSized`                      | `T: ?(const MetaSized)`             |
| `Pointee`         | `T: Pointee`                        | `T: ?MetaSized`                     |

After the edition migration, when adding `?MetaSized`, the default bound remains
`MetaSized + const MetaSized + Sized + const Sized`, which could be changed using:

| Canonically       | Syntax with positive bounds | Syntax adding `?MetaSized` |
| ----------------- | --------------------------- | -------------------------- |
| `const Sized`     | `T: const Sized` or `T`     | `T: const Sized` or `T`    |
| `Sized`           | `T: Sized`                  | `T: ?(const Sized)`        |
| `const MetaSized` | `T: const MetaSized`        | `T: ?Sized`                |
| `MetaSized`       | `T: MetaSized`              | `T: ?(const MetaSized)`    |
| `Pointee`         | `T: Pointee`                | `T: ?MetaSized`            |

In other words, when a less strict bound is desirable, it is achieved by opting out of the
next strictest bound.

### Relaxed bounds and incremental stabilisation
[relaxed-bounds-and-incremental-stabilisation]: #relaxed-bounds-and-incremental-stabilisation

With the above alternatives, the ability to [incrementally stabilise][incremental-stabilisation]
new sizedness traits is made more challenging, as to express a `T: Pointee` bound, a
`T: ?MetaSized` is written, so stability of a degree of sizedness is based not on the stability
of its trait but rather by stability of being able to relax the next strictness trait.

## Why not re-use `std::ptr::Pointee`?
[why-not-re-use-stdptrpointee]: #why-not-re-use-stdptrpointee

`Pointee` is distinct from the existing unstable trait [`std::ptr::Pointee`][api_pointee]
from [rfcs#2580][rfc_pointer_metadata_vtable] as adding a trait with an associated item
to this hierarchy of traits would be backwards incompatible, breaking the example below
as it would be ambigious whether `T::Metadata` refers to `Pointee::Metadata` or
`HasAssocType::Metadata` and it is unambigious today.

```rust
trait HasAssocType {
    type Metadata;
}

fn foo<T: HasAssocType>() -> T::Metadata { // error! ambigious
    todo!()
}
```

This backwards incompatibility also also exists when adding methods to any of the
proposed marker traits. For example, assume methods `x` and `y` were added to `MetaSized`,
then existing calls to methods with the same names would be broken:

```rust
trait HasMethods {
    fn x() {}
    fn y(&self) {}
}

fn foo<T: HasMethods>(t: &T)
    T::x(); // error! ambigious
    t.y(); // error! ambigious
}
```

Due to `Sized` being a default bound, the new marker traits being supertraits, and the
edition migration (so that breakages could happen even with `?Sized`), this backwards
incompatibility would occur immediately when associated types or methods are added.

Instead of introducing a new marker trait, there are three alternatives that would
enable `std::ptr::Pointee` to be re-used:

1. Introduce some mechanism to indicate that associated types or methods of a trait
  could only be referred to with fully-qualified syntax.
2. Introduce a behaviour where the associated types of subtraits take priority over
  the associated types of supertraits. `Pointee` will effectively become a supertrait
  of all traits so its associated types would never take predecence.
  3. Introduce forward-compatibility lints in current edition, the new traits were
  introduced in the next edition and the edition migration previously described
  in the next next edition.

## Why use const traits?
[why-use-const-traits]: #why-use-const-traits

While this RFC is written assuming [const traits (RFC #3762)][rfc_const_traits], it
is not strictly necessary.

Prior to recognising the interactions with constness, previous iterations of this RFC
had both linear[^9] and non-linear[^10] trait hierarchies which included a `RuntimeSized`
supertrait of `Sized`.

However, both of these were found to be backwards-incompatible due to being unable
to relax the supertrait of `Clone`. In the example below, `Foo::uses_supertrait_bound`'s
call to `requires_sized` relies on `Clone` having a `Sized` supertrait and would no
longer compile if `Clone`'s supertrait was relaxed:

```rust
trait Foo: Clone {
    fn uses_supertrait_bound() { requires_sized::<Self>() }
}

fn requires_sized<T: Sized>() { /* ... */ }
```

Once this interaction with constness was realised, this RFC's dependency on const
traits was introduced. Due to const traits (at the time of writing this RFC) being
only a proposal, other alternatives that would avoid const traits but leverage
insights from this RFC's use of const traits have been considered - adding a
subtrait of `Sized` to emulate `const Sized` without const traits proper.

Introduce a subtrait of `Sized`, `ConstSized`, and of `MetaSized`,
`ConstMetaSized`. In order to avoid this breaking a breaking change, as with
introducing arbitrary new automatically-implemented traits, the default `Sized`
bound would be migrated to a `ConstSized` bound over an edition (with `T` and
`T: Sized` being interpreted as `ConstSized` in the current edition). This
hierarchy mirrors that which is created when using const traits.

```rust
trait ConstSized: Sized + ConstMetaSized {}

trait Sized: MetaSized {}

trait ConstMetaSized: MetaSized {}

trait MetaSized: Pointee {}

trait Pointee {}
```

Under this variant, `Clone`'s supertrait does not need to be relaxed and the
previous example would continue to compile. Runtime-sized types would implement
`Sized` but not `ConstSized`, and `MetaSized` but not `ConstMetaSized`. All
types which currently implement `Sized` would implement `ConstSized` and
all types which currently satisfy a `?Sized` bound would implement
`ConstMetaSized`.

As `ConstSized` is a marker trait, it does not need to have any methods usable
in const contexts, so full-fledged const traits aren't strictly necessary, it
is sufficient that it can be used in bounds to prevent runtime-sized types
from being instantiated, such as with `size_of`:

```rust
pub const fn size_of<T: ConstSized>() -> usize {
    /* .. */
}
```

Likewise, `size_of_val`'s bound can be relaxed to `ConstMetaSized`

```rust
pub const fn size_of_val<T: ConstMetaSized>(v: &T) -> usize {
    /* .. */
}
```

In both functions, runtime-sized types are prevented from being valid generic
arguments while all other appropriate types are accepted.

Without const traits' const-conditional bounds (`~const`), it isn't possible
to write a version of `size_of` or `size_of_val` which can be used with
runtime-sized types at runtime but not in a const context.

As this approach replicates the hierarchy which would be introduced by const
traits and it is the view of this RFC's author that some form of const traits
are likely to be adopted: this non-const trait variant is not the primary
proposal of this RFC and instead is included only to demonstrate that such a
dependency need not be considered a blocker.

[^9]: In previous iterations, the proposed linear trait hierarchy was:

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
      represented. `ValueSized` was replaced with `MetaSized` in later versions
      of the proposal.
[^10]: In previous iterations, the proposed non-linear trait hierarchy was:

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
      infeasible anyway. `ValueSized` was replaced with `MetaSized` in later
      versions of the proposal.

## Why change `size_of` and `size_of_val`?
[why-change-size_of-and-size_of_val]: #why-change-size_of-and-size_of_val

It is theoretically possible for `size_of` and `size_of_val` to accept
runtime-sized types in a const context and use the runtime environment of the host
when computing the size of the types. This has some advantages, if implementable
within the compiler's interpreter, it could enable accelerated execution of
const code. However, there are multiple downsides to allowing this:

Runtime-sized types are often platform specific and could require optional
target features, which would necessitate use of `cfg`s and `target_feature`
with const functions, adding a lot of complexity to const code.

More importantly, the size of a runtime-sized type could differ between
the host and the target and if the size from a const context were to be used
at runtime with runtime-sized types, then that could result in incorrect
code. Not only is it unintuitive that `const { size_of::<svint8_t>() }` would
not be equal to `size_of::<svint8_t>()`, but the layout of types could differ
between const and runtime contexts which would be unsound.

Changing `size_of` and `size_of_val` to `~const Sized` bounds ensures that
`const { size_of:<svint8_t>() }` is not possible.

## Why `MetaSized` instead of `ValueSized`?
[why-metasized-instead-of-valuesized]: #why-metasized-instead-of-valuesized

`MetaSized` is defined as inspecting pointer metadata to compute the size,
which is how the size of all existing non-`Sized` types is determined. An
alternative to `MetaSized` is `ValueSized`, which would have a more general
definition of requiring a reference to a value to compute its size.

`ValueSized` has a broader definition than `MetaSized` which does not match
the current behaviour of `?Sized` exactly. `ValueSized` has a downside that
its interaction with mutexes introduces the opportunity for deadlocks which
are unintuitive:

Consider a version of the `CStr` type which is a dynamically sized and
computes its size by counting the characters before the null byte (this
is different from the existing `std::ffi::CStr` which is `MetaSized`).
`CStr` would implement `ValueSized`. If this type were used in a `Mutex<T>`
then the mutex would also implement `ValueSized` and require locking itself
to compute the size of the `CStr` that it guards, which could result in
unexpected deadlocks:

```rust
let mutex = Mutex::new(CStr::from_str("foo"));
let _guard = mutex.lock().unwrap();
size_of_val(&mutex); // deadlock!
```

`MetaSized` avoids this hazard by keeping the size of dynamically sized
types in pointer metadata, which can be accessed without locking a mutex.

`ValueSized` could be introduced as a complement to `MetaSized`, if there are
types whose size cannot be stored in pointer metadata (or where this is not
desirable):

```
           ┌────────────────┐                                 ┌─────────────────────────────┐
           │ const Sized    │ ──────────────────────────────→ │ Sized                       │
           │ {type, target} │             implies             │ {type, target, runtime env} │
           └────────────────┘                                 └─────────────────────────────┘
                   │                                                         │
                implies                                                   implies
                   │                                                         │
                   ↓                                                         ↓
    ┌──────────────────────────────┐                   ┌───────────────────────────────────────────┐
    │ const MetaSized              │ ────────────────→ │ MetaSized                                 │
    │ {type, target, ptr metadata} │      implies      │ {type, target, ptr metadata, runtime env} │
    └──────────────────────────────┘                   └───────────────────────────────────────────┘
                   │                                                         │
                implies                                                   implies
                   │                                                         │
                   ↓                                                         ↓
┌─────────────────────────────────────┐             ┌──────────────────────────────────────────────────┐
│ const ValueSized                    │ ──────────→ │ ValueSized                                       │
│ {type, target, ptr metadata, value} │   implies   │ {type, target, ptr metadata, value, runtime env} │
└─────────────────────────────────────┘             └──────────────────────────────────────────────────┘
                                                                             │
                                                                          implies
                                                                             │
                                             ┌───────────────────────────────┘
                                             ↓
                                   ┌──────────────────┐
                                   │ Pointee          │
                                   │ {runtime env, *} │
                                   └──────────────────┘
```

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
this RFC's `MetaSized` trait is inspired.

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
        - `?Trait` being a negative feature can be confusing to users.
        - Downstream crates need to re-evaluate every API to determine if adding `?Trait`
          makes sense, for each `?Trait` added.
          - This is also true of the traits added in this proposal, regardless of whether a
            relaxed bound or positive bound syntax is used. However, this proposal argues
            that adding supertraits of an existing default bound significantly lessens this
            disadvantage (and moreso given the niche use cases of these particular
            supertraits).
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
    - The proposed `DynSized` trait in [rfcs#2310][rfc_dynsized_without_dynsized]
      is really quite similar to the `MetaSized` trait proposed by this RFC except:
        - It includes an `#[assume_dyn_sized]` attribute to be added to
          `T: ?Sized` bounds instead of replacing them with `T: const MetaSized`,
          which would warn instead of error when a non-`const MetaSized` type is
          substituted into `T`.
            - This is to avoid a backwards compatibility break for uses of
              `size_of_val` and `align_of_val` with extern types, but it is
              unclear why this is necessary given that extern types are
              unstable.
        - It does not include `Pointee` or any of the const traits.
        - Adding an explicit bound for `MetaSized` would not remove the implicit
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
        - Basically identical to this RFC's `MetaSized`.
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
      - `DynSized` is very similar to this RFC's `MetaSized`
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
    - These can be sidestepped by by relying on being a supertrait of `Sized`.

The [Rationale and Alternatives](#rationale-and-alternatives) section provides
rationale for some of the decisions made in this RFC and references the prior
art above when those proposals made different decisions.

No previous proposal captures the specific part of the design space that this
proposal attempts to, but these proposals are the closest matches for parts of
this proposal:

- [Pre-eRFC: Let's fix DSTs][pre_erfc_fix_dsts] was the only other proposal
  removing `Sized` bounds when a bound for another sized trait (only `DynSized`
  in that pre-eRFC's case) was present.
    - However, this proposal had `size_of_val` methods in its `DynSized` trait and
      proposed a bunch of other things necessary for Custom DSTs.
- [rfcs#2310: DynSized without ?DynSized][rfc_dynsized_without_dynsized] was
  proposed at a similar time and was similarly focused only on making `Sized` more
  flexible, but had a bunch of machinery for avoiding backwards incompatibility
  that this RFC believes is unnecessary. Like this proposal, it avoided making
  `DynSized` a default bound and avoided having a relaxed form of it.
    - However, this proposal didn't suggest removing default `Sized` bounds in
      the presence of other size trait bounds.
- [rfcs#3396: Extern types v2][rfc_extern_types_v2] identified that `MetaSized` 
  specifically was necessary moreso than `DynSized` or `ValueSized` and serves
  as the inspiration for this RFC's `MetaSized`.
- [Sized, DynSized, and Unsized][blog_dynsized_unsized] is very similar and a
  major inspiration for this proposal. It has everything this proposal has except
  for the const size traits and all the additional context an RFC needs.

Some prior art referenced [rust#21974][issue_regions_too_simplistic] as a limitation
of the type system which can result in new implicit bounds or implicit supertraits
being infeasible for implementation reasons, but [it is believed that this is no
longer relevant][zulip_issue_regions_too_simplistic].

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- What names should be used for the traits?
- Which syntax should be used for opting out of a default bound with const traits and
  a trait hierarchy?
  - This RFC is primarily written proposing the "positive bounds" approach, where
    introducing a positive bound for a supertrait of the default bound will remove
    the default bound.
  - Alternatively, described in [*Adding `?MetaSized`*][adding-metasized], existing
    relaxed bounds syntax could be used, where a desired bound is written as opting out
    of the next strictest.
  - In a February 2025 design meeting with the language team, a **strong bias towards
    the positive bounds alternative was expressed**, arguing that while a explicit sigil
    indicating an opt-out is happening is valuable, both alternatives have unintuitive
    aspects, but that "asking for what you want" (as in the positive bounds alternative)
    is less confusing than "asking for the next strictest thing you don't need" (as in the
    relaxed bounds alternative).
- Should `std::ptr::Pointee` be re-used instead of introducing a new marker trait?
  - In a February 2025 design meeting with the language team, **no strong opinion was
    expressed on this question**. There are open proposals to change
    `std::ptr::Pointee` to no longer have an associated type, which would render this
    unresolved question moot. A mild preference for the second alternative described in
    [*Why not re-use `str::ptr::Pointee`?*][why-not-re-use-stdptrpointee] was also
    shared.
- What is the precedence for `?const Trait` - `(?const) Trait` or `?(const Trait)`?
  - This isn't a question for this RFC to resolve but this RFC takes a conserative
    approach and always adds explicit parentheses.

# Future possibilities
[future-possibilities]: #future-possibilities

- Additional size traits could be added as supertraits of `Sized` if there are
  other delineations in sized-ness that make sense to be drawn (subject to
  avoiding backwards-incompatibilities when changing APIs).
- The requirement that users cannot implement any of these traits could be
  relaxed in future if required.
- Depending on a trait which has one of the proposed traits as a supertrait could
  imply a bound of the proposed trait, enabling the removal of boilerplate.
  - However, this would limit the ability to relax a supertrait, e.g. if
    `trait Clone: Sized` and `T: Clone` is used as a bound of a function
    and `Sized` is relied on in that function, then the supertrait of
    `Clone` could no longer be relaxed as it can today. 
- All existing associated types will have at least a `const MetaSized` bound
  and relaxing these bounds is a semver-breaking change. It could be worth considering
  introducing mechanisms to make this relaxation non-breaking and apply that
  automatically over an edition.
  - i.e. `type Output: if_rust_2021(Sized) + NewAutoTrait` or something like that,
    out of scope for this RFC.
- Consider allowing traits to relax their bounds and having their implementor have
  stricter bounds - this would enable traits and implementations to migrate towards
  more relaxed bounds.
  - This would be unintuitive to callers but would not break existing code.

## externref
[externref]: #externref

Another compelling feature that requires extensions to Rust's sizedness traits to
fully support is wasm's `externref`. `externref` types are opaque types that cannot
be put in memory [^11]. `externref`s are used as abstract handles to resources in the
host environment of the wasm program, such as a JavaScript object. Similarly, when
targetting some GPU IRs (such as SPIR-V), there are types which are opaque handles
to resources (such as textures) and these types, like wasm's `externref`, cannot
be put in memory.

[^11]: When Rust is compiled to wasm, we can think of the memory of the Rust program
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
       ┌────────────────┐                          ┌─────────────────────────────┐
       │ const Sized    │ ───────────────────────→ │ Sized                       │
       │ {type, target} │         implies          │ {type, target, runtime env} │
       └────────────────┘                          └─────────────────────────────┘
               │                                                  │
            implies                                            implies
               │                                                  │
               ↓                                                  ↓
┌──────────────────────────────┐             ┌───────────────────────────────────────────┐
│ const MetaSized              │ ──────────→ │ MetaSized                                 │
│ {type, target, ptr metadata} │   implies   │ {type, target, ptr metadata, runtime env} │
└──────────────────────────────┘             └───────────────────────────────────────────┘
                                                                  │
                                                               implies
                                                                  │
                                      ┌───────────────────────────┘
                                      ↓
                            ┌──────────────────┐
                            │ Pointee          │
                            │ {runtime env, *} │
                            └──────────────────┘
                                      │
                                   implies
                                      │
                                      ↓
                            ┌──────────────────┐
                            │ Value            │
                            │ {runtime env, *} │
                            └──────────────────┘
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
pointer or a reference. This also implies that `MetaSized` types can be used as values,
which would remain prohibited behind the `unsized_locals` and `unsized_fn_params`
features until these are stabilised.

With these changes to the RFC and possibility additional changes to the language, it
could be possible to support wasm's `externref` and opaque types from some GPU targets.

## Alignment
[alignment]: #alignment

There has been community interest in an [`Aligned` trait][rfc_aligned] and there
are examples of `Aligned` traits being added in the ecosystem:

- `rustc` has [its own `Aligned` trait][rustc_aligned] to support pointer tagging.
- [`unsized-vec`][crate_unsized_vec] implements a `Vec` that depends on knowing
  whether a type has an alignment or not.

An `Aligned` trait hierarchy could be introduced alongside this proposal. It wouldn't
viable to introduce `Aligned` within this hierarchy, as `dyn Trait` which is `MetaSized`
would not be aligned, but some extern types could be `Aligned`, so there isn't an obvious
place that an `Aligned` trait could be included in this hierarchy.

## Custom DSTs
[custom-dsts]: #custom-dsts

Given the community interest in supporting custom DSTs in future (see
[prior art][prior-art]), this RFC was written considering future-compatibility with
custom DSTs in mind.

There are various future changes to these traits which could be used to support
custom DSTs on top of this RFC. None of these have been considered thoroughly, and are
written here only to illustrate.

- Allow `std::ptr::Pointee` to be implemented manually on user types, which would
  replace the compiler's implementation.
- Introduce a trait like [rfcs#2594][rfc_custom_dst_electric_boogaloo]'s `Contiguous`
  which users can implement on their custom DSTs, or add methods to `MetaSized` and
  allow it to be implemented by users.
- Introduce intrinsics which enable creation of pointers with metadata and for
  accessing the metadata of a pointer.

[ack_compiler_errors]: https://github.com/compiler-errors
[ack_eddyb]: https://github.com/eddyb
[ack_fee1dead]: https://github.com/fee1-dead
[ack_jacobbramley]: https://github.com/jacobbramley
[ack_scottmcm]: https://github.com/scottmcm
[api_align_of]: https://doc.rust-lang.org/std/mem/fn.align_of.html
[api_align_of_val]: https://doc.rust-lang.org/std/mem/fn.align_of_val.html
[api_box]: https://doc.rust-lang.org/std/boxed/struct.Box.html
[api_clone]: https://doc.rust-lang.org/std/clone/trait.Clone.html
[api_copy]: https://doc.rust-lang.org/std/marker/trait.Copy.html
[api_phantomdata]: https://doc.rust-lang.org/std/marker/struct.PhantomData.html
[api_pointee]: https://doc.rust-lang.org/std/ptr/trait.Pointee.html
[api_size_of]: https://doc.rust-lang.org/std/mem/fn.size_of.html
[api_size_of_val]: https://doc.rust-lang.org/std/mem/fn.size_of_val.html
[api_sized]: https://doc.rust-lang.org/std/marker/trait.Sized.html
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
[rfc_const_traits]: https://github.com/rust-lang/rfcs/pull/3762
[rfc_custom_dst]: https://github.com/rust-lang/rfcs/pull/1524
[rfc_custom_dst_electric_boogaloo]: https://github.com/rust-lang/rfcs/pull/2594
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
[rfl_want]: https://github.com/Rust-for-Linux/linux/issues/354
[rustc_aligned]: https://github.com/rust-lang/rust/blob/a76ec181fba25f9fe64999ec2ae84bdc393560f2/compiler/rustc_data_structures/src/aligned.rs#L22-L25
[rustc_opaquelistcontents]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/ty/list/foreigntype.OpaqueListContents.html
[zulip_issue_regions_too_simplistic]: https://rust-lang.zulipchat.com/#narrow/channel/144729-t-types/topic/.2321984.20.2B.20implicit.20supertraits.20-.20still.20relevant.3F/near/477630998
