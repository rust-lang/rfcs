- Feature Name: local_trait_restriction
- Start Date: 2025-05-27  
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary

This RFC proposes introducing a compiler-reserved virtual trait, `LocalToThisCrate`, which is automatically implemented for all types defined in the current crate and cannot be implemented manually. It can be used as a bound to indicate that a generic type is guaranteed to be local, allowing generic implementations of external traits for local types without violating Rust’s orphan rule.

# Motivation

Rust’s orphan rule prevents implementing an external trait for an external type in a third-party crate, ensuring trait coherence but limiting expressiveness in some generic use cases.

A common example occurs in operator overloading. Suppose your crate defines several local types (e.g., `Point`, `Vec2`) that can be converted into a shared representation (`Calculable`). You might want to define a generic implementation of `Add` for any two types convertible into `Calculable`. However, Rust disallows such an implementation due to the orphan rule—because the compiler cannot guarantee that the generic parameters are local, and `Add` is an external trait.

Current workarounds—like defining local wrapper types or repeating per-type implementations—are widely used in the Rust community but lead to verbosity, code duplication, and maintenance overhead. As the number of local types grows, these drawbacks become more painful.

This RFC introduces a compiler-defined trait, `LocalToThisCrate`, which allows filtering trait implementations to apply only when type parameters are local. This enables generic trait impls (like `Add`) that remain coherent and safe, without the need for repetitive boilerplate.

Ultimately, this proposal improves code ergonomics, encourages abstraction, and makes Rust’s powerful trait system more accessible in practical, real-world scenarios—without sacrificing safety or coherence.

If accepted, it will enable expressive, maintainable patterns in generic code that are currently blocked, and make Rust codebases cleaner by removing unnecessary duplication.

# Guide-level explanation

This feature introduces a special compiler-defined trait:

```rust
#[compiler_built_in]
trait LocalToThisCrate {}
```

## What is `LocalToThisCrate`?

* It is **automatically implemented** by the compiler for all types defined in the current crate.
* It **cannot be implemented manually**.
* It is only **usable inside the crate** where the types are defined.
* It allows **filtering generic impls** based on crate locality.

## Why does it exist?

In Rust, the *orphan rule* forbids implementing an external trait (like `Add`, `Display`, etc.) for a type you don’t own. This ensures coherence across the ecosystem, but it also prevents writing generic trait impls that are *only* meant for local types—even when those impls are perfectly safe.

The `LocalToThisCrate` trait gives crate authors a way to *opt into* a safe subset of generic impls: *generic, but only for local types*. It works like a type-level permission check.

## When should you use it?

Use `LocalToThisCrate` when you want to:

* Write generic implementations of external traits (`Add`, `Serialize`, etc.)
* But **only for types that are defined in your own crate**
* And avoid repeating the same boilerplate logic for each type

## Example: operator overloading

Without this feature, you might have to write:

```rust
impl Add<Vec2> for Point {
    type Output = Calculable;
    fn add(self, rhs: Vec2) -> Self::Output {
        self.into() + rhs.into()
    }
}
impl Add<Point> for Vec2 { /* same logic */ }
// ... and so on for each pair of types
```

With `LocalToThisCrate`, you can now do:

```rust
impl<T: Into<Calculable> + LocalToThisCrate, U: Into<Calculable>> Add<U> for T {
    type Output = Calculable;
    fn add(self, rhs: U) -> Self::Output {
        self.into() + rhs.into()
    }
}
```

This allows **any pair of local types** to use the overloaded `+` operator, so long as they implement `Into<Calculable>`—no duplication, no wrappers, no manual matching of each type pair.

## Error messages

If you accidentally try to use `LocalToThisCrate` on a non-local type, you will get a compiler error like:

