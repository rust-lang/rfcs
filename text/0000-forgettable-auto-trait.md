- Feature Name: forgettable_auto_trait
- Start Date: 2025-10-05
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Introduce a new auto trait `Forgettable`, implemented by all types that can be safely forgotten,  
and change the signature of `core::mem::forget` to require `T: Forgettable`.

This allows types to opt out of being safely forgotten by explicitly `impl !Forgettable for T`,  
preventing `mem::forget(t)` from compiling when dropping `t` is required for safety invariants.

# Motivation
[motivation]: #motivation

`core::mem::forget` is a powerful primitive that prevents the destructor of a value from running.  
Currently, **any type** can be forgotten. This prevents safely writing data structures that rely on the `Drop` implementation for preserving memory safety.

For most types, forgetting a value is safe — it simply leaks memory.  
However, some types rely on their destructor being called to maintain safety invariants.  

The [scoped_static](https://crates.io/crates/scoped_static) crate demonstrates this issue, where the implementation is technically unsafe since the `guard` can be forgotten.

# Guide-level explanation

[guide-level-explanation]: #guide-level-explanation

After this RFC, `core::mem::forget` would look like:

```rust
pub fn forget<T: Forgettable>(t: T);
```

The `Forgettable` trait is an **auto trait** —
it is implemented for all types unless explicitly opted out of with a negative impl:

```rust
auto trait Forgettable {}
```

For most code, nothing changes — all existing types remain `Forgettable`.

However, future code may declare certain types as **non-forgettable**, preventing misuse:

```rust
pub struct ScopeGuard<'a, T: 'static> { /* ... */ }

impl !Forgettable for ScopeGuard<'_, '_> {}
```

Now, attempting to forget a `ScopeGuard` or any type containing a `ScopeGuard`, will fail to compile:

```rust
let guard = ScopeGuard::new(&value);
std::mem::forget(guard); // ❌ error: `ScopeGuard<'_, _>` does not implement `Forgettable`
```

This gives library authors a clear, compiler-enforced way to ensure their destructors run.

# Reference-level explanation

[reference-level-explanation]: #reference-level-explanation

### New auto trait

```rust
pub auto trait Forgettable {}
```

* Implemented automatically for all types.
* Negative impls can be written:

  ```rust
  impl !Forgettable for MyGuard {}
  ```

### Modified standard library API

`core::mem::forget` would be updated from:

```rust
pub const fn forget<T>(t: T)
```

to:

```rust
pub const fn forget<T: Forgettable>(t: T)
```

### Interaction with existing features

* All existing code continues to compile since all types are `Forgettable` by default.
* The change is **fully backward compatible** at the language level.

### Safety rationale

This change strengthens Rust’s guarantees:

> Safe code can rely on `Drop` being ran for `T` where `T: !Forgettable` to prevent undefined behavior.

# Drawbacks

[drawbacks]: #drawbacks

* Adds a new core trait, increasing language complexity slightly.

# Rationale and alternatives

[rationale-and-alternatives]: #rationale-and-alternatives

### Alternatives

1. **Do nothing.**

   * Safe types that rely on `Drop` for safety are impossible to implement.
   * Using types that rely on drop for safety makes `mem::forget` remain unsafe in practice, despite being marked `safe`.

2. **Mark `mem::forget` as unsafe.**

   * Would be a massive breaking change.
   * Many safe abstractions and crates depend on its current safe semantics.

3. **Add a lint instead.**

   * A lint could warn about forgetting `Drop` types, but it could not guarantee safety statically.

### Why this design?

The `Forgettable` auto-trait model:

* Is **backward compatible**.
* Lets authors **opt out explicitly**.
* Aligns with Rust’s philosophy of “safe by default, opt out explicitly when needed.”

# Prior art

[prior-art]: #prior-art

* **C++ RAII:** Forgetting RAII guards is UB if destructors are skipped manually.
* **Swift ARC:** Always runs destructors; you cannot “forget” safely.
* **Rust unsafe cell patterns:** Similar principles exist with `Send` and `Sync` auto traits, where types opt out for safety.
* **Go finalizers:** No equivalent manual forget; this is Rust’s unique challenge due to `mem::forget`.

# Unresolved questions

[unresolved-questions]: #unresolved-questions

* Are there any future types in the core/std that can now be implemented with this once this guarantee is implemented?
* How would this interact with `ManuallyDrop`?
   * Should `ManuallyDrop` not work for any type that is `!Forgettable`.

# Future possibilities

[future-possibilities]: #future-possibilities

* Extend this mechanism to protect against other “safe UB” leaks such as:
  * `ManuallyDrop` misuse.
  * Forgetting FFI tokens or synchronization guards.

