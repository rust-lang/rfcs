# RFC: Trait Implementation Privacy with `permits`

- Feature Name: `trait_permits`
- Start Date: 2026-01-03
- RFC PR: rust-lang/rfcs#0000
- Rust Issue: rust-lang/rust#0000

## Summary

Introduce a new syntax for restricting trait implementations to specific crates:

```rust
trait Test permits mycrate_extra, crate {
    fn run(&self);
}
```

## Motivation
Rust currently allows any downstream crate to implement traits for types they own, subject to the orphan rules. While flexible, this can lead to:

Accidental or malicious impls: External crates can implement traits in ways that break invariants.

Audit difficulty: It is hard to know which crates are allowed to implement a trait.

Boilerplate sealed traits: Developers often use the "sealed trait" pattern to prevent external impls, which is verbose and indirect.

By introducing permits, Rust gains a first-class mechanism for trait implementation privacy, improving safety and clarity.

## Guide-level explanation

The `permits` clause allows trait authors to specify which crates may provide implementations of the trait.

```rust
pub trait Test permits crate, mycrate_extra {
    fn run(&self);
}
crate refers to the defining crate.

Other identifiers refer to external crates by name.

If no permits clause is present, behavior is unchanged (any crate may implement the trait, subject to orphan rules).

This makes trait privacy explicit and auditable, without relying on sealed traits. It allows traits to be widely used across the ecosystem while restricting who can implement them, reducing accidental or malicious impls and improving auditability.

Code

## Reference-level explanation

### Syntax

A trait definition may include a `permits` clause:

```rust
trait TraitName permits crate, other_crate, another_crate {
    fn method(&self);
}


### Semantics
Only the listed crates may provide impl TraitName for Type.

Attempting to implement the trait in a non-permitted crate results in a compiler error.

Trait objects (&dyn TraitName) remain usable across crates, but only permitted crates can provide concrete impls.

Example
```rust
// In mycrate/lib.rs
pub trait Test permits crate, mycrate_extra {
    fn run(&self);
}

// In mycrate_extra/lib.rs
use mycrate::Test;

struct ExtraType;
impl Test for ExtraType {
    fn run(&self) { println!("extra"); }
}

// In othercrate/lib.rs
use mycrate::Test;

struct OtherType;
impl Test for OtherType {
    fn run(&self) { println!("other"); }
}
// ERROR: Trait `Test` does not permit implementations in `othercrate`
```

### Diagnostics
When a non-permitted crate attempts to implement a restricted trait, the compiler emits an error pointing to the trait definition and listing permitted crates:


```rust
error[E0XXX]: trait `Test` does not permit implementations in crate `othercrate`
  --> othercrate/src/lib.rs:5:1
   |
5  | impl Test for OtherType { /* ... */ }
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = note: permitted crates for `mycrate::Test`: `crate`, `mycrate_extra`
   = help: consider using a wrapper type or requesting inclusion in the permits list
```

## Drawbacks
Reduced flexibility: Downstream crates cannot extend traits unless explicitly permitted.

Ecosystem impact: Existing crates relying on open trait impls may break if traits adopt permits.

Complexity: Adds another axis of privacy control to the language.

## Rationale and alternatives

Several approaches exist today to restrict trait implementations, but each has limitations compared to a language-level `permits` clause:

- **Sealed trait pattern**  
  Authors define a private marker trait and make the public trait inherit from it.  
  This prevents external crates from implementing the trait, but it requires boilerplate and is not discoverable in documentation.  
  `permits` encodes the restriction directly in the trait definition.

- **Documentation and conventions**  
  Library authors may state in docs that a trait should not be implemented externally.  
  This relies on ecosystem norms and cannot prevent accidental or malicious impls.  
  `permits` provides compiler enforcement rather than social convention.

- **Visibility restrictions**  
  Making a trait non-`pub` blocks external use entirely, not just external impls.  
  This is too coarse: traits often need to be widely usable but narrowly implementable.  
  `permits` allows fine-grained control.

- **Attributes instead of syntax**  
  An attribute like `#[permits(crate, mycrate_extra)]` could encode the restriction.  
  However, a keyword clause improves clarity in signatures and avoids attribute proliferation.

- **Tooling-only solutions**  
  Lints or Clippy rules could warn about external impls, but they cannot guarantee enforcement across crates.  
  A language-level mechanism ensures consistency and reliability.

The `permits` clause is chosen because it is explicit, ergonomic, and auditable. It integrates naturally into trait definitions and provides compiler-backed enforcement, reducing reliance on patterns or conventions.



The permits syntax provides a direct, ergonomic way to restrict trait implementations to specific crates, improving safety, auditability, and clarity compared to current patterns.


## Prior art

Rust developers today often rely on the **sealed trait pattern** to restrict external implementations. This involves defining a private marker trait and making the public trait inherit from it, preventing downstream crates from writing impls. While effective, it is verbose and indirect, and the restriction is not visible in documentation.

Other languages provide similar mechanisms:

- **Swift** distinguishes between `open` and `public` to control subclassing and overriding.  
- **Java** introduced **sealed classes and interfaces**, which restrict inheritance to a fixed set of types.  
- **Haskell** has long debated the problem of **orphan instances**, where typeclass implementations can appear outside the defining module, leading to conflicts and incoherence. Libraries often discourage or forbid such instances by convention.

These examples show that ecosystems benefit from explicit language-level controls on extension and implementation. Rust’s `permits` clause would provide a comparable, ergonomic solution tailored to Rust’s trait system.

## Unresolved questions

Several aspects of the `permits` design remain open for discussion:

- **Crate identity**  
  Should `permits` list Cargo package names, crate names, or some stable identity?  
  How should renames and `extern crate` aliases be handled?

- **Paths and modules**  
  Should `permits` support finer granularity, such as submodules or subcrates, or remain crate-scoped only?

- **Blanket impls**  
  Are special diagnostics or rules needed for blanket impls (`impl<T> Trait for T`) that could broadly constrain downstream type space?

- **Negative impls**  
  Should negative impls (`impl !Trait for Type`) be restricted in the same way, and are there additional caveats?

- **Dependency cycles**  
  Does permitting multiple crates introduce any issues with cyclic dependencies or mutually permitted crates?

- **Re-exported traits**  
  If a trait is re-exported from another crate, does the `permits` clause need additional metadata to clarify permissions?

- **Unsafe traits**  
  Should unsafe traits combined with `permits` have distinct diagnostics to highlight safety boundaries?


## Future possibilities

The `permits` clause could be extended or combined with other language features in the future:

- **Finer-grained control**  
  Extend `permits` to allow restrictions at the module or type level, not just crate-wide.

- **Conditional permits**  
  Integrate with Cargo features to enable or disable permitted crates based on feature flags.

- **Discovery tooling**  
  Provide compiler and IDE support to surface permitted implementors, audit logs, and cross-crate intent in documentation.

- **Library design guidance**  
  Establish best practices for introducing `permits` without disrupting ecosystems, including migration strategies and communication patterns.

- **Safety integration**  
  Combine with `unsafe trait` semantics to enforce stricter safety boundaries across crate boundaries.
