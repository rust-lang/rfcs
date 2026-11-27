- Feature Name: `freeze`
- Start Date: 2024-05-10
- RFC PR: [rust-lang/rfcs#3633](https://github.com/rust-lang/rfcs/pull/3633)
- Tracking Issue: [rust-lang/rust#121675](https://github.com/rust-lang/rust/issues/121675)

# Summary
[summary]: #summary

- Stabilize `core::marker::Freeze` in trait bounds, renamed as `core::marker::NoCell` (this RFC will keep using `Freeze` when discussing historical uses of the trait, and use `NoCell` when discussing the newly stabilized trait).
- Provide a `PhantomCell` marker type to opt out of `NoCell`.
    - This type implements all auto traits except for `NoCell`.
- Change `PhantomData<T>` to implement `NoCell` only if `T: NoCell`.

# Motivation
[motivation]: #motivation

With 1.78, Rust [changed behavior](https://github.com/rust-lang/rust/issues/121250): previously, `const REF: &T = &expr;` was (accidentally) accepted even when `expr` may contain interior mutability.
Now this requires that the type of `expr` satisfies `T: core::marker::Freeze`, which indicates that `T` doesn't contain any un-indirected `UnsafeCell`, meaning that `T`'s memory cannot be modified through a shared reference.

The purpose of this change was to ensure that interior mutability cannot affect content that may have been static-promoted in read-only memory, which would be a soundness issue.
However, this new requirement also prevents using static-promotion to create constant references to data of generic type. This pattern can be used to approximate "generic `static`s" (with the distinction that static-promotion doesn't guarantee a unique address for the promoted content). An example of this pattern can be found in `stabby` and `equator`'s shared way of constructing v-tables:
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

Orthogonally to static-promotion, `core::marker::Freeze` can also be used to ensure that transmuting `&T` to a reference to an interior-immutable type (such as `[u8; core::mem::size_of::<T>()]`) is sound (as interior-mutation of a `&T` may lead to UB in code using the transmuted reference, as it expects that reference's pointee to never change). This is notably a safety requirement for `zerocopy` and `bytemuck` which are currently evaluating the use of `core::marker::NoCell` to ensure that requirement; or rolling out their own equivalents (such as zerocopy's `Immutable`) which imposes great maintenance pressure on these crates to ensure they support as many types as possible. They could stand to benefit from `core::marker::Freeze`'s status as an auto-trait, and `zerocopy` intends to replace its bespoke trait with a re-export of `core::marker::Freeze`.

Note that for this latter use-case, `core::marker::Freeze` isn't entirely sufficient, as an additional proof that `T` doesn't contain padding bytes is necessary to allow this transmutation to be safe, as reading one of `T`'s padding bytes as a `u8` would be UB.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

`core::marker::NoCell` is a trait that is implemented for any type whose memory layout doesn't contain any `UnsafeCell`: it indicates that the memory referenced by `&T` is guaranteed not to change while the reference is live.

It is automatically implemented by the compiler for any type that doesn't contain an un-indirected `core::cell::UnsafeCell`.

Notably, a `const` can only store a reference to a value of type `T` if `T: core::marker::NoCell`, in a pattern named "static-promotion".

As `core::marker::NoCell` is an auto-trait, it poses an inherent semver-hazard (which is already exposed through static-promotion). This RFC proposes the simultaneous addition and stabilization of a `core::marker::PhantomCell` type, to provide a stable means for maintainers to reliably opt out of `NoCell`, without forbidding zero-sized types. These types are currently `!NoCell` due to the conservativeness of `NoCell`'s implementation.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## `core::marker::NoCell`

The following documentation is lifted from `core::marker::Freeze`'s current nightly documentation.
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

From a cursory review, the following documentation improvements may be considered:

```markdown
[`NoCell`] marks all types that do not contain any un-indirected interior mutability.
This means that their byte representation cannot change as long as a reference to them exists.

Note that `T: NoCell` is a shallow property: `T` is still allowed to contain interior mutability,
provided that it is behind an indirection (such as `Box<UnsafeCell<U>>`).
Notable `!NoCell` types are [`UnsafeCell`](core::cell::UnsafeCell) and its safe wrappers
such as the types in the [`cell` module](core::cell), [`Mutex`](std::sync::Mutex), and [atomics](core::sync::atomic).
Any type which contains a non-`NoCell` type without indirection also does not implement `NoCell`.

`T: NoCell` is notably a requirement for static promotion (`const REF: &'a T;`) to be legal.

Note that static promotion doesn't guarantee a single address: if `REF` is assigned to multiple variables,
they may still refer to distinct addresses.

Whether or not `T` implements `NoCell` may also affect whether `static STATIC: T` is placed
in read-only static memory or writeable static memory, or the optimizations that may be performed
in code that holds an immutable reference to `T`.

# Semver hazard
`NoCell` being an auto-trait that encodes a low level property of the types it is implemented for,
you should avoid relying on external types maintaining that property, unless that
contract is explicitly stated out-of-band (through documentation, for example).

Conversely, authors that consider `NoCell` to be part of a type's contract should document this
fact explicitly.

## The ZST caveat
While `UnsafeCell<T>` is currently `!NoCell` regardless of `T`, allowing `UnsafeCell<T>: NoCell` if `T` is
a Zero-Sized-Type is currently under consideration.

Therefore, the advised way to make your types `!NoCell` regardless of their actual contents is to add a
[`PhantomCell`](core::marker::PhantomCell) field to it.

[`PhantomData<T>`](core::marker::PhantomData) only implements `NoCell` if `T` does, making it a good way
to conditionally remove a generic type's `NoCell` auto-impl.

# Safety
This trait is a core part of the language, it is just expressed as a trait in libcore for
convenience. Do *not* implement it for other types.
```

Mention could be added to `UnsafeCell` and atomics that adding one to a previously `NoCell` type without an indirection (such as a `Box`) is a SemVer hazard, as it will revoke its implementation of `NoCell`.

## Fixing `core::marker::PhantomData`'s `NoCell` impl

At time of writing, `core::marker::PhantomData<T>` implements `NoCell` regardless of whether or not `T` does.

This is now considered a bug, with the corrected behaviour being that `core::marker::PhantomData<T>` only implements `NoCell` if `T` does.

While crates that would "observe" this change exist, the current consensus is that it would only break invalid usages of that invariable bound.
Most crates that would observe this change could replace their usage of `core::marker::PhantomData<T>` by `core::marker::PhantomData<Ptr<T>>` where `Ptr<T>`
is a pointer-type with the relevant caracteristics regarding lifetime, `Send` and `Sync`ness.

The author doesn't have the necessary knowledge to implement this change, which should still be subject to a crater run to ensure no valid use-cases were missed.

This behaviour change shall be introduced at the same time as the stabilization of `NoCell` in bounds.

## `core::marker::PhantomCell`

This ZST is proposed as a means for maintainers to reliably opt out of `NoCell` without constraining currently `!NoCell` ZSTs to remain so.

Leveraging the proposed changes to `core::marker::PhantomData`'s `NoCell` impl, its implementation could be as trivial as a newtype or type alias on `core::marker::PhantomData<core::cell::SyncUnsafeCell<u8>>`,
with the following documentation:

```markdown
[`PhantomCell`] is a type with the following guarantees:
- It is guaranteed not to affect the layout of a type containing it as a field.
- Any type including it in its fields (including nested fields) without indirection is guaranteed to be `!NoCell`.

This latter property is [`PhantomCell`]'s raison-d'être: while other Zero-Sized-Types may currently be `!NoCell`,
[`PhantomCell`] is the only ZST (outside of [`PhantomData<T>`] where `T` isn't `NoCell`) that is guaranteed to stay that way.

Notable types that are currently `!NoCell` but might not remain so in the future are:
- `UnsafeCell<T>` where `core::mem::size_of::<T>() == 0`
- `[T; 0]` where `T: !NoCell`.
```

As this marker exists solely to remove `NoCell` implementations, it shall be `Send`, `Sync` and generally implement all non-`NoCell` traits that `PhantomData<()>` implements, in a similar fashion to `PhantomData<()>`

This new marker type shall be introduced at the same time as the stabilization of `NoCell` in bounds.

## Addressing the naming

A point of contention during the RFC's discussions was whether `NoCell` should be renamed, as `freeze` is already a term used in `llvm` to refer to an intrinsic which allows to safely read from uninitialized memory.
[Another RFC](https://github.com/rust-lang/rfcs/pull/3605) is currently open to expose this intrinsic in Rust.

Debates have landed on the conservation of the `NoCell` name, under the main considerations that:
- No better name was found despite the efforts in trying to find one,
- that the current name was already part of the Rust jargon,
- and that stabilizing this feature is too valuable to hold it back on naming.

# Drawbacks
[drawbacks]: #drawbacks

- Some people have previously argued that this would be akin to exposing compiler internals.
    - The RFC author disagrees, viewing `NoCell` in a similar light as `Send` and `Sync`: a trait that allows soundness requirements to be proven at compile time.
- `NoCell` being an auto-trait, it is, like `Send` and `Sync` a sneaky SemVer hazard.
    - Note that this SemVer hazard already exists through the existence of static-promotion, as exemplified by the following example:
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
    - The provided example is also, in RFC author's estimation, the main way in which `NoCell` is likely to be depended upon: allowing bounds on it will likely not expand the hazard much.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- The benefits of stabilizing `core::mem::NoCell` have been highlighted in [Motivation](#motivation).
    - By not stabilizing `core::mem::NoCell` in trait bounds, we are preventing useful and sound code patterns from existing which were previously supported.
- Alternatively, a non-auto sub-trait of `core::mem::NoCell` may be defined:
    - While this reduces the SemVer hazard by making its breakage more obvious, this does lose part of the usefulness that `core::mem::NoCell` would provide to projects such as `zerocopy`.
    - A "perfect" derive macro should then be introduced to ease the implementation of this trait. A lint may be introduced in `clippy` to inform users of the existence and applicability of this new trait.

# Prior Art
[prior-art]: #prior-art
- This trait has a long history: it existed in ancient times but got [removed](https://github.com/rust-lang/rust/pull/13076) before Rust 1.0.
  In 2017 it got [added back](https://github.com/rust-lang/rust/pull/41349) as a way to simplify the implementation of the `interior_unsafe` query, but it was kept private to the standard library.
  In 2019, a [request](https://github.com/rust-lang/rust/issues/60715) was filed to publicly expose the trait, but not a lot happened until recently when the issue around static promotion led to it being [exposed unstably](https://github.com/rust-lang/rust/pull/121840).
- The work necessary for this RFC has already been done and merged in [this PR](https://github.com/rust-lang/rust/issues/121675), and a [tracking issue](https://github.com/rust-lang/rust/issues/121675) was opened.
- zerocopy's [`Immutable`](https://docs.rs/zerocopy/0.8.0-alpha.11/zerocopy/trait.Immutable.html) seeks to provide the same guarantees as `core::marker::NoCell`.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None.

# Future possibilities
[future-possibilities]: #future-possibilities

- During design meetings, the problem of auto-traits as a semver-hazard was considered more broadly, leading to the idea of a new lint.
  This lint would result in a warning if code relied on a type implementing an trait that was automatically implemented for it, but that
  the authors haven't opted into explicitly:
    - Under these considerations, removing the auto-trait implementation of a type would no longer be considered a breaking change.
    - `#[derive(NoCell, Send, Sync, Pin)]` was proposed as the way for authors to explicitly opt into these trait, making their removal a breaking change.
    - Note that a syntax to express this for `async fn`'s resulting opaque type would need to be established too.
    - Such a lint would have the additional benefit of helping authors spot when they accidentally remove one of these properties from their types.

- Complementary to that lint, a lint encouraging explicitly opting in or out of auto-traits that are available for a type would help raise the
  awareness around auto-traits and their semver implications.

- Adding a `trait Pure: NoCell` which extends the interior immutability guarantee to indirected data could be valuable:
    - This is however likely to be a fool's errand, as indirections could (for example) be hidden behind keys to global collections.
    - Providing such a trait could be left to the ecosystem unless we'd want it to be an auto-trait also (unlikely).

- Given that removing a `NoCell` implementation from a type would only be considered a breaking change if its documentation states so, we may want a way for the standard library express which types are stably `NoCell`.
    - Maybe we could do this as a blanket statement about primitive types (including function pointers).
    - Or we might make the statement individually about "common sense" types such as `IpAddr`, `Box`, `Arc`.
