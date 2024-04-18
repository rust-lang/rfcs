- Feature Name: `constants_in_patterns`
- Start Date: 2023-11-19
- RFC PR: [rust-lang/rfcs#3535](https://github.com/rust-lang/rfcs/pull/3535)
- Tracking Issue: [rust-lang/rust#120362](https://github.com/rust-lang/rust/issues/120362)

# Summary
[summary]: #summary

When a constant appears as a pattern, this is syntactic sugar for writing a pattern that corresponds to the constant's value by hand.
This operation is only allowed when (a) the type of the constant implements `PartialEq`, and (b) the *value* of the constant being matched on has "structural equality", which means that `PartialEq` behaves the same way as that desugared pattern.

This RFC does not allow any new code, compared to what already builds on stable today.
Its purpose is to explain the rules for constants in patterns in one coherent document,
and to justify why we will start *rejecting* some code that currently works (see the [breaking changes](#breaking-changes) below).

# Motivation
[motivation]: #motivation

The main motivation to write this RFC is to finish what started in [RFC 1445][rfc-1445]: define what happens when a constant is used as a pattern.
That RFC is incomplete in several ways:

- It was never fully implemented; due to bugs in the early implementation, parts of it are still behind future-compatibility lints.
  This also leads to a rather messy situation in rustc where const-in-pattern handling has to deal with fallback cases and emitting four (!) such lints.
  We should clean up both our language specification and the implementation in rustc.
- The RFC said that matching on floats should be fully rejected, but when a PR was made to enforce this, [many people spoke up against that and it got rejected](https://github.com/rust-lang/rust/pull/84045).
- The RFC does not explain how to treat raw pointers and function pointers.

[rfc-1445]: https://rust-lang.github.io/rfcs/1445-restrict-constants-in-patterns.html

RFC 1445 had the goal of leaving it open whether we want constants in patterns to be treated like sugar for primitive patterns or for `PartialEq`-based equality tests.
This new RFC takes the stance it does the former based on the following main design goals:

- Refactoring a pattern that has no binders, wildcards, or ranges into a constant should never change behavior.
  This aligns with the oft-repeated intuition that a constant works "as if" its value was just copy-pasted everywhere the constant is used.
  This is particularly important for patterns that syntactically look exactly like constants, namely zero-field enum variants. Consider:

    ```rust
    enum E { Var1, Var2 }

    const BEST_VAR: E = E::Var1;

    fn is_best(e: E) -> bool { matches!(e, BEST_VAR) }
    fn is_var1(e: E) -> bool { matches!(e, E::Var1) }
    ```

    It would be very surprising if those two functions could behave differently.
  It follows that we cannot allow `PartialEq` to affect the behavior of constants in patterns.

- We do not want to expose equality tests on types where the library author did not explicitly expose an equality test.
  This means not allowing matching on constants whose type does not implement `PartialEq`.
  It also means not allowing matching on constants where running `PartialEq` would behave different from the corresponding pattern:
  the pattern can be accessing private fields, and the `PartialEq` implementation provided by the library might be treating those fields in a particular way;
  we should not let people write patterns that "bypass" any such treatment and do structural matching when the crate author does not provide a structural `PartialEq`.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Constants can be used as patterns, but only if their type implements `PartialEq`.
Moreover, this implementation must be the automatically derived one, and that also applies recursively for the types of their fields:

```rust
#[derive(PartialEq)] // code fails to build if we remove this or replace it by a manual impl
enum E { Var1, Var2 }

const BEST_VAR: E = E::VAR1;

fn is_best(e: E) -> bool { matches!(e, BEST_VAR) }
```

```rust
#[derive(PartialEq)]
struct S { f1: i32, f2: E }

const MY_S: S = S { f1: 42, f2: BEST_VAR };

// Removing *either* `derive` or implementing `PartialEq` manually would lead to rejecting this code
fn is_mine(s: S) -> bool { matches!(s, MY_S) }
```

We say that a type that derives `PartialEq` has "structural equality": the
equality of this type is defined fully by the equality on its fields.

For matching on values of enum type, it is sufficient if the actually chosen
variant has structural equality; other variants do not matter:

```rust
struct NonStructuralEq(i32);

impl PartialEq for NonStructuralEq {
    fn eq(&self) -> bool { true }
}

#[derive(PartialEq)]
enum MyEnum { GoodVariant(i32), NonStructuralVariant(NonStructuralEq) }

// This constant *can* be used in a pattern.
const C: MyEnum = MyEnum::GoodVariant(0);
```

This means the eligibility of a constant for a pattern depends on its value, not just on its type.
That is already the case on stable Rust for many years and relied upon by widely-used crates such as [`http`](https://github.com/rust-lang/rust/issues/62411#issuecomment-510604193).

Overall we say that the *value* of the constant must have recursive structural equality,
which is the case when all the types that actually appear recursively in the value (ignoring "other" enum variants) have structural equality.

Most of the values of primitive Rust types have structural equality (integers, `bool`, `char`, references), but two families of types need special consideration:

- Pointer types (raw pointers and function pointers): these compare by testing the memory address for equality.
  It is unclear whether that should be considered "structural", but it is fairly clear that this should be considered a bad idea:
  Rust makes basically no guarantees for when two function pointers are equal or unequal
  (the "same" function can be duplicated across codegen units and this have different addresses,
  and different functions can be merged when they compile to the same assembly and thus have the same address).
  Similarly, there are no or few guarantees for equality of pointers that are generated in constants.
  However, there *is* a very clear notion of equality on pointers like `4 as *const i32`, and such pointers are occasionally used as sentinel value and used in `match`.
  This is common enough to occur [even in the standard library](https://github.com/rust-lang/rust/blob/b18db7a13e52f71e94bdf221a7a013fd9ace4c7f/library/std/src/sys/windows/thread_parking.rs#L225-L226).
  Thus we declare that values of raw pointer type have structural equality only if they are such pointers created by casting an integer.
  Values of function pointer type never have structural equality;
  it is very unusual to "cast" (really: transmute) an integer into a function pointer, and there is currently not sufficient motivation for allowing this in a pattern.
- Floating-point types: in `f32` and `f64`, NaNs are not equal to themselves.
  Furthermore, `+0.0` (written just `0.0` in Rust) and `-0.0` have different bit representations, but they *do* compare equal.
  We can easily declare NaNs to not have structural equality, and reject them in patterns, since there is no situation where it makes sense to have a NaN in a pattern.
  However, for zeroes this is more tricky -- allowing matching on `1.0` but rejecting `0.0` is likely going to be considered extremely arbitrary and inconsistent.
  This RFC therefore suggests that all zeroes should have structural equality, and match all zeroes.
  `0.0` and `-0.0` as a value of a constant used in a pattern will all match both `0.0` and `-0.0`, as they do today.
  (As a literal, `0.0` and `-0.0` are currently both accepted but `+0.0` is not. This RFC does not propose to change anything here, though we could consider linting against `-0.0`.)

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

When a constant `C` of type `T` is used as a pattern, we first check that `T: PartialEq`.
Furthermore we require that the value of `C` *has (recursive) structural equality*, which is defined recursively as follows:

- Integers as well as `bool` and `char` values always have structural equality.
- Tuples, arrays, and slices have structural equality if all their fields/elements have structural equality.
  (In particular, `()` and `[]` always have structural equality.)
- References have structural equality if the value they point to has structural equality.
- A value of `struct` or `enum` type has structural equality if its `PartialEq` behaves exactly like the one generated by `derive(PartialEq)`,
  and all fields (for enums: of the active variant) have structural equality.
- A raw pointer has structural equality if it was defined as a constant integer (and then cast/transmuted).
- A float value has structural equality if it is not a `NaN`.
- Nothing else has structural equality.

In particular, the value of `C` must be known at pattern-building time (which is pre-monomorphization).

After ensuring all conditions are met, the constant value is translated into a pattern, and now behaves exactly as-if that pattern had been written directly.
In particular, it fully participates in exhaustiveness checking.
(For raw pointers, constants are the only way to write such patterns. Only `_` is ever considered exhaustive for these types.)

Practically speaking, to determine whether a `struct`/`enum` type's `PartialEq` behaves exactly like the one generated by `derive(PartialEq)`, we use a trait:

```rust
/// When implemented on a `struct` or `enum`, this trait indicates that
/// the `PartialEq` impl of `Self` behaves exactly like the one
/// generated by `derive(PartialEq)`:
/// - on a `struct`, it returns `true` if and only if comparing all
///   fields with their respective `PartialEq` returns `true`.
/// - in an `enum`, it returns `true` if and only if both values
///   have the same variant, and furthermore comparing the fields of
///   that variant with their respective `PartialEq` returns `true`
///   for all fields.
///
/// This trait should not be implemented on `union` types.
///
/// This is a "shallow" property in the sense that it says nothing about
/// the behavior of `PartialEq` on the fields of this type, it only relates
/// `PartialEq` on this type to that of its fields.
///
/// This trait is used when determining whether a constant may be used as a pattern:
/// all types appearing in the value of the constant must implement this trait.
///
/// All that said, this is a safe trait, so violating these requirements
/// can only lead to logic bugs or accidentally exposing an equality test that your
/// library would otherwise not provide, not to unsoundness.
trait StructuralPartialEq: PartialEq {}
```

This trait is automatically implemented when writing `derive(PartialEq)`.
For this RFC to be implemented, the trait can remain unstable can hence cannot be implemented directly by users.
In the future, it might be possible for libraries that implement `PartialEq` by hand (for instance for performance reasons) to also implement `StructuralPartialEq` by hand, if they can promise that the comparison behaves as documented.
(See "Future possibilities" for some of the open questions around that option.)
The trait has `PartialEq` as a supertrait because its entire contract only makes sense for types that implement `PartialEq`.

Range patterns are only allowed on integers, `char`, and floats; for floats, neither end must be a `NaN`.

The *behavior* of such a constant as a pattern is the same as the corresponding native pattern.
On floats are raw pointers, pattern matching behaves like `==`,
which means in particular that the value `-0.0` matches the pattern `0.0`, and NaN values match no pattern (except for wildcards).

## Breaking changes

This RFC breaks code that compiles today, but only code that already emits a future compatibility lint:
- Matching on constants that do not implement `PartialEq` sometimes accidentally works, but triggers `const_patterns_without_partial_eq`.
  This lint landed with Rust 1.74 (the most recent stable release), and is shown in dependencies as well.
- Matching on `struct`/`enum` that do not `derive(PartialEq)` is accidentally possible under some conditions, but triggers `indirect_structural_match`.
  This has been a future-compatibility lint for many years, though it is currently not shown in dependencies.
- Matching on function pointers, or raw pointers that are not defined as a constant integer, triggers `pointer_structural_match`.
  This only recently landed (Rust 1.75, currently in beta), and is not currently shown in dependencies.
  Crater found [three cases](https://github.com/rust-lang/rust/pull/116930#issuecomment-1784648989) across the ecosystem where `match` was used to compare function pointers;
  that code is buggy for the reasons mentioned above that make comparing function pointers unreliable.
- Matching on floats triggers `illegal_floating_point_literal_pattern`. This triggers on *all* float matches, not just the ones forbidden by this RFC.
  It has been around for years, but is not currently shown in dependencies.

When the RFC gets accepted, the floating-point lint should be adjusted to only cover the cases we are really going to reject,
and all of them should be shown in dependencies or directly turned into hard errors.

## Compiler/library cleanup

There also exists the `nontrivial_structural_match` future compatibility lint;
it is not needed for this RFC so it can be removed when the RFC gets accepted.

Similarly, the `StructuralEq` trait no longer serves a purpose and can be removed.

# Drawbacks
[drawbacks]: #drawbacks

- The biggest drawback of this proposal is that it conflates `derive(PartialEq)` with a semver-stable promise that this type will always have structural equality.
  Once a type has `derive(PartialEq)`, it may appear in patterns, so replacing this `PartialEq` by a custom implementation is a breaking change.
  Once the `StructuralPartialEq` trait is stable, `derive(PartialEq)` *can* be replaced by a custom implementation as long as one also implements `StructuralPartialEq`,
  but that entails a promise that the `impl` still behaves structurally, including on all private fields.
  This still prevents adding fields that are supposed to be completely ignored by `PartialEq`.

    Fixing that drawback requires a completely new language feature: user-controlled behavior of patterns.
  This is certainly interesting, but requires a lot of design work.
  This RFC does no preclude us from doing that in the future, but proposes that we clean up our const-in-pattern story *now* without waiting for such a design to happen.

- Another drawback is that we require the constant value to be known at pattern building time, which is pre-monomorphization.
  To allow matching on "opaque" constants, we would have to add new machinery, such as a trait that indicates that *all* values of a given type have recursive structural equality.
  (Remember that `StructuralPartialEq` only reflects "shallow" structural equality.)

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The main design rationale has been explained in the "Motivation" section.

Some possible alternatives include:

- **Strictly implement RFC 1445.**
  The RFC is unclear on whether *all* fields that occur in a type must recursively have structural equality or whether that only applies to those fields we actually encounter in the constant value;
  however, the phrasing "When converting a constant value into a pattern" indicates value-based thinking.
  The RFC is also silent on raw pointers and function pointers.
  That means the only difference between this RFC and RFC 1445 is the treatment of floats.
  This is done to account for [this PR](https://github.com/rust-lang/rust/pull/84045) where the lang team decided *not* to reject floats in patterns, since they are used too widely and there's nothing obviously wrong with most ways of using them---the exception being `NaN` and zeroes, and this RFC excludes `NaN`.
  That makes accepting float zeroes as a pattern the only true divergence of this RFC from RFC 1445.
  That is done because the majority of programmers are not aware of the fact that `0.0` and `-0.0` are different values with different bit representations that compare equal, and would likely be stumped by Rust accepting `1.0` as a pattern but rejecting `0.0`.
- **Reject pointers completely.**
  This was considered, but matching against sentinel values of raw pointers is a pretty common pattern, so we should have a really good reason to break that code---and we do not.
- **Involve `Eq`.**
  This RFC is completely defined in terms of `PartialEq`; the `Eq` trait plays no role.
  This is primarily because we allow floating-point values in patterns, which means that we cannot require the constant to implement `Eq` in the first place.
- **Do not require `PartialEq`.**
  Currently we check both that the constant value has recursive structural equality, and that its type implements `PartialEq`.
  Therefore, matching against `const NONE: Option<NotPartialEq> = None;` is rejected.
  This is to ensure that matching only does things that could already be done with `==`,
  so that library authors do not have to take into account matching when reasoning about semver stability.
- **Fallback to `==`.**
  When the constant fails the structural equality test, instead of rejecting the code outright, we could accept it and compare with `==` instead.
  This might be surprising since the user did not ask for `==` to be invoked.
  This also makes it harder to later add support for libraries controlling matching themselves, since one can already match on everything that has `PartialEq`,
  but that latter point could be mitigated by only allowing such matching when a marker trait is implemented.
  There currently does not seem to be sufficient motivation for doing this, and the RFC as proposed is forward-compatible with doing this in the future should the need come up.
- Do something that violates the core design principles laid out in the "Motivation" section.
  This is considered a no-go by this RFC:
  having possible behavior changes upon "outlining" a fieldless enum variant into a constant is too big of a footgun,
  and allowing matching without opt-in from the crate that defines the type makes abstraction and semver too tricky to maintain (consider that private fields would be compared when matching).

# Prior art
[prior-art]: #prior-art

[RFC 1445][rfc-1445] defines basically the same checks as this RFC; this RFC merely spells them out more clearly for cases the old RFC did not explicitly over (nested user-defined types, raw pointers),
and adjusts to decisions that have been made in the mean time (accepting floating-point patterns).

This RFC came out of discussions in a [t-lang design meeting](https://github.com/rust-lang/lang-team/issues/220).

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- When a constant is used as a pattern in a `const fn`, what exactly should we require?
  Writing `x == C` would *not* be possible here since calling trait methods like `partial_eq` is not possible in `const fn` currently.
  In the future, writing `x == C` will likely require a `const impl PartialEq`.
  So by allowing `match x { C => ... }`, we are allowing uses of `C` that would not otherwise be permitted, which is exactly what the `PartialEq` check was intended to avoid.
  On the other hand, all this does is to allow at compile-time what could already be done at run-time, so maybe that's okay?
  Rejecting this would definitely be a breaking change; we currently don't even lint against these cases.
  Also see [this issue](https://github.com/rust-lang/rust/issues/119398).

# Future possibilities
[future-possibilities]: #future-possibilities

- At some point, we might want to stabilize the `StructuralPartialEq` trait.
  There are however plenty of open questions here:
  + Should `StructuralPartialEq` be an unsafe trait?
    That trait has a clear semantic meaning, so making it `unsafe` to be able to rely on it for soundness is appealing.
    In particular, we could then be *sure* that pattern matching always has the same semantics as `==`.
    With the trait being safe, it is actually possible to write patterns that behave different from the `PartialEq` that is explicitly defined and intentionally exposed on that type,
    but only if one of the involved crates implements `StructuralPartialEq` incorrectly.
    This can lead to semver issues and logic bugs, but that is all allowed for safe traits.
    However, this also means unsafe code cannot rely on `==` and pattern matching having the same behavior.
    To make the trait unsafe, the logic for `derive(PartialEq)` should use `#[allow_internal_unsafe]` to still pass `forbid(unsafe_code)`.
  + What should the `StructuralPartialEq` trait be called?
    The current name can be considered confusing because the trait reflects a shallow property, while the value-based check performed when a constant is used in a pattern is defined recursively.
  + Trait bounds `T: StructuralPartialEq` seem like a strange concept. Should we really allow them?

- To avoid interpreting `derive(PartialEq)` as a semver-stable promise of being able to structurally match this type,
  we could introduce an explicit `derive(PartialEq, StructuralPartialEq)`.
  However, that would be a massive breaking change, so it can only be done over an edition boundary.
  It probably would also want to come with some way to say "derive me all the usual traits" so that one does not have to spell out so many trait names.
- The semver stability argument only applies cross-crate.
  That indicates a possible future where inside the crate that a type was defined in (or, if we want to take into account abstraction: everywhere that we can access all private fields of the type),
  we allow matching even when there is no `PartialEq` implementation.
  However, when there is a custom (non-derived) `PartialEq`, we need to be mindful of programmers that expect `match` to work like `==`, so it is not clear whether we want to allow that.
  The RFC is deliberately minimal and hence does not introduce any crate-dependent rules for constants in patterns;
  it's been more than seven years since RFC 1445 was accepted, so we don't want to delay completing its implementation any further by adding new features.
- We could consider introducing the concept of "pattern aliases" to let one define named "constants" also for patterns that contain wildcards and ranges.
  These pattern aliases could even have binders.
- Eventually we might want to allow matching in constants that do *not* have a structural equality operation, and instead have completely user-defined matching behavior.
  By rejecting matching on non-structural-equality constants, this proposal remains future compatible with such a new language feature.
- In the future, we could consider allowing more types in range patterns via a further opt-in, e.g. something like `StructuralPartialOrd`.