```
error[E0321]: `T` may be a type defined outside of the current crate
 --> src/lib.rs:10:6
  |
9 | impl<T: ExternalTrait + LocalToThisCrate> ExternalTrait for T {
  |                                          --------------------
  |                                          |
  |                                          doesn't implement `LocalToThisCrate`
10|     fn do_something(&self) { ... }
  |      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

Or, if used outside the defining crate:

```
error[E0277]: the trait bound `ForeignType: LocalToThisCrate` is not satisfied
 --> src/main.rs:5:10
  |
5 | impl<T: LocalToThisCrate> Debug for T {
  |          ^^^^^^^^^^^^^^^^ `ForeignType` is not defined in this crate
```

These are meant to guide you toward correct usage by pointing out that only local types can participate.

## Teaching perspective

* **For new Rust programmers**, this feature can be introduced after covering traits and the orphan rule. It offers a useful *middle ground* between local-only and fully generic trait impls.
* **For experienced Rustaceans**, it offers a new design tool: safe generic overloading for local APIs, without sacrificing coherence or needing verbose newtype patterns.

## Impact on code clarity

This feature makes code **more readable and more maintainable** by eliminating:

* Boilerplate impls that repeat the same logic
* Wrappers that hide true semantics
* Special-case macros to work around the orphan rule

It lets you focus on **what your code does**, not on **how to appease the coherence checker**.

# Reference-level explanation

[reference-level-explanation]: #reference-level-explanation

This RFC proposes the addition of a compiler-defined trait, `LocalToThisCrate`, which is automatically implemented for all types defined within the current crate, and only for those types. The trait itself cannot be implemented or derived by users.

## Trait Definition

```rust
#[compiler_built_in]
pub trait LocalToThisCrate {}
```

* The compiler injects this trait into the prelude or a known internal namespace.
* The trait is automatically implemented for all types defined in the current crate.
* It is **not** implemented for any foreign types (types defined in other crates).
* It cannot be manually implemented by users, even through `unsafe impl`.

This trait acts as a *type-level marker* to indicate that a given type is "local", in the same sense as defined by the orphan rule. This gives users a sound and compiler-checked way to write trait implementations that are **semantically generic** but **coherently limited to local types**.

## Implementation Details

The implementation of this feature involves:

1. **Compiler-generated trait**: The Rust compiler adds `LocalToThisCrate` to all local types (structs, enums, unions, aliases, etc.), including those inside generic modules.
2. **Type checker enforcement**: Any usage of `LocalToThisCrate` in bounds (e.g., `T: LocalToThisCrate`) is checked during type inference. If `T` might be external, the bound fails to be satisfied.
3. **Trait coherence check**: `impl` blocks using `LocalToThisCrate` as a constraint are permitted even when the trait and the type are both external, because the bound ensures that the `impl` applies only to local instantiations.

This avoids violating the orphan rule because the impl is not actually applicable to any types outside the crate.

## Revisiting the `Add` Example

Given:

```rust
struct Point { x: f64, y: f64 }
struct Vec2  { dx: f64, dy: f64 }

impl From<Point> for Calculable { /* ... */ }
impl From<Vec2>  for Calculable { /* ... */ }
```

Without this feature, this impl would be illegal:

```rust
impl<T: Into<Calculable>, U: Into<Calculable>> Add<U> for T {
    type Output = Calculable;
    fn add(self, rhs: U) -> Self::Output {
        self.into() + rhs.into()
    }
}
```

Because `T` and `U` might be foreign, and `Add` and `T` are both external.

With this feature, we can safely write:

```rust
impl<T: Into<Calculable> + LocalToThisCrate, U: Into<Calculable>> Add<U> for T {
    type Output = Calculable;
    fn add(self, rhs: U) -> Self::Output {
        self.into() + rhs.into()
    }
}
```

This impl is now allowed because `LocalToThisCrate` ensures `T` is local to the crate, making this `impl` valid under Rust’s coherence rules.

## Interactions with Other Features

* **Blanket impls**: This does not conflict with existing blanket impls because it only applies to local types. It is also guaranteed not to overlap with impls from other crates.
* **Auto traits**: This trait is not auto or marker in the Rust standard sense, but behaves similarly in how it is implemented automatically and used in trait bounds.
* **Macros and generic code**: Macros can leverage `LocalToThisCrate` to auto-generate coherent generic trait impls safely.

## Corner Cases

* **Type aliases**: Aliases to external types do not implement `LocalToThisCrate`.

* **Generic parameters**: If a type is generic over another (e.g. `MyWrapper<T>`), `MyWrapper<T>` implements `LocalToThisCrate` if and only if it is defined locally — even if `T` is foreign.

  ```rust
  struct MyWrapper<T>(T); // Local

  fn only_local<T: LocalToThisCrate>(value: T) { ... }

  only_local(MyWrapper<ExternalType>); // OK
  only_local(ExternalType); // Error
  ```

# Drawbacks

[drawbacks]: #drawbacks

The main drawback of this proposal is **increased language complexity**. Introducing a new built-in trait (`LocalToThisCrate`) adds an additional implicit concept to learn for users, particularly for those unfamiliar with the orphan rule or trait coherence.

Additionally:

* It may obscure the boundaries of what is and isn't allowed in trait implementations if not well documented.
* There's potential for confusion if users expect to be able to implement or use `LocalToThisCrate` themselves.
* It creates a **new kind of compiler magic**, which might be seen as violating Rust’s ethos of explicitness.
* The feature could potentially be **misused**, with developers relying on it to bypass coherence in cases where more principled design would be better (e.g., rethinking ownership or trait scopes).

It also introduces some **implementation complexity in the compiler**, as this trait must be specially handled and deeply integrated into trait resolution and coherence checking.

---

## Rationale and alternatives

[rationale-and-alternatives]: #rationale-and-alternatives

This design strikes a balance between **generic expressiveness** and **coherence safety**. It enables useful, previously impossible trait impls, while preserving the integrity of Rust’s orphan rule.

## Why this design?

* It provides a **lightweight, scoped mechanism** to declare “this is a local type” — without weakening coherence for external types.
* It lets developers write **fewer boilerplate impls** and more DRY code, especially in generic libraries using operator traits (`Add`, `Mul`, etc.).
* The design is **non-invasive**: it introduces no new syntax and has no runtime effect. It's fully opt-in and only affects trait resolution.
* It is **coherence-preserving**. Since the trait is unimplementable for foreign types, blanket impls stay sound.

## Alternatives considered

1. **Procedural macros to generate impls per-type**
   Common in current Rust, but:

   * Requires macro complexity.
   * Leads to scattered impls and harder-to-read code.
   * Does not scale well when combinatorially many impls are needed.

2. **New syntax to allow “local impls of foreign traits for foreign types”**
   Would break coherence and global reasoning. Not sound under current model.

3. **Negative trait bounds (e.g., `!Foreign`)**
   Proposed in the past, but:

   * Hard to define consistently.
   * Would require extensive trait system changes.

4. **Using sealed traits to emulate coherence guards**
   Sealed traits can simulate some behavior but:

   * Require boilerplate and coordination across crates.
   * Don’t work in general for generic impls or external traits.

## What is the impact of not doing this?

* Developers are forced to write **duplicated impls** or wrap types unnecessarily.
* Libraries may become **less ergonomic**, especially for user-defined types that conceptually share behaviour.
* It discourages idiomatic use of generic programming in Rust due to coherence friction.

## Could this be done in a library?

No — coherence and orphan rules are enforced at the language level. A library cannot define a trait that is *only* implemented for local types in a sound, compiler-guaranteed way. This proposal **requires compiler support** to be safe and effective.

# Prior art

The problem of implementing traits for types across crate boundaries while preserving coherence is well-known in Rust and other languages with trait or typeclass systems.

## Rust and orphan rule

Rust’s orphan rule is a fundamental design choice ensuring trait coherence, but it often leads to boilerplate and ergonomic limitations, especially in generic code involving external traits and local types.

## Similar problems and solutions in Rust ecosystem

* **Newtype pattern**: Wrapping types locally to implement external traits is the most common workaround. It is widely used but criticized for verbosity and poor ergonomics.
* **Sealed traits**: Used to restrict implementations to local types or crates, but they require coordination and don’t solve the problem of generic implementations of external traits for generic local types.
* **Procedural macros**: Sometimes used to generate repetitive impls, but this adds complexity and maintenance burden.

## Other languages with similar challenges

* **Haskell**: Uses typeclasses with orphan instances rules similar to Rust’s orphan rule. The community has developed workarounds such as newtype wrappers and advanced extensions like `FlexibleInstances` or `OverlappingInstances` (which can break coherence and are controversial).
* **Scala**: Uses implicits and extension methods with less strict coherence rules, but this sometimes leads to ambiguities and implicit resolution conflicts.
* **C++**: Uses template specialization and SFINAE for similar purposes, but lacks a direct equivalent to Rust’s orphan rule; thus, some problems are solved differently with trade-offs.

## Academic papers and discussions

* Various papers on typeclass coherence, trait resolution, and modular type systems discuss the trade-offs between coherence and flexibility, highlighting the difficulty of allowing generic external trait impls while preserving soundness.
* Discussions in the Rust community (RFCs, forums, and GitHub issues) frequently touch on coherence and orphan rules, showing ongoing interest in improving ergonomics without sacrificing safety.

## Lessons learned

* Introducing a built-in “local type” marker trait is a novel but natural extension of existing patterns.
* It respects Rust’s fundamental coherence principles while improving ergonomics.
* It avoids the pitfalls of other language approaches that sacrifice soundness or cause ambiguity.

# Unresolved questions

[unresolved-questions]: #unresolved-questions

* **Scope of `LocalToThisCrate` trait:**
  Should the trait only apply to nominal types defined in the current crate, or should it also cover certain generic instantiations or type aliases? Clarifying what "local" means precisely may require further discussion.

* **Interaction with other coherence rules:**
  How will this new trait interact with existing rules about trait coherence, specialization, and orphan rules? Are there edge cases or conflicts that need to be resolved during implementation?

* **Compiler implementation details:**
  What is the best approach for the compiler to automatically implement and enforce this trait? Will it require changes to type inference or trait resolution logic?

* **Impact on ecosystem:**
  Could this trait affect existing crates or ecosystems, especially those using complex trait patterns or unsafe code relying on coherence assumptions?

* **Extensibility and future features:**
  Can this mechanism be extended to support similar use cases, such as restricting impls to "local-ish" types or specific crate hierarchies?

* **Related features out of scope:**
  This RFC does not address other coherence challenges like overlapping impls, or orphan rule exceptions for specific external crates. Those remain for future RFCs.

# Future possibilities

* **Extension to other orphan-rule related cases:**
  The concept of a compiler-reserved trait like `LocalToThisCrate` could be extended to express other ownership or locality properties of types, helping with related coherence challenges beyond operator overloading.

* **Granular locality predicates:**
Future extensions might allow specifying locality at a finer granularity, for example "local to this file," "local to this module," "local to this workspace," or "local to a set of crates," enabling more flexible control over implementation scopes.

* **Integration with specialization and GATs:**
  As Rust evolves with features like specialization and generic associated types, `LocalToThisCrate` could play a role in resolving coherence ambiguities or enabling new patterns that require fine-grained control over impl applicability.

* **Tooling and diagnostics support:**
  Enhanced compiler diagnostics could leverage the locality trait to provide clearer error messages and migration suggestions, especially when orphan rule violations occur.

* **Macros or attribute-driven variants:**
  Explore whether macros or attributes can be designed to simulate or partially implement locality restrictions in user code, possibly easing migration or experimentation before compiler support is stable.

* **Ecosystem patterns and best practices:**
  Over time, best practices could emerge around using this feature in popular crates and libraries, potentially influencing idiomatic Rust for generic abstractions and operator overloading.

If no immediate further possibilities are identified, this section can remain open for future exploration.
