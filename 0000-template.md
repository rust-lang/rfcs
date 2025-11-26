- Feature Name: pub_use_pub_glob
- Start Date: 2025-11-26
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC proposes a new syntax for safer glob exports:

```rust
pub use crate::mymod::pub *;
```

This re‑exports all items from a module that are already marked `pub`, while excluding private helpers. It balances ergonomics (no need to maintain huge allowlists) with safety (no accidental leakage of private items).

# Motivation
[motivation]: #motivation

Rust developers face a tension when designing crate APIs:

- **Allowlist exports** (`pub use crate::mymod::{A, B, C};`) are safe but tedious.  
  In large crates with hundreds of items, every new public item requires updating the allowlist.  
  This creates maintenance overhead and noisy diffs.

- **Glob exports** (`pub use crate::mymod::*;`) are convenient but unsafe.  
  They export *everything*, including private helpers, sealed traits, or unsafe functions.  
  This can unintentionally expand the public API surface.

- **Denylist exports** (imagined `*!{}` syntax) are error‑prone.  
  Forgetting to exclude a new private item leaks it into the public API.  
  This undermines Rust’s safety guarantees.

**Use cases:**
- Large frameworks (`tokio`, `serde`, `bevy`) with hundreds of public items.  
- Crates that want to expose a curated API surface without constant allowlist maintenance.  
- Libraries that evolve quickly, where new private helpers are added often.

The proposed `pub *` glob solves this by automatically exporting only items already marked `pub`.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Imagine teaching this to another Rust programmer:

```rust
mod mymod {
    pub fn useful() {}
    fn helper() {}
    pub struct Widget;
    struct Hidden;
}

pub use crate::mymod::pub *;
```

- At the crate root, only `useful` and `Widget` are exported.  
- `helper` and `Hidden` remain private.  
- Adding new public items automatically exports them.  
- Adding new private helpers requires no denylist updates.

**How to think about it:**  
- `pub use …::pub *;` is a **filtered glob export**.  
- It’s equivalent to enumerating all public items explicitly, but without the boilerplate.  
- It makes crate roots easier to maintain and safer to evolve.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

- Parsing: `pub *` is treated as a glob pattern filtered by visibility.  
- Semantics:  
  - Only items with `pub` visibility are re‑exported.  
  - Items with `pub(crate)` or `pub(super)` are excluded.  
  - Works with functions, structs, enums, traits, and constants.  
- Corner cases:  
  - `#[doc(hidden)]` items are still exported if `pub`.  
  - Macro exports (`pub macro`) follow the same rule.  
  - Nested modules: only their public items are exported if the module itself is `pub`.

# Drawbacks
[drawbacks]: #drawbacks

- API surface becomes implicit: readers must inspect the module to know what’s exported.  
- Could encourage over‑broad exports, reducing intentional curation.  
- Adds grammar complexity to `use` syntax.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- **Allowlist**: explicit but tedious.  
- **Denylist**: convenient but unsafe.  
- **Plain glob**: unsafe, leaks privates.  
- **Macros**: possible, but clunky and non‑idiomatic.  
- **Proposed `pub *`**: balances ergonomics and safety.

# Prior art
[prior-art]: #prior-art

- **Python**: `__all__` defines explicit exports, but requires manual maintenance (like allowlists).  
- **C#**: `using static` imports public members only.  
- **Rust today**: `pub use …::*` exports everything, no filter.  
- **This proposal**: filtered glob export, unique to Rust.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Should `pub(crate)` items be included if the export is inside the same crate?  
- Should `#[doc(hidden)]` items be excluded automatically?  
- Should this syntax allow nested filtering (e.g. `pub use …::pub {Structs, Traits}`)?

# Future possibilities
[future-possibilities]: #future-possibilities

- Extend to **visibility‑scoped globs**:  
  ```rust
  pub(crate) use crate::mymod::pub *;
  ```
- Extend to **denylist + pub filter hybrid**:  
  ```rust
  pub use crate::mymod::pub * !{UnsafeFn};
  ```
- Could integrate with **Cargo features** for conditional exports.
