- Feature Name: `stabilize_marker_freeze`
- Start Date: 2024-05-10
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Stabilize `core::marker::Freeze` in trait bounds.

# Motivation
[motivation]: #motivation

With 1.78, Rust [stabilized](https://github.com/rust-lang/rust/issues/121250) the requirement that `T: core::marker::Freeze` for `const REF: &'a T = T::const_new();` (a pattern referred to as "static-promotion" regardless of whether `'a = 'static`) to be legal. `T: core::marker::Freeze` indicates that `T` doesn't contain any un-indirected `UnsafeCell`, meaning that `T`'s memory cannot be modified through a shared reference.

The purpose of this change was to ensure that interior mutability cannot affect content that may have been static-promoted in read-only memory, which would be a soundness issue.

However, this new requirement also prevents using static-promotion to allow generics to provide a generic equivalent to `static` (with the distinction that static-promotion doesn't guarantee a unique address for the promoted content). An example of this pattern can be found in `stabby` and `equator`'s shared way of constructing v-tables:
```rust
pub trait VTable<'a>: Copy {
	const VT: &'a Self;
}
pub struct VtAccumulator<Tail, Head> {
	tail: Tail,
	head: Head,
}
impl<Tail: VTable<'a>, Head: VTable<'a>> VTable<'a> for VtAccumulator<Tail, Head> {
	const VT: &'a Self = &Self {tail: *Tail::VT, head: *Head::VT}; // Doesn't compile since 1.78
} 
```

Making `VTable` a subtrait of `core::marker::Freeze` in this example is sufficient to allow this example to compile again, as static-promotion becomes legal again. This is however impossible as of today due to `core::marker::Freeze` being restricted to `nightly`.

Orthogonally to static-promotion, `core::marker::Freeze` can also be used to ensure that transmuting `&T` to a reference to an interior-immutable type (such as `[T; core::mem::size_of::<T>()]`) is sound (as interior-mutation of a `&T` may lead to UB in code using the transmuted reference, as it expects that reference's pointee to never change). This is notably a safety requirement for `zerocopy` and `bytemuck` which currently use their own equivalents of `core::marker::Freeze` to ensure that requirement, which imposes great maintenance pressure on these crates to ensure they support as many types as possible. They would notably benefit from `core::marker::Freeze`'s status as an auto-trait.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

`core::marker::Freeze` is a trait that indicates the shallow lack of interior mutability in a type: it indicates that the memory referenced by `&T` is guaranteed not to change under defined behaviour.

It is automatically implemented by the compiler for any type that doesn't shallowly contain a `core::cell::UnsafeCell`.

Notably, a `const` can only store a reference to a value of type `T` if `T: core::marker::Freeze`, in a pattern named "static-promotion".

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The following documentation is lifted from the current nightly documentation.
```markdown
Used to determine whether a type contains
any `UnsafeCell` internally, but not through an indirection.
This affects, for example, whether a `static` of that type is
placed in read-only static memory or writable static memory.
This can be used to declare that a constant with a generic type
will not contain interior mutability, and subsequently allow
placing the constant behind references.
# Safety
This trait is a core part of the language, it is just expressed as a trait in libcore for
convenience. Do *not* implement it for other types.
```

The current _Safety_ section may be removed, as manual implementation of this trait is forbidden.

From a cursary review, the following documentation improvements may be considered:

```markdown
[`Freeze`](core::marker::Freeze) marks all types that do not contain any un-indirected interior mutability.
This means that their byte representation cannot change as long as a reference to them exists.

Note that `T: Freeze` is a shallow property: `T` is still allowed to contain interior mutability,
provided that it is behind an indirection (such as `Box<UnsafeCell<U>>`).

Notable interior mutability sources are [`UnsafeCell`](core::cell::UnsafeCell) (and any of its safe wrappers
such the types in the [`cell` module](core::cell) or [`Mutex`](std::sync::Mutex)) and [atomics](core::sync::atomic). 

`T: Freeze` is notably a requirement for static promotion (`const REF: &'a T;`) to be legal.

Note that static promotion doesn't guarantee a single address: if `REF` is assigned to multiple variables,
they may still refer to distinct addresses.

Whether or not `T: Freeze` may also affect whether `static STATIC: T` is placed
in read-only static memory or writeable static memory, or the optimizations that may be performed
in code that holds an immutable reference to `T`.
```

Mention could be added to `UnsafeCell` and atomics that adding one to a previously `Freeze` type without an indirection (such as a `Box`) is a SemVer hazard, as it will revoque its implementation of `Freeze`.

# Drawbacks
[drawbacks]: #drawbacks

- Some people have previously argued that this would be akin to exposing compiler internals.
	- The RFC author disagrees, viewing `Freeze` in a similar light as `Send` and `Sync`: a trait that allows soundness requirements to be proven at compile time.
- `Freeze` being an auto-trait, it is, like `Send` and `Sync` a sneaky SemVer hazard.
	- Note that this SemVer hazard already exists through the existence of static-promotion, as examplified by the following example:
	```rust
	// old version of the crate.
	mod v1 {
		pub struct S(i32);
		impl S {
			pub const fn new() -> Self { S(42) }
		}
	}

	// new version of the crate, adding interior mutability.
	mod v2 {
		use std::cell::Cell;
		pub struct S(Cell<i32>);
		impl S {
			pub const fn new() -> Self { S(Cell::new(42)) }
		}
	}

	// Old version: builds
	const C1: &v1::S = &v1::S::new();
	// New version: does not build
	const C2: &v2::S = &v2::S::new();
	```

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- The benefits of stabilizing `core::mem::Freeze` have been highlighted in [Motivation](#motivation).
	- By not stabilizing `core::mem::Freeze` in trait bounds, we are preventing useful and sound code patterns from existing which were previously supported.
- Alternatively, a non-auto sub-trait of `core::mem::Freeze` may be defined:
	- While this reduces the SemVer hazard by making its breakage more obvious, this does lose part of the usefulness that `core::mem::Freeze` would provide to projects such as `zerocopy`.
	- A "perfect" derive macro should then be introduced to ease the implementation of this trait. A lint may be introduced in `clippy` to inform users of the existence and applicability of this new trait.

# Prior art
[prior-art]: #prior-art

- This feature has been available in `nightly` for 7 years, and is used internally by the compiler.
- The work necessary for this RFC has already been done and merged in [this PR](https://github.com/rust-lang/rust/issues/121675), and a [tracking issue](https://github.com/rust-lang/rust/issues/121675) was opened.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

[Should the trait be exposed under a different name?](https://github.com/rust-lang/rust/pull/121501#issuecomment-1962900148)

# Future possibilities
[future-possibilities]: #future-possibilities

- One might later consider whether `core::mem::Freeze` should be allowed to be `unsafe impl`'d like `Send` and `Sync` are, possibly allowing wrappers around interiorly mutable data to hide this interior mutability from constructs that require `Freeze` if the logic surrounding it guarantees that the interior mutability will never be used.
	- The current status-quo is that it cannot be implemented manually (experimentally verified with 2024-05-12's nightly).
	- The RFC author is unable to tell whether allowing manual implementation may cause the compiler to generate unsound code (even if the wrapper correctly prevents interior mutation), but judges that the gains of allowing these implementations are too small to justify allowing the risk.
	- This consideration is purposedly left out of scope for this RFC to allow the stabilization of its core interest to go more smoothly; these two debates being completely orthogonal.
- Adding a `trait Pure: Freeze` which extends the interior immutability guarantee to indirected data could be valuable:
	- This is however likely to be a fool's errand, as indirections could (for example) be hidden behind keys to global collections. 
	- Providing such a trait could be left to the ecosystem unless we'd want it to be an auto-trait also (unlikely).