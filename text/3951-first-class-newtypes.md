- Feature Name: (`first_class_newtypes`)
- Start Date: (2026-04-18)
- RFC PR: [rust-lang/rfcs#3892](https://github.com/rust-lang/rfcs/pull/3951)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary

This RFC proposes first-class language support for *nominal newtypes*, providing a lightweight mechanism to create distinct types from existing types with controlled trait propagation, optional structural transparency, and improved ergonomics over tuple structs and proc-macro wrappers.

The goal is to make the "newtype pattern" a first-class abstraction rather than a purely idiomatic convention, while preserving Rust’s emphasis on explicitness, zero-cost abstractions, and predictable trait coherence.

---

## Motivation

Rust’s current story for creating distinct types around existing representations relies on tuple structs:

```rust
#[derive(Clone)]
pub struct Identity([u8; 16]);

#[derive(Clone)]
struct ProtectFromForgery(Identity);
```

This works, but has several limitations:

1. **Boilerplate and inconsistency**

   * Every newtype requires manual forwarding of traits.
   * Derive works only for a subset of traits (Copy, Clone, Debug, etc.).
   * Behaviorally meaningful traits must be manually implemented or macro-generated.

2. **No structured control over shared behavior**

   * Traits on the inner type are not automatically considered.
   * Deref makes sense for smart pointers but it is frequently seen as an anti-pattern.

3. **Cross cutting concerns of smart pointers and type wrappers**

   * Newtypes are used for:
     * semantic typing `Kilometers = i32`
     * capability filtering
     * API contracts that depend on type system visibility
   * but the language encourages us to treat them like a smart pointer.

This RFC proposes treating newtypes as a *first-class nominal abstraction*, enabling both stronger guarantees and better ergonomics.

---

## Guide-level explanation

A **newtype** is a distinct type created from an existing type, with optional rules governing:

* representation (transparent or opaque)
* derived behavior
* capability restriction

### Proposed syntax

A newtype declaration resembles a type alias, but introduces a nominal type:

```rust
newtype ProtectFromForgery = Identity;
```

This creates a distinct type:

```rust
ProtectFromForgery != Identity
```

By default, this type is **opaque**, but may opt into representation transparency:

```rust
#[repr(transparent)]
newtype ProtectFromForgery = Identity;
```

---

## Trait behavior

One of the central design questions is trait propagation.

This RFC proposes **opt-in derives**, rather than implicit blanket forwarding.

### Explicit derivation

Traits may be derived on the newtype:

```rust
#[derive(Clone, Token)]
newtype ProtectFromForgery = Identity;
```

This differs from `#[derive]` on structs in that:

* it is not limited to compiler-known traits
* it can apply to user-defined traits
* it can generate forwarding impls to the inner type

---

## Key design question: automatic trait resolution

A central unresolved question is:

> Should impls on the inner type automatically apply to the newtype?

### Option A: No automatic resolution (recommended default)

```text
Identity implements Token
ProtectFromForgery does NOT
```

Pros:

* preserves capability restriction guarantees
* avoids accidental API leakage
* aligns with Rust’s explicitness philosophy

Cons:

* requires boilerplate or derives

---

### Option B: Full transparent inheritance

All impls on `Identity` are visible on `ProtectFromForgery`.

Pros:

* maximal ergonomics
* zero boilerplate

Cons:

* destroys the primary value of newtypes (capability boundary)
* makes reasoning about APIs difficult
* blurs nominal distinction

---

### Option C: Allow auto-deriving traits for a newtype (proposed)

This is the safest option as it requires the smallest amount of new syntax.
Reserving `newtype`.

This preserves:

* explicit capability boundaries
* ergonomic forwarding
* predictable trait resolution

This is the recommended design direction.

We can emulate it today with a procedural macro and trait resolution rules do
not have to change.

```rust
newtype! {
    #[derive(Clone, Token)]
    type ProtectFromForgery = Identity;
}
```

---

## Representation

A newtype may optionally guarantee identical layout:

```rust
#[repr(transparent)]
newtype ProtectFromForgery = Identity;
```

This guarantees:

* identical ABI representation
* safe transmutation (where allowed)
* FFI compatibility

Without `repr(transparent)`, layout is an implementation detail.

---

## Example: HTTP session extension pattern

This RFC targets a common Rust pattern: request-local state stored in extensions.

Current pattern:

```rust
self.extensions()
    .get::<ProtectFromForgery>()
```

With newtypes:

```rust
newtype ProtectFromForgery = Identity;
```

Trait separation becomes clearer:

```rust
pub trait Session {
    fn session(&self) -> Option<&Identity>;
}
```

Capability restriction is enforced at the type level rather than runtime casting.

---

## Separation of behavior and data

This RFC strongly encourages a separation model:

* **data types**: represent state
* **traits**: represent capability

Newtypes act as a *boundary layer* between them.

Example:

```rust
newtype Identity = [u8; 16];

pub trait Token {
    fn expires_at(&self) -> Result<i64, Error>;
}
```

This makes it possible to:

* restrict Token behavior to specific contexts
* avoid accidental API surface expansion
* encode domain constraints explicitly

---

## Drawbacks

### 1. Increased type system complexity

Depending on how newtypes are implemented, trait resolution can become a challenge.

### 2. Potential confusion with aliases

Users may conflate:

* `type X = Y;`
* `newtype X = Y;`

Clear syntax differentiation is required.

### 3. Trait resolution complexity

The interaction between:

* blanket impls
* orphan rules
* derive

requires careful compiler design.

---

## Alternatives considered

### 1. Continue using tuple structs

Rejected due to boilerplate and lack of expressive control.

### 2. Expand derive system only

Insufficient for user-defined traits and capability control.

### 3. Standardize macro-based newtypes

Already partially solved in ecosystem, but inconsistent and non-semantic.

---

## Open questions

1. Should we allow deriving `Deref` if [rust-lang/rfcs#3911](https://github.com/rust-lang/rfcs/pull/3911) is accepted?
2. Should newtypes have a way of applying impl blocks from their inner type?
3. How do we teach beginners about the difference between a tuple struct and newtype?

---

## Future work

* integration with const generics for parameterized newtypes
* potential linting around overuse of extensions maps
* ergonomic sugar for request-scoped capability wrappers

---

## Closing thought

This proposal attempts to formalize what is currently an idiomatic but fragmented pattern in Rust. The goal is not to reduce explicitness, but to make *explicit structure easier to express than accidental structure*, particularly in systems where type-based capability control is a core part of correctness.
