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

Detailed Design
Syntax
A trait definition may include a permits clause:

```rust
trait TraitName permits crate, other_crate, another_crate {
    fn method(&self);
}
```

crate refers to the defining crate.

Other identifiers refer to external crates by name.

If no permits clause is present, behavior is unchanged (any crate may implement the trait, subject to orphan rules).

## Semantics
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

## Drawbacks
Reduced flexibility: Downstream crates cannot extend traits unless explicitly permitted.

Ecosystem impact: Existing crates relying on open trait impls may break if traits adopt permits.

Complexity: Adds another axis of privacy control to the language.

Alternatives
Continue using the sealed trait pattern (private marker traits).

Use documentation and conventions to discourage external impls.

Unresolved Questions
Should permits support paths (e.g., permits crate::submodule)?

How does this interact with crate renaming in Cargo?

Should permits apply to blanket impls (impl<T> Trait for T)?

Future Directions
Extend permits to allow finer-grained control (e.g., permitting only certain modules).

Combine with unsafe traits to enforce stricter safety boundaries.

Explore integration with Cargo features for conditional permitting.


The permits syntax provides a direct, ergonomic way to restrict trait implementations to specific crates, improving safety, auditability, and clarity compared to current patterns.

