# RFC: result_ffi_guarantees

- Feature Name: `result_ffi_guarantees`
- Start Date: 2023-02-15
- RFC PR: [rust-lang/rfcs#3391](https://github.com/rust-lang/rfcs/pull/3391)
- Rust Issue: [rust-lang/rust#110503](https://github.com/rust-lang/rust/issues/110503)

# Summary
[summary]: #summary

This RFC gives specific layout and ABI guarantees when wrapping "non-zero" data types from `core` in `Option` or `Result`. This allows those data types to be used directly in FFI, in place of the primitive form of the data (eg: `Result<(), NonZeroI32>` instead of `i32`).

# Motivation
[motivation]: #motivation

Rust often needs to interact with foreign code. However, foreign function type signatures don't normally support any of Rust's rich type system. Particular function inputs and outputs will simply use 0 (or null) as a sentinel value and the programmer has to remember when that's happening.

Though it's common for "raw bindings" crates to also have "high level wrapper" crates that go with them (eg: `windows-sys`/`windows`, or `sdl2-sys`/`sdl2`, etc), someone still has to write those wrapper crates which use the foreign functions directly. Allowing Rust programmers to use more detailed types with foreign functions makes their work easier.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

I'm not sure how to write a "guide" portion of this that's any simpler than the "reference" portion, which is already quite short.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

When either of these two `core` types:

* `Option<T>`
* `Result<T, E>` where either `T` or `E` meet all of the following conditions:
  * Is a zero-sized type with alignment 1 (a "1-ZST").
  * Has no fields.
  * Does not have the `#[non_exhaustive]` attribute.

Is combined with a non-zero or non-null type (see the chart), the combination has the same layout (size and alignment) and the same ABI as the primitive form of the data.

| Example combined Type | Primitive Type |
|:-|:-|
| `Result<NonNull<T>, ()>` | `*mut T` |
| `Result<&T, ()>` | `&T` |
| `Result<&mut T, ()>` | `&mut T` |
| `Result<fn(), ()>` | `fn()` |
| `Result<NonZeroI8, ()>` | `i8` |
| `Result<NonZeroI16, ()>` | `i16` |
| `Result<NonZeroI32, ()>` | `i32` |
| `Result<NonZeroI64, ()>` | `i64` |
| `Result<NonZeroI128, ()>` | `i128` |
| `Result<NonZeroIsize, ()>` | `isize` |
| `Result<NonZeroU8, ()>` | `u8` |
| `Result<NonZeroU16, ()>` | `u16` |
| `Result<NonZeroU32, ()>` | `u32` |
| `Result<NonZeroU64, ()>` | `u64` |
| `Result<NonZeroU128, ()>` | `u128` |
| `Result<NonZeroUsize, ()>` | `usize` |

* While `fn()` is listed just once in the above table, this rule applies to all `fn` types (regardless of ABI, arguments, and return type).

For simplicity the table listing only uses `Result<_, ()>`, but swapping the `T` and `E` types, or using `Option<T>`, is also valid.
What changes are the implied semantics:
* `Result<NonZeroI32, ()>` is "a non-zero success value"
* `Result<(), NonZeroI32>` is "a non-zero error value"
* `Option<NonZeroI32>` is "a non-zero value is present"
* they all pass over FFI as if they were an `i32`.

Which type you should use with a particular FFI function signature still depends on the function.
Rust can't solve that part for you.
However, once you've decided on the type you want to use, the compiler's normal type checks can guide you everywhere else in the code.

# Drawbacks
[drawbacks]: #drawbacks

* The compiler has less flexibility with respect to discriminant computation and pattern matching optimizations when a type is niche-optimized.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

It's always possible to *not* strengthen the guarantees of the language.

# Prior art
[prior-art]: #prior-art

The compiler already supports `Option` being combined with specific non-zero types, this RFC mostly expands the list of guaranteed support.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None at this time.

# Future possibilities
[future-possibilities]: #future-possibilities

* This could be expanded to include [ControlFlow](https://doc.rust-lang.org/nightly/core/ops/enum.ControlFlow.html) and [Poll](https://doc.rust-lang.org/nightly/core/task/enum.Poll.html).
* This could be extended to *all* similar enums in the future. However, without a way to opt-in to the special layout and ABI guarantees (eg: a trait or attribute) it becomes yet another semver hazard for library authors. The RFC is deliberately limited in scope to avoid bikesheding.
